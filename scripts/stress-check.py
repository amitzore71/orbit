"""Run the opt-in native lifecycle driver and sample real Win32 process resources.

Requires Python + psutil, and cargo build --release --features diagnostics --locked.
No synthetic mouse/keyboard input or changes to the user's app data.
"""
import argparse
import ctypes
import json
import statistics
import subprocess
import time
from pathlib import Path
import psutil

ROOT = Path(__file__).resolve().parents[1]
kernel = ctypes.WinDLL("kernel32", use_last_error=True)
user = ctypes.WinDLL("user32", use_last_error=True)
kernel.OpenProcess.argtypes = [ctypes.c_uint32, ctypes.c_int, ctypes.c_uint32]
kernel.OpenProcess.restype = ctypes.c_void_p
kernel.CloseHandle.argtypes = [ctypes.c_void_p]
kernel.GetProcessTimes.argtypes = [ctypes.c_void_p] + [ctypes.POINTER(ctypes.c_uint64)] * 4
kernel.GetProcessTimes.restype = ctypes.c_int
kernel.QueryProcessCycleTime.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint64)]
user.GetGuiResources.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
user.GetGuiResources.restype = ctypes.c_uint32


def sample(process, handle):
    fields = [ctypes.c_uint64() for _ in range(4)]
    if not kernel.GetProcessTimes(handle, *map(ctypes.byref, fields)):
        raise ctypes.WinError(ctypes.get_last_error())
    cycles = ctypes.c_uint64()
    if not kernel.QueryProcessCycleTime(handle, ctypes.byref(cycles)):
        raise ctypes.WinError(ctypes.get_last_error())
    memory = process.memory_info()
    return dict(elapsed=time.monotonic(), privateBytes=memory.private, workingSet=memory.rss,
                handles=process.num_handles(), threads=process.num_threads(),
                gdi=user.GetGuiResources(handle, 0), user=user.GetGuiResources(handle, 1),
                cpuMs=(fields[2].value + fields[3].value) / 10000, cpuCycles=cycles.value)


def summarize(samples):
    if not samples:
        return {}
    first, last = samples[0], samples[-1]
    elapsed = last["elapsed"] - first["elapsed"]
    return dict(samples=len(samples), elapsedSeconds=elapsed,
                privateBytesMin=min(s["privateBytes"] for s in samples),
                privateBytesMax=max(s["privateBytes"] for s in samples),
                privateBytesMedian=statistics.median(s["privateBytes"] for s in samples),
                privateBytesFirst=first["privateBytes"], privateBytesLast=last["privateBytes"],
                handlesFirst=first["handles"], handlesLast=last["handles"],
                gdiFirst=first["gdi"], gdiLast=last["gdi"],
                userFirst=first["user"], userLast=last["user"],
                threadsFirst=first["threads"], threadsLast=last["threads"],
                cpuMs=last["cpuMs"]-first["cpuMs"], cpuCycles=last["cpuCycles"]-first["cpuCycles"],
                cpuPercentOneCore=100*(last["cpuMs"]-first["cpuMs"])/(elapsed*1000) if elapsed else 0)


def run(executable, data, options, duration, verifier=False):
    started = time.monotonic()
    proc = subprocess.Popen([str(executable), "--isolated", "--data-dir", str(data),
                             "--quit-after", str(duration), *options],
                            creationflags=subprocess.CREATE_NO_WINDOW)
    process = psutil.Process(proc.pid)
    handle = kernel.OpenProcess(0x1410, 0, proc.pid)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    samples = []
    verified_modules = []
    try:
        while proc.poll() is None:
            if time.monotonic()-started > duration+20:
                proc.kill()
                raise AssertionError("Native lifecycle driver timed out")
            progress = {}
            try:
                progress = json.loads((data / "stress-progress.json").read_text())
            except (OSError, ValueError):
                pass
            try:
                item = sample(process, handle)
                if verifier and not verified_modules and time.monotonic()-started > 2:
                    verified_modules = sorted({Path(m.path).name.lower() for m in process.memory_maps() if Path(m.path).name.lower() in ("verifier.dll", "vfbasics.dll", "vrfcore.dll")})
                    assert "vfbasics.dll" in verified_modules, "Application Verifier was not loaded"
                if verified_modules:
                    item["verifierModules"] = verified_modules
                item.update(progress)
                samples.append(item)
            except (psutil.NoSuchProcess, OSError):
                if proc.poll() is None:
                    raise
            time.sleep(0.5)
        assert proc.returncode == 0, f"Application exited {proc.returncode}"
    finally:
        kernel.CloseHandle(handle)
        if proc.poll() is None:
            proc.kill()
    return samples


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--exe", type=Path, default=ROOT / "target/release/orbit.exe")
    parser.add_argument("--cycles", type=int, default=500)
    parser.add_argument("--tasks", type=int, default=2000)
    parser.add_argument("--verifier", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts/stress-results.json")
    args = parser.parse_args()
    data = ROOT / "artifacts" / ("stress-"+str(time.time_ns()))
    data.mkdir(parents=True)
    # Initialize the isolated schema without displaying any app windows.
    run(args.exe, data, ["--tray"], 1)
    import sqlite3
    with sqlite3.connect(data / "orbit.db") as db:
        db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
            volume=0, notifications=False, hotkey_toggle="", hotkey_show="", hotkey_skip="")),))
        db.executemany("INSERT INTO tasks(title,estimate) VALUES(?,2)",
                       [(f"Task {n} — 日本語 हिंदी",) for n in range(args.tasks)])
    samples = run(args.exe, data, ["--tray", "--stress-cycles", str(args.cycles)],
                  max(180, args.cycles*2+90), args.verifier)
    result = json.loads((data / "stress-progress.json").read_text())
    assert result["phase"] == "complete", result
    assert result["retainedWidgets"] == 0, result
    assert result["weakChecks"] == args.cycles*3, result
    assert result["trayDisplayTicks"] == 0, result
    # Compare the same lifecycle phase after warmup, never an open window to a closed one.
    quiet = [s for s in samples if s.get("phase") == "tray-cooldown"]
    settled = quiet[20:]  # allow 10 seconds for OS cleanup
    assert len(settled) >= 20, "Insufficient cooldown samples"
    tail = settled[-1]
    head = settled[0]
    assert tail["handles"] <= head["handles"]+2, (head, tail)
    assert tail["gdi"] <= head["gdi"], (head, tail)
    assert tail["user"] <= head["user"], (head, tail)
    if not args.verifier:
        assert tail["privateBytes"] <= head["privateBytes"]+1024*1024, (head, tail)
    batches = []
    for start in range(50, args.cycles-49, 100):
        batch = [s for s in samples if s.get("phase") == "cycling" and start <= s.get("cycles", 0) < start+100]
        if batch:
            batches.append(dict(firstCycle=start, lastCycle=start+99, **summarize(batch)))
    # The lifecycle includes both open and closed windows. Compare batch minima to avoid
    # mistaking an open-window allocation for a leak; weak checks test ownership directly.
    if len(batches) >= 2 and not args.verifier:
        assert batches[-1]["privateBytesMin"] <= batches[0]["privateBytesMin"] + 2*1024*1024, batches
    report = dict(driver=result, tasks=args.tasks, executable=str(args.exe),
                  verifierEnabled=args.verifier,
                  cycling=summarize([s for s in samples if s.get("phase") == "cycling"]),
                  lifecycleBatches=batches, trayAfterUse=summarize(settled), samples=samples)
    output = args.output
    output.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k:v for k,v in report.items() if k != "samples"}, indent=2))
    print(f"Raw samples: {output}")

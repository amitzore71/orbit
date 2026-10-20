"""Measure launch to an externally observed, responsive, painted Orbit window.

This supplements native input-idle. A pixel probe observes distinct app regions
and a bounded WM_NULL checks the message loop. It does not measure physical
display latency, cold-cache startup, or every control's input-to-paint latency.
"""
import argparse
import ctypes
from ctypes import wintypes
from datetime import datetime
import hashlib
import json
from pathlib import Path
import sqlite3
import statistics
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
root = Path(__file__).resolve().parents[1]
parser.add_argument("--executable", type=Path, default=root / "target/release/orbit.exe")
parser.add_argument("--output", type=Path, default=root / "artifacts/window-startup.json")
parser.add_argument("--trials", type=int, default=30)
args = parser.parse_args()
if args.trials < 2:
    parser.error("Use at least two trials.")
exe = args.executable.resolve()
data = root / "artifacts" / f"window-startup-{time.time_ns()}"
data.mkdir(parents=True)
base = [str(exe), "--isolated", "--data-dir", str(data)]
subprocess.run(base + ["--tray", "--quit-after", "1"], check=True, timeout=20)
with sqlite3.connect(data / "orbit.db") as db:
    settings = dict(volume=0, notifications=False, autostart=False, appearance=1,
                    hotkey_toggle="", hotkey_show="", hotkey_skip="")
    db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(settings),))

user = ctypes.WinDLL("user32", use_last_error=True)
gdi = ctypes.WinDLL("gdi32", use_last_error=True)
try:
    user.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
except AttributeError:
    pass
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
user.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
user.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user.IsWindowVisible.argtypes = [wintypes.HWND]
user.GetClientRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
user.GetDC.argtypes = [wintypes.HWND]
user.GetDC.restype = wintypes.HDC
user.ReleaseDC.argtypes = [wintypes.HWND, wintypes.HDC]
gdi.GetPixel.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi.GetPixel.restype = wintypes.DWORD
user.SendMessageTimeoutW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM,
                                   wintypes.LPARAM, wintypes.UINT, wintypes.UINT,
                                   ctypes.POINTER(ctypes.c_size_t)]
user.SendMessageTimeoutW.restype = wintypes.LPARAM


def ready_window(pid):
    matches = []

    @callback_type
    def visit(hwnd, _):
        owner = wintypes.DWORD()
        user.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == pid and user.IsWindowVisible(hwnd):
            matches.append(hwnd)
        return True

    user.EnumWindows(visit, 0)
    for hwnd in matches:
        rect = wintypes.RECT()
        if not user.GetClientRect(hwnd, ctypes.byref(rect)) or rect.right < 600:
            continue
        result = ctypes.c_size_t()
        if not user.SendMessageTimeoutW(hwnd, 0, 0, 0, 2, 20, ctypes.byref(result)):
            continue
        dc = user.GetDC(hwnd)
        if not dc:
            continue
        try:
            # Sample the nav, selected item, mode control, canvas, and timer.
            coordinates = [(0.03, 0.07), (0.04, 0.14), (0.34, 0.16),
                           (0.8, 0.7), (0.56, 0.51), (0.75, 0.85)]
            colors = {gdi.GetPixel(dc, int(rect.right * x), int(rect.bottom * y))
                      for x, y in coordinates}
            colors.discard(0xFFFFFFFF)
            if len(colors) >= 3:
                return True
        finally:
            user.ReleaseDC(hwnd, dc)
    return False


samples = []
for trial in range(args.trials):
    started = time.perf_counter()
    process = subprocess.Popen(base)
    try:
        while not ready_window(process.pid):
            if process.poll() is not None:
                raise RuntimeError(f"Orbit exited before the paint probe on trial {trial}")
            if time.perf_counter() - started > 20:
                raise RuntimeError("Timed out observing a painted Orbit window")
            time.sleep(0.002)
        samples.append((time.perf_counter() - started) * 1000)
        process.terminate()
        process.wait(timeout=15)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)

warm = sorted(samples[1:])
p95 = warm[min(len(warm) - 1, max(0, int(len(warm) * 0.95 + 0.999) - 1))]
report = dict(measuredAt=datetime.now().astimezone().isoformat(), executable=str(exe),
              executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
              trials=len(samples), samplesMs=samples,
              warmMedianMs=statistics.median(warm), warmP95Ms=p95,
              scope="Launch to distinct painted app regions plus responsive WM_NULL; 2 ms polling. First trial excluded from warm statistics. Empty database, light appearance. Not cold-cache or physical display latency.")
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(report, indent=2))

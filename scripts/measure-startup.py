"""Measure Windows launch to native input-idle, separately from first paint."""
import argparse
import ctypes
import datetime
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--executable", type=Path, default=root / "target/release/orbit.exe")
parser.add_argument("--output", type=Path, default=root / "artifacts/startup.json")
parser.add_argument("--trials", type=int, default=7)
args = parser.parse_args()
if args.trials < 2:
    parser.error("At least two trials are needed to separate the initial launch.")
executable = args.executable.resolve()
data = root / "artifacts" / f"startup-{time.time_ns()}"
user = ctypes.WinDLL("user32", use_last_error=True)
user.WaitForInputIdle.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
user.WaitForInputIdle.restype = ctypes.c_uint32
samples = []
for _ in range(args.trials):
    started = time.perf_counter()
    process = subprocess.Popen([str(executable), "--isolated", "--tray", "--data-dir", str(data), "--quit-after", "1"],
                               creationflags=subprocess.CREATE_NO_WINDOW)
    try:
        status = user.WaitForInputIdle(int(process._handle), 10000)
        elapsed = (time.perf_counter() - started) * 1000
        if status != 0:
            raise RuntimeError(f"Native input-idle measurement failed: {status}")
        process.wait(timeout=15)
        if process.returncode != 0:
            raise RuntimeError(f"Orbit exited with status {process.returncode}")
        samples.append(elapsed)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
report = dict(measuredAt=datetime.datetime.now().astimezone().isoformat(), executable=str(executable),
              executableSha256=hashlib.sha256(executable.read_bytes()).hexdigest().upper(),
              executableBytes=executable.stat().st_size, samplesMs=samples,
              warmMedianMs=statistics.median(samples[1:]),
              scope="Tray launch to native input-idle; warm median excludes initial launch. This does not measure first paint or cold file-cache startup.")
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2), encoding="utf-8", newline="\n")
print(json.dumps(report, indent=2))

"""Measure startup to a React workspace mounted after its initial real Rust snapshot."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--executable', type=Path, default=root / 'target/release/orbit.exe')
parser.add_argument('--trials', type=int, default=8)
parser.add_argument('--output', type=Path, default=root / 'artifacts/desktop-startup.json')
args = parser.parse_args()
exe = args.executable.resolve()
samples = []
for trial in range(args.trials):
    data = root / 'artifacts' / f'startup-{time.time_ns()}'
    process = subprocess.Popen([str(exe), '--isolated', '--data-dir', str(data), '--startup-probe', '--quit-after', '3'])
    try:
        deadline = time.monotonic() + 25
        while not (data / 'startup.json').exists():
            if process.poll() is not None:
                raise RuntimeError(f'Orbit exited before its initial snapshot was rendered ({process.returncode}).')
            if time.monotonic() > deadline:
                raise RuntimeError('Timed out waiting for the mounted desktop workspace.')
            time.sleep(.01)
        samples.append(json.loads((data / 'startup.json').read_text())['interactiveMs'])
        process.wait(timeout=15)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
warm = sorted(samples[1:] or samples)
report = dict(measuredAt=datetime.now(timezone.utc).isoformat(), executable=str(exe),
    executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(), trials=len(samples), samplesMs=samples,
    warmMedianMs=statistics.median(warm), warmP95Ms=warm[min(len(warm)-1, int((len(warm)-1)*.95+.5))],
    scope='Process entry to React workspace mounted after initial Rust snapshot. First trial excluded from warm statistics. Empty isolated database. Does not measure physical display latency or cold boot.')
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2), encoding='utf-8')
print(json.dumps(report, indent=2))

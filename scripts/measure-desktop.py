"""Measure the entire Orbit/WebView2 process tree, rather than only the Rust host."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import time
import psutil

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('executable', type=Path)
parser.add_argument('--seconds', type=int, default=10)
parser.add_argument('--warmup', type=int, default=5)
parser.add_argument('--hidden-soak', action='store_true', help='Measure a running tray session for ten minutes.')
parser.add_argument('--output', type=Path, default=root / 'artifacts/desktop-footprint.json')
args = parser.parse_args()
exe = args.executable.resolve()
scenarios = [('tray-idle', ['--tray']), ('tray-running', ['--tray', '--running']), ('main-paused', []),
    ('main-running', ['--running']), ('main-minimized-running', ['--running', '--minimized']), ('all-widgets-running', ['--running', '--all-widgets'])]
if args.hidden_soak:
    scenarios = [('tray-running-10min', ['--tray', '--running'])]
    args.seconds = 600
results = []
for name, options in scenarios:
    data = root / 'artifacts' / f'footprint-{name}-{time.time_ns()}'
    proc = subprocess.Popen([str(exe), '--isolated', '--data-dir', str(data), '--quit-after', str(args.warmup+args.seconds+120), *options])
    process = psutil.Process(proc.pid)
    cpu_seen, samples, working, counts = {}, [], [], []
    def tree():
        return [process, *process.children(recursive=True)]
    def sample_cpu(members):
        total = 0
        for member in members:
            try:
                key=(member.pid,member.create_time()); cpu=member.cpu_times(); value=cpu.user+cpu.system
                if key in cpu_seen: total += max(0, value-cpu_seen[key])
                cpu_seen[key]=value
            except (psutil.NoSuchProcess, psutil.AccessDenied): pass
        return total
    try:
        time.sleep(args.warmup)
        sample_cpu(tree()); cpu_total=0; started=time.perf_counter()
        tick=0
        while time.perf_counter()-started < args.seconds:
            time.sleep(min(1,max(0,args.seconds-(time.perf_counter()-started))))
            if proc.poll() is not None: raise RuntimeError(f'Orbit exited in {name}: {proc.returncode}')
            members=tree(); private=resident=0
            for member in members:
                try:
                    mem=member.memory_info(); private += mem.private; resident += mem.rss
                except (psutil.NoSuchProcess, psutil.AccessDenied): pass
            cpu_total += sample_cpu(members); samples.append(private); working.append(resident); counts.append(len(members))
            tick+=1
            if args.hidden_soak and tick%60==0: print(f'{name}: {int(time.perf_counter()-started)}/{args.seconds} seconds sampled', flush=True)
        elapsed=time.perf_counter()-started
        result=dict(scenario=name, privateBytesMean=int(statistics.mean(samples)), privateBytesPeak=max(samples), workingSetBytesMean=int(statistics.mean(working)), processCountPeak=max(counts), cpuPercentOfOneCore=round(100*cpu_total/elapsed,4), sampleSeconds=args.seconds, actualSampleSeconds=round(elapsed,3), warmupSeconds=args.warmup)
        results.append(result); print(json.dumps(result), flush=True)
        # End the isolated test process after sampling; do not wait out the safety margin.
        proc.terminate()
        proc.wait(timeout=20)
    finally:
        if proc.poll() is None:
            for member in reversed(tree()):
                try: member.kill()
                except psutil.NoSuchProcess: pass
            proc.wait(timeout=10)
report=dict(measuredAt=datetime.now(timezone.utc).isoformat(), executable=str(exe), executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(), scenarios=results,
    note='Combined private committed bytes and summed working sets of the Rust host and all descendants. Shared pages may be counted more than once in summed working sets. CPU is relative to one logical core; exited child process tails can be missed. Empty isolated databases. No working-set trimming.')
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report,indent=2),encoding='utf-8')

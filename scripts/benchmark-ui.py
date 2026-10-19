"""Compare diagnostic builds before and after task-model/scheduling changes."""
import argparse
import contextlib
import hashlib
import json
from pathlib import Path
import sqlite3
import statistics
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--before", type=Path, required=True)
parser.add_argument("--after", type=Path, default=root / "target/debug/orbit.exe")
parser.add_argument("--output", type=Path, default=root / "artifacts/ui-optimization.json")
args = parser.parse_args()
run_root = root / "artifacts" / f"ui-performance-{time.time_ns()}"
run_root.mkdir(parents=True)
results = dict(buildProfile="debug", tasks=5000, selectionsPerSample=100,
               measuredScope="Task-model updates only; repaint and persistence are outside timed samples")

for name, executable in [("before", args.before), ("after", args.after)]:
    executable = executable.resolve()
    data = run_root / name
    command = [str(executable), "--isolated", "--data-dir", str(data)]
    subprocess.run(command + ["--tray", "--quit-after", "1"], check=True, timeout=20)
    with contextlib.closing(sqlite3.connect(data / "orbit.db")) as db, db:
        db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
            volume=0, notifications=False, autostart=False,
            hotkey_toggle="", hotkey_show="", hotkey_skip="")),))
    subprocess.run(command + ["--performance-audit", "--quit-after", "75"],
                   check=True, timeout=90)
    report = json.loads((data / "performance.json").read_text())
    # The task benchmark blocks the UI thread before the first display window.
    # Discard that warm-up window; subsequent windows run after it has settled.
    report["discardedWarmupTicks"] = report["wakeups"].pop("tasks-page")
    report["medianMsPer100Selections"] = statistics.median(report["samplesMs"])
    with executable.open("rb") as binary:
        report["executableSha256"] = hashlib.file_digest(binary, "sha256").hexdigest()
    results[name] = report
    print(json.dumps({name: report}), flush=True)

assert results["before"]["modelReplacements"] == 700
assert results["after"]["modelReplacements"] == 0
for scenario in ["tasks-widget-only"]:
    assert results["before"]["wakeups"][scenario] > 0
    assert results["after"]["wakeups"][scenario] == 0
for scenario in ["timer-page", "mini-visible"]:
    assert results["after"]["wakeups"][scenario] > 0
results["taskModelSpeedup"] = (results["before"]["medianMsPer100Selections"] /
                               results["after"]["medianMsPer100Selections"])
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(results, indent=2), encoding="utf-8")
print(f"Results: {args.output}", flush=True)

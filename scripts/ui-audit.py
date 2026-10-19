"""Capture Orbit's native UI using isolated sample data at Windows UI scales.

Build first: cargo build --features diagnostics --locked
Run: python scripts/ui-audit.py
"""
import argparse
import contextlib
from datetime import datetime, timedelta
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--exe", type=Path, default=root / "target/debug/orbit.exe")
parser.add_argument("--scales", nargs="+", type=float, default=[1, 1.25, 1.5, 2])
args = parser.parse_args()
output = root / "artifacts" / f"ui-audit-{time.time_ns()}"
output.mkdir(parents=True)
results = []

for scale in args.scales:
    data = output / f"scale-{scale:g}"
    data.mkdir()
    env = dict(os.environ, SLINT_SCALE_FACTOR=str(scale))
    command = [str(args.exe.resolve()), "--isolated", "--data-dir", str(data)]
    subprocess.run(command + ["--tray", "--quit-after", "1"], env=env, check=True, timeout=20)
    with contextlib.closing(sqlite3.connect(data / "orbit.db")) as db, db:
        db.executemany("INSERT INTO tasks(id,title,estimate,done) VALUES(?,?,?,?)", [
            (1, "Prepare the weekly project review", 3, 0),
            (2, "Read 日本語 notes and plan the next iteration हिंदी", 2, 0),
            (3, "A deliberately long task title to verify that task actions stay visible when the text is truncated at compact window sizes", 99, 0),
            (4, "Finish the first draft", 1, 1),
        ])
        db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
            volume=0, notifications=False, autostart=False,
            hotkey_toggle="", hotkey_show="", hotkey_skip="")),))
        checkpoint = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
        checkpoint["selected_task"] = 1
        db.execute("UPDATE meta SET value=? WHERE key='timer'", (json.dumps(checkpoint),))
        today = datetime.now().replace(hour=10, minute=0, second=0, microsecond=0)
        for i, minutes in enumerate([25, 25, 12]):
            start = int((today - timedelta(days=i)).timestamp() * 1000)
            end = start + minutes * 60_000
            db.execute("INSERT INTO sessions VALUES(?,?,?,?)", (101+i, 1, int(minutes == 25), end))
            db.execute("INSERT INTO intervals VALUES(?,?,?,?,?,?)", (101+i, 101+i, 1, start, end, minutes*60_000))
        db.execute("INSERT INTO tasks(id,title,estimate,deleted) VALUES(5,'Research archived project',1,1)")
        for i, (task_id, minutes, completed) in enumerate([(2, 18, 0), (3, 8, 0), (4, 25, 1), (5, 6, 0), (None, 2, 0)]):
            start = int(today.timestamp() * 1000) + (i + 1) * 3_600_000
            end = start + minutes * 60_000
            db.execute("INSERT INTO sessions VALUES(?,?,?,?)", (201+i, task_id, completed, end))
            db.execute("INSERT INTO intervals VALUES(?,?,?,?,?,?)", (201+i, 201+i, task_id, start, end, minutes*60_000))
    subprocess.run(command + ["--all-widgets", "--ui-audit", "--quit-after", "120"],
                   env=env, check=True, timeout=135)
    manifest = json.loads((data / "screenshots/audit.json").read_text())
    assert len(manifest) == 20, f"Incomplete screenshots at scale {scale}: {len(manifest)}"
    assert all(abs(item["scale"] - scale) < 0.01 for item in manifest), "Unexpected render scale"
    assert all((data / "screenshots" / item["image"]).stat().st_size > 100 for item in manifest)
    for item in manifest:
        if item["image"].startswith("widget-"):
            continue
        size = (940, 740) if item["image"] in {"timer.png", "tasks.png", "reports.png", "widgets.png", "settings.png", "about.png"} else (760, 540)
        assert (item["width"], item["height"]) == tuple(round(n * scale) for n in size), item
    results.append(dict(scale=scale, captures=len(manifest), directory=str(data / "screenshots")))
    print(json.dumps(results[-1]), flush=True)

(output / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
print(f"UI audit saved to {output}", flush=True)

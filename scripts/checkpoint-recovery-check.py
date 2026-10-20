"""Verify a real one-minute checkpoint survives abrupt termination exactly once."""
import argparse
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import time

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("executable", type=Path)
args = parser.parse_args()
exe = args.executable.resolve()
data = root / "artifacts" / f"checkpoint-recovery-{time.time_ns()}"
base = [str(exe), "--isolated", "--tray", "--data-dir", str(data)]
subprocess.run(base + ["--quit-after", "1"], check=True, timeout=20)
with sqlite3.connect(data / "orbit.db") as db:
    db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
        focus_minutes=3, volume=0, notifications=False, autostart=False,
        hotkey_toggle="", hotkey_show="", hotkey_skip="")),))

process = subprocess.Popen(base + ["--running", "--quit-after", "80"])
try:
    started = time.monotonic()
    while True:
        if process.poll() is not None:
            raise RuntimeError("Orbit exited before its periodic checkpoint")
        with sqlite3.connect(data / "orbit.db") as db:
            checkpoint = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
            active = db.execute("SELECT COALESCE(SUM(active_ms),0) FROM intervals").fetchone()[0]
        if active >= 59_000:
            assert checkpoint["running"]
            assert active < 70_000, active
            assert checkpoint["remaining_ms"] == 180_000 - active, checkpoint
            break
        if time.monotonic() - started > 72:
            raise RuntimeError("No durable one-minute checkpoint was observed")
        time.sleep(0.25)
    process.kill()
    process.wait(timeout=10)
finally:
    if process.poll() is None:
        process.kill()
        process.wait(timeout=10)

subprocess.run(base + ["--quit-after", "1"], check=True, timeout=20)
with sqlite3.connect(data / "orbit.db") as db:
    restored = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    assert not restored["running"]
    assert restored["remaining_ms"] == checkpoint["remaining_ms"]
    assert db.execute("SELECT COALESCE(SUM(active_ms),0) FROM intervals").fetchone()[0] == active
    assert db.execute("SELECT COUNT(*) FROM sessions WHERE completed=1").fetchone()[0] == 0
    assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
report = dict(executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
              realPeriodicCheckpoint=True, abruptTerminationRestoredPaused=True,
              creditedFocusMs=active, duplicateCredit=False, sqliteIntegrity="ok",
              dataDirectory=str(data))
(root / "artifacts/checkpoint-recovery-check.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
print(json.dumps(report, indent=2))

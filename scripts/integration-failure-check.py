"""Real process tests for SQLite write failure and abrupt termination/restart."""
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time
import psutil

root = Path(__file__).resolve().parents[1]
exe = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target/release/orbit.exe"
data = root / "artifacts" / f"failure-integration-{time.time_ns()}"


def args(*options):
    return [str(exe), "--isolated", "--tray", "--data-dir", str(data), *options]


def run(*options):
    subprocess.run(args(*options), check=True, timeout=30)


run("--quit-after", "1")
with sqlite3.connect(data / "orbit.db") as db:
    db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
        focus_minutes=1, volume=0, notifications=False, hotkey_toggle="", hotkey_show="", hotkey_skip="")),))
    db.executescript("""CREATE TRIGGER deny_checkpoint BEFORE INSERT ON meta WHEN NEW.key='timer'
                       BEGIN SELECT RAISE(ABORT,'Injected write failure'); END;""")
proc = subprocess.Popen(args("--running", "--quit-after", "5"))
try:
    time.sleep(1)
    assert proc.poll() is None
    private = psutil.Process(proc.pid).memory_info().private
    with sqlite3.connect(data / "orbit.db") as db:
        db.execute("DROP TRIGGER deny_checkpoint")
    assert proc.wait(timeout=15) == 0
finally:
    if proc.poll() is None:
        proc.kill()
with sqlite3.connect(data / "orbit.db") as db:
    checkpoint = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    active = db.execute("SELECT COALESCE(SUM(active_ms),0) FROM intervals").fetchone()[0]
    assert checkpoint["running"] is False
    assert checkpoint["total_ms"] == 60_000
    assert 0 <= active < 1000, (checkpoint, active)
    assert 60_000 - checkpoint["remaining_ms"] == active, (checkpoint, active)
    assert db.execute("SELECT COUNT(*) FROM sessions WHERE completed=1").fetchone()[0] == 0

# Terminate a genuinely running process after its Start checkpoint is durable.
proc = subprocess.Popen(args("--running", "--quit-after", "20"))
try:
    time.sleep(2)
    assert proc.poll() is None
    with sqlite3.connect(data / "orbit.db") as db:
        crashed = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
        assert crashed["running"] is True
    proc.kill()
    proc.wait(timeout=10)
finally:
    if proc.poll() is None:
        proc.kill()
run("--quit-after", "1")
with sqlite3.connect(data / "orbit.db") as db:
    restored = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    assert restored["running"] is False
    assert restored["remaining_ms"] == crashed["remaining_ms"]
    assert db.execute("SELECT COALESCE(SUM(active_ms),0) FROM intervals").fetchone()[0] == active
    assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
report = dict(sqliteFailurePausedTimer=True, recoveredCheckpoint=True, creditedMs=active,
              realProcessTerminationPausedRestore=True, sqliteIntegrity="ok", privateBytesDuringFailure=private)
(root / "artifacts/integration-failure-check.json").write_text(json.dumps(report, indent=2))
print(json.dumps(report, indent=2))

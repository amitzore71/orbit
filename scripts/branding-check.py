"""Smoke-test Orbit and verify it reuses a database with the previous filename."""
import contextlib
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[1]
exe = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "target/release/orbit.exe"
data = root / "artifacts" / f"orbit-branding-{time.time_ns()}"


def run(*options):
    subprocess.run([str(exe), "--isolated", "--data-dir", str(data), *options], check=True, timeout=20)


run("--tray", "--quit-after", "1")
assert (data / "orbit.db").is_file()
# Model an existing pre-rename database, retaining its actual bytes/schema in place.
(data / "orbit.db").rename(data / "still.db")
with contextlib.closing(sqlite3.connect(data / "still.db")) as db:
    with db:
        db.execute("INSERT INTO tasks(title,estimate) VALUES(?,3)", ("Orbit preserves 日本語 हिंदी tasks",))
        db.execute("INSERT OR REPLACE INTO meta VALUES('settings',?)", (json.dumps(dict(
            volume=0, notifications=False, autostart=False, hotkey_toggle="", hotkey_show="", hotkey_skip="")),))
        # Exercise the actual schema-2 upgrade, including an unusually long old interval.
        db.execute("DROP INDEX intervals_long_start")
        db.execute("PRAGMA user_version=2")
        db.execute("INSERT INTO intervals(session_id,start_ms,end_ms,active_ms) VALUES(987654,0,172800000,172800000)")
run("--all-widgets", "--running", "--quit-after", "3")
assert not (data / "orbit.db").exists(), "The existing database must not be replaced"
with contextlib.closing(sqlite3.connect(data / "still.db")) as db:
    assert db.execute("SELECT title,estimate FROM tasks WHERE deleted=0").fetchall() == [("Orbit preserves 日本語 हिंदी tasks", 3)]
    assert db.execute("PRAGMA integrity_check").fetchone()[0] == "ok"
    assert db.execute("PRAGMA user_version").fetchone()[0] == 3
    assert db.execute("SELECT COUNT(*) FROM sqlite_master WHERE name='intervals_long_start'").fetchone()[0] == 1
    assert db.execute("SELECT start_ms,end_ms,active_ms FROM intervals WHERE session_id=987654").fetchone() == (0,172800000,172800000)
    checkpoint = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    assert checkpoint["running"] is False
report = dict(application="Orbit", executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
              canonicalDatabaseCreated=True,
              legacyDatabaseReused=True, schema2MigrationPreservedHistory=True, unicodeTaskPreserved=True, allWindowsSmokePassed=True,
              sqliteIntegrity="ok")
(root / "docs/branding-validation.json").write_text(json.dumps(report, indent=2), encoding="utf-8", newline="\n")
print(json.dumps(report, indent=2))

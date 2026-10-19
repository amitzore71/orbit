"""Exercise a real background expiry, persistence, and paused restart without UI input."""
import json
import pathlib
import sqlite3
import subprocess
import sys
import tempfile
import time

root = pathlib.Path(__file__).resolve().parent.parent
executable = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / 'target/release/orbit.exe'
(root / 'artifacts').mkdir(exist_ok=True)
data = pathlib.Path(tempfile.mkdtemp(prefix='integration-', dir=root / 'artifacts'))

def run(*args):
    started = time.perf_counter()
    subprocess.run([str(executable), '--isolated', '--data-dir', str(data), '--tray', *args], check=True, timeout=80)
    return time.perf_counter() - started

run('--quit-after', '1')
with sqlite3.connect(data / 'orbit.db') as db:
    settings = dict(focus_minutes=1, short_minutes=1, long_minutes=2, long_interval=4,
                    volume=0, notifications=False, hotkey_toggle='', hotkey_show='', hotkey_skip='')
    db.execute("INSERT INTO meta(key,value) VALUES('settings',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value", (json.dumps(settings),))
    db.execute("DELETE FROM meta WHERE key='timer'")

elapsed = run('--running', '--quit-after', '64')
with sqlite3.connect(data / 'orbit.db') as db:
    checkpoint = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    completed = db.execute('SELECT COUNT(*) FROM sessions WHERE completed=1').fetchone()[0]
    focus_ms = db.execute('SELECT COALESCE(SUM(active_ms),0) FROM intervals').fetchone()[0]
    assert completed == 1, completed
    assert focus_ms == 60_000, focus_ms
    assert checkpoint['mode'] == 'ShortBreak' and checkpoint['cycle'] == 1, checkpoint
    assert checkpoint['running'] is False, checkpoint

# An interrupted run must reappear paused, without crediting time while the app was absent.
with sqlite3.connect(data / 'orbit.db') as db:
    checkpoint.update(mode='Focus', running=True, remaining_ms=12_345, total_ms=60_000, started=True)
    db.execute("UPDATE meta SET value=? WHERE key='timer'", (json.dumps(checkpoint),))
run('--quit-after', '1')
with sqlite3.connect(data / 'orbit.db') as db:
    restored = json.loads(db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()[0])
    assert restored['running'] is False and restored['remaining_ms'] == 12_345, restored
    assert db.execute('SELECT SUM(active_ms) FROM intervals').fetchone()[0] == 60_000

report = dict(backgroundExpiry=True, completedSessions=completed, creditedFocusMs=focus_ms,
              pausedRestart=True, realExpiryRunSeconds=round(elapsed, 3), dataDirectory=str(data))
destination = root / 'artifacts/integration-check.json'
destination.write_text(json.dumps(report, indent=2), encoding='utf-8')
print(json.dumps(report, indent=2))

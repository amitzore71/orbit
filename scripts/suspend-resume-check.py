"""Exercise native power-message handling in an isolated Orbit process, without suspending Windows."""
import ctypes
from ctypes import wintypes
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[1]
exe = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / 'target/release/orbit.exe'
data = root / 'artifacts' / f'power-message-check-{time.time_ns()}'
proc = subprocess.Popen([str(exe), '--isolated', '--data-dir', str(data), '--tray', '--running', '--quit-after', '15'])
user32 = ctypes.WinDLL('user32', use_last_error=True)
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
user32.EnumWindows.argtypes = [callback_type, wintypes.LPARAM]
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
user32.SendMessageTimeoutW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM, wintypes.UINT, wintypes.UINT, ctypes.POINTER(ctypes.c_size_t)]
user32.SendMessageTimeoutW.restype = wintypes.LPARAM

def checkpoint():
    with sqlite3.connect(data / 'orbit.db') as db:
        row = db.execute("SELECT value FROM meta WHERE key='timer'").fetchone()
        return json.loads(row[0]) if row else None

def credited():
    with sqlite3.connect(data / 'orbit.db') as db:
        return db.execute('SELECT COALESCE(SUM(active_ms),0) FROM intervals').fetchone()[0]

try:
    found = []
    @callback_type
    def locate(hwnd, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(owner))
        if owner.value == proc.pid:
            title = ctypes.create_unicode_buffer(128)
            user32.GetWindowTextW(hwnd, title, len(title))
            if title.value == 'Orbit integration':
                found.append(hwnd)
        return True

    deadline = time.monotonic() + 15
    while not found:
        user32.EnumWindows(locate, 0)
        if proc.poll() is not None or time.monotonic() > deadline:
            raise RuntimeError('The isolated native integration window did not start.')
        if not found:
            time.sleep(.05)
    time.sleep(1)
    result = ctypes.c_size_t()
    def power_message(code):
        if not user32.SendMessageTimeoutW(found[0], 0x218, code, 0, 2, 2000, ctypes.byref(result)):
            raise ctypes.WinError(ctypes.get_last_error())

    before = checkpoint()
    power_message(4)  # PBT_APMSUSPEND, addressed only to this test process.
    deadline = time.monotonic() + 5
    while True:
        suspended = checkpoint()
        if suspended and suspended['saved_utc_ms'] > before['saved_utc_ms'] and credited() > 0:
            break
        if time.monotonic() > deadline:
            raise RuntimeError('Native suspend did not flush the active focus interval.')
        time.sleep(.05)
    credit_before = credited()
    time.sleep(2)
    assert credited() == credit_before, 'Focus was credited while the session was suspended.'
    power_message(18)  # PBT_APMRESUMEAUTOMATIC.
    time.sleep(.5)
    resumed = checkpoint()
    assert resumed['saved_utc_ms'] > suspended['saved_utc_ms'], 'Native resume did not persist its transition.'
    assert resumed['running'], 'Native resume should preserve the existing running-session behavior.'
    assert credited() == credit_before, 'Suspended time was credited as focus.'
    proc.wait(timeout=20)
    assert proc.returncode == 0
    with sqlite3.connect(data / 'orbit.db') as db:
        assert db.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
    report = dict(executableSha256=hashlib.sha256(exe.read_bytes()).hexdigest().upper(),
                  nativeSuspendFlushedInterval=True, nativeResumeContinuedSession=True,
                  suspendedTimeCredited=False, sqliteIntegrity='ok',
                  scope='Synthetic Win32 power messages to an isolated Orbit HWND; Windows itself was not suspended. Hardware sleep still requires manual validation.')
    output = root / 'artifacts/upgrade/suspend-resume.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps(report, indent=2))
finally:
    if proc.poll() is None:
        proc.terminate()
        proc.wait(timeout=10)

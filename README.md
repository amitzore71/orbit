# Orbit

An offline Windows focus timer built with Rust and Slint. Plan today's work, start a focus session, and review your effort in a light, responsive desktop interface. No account, telemetry, or background service.

## Run

Requires Windows 11 22H2 or newer, x64. Open `orbit.exe` from a portable package or use the per-user installer. Closing the window hides Orbit in the tray; right-click the tray icon to control or quit it.

Data is stored in `%LOCALAPPDATA%\Orbit\orbit.db`. Use `orbit.exe --data-dir "D:\FocusData"` for another location. Existing pre-Orbit databases in `%LOCALAPPDATA%\StillPomodoro\still.db`, or `still.db` in a custom directory, are reused in place. Uninstalling retains your data. Use **Settings → Data** to create a consistent backup or restore one. Backups pause the timer; restoration replaces local data after creating a recovery copy in the data directory's `backups` folder. Save a copy on another drive to protect against disk failure. Existing backup files are never overwritten.

Version 0.2 migrates existing databases to schema 4 after making an upgrade backup. The latest three automatic upgrade backups are retained; manual and pre-restore backups are retained. Older versions cannot open schema 4. To downgrade, use the pre-upgrade backup with the older application in a separate data directory.

For portable notifications, open **Settings → Notifications** to register the current executable. Repeat after moving it. The installer handles registration automatically. If Windows notifications are unavailable, Orbit uses a tray alert.

## Use

- **Timer:** Focus, short break, and long break. Defaults are 25/5/15 minutes, with a long break every four completed focus sessions. Sleep and hibernation do not consume time. Automatic starts are off by default.
- **Tasks:** Plan and order today's tasks, search All active, inspect Completed, recover deleted tasks from Trash, and deliberately carry earlier plans forward. Use **Start focus** for a fresh focus session or **Use next** during a started session/break. Timer offers a task picker and quick-add. A started session keeps its original task. Only finished focus sessions increase completion counts; skips and resets retain partial focus time. Undo and Trash preserve task IDs and history.
- **Reports:** Daily, Monday-based weekly, monthly, and custom date ranges up to 367 days. Inspect session details by task and export a spreadsheet-safe UTF-8 CSV. Includes partial focus time, deleted task history, and time without a task; excludes pauses and sleep, and uses the current Windows time zone. CSV reports are separate from restorable backups.
- **Widgets:** Timer ring, today's planned tasks, and mini timer. Drag a header to move a widget. Configure them in **Settings → Widgets**; layering and click-through are under Advanced controls. Use **Unlock all widgets** in the tray to recover click-through windows.
- **Settings:** Grouped session, sound, appearance, shortcut/startup, widget, data, and About controls. Follow Windows or choose light/dark appearance; Windows high contrast takes precedence. Drafts survive navigation, with explicit Save/Discard. Duration changes apply to the next session.

Default shortcuts: `Ctrl+Alt+P` start/pause and `Ctrl+Alt+O` open. Skip is disabled for new installations; existing shortcuts are preserved. Record, edit, or disable shortcuts in Settings.

State is saved on transitions and once a minute while running, with periodic checkpoints coalesced while a save is pending. Normal exit saves the session paused; crash recovery restores the last durable checkpoint paused without crediting unsaved time. A database write failure pauses activity and queues accepted changes for retry. Use **Retry saving**, then explicitly resume after recovery.

## Build and test

Requires stable Rust MSVC and Visual Studio C++ Build Tools with the Windows SDK. Tested with Rust 1.98.1. Python 3.11+ is used by the packaging and audit scripts; resource monitoring also requires `psutil`.

```powershell
git clone https://github.com/amitzore71/orbit.git
cd orbit
cargo test --workspace --locked
cargo build --release --locked
.\target\release\orbit.exe
```

The default build uses Slint's software renderer and accessibility support. The MSVC CRT is linked statically; no Visual C++ redistributable is required. The release profile optimizes for size with LTO. Optional comparisons: `--features gpu` for FemtoVG/OpenGL, or `--profile release-speed` for speed optimization.

## Package

The default release keeps the executable uncompressed for fast startup and low private memory. ZIP and installer compression still reduce download size. Optional executable compression requires UPX 5.2.1 and `-CompressExecutable`; it increases startup work and runtime private memory.

```powershell
.\scripts\Build-Release.ps1 -InnoCompiler "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

Requires Inno Setup 6.7.3 or newer. Use `-SkipInstaller` for a portable ZIP only. Outputs go to `dist`. Dependency notices are assembled from the Cargo registry and committed upstream license texts.

Packaging rejects an EXE over 12,000,000 bytes and verifies native imports, Windows relocations, ASLR/NX flags, executable identity, and ZIP contents. Optional UPX builds additionally verify decompression and restored runtime sections. Use `orbit.exe` inside the versioned `dist/Orbit-*-windows-x64` directory for the portable build. Files under `target/debug` are development builds.

The release script tests startup, legacy data preservation, real timer expiry, paused restart, and write-failure recovery against the staged executable before creating the installer. `dist/SHA256SUMS.txt` identifies the final installer and portable ZIP.

## Verification

[Validation](docs/VALIDATION.md) lists automated checks and manual desktop scenarios. [Performance](docs/PERFORMANCE.md) records footprint and task-selection benchmarks; [stability](docs/STABILITY.md) records fault injection and native resource stress. Results identify the binaries measured and are not guarantees for later builds.

## Source layout

- `crates/focus-core`: deterministic timer with an injected clock.
- `crates/storage`: SQLite worker, bounded persistence queue, reports.
- `crates/platform-win`: Windows tray, shortcuts, audio, notifications, and window integration.
- `crates/app`: shared state, Slint interface, and widgets.

## Licenses

Application code is [MIT licensed](LICENSE). Packaged dependency license texts are in `THIRD-PARTY-NOTICES.txt`.

The About page displays the standard AboutSlint attribution widget. Slint is distributed under its Royalty-free Desktop, Mobile, and Web Applications License 2.0; its license text is included in the dependency notices.

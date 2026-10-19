# Orbit

An offline Windows focus timer built with Rust and Slint. Includes tasks, focus reports, and three desktop widgets. No account, telemetry, or background service.

## Run

Requires Windows 11 22H2 or newer, x64. Open `orbit.exe` from a portable package or use the per-user installer. Closing the window hides Orbit in the tray; right-click the tray icon to control or quit it.

Data is stored in `%LOCALAPPDATA%\Orbit\orbit.db`. Use `orbit.exe --data-dir "D:\FocusData"` for another location. Existing pre-Orbit databases in `%LOCALAPPDATA%\StillPomodoro\still.db`, or `still.db` in a custom directory, are reused in place. Uninstalling retains your data. Back up the data directory while Orbit is closed.

For portable notifications, open **Settings → Notifications** to register the current executable. Repeat after moving it. The installer handles registration automatically. If Windows notifications are unavailable, Orbit uses a tray alert.

## Use

- **Timer:** Focus, short break, and long break. Defaults are 25/5/15 minutes, with a long break every four completed focus sessions. Sleep and hibernation do not consume time. Automatic starts are off by default.
- **Tasks:** Add a title and a 1–99 session estimate. Selecting a task affects the next session; an ongoing session keeps its original task. Only finished focus sessions increase completion counts. Skips and resets retain partial focus time. Deleting a task preserves its history.
- **Reports:** Daily, Monday-based weekly, and calendar-month totals. Focus by task shows time spent, percentage of total focus, and completed sessions, sorted by time spent. Includes partial focus time, deleted task history, and time without a task; excludes pauses and sleep, and uses the current Windows time zone.
- **Widgets:** Timer ring, today's tasks, and mini timer. Drag a header to move a widget. Configure layering and click-through in Widgets; use **Unlock all widgets** in the tray to recover click-through windows.
- **Settings:** Session lengths, automatic starts, alarms, volume, notifications, appearance, shortcuts, and opt-in startup. Duration changes apply to the next session.

Default shortcuts: `Ctrl+Alt+P` start/pause, `Ctrl+Alt+O` open, `Ctrl+Alt+N` skip. Change or disable them in Settings.

State is saved on transitions. Normal exit saves the session paused; crash recovery restores the last checkpoint paused without crediting unsaved time. A database write failure pauses activity and queues accepted changes for retry. Use a timer control to retry saving, then resume after recovery.

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

Install [UPX 5.2.1](https://github.com/upx/upx/releases/tag/v5.2.1) on PATH, or pass its executable with `-UpxPath`. The compact release uses executable compression; ordinary Cargo builds remain uncompressed.

```powershell
.\scripts\Build-Release.ps1 -UpxPath "C:\Tools\upx.exe" -InnoCompiler "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

Requires Inno Setup 6.7.3 or newer. Use `-SkipInstaller` for a portable ZIP only. Outputs go to `dist`. Dependency notices are assembled from the Cargo registry and committed upstream license texts.

Packaging rejects an EXE over 10,000,000 bytes and verifies the compression checksum, native imports, Windows relocations, ASLR/NX flags, runtime code, and ZIP contents. Use `orbit.exe` inside `dist/Orbit-0.1.0-windows-x64` for the compact build. Files under `target/debug` are development builds.

## Verification

[Validation](docs/VALIDATION.md) lists automated checks and manual desktop scenarios. [Performance](docs/PERFORMANCE.md) records footprint and task-selection benchmarks; [stability](docs/STABILITY.md) records fault injection and native resource stress. Results identify the binaries measured and are not guarantees for later builds.

## Source layout

- `crates/focus-core`: deterministic timer with an injected clock.
- `crates/storage`: SQLite worker, bounded persistence queue, reports.
- `crates/platform-win`: Windows tray, shortcuts, audio, notifications, and window integration.
- `crates/app`: shared state, Slint interface, and widgets.

## Licenses

Application code is [MIT licensed](LICENSE). Packaged dependency license texts are in `THIRD-PARTY-NOTICES.txt`.

**Slint attribution is unresolved for distribution.** The current interface omits AboutSlint. Before distributing a build under Slint's Royalty-free License 2.0, provide the required public attribution badge or an alternative license. See the [Slint license](https://slint.dev/agreements/slint-royalty-free-license.pdf).

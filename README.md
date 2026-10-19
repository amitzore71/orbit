# Orbit

A local, native Windows focus timer written in Rust and Slint. One process, three optional desktop widgets, no account, network calls, browser runtime, telemetry, or background service.

## Run

Windows 11 22H2 (build 22621) or newer, x64. Open `orbit.exe` from the portable ZIP, or run the per-user installer. Closing the main window hides it in the tray. Right-click the coral ring in the tray to open, control, or quit Orbit and recover click-through widgets.

The portable build uses `%LOCALAPPDATA%\Orbit\orbit.db`. It needs no Visual C++ redistributable. To keep data elsewhere, run `orbit.exe --data-dir "D:\FocusData"`. Only one ordinary instance runs per user session. Existing pre-Orbit databases in `%LOCALAPPDATA%\StillPomodoro\still.db` are reused in place; an existing `still.db` in a custom data directory is also preserved.

For notification activation in portable mode, open **Settings → Set up notifications**. This explicitly creates a Start menu shortcut and a local `orbit-pomodoro:` protocol registration pointing at the current executable. Repeat after moving the executable. The installer performs this setup automatically. If Windows notifications are unavailable, Orbit uses a tray alert; the selected alarm remains independent of notification settings.

## Use

- **Timer:** Focus, short break, long break; start/pause, skip, and reset. Defaults are 25/5/15 minutes, with a long break after every four completed focus sessions. Sleep and hibernation do not consume the timer. Automatic starts are off by default.
- **Tasks:** Add or edit a title and a 1–99 pomodoro estimate. Select the task for the next session. An ongoing session retains its original task. Completion counts only increase when a focus timer finishes; skips and resets retain partial focus time without completion credit. Deleting a task preserves its historical focus data.
- **Reports:** Daily, Monday-based weekly, and calendar-month views, with previous/next controls. Focus time includes partial sessions and excludes pauses and sleep. Reports split intervals across local midnight and use your current Windows time zone. They refresh on session changes and when opened, rather than querying SQLite every second.
- **Widgets:** Timer ring, today's tasks/progress, and a mini timer. All share the same timer. Drag a widget's header to move it; choose Bottom, Normal, or Always on top, enable click-through, or reset its position. Mini defaults to always on top. These are ordinary windows; they are not attached behind desktop icons.
- **Settings:** Durations, long-break interval, automatic starts, three soft alarms, volume, notifications, reduced motion, optional Mica, shortcuts, and opt-in startup in the tray. Duration changes take effect when a new session starts.

Default shortcuts are `Ctrl+Alt+P` (start/pause), `Ctrl+Alt+O` (open), and `Ctrl+Alt+N` (skip). Each can be changed or disabled. Conflicts with other applications are reported.

State is saved on transitions, not on each display tick. A normal exit saves the current session paused. After a crash, Orbit restores the last checkpoint paused and explains that recovery is approximate: running time not committed before interruption is not credited. If a database write fails, activity pauses and accepted changes remain in a bounded retry queue. Retry a timer control to retry saving, then resume after saving recovers. Back up the data directory while Orbit is closed. Uninstalling retains your focus history.

## Build and test

Install the stable Rust MSVC toolchain and Visual Studio C++ Build Tools with the Windows SDK. Tested with Rust 1.98.1. `Cargo.lock` is included for repeatable dependency resolution. First build requires crates.io access; subsequent locked builds can use the local Cargo cache.

```powershell
git clone https://github.com/amitzore71/orbit.git
cd orbit
cargo test --workspace --locked
cargo build --release --locked
.\target\release\orbit.exe
```

The default release enables only Slint's Winit/software renderer and accessibility. `.cargo/config.toml` statically links the MSVC CRT. The release profile uses size optimization, LTO, one codegen unit, stripped symbols, and abort-on-panic. Build-time image/compiler dependencies do not imply matching runtime features.

An optional GPU comparison build is available with `cargo build --release --features gpu`. It selects FemtoVG/OpenGL; it is not the default and must be measured separately. `cargo build --profile release-speed` compares speed optimization against the size profile.

## Package

Install Inno Setup 6.7.3 or newer for the installer, then run:

```powershell
.\scripts\Build-Release.ps1 -InnoCompiler "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
```

Use `-SkipInstaller` for just the portable ZIP. Outputs are in `dist`. Python 3 assembles exact dependency notices from the local registry and the committed upstream-license cache. No license downloads are needed during ordinary packaging. Inno Setup is a build tool, not a runtime dependency; its own licensing applies to its use.

## Performance and verification

See [the performance report](docs/PERFORMANCE.md) and [validation notes](docs/VALIDATION.md). See [the memory and stability audit](docs/STABILITY.md) for leak checks, fault injection, native resource stress, and report-query benchmarks.

The latest [task-selection and display-scheduling optimization](docs/UI-OPTIMIZATION.md) documents model reuse, reduced display wakeups, and measured before/after results.

To repeat the six footprint scenarios:

```powershell
.\scripts\Measure-Footprint.ps1 -WarmupSeconds 10 -SampleSeconds 60
```

The measurement script creates isolated data under `artifacts`, opens temporary app windows where needed, and lets each process exit normally. Diagnostic flags are `--isolated`, `--tray`, `--all-widgets`, `--running`, `--minimized`, and `--quit-after <seconds>`; use them with a separate `--data-dir` for experiments. No working-set trimming or external-runtime exclusion is used to improve the reported numbers.

## Source layout

`focus-core` is a deterministic timer with an injected awake/wall clock. `storage` owns SQLite on one blocking worker with a bounded queue and a 1 MiB page cache. `platform-win` wraps inbox Windows APIs. `app` owns the shared state and lazily creates Slint widget windows. When all timer surfaces are hidden, only the completion deadline is scheduled; visible running timers update once per second.

## Licenses

Application code is MIT licensed. Slint currently uses its **Royalty-free Desktop, Mobile, and Web Applications License 2.0**, which requires either its AboutSlint widget in the application or its attribution badge on a public webpage. The large in-app panel has been removed from the working source at the user's request. **Before distributing a new build, confirm the public attribution page or an alternative Slint license that permits this change.** Existing ZIP/installer artifacts have not been replaced by this source edit. The packaged `THIRD-PARTY-NOTICES.txt` retains dependency license texts and copyright notices. See the [Slint license](https://slint.dev/agreements/slint-royalty-free-license.pdf).

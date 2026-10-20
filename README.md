# Orbit

A private, offline Windows focus workspace built with **Rust, Next.js, and Tauri 2**. Choose today's work, give it your attention, and see the quiet progress that adds up.

Version 0.3 introduces an orbital timer, a shared Focus / Today workspace, a complete light/dark interface, and purposeful motion across navigation and session changes. Existing tasks, session history, schema 4 databases, backups, shortcuts, and desktop widgets are preserved.

## Run

Requires Windows 11 22H2 or newer, x64, and Microsoft Edge WebView2. Use the portable ZIP or the per-user installer. Closing the main window hides Orbit to the tray; use the tray menu to open, control, or quit it.

The installer checks WebView2 and downloads Microsoft's Evergreen bootstrapper only if needed. For offline setup, download the official [Evergreen standalone x64 installer](https://developer.microsoft.com/microsoft-edge/webview2/), rename it `MicrosoftEdgeWebView2RuntimeInstallerX64.exe`, and place it beside Orbit's installer. Portable users should install WebView2 first. Orbit itself needs no internet connection, account, telemetry, or background service.

## Your work

- **Focus:** timer and Today together. Focus without a task or choose one; a started session keeps its original task while another task can be selected for next time.
- **Tasks:** Today, All active, Completed, Trash, Unicode search, manual ordering, carry-forward, editing, and recoverable deletion with Undo. Focus completion does not automatically complete a task.
- **Insights:** daily, weekly, monthly, and custom reports up to 367 days; task/session details; CSV exports that preserve Unicode and neutralize spreadsheet formulas.
- **Settings:** session durations, automatic starts, sound, appearance, shortcuts, widgets, backups, and restore. Drafts survive navigation; Save and Discard are explicit.
- **Widgets:** timer ring, Today's tasks, and mini timer. Drag their headers; layering and click-through are under Advanced controls. Use **Unlock all widgets** in the tray to recover click-through windows.

Defaults are 25/5/15 minutes, a long break every four completed focus sessions, and no automatic starts. Sleep does not count as focus. Global shortcuts remain `Ctrl+Alt+P` for start/pause and `Ctrl+Alt+O` to open Orbit; Skip is disabled by default.

## Your data

The database remains `%LOCALAPPDATA%\Orbit\orbit.db`. `orbit.exe --data-dir "D:\FocusData"` selects another location. Legacy `%LOCALAPPDATA%\StillPomodoro\still.db`, or `still.db` in a custom directory, is reused in place. Uninstalling retains your data. WebView cache files are stored separately under the data directory's `webview` folder.

**Settings → Data** creates consistent backups or restores them after an explicit confirmation and a pre-restore recovery copy. Backups and restoration pause the timer. CSV exports are not restorable backups. Existing schema-4 data needs no new migration in 0.3; older schema upgrades still create an automatic backup first.

State is persisted on transitions and once a minute during running sessions. A write failure pauses activity and retains accepted changes. Choose **Retry saving**, then explicitly resume. Normal exit saves paused; crash recovery restores the last durable checkpoint paused without crediting time spent away.

## Develop

Requires Node.js 22+, pnpm 10, stable Rust MSVC, Visual Studio C++ Build Tools, and the Windows SDK.

```powershell
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend dev
```

In a second terminal:

```powershell
cargo run -p orbit --no-default-features --locked
```

Production builds contain the static frontend and do not launch a Node server:

```powershell
pnpm --dir frontend build
cargo build --release --locked
.\target\release\orbit.exe
```

Checks:

```powershell
cargo test --workspace --locked
pnpm --dir frontend typecheck
pnpm --dir frontend test
pnpm --dir frontend exec playwright install chromium
pnpm --dir frontend test:e2e
node frontend/tests/native-ui.mjs
```

## Package

```powershell
.\scripts\Build-Release.ps1
```

Requires Python 3.11+ with `psutil` and Inno Setup 6.7.3+. Use `-InnoCompiler` for a custom compiler or `-SkipInstaller` for the portable ZIP. The script builds the frontend and Rust host, checks persistence/legacy data, assembles dependency/font notices, measures the full process tree and interactive startup, verifies the archive, and writes SHA-256 sums to `dist`.

See [desktop architecture](docs/DESKTOP-ARCHITECTURE.md), [release validation](docs/RELEASE-VALIDATION.md), and [performance measurements](docs/PERFORMANCE.md) for state ownership, checks, measured tradeoffs, and remaining manual acceptance work. Historical 0.1/0.2 results remain available in the other documents.

## License

Orbit is [MIT licensed](LICENSE). Tauri, Next.js, React, Motion, Phosphor icons, Geist fonts, and transitive dependencies retain their own licenses in the packaged `THIRD-PARTY-NOTICES.txt`. WebView2 is a separately installed Microsoft runtime.

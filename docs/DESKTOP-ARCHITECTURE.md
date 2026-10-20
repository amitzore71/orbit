# Orbit 0.3 desktop architecture

Orbit is a Windows 11 x64 application. The UI is a Next.js static export hosted by Tauri 2 / WebView2. No Next.js server, Node runtime, account, or network backend ships in the application.

## Ownership

- `focus-core` owns deterministic timing, task attribution, sleep handling, and checkpoints. The Windows host injects its awake clock.
- `focus-storage` owns SQLite schema 4, a bounded transaction/retry worker, reports, CSV, and verified snapshots. `Reload` returns durable state after recovery; it does not change the schema.
- `orbit-runtime` owns a serialized application inbox, task state, timer transitions, widget preferences, durable mutation acknowledgements, and lifecycle scheduling. Normal work is capped at 128 queued requests, with two additional slots for lifecycle/exit requests. After bootstrap, UI threads do not wait on SQLite; the worker and runtime perform persistence work off the UI thread.
- `platform-win` retains the Windows tray, native shortcut registration, audio, notifications, sign-in registration, monitor positioning, and window subclass integration.
- The Tauri host owns window handles, native file dialogs, appearance reconciliation, and IPC authorization. Window subclass installation/removal and widget creation run on the UI thread. The Windows manifest explicitly requests Common Controls v6.
- The frontend owns navigation, search/filter presentation, settings drafts, transient notices, and visual motion. A virtualized task list renders at most 22 rows, keyed by stable IDs.

## IPC and recovery

`orbit_command` accepts the tagged Rust/TypeScript command union. Task identifiers cross the boundary as decimal strings. Replies resolve after a durable write, or return an error with `accepted`: an accepted write failure is retained by storage and must be retried through **Retry saving**, not resubmitted as a new mutation.

`orbit://timer` carries timer-only snapshots. `orbit://changed` carries a revision and affected domains. Clients subscribe before fetching initial state, coalesce snapshot requests, and reject older revisions. Reports independently discard superseded queries. Timer display interpolation cannot write focus time or finish a session.

The runtime wakes for expiry, calendar reconciliation, and one-minute running checkpoints. One-second display updates are enabled only for visible, non-minimized timer surfaces. Hidden decorative animation is paused. A background deadline and Windows awake clock remain authoritative even when a webview is throttled.

Generic web IPC rejects backup, restore, export paths, and process termination. Main-window file commands use native dialogs; restoration consumes the host's selected path after a separate explicit confirmation. Widgets have timer/task access but cannot modify settings, invoke file operations, or quit the host. Native validation remains authoritative.

On a database write failure, activity pauses and the final paused checkpoint is retained. Retry flushes accepted transactions, reloads durable settings/tasks/widgets, and restores the timer paused. Recovery never automatically resumes focus.

## Windows compatibility and data

The installer AppId, `Orbit.Pomodoro.Desktop` notification identity, installation directory, sign-in preference, and existing single-instance mutex are retained. `%LOCALAPPDATA%\Orbit\orbit.db`, legacy `StillPomodoro\still.db`, and `--data-dir` resolve through the existing Windows path resolver. WebView caches live under the resolved data directory's `webview` folder, keeping isolated test instances separate.

The installer checks both user and machine WebView2 registrations. When needed, it downloads Microsoft's Evergreen bootstrapper. For disconnected installation, place the official Evergreen standalone x64 installer, named `MicrosoftEdgeWebView2RuntimeInstallerX64.exe`, beside the Orbit installer. Portable users must install WebView2 separately. Orbit itself operates offline.

Microsoft runtime distribution guidance: https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution

## Verification and performance

- Rust: `cargo test --workspace --locked`.
- Frontend contracts: `pnpm --dir frontend test`; TypeScript: `pnpm --dir frontend typecheck`.
- Browser journeys and screenshots: `pnpm --dir frontend test:e2e`. The injected adapter is available only in development/test builds; packaged production builds always use the native bridge.
- Real native UI: `node frontend/tests/native-ui.mjs`. Runs an isolated database, attaches Playwright to a test-only WebView2 debugging port, verifies real Rust state and all widgets, then exits. Remote debugging is never enabled by Orbit itself.
- Process tree: `python scripts/measure-desktop.py target/release/orbit.exe`. `--hidden-soak` samples a running tray session for ten minutes.
- Startup: `python scripts/measure-window-startup.py --executable target/release/orbit.exe`. Measures an opt-in local startup probe after React commits the initial Rust snapshot. It is not physical display latency or cold-boot time.

Acceptance targets: warm interactive startup p95 ≤1.5 seconds; feedback ≤100 ms; frame time p95 ≤20 ms at 60 Hz; hidden CPU <0.5% of one logical core over ten minutes; combined private memory ≤300 MiB main-only and ≤450 MiB with all widgets. Measurements must identify their executable hash and scope. A short CPU sample cannot establish the ten-minute target, and browser-emulated DPI does not establish physical multi-monitor behavior.

Historical 0.1/0.2 Slint measurements remain in the other performance/product documents. They do not describe the current rendering stack.

# Orbit 0.3.0 validation — 21 October 2026

Orbit now uses a static Next.js frontend inside Tauri 2. The deterministic timer, SQLite schema 4, legacy database discovery, native tray/shortcuts, and installer identity remain in Rust. The exact packaged binary and archive hashes are recorded in `build-info.json` and `dist/SHA256SUMS.txt`.

## Automated coverage

- Rust workspace: **55 passed, 3 intentionally ignored**; formatting passed. The ignored checks require an interactive/audio environment or opt-in report benchmarking. This release does not claim a new Clippy or Application Verifier run.
- Frontend: **4 contract tests and 9 browser journeys passed**. Journeys cover task creation and binding, Trash/Undo, settings drafts and late acknowledgements, report detail/CSV, 5,000-task virtualization, storage recovery, modal keyboard containment, long-error toast bounds, compact layouts, light/dark/high contrast, reduced motion, and emulated 100–200% scaling.
- Runtime tests cover serialized concurrent commands, string-encoded large identifiers, durable retry and accepted-error semantics, task attribution, paused restoration, backups, widget recovery, and reserved lifecycle queue capacity.
- Native WebView2 integration exercises the actual Rust IPC, Unicode tasks, all three widgets, 50 widget recreation cycles, close/reopen, settings, appearance, and rejection of renderer-supplied file paths. Production assets do not contain the injected mock adapter. A fixed-400-ms navigation probe initially failed while several WebViews started concurrently; the unchanged product passed on rerun, and a bounded asynchronous-commit assertion passed in the final 50-cycle run.
- Packaged process checks exercise legacy database reuse/schema-2 migration, a real one-minute session, injected SQLite failure and retry, abrupt termination, and recovery from a periodic durable checkpoint. Integrity and duplicate-credit assertions pass.
- Package verification checks x64 PE identity, ASLR/NX and relocations, external Visual C++ runtime imports, embedded static assets, matching measurement hashes, and ZIP/staging equivalence. Dependency and font notices are included.

## Acceptance limits

The full-motion design is retained at the user's request, accepting the main-window private-memory target miss. See [PERFORMANCE.md](PERFORMANCE.md) for actual process-tree memory, background CPU, startup, interaction, and frame measurements. Performance targets are acceptance goals, not universal guarantees.

The browser scaling checks are emulation; native checks use this machine's current Windows display setup. Physical monitor migration, every Windows high-contrast configuration, real suspend/resume hardware, clean-machine WebView2 prerequisite installation, and the administrative Application Verifier suite remain manual acceptance work. Installer compilation and prerequisite detection are checked; the installer was not installed over the user's existing application. Tests use isolated databases and do not modify the user's Orbit data.

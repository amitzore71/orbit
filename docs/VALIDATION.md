# Validation

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --workspace --features diagnostics --locked
cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings
```

Core tests cover timer transitions, delayed expiry, sleep exclusion, task binding, restart recovery, and randomized command sequences. Storage tests cover schema 1–4 migrations, Today planning and ordering, task restore, Unicode search, periodic checkpoint durability, snapshots and restore validation, CSV export, calendar and per-task reports, deleted and unassigned task history, report boundary clipping, transaction rollback, and bounded write-failure recovery. App tests cover ring geometry, task-model selection/reuse, and the product workflows. Windows tests cover clocks and shortcuts.

Native resource tests are opt-in: `cargo test -p platform-win --locked -- --ignored --test-threads=1 --nocapture`. See [the stability audit](STABILITY.md) for fault injection, resource measurements, and Application Verifier commands.

## UI and integration checks

```powershell
cargo build --features diagnostics --locked
python scripts/ui-audit.py
python scripts/integration-check.py target/debug/orbit.exe
# After a release build: python scripts/integration-failure-check.py
```

The UI audit captures 34 images at each of 100%, 125%, 150%, and 200% scale: six pages at regular and compact sizes, light and dark timer/task/report/settings states, Trash, Carry forward, Data, Appearance, report-task detail, the storage-blocked timer, a recovery notice, four high-contrast states, and three widgets plus their dark faces. Fixtures include long and multilingual task titles, Today and earlier plans, deleted and unassigned task history, and non-empty reports. Images and manifests go to ignored `artifacts/ui-audit-<timestamp>` directories. Inspect the images; dimension checks alone do not validate layout.

The scale override and synthetic high-contrast tokens do not test real monitor transitions or Windows theme changes. Diagnostic code and PNG dependencies are excluded from ordinary builds.

Integration scripts use isolated databases to check real expiry, paused restart recovery, SQLite write-failure recovery, a real one-minute checkpoint recovered after abrupt termination, and database integrity. The window-startup probe measures launch to an externally observed painted and responsive window. Historical measurements and executable hashes are retained in [PERFORMANCE.md](PERFORMANCE.md) and [STABILITY.md](STABILITY.md); they do not certify later builds.

## Manual desktop checks

Use a separate data directory and check:

- Start, pause, reset, skip, mode changes, and keyboard activation. Add, edit, select, complete, and delete a Unicode task; navigate reports and save settings.
- Drag the main window and each widget repeatedly, release the mouse over a control and outside the window, then verify navigation, start/pause, and task selection still respond. A header click without movement and a right-click must not enter a window move. WindowMoveArea routes moves through Slint/Winit so the matching release reaches the UI.
- Widget movement, close/reopen, saved placement, layering, click-through, and tray recovery. Test conflicting shortcuts, notification activation, and muted or unavailable audio.
- Actual Windows high contrast and reduced motion; sleep/hibernate and resume; Explorer restart; mixed-DPI monitors, monitor removal, and time-zone/DST changes.
- A clean Windows 11 22H2 machine and the complete install/uninstall flow, including upgrades and retained user data.

The rendered audit does not cover these shell and hardware scenarios.

## Distribution checks

The release script defaults to an uncompressed executable and enforces a 12,000,000-byte EXE limit. Staging must match the built executable, retain Windows relocations and ASLR/NX flags, and have no external Visual C++ runtime dependency. ZIP contents must match staging. Optional `-CompressExecutable` requires UPX 5.2.1 and also verifies checksum and restored runtime sections.

Validate the actual packaged executable with:

```powershell
python scripts/integration-check.py dist/Orbit-0.2.0-windows-x64/orbit.exe
python scripts/integration-failure-check.py dist/Orbit-0.2.0-windows-x64/orbit.exe
python scripts/checkpoint-recovery-check.py dist/Orbit-0.2.0-windows-x64/orbit.exe
python scripts/branding-check.py dist/Orbit-0.2.0-windows-x64/orbit.exe
python scripts/measure-window-startup.py --executable dist/Orbit-0.2.0-windows-x64/orbit.exe --output docs/release-window-startup.json
```

Verify that the ZIP and installer contain the same executable and dependency notices, PE imports have no Visual C++ runtime DLL dependency, uninstall retains user data, and the About page displays the AboutSlint widget. Build metadata identifies the last packaged executable, not necessarily the current source. Sign only with a valid signing certificate.

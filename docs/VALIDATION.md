# Validation

## Automated checks

```powershell
cargo fmt --all -- --check
cargo test --workspace --features diagnostics --locked
cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings
```

Core tests cover timer transitions, delayed expiry, sleep exclusion, task binding, restart recovery, and randomized command sequences. Storage tests cover migrations, Unicode tasks, checkpoint durability, calendar reports, transaction rollback, and bounded write-failure recovery. App tests cover ring geometry and task-model selection/reuse. Windows tests cover clocks and shortcuts.

Native resource tests are opt-in: `cargo test -p platform-win --locked -- --ignored --test-threads=1 --nocapture`. See [the stability audit](STABILITY.md) for fault injection, resource measurements, and Application Verifier commands.

## UI and integration checks

```powershell
cargo build --features diagnostics --locked
python scripts/ui-audit.py
python scripts/integration-check.py target/debug/orbit.exe
# After a release build: python scripts/integration-failure-check.py
```

The UI audit captures six pages at regular and compact sizes, a recovery notice, three high-contrast states, and three widgets: 19 images at each of 100%, 125%, 150%, and 200% scale. Fixtures include long and multilingual task titles and non-empty reports. Images and manifests go to ignored `artifacts/ui-audit-<timestamp>` directories. Inspect the images; dimension checks alone do not validate layout.

The scale override and synthetic high-contrast tokens do not test real monitor transitions or Windows theme changes. Diagnostic code and PNG dependencies are excluded from ordinary builds.

Integration scripts use isolated databases to check real expiry, paused restart recovery, SQLite write-failure recovery, and database integrity. Historical measurements and executable hashes are retained in [PERFORMANCE.md](PERFORMANCE.md) and [STABILITY.md](STABILITY.md); they do not certify later builds.

## Manual desktop checks

Use a separate data directory and check:

- Start, pause, reset, skip, mode changes, and keyboard activation. Add, edit, select, complete, and delete a Unicode task; navigate reports and save settings.
- Widget movement, close/reopen, saved placement, layering, click-through, and tray recovery. Test conflicting shortcuts, notification activation, and muted or unavailable audio.
- Actual Windows high contrast and reduced motion; sleep/hibernate and resume; Explorer restart; mixed-DPI monitors, monitor removal, and time-zone/DST changes.
- A clean Windows 11 22H2 machine and the complete install/uninstall flow, including upgrades and retained user data.

The rendered audit does not cover these shell and hardware scenarios.

## Distribution checks

The release script requires UPX 5.2.1. It compresses the staged copy, enforces a 10,000,000-byte EXE limit, verifies its checksum, and checks restored runtime sections against the uncompressed build. Both binaries must retain Windows relocations and ASLR/NX flags. Native dependency checks inspect the uncompressed PE, and ZIP contents must match staging.

Validate the actual packaged executable with:

```powershell
python scripts/integration-check.py dist/Orbit-0.1.0-windows-x64/orbit.exe
python scripts/integration-failure-check.py dist/Orbit-0.1.0-windows-x64/orbit.exe
python scripts/branding-check.py dist/Orbit-0.1.0-windows-x64/orbit.exe
```

Verify that the ZIP and installer contain the same executable and dependency notices, PE imports have no Visual C++ runtime DLL dependency, and uninstall retains user data. Build metadata identifies the last packaged executable, not necessarily the current source. Resolve the Slint attribution requirement described in the [README](../README.md#licenses) before distribution. Sign only with a valid signing certificate.

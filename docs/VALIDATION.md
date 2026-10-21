# Validation

Build the static frontend before checking the desktop host:

```powershell
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend build
cargo fmt --all -- --check
cargo test --workspace --locked
pnpm --dir frontend typecheck
pnpm --dir frontend test
pnpm --dir frontend exec playwright install chromium
pnpm --dir frontend test:e2e
cargo build --release --locked
node frontend/tests/native-ui.mjs
```

Rust tests cover timer transitions, sleep exclusion, task attribution, serialized commands, migrations, reports, CSV safety, backups, durable retries, and paused recovery. Browser tests exercise the mocked IPC contract, settings drafts, task workflows, report detail, 5,000-task virtualization, keyboard focus, toast bounds, compact layouts, themes, reduced motion, and emulated scaling. The native driver uses real WebView2 and Rust with an isolated database; its debugging port is enabled only by the test harness.

## Packaged integration

```powershell
python scripts/branding-check.py target/release/orbit.exe
python scripts/integration-check.py target/release/orbit.exe
python scripts/integration-failure-check.py target/release/orbit.exe
python scripts/checkpoint-recovery-check.py target/release/orbit.exe
python scripts/stress-check.py --cycles 50
```

These checks cover legacy database reuse, real one-minute expiry, write failures, abrupt termination, paused checkpoint recovery, and widget recreation. Outputs go to ignored `artifacts/` directories. [Release validation](RELEASE-VALIDATION.md) records what was actually run for 0.3.0.

## Performance and packaging

```powershell
python scripts/measure-window-startup.py --executable target/release/orbit.exe --trials 30
python scripts/measure-desktop.py target/release/orbit.exe --warmup 30 --seconds 30
python scripts/measure-desktop.py target/release/orbit.exe --hidden-soak
.\scripts\Build-Release.ps1
```

Measure the complete Orbit/WebView2 process tree. The startup probe ends when React commits its initial native snapshot; it does not measure physical display latency. Inspect screenshots as well as their bounds. See [performance evidence and limits](PERFORMANCE.md).

Release verification checks the uncompressed executable's 50 MB ceiling, x64 PE identity, ASLR/NX and relocations, external Visual C++ runtime imports, matching measurement hashes, notices, and ZIP/staging equivalence.

## Manual Windows checks

Use an isolated data directory for the following:

- Drag the main window and every widget, then use their controls. Check saved positions, layering, click-through, and tray recovery.
- Test conflicting shortcuts, notification activation, muted or unavailable audio, and Explorer restart.
- Exercise real Windows high contrast and reduced motion, sleep/hibernate, mixed-DPI monitors, monitor removal, and time-zone/DST changes.
- Check clean-machine setup, WebView2 installation and offline prerequisites, upgrades, uninstall, and retained user data.

Native resource tests remain opt-in: `cargo test -p platform-win --locked -- --ignored --test-threads=1 --nocapture`. Administrative heap/handle checks use `scripts/Test-AppVerifier.ps1` with Microsoft Application Verifier installed. Browser emulation does not certify these hardware and shell scenarios.

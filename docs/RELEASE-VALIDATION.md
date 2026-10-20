# Orbit 0.2.0 validation — 20 October 2026

The packaged executable is **11,670,016 bytes** with SHA-256 `AB091AADEC257C60DCD0A16D6D4E98276E3851034EA2EF5979664C0AD1BB513B`. `dist/SHA256SUMS.txt` identifies the final portable ZIP and installer. `verify-release.py` confirms that ZIP contents match staging, the PE is x64 with ASLR/NX, relocations are retained, imports have no external Visual C++ runtime, and the executable identity matches [build-info.json](build-info.json).

## Automated validation

- `cargo fmt --all -- --check` passed.
- `cargo test --workspace --features diagnostics --locked` passed: 55 tests, 0 failed, 3 ignored. The ignored tests are the opt-in 100,000-interval report benchmark and two native tests that require an interactive desktop or real audio driver.
- `cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings` passed.
- The release script reran the workspace tests without diagnostics and built the release profile with the same result.
- `scripts/ui-audit.py` captured **34 images at each of 100%, 125%, 150%, and 200% scale** (136 images total) for the diagnostics executable. The retained run is `artifacts/ui-audit-1792499908125501200`; the captured executable hash is `721001CC7C38284A1AE153AB721C40D8A650D3CBB15CB6E0B5A0DE20A01ABBFA`. The audit covers regular and compact pages, light and dark themes, Trash, Carry forward, Data, Appearance, report-task detail, the storage-blocked timer, a recovery notice, high-contrast states, and all widgets. Dimension and capture-count assertions passed; the images were inspected visually.

## Packaged process checks

All checks ran against the staged 0.2.0 executable and wrote their JSON results into the package's `docs` directory.

- Branding and migration: canonical database creation, legacy database reuse in place, schema-2 upgrade history preservation, Unicode task preservation, all-window startup, and SQLite integrity passed. Schema 4 migration retains old task IDs, completed sessions, partial time, and deleted-task history; existing tasks remain unplanned.
- Timer integration: a real 60-second focus session completed exactly once with 60,000 ms credited, followed by a paused restart.
- Failure injection: a SQLite write failure paused the timer; recovery preserved the checkpoint and integrity check passed after abrupt termination.
- Periodic checkpoint: a durable one-minute checkpoint was observed, then the process was terminated abruptly. Recovery restored 59,999 ms paused with no duplicate credit and `ok` integrity.
- Backup and restore: automated product tests cover backup/restore round trips, pre-restore preservation, schema compatibility rejection, and retention of this machine's startup choice. Restore leaves the timer paused.
- Report export: automated tests cover custom ranges, session detail, CSV escaping, and formula-safe task titles.

## Performance evidence

Measurements identify the same packaged executable:

| Measurement | Result |
|---|---:|
| Warm tray launch to native input-idle | 22.80 ms median |
| Warm launch to painted, responsive window | 133.98 ms median, 141.27 ms p95 (30 trials) |
| Tray idle private commit / working set | 3.43 MiB / 20.41 MiB |
| Main window running private commit / working set | 10.13 MiB / 28.39 MiB |
| All widgets running private commit / working set | 12.25 MiB / 30.91 MiB |
| Packaged executable | 11,670,016 bytes (limit 12,000,000) |

The 0.2.0 build passes the warm usable-window, empty-fixture memory, and package-size gates. The retained input-idle run is effectively at the 0.1.1 baseline (22.80 ms versus 23.54 ms); an earlier run of the same executable measured 39.35 ms, so treat this probe as machine-state sensitive. Cold launch, 5,000-task search, input-to-feedback and navigation latency, frame timings, a 10-minute tray CPU sample, and the 500-cycle lifecycle soak are not measured for this release.

## Outstanding manual checks

Physical mouse/keyboard journeys, real Windows high contrast and reduced motion, actual sleep/hibernate, Explorer restart, mixed-DPI monitor changes, notification fallback, conflicting shortcuts, cold-cache startup, and the complete interactive install/upgrade/uninstall flow still require the checks in [VALIDATION.md](VALIDATION.md). The five-user acceptance targets in the product plan were not measured. The earlier long widget soak and Application Verifier records in STABILITY.md remain historical and keep their original executable identities; no fresh sanitizer or Application Verifier run is claimed for 0.2.0.

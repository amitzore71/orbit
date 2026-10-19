# Orbit 0.1.1 validation — 20 October 2026

The executable is uncompressed by default. The release pipeline verifies the portable executable, ZIP, installer build, native imports, ASLR/NX flags, relocations, and checksums. The executable identity is in `build-info.json`.

Automated validation:

- 42 ordinary workspace tests passed with the diagnostics feature, including regression tests for pending task edits, coalesced snapshots, rejected settings, sleep/resume during storage errors, reserved-capacity exit and recovery, repeated exit retries, and historical queries preserving long intervals.
- Clippy passed for all workspace targets with diagnostics and `-D warnings`; Rust formatting passed.
- The native helper lifecycle test passed 200 create/destroy cycles and its handle, GDI, USER, and callback-release assertions.
- The release pipeline runs actual process tests for schema-2 migration and legacy database preservation, Unicode tasks, all three widgets, one complete 60-second focus session, paused restart, injected SQLite write failure, and abrupt termination/recovery. Their JSON results are included in the package.

Current release footprint and native-input-idle startup measurements are in `release-footprint.json` and `release-startup.json`, each identifying its measured executable by SHA-256. Short CPU samples do not establish zero CPU consumption; native input-idle does not measure first paint.

The earlier long widget soak and Application Verifier records in STABILITY.md are historical and retain their original executable identities. No fresh sanitizer or Application Verifier heap run is claimed for 0.1.1. Physical input, actual OS sleep, mixed-DPI monitor changes, and the complete interactive installer/uninstaller flow still require the manual checks in VALIDATION.md.

# Validation

## Automated coverage

Core tests exercise delayed callbacks, duplicate-expiry prevention, awake-time sleep exclusion, pause/resume, task binding, skip/reset counts, wall-clock changes, paused restart recovery, long breaks, changed durations, and delayed automatic starts. Storage tests exercise migrations, Unicode CRUD, checkpoint durability, midnight splitting, partial-time reporting, calendar ranges, and idempotent completion credit. Windows tests check monotonic clock access, shortcut parsing, and duplicate shortcut validation. The interface tests ring path boundaries.

Run `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`.

On 19 October 2026, all 27 ordinary tests passed and the two opt-in native resource tests also passed; Clippy passed with warnings treated as errors. Rust reported a workspace-filesystem limitation when hard-linking its incremental cache and used copies; this was not a source-code lint failure. The background integration script also passed a real 60-second expiry (one completion, exactly 60,000 ms credited), real process termination/restart, SQLite write-failure recovery, and database integrity. The memory audit added 500 widget lifecycle cycles, 150 cycles under Microsoft Application Verifier, a positive invalid-free control, 200 shell-helper cycles, and 20 silent audio playbacks. See [the stability report](STABILITY.md) for scope, measurements, fixes, and reproduction commands. Run it with `python scripts/integration-check.py`.

## Desktop checks

Use a separate data directory. Check start/pause/reset/skip and all modes; add, edit, select, complete, and delete a Unicode task; navigate reports; save settings; and inspect About attribution. Enable each widget, move it, close/reopen it, change its layer, enable click-through, and recover it through the tray. Confirm saved placement after relaunch.

Observed during this run: the main timer rendered, Start changed to Pause, a Japanese/Hindi task was saved and displayed, and widget controls were exercised. Native tray creation succeeded when launched outside the restricted process sandbox. Desktop automation was stopped with the physical Escape key, and no further automated UI input was performed. The remaining desktop checks below are not claimed as completed.

Inspect main, ring, task, and mini windows at 100%, 125%, 150%, and 200% scaling. Check keyboard focus and activation, high contrast, reduced motion, notification activation, and muted or unavailable audio. Test a conflicting shortcut without interfering with other apps' bindings.

Hardware and shell scenarios requiring a separate manual pass: sleep/hibernate and resume; Explorer restart; mixed-DPI monitor transitions and monitor removal; actual DST/time-zone changes; a Windows 11 22H2 clean machine; and installation/uninstallation through the complete installer UI. Unit tests model the timer and calendar invariants, but do not substitute for these system checks.

## Distribution checks

Inspect PE imports to verify no Visual C++ runtime DLL dependency remains. Verify the ZIP and installer contain the same executable and dependency notices. Keep user data on uninstall. Code signing is conditional on a signing certificate and is not performed with an invented identity.

The recorded memory/resource audit predates the Orbit naming update. Its original executable hashes and verifier logs are retained as historical evidence; they do not certify the renamed binary.

Orbit rename validation: all 28 ordinary tests passed, including a regression that verifies legacy databases are selected in place and never overwritten. The installer preserves the earlier AppId for upgrades; its complete installation/uninstallation UI remains a manual check.

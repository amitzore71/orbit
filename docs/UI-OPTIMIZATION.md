# Task selection and display scheduling — 19 October 2026

Selecting a task now updates the previous and new selected rows in stable main-window and widget models. It uses an ID-to-row index instead of rebuilding both lists. Task additions, edits, completion, and deletion still rebuild their data snapshots. Opening a widget binds the existing model. These changes avoid replacing list delegates and disrupting keyboard focus on selection.

One-second display updates now run only when the Timer page, ring widget, or mini timer is visible. Other pages and the tasks widget use the completion deadline. Navigating back to Timer reconciles elapsed time and resumes display updates. Timer strings are shared between visible surfaces, and mini-only rendering skips ring-path generation.

## Measurements

Matched Windows debug/diagnostics builds, 5,000 multilingual tasks, one warm-up batch followed by seven measured batches of 100 selections:

| Measurement | Before | After |
|---|---:|---:|
| Median time for 100 task-model selection updates | 1,481.74 ms | 0.2615 ms |
| Main task-model replacements across 700 measured selections | 700 | 0 |
| Display ticks with Tasks page and only the tasks widget, over approximately 3.2 seconds | 4 | 0 |
| Display ticks with mini timer visible | 3 | 3 |
| Display ticks on Timer page | 3 | 3 |

The timed batches measure model updates; they exclude persistence and completed frame painting. These are debug-build microbenchmarks, not a release-build or complete click-to-paint speedup claim. Display-tick counts vary with alignment to second boundaries. The first display window is discarded because it overlaps task-benchmark work on the UI thread. Full samples and executable hashes are in [ui-optimization.json](ui-optimization.json).

Stable widget rows and an ID index trade retained memory proportional to task count for predictable selection cost. No new process-memory or release-binary size reduction is claimed.

## Verification and reproduction

The workspace's 31 tests passed, including selection changes, completed/missing tasks, deletion/reordering, and model reuse. Clippy with diagnostic features and warnings denied passed. A real background expiry completed exactly one session and credited 60,000 ms; restart restored paused. The UI audit produced 19 native screenshots at 150% rendering scale for all pages, compact layouts, high-contrast states, and widgets. Live native interaction confirmed that task selection retained the button's visible focus outline and persisted the new selection. Returning from Tasks to Timer reconciled the elapsed time; the separate wakeup scenario verified per-second updates on the Timer page. The restricted launch environment still reported unavailable tray creation; shell integration was not revalidated.

Commit `1d4566a` records the benchmark harness with the original implementation. Build it with `cargo build --features diagnostics --locked` and save its executable. Build the optimized revision with the same command, then run:

```powershell
python scripts/benchmark-ui.py --before path/to/baseline/orbit.exe
cargo test --workspace --features diagnostics --locked
cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings
python scripts/integration-check.py target/debug/orbit.exe
python scripts/ui-audit.py --scales 1.5
```

Diagnostic benchmarks and PNG dependencies remain excluded from ordinary release builds. Run data, screenshots, and build outputs are temporary and ignored by Git.

Windows kept an existing regular Orbit instance's executable locked. The final source compiled and linked successfully, but Cargo could not replace that running executable. The final benchmark used the freshly linked executable under a separate filename. The running instance was preserved; exit Orbit before rebuilding its executable at the same path.

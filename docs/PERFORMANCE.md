# Performance measurements

## Orbit 0.2.0 release — 20 October 2026

The packaged executable is **11,670,016 bytes** with SHA-256 `AB091AADEC257C60DCD0A16D6D4E98276E3851034EA2EF5979664C0AD1BB513B`, uncompressed. `dist/SHA256SUMS.txt` identifies the final portable ZIP and installer. Measurements identify that exact staged executable in [release-footprint.json](release-footprint.json), [release-startup.json](release-startup.json), and [release-window-startup.json](release-window-startup.json).

| Scenario | Mean private commit | Mean total working set |
|---|---:|---:|
| Tray idle, fresh process | 3.43 MiB | 20.41 MiB |
| Tray running, fresh process | 3.36 MiB | 20.39 MiB |
| Main window paused | 10.21 MiB | 28.36 MiB |
| Main window running | 10.13 MiB | 28.39 MiB |
| Main window minimized, running | 10.20 MiB | 28.37 MiB |
| Main window and all widgets running | 12.25 MiB | 30.91 MiB |

Startup and interaction evidence:

- Warm tray launch to native input-idle: **22.80 ms** median (7 launches, first excluded), effectively at the 0.1.1 median of 23.54 ms. An earlier run of the same executable measured 39.35 ms, so this probe is sensitive to machine state; the final retained run is the release evidence.
- Warm launch to an externally observed painted, responsive window: **133.98 ms** median and **141.27 ms** p95 over 30 trials (first trial 153.88 ms, excluded from warm statistics). This is inside the median ≤150 ms and p95 ≤300 ms gates for the measured scope.
- The footprint still meets every empty-fixture private-commit and working-set ceiling in the UI and performance plan.
- The executable remains under the 12,000,000-byte limit with 329,984 bytes of headroom.

Footprint scenarios use isolated empty databases, two seconds of warm-up, and eight seconds of sampling. The window probe runs at the recorded display scale with an empty database and light appearance. Cold-cache launch, 5,000-task search, input-to-feedback latency, navigation latency, frame timings, a 10-minute tray CPU sample, and the 500-cycle lifecycle soak are **not** measured for 0.2.0. Short CPU samples do not establish zero CPU use, and none of these numbers support a comparative claim against other applications.

## Orbit 0.1.1 release — 20 October 2026

The default distribution now keeps its executable uncompressed. ZIP and installer downloads remain compressed. This avoids the decompression work and additional private commit observed in the 0.1.0 UPX package; executable compression is available only through `-CompressExecutable`.

Each release build records the actual staged executable in `release-footprint.json` and `release-startup.json`. Verification requires both measurement hashes to match the packaged executable. Footprint scenarios use separate empty databases, two seconds of warm-up and eight seconds of sampling. Startup uses seven tray launches; the warm median excludes the first launch and measures native input-idle, not first paint. Short samples and warm-cache results are not battery, cold-cache, or universal latency guarantees.

Task mutations now return fixed-size acknowledgements, and one outstanding task snapshot coalesces later refresh requests. This removes the full-list reply per mutation that retained about 193 MiB in the audit's delayed-consumer stress case (5,000 tasks and 128 edits). Recovery and exit use reserved queue capacity; persistence completion is asynchronous in the UI. Historical report queries bound the ordinary interval seek and preserve unusually long intervals through a sparse index.

Current correctness checks and the distinction from historical heap/resource audits are recorded in RELEASE-VALIDATION.md. Measurements below retain their original build scope.

## Task analytics optimization — 19 October 2026

Matched release-profile storage benchmarks use an in-memory SQLite database with 100,000 focus intervals, 50,000 completed sessions, and 500 multilingual task titles across 31 days. Each median uses ten samples after one warm-up. These timings measure report construction, excluding painting and disk I/O.

| Measurement | Before | After |
|---|---:|---:|
| Daily task report | 13.1339 ms | 7.4565 ms |
| 31-day task report | 355.5735 ms | 231.5246 ms |
| Tasks widget daily totals | 13.1339 ms (full report) | 3.0885 ms (summary) |

Task titles are loaded once per distinct task rather than once per interval. The tasks widget skips task aggregation, title lookups, sorting, and day-model construction. Startup defers the full report until Reports is opened. Existing interval indexes and asynchronous/coalesced report requests remain in use; the task breakdown is virtualized.

Reproduce the optimized measurement with `cargo test -p focus-storage --release --locked report_analytics_benchmark -- --ignored --nocapture`. The benchmark verifies task totals and daily-summary equality. It uses synthetic data to stress report processing; the values are measurements on this machine, not universal guarantees.

## Historical compact release size — 19 October 2026

These size measurements predate task analytics and the AboutSlint widget. Current executable sizes and hashes are recorded in `build-info.json`.

| Build | Bytes | MB (decimal) |
|---|---:|---:|
| Development EXE from the reported screenshot | 42,775,552 | 42.78 |
| Uncompressed release | 10,646,016 | 10.65 |
| Packaged release, UPX 5.2.1 | 4,238,336 | 4.24 |

Removing unused SQLite extensions reduced the uncompressed release from 11,009,536 to 10,646,016 bytes. The packaged EXE uses LZMA executable compression and expands in memory at startup. The historical 10,000,000-byte packaging limit applied to that build; the current limit is 12,000,000 bytes. [Build metadata](build-info.json) records both compressed and uncompressed sizes and hashes.

The compact binary passed all-window startup, Unicode/legacy database preservation, a real 60-second expiry, paused restart recovery, SQLite write-failure recovery, and integrity checks. Packaging verifies restored runtime code/data, unwind tables, relocations, ASLR/NX, native imports, and ZIP equality. Windows icon resources and the DPI manifest were checked separately. Runtime memory measurements below describe the historical uncompressed binary.

## Historical runtime footprint

Measured on **19 October 2026**, Microsoft Windows NT 10.0.26300.0, x64, 16 logical processors, using the release software renderer and a statically linked CRT. The recorded audit executable was **10.07 MiB** (10,555,392 bytes).

These runtime measurements predate the Orbit branding update. They describe the executable identified in `footprint.json`, not the current source. `build-info.json` identifies the last packaged executable.

| Scenario | Private commit / MiB | Total working set / MiB | Private working set / MiB | CPU / one core |
|---|---:|---:|---:|---:|
| tray-idle | 3.46 | 20.43 | 2.69 | 0.0000% |
| tray-running | 3.47 | 20.45 | 2.69 | 0.0000% |
| main-paused | 10.20 | 28.38 | 9.15 | 0.0000% |
| main-running | 10.21 | 28.44 | 9.26 | 1.6358% |
| main-minimized-running | 10.11 | 28.44 | 9.32 | 0.0256% |
| all-widgets-running | 12.78 | 31.37 | 11.74 | 1.6126% |

The main scenarios show the main window; the all-widgets scenario includes the main window and all three widgets. Tray scenarios launch directly to the tray. A separate 500-cycle lifecycle test with 2,000 tasks measured about 13.7 MiB of private commit after use; see `STABILITY.md` for raw-counter summaries and resource checks. Private commit and resident working set measure different things. Windows system/compositor memory is not assigned to the application by these counters. Each scenario uses an isolated empty database and samples for 60 seconds after 10 seconds of warm-up. No working-set trimming is used. CPU is measured through GetProcessTimes directly; QueryProcessCycleTime deltas are included in the JSON. A zero CPU sample means no process CPU was recorded at the counter's resolution; it does not prove that the process never executes.

## Targets for the recorded build

- Executable ≤10 MiB: **exceeded by 68 KiB**. Static CRT avoids a separate Visual C++ runtime dependency; the measured size is reported without hiding that cost.
- Tray-only private commit ≤30 MiB: **met**.
- Tray-only CPU <0.1% of one logical core: **met in this sample**.

The recorded uncompressed portable distribution, including documentation and dependency notices, was **12.80 MiB** (13,426,814 bytes). SQLite data grows separately as you use the app. New portable ZIP and installer builds are written to `dist`; neither contains an external browser or UI runtime.

## Reproduce

```powershell
cargo build --release --locked
.\scripts\Measure-Footprint.ps1 -WarmupSeconds 10 -SampleSeconds 60
```

Historical measurements are included in `docs/footprint.json`; new runs write to ignored `artifacts/footprint.json`. The native input-idle startup counter is included in that JSON; it is not a first-paint measurement. SHA-256 of the measured executable: `4E0ACF809C90E7EA291F53514A20229D3F48065940AF9D7D920443E6C7271F26`. Startup-to-first-paint, battery draw, cold file-cache behavior, long-inactivity rendering, hardware sleep, and mixed-monitor scenarios have not been quantified. These results describe this machine and an empty database; they are not universal resource guarantees.

## Task selection and display scheduling

Task selection updates the previous and new rows in stable list models through an ID index. Task mutations rebuild the snapshots. One-second display updates run only for a visible Timer page, ring widget, or mini timer; other pages and the tasks widget schedule the completion deadline. Returning to Timer reconciles elapsed time.

On 19 October 2026, matched debug/diagnostics builds with 5,000 tasks produced these results:

| Measurement | Before | After |
|---|---:|---:|
| Median time for 100 task-model selection updates | 1,481.74 ms | 0.2615 ms |
| Model replacements across 700 selections | 700 | 0 |
| Display ticks with Tasks page and tasks widget over about 3.2 seconds | 4 | 0 |
| Display ticks with mini timer visible | 3 | 3 |
| Display ticks on Timer page | 3 | 3 |

These timings exclude persistence and frame painting. Seven batches follow one warm-up batch; the first display window is discarded because it overlaps model work. Retained rows and the ID index use memory proportional to task count. Samples and executable hashes are in [ui-optimization.json](ui-optimization.json).

To reproduce, build baseline commit `1d4566a` and the optimized source with `cargo build --features diagnostics --locked`, saving each executable separately. Run `python scripts/benchmark-ui.py --before path/to/baseline/orbit.exe --after path/to/optimized/orbit.exe`.

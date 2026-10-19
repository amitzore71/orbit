# Performance measurements

## Compact release size — 19 October 2026

| Build | Bytes | MB (decimal) |
|---|---:|---:|
| Development EXE from the reported screenshot | 42,775,552 | 42.78 |
| Uncompressed release | 10,646,016 | 10.65 |
| Packaged release, UPX 5.2.1 | 4,238,336 | 4.24 |

Removing unused SQLite extensions reduced the uncompressed release from 11,009,536 to 10,646,016 bytes. The packaged EXE uses LZMA executable compression and expands in memory at startup. The 10,000,000-byte packaging limit applies to the distributed EXE. [Build metadata](build-info.json) records both compressed and uncompressed sizes and hashes.

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

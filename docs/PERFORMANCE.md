# Performance measurements

Measured on **19 October 2026**, Microsoft Windows NT 10.0.26300.0, x64, 16 logical processors, using the release software renderer and a statically linked CRT. The recorded audit executable was **10.07 MiB** (10,555,392 bytes).

These runtime measurements predate the Orbit branding update and describe the earlier audited binary. They are retained with their original hash, rather than presented as new measurements of Orbit. The current Orbit executable is **10.07 MiB** (10,555,904 bytes), SHA-256 `E396E1B066A603D1640C3F2BCA3B0F33588A6D3F660F0A7548314C2352410980`; current build identity is in `build-info.json`.


| Scenario | Private commit / MiB | Total working set / MiB | Private working set / MiB | CPU / one core |
|---|---:|---:|---:|---:|
| tray-idle | 3.46 | 20.43 | 2.69 | 0.0000% |
| tray-running | 3.47 | 20.45 | 2.69 | 0.0000% |
| main-paused | 10.20 | 28.38 | 9.15 | 0.0000% |
| main-running | 10.21 | 28.44 | 9.26 | 1.6358% |
| main-minimized-running | 10.11 | 28.44 | 9.32 | 0.0256% |
| all-widgets-running | 12.78 | 31.37 | 11.74 | 1.6126% |

The main scenarios show the main window; the all-widgets scenario includes the main window and all three widgets. Tray scenarios launch directly to the tray. A separate 500-cycle lifecycle test with 2,000 tasks measured about 13.7 MiB of private commit after use; see `STABILITY.md` for raw-counter summaries and resource checks. Private commit and resident working set measure different things. Windows system/compositor memory is not assigned to the application by these counters. Each scenario uses an isolated empty database and samples for 60 seconds after 10 seconds of warm-up. No working-set trimming is used. CPU is measured through GetProcessTimes directly; QueryProcessCycleTime deltas are included in the JSON. A zero CPU sample means no process CPU was recorded at the counter's resolution; it does not prove that the process never executes.

## Targets

- Executable ≤10 MiB: **exceeded by 68.5 KiB**. Static CRT avoids a separate Visual C++ runtime dependency; the measured size is reported without hiding that cost.
- Tray-only private commit ≤30 MiB: **met**.
- Tray-only CPU <0.1% of one logical core: **met in this sample**.

The total uncompressed portable distribution, including documentation and all dependency notices, is **12.80 MiB** (13,426,814 bytes). SQLite data grows separately as you use the app. The portable ZIP and installer sizes can be read directly from `dist`; neither contains an external browser or UI runtime.

## Reproduce

```powershell
cargo build --release --locked
.\scripts\Measure-Footprint.ps1 -WarmupSeconds 10 -SampleSeconds 60
python scripts/update-performance.py
```

Raw measurements are included in `docs/footprint.json` (the source workspace also keeps `artifacts/footprint.json`). The native input-idle startup counter is included in that JSON; it is not a first-paint measurement. SHA-256 of the measured executable: `4E0ACF809C90E7EA291F53514A20229D3F48065940AF9D7D920443E6C7271F26`. Startup-to-first-paint, battery draw, cold file-cache behavior, long-inactivity rendering, hardware sleep, and mixed-monitor scenarios have not been quantified. These results describe this machine and an empty database; they are not universal resource guarantees.

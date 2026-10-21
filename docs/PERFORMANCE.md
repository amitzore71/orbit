# Performance measurements

## Orbit 0.3.0 release — 21 October 2026

The packaged executable is **4,173,312 bytes**, SHA-256 `0E16E20064455DF7A51EF298785C7B05143591521991CAE91A663747B6EA3215`. The ZIP and installer identities are in `dist/SHA256SUMS.txt`. The measurements below identify this exact executable, including the final toast-centering correction.

Reference desktop: AMD Ryzen 7 7735HS, 8 physical / 16 logical cores, 13.3 GiB usable memory, Windows build 26300, WebView2 155.0.4283.62. Native captures used 150% display scale; browser journeys additionally emulated 100–200%.

| Scenario | Mean private commit | Peak private commit | CPU, one logical core |
|---|---:|---:|---:|
| tray-idle | 246.5 MiB | 304.6 MiB | 0.78% |
| tray-running | 250.2 MiB | 306.0 MiB | 0.94% |
| main-paused | 253.4 MiB | 323.9 MiB | 0.26% |
| main-running | 323.6 MiB | 340.2 MiB | 21.86% |
| main-minimized-running | 263.6 MiB | 280.4 MiB | 0.78% |
| all-widgets-running | 425.7 MiB | 431.5 MiB | 28.16% |

These figures include the Rust host and all descendant WebView2 processes. Each scenario uses an empty isolated database, 30 seconds of warm-up, and 30 seconds of wall-clock sampling. Private committed bytes are not physical resident RAM. Summed working-set measurements are retained in [release-steady-footprint.json](release-steady-footprint.json), where shared pages can be counted repeatedly. No working-set trimming is used. Concurrent isolated checks ran on the same desktop; measurements exclude their process trees but retain the effects of host load.

| Acceptance target | Observed result | Status |
|---|---:|---|
| Warm interactive startup p95 <=1,500 ms | 1045.4 ms | Within target |
| Interaction feedback <=100 ms | 26.7 ms | Within target for sampled click |
| Frame interval p95 <=20 ms | 16.9 ms | Within target for sampled frames |
| Running main window <=300 MiB private | 323.6 MiB mean | Above target; accepted with full motion |
| All widgets <=450 MiB private | 425.7 MiB mean | Within target in sampled interval |
| Hidden CPU <0.5% of one core over ten minutes | 0.8229% | Above target |

The full-motion build is retained at the user's explicit preference, accepting the small main-window memory miss. The hidden-CPU target is also unmet and remains a performance limitation; the application should not be described as meeting every performance gate. The ten-minute tray-running sample used 15 seconds of warm-up and 600.018 actual sampled seconds; see [release-hidden-soak.json](release-hidden-soak.json).

Startup uses 30 launches, excludes the first from warm statistics, and measures process entry to React committing its first real Rust snapshot (median 890.2 ms). It does not measure physical display latency or cold boot. Earlier heavily loaded runs reached approximately 1.8 seconds p95, so this result is not a universal guarantee. The interaction probe measures one DOM click to rendered timer feedback; the frame probe measures 120 requestAnimationFrame intervals, not compositor/GPU tracing. These are reference-machine observations, not battery-life or comparative claims.

The static SVG ring follows authoritative timer snapshots; a composited progress dot supplies smooth movement. Reports and hidden windows do not request one-second display updates. Reduced motion remains available, but is not forced by default to satisfy resource measurements.

# Memory, resource, and performance audit — 19 October 2026

The audit found and fixed boundedness, native lifetime, and unnecessary-work problems. No heap/handle errors or continuing widget resource growth were detected in the exercised paths after these changes. This is a finite stress test, not a guarantee that every possible leak or invalid access is absent.

| Finding | Fix and regression evidence |
|---|---|
| Persistent SQLite errors accumulated an unlimited retry deque. | Bound pending operations to 130 and the worker channel to 128; validate payloads before enqueueing. A write failure closes the write gate, pauses the timer, and preserves one final paused checkpoint. Timer commands restore their previous state if submission is rejected. A later control retries saving. Tests cover 20,000 failed writes, 10,000 gated submissions, exact-once recovery of accepted tasks, and a real process with an injected SQLite failure. |
| A task mutation could commit before its returned task-list query failed, then be inserted again during retry. | Keep the mutation and returned snapshot inside one transaction. An intentionally malformed old row makes the response query fail; the test verifies rollback and exactly one insertion after recovery. |
| Every sound preview started another audio thread. The waveOut cleanup ignored driver return values before freeing its header and samples. | Allow one alarm playback at a time; ignore overlapping previews. Play a complete PCM WAVE buffer synchronously on that temporary thread, keeping the buffer alive until the native call returns. Test 100,000 contention attempts, WAV structure/volume, and 20 real silent playbacks with handle counts. Toast work is also limited to one in-flight request. |
| The shell thread had no joined owner, and shell commands were unbounded. Failed subclass installation retained callbacks. | Bound the shell channel to 64. The platform owner stops and joins its thread on shutdown/drop; tray icons and the native window class are released. Retain window hooks only after successful installation. Two hundred helper lifecycle cycles verify released callbacks and stable handle, GDI, and USER counts. |
| Synchronous drag dispatch could enter a nested Windows move loop while App's RefCell was mutably borrowed. | Post the native drag message so the Slint callback releases the borrow first. The thread-local root borrow also ends before calling application code; shell-folder and COM setup calls run outside the application borrow. Physical drag interaction still belongs in the manual desktop pass. |
| Minimized windows kept the one-second display timer alive. Hidden windows still received timer-property updates. | Treat minimized surfaces as invisible and render only visible timer surfaces. When all timer surfaces are hidden/minimized, schedule only expiry. The lifecycle driver's final running tray phase recorded zero display ticks. The final six-scenario measurements include a minimized running window. |
| Task ScrollViews instantiated every task's controls. Report queries scanned old history, queried session completion per interval, and redundant UI requests queued obsolete work. | Use virtualized ListViews. Add schema-v2 `intervals_end`, join completion in SQL, and inspect only overlapping day buckets. Coalesce each report stream to one active query plus the latest requested generation. Refresh task counts only when sessions end; request today's widget report only when its widget exists. |

## Results

- **27 ordinary tests passed**, plus **2 native resource tests explicitly run**. Clippy passed with `-D warnings`, including the diagnostics feature. The timer test explores **800,000 deterministic randomized commands**, checking credited awake time, interval lengths, progress bounds, and unique completion credit.
- **500 complete widget cycles with 2,000 Japanese/Hindi task titles** created/destroyed 1,500 widget windows. All 1,500 weak references expired. Across 100-cycle batches after warm-up, GDI minima stayed at 17, USER minima at 21, and handle minima decreased from 247 to 242. Private commit minima varied from 13.0 to 13.4 MiB rather than growing monotonically. These samples include both open and closed windows; minima and weak ownership checks are reported separately.
- After cycling, **running tray private commit settled near 13.7 MiB**. During the final 34.3-second settled sample it decreased from 14,397,440 to 14,299,136 bytes; handles fell from 242 to 238; GDI/USER stayed at 17/20; threads fell from 7 to 4. No display ticks occurred. GetProcessTimes recorded 0 ms at its accounting resolution; QueryProcessCycleTime still recorded 1,815,470 cycles. Zero quantized CPU time does not mean the process never executes.
- **Microsoft Application Verifier Heaps + Handles:** 150 further widget cycles with 300 tasks, 450 expired weak references, and **zero error log entries**. All three verifier DLLs were observed loaded. The exact final release executable was also run with all windows and produced no verifier entries. Instrumented memory was about 94.5 MiB because of verifier overhead; it is excluded from normal footprint results.
- A disposable positive control deliberately freed one heap block twice. Application Verifier stopped it with `0xC0000421` and logged Heaps stop `0x7`, “Heap block already freed.” This verifies the instrumentation and log collection. All temporary verifier settings were removed afterward. The control is never linked into the app.
- The report-query benchmark used **1,000,000 old intervals plus 10,000 matching intervals**, the same 1 MiB SQLite cache, and eight measured repetitions after warm-up. Median query/row-consumption time fell from **154.7 ms to 16.8 ms (9.2×)**. This is a Python/SQLite comparison of the old/new queries, not an end-to-end Rust UI latency claim. The Rust test separately asserts the seek/join query plan.
- A real 60-second background session produced exactly one completion and **60,000 ms** of focus credit. An injected checkpoint-write failure paused the running process, retained and recovered **39 ms** of actual activity, and exited cleanly. A separate real process termination restored paused without crediting unsaved time; SQLite `integrity_check` returned `ok`.

Machine-readable measurements are in `stability-results.json`; raw process samples, verifier XML/binary logs, and integration results remain under the source workspace's `artifacts` directory. Normal release footprint and distribution sizes are in `PERFORMANCE.md` and `footprint.json`. The diagnostics driver is excluded from distributed executables.

## Reproduce

```powershell
cargo test --workspace --locked
cargo clippy --workspace --all-targets --features diagnostics --locked -- -D warnings
cargo test -p platform-win --locked -- --ignored --test-threads=1 --nocapture
python scripts/benchmark-reports.py
cargo build --release --features diagnostics --locked
New-Item -ItemType Directory artifacts -Force
Copy-Item target/release/orbit.exe artifacts/orbit-diagnostics.exe
python scripts/stress-check.py --exe artifacts/orbit-diagnostics.exe --cycles 500 --tasks 2000
# Requires installed Application Verifier and an elevated terminal:
.\scripts\Test-AppVerifier.ps1
cargo build --release --locked
python scripts/integration-check.py
python scripts/integration-failure-check.py
.\scripts\Measure-Footprint.ps1 -WarmupSeconds 10 -SampleSeconds 60
```

The Python resource monitor requires `psutil`; other Python scripts use the standard library. Tests use isolated databases and open temporary app windows without injecting mouse or keyboard input. Hardware sleep/hibernate, Explorer replacement, real monitor changes, DST/time-zone transitions, physical dragging, and a multi-day soak still require the manual pass described in `VALIDATION.md`. Application Verifier covers exercised native paths; a full address-sanitizer run over every dependency was not performed.

The recorded memory/resource audit predates the Orbit naming update. Its original executable hashes and verifier logs are retained as historical evidence; they do not certify the renamed binary.

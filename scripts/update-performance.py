"""Turn raw process counters into the distributable performance report."""
import json
import pathlib
import hashlib

root = pathlib.Path(__file__).resolve().parent.parent
data = json.loads((root / 'artifacts/footprint.json').read_text(encoding='utf-8-sig'))
staging = root / 'dist/Orbit-0.1.0-windows-x64'
public_data = {**data, 'executable': pathlib.PureWindowsPath(data['executable']).name}
public_json = json.dumps(public_data, indent=2)
(root / 'docs/footprint.json').write_text(public_json, encoding='utf-8', newline='\n')
(staging / 'docs/footprint.json').write_text(public_json, encoding='utf-8', newline='\n')
speed = root / 'target/release-speed/orbit.exe'
mib = 1024 * 1024
current_exe = (staging / 'orbit.exe').read_bytes()
exe_bytes = len(current_exe)
current_sha = hashlib.sha256(current_exe).hexdigest().upper()
scope_note = '' if current_sha == data['executableSha256'] else f"\nThese runtime measurements predate the Orbit branding update and describe the earlier audited binary. They are retained with their original hash, rather than presented as new measurements of Orbit. The current Orbit executable is **{exe_bytes/mib:.2f} MiB** ({exe_bytes:,} bytes), SHA-256 `{current_sha}`; current build identity is in `build-info.json`.\n"
speed_bytes = speed.stat().st_size if speed.exists() else None
rows = []
for row in data['scenarios']:
    rows.append(f"| {row['scenario']} | {row['privateBytesMean']/mib:.2f} | {row['workingSetBytesMean']/mib:.2f} | {row['privateWorkingSetBytesMean']/mib:.2f} | {row['cpuPercentOfOneCore']:.4f}% |")
comparison = ''
if speed_bytes:
    comparison = f"\nThe speed-optimized comparison build (`opt-level=3`) was **{speed_bytes/mib:.2f} MiB** ({speed_bytes:,} bytes), versus **{exe_bytes/mib:.2f} MiB** for the size profile. The size profile is {(1-exe_bytes/speed_bytes)*100:.1f}% smaller. This comparison is of executable size; no claim of equal rendering throughput is made. The speed comparison is from the initial implementation, before the memory audit; it uses the same locked dependencies and static CRT. It is not a rebuilt speed profile of this final revision. The size profile remains the release default.\n"
exe_result = 'met' if exe_bytes <= 10*mib else f"exceeded by {(exe_bytes-10*mib)/1024:.1f} KiB"
tray = next(row for row in data['scenarios'] if row['scenario'] == 'tray-idle')
memory_result = 'met' if tray['privateBytesMean'] <= 30*mib else 'exceeded'
cpu_result = 'met in this sample' if tray['cpuPercentOfOneCore'] < .1 else 'exceeded in this sample'
content = f"""# Performance measurements

Measured on **19 October 2026**, {data['os']}, x64, {data['logicalProcessors']} logical processors, using the release software renderer and a statically linked CRT. The recorded audit executable was **{data['executableBytes']/mib:.2f} MiB** ({data['executableBytes']:,} bytes).
{scope_note}

| Scenario | Private commit / MiB | Total working set / MiB | Private working set / MiB | CPU / one core |
|---|---:|---:|---:|---:|
{chr(10).join(rows)}

The main scenarios show the main window; the all-widgets scenario includes the main window and all three widgets. Tray scenarios launch directly to the tray. A separate 500-cycle lifecycle test with 2,000 tasks measured about 13.7 MiB of private commit after use; see `STABILITY.md` for raw-counter summaries and resource checks. Private commit and resident working set measure different things. Windows system/compositor memory is not assigned to the application by these counters. Each scenario uses an isolated empty database and samples for {data['scenarios'][0]['sampleSeconds']} seconds after {data['scenarios'][0]['warmupSeconds']} seconds of warm-up. No working-set trimming is used. CPU is measured through GetProcessTimes directly; QueryProcessCycleTime deltas are included in the JSON. A zero CPU sample means no process CPU was recorded at the counter's resolution; it does not prove that the process never executes.

## Targets

- Executable ≤10 MiB: **{exe_result}**. Static CRT avoids a separate Visual C++ runtime dependency; the measured size is reported without hiding that cost.
- Tray-only private commit ≤30 MiB: **{memory_result}**.
- Tray-only CPU <0.1% of one logical core: **{cpu_result}**.

The total uncompressed portable distribution, including documentation and all dependency notices, is **{{distribution_mib}} MiB** ({{distribution_bytes}} bytes). SQLite data grows separately as you use the app. The portable ZIP and installer sizes can be read directly from `dist`; neither contains an external browser or UI runtime.
{comparison}
## Reproduce

```powershell
cargo build --release --locked
.\\scripts\\Measure-Footprint.ps1 -WarmupSeconds 10 -SampleSeconds 60
python scripts/update-performance.py
```

Raw measurements are included in `docs/footprint.json` (the source workspace also keeps `artifacts/footprint.json`). The native input-idle startup counter is included in that JSON; it is not a first-paint measurement. SHA-256 of the measured executable: `{data['executableSha256']}`. Startup-to-first-paint, battery draw, cold file-cache behavior, long-inactivity rendering, hardware sleep, and mixed-monitor scenarios have not been quantified. These results describe this machine and an empty database; they are not universal resource guarantees.
"""
destination = root / 'docs/PERFORMANCE.md'
other_bytes = sum(path.stat().st_size for path in staging.rglob('*') if path.is_file() and path.relative_to(staging).as_posix() != 'docs/PERFORMANCE.md')
total = other_bytes
for _ in range(5):
    text = content.replace('{distribution_mib}', f'{total/mib:.2f}').replace('{distribution_bytes}', f'{total:,}')
    total = other_bytes + len(text.encode('utf-8'))
destination.write_text(text, encoding='utf-8', newline='\n')
(staging / 'docs/PERFORMANCE.md').write_text(text, encoding='utf-8', newline='\n')
print(f"Updated {destination}; distribution {total:,} bytes")

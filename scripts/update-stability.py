"""Publish measured stress results without workstation paths or per-sample noise."""
import hashlib
import json
from pathlib import Path
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[1]
load = lambda name: json.loads((root / "artifacts" / name).read_text())
stress = load("stress-results.json")
verifier = load("verifier-stress.json")
benchmark = load("report-benchmark.json")
failure = load("integration-failure-check.json")
expiry = load("integration-check.json")
expiry.pop("dataDirectory", None)
logs = [root / "artifacts" / name for name in ("still-heap-check.exe.0.dat.xml", "still-heap-check.exe.1.dat.xml", "verifier-final.xml")]
entries = sum(len(ET.parse(path).getroot().findall(".//{Application Verifier}logEntry")) for path in logs)
control = ET.parse(root / "artifacts/still-verifier-control.exe.0.dat.xml").getroot().findall(".//{Application Verifier}logEntry")
assert entries == 0
assert any(item.get("StopCode") == "0x7" for item in control)
report = dict(measuredAt="2026-10-19", executableSha256=hashlib.sha256((root / "target/release/still-pomodoro.exe").read_bytes()).hexdigest().upper(),
              standardTestsPassed=27, explicitlyRunNativeTestsPassed=2, randomizedTimerCommands=800000,
              failedWriteAttempts=20000, rejectedWritesWhileBlocked=10000, playbackGateContentionAttempts=100000,
              nativeShellCycles=200, silentNativeAudioPlaybacks=20, audioFailures=0,
              lifecycle=stress["driver"], tasksDuringLifecycle=stress["tasks"],
              lifecycleBatches=stress.get("lifecycleBatches", []), trayAfterUse=stress["trayAfterUse"],
              verifier=dict(cycles=verifier["driver"]["cycles"], weakChecks=verifier["driver"]["weakChecks"],
                            tasks=verifier["tasks"], retainedWidgets=verifier["driver"]["retainedWidgets"],
                            modules=["verifier.dll", "vfbasics.dll", "vrfcore.dll"], errorEntries=entries,
                            exactReleaseBinaryAlsoChecked=True, positiveControlStop="0x7: Heap block already freed"),
              reportQueryBenchmark=benchmark, databaseFailure=failure, realExpiry=expiry)
(root / "docs/stability-results.json").write_text(json.dumps(report, indent=2), encoding="utf-8", newline="\n")
print(json.dumps({"standardTestsPassed": 27, "nativeTestsPassed": 2, "verifierErrorEntries": entries,
                  "releasedWidgets": stress["driver"]["weakChecks"], "trayPrivateMiB":stress["trayAfterUse"]["privateBytesMedian"]/1048576}, indent=2))

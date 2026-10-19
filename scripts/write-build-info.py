"""Identify the current build separately from retained historical measurements."""
import argparse
import hashlib
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--exe", type=Path, default=root / "target/release/orbit.exe")
parser.add_argument("--compression", choices=["none", "upx"], default="none")
args = parser.parse_args()
exe = args.exe.read_bytes()
source = (root / "target/release/orbit.exe").read_bytes()
info = dict(application="Orbit", version="0.1.0", executable="orbit.exe", executableBytes=len(exe),
            executableSha256=hashlib.sha256(exe).hexdigest().upper(),
            compression=args.compression, sourceExecutableBytes=len(source),
            sourceExecutableSha256=hashlib.sha256(source).hexdigest().upper(),
            historicalAuditSha256=json.loads((root / "docs/footprint.json").read_text(encoding="utf-8"))["executableSha256"])
(root / "docs/build-info.json").write_text(json.dumps(info, indent=2), encoding="utf-8", newline="\n")

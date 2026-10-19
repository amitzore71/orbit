"""Verify the staged PE, build metadata, and ZIP agree."""
import hashlib
import json
from pathlib import Path
import struct
import zipfile

root = Path(__file__).resolve().parents[1]
stage = root / "dist/Orbit-0.1.0-windows-x64"
exe = (stage / "orbit.exe").read_bytes()
digest = hashlib.sha256(exe).hexdigest().upper()
footprint = json.loads((stage / "docs/footprint.json").read_text())
stability = json.loads((stage / "docs/stability-results.json").read_text())
build = json.loads((stage / "docs/build-info.json").read_text())
assert digest == build["executableSha256"]
assert build["application"] == "Orbit" and build["executable"] == "orbit.exe"
assert footprint["executableSha256"] == stability["executableSha256"] == build["historicalAuditSha256"]
assert len(exe) == build["executableBytes"]
assert exe == (root / "target/release/orbit.exe").read_bytes()
pe = struct.unpack_from("<I", exe, 0x3C)[0]
assert exe[pe:pe+4] == b"PE\0\0"
machine, count = struct.unpack_from("<HH", exe, pe+4)
assert machine == 0x8664
optional_size = struct.unpack_from("<H", exe, pe+20)[0]
optional = pe+24
assert struct.unpack_from("<H", exe, optional)[0] == 0x20B
sections = []
for n in range(count):
    base = optional+optional_size+n*40
    vsize, rva, raw_size, raw = struct.unpack_from("<IIII", exe, base+8)
    sections.append((rva, max(vsize, raw_size), raw))


def offset(rva):
    return next(raw+rva-start for start, length, raw in sections if start <= rva < start+length)


import_rva = struct.unpack_from("<I", exe, optional+120)[0]
cursor = offset(import_rva)
imports = []
while any(exe[cursor:cursor+20]):
    name = offset(struct.unpack_from("<I", exe, cursor+12)[0])
    imports.append(exe[name:exe.index(0, name)].decode("ascii"))
    cursor += 20
assert not any(name.lower().startswith(("vcruntime", "msvcp", "api-ms-win-crt")) for name in imports), imports
payload = {str(p.relative_to(stage)).replace("\\", "/"): p.read_bytes() for p in stage.rglob("*") if p.is_file()}
with zipfile.ZipFile(root / "dist/Orbit-0.1.0-windows-x64.zip") as archive:
    archived = {name.replace("\\", "/"): archive.read(name) for name in archive.namelist() if not name.endswith("/")}
    assert payload == archived, "ZIP contents differ from the verified staging payload"
total = sum(map(len, payload.values()))
assert (root / "dist/Orbit-0.1.0-Setup-x64.exe").stat().st_size > 1000000
result = dict(executableSha256=digest, executableBytes=len(exe), distributionBytes=total,
              zipMatchesStaging=True, x64=True, externalVCRuntime=False, imports=imports)
(root / "artifacts/release-verification.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))

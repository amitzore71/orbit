"""Verify executable size, native imports, compression, metadata, and ZIP contents."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import zipfile

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--skip-installer", action="store_true")
parser.add_argument("--upx", type=Path)
args = parser.parse_args()
stage = root / "dist/Orbit-0.1.0-windows-x64"
exe = (stage / "orbit.exe").read_bytes()
source = (root / "target/release/orbit.exe").read_bytes()
digest = hashlib.sha256(exe).hexdigest().upper()
footprint = json.loads((stage / "docs/footprint.json").read_text())
stability = json.loads((stage / "docs/stability-results.json").read_text())
build = json.loads((stage / "docs/build-info.json").read_text())
assert digest == build["executableSha256"]
assert build["application"] == "Orbit" and build["executable"] == "orbit.exe"
assert footprint["executableSha256"] == stability["executableSha256"] == build["historicalAuditSha256"]
assert len(exe) == build["executableBytes"]
assert len(exe) <= 10_000_000, "Release executable exceeds 10 MB"
assert len(source) == build["sourceExecutableBytes"]
assert hashlib.sha256(source).hexdigest().upper() == build["sourceExecutableSha256"]


def read_pe(data):
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    assert data[pe:pe+4] == b"PE\0\0"
    machine, count = struct.unpack_from("<HH", data, pe+4)
    assert machine == 0x8664
    optional = pe+24
    assert struct.unpack_from("<H", data, optional)[0] == 0x20B
    characteristics = struct.unpack_from("<H", data, optional+70)[0]
    assert characteristics & 0x140 == 0x140, "ASLR and NX must remain enabled"
    reloc_rva, reloc_size = struct.unpack_from("<II", data, optional+152)
    assert reloc_rva and reloc_size, "Windows relocations must be retained"
    table = optional + struct.unpack_from("<H", data, pe+20)[0]
    sections = {}
    for n in range(count):
        base = table+n*40
        name = data[base:base+8].split(b"\0")[0].decode("ascii")
        vsize, rva, raw_size, raw = struct.unpack_from("<IIII", data, base+8)
        sections[name] = (rva, max(vsize, raw_size), raw, raw_size)
    return optional, sections


def section_bytes(data, sections, name):
    _, _, raw, size = sections[name]
    return data[raw:raw+size]


optional, sections = read_pe(source)
read_pe(exe)
if build["compression"] == "upx":
    assert args.upx, "Pass --upx to verify a compressed build"
    subprocess.run([str(args.upx), "-t", str(stage / "orbit.exe")], check=True)
    with tempfile.TemporaryDirectory() as directory:
        restored_path = Path(directory) / "orbit.exe"
        subprocess.run([str(args.upx), "-d", "-o", str(restored_path), str(stage / "orbit.exe")], check=True)
        restored = restored_path.read_bytes()
        _, restored_sections = read_pe(restored)
        # UPX may strip debug metadata; runtime code, data and unwind tables must match.
        for name in (".text", ".data", ".pdata", ".reloc"):
            assert section_bytes(source, sections, name) == section_bytes(restored, restored_sections, name), name
else:
    assert build["compression"] == "none" and exe == source


def offset(rva):
    return next(raw+rva-start for start, length, raw, _ in sections.values() if start <= rva < start+length)


import_rva = struct.unpack_from("<I", source, optional+120)[0]
cursor = offset(import_rva)
imports = []
while any(source[cursor:cursor+20]):
    name = offset(struct.unpack_from("<I", source, cursor+12)[0])
    imports.append(source[name:source.index(0, name)].decode("ascii"))
    cursor += 20
assert not any(name.lower().startswith(("vcruntime", "msvcp", "api-ms-win-crt")) for name in imports), imports
payload = {p.relative_to(stage).as_posix(): p.read_bytes() for p in stage.rglob("*") if p.is_file()}
with zipfile.ZipFile(root / "dist/Orbit-0.1.0-windows-x64.zip") as archive:
    archived = {name.replace("\\", "/"): archive.read(name) for name in archive.namelist() if not name.endswith("/")}
    assert payload == archived, "ZIP contents differ from the verified staging payload"
if not args.skip_installer:
    assert (root / "dist/Orbit-0.1.0-Setup-x64.exe").stat().st_size > 1000000
result = dict(executableSha256=digest, executableBytes=len(exe), sourceExecutableBytes=len(source),
              compression=build["compression"], distributionBytes=sum(map(len, payload.values())),
              zipMatchesStaging=True, x64=True, aslr=True, nx=True, externalVCRuntime=False, imports=imports)
(root / "artifacts").mkdir(exist_ok=True)
(root / "artifacts/release-verification.json").write_text(json.dumps(result, indent=2))
print(json.dumps(result, indent=2))

"""Fail packaging when the release executable exceeds 10 MB (decimal)."""
import argparse
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("executable", nargs="?", type=Path, default=root / "dist/Orbit-0.1.0-windows-x64/orbit.exe")
args = parser.parse_args()
size = args.executable.stat().st_size
limit = 10_000_000
print(f"{args.executable.name}: {size:,} bytes ({size / 1_000_000:.2f} MB); limit {limit:,} bytes")
if size > limit:
    parser.exit(1, "Release executable exceeds the size limit.\n")

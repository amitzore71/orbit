"""Bound the Tauri executable; WebView2 is a separately installed shared runtime."""
import argparse
from pathlib import Path
import tomllib

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
parser.add_argument("executable", nargs="?", type=Path, default=root / f"dist/Orbit-{version}-windows-x64/orbit.exe")
args = parser.parse_args()
size = args.executable.stat().st_size
limit = 50_000_000
print(f"{args.executable.name}: {size:,} bytes ({size / 1_000_000:.2f} MB); limit {limit:,} bytes")
if size > limit:
    parser.exit(1, "Release executable exceeds the size limit.\n")

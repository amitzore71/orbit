"""Bundle exact selected dependencies' notices from the locked local Cargo registry."""
import json
import pathlib
import subprocess
import sys
import urllib.request

root = pathlib.Path(__file__).resolve().parent.parent
destination = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else root / "artifacts" / "THIRD-PARTY-NOTICES.txt"
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc"], cwd=root))
selected = subprocess.check_output(["cargo", "tree", "--locked", "--target", "x86_64-pc-windows-msvc", "-e", "normal,build", "--prefix", "none", "--format", "{p}"], cwd=root, text=True)
selected = {line.split(" (", 1)[0].removesuffix(" (*)").strip() for line in selected.splitlines()}
sections = ["Orbit — third-party notices\n\nSlint is distributed as part of this desktop application under the Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0. This license requires either the AboutSlint widget in the app or the Slint badge on a public webpage. Before distributing a build without the widget, confirm the public attribution page or an alternative applicable Slint license. Other dependencies retain their own licenses. Build-tool dependencies are also included below.\n"]
missing = []
cache = root / "packaging" / "licenses"
fetch_missing = "--fetch-missing" in sys.argv
def upstream_licenses(package, folder):
    """Use the crate's published VCS commit rather than a moving branch."""
    info_path = folder / ".cargo_vcs_info.json"
    repository = (package.get("repository") or "").removesuffix(".git").rstrip("/")
    if not info_path.exists() or not repository.startswith("https://github.com/"):
        return []
    commit = json.loads(info_path.read_text())["git"]["sha1"]
    repo = repository.removeprefix("https://github.com/")
    saved = cache / (repo.replace("/", "--") + "--" + commit)
    if not saved.exists() and fetch_missing:
        request = urllib.request.Request(f"https://api.github.com/repos/{repo}/git/trees/{commit}", headers={"User-Agent": "Orbit-license-bundler"})
        tree = json.load(urllib.request.urlopen(request, timeout=30))["tree"]
        files = [entry["path"] for entry in tree if entry["type"] == "blob" and entry["path"].upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))]
        saved.mkdir(parents=True, exist_ok=True)
        for filename in files:
            request = urllib.request.Request(f"https://raw.githubusercontent.com/{repo}/{commit}/{filename}", headers={"User-Agent": "Orbit-license-bundler"})
            data = urllib.request.urlopen(request, timeout=30).read()
            (saved / filename).write_bytes(data)
        (saved / "SOURCE.txt").write_text(f"{repository}/tree/{commit}\n", encoding="utf-8")
    return sorted(path for path in saved.glob("*") if path.is_file() and path.name != "SOURCE.txt")
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["source"] is None or f'{package["name"]} v{package["version"]}' not in selected:
        continue
    folder = pathlib.Path(package["manifest_path"]).parent
    paths = sorted({p for pattern in ["LICENSE*", "LICENCE*", "COPYING*", "NOTICE*", "license*", "LICENSES/*"] for p in folder.glob(pattern) if p.is_file()})
    if package.get("license_file"):
        explicit = folder / package["license_file"]
        if explicit.is_file() and explicit not in paths:
            paths.append(explicit)
    if not paths:
        paths.extend(upstream_licenses(package, folder))
    heading = f'\n{"=" * 72}\n{package["name"]} {package["version"]}\nLicense expression: {package.get("license") or "see license file"}\nSource: {package.get("repository") or package["source"]}\n'
    texts = []
    for path in paths:
        try:
            label = path.relative_to(folder) if path.is_relative_to(folder) else path.relative_to(root)
            texts.append(f"\n--- {label} ---\n{path.read_text(encoding='utf-8', errors='replace')}\n")
        except OSError:
            pass
    if not texts:
        missing.append(f'{package["name"]} {package["version"]}')
    sections.append(heading + "".join(texts))
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text("\n".join(sections), encoding="utf-8")
print(f"Wrote {destination} ({destination.stat().st_size:,} bytes)")
if missing:
    print("Registry packages without standalone license text: " + ", ".join(missing))
    sys.exit(1)

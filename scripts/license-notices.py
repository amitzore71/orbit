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
sections = ["Orbit — third-party notices\n\nOrbit uses Rust, Tauri 2, Next.js, React, Motion, Phosphor icons, and Geist fonts. Each dependency retains its license below. Build and test dependency notices are also included. Microsoft Edge WebView2 is a separately installed Microsoft runtime.\n"]
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
    if not paths and package.get("license") == "MPL-2.0":
        mpl = cache / "Mozilla-MPL-2.0" / "LICENSE.txt"
        if not mpl.exists() and fetch_missing:
            url = "https://www.mozilla.org/MPL/2.0/index.txt"
            mpl.parent.mkdir(parents=True, exist_ok=True)
            mpl.write_bytes(urllib.request.urlopen(url, timeout=30).read())
            (mpl.parent / "SOURCE.txt").write_text(url + "\n", encoding="utf-8")
        if mpl.exists():
            paths.append(mpl)
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
npm_packages = set()
tree = json.loads(subprocess.check_output(["pnpm.cmd" if sys.platform == "win32" else "pnpm", "list", "--prod", "--json", "--depth", "Infinity"], cwd=root / "frontend"))
def visit_npm(node):
    for item in node.get("dependencies", {}).values():
        if item.get("path"):
            manifest = pathlib.Path(item["path"]).resolve() / "package.json"
            if manifest in npm_packages:
                continue
            npm_packages.add(manifest)
        visit_npm(item)
for node in tree:
    visit_npm(node)
for manifest in sorted(npm_packages):
    package = json.loads(manifest.read_text(encoding="utf-8"))
    folder = manifest.parent
    paths = sorted({p for pattern in ("LICENSE*", "LICENCE*", "COPYING*", "NOTICE*", "license*", "OFL*") for p in folder.glob(pattern) if p.is_file()})
    # Next.js also embeds runtime dependencies under dist/compiled.
    if package.get("name") == "next":
        paths += sorted((folder / "dist/compiled").glob("*/LICENSE*"))
    # Next's platform/env packages and React's client-only marker are published
    # without standalone notices; use their exact installed monorepo license.
    parent = "next" if package.get("name", "").startswith("@next/") else "react" if package.get("name") == "client-only" else None
    if not paths and parent:
        paths = list((root / "frontend/node_modules" / parent).resolve().glob("LICENSE*"))
    if not paths:
        missing.append(f'{package.get("name")} {package.get("version")} (npm)')
    sections.append(f'\n{"=" * 72}\n{package.get("name")} {package.get("version")} (npm)\nLicense: {package.get("license", "see license file")}\n' + "".join(f'\n--- {p.name} ({parent or package.get("name")}) ---\n{p.read_text(encoding="utf-8", errors="replace")}\n' for p in paths))
if not npm_packages:
    missing.append("Frontend dependencies: run pnpm --dir frontend install --frozen-lockfile")
destination.parent.mkdir(parents=True, exist_ok=True)
destination.write_text("\n".join(sections), encoding="utf-8")
print(f"Wrote {destination} ({destination.stat().st_size:,} bytes)")
if missing:
    print("Registry packages without standalone license text: " + ", ".join(missing))
    sys.exit(1)

#!/usr/bin/env python3
"""Packs dist/vastpacks_native-<v>.hnmod (the native mod) and dist/vastpacks-<v>.hmod (the content). Deterministic."""
import hashlib
import json
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUNDLE = ROOT / "mixins/vastpacks_native"
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"


def add(z, name, data):
    info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_DEFLATED
    z.writestr(info, data)


def main():
    mixin = tomllib.loads((BUNDLE / "horizon-mixin.toml").read_text())
    version = mixin["mod"]["version"]
    meta = {"game": "26.3", "horizon": mixin["compatibility"]["horizon_revision"], "id": mixin["mod"]["id"],
            "mixins": MIXINS, "schema": 1, "version": version}
    (ROOT / "horizon-native.json").write_text(json.dumps(meta, indent=2, sort_keys=True) + "\n")
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    out = dist / f"vastpacks_native-{version}.hnmod"
    with zipfile.ZipFile(out, "w") as z:
        add(z, "horizon-native.json", (ROOT / "horizon-native.json").read_bytes())
        for f in sorted(BUNDLE.iterdir()):
            if f.is_file():
                add(z, f"mixins/vastpacks_native/{f.name}", f.read_bytes())
    print(f"{out}  {hashlib.sha256(out.read_bytes()).hexdigest()}")
    # The content mod, built fresh.
    import subprocess, sys
    subprocess.run([sys.executable, "-I", str(ROOT / "tools/build_content.py"), str(ROOT / "build")], check=True)
    content = ROOT / "build/vastpacks"
    cver = tomllib.loads((content / "horizon-mod.toml").read_text())["version"]
    hmod = dist / f"vastpacks-{cver}.hmod"
    with zipfile.ZipFile(hmod, "w") as z:
        for f in sorted(p for p in content.rglob("*") if p.is_file()):
            add(z, f.relative_to(content).as_posix(), f.read_bytes())
        add(z, "LICENSE", (ROOT / "LICENSE").read_bytes())
    print(f"{hmod}  {hashlib.sha256(hmod.read_bytes()).hexdigest()}")


if __name__ == "__main__":
    main()

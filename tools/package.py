#!/usr/bin/env python3
"""Pack the installable .hmod for Horizon's sandboxed WASM loader.

A .hmod is a ZIP whose root holds `horizon-mod.toml` and `mod.wasm` (plus
assets/ and data/ for content mods). This mirrors `Package::write_files` in
Horizon's `cargo-horizon-mod`, and the exact layout of the working Zoom .hmod.
"""
from __future__ import annotations

import hashlib
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "horizon-mod.toml"
WASM = ROOT / "mod.wasm"
OUT = ROOT / "free_look.hmod"
SUMS = ROOT / "SHA256SUMS.txt"
FILES = ["horizon-mod.toml", "mod.wasm"]

for path in (MANIFEST, WASM):
    if not path.is_file():
        raise SystemExit(f"missing deliverable: {path.name}")

with MANIFEST.open("rb") as f:
    manifest = tomllib.load(f)

if manifest.get("id") != "free_look":
    raise SystemExit(f"unexpected mod id: {manifest.get('id')!r}")
if manifest.get("api") != "0.1" or manifest.get("side") != "client":
    raise SystemExit("manifest must be api = \"0.1\", side = \"client\"")
for cap in ("ui", "input"):
    if cap not in manifest.get("experimental", []):
        raise SystemExit(f"experimental capability {cap} not declared")

with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    z.write(MANIFEST, arcname="horizon-mod.toml")
    z.write(WASM, arcname="mod.wasm")

with zipfile.ZipFile(OUT) as z:
    names = z.namelist()
    if names != FILES:
        raise SystemExit(f"unexpected .hmod layout: {names}")

digest = hashlib.sha256(OUT.read_bytes()).hexdigest()
SUMS.write_text(f"{digest}  {OUT.name}\n", encoding="utf-8")
print(f"wrote {OUT.name} ({OUT.stat().st_size} bytes)")
print(f"sha256 {digest}")
for info in zipfile.ZipFile(OUT).infolist():
    print(f"  {info.filename}: {info.file_size} bytes")

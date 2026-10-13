#!/usr/bin/env python3
"""Install a pinned public CPU runtime artifact into this checkout's ignored cache."""
import hashlib
import json
import platform
from pathlib import Path
import tarfile
from urllib.request import urlopen

root = Path(__file__).resolve().parents[1]
arch = {"x86_64": "x64", "aarch64": "aarch64"}.get(platform.machine())
if arch is None:
    raise SystemExit("No qualified CPU runtime artifact for this architecture")
record = next(r for r in json.loads((root / "models/runtime-manifest.json").read_text())["runtimes"] if r["arch"] == arch)
cache = root / "runtime"
cache.mkdir(exist_ok=True)
archive = cache / record["url"].rsplit("/", 1)[1]
if not archive.exists() or hashlib.sha256(archive.read_bytes()).hexdigest() != record["sha256"]:
    with urlopen(record["url"], timeout=60) as response:
        data = response.read(record["bytes"] + 1)
    if len(data) != record["bytes"] or hashlib.sha256(data).hexdigest() != record["sha256"]:
        raise ValueError("CPU runtime artifact mismatch")
    archive.write_bytes(data)
with tarfile.open(archive) as source:
    source.extractall(cache, filter="data")
print("Verified CPU runtime", record["version"], arch)
print(cache / f"onnxruntime-linux-{arch}-{record['version']}" / "lib" / f"libonnxruntime.so.{record['version']}")

#!/usr/bin/env python3
"""Fetch exact public candidate artifacts and verify their manifest hashes."""
import hashlib
import json
from pathlib import Path
from urllib.request import urlopen

root = Path(__file__).resolve().parents[1] / "models"
for model in json.loads((root / "manifest.json").read_text())["models"]:
    target = root / model["file"]
    if target.name != model["file"]:
        raise ValueError("model file must be a basename")
    if target.exists() and hashlib.sha256(target.read_bytes()).hexdigest() == model["sha256"]:
        print(f"Verified {target.name}")
        continue
    with urlopen(model["url"], timeout=60) as response:
        data = response.read(model["bytes"] + 1)
    if len(data) != model["bytes"] or hashlib.sha256(data).hexdigest() != model["sha256"]:
        raise ValueError(f"artifact mismatch: {target.name}")
    staging = target.with_suffix(".download")
    staging.write_bytes(data)
    staging.replace(target)
    print(f"Fetched and verified {target.name}")

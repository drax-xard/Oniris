#!/usr/bin/env python3
"""Keep packs/index.json — the in-app catalog — in sync with released packs.

The Android app fetches `packs/index.json` from the repository and offers each
entry for download, so this file is the contract between a release and the app.
It is generated from the pack spec (identity, copy, licence) plus the released
archive (size, SHA-256) — never hand-edited, because a digest that drifts from
the asset would either brick installs or silently disable the download.

Usage:
    scripts/update-pack-index.py            # every spec with a built archive
    scripts/update-pack-index.py seaside    # one pack
"""

import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPECS = ROOT / "tools" / "asset-pipeline" / "specs"
INDEX = ROOT / "packs" / "index.json"
REPO = "drax-xard/Oniris"
# Packs are `manifest.json` schemaVersion 1, which every build since beta1 can
# import; bump this only if a pack needs a newer importer than that.
MIN_APP_VERSION_CODE = 1


def archive_for(pack_id, version):
    return ROOT / "dist" / "release" / pack_id / f"{pack_id}-v{version}.onirispack"


def entry_for(spec):
    pack_id = spec["id"]
    version = spec["version"]
    archive = archive_for(pack_id, version)
    if not archive.exists():
        return None
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    authors = sorted({f"{credit['author']} ({credit['license']})" for credit in collect_credits(spec)})
    return {
        "id": pack_id,
        "name": spec["name"],
        "description": spec["description"],
        "version": version,
        "license": spec["license"],
        "credits": ", ".join(authors),
        "tags": spec.get("tags", []),
        "beds": len(spec.get("beds", [])),
        "events": len(spec.get("events", [])),
        "sizeBytes": archive.stat().st_size,
        "sha256": digest,
        "url": (f"https://github.com/{REPO}/releases/download/"
                f"{pack_id}-v{version}/{pack_id}-v{version}.onirispack"),
        "minAppVersionCode": MIN_APP_VERSION_CODE,
    }


def collect_credits(spec):
    credits = [bed["credit"] for bed in spec.get("beds", [])]
    for event in spec.get("events", []):
        credits.extend(file_spec["credit"] for file_spec in event["files"])
    return credits


def pack_license(spec):
    rank = {"CC0-1.0": 0, "Public Domain": 0, "CC-BY-3.0": 1, "CC-BY-4.0": 2}
    worst = max((rank.get(c["license"], 9) for c in collect_credits(spec)), default=0)
    for name, value in rank.items():
        if value == worst:
            return name
    return "CC-BY-4.0"


def main():
    wanted = sys.argv[1:]
    specs = []
    for path in sorted(SPECS.glob("*.json")):
        spec = json.loads(path.read_text())
        if wanted and spec["id"] not in wanted:
            continue
        spec["license"] = pack_license(spec)
        specs.append(spec)

    index = {"schemaVersion": 1, "packs": []}
    if INDEX.exists():
        existing = json.loads(INDEX.read_text())
        index["packs"] = [p for p in existing.get("packs", [])
                          if p["id"] not in {s["id"] for s in specs}]

    added = []
    for spec in specs:
        entry = entry_for(spec)
        if entry is None:
            print(f"skip {spec['id']}: no archive at {archive_for(spec['id'], spec['version'])}")
            continue
        index["packs"].append(entry)
        added.append(entry)

    index["packs"].sort(key=lambda p: p["name"].get("en", p["id"]))
    INDEX.parent.mkdir(parents=True, exist_ok=True)
    INDEX.write_text(json.dumps(index, indent=2, sort_keys=False) + "\n")
    for entry in added:
        print(f"index: {entry['id']} {entry['version']}  {entry['sizeBytes']} B  "
              f"{entry['sha256'][:16]}…  {entry['license']}")
    print(f"wrote {INDEX.relative_to(ROOT)} ({len(index['packs'])} pack(s))")


if __name__ == "__main__":
    main()

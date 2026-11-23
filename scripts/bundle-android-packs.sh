#!/usr/bin/env bash
# Generate the procedural packs and copy them into the Android assets so a
# release APK works offline on first launch.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$root/core/Cargo.toml"
destination="$root/android/app/src/main/assets/packs"
temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

packs=(
  pure-noise
  rainy-night-procedural
  deep-space-cabin
  enchanted-forest
  windy-sea-voyage
)

cargo run --quiet --manifest-path "$manifest" -p oniris-cli -- \
  gen-procedural-packs "$temporary"

rm -rf "$destination"
mkdir -p "$destination"
for pack in "${packs[@]}"; do
  if [[ ! -d "$temporary/$pack" ]]; then
    echo "missing generated pack: $pack" >&2
    exit 1
  fi
  cp -R "$temporary/$pack" "$destination/$pack"
done

echo "bundled ${#packs[@]} packs into $destination"
du -sh "$destination"

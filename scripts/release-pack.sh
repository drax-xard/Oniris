#!/usr/bin/env bash
# Build one recorded pack and publish it as a GitHub release asset.
#
# Usage:
#   scripts/release-pack.sh seaside            # build + publish seaside-v<version>
#   scripts/release-pack.sh forest-deep --prerelease
#   scripts/release-pack.sh seaside --dry-run  # everything except the API calls
#
# Credentials, in order of preference (never pass a token as an argument, it
# lands in shell history and process listings):
#   1. $GITHUB_TOKEN or $GH_TOKEN
#   2. a file in $ONIRIS_GITHUB_TOKEN_FILE, default ~/.config/oniris/github-token
#   3. `gh auth token` when the gh CLI is installed and logged in
#
# The release tag is "<pack>-v<version>" and it must point at a commit that is
# already pushed: a content release that tags a local-only commit is not
# reproducible for anyone else, so the script refuses that case.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
spec_name="${1:-}"
if [[ -z "$spec_name" ]]; then
  echo "usage: release-pack.sh <spec-name> [--prerelease] [--dry-run]" >&2
  exit 2
fi
shift

prerelease=false
dry_run=false
for arg in "$@"; do
  case "$arg" in
    --prerelease) prerelease=true ;;
    --dry-run) dry_run=true ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

PYTHON="${PYTHON:-python3}"
cd "$root"

spec="tools/asset-pipeline/specs/${spec_name}.json"
if [[ ! -f "$spec" ]]; then
  echo "no such spec: $spec" >&2
  exit 1
fi

# ---------------------------------------------------------------- build
echo "== building $spec_name"
"$PYTHON" tools/asset-pipeline/ingest.py build "$spec"

pack_dir="dist/packs/$spec_name"
archive="dist/${spec_name}.onirispack"
[[ -f "$archive" ]] || { echo "build produced no archive at $archive" >&2; exit 1; }

read -r pack_id version < <("$PYTHON" - "$pack_dir/manifest.json" <<'PY'
import json, sys
m = json.load(open(sys.argv[1]))
print(m["id"], m["version"])
PY
)
tag="${pack_id}-v${version}"
asset="${pack_id}-v${version}.onirispack"
sha_want=$(sha256sum "$archive" | cut -d' ' -f1)
size_want=$(stat -c %s "$archive")

out="dist/release/${pack_id}"
mkdir -p "$out"
cp "$archive" "$out/$asset"
printf '%s  %s\n' "$sha_want" "$asset" > "$out/SHA256SUMS"

echo "== pack $pack_id v$version"
echo "   archive $out/$asset"
echo "   sha256  $sha_want"
echo "   size    $size_want bytes"

# ------------------------------------------------------------- notes
"$PYTHON" - "$pack_dir/manifest.json" "$out/SHA256SUMS" "$tag" > "$out/release-notes.md" <<'PY'
import json, sys
manifest = json.load(open(sys.argv[1]))
sums = open(sys.argv[2]).read().strip().split("\n")[0]
tag = sys.argv[3]
name, version = manifest["name"]["en"], manifest["version"]
beds, events = manifest["beds"], manifest["events"]

print(f"# {name} — {version}")
print()
print(manifest["description"]["en"])
print()
print(f"- License: **{manifest['license']}**")
print(f"- Loudness: {manifest['loudnessLufs']} LUFS, true peak ≤ {manifest['truePeakDbtp']} dBTP")
print(f"- Contents: {len(beds)} bed(s), {len(events)} event(s)")
print(f"- Tags: {', '.join(manifest.get('tags', []))}")
print()
print("## How to install")
print()
print("1. Download the `.onirispack` asset below.")
print("2. In Oniris, open **Library → Import** and pick the file.")
print("3. Select it into the mix and press play.")
print()
print("## Beds")
print()
for bed in beds:
    loop = bed["loop"]
    seconds = (loop["endSample"] - loop["startSample"]) / 48000
    print(f"- `{bed['id']}`: {seconds:.1f} s loop, gain {bed['gainDb']:+.1f} dB")
print()
if events:
    print("## Events")
    print()
    for event in events:
        print(f"- `{event['id']}`: {len(event['files'])} file(s), "
              f"gain {event['gainDb'][0]:+.1f}..{event['gainDb'][1]:+.1f} dB, "
              f"every {event['minIntervalSec']:.0f}-{event['maxIntervalSec']:.0f} s")
    print()
print("## Credits")
print()
for credit in manifest.get("credits", []):
    print(f"- `{credit['asset'].split('/')[-1]}` — {credit['title']} by {credit['author']} "
          f"({credit['license']}, {credit['source']})")
print()
print("## Verification")
print()
print("The archive is a deterministic zip: repacking the same directory yields the same digest.")
print()
print("```")
print(sums)
print("```")
PY

# --------------------------------------------------------------- publish
repo="$(git remote get-url origin | sed -E 's|.*github\.com[:/]||; s|\.git$||')"

if [[ "$dry_run" == true ]]; then
  echo
  echo "== dry run: nothing published"
  echo "   release tag     $tag"
  echo "   release title   $(head -1 "$out/release-notes.md" | sed 's/^# //')"
  echo "   assets          $out/$asset, $out/SHA256SUMS"
  echo "   notes           $out/release-notes.md"
  if [[ "$(git rev-parse HEAD)" != "$(git rev-parse "origin/main" 2>/dev/null || echo none)" ]]; then
    echo "   NOTE: HEAD is not origin/main — push the branch before publishing,"
    echo "         the release tag must point at a pushed commit."
  fi
  exit 0
fi

token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
if [[ -z "$token" ]]; then
  token_file="${ONIRIS_GITHUB_TOKEN_FILE:-$HOME/.config/oniris/github-token}"
  if [[ -f "$token_file" ]]; then
    token="$(tr -d '\n\r' < "$token_file")"
  elif command -v gh >/dev/null 2>&1; then
    token="$(gh auth token 2>/dev/null || true)"
  fi
fi
if [[ -z "$token" ]]; then
  echo "no GitHub token: set GITHUB_TOKEN, or write one to ${ONIRIS_GITHUB_TOKEN_FILE:-$HOME/.config/oniris/github-token} (chmod 600)" >&2
  exit 1
fi

api() { curl -sS -H "Authorization: token $token" -H "Accept: application/vnd.github+json" "$@"; }

if [[ "$(git rev-parse HEAD)" != "$(git rev-parse "origin/main" 2>/dev/null || echo none)" ]]; then
  echo "HEAD is not origin/main: push the commit that contains this pack before releasing." >&2
  exit 1
fi

if api "https://api.github.com/repos/$repo/releases/tags/$tag" | grep -q '"id"'; then
  echo "release $tag already exists; bump the pack version in $spec" >&2
  exit 1
fi

body=$("$PYTHON" -c 'import json,sys; print(json.dumps(open(sys.argv[1]).read()))' "$out/release-notes.md")
payload=$("$PYTHON" - "$tag" "$body" "$prerelease" <<'PY'
import json, sys
tag, body, pre = sys.argv[1], json.loads(sys.argv[2]), sys.argv[3] == "true"
print(json.dumps({"tag_name": tag, "name": tag, "body": body,
                  "draft": False, "prerelease": pre}))
PY
)

echo "== creating release $tag on $repo"
release=$(api -X POST -H "Content-Type: application/json" \
  "https://api.github.com/repos/$repo/releases" -d "$payload")
release_id=$(printf '%s' "$release" | "$PYTHON" -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "   release id $release_id"

upload() {
  local file="$1" name="$2"
  echo "   uploading $name ($(stat -c %s "$file") bytes)"
  api -X POST -H "Content-Type: application/octet-stream" \
    --data-binary "@$file" \
    "https://uploads.github.com/repos/$repo/releases/$release_id/assets?name=$name" \
    >/dev/null
}
upload "$out/$asset" "$asset"
upload "$out/SHA256SUMS" "SHA256SUMS"

# ---------------------------------------------------------------- verify
echo "== verifying upload"
api "https://api.github.com/repos/$repo/releases/tags/$tag" | "$PYTHON" - "$size_want" "$sha_want" "$asset" <<'PY'
import json, sys
size_want, sha_want, asset = int(sys.argv[1]), sys.argv[2], sys.argv[3]
release = json.load(sys.stdin)
assets = {a["name"]: a for a in release.get("assets", [])}
ok = True
if asset not in assets:
    print(f"   FAIL {asset} is missing from the release")
    ok = False
else:
    got = assets[asset]
    digest = (got.get("digest") or "").replace("sha256:", "")
    print(f"   {asset}: {got['size']} bytes, state={got['state']}, digest={digest[:16]}…")
    if got["size"] != size_want:
        print(f"   FAIL size {got['size']} != local {size_want}")
        ok = False
    if digest and digest != sha_want:
        print(f"   FAIL digest {digest} != local {sha_want}")
        ok = False
if "SHA256SUMS" not in assets:
    print("   FAIL SHA256SUMS is missing")
    ok = False
print(f"   url: {release['html_url']}")
sys.exit(0 if ok else 1)
PY

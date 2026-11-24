#!/usr/bin/env bash
# Publish the signed beta APK as a GitHub release.
#
# Usage:
#   scripts/publish-beta-apk.sh              # publish dist/oniris-beta.apk as v<versionName>
#   scripts/publish-beta-apk.sh --dry-run    # print what would be published
#
# `make beta` builds and signs the APK (dist/oniris-beta.apk + .sha256); this
# script only publishes. The tag is the APK's versionName (v1.0.0-beta8), the
# release is a prerelease, and assets keep the historical names so tester links
# and scripts keep working: oniris-beta.apk and oniris-beta.apk.sha256.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
inside_git_repo() { git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; }
dry_run=false
[[ "${1:-}" == "--dry-run" ]] && dry_run=true
cd "$root"

apk="dist/oniris-beta.apk"
sidecar="$apk.sha256"
[[ -f "$apk" ]] || { echo "no APK at $apk — run 'make beta' first" >&2; exit 1; }
[[ -f "$sidecar" ]] || { echo "no digest at $sidecar" >&2; exit 1; }

sdk="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
build_tools=""
if [[ -n "$sdk" && -d "$sdk/build-tools" ]]; then
  build_tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"
fi
if [[ -z "$build_tools" ]]; then
  echo "ANDROID_SDK_ROOT must point to the Android SDK to read the APK version" >&2
  exit 1
fi

badging="$("$build_tools/aapt" dump badging "$apk")"
version_name="$(sed -n "s/^package: .*versionName='\\([^']*\\)'.*/\\1/p" <<<"$badging" | head -1)"
version_code="$(sed -n "s/^package: .*versionCode='\\([^']*\\)'.*/\\1/p" <<<"$badging" | head -1)"
[[ -n "$version_name" ]] || { echo "could not read versionName from $apk" >&2; exit 1; }
tag="v$version_name"
sha="$(sha256sum "$apk" | cut -d' ' -f1)"
recorded="$(tr -d '[:space:]' < "$sidecar")"
if [[ "$sha" != "$recorded" ]]; then
  echo "APK digest $sha does not match $sidecar ($recorded) — rebuild before publishing" >&2
  exit 1
fi
size="$(stat -c %s "$apk")"

echo "== APK $version_name (versionCode $version_code), $size bytes, sha256 $sha"
echo "== tag $tag"

if [[ "$dry_run" == true ]]; then
  echo "== dry run: nothing published"
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
[[ -n "$token" ]] || { echo "no GitHub token (GITHUB_TOKEN, $HOME/.config/oniris/github-token, or gh auth)" >&2; exit 1; }

api() { curl -sS -H "Authorization: token $token" -H "Accept: application/vnd.github+json" "$@"; }
repo="$(git remote get-url origin | sed -E 's|.*github\.com[:/]||; s|\.git$||')"

head_sha="$(git rev-parse HEAD)"
origin_sha="$(git rev-parse "origin/main" 2>/dev/null || echo none)"
if [[ "$head_sha" != "$origin_sha" ]]; then
  echo "HEAD ($head_sha) is not origin/main ($origin_sha): push before releasing," >&2
  echo "otherwise the tag points at code that did not build this APK." >&2
  exit 1
fi

existing="$(mktemp)"
trap 'rm -f "$existing"' EXIT
api -o "$existing" -w '%{http_code}' "https://api.github.com/repos/$repo/releases/tags/$tag" >/dev/null || true
if grep -q '"id"' "$existing"; then
  echo "release $tag already exists; bump versionName/versionCode and rebuild" >&2
  exit 1
fi

notes_file="$(mktemp)"
{
  echo "## Build"
  echo
  echo "- versionName \`$version_name\`, versionCode \`$version_code\`"
  echo "- signed beta APK, $((size / 1024 / 1024)) MB"
  echo "- sha256 \`$sha\`"
  echo
  echo "## Install"
  echo
  echo '```'
  echo "adb install -r oniris-beta.apk"
  echo '```'
  echo
  echo "Recorded packs (Rainy Night, Seaside, Forest Deep) download from the"
  echo "Library in the app; see \`docs/BETA.md\`."
  echo
  echo "## Verification"
  echo
  echo '```'
  echo "$sha  oniris-beta.apk"
  echo '```'
} > "$notes_file"

body="$(python3 -c 'import json,sys; print(json.dumps(open(sys.argv[1]).read()))' "$notes_file")"
payload="$(python3 - "$tag" "$body" <<'PY'
import json, sys
print(json.dumps({"tag_name": sys.argv[1], "name": "Oniris " + sys.argv[1][1:],
                  "body": json.loads(sys.argv[2]), "draft": False, "prerelease": True}))
PY
)"

echo "== creating release $tag on $repo"
release="$(api -X POST -H "Content-Type: application/json" \
  "https://api.github.com/repos/$repo/releases" -d "$payload")"
release_id="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])' <<<"$release")"

for file in "$apk" "$sidecar"; do
  name="$(basename "$file")"
  echo "   uploading $name ($(stat -c %s "$file") bytes)"
  api -X POST -H "Content-Type: application/octet-stream" --data-binary "@$file" \
    "https://uploads.github.com/repos/$repo/releases/$release_id/assets?name=$name" >/dev/null
done

echo "== verifying upload"
release_json="$(mktemp)"
api -o "$release_json" "https://api.github.com/repos/$repo/releases/tags/$tag"
python3 - "$release_json" "$size" "$sha" <<'PY'
import json, sys
release = json.load(open(sys.argv[1]))
size_want, sha_want = int(sys.argv[2]), sys.argv[3]
assets = {a["name"]: a for a in release.get("assets", [])}
ok = True
for name in ("oniris-beta.apk", "oniris-beta.apk.sha256"):
    if name not in assets:
        print(f"   FAIL {name} missing")
        ok = False
apk = assets.get("oniris-beta.apk")
if apk:
    digest = (apk.get("digest") or "").replace("sha256:", "")
    print(f"   oniris-beta.apk: {apk['size']} bytes, state={apk['state']}, digest={digest[:16]}…")
    if apk["size"] != size_want or (digest and digest != sha_want):
        print("   FAIL size or digest differs from the local APK")
        ok = False
print(f"   url: {release['html_url']}")
raise SystemExit(0 if ok else 1)
PY

#!/usr/bin/env bash
# Build, sign, and verify an internal beta APK.
#
# Requires JAVA_HOME (JDK 17/21), ANDROID_SDK_ROOT (or ANDROID_HOME), and
# ANDROID_NDK_HOME (or an NDK installed under the SDK). The signing key is kept
# outside the repository so it survives clean builds.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
keydir="${ONIRIS_BETA_KEYDIR:-$HOME/.oniris-beta}"
keystore="$keydir/oniris-beta.jks"
credentials="$keydir/credentials.txt"
alias_name="oniris-beta"

if [[ -z "${JAVA_HOME:-}" || ! -x "$JAVA_HOME/bin/keytool" ]]; then
  echo "JAVA_HOME must point to a JDK 17 or 21 installation" >&2
  exit 1
fi

sdk="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
if [[ -z "$sdk" || ! -d "$sdk/build-tools" ]]; then
  echo "ANDROID_SDK_ROOT (or ANDROID_HOME) must point to the Android SDK" >&2
  exit 1
fi
build_tools="$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -1)"

mkdir -p "$keydir"
chmod 700 "$keydir"
if [[ ! -f "$keystore" ]]; then
  password="$(openssl rand -hex 24)"
  "$JAVA_HOME/bin/keytool" -genkeypair -v \
    -keystore "$keystore" -storetype PKCS12 \
    -alias "$alias_name" -keyalg RSA -keysize 4096 -validity 10000 \
    -storepass "$password" -keypass "$password" \
    -dname "CN=Oniris Beta, OU=Internal Testing, O=Oniris, C=US" >/dev/null
  printf 'keystore=%s\nalias=%s\nstore_password=%s\nkey_password=%s\n' \
    "$keystore" "$alias_name" "$password" "$password" > "$credentials"
  chmod 600 "$keystore" "$credentials"
  echo "generated beta signing key at $keystore"
fi
password="$(grep '^store_password=' "$credentials" | cut -d= -f2-)"

make -C "$root" android-native android-packs
(
  cd "$root/android"
  ./gradlew :app:assembleRelease
)

unsigned="$root/android/app/build/outputs/apk/release/app-release-unsigned.apk"
aligned="$root/android/app/build/outputs/apk/release/app-release-aligned.apk"
if "$build_tools/zipalign" -c -v 4 "$unsigned" >/dev/null 2>&1; then
  cp "$unsigned" "$aligned"
else
  "$build_tools/zipalign" -p -f 4 "$unsigned" "$aligned"
fi

mkdir -p "$root/dist"
signed="$root/dist/oniris-beta.apk"
"$build_tools/apksigner" sign \
  --ks "$keystore" --ks-key-alias "$alias_name" \
  --ks-pass "pass:$password" --key-pass "pass:$password" \
  --out "$signed" "$aligned"
"$build_tools/apksigner" verify --verbose "$signed"

version="$("$build_tools/aapt" dump badging "$signed" | sed -n "s/^package: .*versionName='\([^']*\)'.*/\1/p" | head -1)"
code="$("$build_tools/aapt" dump badging "$signed" | sed -n "s/^package: .*versionCode='\([^']*\)'.*/\1/p" | head -1)"
sha="$(shasum -a 256 "$signed" | awk '{print $1}')"
printf '%s\n' "$sha" > "$signed.sha256"

echo
echo "beta APK: $signed"
echo "version: $version ($code)"
echo "sha256:  $sha"
echo "install: adb install -r '$signed'"

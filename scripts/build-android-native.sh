#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "cargo-ndk not found. Install it with: cargo install cargo-ndk" >&2
  exit 1
fi

rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

cd core
cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o ../android/app/src/main/jniLibs \
  build -p oniris-jni --release

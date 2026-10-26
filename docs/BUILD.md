# Build notes

## Verified toolchain

The scaffold is built and tested with:

- Rust 1.98 (edition 2021, MSRV 1.88)
- Temurin JDK 21
- Gradle 8.14.3 (wrapper), AGP 8.13.2, Kotlin 2.4.21
- Android SDK 36, build-tools 36.0.0
- NDK 28.2.13676358, `cargo-ndk` 4.x

Newer AndroidX releases require compileSdk 37 and AGP 9.1+, so the dependency
catalog is intentionally pinned to the newest versions that work with SDK 36.

## Requirements

- Rust stable (1.88+)
- JDK 17 or 21 (AGP does not officially support JDK 25 yet)
- Android SDK 36 and build-tools 36.0.0
- `cargo-ndk`
- Android Rust targets: `aarch64-linux-android`, `armv7-linux-androideabi`,
  `x86_64-linux-android`

## Rust core

```bash
cargo test --manifest-path core/Cargo.toml --workspace
cargo clippy --manifest-path core/Cargo.toml --workspace --all-targets
cargo fmt --manifest-path core/Cargo.toml --all --check
```

Generate test content and render a mix:

```bash
# Ogg/Opus by default, matching production assets
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-test-packs packs
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- validate packs/test-rain
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  render --pack packs/test-rain --pack packs/test-fire \
  --minutes 10 --out /tmp/oniris-test.wav
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- analyze /tmp/oniris-test.wav

# Lossless WAV fallback for codec debugging
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  gen-test-packs /tmp/oniris-wav --format wav

# Encode a WAV master as a gapless 48 kHz Ogg/Opus asset
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  encode-opus master.wav master.opus --bitrate 96000

# Package and verify a distributable archive
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  pack packs/test-rain /tmp/test-rain.onirispack
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  verify-pack /tmp/test-rain.onirispack
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  unpack /tmp/test-rain.onirispack /tmp/test-rain
```

Ogg/Opus uses the pure-Rust `opus-pure` crate: no libopus, CMake, or NDK codec
build is required. Archives use the pure-Rust `zip` crate. Generated audio is
ignored by git.

## Native library for Android

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk

cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o ../android/app/src/main/jniLibs \
  build -p oniris-jni --release
```

Run this from `core/`; `cargo-ndk` does not accept `--manifest-path`.

This produces `liboniris_jni.so` under
`android/app/src/main/jniLibs/<abi>/`.

`jniLibs` is git-ignored: native libraries are build artifacts. Build them
before assembling the app, or the UI will run in a "native engine unavailable"
state.

## Android app

```bash
cd android
./gradlew :app:assembleDebug
```

Install on a device:

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

The Gradle wrapper is committed. The first build downloads Gradle and the
Android/Compose dependencies.

### Local SDK setup

If `ANDROID_HOME` is not set, create `android/local.properties` (git-ignored):

```properties
sdk.dir=/path/to/Android/Sdk
```

Install the SDK packages with Android Studio's SDK Manager or:

```bash
sdkmanager "platforms;android-36" "build-tools;36.0.0" "platform-tools"
```

## Content

Put development packs (Ogg/Opus or WAV) in
`android/app/src/main/assets/packs/<id>/` to have the app copy them to internal
storage on first launch, or push them with:

```bash
adb push packs/test-rain /sdcard/Android/data/dev.oniris/files/packs/test-rain
```

The scaffold's pack installer and import UI are intentionally minimal; see the
roadmap.

## Troubleshooting

- **`UnsatisfiedLinkError` / native engine unavailable:** rebuild `jniLibs` for
  the device ABI.
- **Gradle JDK error:** set `org.gradle.java.home` to a JDK 17/21 installation,
  or select it in Android Studio.
- **`compileSdk 36` not found:** install platform 36 in the SDK Manager.
- **Audio starts then stops:** check notification permission and the
  foreground-service declaration; inspect `adb logcat -s Oniris`.

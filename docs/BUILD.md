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

# Procedural packs: Pure Noise, synthetic Rainy Night, Deep Space Cabin, Enchanted Forest
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-procedural-packs packs
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- credits packs/deep-space-cabin

# Generate one named procedural recipe
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  gen-procedural-packs packs --pack enchanted-forest

# Pure Noise only
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-noise-pack packs

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

# Measure EBU R128 loudness/true peak, then normalize to -23 LUFS / -1.5 dBTP
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  loudness master.wav
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  normalize master.wav master-normalized.wav --target-lufs -23 --true-peak -1.5

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

The simplest path builds the native library, generates/bundles the procedural
packs into `assets/packs`, and assembles the debug APK:

```bash
make android
```

Or run the steps separately:

```bash
make android-native   # native library into android/app/src/main/jniLibs
make android-packs    # procedural packs into android/app/src/main/assets/packs
cd android && ./gradlew :app:assembleDebug
```

Install on a device:

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

## Beta APK

For real-device testing, build a release-mode (R8, no debug overhead) beta APK:

```bash
export JAVA_HOME=/path/to/jdk-21
export ANDROID_SDK_ROOT=/path/to/Android/Sdk
export ANDROID_NDK_HOME="$ANDROID_SDK_ROOT/ndk/28.2.13676358"
make beta
```

`make beta` rebuilds the native libraries and bundled packs, assembles the
release APK, signs it with a beta key, verifies the signature, and writes:

- `dist/oniris-beta.apk`
- `dist/oniris-beta.apk.sha256`

The signing key lives outside the repository at `~/.oniris-beta/oniris-beta.jks`
(override with `ONIRIS_BETA_KEYDIR`); keep it to sign updates for the same
install. This is an internal test key, not a Play Store release key.

```bash
adb install -r dist/oniris-beta.apk
```

See [`BETA.md`](BETA.md) for the tester install steps, use guide, test pass,
and known limitations.

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

`make android-packs` runs `scripts/bundle-android-packs.sh`, which generates the
five procedural packs and copies them to
`android/app/src/main/assets/packs/<id>/`. `PackRepository` copies them to the
app's internal `files/packs` directory and refreshes them when the app version
changes, while leaving imported and user packs untouched.

To try an import without rebuilding, generate an archive and pick it from the
app's **Import** button:

```bash
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  pack packs/enchanted-forest /tmp/enchanted-forest.onirispack
adb push /tmp/enchanted-forest.onirispack /sdcard/Download/
```

The importer validates the zip, decodes every asset, and publishes it under
`filesDir/packs/<id>`. A failed or duplicate import leaves the existing library
untouched. Packs pushed with `adb push .../files/packs/<id>` still work during
development.

## Troubleshooting

- **`UnsatisfiedLinkError` / native engine unavailable:** rebuild `jniLibs` for
  the device ABI.
- **Gradle JDK error:** set `org.gradle.java.home` to a JDK 17/21 installation,
  or select it in Android Studio.
- **`compileSdk 36` not found:** install platform 36 in the SDK Manager.
- **Audio starts then stops:** check notification permission and the
  foreground-service declaration; inspect `adb logcat -s Oniris`.

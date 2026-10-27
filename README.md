# Oniris

Oniris is a battery-first Android sleep-soundscape app. It mixes seamless ambient beds with randomized one-shot events (thunder, fire crackles, gulls, owls) and is designed to run all night without waking the CPU for decoding, networking, or timers.

The audio engine is written in Rust so the same sound model can run on Android today and on desktop, Raspberry Pi, or the web later.

**Status:** v1.0 feature set is nearly complete. The Rust core, pack format, deterministic `.onirispack` archives, attribution credits, four CC0 procedural packs (Pure Noise, Rainy Night, Deep Space Cabin, Enchanted Forest), CLI, Android shell, Ogg/Opus decode/encode, Android pack import, per-pack volume, presets, live mix crossfades, lock-screen metadata, and bundled-pack build step are in place, and the Android debug and release APKs build with JDK 21 + SDK 36. Field-recorded sound packs, pack previews/downloads, real-device listening/battery QA, OEM soak testing, code signing, and the Media3 adapter remain.

## What works today

- Rust core: manifest validation, Ogg/Opus and WAV decode/resample, seamless bed loops, randomized event voices, live mix crossfades, click-free per-pack gain ramps, LFOs, fades, sleep timer, lookahead limiter.
- `oniris` CLI: generate synthetic Opus/WAV test packs, generate procedural packs, encode Opus, validate, print credits, render, analyze, measure loudness/true peak, normalize, and package/verify/unpack `.onirispack` archives.
- `oniris analyze`: peak, RMS, maximum sample-to-sample delta, and large-jump counts for render QA.
- Android app: pack library, `.onirispack` import through the system document picker, multi-pack selection (up to 4) with per-pack volume, master volume, procedural-only library mode, built-in and user-saved presets, sleep timer, live mix changes without rebuilding the engine, foreground media notification with the active mix name, audio focus, wake lock, persisted settings.
- Native library builds for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; debug and release APKs assemble.
- `make android` generates four procedural packs and bundles them into the APK, so a fresh install has content without network access.

Smoke-tested on an Android 36 x86_64 emulator: pack discovery, play/pause, media-session state, foreground notification, and wake-lock acquire/release all work. Opus decode, per-pack gain ramps, archive import, and golden render tests (level, brightness, loop seams, determinism) are covered by the Rust suite; on-device audio, live-mix listening, and battery soak tests are still pending. The codec is pure Rust (`opus-pure`), so Android builds need no extra native library.

## Why this architecture

- **Decode once, mix from memory.** Packs are decoded when a mix starts. During playback the app only mixes PCM and writes one `AudioTrack`. There are no live decoders, file reads, or network calls at night.
- **Portable core.** DSP, scheduling, fades, and pack parsing live in `core/` and have no Android dependencies.
- **Open pack format.** A pack is a directory/zip with a versioned `manifest.json`, Opus/WAV audio, and art. New content does not require app code.
- **Thin Android shell.** Kotlin owns the foreground media service, notification/lock screen, audio focus, and Compose UI.

## Repository layout

```
android/                 Android app (Kotlin, Compose, media service)
core/                    Rust workspace
  oniris-pack/           Manifest types, validation, archives, path safety
  oniris-core/           Decoder, mixer, scheduler, limiter, sleep timer
  oniris-cli/            render / validate / credits / analyze / encode-opus / loudness / normalize / pack
  oniris-jni/            JNI bridge for Android
docs/                    Plan, architecture, pack format, content sources, procedural audio, desktop plan
packs/                   Generated test packs (ignored by git)
tools/                   Asset pipeline (placeholder)
scripts/                 Helper scripts
```

## Quick start (Rust)

```bash
# Build and test the core
cargo test --manifest-path core/Cargo.toml --workspace

# Generate synthetic test packs (rain + fire); Ogg/Opus by default
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-test-packs packs

# Generate procedural packs: noise, synthetic rain, space cabin, enchanted forest
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-procedural-packs packs

# Generate just one recipe
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  gen-procedural-packs packs --pack deep-space-cabin

# Validate a pack and print its attribution credits
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- validate packs/deep-space-cabin
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- credits packs/deep-space-cabin

# Lossless fallback for debugging or codec A/B tests
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  gen-test-packs /tmp/oniris-wav-packs --format wav

# Validate a pack (decodes every asset)
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- validate packs/test-rain

# Encode a WAV master as a gapless 48 kHz Opus asset
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  encode-opus path/to/master.wav /tmp/master.opus --bitrate 96000

# Measure EBU R128 loudness and true peak
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- loudness /tmp/oniris-test.wav

# Normalize to -23 LUFS while keeping true peak at or below -1.5 dBTP
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  normalize path/to/master.wav /tmp/master-normalized.wav \
  --target-lufs -23 --true-peak -1.5

# Package, verify, and unpack a distributable archive
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  pack packs/test-rain /tmp/test-rain.onirispack
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  verify-pack /tmp/test-rain.onirispack
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  unpack /tmp/test-rain.onirispack /tmp/test-rain-unpacked

# Render a 10-minute mix to WAV
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  render --pack packs/test-rain --pack packs/test-fire --minutes 10 --out /tmp/oniris-test.wav

# Inspect a render
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- analyze /tmp/oniris-test.wav
```

Test packs are Ogg/Opus by default; `--format wav` keeps a lossless fallback. Production packs are Opus too. Decoding and encoding use the pure-Rust `opus-pure` crate, so the core and Android builds need no system codec, FFI, or CMake.

## Android build

Prerequisites: JDK 17 or 21, Android SDK 36, Rust, `cargo-ndk`, and the Android Rust targets.

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
cargo install cargo-ndk

# Builds jniLibs, generates/bundles the procedural packs, and assembles the APK.
make android

# Builds, signs, verifies, and writes dist/oniris-beta.apk for device testing.
make beta
```

Or run the steps separately:

```bash
make android-native   # native library into android/app/src/main/jniLibs
make android-packs    # procedural packs into android/app/src/main/assets/packs
cd android && ./gradlew :app:assembleDebug
```

The app starts gracefully if `liboniris_jni.so` is missing, but playback requires the native library. The native library is also required for `.onirispack` import: the importer reuses the Rust archive extractor and validates every asset before installing it. See [docs/BUILD.md](docs/BUILD.md).

## Documentation

- [Development plan](docs/DEVELOPMENT_PLAN.md)
- [Beta tester guide](docs/BETA.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Pack format](docs/PACK_FORMAT.md)
- [Manifest JSON Schema](docs/manifest.schema.json)
- [Build notes](docs/BUILD.md)
- [Content licensing](docs/LICENSING.md)

## Decisions locked for v1

- Rust audio core, Kotlin/Compose Android shell.
- Free app; no ads, accounts, or subscription.
- Royalty-free (CC0/CC-BY with attribution) or self-authored content.
- Phone/tablet first.
- Desktop and web are portability targets for the same core.

Code license and final content licenses still need to be chosen before a public release.

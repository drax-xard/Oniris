# Oniris

Oniris is a battery-first Android sleep-soundscape app. It mixes seamless ambient beds with randomized one-shot events (thunder, fire crackles, gulls, owls) and is designed to run all night without waking the CPU for decoding, networking, or timers.

The audio engine is written in Rust so the same sound model can run on Android today and on desktop, Raspberry Pi, or the web later.

**Status:** early scaffold, with the production audio format and pack distribution working end to end. The Rust core, pack format, deterministic `.onirispack` archives, CLI, Android shell, and Ogg/Opus decode/encode are in place, and the Android debug APK builds with JDK 21 + SDK 36. Real sound packs, Android pack import UI, and on-device/battery validation are next.

## What works today

- Rust core: manifest validation, Ogg/Opus and WAV decode/resample, seamless bed loops, randomized event voices, LFOs, fades, sleep timer, lookahead limiter.
- `oniris` CLI: generate synthetic Opus/WAV test packs, encode Opus, validate, render, analyze, measure loudness/true peak, normalize, and package/verify/unpack `.onirispack` archives.
- `oniris analyze`: peak, RMS, maximum sample-to-sample delta, and large-jump counts for render QA.
- Android app: pack library, multi-pack selection, volume, sleep timer, foreground media notification, audio focus, wake lock, persisted settings.
- Native library builds for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; debug and release APKs assemble.

Smoke-tested on an Android 36 x86_64 emulator: pack discovery, play/pause, media-session state, foreground notification, and wake-lock acquire/release all work. Opus decode and golden render tests (level, brightness, loop seams, determinism) are covered by the Rust suite; on-device audio and battery soak tests are still pending. The codec is pure Rust (`opus-pure`), so Android builds need no extra native library.

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
  oniris-cli/            render / validate / analyze / encode-opus / loudness / normalize / pack
  oniris-jni/            JNI bridge for Android
docs/                    Plan, architecture, pack format, build notes
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

cd core
cargo ndk \
  -t arm64-v8a -t armeabi-v7a -t x86_64 \
  -o ../android/app/src/main/jniLibs \
  build -p oniris-jni --release

cd ..
cd android
./gradlew :app:assembleDebug
```

The app starts gracefully if `liboniris_jni.so` is missing, but playback requires the native library. See [docs/BUILD.md](docs/BUILD.md).

## Documentation

- [Development plan](docs/DEVELOPMENT_PLAN.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Pack format](docs/PACK_FORMAT.md)
- [Build notes](docs/BUILD.md)
- [Content licensing](docs/LICENSING.md)

## Decisions locked for v1

- Rust audio core, Kotlin/Compose Android shell.
- Free app; no ads, accounts, or subscription.
- Royalty-free (CC0/CC-BY with attribution) or self-authored content.
- Phone/tablet first.
- Desktop and web are portability targets for the same core.

Code license and final content licenses still need to be chosen before a public release.

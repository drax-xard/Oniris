# Oniris

Oniris is a battery-first Android sleep-soundscape app. It mixes seamless ambient beds with randomized one-shot events (thunder, fire crackles, gulls, owls) and is designed to run all night without waking the CPU for decoding, networking, or timers.

The audio engine is written in Rust so the same sound model can run on Android today and on desktop, Raspberry Pi, or the web later.

**Status:** early scaffold. The Rust core, pack format, CLI, and Android shell are in place, and the Android debug APK builds with JDK 21 + SDK 36. The first real sound packs and the production Opus decoder are next.

## Why this architecture

- **Decode once, mix from memory.** Packs are decoded when a mix starts. During playback the app only mixes PCM and writes one `AudioTrack`. There are no live decoders, file reads, or network calls at night.
- **Portable core.** DSP, scheduling, fades, and pack parsing live in `core/` and have no Android dependencies.
- **Open pack format.** A pack is a directory/zip with a versioned `manifest.json`, Opus/WAV audio, and art. New content does not require app code.
- **Thin Android shell.** Kotlin owns the foreground media service, notification/lock screen, audio focus, and Compose UI.

## Repository layout

```
android/                 Android app (Kotlin, Compose, media service)
core/                    Rust workspace
  oniris-pack/           Manifest types, validation, path safety
  oniris-core/           Decoder, mixer, scheduler, limiter, sleep timer
  oniris-cli/            render / validate / analyze / gen-test-packs
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

# Generate synthetic test packs (rain + fire)
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- gen-test-packs packs

# Validate a pack
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- validate packs/test-rain

# Render a 10-minute mix to WAV
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  render --pack packs/test-rain --pack packs/test-fire --minutes 10 --out /tmp/oniris-test.wav

# Inspect a render
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- analyze /tmp/oniris-test.wav
```

The generated test packs use WAV so the scaffold works without native codec dependencies. Production packs will be Opus; the decoder is behind an explicit trait so it can be added without changing the mixer.

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

# Oniris — Development Plan

**Status:** Draft v0.3 — decisions locked, core and Android scaffold working  
**Product:** Android sleep-soundscape app with a portable audio core and an open sound-pack format.  
**Working name:** Oniris (Oneiros = dream).  
**Goal:** Mix seamless, non-repetitive soundscapes, run all night on battery, and make new content cheap to add.

**Progress (Oct 2026):** Rust core, pack spec, CLI, JNI, Android shell,
Ogg/Opus decode/encode, deterministic `.onirispack` packaging/extraction, EBU
R128 loudness/true-peak measurement and normalization, live mix crossfades,
manifest attribution credits, and a generated **Pure Noise** default pack are
implemented and build. The debug APK runs on an Android 36 emulator: pack
discovery, play/pause, media session, foreground notification, audio focus, and
wake lock all work. Real field-recorded content, Android pack import, OEM
testing, listening QA, and the 8-hour battery soak remain.

**Local QA (Oct 2026):** a 60-minute rain+fire render completes in about 6 s with
~17 MB peak RSS on the development machine. `oniris-core/tests/golden.rs` covers
level, brightness, loop continuity, and seed determinism; `oniris analyze`
reports peak, RMS, max sample delta, and large-jump counts.

## Locked decisions (v1)

1. **Core:** Rust (`core/oniris-core`), with a C ABI/JNI adapter. Kotlin/Compose Android shell.
2. **Business model:** free; no ads, accounts, or subscription.
3. **Content:** royalty-free (CC0, or CC-BY with attribution) or self-authored recordings.
4. **Device scope:** phones and tablets for v1.0.
5. **Portability:** desktop and web are targets for the same core; Android is the reference client.
6. **Code license:** still to be chosen before public release. Content licenses are per pack.

---

## 1. Product thesis

Most sleep-sound apps are either:

- simple loop players with audible repetition and no real mixing, or
- heavy streaming apps with accounts, ads, analytics, and network wakeups.

Oniris should win on three things:

1. **Battery and reliability.** Decode once at load, mix from memory, output one stream, keep the CPU near idle for 8 hours.
2. **Sound quality.** Seamless loops, randomized events, slow modulation, loudness-matched packs, no clicks or jump-scares.
3. **Extensibility.** A documented pack format plus a portable Rust engine means new sounds, other devices, and community content do not require rewriting the app.

**Non-goals for v1.0:** sleep tracking, social features, accounts, cloud sync, AI generation, smart alarms, ads.

---

## 2. Product principles

1. **Battery first.** Every feature must justify its cost in wakeups, CPU, or network. No analytics, no polling, no visualizer while playing with the screen off.
2. **Sound first.** A sleep app is judged after 20 minutes, not in the first 10 seconds. Variation and loop quality matter more than a large catalog.
3. **Offline by default.** The app works in airplane mode forever. Downloads are optional and explicit.
4. **One core, many shells.** DSP and scheduling live in a platform-neutral core. Android, desktop, Raspberry Pi, and web are adapters.
5. **Calm UI.** Dark, quiet, one-handed, no streaks, no badges, no engagement mechanics.
6. **No surprises at 3 a.m.** Audio focus loss, Bluetooth disconnect, process death, and low battery must all fail gracefully.

---

## 3. Scope

### 3.1 v1.0 (launch)

- 3 bundled packs, 5–7 downloadable packs.
- Mix up to 4 packs with independent volume; master volume.
- Seamless beds + randomized one-shot events (thunder, crackles, gulls, owls).
- Play/pause, fade-in, fade-out.
- Sleep timer (15/30/45/60/90 min, custom) with 60-second fade-out.
- Background playback via foreground media service.
- Lock-screen/notification controls and metadata.
- Audio focus: pause/duck for calls and other media; optional auto-resume.
- Presets: 4 built-in + user-saved mixes.
- Pack library with tags, previews, and add/remove.
- Dark AMOLED UI, screen-off friendly.
- Battery saver behavior, headphone-disconnect handling.
- Licenses/attribution screen; no network permission required for playback.
- Android 8.0+ (minSdk 26), target SDK 36.

### 3.2 v1.1–v1.2

- Adaptive "night curve": packs slowly change level/event density over the night.
- Randomize mix from tags, with compatibility rules.
- Quick Settings tile and home-screen widget.
- Custom pack import (`.onirispack`) and pack validation.
- Desktop pack editor/preview CLI.
- Localization infrastructure (English first).
- Optional opt-in crash reporting (Play Console vitals only by default).

### 3.3 Later / research

- Smart wake: gentle fade-in before an alarm; requires alarm integration and permissions.
- Generative layers (procedural rain/wind/fire) for infinite variation and tiny downloads.
- Wear OS / Android TV / Android Auto controls.
- Chromecast / multi-room.
- Snore masking / adaptive EQ.
- Breathing pacer and wind-down routines.
- Export a rendered mix to a file.
- Headless Raspberry Pi player using the same core.

---

## 4. Feature map

| Area | Feature | Priority | Why / notes |
|---|---|---|---|
| Playback | Mix 1–4 packs | P0 | Core product. Per-pack gain, master gain. |
| Playback | Seamless loops + random events | P0 | Prevents the "2-minute loop" effect. |
| Playback | Fade in/out, click-free pause | P0 | Sleep UX; avoids startling. |
| Playback | Sleep timer with fade | P0 | Expected feature; sample-accurate in engine. |
| Platform | Foreground media service | P0 | Survives screen-off/doze; required for all-night use. |
| Platform | MediaSession + notification + lock screen | P0 | User control without unlocking. |
| Platform | Audio focus / calls / Bluetooth disconnect | P0 | Reliability. |
| Platform | Wake lock only while playing | P0 | Verify; audio HAL may already hold one. |
| Content | Pack library + previews | P0 | Discovery and mixing. |
| Content | 8–10 packs at launch | P0 | Enough variety; quality over quantity. |
| Content | Loudness normalization | P0 | Mixing must not require constant volume fiddling. |
| UX | Presets + last mix restore | P0 | Bedtime flow should be 2 taps. |
| UX | Dark/AMOLED, large controls | P0 | Screen is often bright in a dark room. |
| UX | Quick tile / widget | P1 | Fast bedtime start; v1.1. |
| UX | Randomize / suggest mix | P1 | Fun, low cost; v1.1. |
| UX | Adaptive night curve | P1 | Differentiator; fights habituation. |
| Content | Downloadable packs | P1 | Keeps APK small; optional network. |
| Content | Custom pack import | P2 | Community/extensibility; v1.2. |
| Advanced | Smart wake | P3 | High complexity/permissions; later. |
| Advanced | Generative layers | P3 | Research spike; CPU/quality risk. |
| Advanced | Wear/TV/Auto | P3 | Niche; only after phone app is solid. |

**Deliberately excluded from v1:** accounts, social, sleep tracking, ads, subscriptions, cloud sync, in-app visualizers that run with the screen off.

---

## 5. Architecture

### 5.1 Shape

```
┌──────────────────────────────────────────────┐
│ Compose UI (library, mixer, timer, settings) │
└──────────────────────┬───────────────────────┘
                       │ StateFlow / intents
┌──────────────────────▼───────────────────────┐
│ PlaybackService (MediaSessionService)         │
│  ├─ MediaSession (notification, lock screen)  │
│  ├─ SoundscapePlayer (SimpleBasePlayer)       │
│  ├─ AudioFocus handling                       │
│  └─ AudioRenderThread ── AudioTrack           │
└──────────────────────┬───────────────────────┘
                       │ JNI (coarse calls only)
┌──────────────────────▼───────────────────────┐
│ oniris-core (Rust, no Android dependencies)   │
│  ├─ Pack loader (Opus → PCM, resample)        │
│  ├─ Mixer (beds + event voices, LFOs, pan)    │
│  ├─ Scheduler (random events, fade envelopes) │
│  ├─ Sleep timer + master limiter              │
│  └─ Render API: render_i16(buffer, frames)    │
└──────────────────────┬───────────────────────┘
                       │ std::fs / file paths
┌──────────────────────▼───────────────────────┐
│ Pack files (manifest.json + Opus + art)       │
└──────────────────────────────────────────────┘
```

### 5.2 The key audio decision: decode once, mix from memory

For a sleep app, playback is long and interaction is rare. The cheapest architecture is:

1. On mix start, decode every bed and event into 16-bit PCM in memory.
2. During playback, only mix PCM and write to `AudioTrack`.
3. Never decode, resample, or allocate on the audio thread.

Costs and benefits:

- Beds are 20–45 s; 4 packs × 1 bed ≈ 23 MB. Events add ~10–20 MB. Typical load: 35–60 MB, capped at 96 MB.
- One-time decode is fast (tens of ms) and can be done off the audio thread.
- During 8 hours of playback there are zero decoders, zero file reads, and zero network calls.
- CPU is dominated by memory reads and a few multiply-adds: well under 2% of one core on modern phones.
- This is dramatically lighter than running 3–4 ExoPlayer instances with live decoders all night.

The tradeoff is memory and a short load delay when changing mixes. Both are acceptable for this product. The engine crossfades live `set_mix` changes over 2 s, so the only audible gap is the time needed to decode newly added packs. If a pack grows beyond the budget, the loader can stream that one bed later without changing the API.

### 5.3 Why a Rust core

- One implementation for Android, Linux/Pi, macOS, Windows, and eventually WASM.
- Memory safety in a component that handles untrusted packs.
- Easy offline testing: the same engine renders a WAV in CI and on a phone.
- Clean C ABI and JNI shim; Android is not baked into the DSP.

**Fallback:** if the native build becomes a schedule risk, a Kotlin/Media3 prototype can validate UX, but the production engine should remain the portable core. Avoid writing two full mixers.

### 5.4 Core API sketch

```rust
pub struct Engine { /* ... */ }

impl Engine {
    pub fn new(sample_rate: u32) -> Self;
    pub fn load_pack(&mut self, dir: &Path) -> Result<PackId, LoadError>;
    pub fn unload_pack(&mut self, id: PackId);
    pub fn set_mix(&mut self, mix: &MixSpec) -> Result<(), MixError>;
    pub fn set_pack_gain(&mut self, id: PackId, gain: f32);
    pub fn set_master_gain(&mut self, gain: f32);
    pub fn play(&mut self);
    pub fn pause(&mut self);
    pub fn set_sleep_timer(&mut self, duration: Duration);
    pub fn render_i16(&mut self, out: &mut [i16], frames: usize) -> RenderStats;
    pub fn decoded_bytes(&self) -> usize;
}
```

JNI surface stays small:

- `nativeCreate(sampleRate)`
- `nativeLoadPack(path)` → id
- `nativeSetMix(json)` (or flat arrays)
- `nativeSetGain(packId, gain)`, `nativeSetMasterGain(gain)`
- `nativePlay()`, `nativePause()`, `nativeSetTimer(ms)`
- `nativeRender(directByteBuffer, frames)` → written frames
- `nativeDestroy()`

`render_i16` writes interleaved stereo. Kotlin calls it once per 4096-frame block (~85 ms at 48 kHz), not per sample.

### 5.5 Mixer details

- Internal mix in `f32`; stored PCM in `i16` to halve memory. Convert on read.
- Stereo, 48 kHz default; support 44.1 kHz by resampling packs once at load (`rubato`) if the device output rate differs.
- Block size: 4096 frames, `AudioTrack` buffer ≥ 2 blocks. Latency is irrelevant for sleep; fewer wakeups is better.
- Per-bed gain/pan ramps over 50 ms; equal-power pan.
- Each bed has a loop position, random start offset, and optional slow LFO (gain ±0.5–2 dB, period 30–180 s; pan ±0.1–0.3).
- Event scheduler runs on the audio thread using a deterministic seeded RNG; next event time = now + uniform(min, max); choose variant, gain, pan, and fade envelope.
- Event voices are preloaded PCM; polyphony capped per event type (`maxVoices`).
- Master chain: sum → gentle high-pass at 25–30 Hz → soft-knee limiter (-1.5 dBTP) → i16.
- Sleep timer/fade is sample-accurate in the engine: fade starts `fadeDuration` before the deadline; at zero, signal stop to Kotlin.
- No allocations, locks, file I/O, logging, or JNI callbacks on the audio thread.

### 5.6 Android layer

- **Service:** foreground service with `FOREGROUND_SERVICE_MEDIA_PLAYBACK` and service type `mediaPlayback`. The scaffold uses `MediaSessionCompat`; the production refactor moves to a Media3 `MediaSessionService` with a `SimpleBasePlayer` wrapper.
- **Player:** a dedicated render thread drives the native engine; Media3 exposes play/pause, live duration, metadata, and custom commands for timer/gain once the adapter lands.
- **Audio:** `AudioTrack` streaming, `ENCODING_PCM_16BIT`, 48 kHz stereo, `PERFORMANCE_MODE_POWER_SAVING` (API 26+), music stream, buffer 2×4096 frames, `WRITE_BLOCKING`.
- **Thread:** dedicated render thread at `THREAD_PRIORITY_URGENT_AUDIO`; it owns the audio loop and `AudioTrack.write`.
- **Focus:** `AudioFocusRequest` with `AUDIOFOCUS_GAIN`; duck to 20% on transient-can-duck; pause on loss; resume only if it was playing. Do not fight other media.
- **Becoming noisy:** pause when headphones are unplugged by default (configurable), to avoid blasting a speaker at night.
- **Wake lock:** acquire a named `PARTIAL_WAKE_LOCK` only while playing if testing shows it is needed; release on pause/stop/destroy. Audio playback may already hold an audio-HAL wake lock, but verify on real devices.
- **Notification:** media-style notification with play/pause/stop; update only on state changes, not per second. Timer remaining and pack art are not shown yet.
- **Lifecycle:** if the process dies, do not silently restart audio. Persist "was playing" and let the user resume; use `START_NOT_STICKY` unless testing shows a need otherwise.
- **Permissions:** `POST_NOTIFICATIONS` (Android 13+) requested in context; `FOREGROUND_SERVICE` + `FOREGROUND_SERVICE_MEDIA_PLAYBACK`; no storage permission (app-private packs); no location; no network for bundled playback.
- **Battery optimization:** provide a help screen for OEMs that kill background apps; media foreground services are usually exempt, but Samsung/Xiaomi/Oppo need testing.

### 5.7 Pack format

A `.onirispack` is a zip:

```
manifest.json
audio/*.opus
art/cover.webp
LICENSE
README.md
```

Manifest example:

```json
{
  "schemaVersion": 1,
  "id": "rain-and-thunder",
  "name": { "en": "Rain & Thunder" },
  "description": { "en": "Steady rain with distant thunder." },
  "version": "1.0.0",
  "license": "CC0-1.0",
  "tags": ["water", "storm", "night"],
  "loudnessLufs": -23.0,
  "truePeakDbtp": -1.5,
  "art": "art/cover.webp",
  "beds": [
    {
      "id": "rain",
      "file": "audio/rain.opus",
      "loop": { "startSample": 0, "endSample": 1440000 },
      "gainDb": -3.0,
      "pan": 0.0,
      "lfo": { "target": "gain", "depthDb": 1.5, "periodSec": 47 }
    }
  ],
  "events": [
    {
      "id": "thunder",
      "files": ["audio/thunder-01.opus", "audio/thunder-02.opus"],
      "minIntervalSec": 25,
      "maxIntervalSec": 90,
      "gainDb": [-10, -4],
      "pan": [-0.5, 0.5],
      "fadeInMs": 800,
      "fadeOutMs": 1200,
      "maxVoices": 1
    }
  ]
}
```

Rules:

- 1 primary bed, optional second bed ≤20 s; up to 8 event files per event type; total decoded budget enforced by the loader.
- All audio 48 kHz; beds stereo; events mono or stereo.
- `loop.endSample` is authoritative; the engine ignores container padding.
- Packs are data, not code. Validate paths (zip-slip), sizes, sample rates, loop bounds, and JSON schema. Reject unknown schema major versions.
- `.onirispack` archives are deterministic zips with fixed timestamps and sorted entries. `oniris-cli pack`, `verify-pack`, and `unpack` implement the format and the security checks; compute the SHA-256 printed by `pack` for download manifests.
- Manifest is versioned; the loader supports `schemaVersion` 1 and migrates where possible.
- Tags drive search, suggestions, and compatibility rules. Add content flags (`thunder`, `animals`) for user warnings.
- Optional `compatibility` field to prevent clashing mixes (e.g., `excludes: ["underwater"]`).

### 5.8 Content pipeline

Source and license research for the first packs is in
[`CONTENT_SOURCES.md`](CONTENT_SOURCES.md). It lists approved sources, excluded
licenses, curated Freesound searches, and a proposed first five packs.
Procedural synthesis is assessed in
[`PROCEDURAL_AUDIO.md`](PROCEDURAL_AUDIO.md): use it for support layers and
variation, not as a replacement for recorded beds.

Python (NumPy, SoundFile, pyloudnorm, ffmpeg) or a Rust CLI:

1. Ingest 48 kHz/24-bit WAV masters.
2. Trim silence, remove DC offset, denoise manually where needed.
3. Choose loop region; apply equal-power crossfade of tail into head; verify periodicity for wave/fire rhythms.
4. Normalize bed to -23 LUFS integrated; normalize events relative to the bed; limit to -1.5 dBTP.
   `oniris-cli loudness` measures BS.1770 integrated LUFS and 4x-oversampled true
   peak; `oniris-cli normalize` applies a single gain that respects the true-peak
   ceiling. Use it as the measurement of record before final listening QA.
5. Encode Opus: 96 kbps stereo beds, 64 kbps mono events, 48 kHz, 20 ms
   frames. `oniris-cli encode-opus` already writes gapless files; the Python
   pipeline should call it for consistency.
6. Generate/validate manifest; compute checksums; render a 60 s preview with the core.
7. QA: listen on phone speaker, wired headphones, Bluetooth; loop 100×; check clicks, hum, hiss, thunder spikes.
8. Package `.onirispack`.

Keep source masters and licenses in a separate asset repo or Git LFS. Do not put large audio in the app Git history.

### 5.9 Portability

| Target | Output adapter | Status |
|---|---|---|
| Android | Kotlin `AudioTrack` via JNI | v1 |
| Linux/Raspberry Pi | `cpal` (ALSA) | v1.x |
| macOS/Windows | `cpal` | v1.x |
| Web | WASM + WebAudio | experimental |
| Embedded | C ABI + I2S DMA | later |

The core exposes a stable C ABI header in addition to JNI. The desktop CLI doubles as a test harness, pack validator, and preview renderer. Platform shells own file paths, audio output, lifecycle, and UI; the core owns sound.

---

## 6. Battery and performance plan

### 6.1 Targets

| Metric | Target | Measurement |
|---|---|---|
| Battery drain, screen off, airplane mode | ≤3%/h mid-range; ≤5%/h low-end | 8 h soak, Battery Historian/Perfetto |
| App CPU during playback | <2% of one core average | `top`/Perfetto |
| Audio underruns | 0 in 8 h | `AudioTrack.getUnderrunCount()` |
| Memory | <100 MB decoded + <50 MB heap | `dumpsys meminfo` |
| Cold start to sound | <1.5 s | Macrobenchmark |
| Crash-free sessions | ≥99.5% | Play Console |
| ANR rate | <0.1% | Play Console |

Total device drain is dominated by audio output (speaker/Bluetooth) and cannot be controlled fully by the app. Optimize app CPU and wakeups; be honest in store copy.

### 6.2 Techniques

- Decode at load, not during playback.
- One `AudioTrack`; large blocks (4096 frames); power-saving performance mode.
- No per-sample JNI; no allocations/locks on the audio thread.
- No timers, polling, `AlarmManager`, or `Handler` wakeups. The audio callback is the clock.
- No network, analytics, location, or ads.
- Screen-off: no Compose recomposition, no animations, no visualizer.
- Notification updates only on state changes.
- Fade/timer in native code.
- Optional: reduce event density or disable pan modulation in system battery-saver mode (only if measurements show benefit; likely unnecessary).
- Thermal: monitor `PowerManager.getCurrentThermalStatus`; log only. CPU load is already tiny.
- Release native resources on stop; do not keep packs decoded while idle.

### 6.3 Measurement protocol

1. Factory-fresh device, 100% battery, airplane mode, screen off, volume 30%.
2. Reset stats: `adb shell dumpsys batterystats --reset`.
3. Play a 4-pack mix for 8 h with a 7 h timer.
4. Capture: `dumpsys batterystats`, Perfetto trace (power/freq/audio), `dumpsys media.audio_flinger`, underrun count, memory.
5. Repeat on: Pixel 6a (or equivalent), a low-end Samsung/Galaxy A-series, a tablet, and Bluetooth headphones.
6. Compare against a baseline: screen off with no playback, and a single ExoPlayer loop.
7. Acceptance: meet targets, no wakelock beyond playback, no network traffic, no thermal throttle.

### 6.4 OEM notes

Samsung, Xiaomi, Oppo, and OnePlus aggressively kill background work. A media foreground service with an active `MediaSession` is usually protected, but test. Provide a help screen for "battery optimization" settings; never nag. Do not use exact alarms for the sleep timer; the service is already alive.

---

## 7. UX

### 7.1 Core flows

**First launch → sleep in under 20 seconds**

1. One-screen intro: "No account. Works offline. Made for sleep."
2. Pick a preset: Rainy Night, Seaside, Cabin Fire, Forest Deep, Pure Noise.
3. Timer defaults to 45 min; big Play button.
4. Optional "Customize" opens the mixer.

**Bedtime (returning user)**

- Home shows last mix + big Play. One tap starts with a 10 s fade-in.
- Quick Settings tile opens/starts the last mix.
- Lock screen controls are available without unlocking.

**Mixing**

- Pack library: cards with art, tags, 10 s preview, "Add".
- Mixer: active packs with sliders; master volume; total decoded-memory indicator hidden from users but enforced.
- Save as preset; rename; delete; reorder (optional).

**Night**

- Now Playing screen: black background, clock, timer remaining, stop. No bright art, no animation.
- Tap anywhere to reveal controls; auto-hide after 5 s.
- Screen-off playback continues.

### 7.2 Visual direction

- AMOLED black, warm off-white text, muted nature-derived accents.
- Large touch targets (≥48 dp), high contrast, no pure white at night.
- One accent per pack, derived from cover art.
- Minimal motion; respect "reduce motion".
- Typography: humanist sans; readable at low brightness.
- Optional "dim red" night mode? Keep as a setting later; red text can be hard to read.

### 7.3 Accessibility

- TalkBack labels for all controls, including sliders ("Rain volume, 60 percent").
- Dynamic type up to 200%; no clipped text.
- No color-only state; icons + text.
- Screen-reader-friendly timer and remaining-time announcements (on demand, not every minute).
- Haptic confirmation for play/stop (optional).
- Hearing safety: volume warning above 60% for long sessions; cap option in settings.

---

## 8. Data and storage

- **Settings/presets:** `DataStore` + `kotlinx.serialization` JSON. No Room needed for v1.
- **Packs:** app-private `filesDir/packs/<id>/`; bundled packs copied from assets on first run (or Play Asset Delivery).
- **Downloads:** WorkManager with foreground notification? Keep simple: a coroutine in the service/activity with progress; verify checksum; atomic rename. No background download while sleeping.
- **Cache:** preview renders and temporary decodes; clear on low storage.
- **No database, no account, no cloud.**
- **Size targets:** base APK/AAB <25 MB (3 packs + engine), each pack 1–2 MB, total install after 8 packs <40 MB.
- **ABI splits:** `arm64-v8a` primary; `armeabi-v7a` optional; `x86_64` for emulator. Use App Bundle.
- **Play Asset Delivery** for bundled packs if base size grows; otherwise bundle 3 and download the rest from a static host/GitHub Releases.

---

## 9. Testing and QA

### 9.1 Automated

- **Rust unit tests:** manifest parsing/validation, loop bounds, gain ramps, scheduler determinism with seeds, timer math, limiter, Opus/WAV decode and pre-skip/end-trim handling, resampler level/alias rejection, loudness gating and true peak.
- **Golden audio tests:** render fixed seeds; compare RMS, spectral centroid, and peak within tolerance; assert no discontinuities above a threshold at loop/fade boundaries. Implemented in `oniris-core/tests/golden.rs`; `oniris analyze` reports max sample delta and large-jump counts for manual render QA.
- **Fuzz:** manifest JSON and zip extraction (paths, sizes, malformed Opus).
- **JNI round-trip:** create/load/mix/render/destroy; error paths.
- **Android instrumentation:** service lifecycle, notification, audio focus loss/gain, timer stop, process death restore, headphone disconnect.
- **Soak test:** 8 h on device, log underruns/memory/CPU/battery.
- **CI:** GitHub Actions for Rust + Gradle unit tests; emulator for instrumentation (audio quality not testable there); self-hosted/real-device job for soak.

### 9.2 Manual/listening

- Loop 100× for each bed; listen for pattern, click, hum, hiss.
- Mix every pair of packs; check masking and level balance.
- Thunder/crackle events at low volume: no jump-scare.
- Phone speaker, wired headphones, Bluetooth (SBC/AAC), mono downmix.
- 30-minute real listening session in a quiet room; note anything that draws attention.
- Low battery, battery saver, thermal throttle, airplane mode, call interruption, Bluetooth disconnect.

### 9.3 Device matrix

- Pixel 6a/7a (reference), Galaxy A14/A15 (low-end), a tablet, OnePlus/Samsung flagship, Bluetooth earbuds/speaker.
- Android 8, 12, 13, 14, 15, 16 (emulators for old APIs; real devices for current).
- 2 GB RAM device for memory pressure.

---

## 10. Repository layout

```
oniris/
  android/                  # Gradle project (app, playback, ui)
  core/                     # Rust workspace
    oniris-core/            # DSP, mixer, scheduler, pack loader
    oniris-pack/            # manifest types + validation
    oniris-jni/             # JNI shim
    oniris-cli/             # render/validate/analyze/preview
    oniris-desktop/         # cpal player (optional)
  tools/
    asset-pipeline/         # Python: normalize, loop, encode, package
    pack-editor/            # later
  packs/                    # source manifests + small test assets (LFS for audio)
  docs/
    DEVELOPMENT_PLAN.md
    PACK_FORMAT.md
    ARCHITECTURE.md
    LICENSING.md
  scripts/                  # build, release, soak
```

Build: Gradle + AGP 8.x, Kotlin 2.x, Compose BOM, Media3 1.x, `cargo-ndk`, vendored libopus. CI builds all ABIs and runs Rust tests. Use a `Makefile`/`justfile` for common tasks.

---

## 11. Roadmap

Assumes one developer working roughly full-time, with content sourcing in parallel. Part-time: multiply by ~2.

| Phase | Weeks | Deliverables | Exit criteria |
|---|---|---|---|
| 0. Decisions & spikes | 1 | Pack spec v0; 2 test beds + 4 events; Rust→WAV render; JNI+AudioTrack spike; Media3 `SimpleBasePlayer` spike; battery baseline | Native core is viable; loop quality acceptable; battery estimate <3%/h |
| 1. Core + CLI | 2–4 | Rust workspace; Opus decode; resampler; mixer; LFOs; event scheduler; fades; limiter; timer; `oniris render/validate/analyze` | 60-min 4-pack render is click-free; deterministic tests pass; memory <96 MB |
| 2. Playback spine | 5–7 | Android project; JNI; AudioTrack; foreground service; MediaSession; notification; audio focus; wake lock; basic play/pause | 8 h playback on device; no underruns; battery measured; calls/Bluetooth handled |
| 3. Mixer UX | 8–9 | Pack library; add/remove; per-pack volume; presets; last mix; timer UI; now playing; lock screen metadata; quick tile | Internal alpha: 4-pack mix, presets, background, timer all work |
| 4. Content | 10–11 | Asset pipeline; 8–10 packs; loudness/loop QA; licenses; downloads; pack validation | Content complete; every pack passes loop/loudness/QA checklist |
| 5. Hardening & beta | 12–13 | OEM testing; accessibility; localization scaffolding; process-death; ANR/crash handling; play store closed testing | No P0 bugs; battery target met; beta feedback triaged |
| 6. Launch | 14–15 | Listing, screenshots, privacy policy, content rating, licenses, baseline profile, staged rollout | v1.0 live |

**MVP checkpoint:** end of week 9 (internal).  
**Beta:** week 13.  
**Launch:** week 15.

**Scaffold progress:** phase 0 (spike), phase 1 (core plus Ogg/Opus), and phase 2
(playback spine) are substantially done; 60-minute listening QA is still open.
Phase 3 (mixer UX) has pack selection, master volume, and the timer; presets,
per-pack volume, lock-screen metadata, and a quick tile are open. Phase 4
(content) is next and should use the new Opus encoder, `.onirispack` packer, and
loudness/normalize tools in `oniris-cli`; the remaining distribution work is the
Android download/import UI. Pure Noise is generated from the CLI; the
field-recorded Rainy Night, Seaside, Cabin Fire, and Forest Deep packs are next.

Content sourcing and licensing start in week 1; they are the most likely schedule risk.

---

## 12. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Battery target missed | Product promise fails | Measure in phase 0; decode-once design; one AudioTrack; 8 h soak; fallback buffer tuning |
| Loops sound repetitive | Users stop using it | Long-ish beds, random start offsets, LFOs, event pools, night curve; 100× listening tests |
| Audio clicks/glitches | Wakes users | 50 ms ramps, sample-accurate loops, limiter, underrun monitoring, large buffers |
| Content licensing | Legal/release blocker | CC0/commissioned/own recordings; per-pack license file; legal review; no YouTube rips |
| Native build complexity | Schedule slip | CI from day 1; vendored libopus; Docker; desktop CLI tests; keep JNI surface tiny |
| OEM background kills | Playback stops at night | MediaSessionService + FGS; test Samsung/Xiaomi/Oppo; help screen; avoid battery-heavy extras |
| Process death / service restart | Silent stop or zombie audio | Persist state; `START_NOT_STICKY`; surface "resume" notification; test |
| App size | Install friction | Opus, downloadable packs, ABI splits, Play Asset Delivery |
| Scope creep | Never ships | Strict v1 scope; park ideas in `docs/BACKLOG.md` |
| Bluetooth/call edge cases | Bad night | Audio focus tests, becoming-noisy receiver, device matrix |
| Android policy changes | Launch delay | Target SDK 36, FGS types, notification permission, privacy policy |

---

## 13. Success metrics

**Technical**

- ≤3%/h battery on mid-range, ≤5%/h on low-end (screen off, airplane mode).
- <2% CPU average; 0 underruns in 8 h; <100 MB memory.
- Crash-free ≥99.5%; ANR <0.1%; cold start <1.5 s.

**Product**

- 7-day retention >25% (sleep apps are habit-driven; refine after beta).
- Median session ≥6 h; timer used in >50% of sessions.
- ≥2 packs used per session on average; preset creation >15% of active users.
- Store rating ≥4.5; qualitative feedback: "doesn't sound looped", "didn't wake me", "battery barely moved".

---

## 14. Open decisions

Resolved: Rust core, free app, CC0/CC-BY/self-authored content, phone/tablet
first, desktop/web portability.

Still open:

1. **Code license:** MIT vs Apache-2.0 (or dual) before public release.
2. **Crash reporting:** Play Console vitals only (privacy-friendly) vs opt-in Sentry.
3. **Name/trademark:** confirm "Oniris" is clear in the sleep/audio category.
4. **Content sourcing:** own recordings vs commissioned vs CC0 curation, and the
   budget for each.
5. **Pack distribution:** bundled in the APK vs downloadable `.onirispack` files
   (with signature/verification) for v1.0.
6. **Analytics:** recommend none in v1; use Play Console + optional in-app feedback.
7. **Streaming:** no network for playback in v1; revisit only for pack downloads.

## 15. First 10 concrete tasks

1. [x] Write `docs/PACK_FORMAT.md` v0; the JSON Schema is still to add.
2. [x] Create the Rust workspace skeleton and a CLI that renders a test loop to WAV.
3. [x] Generate synthetic rain/fire beds and events; loop points and crossfades work.
4. [x] Implement the mixer + scheduler; 60-minute listening QA is still pending.
5. [x] Spike JNI + `AudioTrack` (emulator); real-device CPU/underrun measurement pending.
6. [x] Spike a media session + foreground service; Media3 adapter is a later refactor.
7. [ ] Run a 1-hour battery baseline; extrapolate to 8 hours.
8. [x] Build the Compose shell: library, mixer, timer, now playing.
9. [x] Bundled-pack extraction and app-private storage.
10. [x] Set up CI (Rust tests + Gradle build + ABI packaging).

---

## 16. One-sentence plan

**Build a Rust soundscape engine that decodes packs once and mixes from memory, wrap it in a thin Kotlin/Media3 service and a calm Compose UI, validate battery with an 8-hour soak before investing in content, then ship 8–10 loudness-matched packs with an open, versioned pack format.**

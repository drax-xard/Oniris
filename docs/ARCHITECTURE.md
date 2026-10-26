# Architecture

## Goals

1. Play for 8+ hours with minimal CPU and wakeups.
2. Keep the sound engine independent of Android so it can run on desktop, Raspberry Pi, and the web.
3. Make content additive: a new pack should be data, not a new build of the engine.

## Components

```
┌──────────────────────────────────────────────┐
│ Compose UI (library, mixer, timer, settings) │
└──────────────────────┬───────────────────────┘
                       │ StateFlow / intents
┌──────────────────────▼───────────────────────┐
│ PlaybackService (foreground media service)    │
│  ├─ MediaSessionCompat (notification/lock)   │
│  ├─ Audio focus / becoming-noisy handling    │
│  └─ AudioRenderThread ── AudioTrack          │
└──────────────────────┬───────────────────────┘
                       │ JNI, coarse calls only
┌──────────────────────▼───────────────────────┐
│ oniris-core (Rust, platform-neutral)          │
│  ├─ WAV/Opus decode at load                  │
│  ├─ Mixer: beds, event voices, LFOs, pan     │
│  ├─ Scheduler: randomized one-shots          │
│  ├─ Sleep timer, fades, limiter              │
│  └─ render_i16(buffer, frames)               │
└──────────────────────┬───────────────────────┘
                       │ std::fs
┌──────────────────────▼───────────────────────┐
│ Pack directory / .onirispack                  │
│  manifest.json + audio + art + LICENSE        │
└──────────────────────────────────────────────┘
```

## The central decision: decode once, mix from memory

A sleep session is long and interaction is rare. Decoding continuously is the
main avoidable CPU cost. Oniris therefore:

1. Decodes every bed and event into 16-bit PCM when a mix starts.
2. Mixes PCM on a dedicated audio thread and writes one `AudioTrack`.
3. Never decodes, resamples, allocates, locks, or reads files on the audio
   thread during playback.

Tradeoff: memory and a short load when changing mixes. Typical four-pack mixes
use 30–60 MB decoded, capped at 96 MB. This is a good trade for a device that
will run for eight hours.

## Thread model

- **UI thread:** Compose; never touches the audio engine.
- **Control thread (Kotlin):** calls JNI to load packs, change mix/gain, set the
  timer. Calls are serialized by the controller.
- **Audio thread:** native render → `AudioTrack.write` blocking. Priority
  `THREAD_PRIORITY_URGENT_AUDIO`. Block size 4096 frames (~85 ms at 48 kHz).
- **No decoder threads during playback.**

The native engine is not internally synchronized. The Kotlin layer owns the
handle and ensures control calls do not race with render. A later refinement can
move to a command ring buffer if needed.

## Render pipeline

1. For each active pack:
   - Schedule events whose timer expired this block.
   - Mix each bed sample-by-sample, honoring loop points, gain, pan, and slow
     LFOs.
2. Mix event voices with per-voice fade envelopes.
3. Apply transport fade, sleep-timer fade, and master gain.
4. Limit to -1.5 dBFS.
5. Convert to interleaved stereo i16.

All gain changes are ramped over at least 50 ms (event fades, transport fades,
timer). Event scheduling happens at block boundaries; ~85 ms of timing slack is
inaudible for thunder and crackles.

## Loop semantics

- `loop.startSample` and `loop.endSample` are expressed at the source file's
  sample rate.
- The loader scales them to the engine rate if resampling was needed.
- `loop.endSample` is exclusive and must be within the decoded file.
- The asset pipeline should crossfade the tail into the head so the loop is
  seamless at the sample boundary. The engine simply wraps.

## Memory budget

- Bed: 20–45 s stereo at 48 kHz, 16-bit ≈ 3.8–8.6 MB.
- Event: 1–8 s mono/stereo ≈ 0.1–0.8 MB.
- Pack guideline: one primary bed, optional second bed ≤20 s, up to 8 event
  files.
- Mix limit: 4 packs, 96 MB decoded.
- Loader rejects a pack/mix that would exceed the budget.

The engine reports `decoded_bytes()` through JNI so the UI can enforce the
budget before starting.

## Android shell

- **Service:** foreground service with `mediaPlayback` type and a
  `MediaSessionCompat`. The scaffold uses the compatibility session because the
  custom engine is not an ExoPlayer `Player`; a Media3 `SimpleBasePlayer` adapter
  is a later refactor.
- **Audio:** `AudioTrack` streaming, 48 kHz stereo, PCM 16-bit,
  `PERFORMANCE_MODE_POWER_SAVING` (API 26+).
- **Audio focus:** pause on loss, duck on transient-can-duck, resume only if
  playback was active.
- **Becoming noisy:** pause when headphones disconnect (default).
- **Wake lock:** a partial wake lock only while playing; verify on real devices
  whether the audio HAL already holds one.
- **Notification:** play/pause/stop; update on state changes only.

## Portability

| Target | Adapter | Status |
|---|---|---|
| Android | Kotlin `AudioTrack` + JNI | scaffold |
| Linux/Pi | `cpal`/ALSA | planned |
| macOS/Windows | `cpal` | planned |
| Web | WASM + WebAudio | planned |
| Embedded | C ABI + I2S | later |

The core has no Android types. The JNI crate is a thin shim; a C header can be
generated for other platforms. Pack parsing and the asset pipeline are
platform-neutral.

## Known scaffold gaps

- Opus/Vorbis decode (WAV only today).
- Media3 `SimpleBasePlayer` adapter (using `MediaSessionCompat`).
- Lookahead limiter (instant-attack limiter today).
- High-quality resampler (linear today).
- Crossfade when changing mixes while playing.
- Download manager, pack import UI, and bundled pack installation.
- Real content and loudness normalization pipeline.

These are tracked in `docs/DEVELOPMENT_PLAN.md`.

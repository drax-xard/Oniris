# Oniris — Current Roadmap

_Status assessment and task tree as of 2026-11-23. The engine section was
refreshed after the 2026-10-26 hardening pass; everything else is unchanged from
the 2026-10-26 assessment._

---

## Current Status

**What’s done:** the scaffold is solid, and the audio engine is no longer
scaffold-quality.
- **Rust core** (`oniris-core`): Ogg Opus + WAV decode (pre-skip/granule
  trimming, PLC on lost packets), band-limited windowed-sinc resampling, mixer
  with beds + LFOs, event scheduler with fades, sleep timer, 5 ms lookahead
  limiter, 80 ms crossfade on mix change, memory budget enforcement, and JNI
  render loop all work.
- **JNI bridge** (`oniris-jni`): Complete control surface (loadPack, setMix, gains, play/pause/stop, timer, render, decodedBytes).
- **Android shell** (`android/`): Foreground service, `MediaSessionCompat`, notification, audio focus, wake lock, `AudioTrack` render thread, pack library, master volume, and sleep timer UI are in place. Debug APK builds and smoke-tests on an x86_64 emulator pass.
- **CLI** (`oniris-cli`): gen-test-packs, validate, render, analyze all functional.

**What’s missing (production blockers):**
- **Production content** — no real sound packs, no loudness/loop QA pipeline.
- **Battery/audio validation** — 1-hour baseline, 8-hour soak, underrun measurement, OEM tests all pending.
- **UX gaps for v1.0** — per-pack volume sliders, presets + last-mix restore, now-playing metadata, quick tile.
- **Engine polish (non-blocking)** — Media3 `SimpleBasePlayer` refactor; the
  limiter ceiling is sample peak rather than true peak (true-peak gating lives in
  the asset pipeline).

---

## Recommended Task/Subtask Tree

Ordered by logical dependency: engine → content → UX → reliability → launch.

### 1. Audio Engine Hardening — **done (2026-10-26)**
**Goal:** Replace scaffold-quality audio with production-quality internals.

- [x] **1.1 Add Opus decoder**
  - [x] Integrate `ogg` + `opus-decoder` into `oniris-core`.
  - [x] Extend `decode_audio()` to route `.opus`/`.ogg`/`.oga` through the new decoder.
  - [x] Enforce 48 kHz decode and mono/stereo contracts (channel mapping family 0 or the standard stereo family 1); packet size is not constrained, so 20 ms frames are a content-pipeline rule, not an engine check.
  - [x] Unit tests: decode round-trip, pre-skip/granule trimming, invalid packet handling, missing-packet recovery via Opus PLC.
- [x] **1.2 Replace linear resampler**
  - [x] Hand-written 32-tap Blackman-Harris windowed-sinc resampler (chose this over `rubato`: no new dependency, ~40 lines, and only 44.1/48 kHz ratios matter).
  - [x] Tests gate on level accuracy (±2% RMS) and anti-aliasing (20 kHz tone killed on 48→24 kHz).
- [x] **1.3 Upgrade limiter**
  - [x] 5 ms lookahead, 100 ms release, stereo-linked, −1.5 dBFS *sample* peak ceiling. True peak (−1.5 dBTP) is still measured in the asset pipeline.
  - [x] Tests: sustained peaks, transient impulse, hot-signal no-clipping, stereo linking.
- [x] **1.4 Crossfade on mix change**
  - [x] 80 ms equal-linear crossfade when `set_mix` is called during playback; old layers and their in-flight voices fade out, fading layers stop spawning events.
  - [x] Test asserts no step above 500 LSB at the seam and that stale layers are reclaimed.

### 2. Content Pipeline & First Packs
**Goal:** Deliver 8–10 loudness-matched packs inside size budget.

- [ ] **2.1 Finalize asset pipeline**
  - [ ] Python/Rust tool: ingest WAV → trim/DC offset → loop crossfade → LUFS normalize (−23) → limit (−1.5 dBTP) → Opus encode → package `.onirispack`.
  - [ ] Manifest schema validation + checksum generation.
- [ ] **2.2 Produce first packs**
  - [ ] 3 bundled packs (e.g., Rain, Fire, Wind).
  - [ ] 5–7 downloadable packs.
  - [ ] Loop QA: 100× listen test checklist per bed; metadata + loop-point sign-off.
- [ ] **2.3 Licenses & attribution**
  - [ ] Per-pack `LICENSE` file.
  - [ ] In-app attribution screen.
  - [ ] Choose code license (MIT/Apache-2.0/dual).

### 3. Android UX Completion
**Goal:** Hit every P0/P1 feature in the v1.0/1.1 scope.

- [ ] **3.1 Mixer & library**
  - [x] Per-pack volume sliders with live gain update via `setPackGainDb`
        (2026-11-23, persisted in `DataStore`).
  - [x] One screen per concern behind a side navigation drawer — Play, Library,
        Mixer, Timer, Settings; permanent drawer ≥600 dp (2026-11-23).
  - [ ] Add/remove packs, tag-based filtering, pack preview (10 s render).
- [ ] **3.2 Presets & last-mix restore**
  - [ ] 4 built-in presets.
  - [ ] User-saved presets via `DataStore` + `kotlinx.serialization`.
  - [ ] Restore last mix on cold start (if was-playing = true).
- [ ] **3.3 Now Playing & system integration**
  - [ ] Lock-screen/notification metadata (pack art, title, timer remaining).
  - [ ] Quick Settings tile (start/stop last mix).
- [ ] **3.4 Pack delivery**
  - [ ] Bundled pack extraction on first run (or Play Asset Delivery if size demands).
  - [ ] `.onirispack` download + verify + install flow (WorkManager or service coroutine).
  - [ ] Custom pack import (file picker + validation).

### 4. Reliability & Battery Validation
**Goal:** Meet the hard performance targets before beta.

- [ ] **4.1 On-device measurement**
  - [ ] 1-hour battery baseline on Pixel 6a/7a (screen off, airplane mode, 30% vol).
  - [ ] Extrapolate to 8-hour drain target (≤3%/h mid-range, ≤5%/h low-end).
  - [ ] Audio underrun capture via `AudioTrack.getUnderrunCount()`.
- [ ] **4.2 Device matrix soak**
  - [ ] 8-hour playback soak on: Pixel 6a/7a, Galaxy A-series (low-end), tablet, Bluetooth.
  - [ ] Measure: underruns, CPU, memory, thermal, network traffic (should be zero).
- [ ] **4.3 Edge cases**
  - [ ] Audio focus loss/gain/duck + resume.
  - [ ] `AudioManager.ACTION_AUDIO_BECOMING_NOISY` (headphone disconnect).
  - [ ] Process death → persisted state → resume notification.
  - [ ] ANR/crash handling; `START_NOT_STICKY` verification.
- [ ] **4.4 OEM hardening**
  - [ ] Battery optimization help screen (Samsung/Xiaomi/Oppo).
  - [ ] Test background-media-service survival on 3+ OEM skins.

### 5. Testing, Polish & Launch
**Goal:** Close the remaining gaps from the development plan.

- [ ] **5.1 Automated tests**
  - [ ] Rust: determinism/seeded scheduler, manifest fuzz, memory budget overflow, limiter golden audio.
  - [ ] JNI round-trip (create/load/mix/render/destroy, error paths).
  - [ ] Android instrumentation: service lifecycle, timer stop, focus loss, process death.
- [ ] **5.2 Accessibility & localization**
  - [ ] TalkBack labels, dynamic type up to 200%, color+icon state.
  - [ ] `strings.xml` scaffolding for localization (English first).
- [ ] **5.3 Store readiness**
  - [ ] Baseline profile.
  - [ ] Screenshots, listing copy, privacy policy.
  - [ ] Closed testing (beta) → triage feedback → staged rollout.

---

## Critical Path

**Asset pipeline + first real Opus packs → 1-hour baseline → 8-hour battery soak.**

The engine no longer blocks anything. Presets, quick tile, and the Media3
refactor can run in parallel with content and do not block v1.0 launch.

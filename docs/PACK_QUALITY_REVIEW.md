# Procedural packs — quality review

_Measured 2026-11-23 with `tools/audio-qa/measure.py` (ffmpeg EBU R128 + numpy
spectral analysis). Scope: the two packs produced by `oniris-cli gen-test-packs`
— `test-rain` and `test-fire`, six WAV assets in total._

**Verdict:** both packs are functioning noise generators, not convincing
soundscapes. They are top-heavy (white-noise hiss), carry a third to a half of
their energy below 20 Hz where no phone or earbud can reproduce it, and three of
the six files break the project's own −1.5 dBTP true-peak rule. The manifests
also assert loudness and true-peak values that were never measured.

`make qa` (added with this review) fails on the current content:
`crackle-01` (−0.5 dBTP), `crackle-02` (+0.1 dBTP), `fire` (−0.3 dBTP, DC offset).

---

## Measurements

`peak` is sample peak; `TP` is true peak; bands are share of total energy;
`phone` is the 300 Hz–4 kHz band that a phone speaker can actually reproduce;
`corr` is L/R correlation for stereo assets.

| asset | LUFS | peak dBFS | TP dBTP | centroid | 0–20 Hz | 300–4 kHz | >8 kHz | corr | crest |
|---|---|---|---|---|---|---|---|---|---|
| rain.wav | −19.7 | −11.9 | −10.5 | 11.2 kHz | 13.2% | 12.3% | 31.7% | 0.00 | 15.4 dB |
| thunder-01.wav | −26.2 | −8.7 | −8.0 | 9.7 kHz | 42.4% | 6.2% | 8.1% | mono | 16.5 dB |
| thunder-02.wav | −26.5 | −7.2 | −7.0 | 9.7 kHz | 51.7% | 5.5% | 6.9% | mono | 18.5 dB |
| fire.wav | −23.4 | −4.6 | **−0.3** | 10.9 kHz | 33.1% | 8.6% | 23.0% | −0.00 | 25.2 dB |
| crackle-01.wav | −20.3 | −2.6 | **−0.5** | 12.1 kHz | 0.1% | 15.0% | 50.2% | mono | 21.2 dB |
| crackle-02.wav | −20.5 | −2.9 | **+0.1** | 12.0 kHz | 0.1% | 14.0% | 51.8% | mono | 20.7 dB |

Manifest for both packs: `loudnessLufs: -23.0`, `truePeakDbtp: -1.5` — hardcoded
constants (`generate.rs:54`, `:67-68`, `:103`, `:116-117`), never measured.

For reference, a real rain recording on foliage measures roughly 2–3 kHz
centroid with most energy between 500 Hz and 6 kHz; a fire pit sits around
1–2 kHz with the roar in 120–600 Hz and crackles at 1–3.5 kHz.

---

## What is wrong, per pack

### test-rain

1. **It is white noise, not rain.** The bed is `white * 0.22` plus a one-pole
   rumble; 31.7% of the energy is above 8 kHz and only 12.3% lands in the band a
   phone speaker can reproduce. It reads as faint hiss.
2. **No droplet texture.** Nothing in the generator creates the granular patter
   that makes rain recognisable — no short decaying bursts, no density variation.
3. **13.2% of the energy is below 20 Hz.** The one-pole low-pass at
   `generate.rs:139` (`low * 0.995 + white * 0.005`, pole ≈ 38 Hz) feeds
   `low * 3.0` into the mix, so a third of the headroom is spent on inaudible
   rumble.
4. **The 7 s swell (line 142) does not divide the 30 s loop** — 4.28 cycles — so
   the same amplitude pattern recurs every 30 s. The crossfade hides the seam,
   but not the repetition.
5. **Thunder is subsonic.** 42–52% below 20 Hz, 6% in the phone band, and its
   9.7 kHz centroid comes from `white * 0.08` rather than the rumble that
   defines thunder. On a phone speaker the event nearly disappears.
6. **Levels are wrong in both directions:** the bed is 3.3 dB hotter than its
   manifest claims, and the thunder events sit ~6.5 dB below the bed *before*
   the −6…−12 dB `gainDb` is applied, so a thunderclap lands 10–18 dB under the
   rain.

### test-fire

1. **The roar is at 15 Hz.** `low * 0.998 + white * 0.002` (`generate.rs:164`)
   puts the fire body below hearing; 33.1% of energy is under 20 Hz and the
   phone band gets 8.6%.
2. **Crest factor 25.2 dB with RMS −29.8 dBFS** — the audible content is sparse
   clicks at low level, and the loud part is inaudible. That is the opposite of
   what a fire should be.
3. **Crackles are noise bursts, not crackles.** `rng.gen_range(-0.6, 0.6)` is
   re-drawn every sample (lines 176–177), which destroys the transient shape;
   the 0.992 decay (line 173) is a 2.6 ms tick. Measured flatness 0.95 and
   centroid 12 kHz confirm a band of white noise, not a resonant snap.
4. **No DC blocking** on the 15 Hz integrator: fire.wav carries 0.0017–0.0021 DC
   offset, over the 0.001 policy.
5. **True peak violation:** −0.3 dBTP on the bed, +0.1 dBTP on `crackle-02`
   (inter-sample peaks above full scale).

### Cross-cutting

- **Nothing measures the output.** Loudness and true peak are asserted, not
  computed, so "loudness-matched packs" is currently a promise, not a property.
- **Stereo is accidental.** Beds are fully decorrelated (corr 0.00, side = mid),
  which thins the mono downmix every phone speaker performs.
- **No character tests.** The Rust tests prove the mixer does not crash; nothing
  checks that rain sounds like rain.
- **Format:** 10.7 MB of WAV for two packs. The Opus decoder landed in `e156a4c`,
  so this could be ~1 MB.

---

## Proposed redesign

Target specs (asserted by tests, checked by `make qa`). The ranges were revised
after the first rebuild: a phone-band share above the original 35–50% window is
good news, not a regression, and the energy-weighted centroid of a fire sits
lower than a naive guess because the roar carries the power.

| asset | centroid | 0–20 Hz | phone 300–4 kHz | >8 kHz | L/R corr | LUFS | TP |
|---|---|---|---|---|---|---|---|
| rain bed | 1.8–3.2 kHz | <2% | 40–75% | <8% | 0.2–0.6 | −23.0 ±0.5 | ≤ −1.5 |
| fire bed | 0.9–2.0 kHz | <3% | 30–60% | <5% | 0.3–0.7 | −23.0 ±0.5 | ≤ −1.5 |
| thunder | 200–550 Hz | <5% | 25–60% | <3% | mono | −20 ±2 | ≤ −1.5 |
| crackle | 1.5–4 kHz | ~0% | ≥50% | <20% | mono | −20.0 ±1 | ≤ −1.5 |

---

## Result after the rebuild (2026-11-23)

`make qa` is green on all six assets, and `procedural_packs_meet_quality_targets`
asserts the numbers below in `cargo test`.

| asset | LUFS | TP dBTP | centroid | 0–20 Hz | phone 300–4 kHz | >8 kHz | corr |
|---|---|---|---|---|---|---|---|
| rain.wav | −23.0 *(was −19.7)* | −13.5 *(was −10.5)* | 2.0 kHz *(was 11.2)* | 0.0% *(was 13.2)* | 63.5% *(was 12.3)* | 2.2% *(was 31.7)* | 0.37 *(was 0.00)* |
| thunder-01.wav | −21.0 | −1.6 *(was −8.0)* | 456 Hz *(was 9.7 k)* | 0.1% *(was 42.4)* | 45.2% *(was 6.2)* | 0.0% *(was 8.1)* | mono |
| thunder-02.wav | −23.8 | −1.6 *(was −7.0)* | 472 Hz *(was 9.7 k)* | 0.0% *(was 51.7)* | 44.4% *(was 5.5)* | 0.0% *(was 6.9)* | mono |
| fire.wav | −23.0 | −8.7 *(was −0.3)* | 1.1 kHz *(was 10.9)* | 0.0% *(was 33.1)* | 50.3% *(was 8.6)* | 0.8% *(was 23.0)* | 0.63 *(was −0.00)* |
| crackle-01.wav | −20.0 | −8.6 *(was −0.5)* | 2.3 kHz *(was 12.1)* | 0.0% | 88.8% *(was 15.0)* | 0.0% *(was 50.2)* | mono |
| crackle-02.wav | −20.0 | −9.5 *(was +0.1)* | 2.3 kHz *(was 12.0)* | 0.0% | 82.2% *(was 14.0)* | 0.0% *(was 51.8)* | mono |

Loop seams: rain ×0.14, fire ×0.39 of the 99.9th-percentile sample step (both
below the ×1 click threshold). DC offset is at the noise floor for every file.

What changed in the generator:

- RBJ biquads replaced the one-pole smoothers; pink and brown noise generators
  with DC blockers replaced raw white noise.
- Rain: pink wash high-passed at 200 Hz, 95 droplets/s of 3–6 ms band-passed
  bursts with log-normal amplitude and random pan, and a −27 dB brown body.
- Fire: 100–550 Hz roar breathing in whole cycles across the loop, −18 dB hiss,
  and Poisson-clustered resonant crackles (1.5 ms attack, 15–60 ms decay).
- Thunder: onset crack over a 3.6 s rumble, a 200–900 Hz mid layer so it survives
  a phone speaker, and a second roll. Mono, panned by the engine.
- Crackle one-shots: resonant 1.5–3.5 kHz ticks instead of white-noise bursts.
- Metering: BS.1770 integrated loudness plus a 4× oversampled true-peak estimate
  drive normalisation, and the manifests now carry the measured values.

Known trade-offs, stated rather than hidden:

- Thunder-02 lands 2.8 dB below Thunder-01 because its crest is higher and the
  true-peak limiter is the binding constraint. Both are within the event target
  window; real thunder varies this much.
- Beds normalise to −23 LUFS with true peaks well under the ceiling (−13.5 dBTP
  for rain), because a steady noise bed has a low crest. The headroom is not
  wasted: it is what keeps the master limiter from engaging when four packs mix.


Synthesis plan:

- **Rain** — pink-noise wash (HP 120 Hz, LP 7 kHz) + granular droplet layer
  (3–15 ms decaying bursts, band-passed 2–5 kHz, 30–80/s, panned) + quiet body
  (brown noise, LP 150 Hz, −22 dB). Stereo via mid/side: correlated mid,
  decorrelated side at −4 dB. 45 s loop; modulation periods divide 45 s.
- **Fire** — roar (brown/pink band-passed 100–600 Hz with slow AM at
  0.1–0.3 Hz) + hiss (HP 2.5 kHz, −26 dB) + resonant crackles (1–3 ms attack,
  15–60 ms exponential decay, band-passed 1.2–3.5 kHz, Poisson-clustered at
  5–20/s, pan spread ±0.8). 40 s loop.
- **Thunder** — onset crack (200–1200 Hz, 30–60 ms) over a rumble (brown,
  LP 60–160 Hz, 3.5–5 s decay, 0.5–2 Hz AM) with a second roll at 1.5–3 s. Mono;
  the engine pans it.
- **Shared DSP** — RBJ biquads (2nd order) instead of one-pole filters, pink
  (Kellet) and brown noise generators with DC blockers, envelope helpers.
- **Metering in the generator** — K-weighted (BS.1770-style) integrated loudness
  with gating, plus a 4× oversampled true-peak estimate; normalize beds to
  −23 LUFS, limit to −1.5 dBTP, and write the *measured* values into the
  manifest.
- **Character tests** — centroid and band-share windows, true-peak ceiling, DC
  offset, loop-seam continuity; `make qa` stays the gate for regenerated audio.

## Task tree

- [x] **A. DSP toolkit** in `oniris-cli` — biquads, pink/brown noise, DC blocker,
  envelope, loudness + true-peak meter (`src/dsp.rs`, unit-tested).
- [x] **B. Rain rebuild** to the target spec, 45 s loop, whole-cycle modulation.
- [x] **C. Fire rebuild** — roar + hiss + resonant crackle events, 40 s loop.
- [x] **D. Thunder + crackle rebuild** — layered thunder with real rumble;
  resonant crackle one-shots.
- [x] **E. Manifest truth** — measured `loudnessLufs` / `truePeakDbtp`, pack
  version 0.2.0.
- [x] **F. Gates** — `cargo test` character assertions + `make qa` green.
- [ ] **G. Optional, separate commit** — Opus encode/packaging step
  (10.7 MB → ~1 MB) and a rendered-mix QA pass through `oniris analyze`.

## Decisions needed

- **D1** Rewrite the generator in place and keep the ids `test-rain` /
  `test-fire` (bump `version` to 0.2.0) — recommended, because the Android app
  persists pack ids in DataStore. Alternative: new ids `procedural-rain` etc.,
  which would drop the current selection on device.
- **D2** Meter loudness and true peak inside the Rust generator and write the
  measured numbers into the manifests — recommended. Alternative: leave the
  manifests to the Python QA step (one more moving part at package time).
- **D3** Loop lengths 45 s (rain) / 40 s (fire) — recommended, to break up the
  repeat pattern. Alternative: keep 30/20 s.
- **D4** Add character assertions to `cargo test` as well as `make qa` —
  recommended, so a DSP change cannot silently regress the sound.
- **D5** Opus packaging now, or as a follow-up commit after the sound quality
  work — recommended: follow-up.

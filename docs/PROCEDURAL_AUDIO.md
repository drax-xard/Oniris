# Procedural nature-sound generation

Research date: **2026-10-26**. This is an assessment of generating synthetic
"natural-sounding" soundscapes instead of, or alongside, recorded audio.

## Executive assessment

Procedural generation is a strong **supplement** for Oniris, not a replacement
for field recordings in v1.

- It is excellent for **textures** made of many similar micro-events: rain,
  wind, ocean surf, fire beds, streams, insects, and noise colors.
- It is weak for **discrete, recognizable events**: species-specific birds,
  gulls, owls, frogs, and thunder cracks. These are where synthetic audio most
  often sounds uncanny or generic.
- A 2026 perceptual study found that real recordings were rated as more
  believable for everyday/natural sounds, while procedural audio performed
  better for synthetic, sci-fi, and abstract sounds.
- The engineering and sound-design cost is real: a convincing rain model is a
  research project in itself, and tuning it for eight hours of non-fatiguing
  playback is harder than looping a good recording.
- For Oniris, the best near-term use is **offline procedural support layers and
  event variants** inside the existing pack format. This adds variation and
  reduces repetition/licensing cost with **zero runtime changes** and no battery
  penalty. Real-time procedural beds should wait for a proven prototype and
  on-device CPU/battery measurements.

## What "procedural" can mean

1. **Offline generation**: run an algorithm once, render a 60 s loop, encode it
   to Opus, and ship it as a normal pack asset. No engine changes, no runtime
   cost, deterministic output.
2. **Real-time synthesis**: generate samples on the audio thread or in a worker
   while playing. Enables infinite variation and parameter control (rain
   intensity, wind strength), but adds CPU, battery, and real-time-safety work.
3. **Hybrid**: recorded "hero" beds plus procedurally generated support layers
   or event variants. Usually the best quality/effort tradeoff.

The main synthesis families:

- **Filtered noise and stochastic textures** — rain hiss, wind, surf, fire base.
- **Physically informed models** — bubbles, droplets, impacts, modal resonators,
  fluid/foam models.
- **Granular / concatenative synthesis** — rearrange small recorded grains;
  sounds natural but still depends on source recordings and can be CPU-heavy.
- **Neural generation** — diffusion/autoregressive text-to-audio; high quality
  potential, but heavy, non-deterministic, and licensing-sensitive.

## Evidence and prior art

### Sound textures are statistical

McDermott and Simoncelli showed that "sound textures" — rainstorms, insect
swarms, galloping horses — are perceived through time-averaged statistics of
early auditory representations. This is the strongest argument for procedural
textures: if the statistics match, the brain accepts the result.

It also explains the limit: a texture is a collective of many similar events. A
single bird call, a thunder crack, or a door slam is not a texture; it has a
specific source and temporal structure that statistics do not capture well.

Reference: McDermott & Simoncelli, *Sound Texture Perception via Statistics of
the Auditory Periphery*, Neuron 2011.
<https://www.sciencedirect.com/science/article/pii/S0896627311005629>

### Rain synthesis is well studied

Real-time rain synthesis has a long history: stochastic droplet models, filtered
noise beds, and density/intensity controls. Zita's 2003 thesis focuses on
real-time stochastic rain simulation; game-audio engines such as Wwise have
published practical "pure synthesis" rain recipes that morph from light rain to
downpour.

This makes rain the best candidate for procedural support: the model is
understood, the parameters are intuitive, and the result can be layered under a
real recording.

References:

- A. Zita, *Computational Real-Time Sound Synthesis of Rain*, 2003.
  <https://www.diva-portal.org/smash/record.jsf?pid=diva2:19156>
- S. Liu et al., *Physically-based Statistical Simulation of Rain Sound*,
  SIGGRAPH 2019. <http://haonancheng.cn/attaches/2019%20SIGGRAPH.pdf>
- Audiokinetic, *Generating Rain With Pure Synthesis*.
  <https://www.audiokinetic.com/en/community/blog/generating-rain-with-pure-synthesis>

### Fire, water, and impacts have physical models

Physically informed models exist for bubbles, liquid sounds, fire, and impacts.
They are good at the "micro-event" layer (droplets, crackles, bubbles) but
rarely sound like a complete field recording on their own. They are most useful
as detail layers on top of a recorded bed.

References:

- K. van den Doel, *Physically Based Models for Liquid Sounds*, ACM TAP 2005.
  <https://dl.acm.org/doi/10.1145/1101530.1101554>
- J. Chadwick & D. James, *Animating Fire with Sound*, SIGGRAPH 2011.
- Andy Farnell, *Designing Sound*, MIT Press 2010 — practical Pure Data models
  for rain, fire, wind, water, insects, and more. The techniques are useful; the
  book's patches are educational material, so implement original code rather
  than copying them.

### Perceptual comparisons favor recordings for everyday sounds

A 2026 study compared procedural samples from an online engine with library
recordings in live-action and animated scenes. Real sound designs were rated as
more believable overall; procedural audio performed better for sci-fi/abstract
sounds and worse for everyday drama and natural actions. Participants asked for
more layering, stronger low frequencies, better attacks, and more variation in
waves, rain, fire, and droplets.

Reference: N. Garcia & J. Reiss, *Sound effects in media: A comparative analysis
of recorded and synthetic samples in live-action and animation*, arXiv 2026.
<https://arxiv.org/html/2605.31082>

This matches the practical consensus: procedural audio is a tool for variation
and interaction, not a drop-in replacement for a good recording.

### Commercial proof of demand

myNoise demonstrates that listeners value adjustable, non-looping, spectrally
shaped soundscapes. It combines generated noise, spectral shaping, and processed
recordings. It is a useful product reference, not a content source: the site is
for personal use only.

<https://mynoise.net/>

## Feasibility by sound

| Sound | Procedural feasibility | Best approach | Quality ceiling | Runtime cost |
|---|---|---|---|---|
| White/pink/brown noise | Very high | Filtered noise | Excellent | Very low |
| Rain bed | High | Filtered noise + Poisson droplets | Good; can sound "synthetic" if overdone | Low–medium |
| Rain droplets / drips | High | Modal resonators + stochastic events | Good | Low |
| Wind | High | Time-varying bandpass noise + gusts | Good for background; less convincing as a "place" | Low |
| Ocean surf | High | Modulated noise + foam/bubble detail | Good background; lacks spatial realism | Low–medium |
| Stream / river | Medium–high | Noise + bubble resonators | Good at low levels; can sound like static | Medium |
| Fire bed | Medium–high | Brown noise + crackle events | Convincing crackle; bed can sound like noise | Low |
| Fire crackles / pops | High | Modal clicks with random timing | Good | Low |
| Crickets / insects | High | Pulse trains + resonators | Good; species-specific patterns need tuning | Low |
| Frogs | Medium | Pulse trains + formants | "Froggy" but not species-specific | Low |
| Thunder rumble | Medium | Low-frequency noise + envelopes + reverb | Distant rumble works; close cracks are hard | Medium |
| Birds / gulls / owls | Low–medium | Physical models, concatenative grains, or samples | Easily uncanny; species identity is hard | Medium–high |
| Rich forest soundscape | Low | Layered models + recordings | Procedural alone rarely convinces | High |
| Neural text-to-audio | High quality potential | Diffusion/autoregressive models | Can be impressive, but licensing/eval issues | Very high |

## Tools and licensing

| Tool / crate | License | Fit for Oniris |
|---|---|---|
| Hand-written Rust DSP (biquads, noise, envelopes, modal resonators) | Our code | **Best long-term fit**: no dependency, full control, deterministic |
| `fundsp` | MIT OR Apache-2.0, `no_std` | Good for prototyping offline graphs; pure Rust |
| `dasp` | MIT OR Apache-2.0 | Useful PCM/signal utilities |
| `noise` | MIT OR Apache-2.0 | Useful for parameter modulation, not audio synthesis itself |
| `synfx-dsp` | **GPL-3.0-or-later** | Avoid unless the app is GPL |
| Faust | Compiler GPL; generated code can be commercial | Good prototyping language, but C++ integration adds build complexity |
| STK (C++) | Free/academic, patent caveats | Avoid for a commercial Rust app |
| Pure Data / SuperCollider / Csound | Various open-source licenses; heavy runtimes | Prototype only; not embedded in the Android core |
| Neural models (MusicGen, AudioLDM, etc.) | Often CC BY-NC or unclear training data | Avoid for v1; revisit for offline generation with licensed models |

`fundsp` version 0.23.0 is MIT/Apache-2.0 and `no_std`, which makes it a
reasonable offline prototyping tool. Even so, a small purpose-built DSP module
will be easier to keep deterministic, testable, and allocation-free on the audio
thread.

## Fit with Oniris today

The engine already provides most of the variation machinery:

- seamless bed loops;
- randomized event selection, gain, pan, and timing;
- per-event fades and `maxVoices`;
- LFOs on gain and pan;
- a lookahead limiter and loudness/true-peak tooling;
- a pack format with two beds and up to 16 event types;
- deterministic seeds.

That means we can test procedural content **without engine changes**:

1. Generate procedural **support beds** offline (rain detail, wind, fire base)
   and add them as the second bed.
2. Generate a pool of short procedural **event variants** (droplets, crackles,
   insect chirps) offline and list them in the existing `events[].files`
   arrays.
3. Let the existing scheduler randomize variants, timing, gain, and pan.

One constraint: a pack supports **two beds**. A procedural support bed competes
with a second recorded bed. For packs that already need two beds, either bake the
support layer into one of them offline, or prefer procedural **event variants**
(up to 16 files per event) which do not consume a bed slot. Adding a third bed is
a format and memory decision, not a free option.

This is the lowest-risk way to learn whether procedural content helps. It costs
no battery, no new manifest features, and no audio-thread work.

## Recommendation

### 1. Keep recorded beds as the "hero" layer

Use field recordings for the main identity of each pack: the rain-on-window
texture, the surf, the campfire, the forest. They carry authenticity and
emotional place in a way procedural models still struggle to match.

### 2. Use procedural content for support and variation

Prioritize:

- **Rain**: procedural droplet/detail layer under a recorded rain bed.
- **Wind**: procedural gust layer to add movement to a recorded forest/sea bed.
- **Fire**: procedural crackle event pool over a recorded fire bed.
- **Stream/water**: procedural bubble detail.
- **Insects**: procedural chirp/cricket event variants.
- **Noise**: Pure Noise is already procedural and is the cleanest case.

Generate these **offline** now, as normal Opus assets. Do not add a runtime
synthesizer until the value is proven.

### 3. Prototype one hybrid pack

Take **Rainy Night** and build two versions:

- A: recorded rain bed + recorded thunder events (current plan).
- B: recorded rain bed + procedural droplet layer + the same thunder events.

Run a blind listening test (ABX or MUSHRA) with 10–20 listeners, including a
30-minute fatigue check. Measure spectral centroid, modulation spectrum, event
rate, loudness, and true peak. If B is preferred or indistinguishable, keep the
procedural layer; otherwise drop it.

### 4. Add real-time synthesis only after a clear win

If hybrid content is clearly better, the next step is a **procedural event
generator** in Rust: tiny droplets, crackles, or chirps synthesized on demand.
This avoids storing dozens of variants and gives infinite variation. It is much
cheaper than synthesizing whole beds and fits the existing event model.

A real-time procedural **bed** (rain intensity, wind strength, surf height) is a
larger feature: it needs a new manifest layer type, parameter smoothing, CPU
budgets, and battery testing. Treat it as a later phase, not v1.

### 5. Avoid neural generation for v1

Neural audio can sound impressive, but the model weights often carry
non-commercial licenses (MusicGen weights are CC BY-NC), training-data
provenance is unclear, inference is expensive, and output is non-deterministic.
It is also hard to validate for eight-hour, low-level sleep playback. Revisit
only for offline asset creation with a model whose weights and training data are
clearly licensed for commercial use.

## Prototype plan

### Phase A — offline procedural layers (no engine changes)

1. Add a small Rust DSP module or CLI generator (separate from the engine) with:
   - colored noise (white/pink/brown);
   - one-pole/biquad filters;
   - Poisson event scheduling;
   - damped modal resonators for droplets/crackles;
   - equal-power loop crossfades.
2. Generate 60 s loops:
   - `rain-detail` (high-frequency droplets + light hiss);
   - `wind-gust` (bandpass noise with slow modulation);
   - `fire-crackle` event pool (8–16 short variants);
   - `insect-chirp` event pool.
3. Normalize to the pack targets and encode with `oniris encode-opus`.
4. Validate with `oniris validate`, `oniris analyze`, and the golden tests.

### Phase B — hybrid evaluation

1. Build Rainy Night A and B as above.
2. Run ABX/MUSHRA and a 30-minute fatigue test.
3. Compare objective metrics: spectral centroid, modulation spectrum, event
   rate, crest factor, loudness, true peak.
4. Decide: keep, tune, or drop the procedural layer.

### Phase C — runtime events (only if Phase B wins)

1. Add a `synth` event source to the manifest/engine (or generate tiny event
   buffers at load time).
2. Keep the generator allocation-free and deterministic.
3. Measure CPU and battery on a real device during an 8-hour soak.
4. Set a hard CPU budget (for example, <2–3% of one core) and fall back to
   samples if exceeded.

## Success criteria

- Blind listeners cannot reliably distinguish the hybrid from the recorded-only
  version, or prefer the hybrid.
- No fatigue or "synthetic" artifacts over 30 minutes at sleep volume.
- Loudness/true peak on target; no clicks at loop seams.
- Offline generation is deterministic and reproducible from a seed.
- Any runtime synthesis stays within the CPU/battery budget and never allocates
  on the audio thread.

## Risks

- **Uncanny valley**: synthetic rain/wind can sound like filtered noise rather
  than a place.
- **Tuning cost**: parameter tuning is open-ended and needs listening tests.
- **Repetition**: procedural variation can still fall into perceptible patterns
  if the event model is too simple.
- **CPU/battery**: real-time synthesis competes with the battery-first goal.
- **Asset licensing**: generated audio is clean, but reference code, neural
  weights, and training data may not be.
- **QA complexity**: more parameters and seeds mean more test cases.

## Bottom line

Procedural generation is worth pursuing as a **variation and support layer**, and
it is already a good fit for Pure Noise. It is not yet a substitute for the
recorded nature beds that give a sleep app its sense of place. Build the hybrid
offline first, measure it, and only then consider real-time synthesis.

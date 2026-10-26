# Procedural pack ideas: sci-fi and fantasy

Companion to [`PROCEDURAL_AUDIO.md`](PROCEDURAL_AUDIO.md). This explores how to
use procedural synthesis for **exotic, non-natural soundscapes** rather than
realistic nature recordings.

**Status:** **Deep Space Cabin** and **Enchanted Forest** are implemented in
`oniris gen-procedural-packs`. The other concepts below are proposals.

## Why exotic packs are a better procedural fit

The 2026 perceptual study found procedural audio was most convincing for
synthetic, sci-fi, and abstract sounds, and least convincing for everyday
natural actions. That is the opposite of the usual assumption.

For Oniris this suggests a product split:

- **Natural packs** (rain, seaside, forest, fire): real recordings are the
  quality benchmark.
- **Exotic packs** (space, fantasy, underwater, machines): procedural synthesis
  is competitive because there is no real-world reference to fail against. The
  listener judges atmosphere, not species accuracy.

Exotic procedural packs also give us:

- no recording licenses or attribution;
- tiny downloads (a few hundred KB to ~1 MB);
- infinite, non-repeating variation;
- easy variants and personalization;
- a clear reason for the “Procedural only” mode.

## Sleep-safe design rules

Exotic does not mean harsh. These packs still run all night at low volume.

- **No alarms.** Avoid siren-like beeps, rising alarm patterns, and notification
  intervals. Use soft, slow, low-passed tones.
- **No sudden transients.** Every event gets a fade-in/out and stays well below
  the limiter.
- **Slow motion.** LFO periods of 20–90 s; event intervals of 15–180 s.
- **Low event density.** A strange beep every 30–90 s is atmosphere; every 2 s
  is a stressor.
- **Balanced spectrum.** Sci-fi can easily become harsh; keep high-frequency
  energy modest and use low-mid body.
- **No voices, no intelligible speech, no animal distress.**
- **Targets:** beds about −23 LUFS, true peak ≤ −1.5 dBTP, no clicks at loop
  seams.
- **Naming:** evocative, not medical. “Deep Space Cabin”, not “Anxiety Relief
  Frequency”.

## Synthesis primitives

These are enough to build most of the concepts below:

| Primitive | Use |
|---|---|
| Colored noise + filters | wind, surf, air handling, dust, rain, fire base |
| Additive drone (5–12 partials, slow detune) | engine hum, magical hum, deep space, choirs |
| Modal resonators | metal creaks, stone, bells, ice, hull, chimes |
| FM / phase-modulated tones | soft beeps, chimes, alien calls, sonar |
| Pulse trains + bandpass | insects, crickets, alien chirps, frogs |
| Damped sine with pitch rise/fall | bubbles, drips, sonar pings |
| Granular texture | shimmer, dust, leaves, magical motes, water detail |
| Noise-burst envelopes | creaks, page turns, rustles, airlock thumps |
| Feedback delay / convolution reverb | cave, hall, space, underwater (offline) |
| Slow random modulation | gusts, swell, drift, engine instability |

All of these can be generated offline into 60 s beds and short event files, then
encoded to Opus. No engine changes are required for the first versions.

## Sci-fi pack concepts

### 1. Deep Space Cabin (`deep-space-cabin`)

A quiet habitat drifting between stars. Low engine hum, air handling, occasional
soft system tones and structural creaks.

- **Bed A:** 55 Hz + harmonics additive drone, very slow detune, low-pass.
- **Bed B:** filtered air-handling noise, slow bandpass drift.
- **Events:** soft console beeps (FM, 600–1200 Hz, very quiet), hull creaks
  (modal resonators), distant airlock thumps (low noise burst + reverb).
- **Mood:** safe, enclosed, steady.
- **Sleep safety:** no alarm intervals; beeps at least 30 s apart.

### 2. Alien Ocean (`alien-ocean`)

A sea on another world: slow surf-like noise, bubbles at unfamiliar pitches, and
distant whale-like calls.

- **Bed A:** filtered noise with slow swell; center frequency and Q drift.
- **Bed B:** sparse bubble resonators and low current rumble.
- **Events:** FM/formant calls with slow pitch contours, soft sonar pings,
  shell clicks.
- **Mood:** vast, organic, strange but calm.

### 3. Cryo Chamber (`cryo-chamber`)

A sleeping pod: sub-bass hum, fluid circulation, frost cracks, valve hisses.

- **Bed A:** 30–60 Hz hum + filtered fluid noise.
- **Bed B:** very low brown noise and slow pulse.
- **Events:** ice cracks (modal clicks), valve hiss (filtered noise), soft
  completion chime.
- **Mood:** suspended, protected, cold.

### 4. Orbital Greenhouse (`orbital-greenhouse`)

A research station growing plants in space: ventilation, water drips, soft
equipment tones, occasional leaf rustle.

- **Bed A:** air handling + low electrical hum.
- **Bed B:** water circulation + drip detail.
- **Events:** soft valve clicks, leaf rustles, distant console tones.
- **Mood:** optimistic, living, enclosed.

### 5. Derelict Ship (`derelict-ship`)

A quiet abandoned vessel: low drone, dripping water, electrical buzz, distant
metal groans.

- **Bed A:** low hull drone with slow pitch drift.
- **Bed B:** dripping water + air leak hiss.
- **Events:** metal groans (modal), sparks (short filtered bursts), distant
  clanks.
- **Caution:** keep it melancholic, not horror; no sudden bangs.

### 6. Terraform Wind (`terraform-wind`)

A thin wind across a new world: dust, pressure domes, distant machinery.

- **Bed A:** bandpass wind with slow gusts.
- **Bed B:** low dust rumble.
- **Events:** dust devil swells, dome creaks, soft machinery pulses.
- **Mood:** exposed but sheltered.

## Fantasy pack concepts

### 1. Enchanted Forest (`enchanted-forest`)

A moonlit forest with soft wind, a hidden brook, distant owls, and rare chimes.

- **Bed A:** wind through leaves (filtered noise + slow resonances).
- **Bed B:** brook (noise + bubble resonators).
- **Events:** soft chimes (additive/FM bells), owl calls (formant/pulse synth),
  leaf rustles, distant frogs.
- **Mood:** magical, gentle, alive.
- **Sleep safety:** chimes are quiet and sparse; no sparkling high-frequency
  showers.

### 2. Wizard's Tower (`wizards-tower`)

A stone tower at night: fireplace, wind through arrow slits, a settling magical
hum, pages and potion bubbles.

- **Bed A:** fire (brown noise + crackle events).
- **Bed B:** stone room tone + wind.
- **Events:** page turns (noise bursts), potion bubbles (rising-pitch
  resonators), soft bell, distant thunder.
- **Mood:** warm, studious, safe.

### 3. Dragon's Cave (`dragons-cave`)

A vast cave with a sleeping dragon: deep breathing rumble, dripping water, ember
cracks.

- **Bed A:** low breathing-like rumble (brown noise with slow swell).
- **Bed B:** cave room tone + water drips.
- **Events:** deep breath swells, ember pops, scale clicks, distant rock falls.
- **Caution:** no roar, no growl; the dragon is asleep.

### 4. Elven Grove (`elven-grove`)

An ethereal grove: airy choir-like drone, wind, water, harp-like chimes.

- **Bed A:** additive/formant choir drone, slow detune.
- **Bed B:** wind + water.
- **Events:** harp/chime tones, soft bird-like calls, leaf rustles.
- **Mood:** luminous, weightless.
- **Caution:** avoid recognisable melody loops; keep the chimes stochastic.

### 5. Dwarven Hall (`dwarven-hall`)

A deep stone hall: room tone, distant forge, water, slow hammering.

- **Bed A:** stone room tone + low rumble.
- **Bed B:** water drip + distant ventilation.
- **Events:** slow hammer strikes (modal), anvil ring, ember pops.
- **Caution:** hammering can be alerting; use a slow, soft, irregular pulse and
  keep it distant.

### 6. Fae Market at Night (`fae-market-night`)

A distant magical market: soft murmurs without intelligible voices, lantern
chimes, wind, water.

- **Bed A:** filtered crowd-like noise without speech (formant-shaped noise).
- **Bed B:** wind + water.
- **Events:** soft chimes, cloth rustles, distant laughter-like contours (no
  words).
- **Caution:** human-like sounds are risky; keep them abstract and quiet.

### 7. Floating Islands (`floating-islands`)

High-altitude wind, distant waterfalls, sky-whale calls.

- **Bed A:** high wind with slow gusts.
- **Bed B:** distant waterfall noise.
- **Events:** sky-whale calls (FM/formant), bird-like calls, wind chimes.
- **Mood:** open, airy, adventurous but calm.

## Manifest sketches

### Deep Space Cabin

```json
{
  "schemaVersion": 1,
  "id": "deep-space-cabin",
  "name": { "en": "Deep Space Cabin" },
  "description": {
    "en": "A quiet habitat drifting between stars: low engine hum, air handling, and rare soft system tones."
  },
  "version": "1.0.0",
  "license": "CC0-1.0",
  "procedural": true,
  "tags": ["space", "scifi", "drone", "procedural"],
  "loudnessLufs": -23.0,
  "truePeakDbtp": -1.5,
  "beds": [
    {
      "id": "engine",
      "file": "audio/engine.opus",
      "loop": { "startSample": 0, "endSample": 2880000 },
      "gainDb": -6.0,
      "lfo": { "target": "gain", "depth": 0.4, "periodSec": 61.0 }
    },
    {
      "id": "air",
      "file": "audio/air.opus",
      "loop": { "startSample": 0, "endSample": 2880000 },
      "gainDb": -12.0,
      "pan": 0.15,
      "lfo": { "target": "pan", "depth": 0.2, "periodSec": 43.0 }
    }
  ],
  "events": [
    {
      "id": "beep",
      "files": ["audio/beep-01.opus", "audio/beep-02.opus", "audio/beep-03.opus"],
      "minIntervalSec": 30.0,
      "maxIntervalSec": 90.0,
      "gainDb": [-18.0, -12.0],
      "pan": [-0.3, 0.3],
      "fadeInMs": 30,
      "fadeOutMs": 300,
      "maxVoices": 1
    },
    {
      "id": "creak",
      "files": ["audio/creak-01.opus", "audio/creak-02.opus", "audio/creak-03.opus"],
      "minIntervalSec": 20.0,
      "maxIntervalSec": 70.0,
      "gainDb": [-20.0, -12.0],
      "pan": [-0.6, 0.6],
      "fadeInMs": 200,
      "fadeOutMs": 800,
      "maxVoices": 1
    }
  ],
  "credits": [
    {
      "title": "Procedural deep-space cabin",
      "author": "Oniris",
      "source": "synthetic",
      "license": "CC0-1.0",
      "modified": false
    }
  ]
}
```

### Enchanted Forest

```json
{
  "schemaVersion": 1,
  "id": "enchanted-forest",
  "name": { "en": "Enchanted Forest" },
  "description": {
    "en": "A moonlit forest with soft wind, a hidden brook, distant owls, and rare chimes."
  },
  "version": "1.0.0",
  "license": "CC0-1.0",
  "procedural": true,
  "tags": ["forest", "fantasy", "magic", "procedural"],
  "loudnessLufs": -23.0,
  "truePeakDbtp": -1.5,
  "beds": [
    {
      "id": "wind",
      "file": "audio/wind.opus",
      "loop": { "startSample": 0, "endSample": 2880000 },
      "gainDb": -8.0,
      "lfo": { "target": "gain", "depth": 0.8, "periodSec": 53.0 }
    },
    {
      "id": "brook",
      "file": "audio/brook.opus",
      "loop": { "startSample": 0, "endSample": 2880000 },
      "gainDb": -12.0,
      "pan": -0.2
    }
  ],
  "events": [
    {
      "id": "chime",
      "files": ["audio/chime-01.opus", "audio/chime-02.opus", "audio/chime-03.opus"],
      "minIntervalSec": 20.0,
      "maxIntervalSec": 60.0,
      "gainDb": [-20.0, -14.0],
      "pan": [-0.7, 0.7],
      "fadeInMs": 20,
      "fadeOutMs": 1500,
      "maxVoices": 1
    },
    {
      "id": "owl",
      "files": ["audio/owl-01.opus", "audio/owl-02.opus"],
      "minIntervalSec": 40.0,
      "maxIntervalSec": 120.0,
      "gainDb": [-20.0, -14.0],
      "pan": [-0.5, 0.5],
      "fadeInMs": 100,
      "fadeOutMs": 600,
      "maxVoices": 1
    }
  ],
  "credits": [
    {
      "title": "Procedural enchanted forest",
      "author": "Oniris",
      "source": "synthetic",
      "license": "CC0-1.0",
      "modified": false
    }
  ]
}
```

## Implementation plan

### Shared procedural library

Create a small `procedural` module (CLI-only at first) with:

- noise colors and filter helpers;
- additive drone builder;
- modal resonator bank;
- FM/chime helper;
- pulse-train insect/chirp helper;
- bubble/drip helper;
- noise-burst envelope helper;
- simple reverb (Schroeder/FDN) for offline renders;
- event-pool writer that normalizes peaks and emits manifest credits.

### Recipe registry

Add a registry of pack recipes:

```rust
struct ProceduralPack {
    id: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    seed: u64,
    build: fn(&Path) -> Result<()>,
}
```

`oniris gen-procedural-packs` iterates the registry. This keeps each pack
reproducible and makes it easy to add variants.

### Validation

Every generated pack must pass:

- `oniris validate`;
- loudness/true-peak targets;
- loop-seam continuity;
- `oniris analyze` jump check for smooth content;
- credits and `procedural: true`;
- deterministic output for a fixed seed.

### UI

- The existing “Procedural only” switch already filters by the `procedural` flag.
- Add tags/visual cues for **Sci-fi** and **Fantasy** sections later.
- Consider a “Synthetic” badge on pack cards.
- Keep recorded and procedural packs visually distinct without stigmatizing
  either.

## Product recommendation

### Start with two

1. **Deep Space Cabin** — the safest sci-fi concept: steady, enclosed, no alarms.
2. **Enchanted Forest** — the safest fantasy concept: soft wind/water/chimes.

Both are fully procedural, low-event-density, and sleep-safe. They also test two
different synthesis palettes (drone/noise vs. wind/water/chimes).

**Implemented:** both are generated by `oniris gen-procedural-packs`.

### Then evaluate

- ABX/MUSHRA against a recorded nature pack to check quality expectations.
- 30-minute fatigue test at sleep volume.
- Retention: do users keep the procedural packs selected after a week?
- Skip "Derelict Ship" and "Dragon's Cave" until the gentle concepts prove
  themselves; horror-adjacent sounds are riskier for sleep.

### Longer term

If exotic procedural packs work, a **parameterized soundscape engine** becomes
possible: the user chooses a palette (space, forest, cave, ocean) and adjusts
density, brightness, and motion. That requires real-time synthesis, CPU/battery
budgets, and a new manifest/layer model. Treat it as a v2 differentiator, not a
v1 requirement.

## Risks

- **Trope fatigue:** sci-fi beeps and drones can sound cheap if overused.
- **Sleep safety:** alarms, sudden cracks, and dense events wake people.
- **Tuning cost:** every concept needs its own listening passes.
- **Quality expectations:** users may compare exotic packs to film/game sound
  design, which has a much larger production budget.
- **Differentiation:** if the packs sound like generic noise, they add little.
- **CPU/battery:** only relevant once real-time synthesis is added; offline
  packs are free at playback.

## Bottom line

Exotic packs are the **best first place to apply procedural generation** because
the listener has no real-world reference to compare against. Start with **Deep
Space Cabin** and **Enchanted Forest**, keep them quiet and sparse, and use the
existing procedural-only mode to let users opt in. Realistic nature packs should
stay recording-led.

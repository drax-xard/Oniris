# Asset pipeline

Turns source recordings into validated, packaged Oniris packs. `ingest.py` drives
the whole chain; the Rust CLI does the loudness math, Opus encoding, and
packaging.

Source research and approved licenses: [`../../docs/CONTENT_SOURCES.md`](../../docs/CONTENT_SOURCES.md).
Every recording that enters a pack is logged in [`sources.csv`](sources.csv).

## Layout

```
tools/asset-pipeline/
  ingest.py          # the pipeline (one JSON spec -> one pack)
  specs/*.json       # one spec per pack: beds, events, credits, loop choices
  sources.csv        # source log: SHA-256, format, license, edits
```

Masters never enter the repository. Specs point at absolute paths outside the
repo (the convention is `~/oniris-sources/masters/` for beds and
`~/oniris-sources/events/` for event material). Generated packs and archives go
to `dist/`, which is gitignored.

## Run it

```bash
# build, validate, check the mix, log the sources, and package the pack
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json

# re-process one asset while tuning (nothing else is rebuilt)
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json --only gull

# stop after validate, before oniris pack
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json --skip-package

# render and check the mix without rebuilding anything
/usr/bin/python3 tools/asset-pipeline/ingest.py verify tools/asset-pipeline/specs/seaside.json

# format + SHA-256 of a new download, for the sources log and the spec
/usr/bin/python3 tools/asset-pipeline/ingest.py probe ~/oniris-sources/masters/*.wav
```

`make spec SPEC=<pack>` and `make audition SPEC=<pack>` wrap the build and the
6-minute listening render.

Output on a clean seaside v0.2 build (`dist/`), 37 s wall clock:

```
== bed surf  <- 827530__yevgverh__ocean_coast_04_092025_0659am.wav
   loop 120.0 s  |  -23.0 LUFS  TP -6.5 dBTP  peak -6.54 dBFS
   seam 0.012634 (0.19x p99.9 step)  spectral 0.7748  DC [-0.0, -0.0] -> cleaned
   opus 1427.9 kB  |  all files within pack policy
== bed breeze  <- 575380__cinetony__leaves-rustle-in-the-wind.wav
   loop 40.0 s  |  -23.0 LUFS  TP -4.5 dBTP  peak -5.1 dBFS
   seam 0.000783 (0.03x p99.9 step)  spectral 0.8118  DC [-0.0, 0.0] -> cleaned
   opus 476.4 kB  |  all files within pack policy
== validate OK  OK Seaside
== mix  -23.1 LUFS  TP -5.6 dBTP  (target -23.0 / -1.5 dBTP)
   surf     gain   +0.0 dB  solo -22.9 LUFS  (+0.2 LU vs mix)
   breeze   gain  -12.0 dB  solo -34.9 LUFS  (-11.8 LU vs mix)
== sources.csv updated (2 row(s))
== packed seaside -> dist/seaside.onirispack (1900.4 kB, 4 files, sha256 2bf47f...)
```

Then audition a render before shipping:

```bash
core/target/release/oniris render --pack dist/packs/seaside --minutes 6 --seed 7 --out dist/work/render.wav
core/target/release/oniris analyze dist/work/render.wav
```

## What each step does

1. **Decode** at 48 kHz through ffmpeg, with cleanup applied at the source rate:
   `highpass` (DC and sub-rumble, 12 dB/oct), optional `bandreject` notches at
   the mains hum frequency and its first three harmonics, optional `lowpass`, and
   optional shelf EQ (`"eq": [{"filter": "treble", "f": 4000, "g": -4}]`) for a
   recording whose balance is too bright for a sleep bed.
2. **Loop**: cut `lengthSec` from `startSec`, then fold the following
   `crossfadeSec` of source back over the head with an equal-power (sin/cos)
   crossfade. The result is exactly `lengthSec` long and its last sample flows
   into its first, so the engine's loop point is inaudible. The pipeline refuses
   a region that does not have `length + crossfade` seconds of source left, and
   `source_start_sec` is recorded in the log so a loop can be reproduced.
   Choose `startSec` by crest as well as by seam: a window that looks flat can
   still contain a close-mic rustle cluster that forces the whole bed quiet.
3. **Normalize** through `oniris normalize` (EBU R128 to `loudnessLufs`, true
   peak ceiling `truePeakDbtp`). When the ceiling binds, the bed lands quieter
   than target rather than clipping — the printed LUFS is the authority.
4. **Limit (optional)** — `"limiter": {"peakDb": -12.0, "releaseMs": 30.0}`
   caps isolated transients with a smoothed gain envelope, then normalises again
   so the freed headroom goes back into the steady-state level. The detector is a
   sliding maximum, not a moving average: a single-sample snap vanishes inside a
   mean and the limiter would never fire on the events it exists for.
5. **Gate** with [`tools/audio-qa/measure.py --check`](../audio-qa/measure.py):
   true peak, clipping, DC offset. A failure aborts the build.
6. **Encode** with `oniris encode-opus`: 96 kbps stereo beds, 64 kbps events,
   gapless (pre-skip and final granule handled by the writer).
7. **Manifest**: `endSample` is written from the actual loop frame count, so it
   always equals the decoded frame count of the encoded asset. Bed pan is a
   scalar; **event gain is a `[min, max]` range** and event pan is a `[min, max]`
   pair — the engine samples both per instance. Do not try to give a file its own
   gain; that is not what the format means.
7. **Docs**: `LICENSE` lists per-asset attribution plus the transformation
   notice; `README.md` lists loops and credits. The pack license is the strictest
   asset license in the spec (`CC0` / `Public Domain` < `CC BY 3.0` < `CC BY 4.0`).
8. **Package**: `oniris validate`, `oniris pack`, then `oniris verify-pack`.
   Archives are deterministic — the same directory packs to the same SHA-256.

## Measuring a loop

Two numbers matter and the pipeline prints both:

- `seam` / `seam_vs_p99` — the sample step between the last and first sample of
  the loop, as a multiple of the file's own 99.9th-percentile sample step. Under
  ~1× is inaudible; the seaside bed measures 0.19×.
- `spectral` — cosine similarity of the magnitude spectra of the last and first
  4096 samples. Ambience sits around 0.7–0.85; a value below ~0.5 means the loop
  point changes the timbre and the region needs moving.

Pick `startSec` by comparing candidates, not by taste alone: measure the level
before and after the wrap and prefer a window whose join does not step the
level. The seaside window (20 s, 120 s long, 3 s crossfade) moves +0.2 dB across
the join; the 0 s window moves −5 dB, which is audible as pumping.

Also compare the crest factor (peak ÷ RMS) of candidates. A window can have a
perfect seam and still be wrong because it holds a close-mic rustle cluster:
`Lot 2 Forest` at 10 s had a 31 dB crest, so normalisation stopped at −23.7 LUFS
on a −1.5 dBTP ceiling and left isolated snaps at −6 dBFS; the 40 s window peaks
at −17.2 dBFS and reaches −23.0 LUFS, and a −12 dBFS limiter finally brings the
crest to 20 dB with zero sample steps above a quarter of full scale.

## Verifying the mix, not just the assets

`build` renders the pack and checks the summed result before it packages, and
`verify` does the same on demand:

```
== mix  -23.1 LUFS  TP -5.6 dBTP  (target -23.0 / -1.5 dBTP)
   surf     gain   +0.0 dB  solo -22.9 LUFS  (+0.2 LU vs mix)
   breeze   gain  -12.0 dB  solo -34.9 LUFS  (-11.8 LU vs mix)
```

This matters as soon as a pack has two beds. Each bed is normalised to the pack
target *on its own*, so their sum plays louder than either: seaside's second bed
had to come down 12 dB to stay a support layer, and forest-deep's main bed needed
−0.7 dB because the rustle layer alone pushed the mix to −22.3 LUFS. A failure
here stops the build before packaging, with the fix being a `gainDb` in the spec
rather than a new loudness target.

## Checking a shipped pack in the app

The pilot's proof is an import, not a byte count:

```bash
export PATH=$PATH:$HOME/Android/Sdk/platform-tools:$HOME/Android/Sdk/emulator
adb push dist/seaside.onirispack /sdcard/Download/
# Library -> Import -> (picker: Show roots -> Downloads) -> seaside.onirispack
adb shell dumpsys activity services dev.oniris | grep isForeground
adb shell dumpsys media_session | grep -m2 PlaybackState
adb logcat -d | grep -E "FATAL|E AndroidRuntime"
```

A successful import lists the pack with its description and `N beds · M events`
plus its tags (no `procedural` tag), selects into the mix, and reports
`Playing · 29 MB decoded · offline` — the decoded size matching `oniris validate`
output is the check that every Opus asset decoded.

## Dependencies

`python3` with numpy (system `/usr/bin/python3` has it), `ffmpeg`, and a built
`oniris` binary (`cargo build --release --manifest-path core/Cargo.toml -p oniris-cli`;
the binary lands at `core/target/release/oniris`). `ingest.py` falls back to
`cargo run` when no binary is present, which is much slower per asset.

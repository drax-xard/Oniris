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
# build, validate, log the sources, and package the pilot pack
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json

# re-process one asset while tuning (nothing else is rebuilt)
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json --only gull

# stop after validate, before oniris pack
/usr/bin/python3 tools/asset-pipeline/ingest.py build tools/asset-pipeline/specs/seaside.json --skip-package

# format + SHA-256 of a new download, for the sources log and the spec
/usr/bin/python3 tools/asset-pipeline/ingest.py probe ~/oniris-sources/masters/*.wav
```

Output on a clean seaside build (`dist/`), 26 s wall clock:

```
== bed surf  <- 827530__yevgverh__ocean_coast_04_092025_0659am.wav
   loop 120.0 s  |  -23.0 LUFS  TP -6.5 dBTP  peak -6.54 dBFS
   seam 0.012634 (0.19x p99.9 step)  spectral 0.7748  DC [-0.0, -0.0] -> cleaned
   opus 1427.9 kB  |  all files within pack policy
== event gull  (2 file(s))
   audio/gull-01.opus: 1.85 s  -23.0 LUFS  TP -9.3 dBTP  15.1 kB  |  all files within pack policy
   audio/gull-02.opus: 1.17 s  -23.0 LUFS  TP -11.5 dBTP  9.7 kB  |  all files within pack policy
== validate OK  OK Seaside
== sources.csv updated (2 row(s))
== packed seaside -> dist/seaside.onirispack (1450.0 kB, 5 files, sha256 fa5865...)
```

Then audition a render before shipping:

```bash
core/target/release/oniris render --pack dist/packs/seaside --minutes 6 --seed 7 --out dist/work/render.wav
core/target/release/oniris analyze dist/work/render.wav
```

## What each step does

1. **Decode** at 48 kHz through ffmpeg, with cleanup applied at the source rate:
   `highpass` (DC and sub-rumble, 12 dB/oct), optional `bandreject` notches at
   the mains hum frequency and its first three harmonics, optional `lowpass`.
2. **Loop**: cut `lengthSec` from `startSec`, then fold the following
   `crossfadeSec` of source back over the head with an equal-power (sin/cos)
   crossfade. The result is exactly `lengthSec` long and its last sample flows
   into its first, so the engine's loop point is inaudible. The pipeline refuses
   a region that does not have `length + crossfade` seconds of source left, and
   `source_start_sec` is recorded in the log so a loop can be reproduced.
3. **Normalize** through `oniris normalize` (EBU R128 to `loudnessLufs`, true
   peak ceiling `truePeakDbtp`). When the ceiling binds, the bed lands quieter
   than target rather than clipping — the printed LUFS is the authority.
4. **Gate** with [`tools/audio-qa/measure.py --check`](../audio-qa/measure.py):
   true peak, clipping, DC offset. A failure aborts the build.
5. **Encode** with `oniris encode-opus`: 96 kbps stereo beds, 64 kbps events,
   gapless (pre-skip and final granule handled by the writer).
6. **Manifest**: `endSample` is written from the actual loop frame count, so it
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

## Dependencies

`python3` with numpy (system `/usr/bin/python3` has it), `ffmpeg`, and a built
`oniris` binary (`cargo build --release --manifest-path core/Cargo.toml -p oniris-cli`;
the binary lands at `core/target/release/oniris`). `ingest.py` falls back to
`cargo run` when no binary is present, which is much slower per asset.

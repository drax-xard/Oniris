# Audio QA

Measurement gate for generated content. It reports the numbers used in
`docs/PACK_QUALITY_REVIEW.md` and fails a build when an asset breaks pack policy.

## Requirements

- Python 3 with numpy: `pip install -r tools/audio-qa/requirements.txt`
- `ffmpeg` on PATH (EBU R128 integrated loudness and true peak)

If the default `python3` has no numpy, point the Makefile at one that does:

```bash
make qa PYTHON=/usr/bin/python3
```

## Usage

```bash
# Report on every generated asset
tools/audio-qa/measure.py packs/*/audio/*.wav

# Gate: exit 1 on policy violations (true peak > -1.5 dBTP, clipped samples,
# DC offset > 0.001). This is what `make qa` runs after regenerating packs.
tools/audio-qa/measure.py --check packs/*/audio/*.wav

# Machine-readable, for trend tracking
tools/audio-qa/measure.py --json packs/test-rain/audio/rain.wav
```

## What it measures

- **Loudness:** integrated LUFS and true peak (ffmpeg `ebur128=peak=true`).
- **Levels:** sample peak, RMS, crest factor, clipped-sample count, per-channel
  DC offset.
- **Stereo:** L/R correlation and side-minus-mid level.
- **Spectrum:** energy-weighted spectral centroid, 8-band energy share (including
  the 0–20 Hz band that wastes headroom and the 300 Hz–4 kHz band a phone
  speaker can actually reproduce), and spectral flatness.
- **Loop:** seam delta at the wrap point, expressed as a multiple of the 99.9th
  percentile sample-to-sample step, so a seam that would click stands out.

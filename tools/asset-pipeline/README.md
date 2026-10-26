# Asset pipeline

This directory will hold the scripts that turn source recordings into validated
packs. The current scaffold provides the Rust validator/renderer; the Python
pipeline is the next content milestone.

## Intended flow

1. **Ingest** 48 kHz/24-bit WAV masters.
2. **Clean**: trim, DC offset, denoise, de-hum.
3. **Loop**: choose a loop region, crossfade tail into head, verify periodicity.
4. **Normalize**: bed to -23 LUFS integrated; events balanced relative to the
   bed; true peak ≤ -1.5 dBTP.
5. **Encode**: Opus (96 kbps stereo beds, 64 kbps mono/stereo events).
6. **Manifest**: generate/validate `manifest.json`; verify loop sample counts.
7. **QA**: render a 60 s preview, loop 100×, listen on speaker/headphones.
8. **Package**: zip with `LICENSE`, `README.md`, art, and audio.

## Suggested dependencies

See `requirements.txt`. Prefer ffmpeg for encoding and `pyloudnorm` for
measurement; keep the pipeline deterministic and scriptable.

## Current commands

The Rust CLI already provides the codec and archive steps; the Python spool
will add ingest and loudness measurement around it.

```bash
# WAV master -> gapless 48 kHz Ogg/Opus
cargo run --manifest-path ../../core/Cargo.toml -p oniris-cli -- \
  encode-opus master.wav master.opus --bitrate 96000

cargo run --manifest-path ../../core/Cargo.toml -p oniris-cli -- validate <pack-dir>

# Pack, verify, and unpack a distributable archive
cargo run --manifest-path ../../core/Cargo.toml -p oniris-cli -- \
  pack <pack-dir> <pack-id>.onirispack
cargo run --manifest-path ../../core/Cargo.toml -p oniris-cli -- \
  verify-pack <pack-id>.onirispack

cargo run --manifest-path ../../core/Cargo.toml -p oniris-cli -- render --pack <dir> --minutes 30 --out preview.wav
```

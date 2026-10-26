# Pack format

A sound pack is a directory during development and a `.onirispack` zip for
distribution. Packs are data only; they must never contain executable code.

## Layout

```
rain-and-thunder/
  manifest.json
  audio/
    rain.opus
    thunder-01.opus
    thunder-02.opus
  art/
    cover.webp
  LICENSE
  README.md
```

Rules:

- `manifest.json` is required.
- Audio paths are relative, use `/`, and may not contain `..`, `.` segments,
  backslashes, or absolute paths.
- `LICENSE` must state the license of every audio/art asset. Use CC0 where
  possible; CC-BY requires attribution in the app.
- Art is optional; 512×512 WebP is the target.

## Manifest (schema version 1)

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
      "lfo": { "target": "gain", "depth": 1.5, "periodSec": 47.0 }
    }
  ],
  "events": [
    {
      "id": "thunder",
      "files": ["audio/thunder-01.opus", "audio/thunder-02.opus"],
      "minIntervalSec": 25.0,
      "maxIntervalSec": 90.0,
      "initialDelaySec": 15.0,
      "gainDb": [-10.0, -4.0],
      "pan": [-0.5, 0.5],
      "fadeInMs": 800,
      "fadeOutMs": 1200,
      "maxVoices": 1
    }
  ],
  "compatibility": { "excludes": [] }
}
```

### Top-level fields

| Field | Required | Notes |
|---|---|---|
| `schemaVersion` | yes | Must be `1`. |
| `id` | yes | Lowercase ASCII letters, digits, `-`; 1–64 chars; unique. |
| `name` | yes | Localized map; `en` required. |
| `description` | no | Localized map. |
| `version` | yes | Pack version, not app version. |
| `license` | yes | SPDX identifier or short name. |
| `tags` | no | Used for search and suggestions. |
| `loudnessLufs` | no | Integrated loudness of the bed. Target -23 LUFS. |
| `truePeakDbtp` | no | Target -1.5 dBTP. |
| `art` | no | Safe relative path. |
| `beds` | yes | 1–2 beds. |
| `events` | no | Up to 16 event types. |
| `compatibility` | no | `excludes` lists pack IDs that should not be mixed. |

### Bed fields

| Field | Required | Notes |
|---|---|---|
| `id` | yes | Unique within the pack. |
| `file` | yes | Safe relative path. Ogg/Opus in production; WAV is a test/fallback format. |
| `loop.startSample` | yes | Source-rate sample index; inclusive. |
| `loop.endSample` | yes | Source-rate sample index; exclusive. Loop length 1 s–10 min. |
| `gainDb` | no | -60..6 dB; default 0. |
| `pan` | no | -1..1; default 0. Stereo balance law. |
| `lfo.target` | no | `gain` or `pan`. |
| `lfo.depth` | no | dB for gain (0–6), -1..1 for pan (0–1 recommended). |
| `lfo.periodSec` | no | 5–600 s. |

Beds are mixed continuously. The asset pipeline must make the loop seamless;
the engine wraps at `endSample` without an additional crossfade.

### Event fields

| Field | Required | Notes |
|---|---|---|
| `id` | yes | Unique within the pack. |
| `files` | yes | 1–16 variants. |
| `minIntervalSec` | no | 1–3600 s; default 20. |
| `maxIntervalSec` | no | ≥ min; ≤3600; default 60. |
| `initialDelaySec` | no | 0–3600; default random in [min, max]. |
| `gainDb` | no | `[min, max]`; -60..6; default [-8, -3]. |
| `pan` | no | `[min, max]`; -1..1; default [0, 0]. |
| `fadeInMs` / `fadeOutMs` | no | ≤10000; defaults 500/1000. |
| `maxVoices` | no | 1–4; default 1. |

Events are scheduled independently per active pack. The engine picks a variant,
gain, and pan uniformly within the ranges. Use wider pan ranges for small,
localized sounds and narrow ranges for thunder/rumble.

## Audio format

**Production target:** Opus in Ogg, 48 kHz.

- Beds: stereo, 96 kbps, 20 ms frames.
- Events: mono or stereo, 64 kbps, 20 ms frames.
- Source masters: 48 kHz, 24-bit WAV, archived outside the app repository.
- Loudness: beds normalized to -23 LUFS integrated; events balanced relative to
  the bed; true peak ≤ -1.5 dBTP.

**Current decoder:** Ogg/Opus is the production path; `.opus` and `.ogg` files
are decoded natively. WAV remains supported for lossless tests and debugging.
Both formats pass through `decode_audio()`, which normalizes channels and
applies any required resampling once at load time.

For Opus, loop points are expressed at 48 kHz. The decoder applies the RFC 7845
pre-skip and end-trim, so a gapless file's decoded frame count matches the
samples that were encoded. `oniris-cli encode-opus` writes such files. Do not
ship production content as WAV.

## Safety and validation

The loader must:

- Reject paths that escape the pack directory.
- Reject unknown major schema versions.
- Enforce size and decoded-memory limits.
- Validate loop bounds against the decoded file.
- Reject NaN/infinite numeric fields.
- Treat audio and JSON as untrusted input.

`cargo run -p oniris-cli -- validate <dir>` performs structural, path, and decode
validation for both WAV and Ogg/Opus assets. Add a zip extractor with the same
rules before supporting `.onirispack` imports.

## Authoring checklist

- [ ] Loop 100× and listen for clicks, hum, pattern, and level drift.
- [ ] Check on phone speaker, wired headphones, and Bluetooth.
- [ ] Normalize loudness; verify true peak.
- [ ] Test every event variant at low volume for jump-scares.
- [ ] Write `LICENSE` and `README.md` with source/attribution.
- [ ] Run the validator.
- [ ] Render a 30-minute mix and listen for repetition.

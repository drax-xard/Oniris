# Content sources for the first packs

Research date: **2026-10-26**. Licenses change; re-check the linked terms before
every release and keep a dated copy of each license page with the source
material.

This document answers one question: **where can we obtain high-quality nature
audio that we may legally ship inside Oniris and inside downloadable
`.onirispack` archives?**

## Decision

For v1, use only:

1. **CC0 1.0** — public-domain dedication; redistribution and modification are
   allowed without attribution.
2. **CC BY 3.0/4.0** — commercial use, modification, and redistribution allowed
   with attribution and a change notice.
3. **Public domain** — U.S. Government works (NPS) and verified public-domain
   archive items.

Avoid **CC BY-NC**, **CC BY-ND**, **CC BY-SA**, **Sampling+**, and every custom
"royalty-free" license that restricts app use or redistribution. The packs are
distributed as `.onirispack` files containing audio, so a license that only
permits "use in a project" is not enough if it forbids supplying the audio as an
asset pack.

**Recommended mix:** CC0/PD for the bulk of the beds, CC BY only where the
recording quality clearly justifies the attribution burden. The manifest
`credits` array and the app's “Licenses & credits” section are implemented; use
them for every CC BY asset.

Procedural generation is assessed separately in
[`PROCEDURAL_AUDIO.md`](PROCEDURAL_AUDIO.md). The short version: use it for
support layers and variation, not as a replacement for recorded beds.

## License matrix

| Source | License | Commercial | Modify | Redistribute in `.onirispack` | Attribution |
|---|---|---:|---:|---:|---|
| Freesound CC0 | CC0 1.0 | yes | yes | yes | not required |
| Freesound CC BY | CC BY 3.0/4.0 | yes | yes | yes | required |
| NPS Sound Gallery | public domain | yes | yes | yes | requested |
| Luftrum free recordings | CC BY 4.0 | yes | yes | yes | required |
| Field Recording Project | CC0 1.0 | yes | yes | yes | not required |
| Internet Archive `aporee_*` | Public Domain Mark | yes | yes | yes, verify item | not required |
| Wikimedia Commons | per file | depends | depends | depends | depends |
| Synthesized noise | CC0 (our work) | yes | yes | yes | not required |
| 99Sounds | custom | **no sleep apps** | yes | **no** | optional |
| Free To Use Sounds (free) | custom | **no sleeping apps** | yes | **no** | required |
| Sonniss #GameAudioGDC | custom | yes in projects | yes | **no asset packs** | not required |
| BBC Sound Effects | RemArc | **no** | limited | **no** | required |
| Xeno-canto | mostly BY-NC-ND | **no** | **no** | **no** | required |
| ZapSplat | custom | **no if main value** | yes | **no** | required |
| Pixabay / Mixkit | custom | yes in projects | yes | **standalone restricted** | not required |

"Redistribute in `.onirispack`" is the critical column: a license that permits
use in an app but forbids supplying the audio as files is not sufficient for a
downloadable pack.

## Approved sources

### 1. Freesound — primary source

<https://freesound.org/>

- **Licenses:** per sound: CC0, CC BY, or CC BY-NC. Filter out NC and any
  Sampling+ uploads.
- **Why:** the largest catalogue of long field recordings; downloads return the
  **original file** (WAV/FLAC/AIFF/MP3), not a transcode. The advanced search
  can filter license, duration, format, sample rate, bit depth, and channels.
- **Account:** required for downloads.
- **Best for:** rain, fire, ocean, forest, wind, streams, and event one-shots.
- **Practical filters:** `Soundscapes` category, `duration >= 60 s`,
  `type: wav` or `flac`, `samplerate: 48000` or `96000`, `bitdepth: 24` or
  `32`, `channels: 2`. 16-bit is acceptable for short events.
- **Attribution:** required for CC BY; include author, sound title, Freesound
  URL, license, and a note that the audio was edited/looped/encoded.

Curated searches (CC0, soundscapes, 60 s+, 48 kHz stereo WAV):

- Rain:
  <https://freesound.org/search/?q=rain&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Ocean/waves:
  <https://freesound.org/search/?q=ocean+waves&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Forest:
  <https://freesound.org/search/?q=forest&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Wind:
  <https://freesound.org/search/?q=wind&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Fire (use 30 s+; long CC0 fire beds are scarce):
  <https://freesound.org/search/?q=fire&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B30+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Crickets / night insects:
  <https://freesound.org/search/?q=crickets&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>
- Streams:
  <https://freesound.org/search/?q=stream&f=license:%22Creative+Commons+0%22+category:%22Soundscapes%22+duration:%5B60+TO+*%5D+type:%22wav%22+samplerate:48000+channels:2>

Known contributors worth auditioning (check the license on each file):

- `felix.blume` — very high-quality CC0 rain, forest, ocean, birds.
- `kijjaz` — CC0 rain/thunder, recent recordings.
- `giddster` — CC0 cabin/forest rain.
- `selsel` — CC0 rain on tin roof.
- `mxlacy`, `sokworks`, `saralana`, `bruno.auzet`, `YevgVerh` — CC0
  nature/weather recordings.
- `klankbeeld`, `InspectorJ` — excellent CC BY ambiences; use when the quality
  justifies attribution.

### 2. National Park Service Sound Gallery — public domain

<https://www.nps.gov/subjects/sound/gallery.htm>

- **License:** the gallery states the files are **in the public domain** and may
  be downloaded; credit is requested, not required.
- **Formats:** MP3, usually 56–160 kbps. Good for **events and accents**, not
  long lossless beds.
- **Best for:** thunder, crickets, spotted owl, western gull, dawn chorus,
  streams, frogs, coyotes, whales.
- **Credit line:** `National Park Service, public domain, <page URL>`.

Direct examples (verified 2026-10-26):

- Rain: <https://www.nps.gov/subjects/sound/sounds-rain.htm>
- Thunder (46 s, 128 kbps): <https://www.nps.gov/subjects/sound/sounds-thunder.htm>
- Ocean: <https://www.nps.gov/subjects/sound/sounds-ocean.htm>
- Stream: <https://www.nps.gov/subjects/sound/sounds-stream.htm>
- Crickets (29 s, 160 kbps): <https://www.nps.gov/subjects/sound/sounds-crickets.htm>
- Spotted owl (5 s, 128 kbps): <https://www.nps.gov/subjects/sound/sounds-spotted-owl.htm>
- Western gull: <https://www.nps.gov/subjects/sound/sounds-western-gull.htm>

The NPS pages also link Yellowstone, Rocky Mountain, and Glacier Bay sound
libraries. Check each clip's credit line and file quality; use the same
public-domain rule.

### 3. Luftrum Free Field Recordings — CC BY 4.0

<https://www.luftrum.com/free-field-recordings/>

- **License:** **CC BY 4.0**; attribution required. The paid/donation 22 GB
  library waives the CC license, but the free playlist is CC BY.
- **Content:** 60 free recordings (ocean, forest, rain, dawn chorus, room tones,
  cityscapes) from a 22 GB library; gear includes Sound Devices 702 and DPA
  4060. Quality is high.
- **Formats:** the free playlist is downloadable from SoundCloud; verify whether
  each track downloads as WAV or MP3. The full library ranges 16/44.1 to
  24/192 kHz.
- **Best for:** high-quality ocean, forest, and rain beds if the download is
  lossless; otherwise use for events or request the full library.
- **Attribution:** credit Luftrum and the original recordist listed on the page,
  link to the library, note CC BY 4.0 and the edits.

### 4. The Field Recording Project — CC0

<https://thefieldrecordingproject.co.uk/sound-library-for-creatives>

- **License:** **CC0**; commercial use, modification, and distribution allowed;
  no attribution required. The site asks users not to repackage recordings and
  sell them as a sound/sample library. Oniris is a sleep app, not a sample
  library, but keep packs as app content rather than marketing them as a raw
  sound library.
- **Content:** UK beaches, forests, rivers, parks, markets, stations; WAV and
  MP3 downloads.
- **Best for:** ocean/beach beds, forest/river beds, and some events.
- **Quality:** varies by contributor; audition for wind noise, clipping, and
  handling noise.

### 5. Internet Archive / Radio Aporee — public domain mark (verify)

<https://archive.org/details/radio-aporee-maps>

- **License:** the `aporee_*` soundmap items are usually marked **Public Domain
  Mark 1.0**; the `radio_aporee_log_*` live-radio items are **CC BY-SA 3.0** and
  must be excluded.
- **Content:** a very large field-recording map (rain, thunder, forests, cities,
  water). Files often include FLAC and WAV.
- **Best for:** additional rain/forest/water beds and specific locations.
- **Caveat:** Public Domain Mark is a statement, not a license. Prefer items with
  clear provenance and keep a copy of the item metadata. Audition carefully;
  quality varies widely.

### 6. Wikimedia Commons — per-file CC0/CC BY/public domain

<https://commons.wikimedia.org/>

- **License:** per file. Many audio files are CC BY-SA, which is **not** in the
  v1 allow-list. Use the file page and `extmetadata` API to confirm CC0, CC BY,
  or public domain before downloading.
- **Best for:** specific one-shots (frogs, birds, weather) and a few long
  ambiences.
- **Caveat:** OGG/OGV is common; check the original file and license.

### 7. Synthesized noise — no external source

- **Pure Noise** packs can be generated from pink/brown/white noise with slow
  filtering and LFOs. No license, no provenance, and perfectly loopable.
- Generate at 48 kHz/24-bit, then run the same loop, loudness, and true-peak
  checks as recorded content.
- This should be the first default pack because it is completely unblocked.

## Excluded or high-risk sources

| Source | Why not for v1 |
|---|---|
| **99Sounds** | License explicitly forbids use in "noise apps", "white noise apps", and "sleep sound apps". <https://99sounds.org/license/> |
| **Free To Use Sounds** (free license) | Free license requires attribution and explicitly forbids uploading to "sleeping applications"; app developers need a separate developer license. <https://www.freetousesounds.com/license-agreement> |
| **Sonniss #GameAudioGDC** | Excellent quality, but the license forbids supplying the sounds as an asset pack/sample library, even modified. A `.onirispack` is hard to defend as "not an asset pack" without legal review. <https://sonniss.com/gdc-bundle-license/> |
| **BBC Sound Effects** | RemArc license is non-commercial; commercial use requires a separate license. |
| **Xeno-canto** | Default is CC BY-NC-ND; many recordings are not usable in a commercial app, and ND blocks our edits. Some CC BY-SA uploads exist but are not worth the curation risk. |
| **Macaulay Library (Cornell)** | Mostly all rights reserved; only a small subset is CC licensed and hard to filter. |
| **ZapSplat** | License restricts use where the sounds are the main value of the app; a sleep-sound app is exactly that. |
| **Pixabay / Mixkit** | Custom licenses allow app use but restrict standalone redistribution; `.onirispack` files make that ambiguous. Also mostly MP3, not lossless masters. Review only with legal sign-off. |
| **YouTube / social media "free" nature audio** | No reliable license or provenance. Do not use. |
| **CC BY-NC / ND / SA** | NC blocks commercial use; ND blocks looping/editing; SA forces the whole pack to share-alike. |

## Proposed first five packs

These map to the presets in `docs/DEVELOPMENT_PLAN.md`. Exact files are chosen
after auditioning 3–5 candidates per role.

### 1. Rainy Night (`rainy-night`)

- **Bed:** long steady rain on a roof/window or in a forest. Candidates:
  Freesound CC0 `giddster` "Forest cabin summer rain 1" (719062), `selsel`
  "Rain on tin roof" (824785), `felix.blume` "Birds and light rain" (414149);
  Luftrum "Binaural Rain" / "Forest Before The Rain" (CC BY).
- **Second bed (optional):** distant low rumble, or a second rain texture with
  a different spectral balance.
- **Events:** thunder (NPS Thunder, public domain; Freesound CC0
  `GaryBran` 76685), rain drips (Freesound CC0 `TheGloomWorker` 683249).
- **Target license:** CC0 if all Freesound CC0 + NPS; CC BY if Luftrum is used.

### 2. Seaside (`seaside`)

- **Bed:** gentle surf on sand or rocks. Candidates: Freesound CC0 `YevgVerh`
  "Ocean_coast_04..." (827530), `bruno.auzet` "standing in Atlantic waves"
  (851795); Luftrum "Ocean Waves Crashing" (CC BY); Field Recording Project
  beaches (CC0).
- **Second bed (optional):** distant deep surf, or a quieter lapping texture.
- **Events:** western gull (NPS, public domain), seagulls (Freesound CC0
  `felix.blume` 155747).
- **Target license:** CC0/PD preferred; CC BY if Luftrum is used.

### 3. Cabin Fire (`cabin-fire`)

- **Bed:** fireplace or campfire. CC0 long beds are scarce; search 30 s+ and
  stitch. CC BY candidates: `FrédéricDubois` "Fireplace oven..." (804852),
  `klankbeeld` campfire, `InspectorJ` fireplace.
- **Events:** crackles/pops extracted from the bed or separate CC0/CC BY
  recordings. Avoid events with speech, music, or traffic.
- **Target license:** CC0 if possible; CC BY is acceptable for a convincing fire.

### 4. Forest Deep (`forest-deep`)

- **Bed:** forest ambience with wind/leaves. Candidates: Luftrum "Dawn Chorus"
  and "Forest Before The Rain" (CC BY); Field Recording Project "West Woods" /
  "Janesmoor, New Forest" (CC0); Freesound CC0 `mxlacy` 636946, `felix.blume`
  forest recordings.
- **Second bed (optional):** stream/river (NPS Stream, public domain; Field
  Recording Project rivers; Freesound CC0 streams).
- **Events:** spotted owl (NPS), crickets (NPS or Freesound CC0), dawn chorus
  (NPS), frogs (NPS/Freesound CC0).
- **Target license:** CC0/PD preferred; CC BY if Luftrum is used.

### 5. Pure Noise (`pure-noise`)

- **Bed:** generated pink/brown noise, gently low-passed, with very slow gain
  LFO. No external source.
- **Variants:** soft pink, deep brown, "rain-like" filtered noise.
- **Target license:** CC0.

## Acquisition and QA workflow

1. **Create a source log** (`tools/asset-pipeline/sources.csv` or a private
   spreadsheet) with: pack, role, source, URL, author, license, download date,
   original filename, SHA-256, sample rate/bit depth/channels, duration, edits,
   and audition notes. A starter file with placeholder rows is committed at
   `tools/asset-pipeline/sources.csv`; replace the `TODO` fields as candidates
   are auditioned.
2. **Download the original**, not a preview. Save a dated copy of the license
   page and the item metadata alongside the master.
3. **Audition** on headphones and phone speaker. Reject clipping, hum, handling
   noise, wind blasts, speech, music, traffic, and obvious MP3 artifacts.
4. **Prepare the bed:** trim, remove DC offset, repair obvious clicks, choose a
   stationary loop region, apply an equal-power tail-to-head crossfade (1–3 s),
   and verify periodicity for wave/fire rhythms.
5. **Normalize:** bed to -23 LUFS integrated; events relative to the bed; true
   peak at or below -1.5 dBTP. Use `oniris loudness` and `oniris normalize`.
   The current Rust decoder normalizes WAV to 16-bit PCM for playback and
   analysis. For 24/32-bit masters, do repeated gain/loop edits in a
   float-capable editor or the Python pipeline before the final
   `encode-opus` pass; do not round-trip through 16-bit multiple times.
6. **Encode:** 96 kbps stereo beds, 64 kbps mono events, 48 kHz, 20 ms frames,
   with `oniris encode-opus`.
7. **Validate and QA:** `oniris validate`, `oniris analyze`, loop 100×, render a
   60 s preview, and listen on speaker/wired/Bluetooth.
8. **Package:** `oniris pack`; keep the SHA-256 for the release manifest.
9. **Credit:** write `LICENSE` and `README.md` inside the pack; add the
   machine-readable credits to the manifest (see below).

Do **not** commit large masters to the app repository. Keep them in a separate
asset store or Git LFS and commit only manifests, scripts, small previews, and
license/credit files.

## Manifest credits (needed before CC BY content ships)

The current manifest has a pack-level `license` but no per-asset attribution.
Add an optional `credits` array (schema v1, backward-compatible):

```json
"credits": [
  {
    "asset": "audio/rain.opus",
    "title": "Forest cabin summer rain 1",
    "author": "giddster",
    "source": "https://freesound.org/s/719062/",
    "license": "CC0-1.0",
    "modified": true
  },
  {
    "asset": "audio/thunder-01.opus",
    "title": "Thunder",
    "author": "National Park Service",
    "source": "https://www.nps.gov/subjects/sound/sounds-thunder.htm",
    "license": "Public Domain",
    "modified": true
  }
]
```

Rules:

- `license` is the SPDX identifier or `Public Domain`.
- `modified: true` covers trimming, looping, EQ, normalization, and Opus
  encoding; the app's attribution screen should say "edited for this app".
- If any asset is CC BY, the pack-level `license` should be the most restrictive
  license in the pack (usually `CC-BY-4.0`) and every CC BY asset must appear in
  `credits`.
- CC0 and public-domain assets may be credited for courtesy but are not required.
- The app should show credits per pack and a global "Licenses" screen. This is
  already required by `docs/LICENSING.md`.

## Cover art and metadata

- Prefer original abstract art derived from the app palette (gradients, grain,
  simple shapes). It has no third-party license and stays consistent across
  packs.
- If using photography or illustration, require CC0, CC BY, or public domain.
  Unsplash/Pexels-style licenses usually allow app use but restrict selling
  unmodified copies; keep the license proof and do not treat them as CC0.
- Target 512×512 WebP, muted colors, no text that becomes illegible at small
  sizes, and no bright white at night.
- Write original descriptions and tags; do not copy text from source pages.
- Fill `loudnessLufs` and `truePeakDbtp` from `oniris loudness`, not from the
  source file's metadata.

## Open decisions

1. **Commercial model:** if Oniris is paid or ad-supported, NC sources stay
   excluded (they are excluded anyway for simplicity).
2. **Attribution tolerance:** if a CC BY attribution screen is acceptable, Luftrum
   and high-quality Freesound CC BY recordings materially improve the beds. If
   not, restrict v1 to CC0/PD and accept a narrower selection.
3. **Downloadable packs:** if `.onirispack` files are distributed publicly,
   custom "royalty-free" licenses are too risky. Stay with CC0/CC BY/PD.
4. **Lossless masters:** confirm the Luftrum free playlist and Field Recording
   Project downloads are WAV/FLAC before committing to them for beds; otherwise
   use them for events or purchase/donate for the lossless library.
5. **Cover art:** source separately under CC0/CC BY, or generate original
   gradients/illustrations. Do not reuse album art or stock images without a
   license.

## Immediate next steps

1. **Done:** manifest `credits`, `oniris credits`, and an Android licenses
   section. The Android UI still needs a device build to verify.
2. **Done:** Pure Noise pack generator (`oniris gen-noise-pack packs`), with
   two normalized CC0 beds and credits.
3. Audition 3–5 candidates for Rainy Night and Seaside from Freesound CC0 and
   NPS; record the license/provenance in `sources.csv`.
4. Add Forest Deep and Cabin Fire once the bed quality is confirmed.
5. Keep a `CREDITS.md`/`LICENSE` per pack and a dated license archive.

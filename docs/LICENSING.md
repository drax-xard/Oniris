# Content licensing

## Policy

- Prefer **CC0** recordings. They are the simplest to ship and redistribute.
- **CC-BY** is acceptable if the pack's `LICENSE`/`README.md` names the author,
  source URL, and license, and the app's attribution screen lists it.
- **Self-authored** recordings are allowed; document the date, location, and
  equipment in the pack README.
- Do **not** use YouTube rips, streaming-service audio, "royalty-free" tracks
  whose terms forbid redistribution, or unlicensed field recordings.
- AI-generated audio is not forbidden, but it must be disclosed, checked for
  quality/artifacts, and covered by the generator's terms. Prefer real
  recordings for launch content.

## Per-pack requirements

Every pack directory contains:

```
LICENSE      # license text or SPDX identifier + attribution
README.md    # sources, author, date, processing notes
```

The manifest's `license` field must match. If a pack mixes assets under
different licenses, list each asset and its license in `README.md`; the most
restrictive license governs the pack.

## Attribution screen

The app must include a licenses/attribution screen listing:

- Pack name and version
- Author/source
- License
- Link to the full license text (bundled or web)

Build this screen before the first public release. It is cheap now and painful
later.

## Code license

The Rust core, Android app, and tools need a code license before public release.
MIT or Apache-2.0 are the usual choices for a portable audio core; choose one
after deciding whether the project is open source. Until then, treat the
repository as all-rights-reserved.

## Third-party code

The audio core uses `opus-pure` (BSD-3-Clause) for Ogg/Opus decode and encode.
Pack archives use `zip` (MIT), `sha2` (MIT/Apache-2.0), and `tempfile`
(MIT/Apache-2.0). Their copyright notices must ship with binary distributions;
add them to the in-app attribution/license screen before the first public
release. Keep the dependency list and notices current as crates are added.

## Launch content sources

- Own field recordings (rain, fire, sea, forest).
- CC0 repositories (Freesound CC0 filter, Wikimedia Commons, Internet Archive
  public-domain collections).
- Commissioned recordings with a written redistribution license.

Keep original masters and license emails in a separate asset archive, not in the
app repository.

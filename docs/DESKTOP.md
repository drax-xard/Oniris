# Desktop app plan (macOS / Linux first)

This sketches the work needed to turn the shared Rust core into a desktop app.
Android remains the primary product; desktop is a second shell around the same
engine, pack format, and procedural content.

## Goal

A native desktop app that can:

- discover and play installed packs;
- select multiple packs, set master/per-pack volume, and use the sleep timer;
- switch between recorded and procedural-only libraries;
- import `.onirispack` archives and show credits/licenses;
- run offline, with no account, telemetry, or network requirement;
- build and package cleanly for macOS and Linux.

Windows is explicitly out of scope for the first pass.

## Recommended stack

| Area | Choice | Why |
|---|---|---|
| UI | **iced 0.14** | Pure Rust, MIT, MSRV 1.88 (matches the workspace), native macOS/Linux, Elm-style state/update/view, `Task`/`Subscription` for async work, `daemon` support for background apps |
| Audio output | **cpal 0.18** | CoreAudio/ALSA/JACK backends (PipeWire via its ALSA/Pulse compatibility), Apache-2.0, small API |
| Command queue | **rtrb 0.4** | Real-time-safe SPSC ring buffer, MIT/Apache |
| File dialogs | **rfd 0.17** | Native open/save dialogs, MIT |
| App dirs | **directories 6** | Correct macOS/Linux config/data locations, MIT/Apache |
| Settings | `serde` + `serde_json` | Already in the workspace; human-readable settings |
| Tray/media (later) | `tray-icon`, `souvlaki` | Tray menu + MPRIS/Now Playing, MIT/Apache and MIT |
| Packaging | `cargo-packager` | macOS `.app`/`.dmg`, Linux AppImage/`.deb`, MIT/Apache |

Notes:

- `tray-icon` 0.26 currently requires Rust 1.90; when tray support lands, either
  bump the desktop crate's MSRV or pin an older compatible version.
- If `cargo-packager` does not cover a format, fall back to `cargo-bundle`
  (macOS) and `cargo-deb`/`cargo-appimage` (Linux).

### UI alternatives

- **eframe/egui 0.33** — MIT/Apache, MSRV 1.88, immediate mode, fastest to
  prototype. 0.37 requires Rust 1.95, which would raise the workspace MSRV.
  Good fallback if iced slows the first milestone.
- **Tauri 2** — web UI in a system webview. Better visual polish and
  accessibility, but adds HTML/CSS/JS, WebKitGTK packaging, and a larger
  attack surface. Not recommended for the first pass.
- **GTK4/libadwaita** — native Linux feel, but macOS packaging is awkward and
  the license/runtime story is heavier.
- **Slint** — GPL/commercial licensing; avoid.
- **Qt** — C++/Python and licensing complexity; avoid.

## Why the shared core makes this cheap

`oniris-core` is already platform-neutral:

- `Engine` loads packs, mixes beds/events, applies LFOs/fades/timer/limiter, and
  renders interleaved stereo `i16`;
- `oniris-pack` validates manifests and packs/unpacks `.onirispack`;
- manifests carry `procedural`, `credits`, and `license`;
- the engine supports `set_mix` crossfades, per-pack gain, sleep timer, and
  memory accounting;
- the CLI already generates procedural packs and renders/analyzes WAVs.

The desktop shell needs to supply **audio output, UI, settings, and OS
integration** — not another engine.

## Proposed crate layout

Add a new workspace member:

```
core/
  oniris-desktop/
    src/
      main.rs          # iced application entry
      app.rs           # AppState, Message, update/view
      audio.rs         # cpal host + command queue
      controller.rs    # play/pause, selection, timer, volume
      library.rs       # pack discovery, install, import
      settings.rs      # JSON settings + presets
      ui/
        mod.rs
        library.rs     # pack list/cards
        controls.rs    # volume, timer, play/pause
        credits.rs     # licenses/credits panel
        settings.rs    # audio device, behavior
    resources/
      icon.png
      oniris.desktop
      Info.plist
```

Add `oniris-desktop` to `members` but **not** to `default-members`, so the
existing core/Android CI does not need desktop system libraries. A separate
desktop workflow builds it on macOS and Ubuntu.

## Feature parity target

| Feature | Android today | Desktop MVP | Later |
|---|---|---|---|
| Pack library / selection | yes | yes | — |
| Multi-pack mix | yes | yes | — |
| Master volume | yes | yes | — |
| Per-pack volume | engine only | no | D2 |
| Sleep timer | yes | yes | — |
| Procedural-only filter | yes | yes | — |
| Credits / licenses | yes | yes | — |
| `.onirispack` import | no | no | D2 |
| Presets | no | no | D2 |
| Background playback | foreground service | stop on close | D3 tray |
| Media controls | MediaSession | no | D3 MPRIS/Now Playing |
| Bundled packs | assets | resources | — |
| Auto-update | no | no | post-v1 |

## Audio host design

### Stream setup

- Use `cpal::default_host().default_output_device()`.
- Prefer **48 kHz**; if the device does not support it, use its default rate.
  The engine already resamples Opus/WAV at load time.
- Prefer `f32` output; convert the engine's interleaved `i16` samples with
  `sample as f32 / 32768.0`.
- Request `BufferSize::Fixed(512)` when supported; fall back to the backend
  default. At 48 kHz this is ~10.7 ms.
- Handle mono/other channel counts:
  - mono: average L/R;
  - stereo: direct;
  - >2: write L/R to the first two channels and zero the rest (or duplicate for
    a simple downmix; decide after testing).
- Subscribe to `StreamError` and device-loss events; surface a UI message and
  fall back to the default device.

### Real-time callback

The callback owns:

- `Engine`;
- a preallocated `Vec<i16>` scratch block;
- an `rtrb::Consumer<AudioCommand>`;
- status atomics (playing/finished/underruns).

Per callback:

1. drain commands (play, pause, stop, gains, timer, mix changes);
2. render `frames` into scratch with `Engine::render_i16`;
3. convert/write to the device buffer;
4. update status atomics;
5. never allocate, lock, log, or read files.

### Loading packs without blocking audio

`Engine::load_pack` decodes and can take hundreds of milliseconds. Do not call it
on the audio thread.

Three options, in increasing order of polish:

1. **Stop/rebuild (v1):** stop the stream, build a new `Engine` on a worker
   thread, swap it in, restart. Simple; a short gap on selection changes.
2. **Prepared engine swap (recommended first):** build `Engine` off-thread, send
   `Box<Engine>` over an SPSC queue, swap at a block boundary, and send the old
   engine back to the UI thread for dropping. No audio-thread allocation or
   deallocation.
3. **Core preload API (long-term):** expose `prepare_pack(dir) -> Arc<LoadedPack>`
   / `Engine::insert_pack`, decode on a worker, send an `Arc` to the audio thread,
   and use the existing `set_mix` crossfade. This also benefits Android's live
   mix changes.

Start with option 1 or 2. Add option 3 when live selection changes need to be
seamless.

### Device changes and errors

- Persist the selected device name; if it disappears, fall back to the default
  and notify the UI.
- Expose a "Retry audio" button.
- Add an underrun counter and a decoded-memory indicator (hidden by default,
  like Android).
- If no device is available, show a clear error screen rather than failing
  silently.

## UI scope

### MVP (parity with the Android main screen)

- Pack library list with:
  - name, description, bed/event count, tags, procedural badge;
  - selection toggle;
  - refresh/scan.
- **Procedural only** filter (same semantics as Android: hide recorded packs,
  exclude them from the active mix).
- Master volume.
- Sleep timer presets (off, 15, 30, 45, 60, 90 min).
- Play/pause, status ("Playing · N MB decoded · offline").
- Credits/licenses panel from manifest `credits`.
- Empty states:
  - no packs: point to `oniris gen-procedural-packs` or import;
  - no procedural packs: point to `make procedural`;
  - native engine/device error.

### Next

- Per-pack volume sliders.
- Presets: save/load/delete named mixes (pack IDs, volumes, timer).
- Import `.onirispack` via file dialog and drag-and-drop.
- Pack details: full description, tags, credits, license.
- Settings: audio device, buffer size, start minimized, close behavior.
- Keyboard shortcuts: Space play/pause, Up/Down volume, M mute, T timer.
- Compact/mini-player window.

### Later

- System tray menu (play/pause, show, quit).
- MPRIS (Linux) / Now Playing (macOS) media controls.
- "Keep playing when the window closes" option.
- Presets sync/export (local only).
- Windows port.

## Data and settings

Use `directories::ProjectDirs`:

- macOS config: `~/Library/Application Support/dev.oniris.Oniris/`
- Linux config: `~/.config/oniris/`
- data/packs: `data_dir/packs/`
- bundled packs: read-only `resources/packs/` copied into `data_dir/packs/`
  on first run (same idea as Android assets).
- imports: unpack into a temporary directory, validate, then move into
  `data_dir/packs/<id>`; never extract untrusted zips in place.

`settings.json`:

```json
{
  "selectedPackIds": ["deep-space-cabin", "rainy-night-procedural"],
  "masterVolume": 0.8,
  "timerMinutes": 45,
  "proceduralOnly": false,
  "audioDevice": "MacBook Pro Speakers",
  "bufferFrames": 512,
  "closeBehavior": "stop",
  "presets": []
}
```

Use atomic writes (temp file + rename) to avoid corrupting settings on crash.

## Packaging and distribution

### macOS

- `.app` bundle with bundle id `dev.oniris.desktop`.
- `.dmg` for direct distribution.
- Icons: `.icns` generated from the app icon.
- Local builds can be ad-hoc signed; public distribution needs a Developer ID
  certificate, hardened runtime, and notarization (`notarytool`). Defer the paid
  signing setup until there is a release candidate.
- Decide whether the app is a normal window app or a menu-bar app. For v1, a
  normal window is simpler; add a tray/menu-bar mode later.

### Linux

- AppImage for broad compatibility, plus `.deb` for Debian/Ubuntu.
- `.desktop` file with `Categories=AudioVideo;Audio;Player;`.
- Register `.onirispack` as a MIME type so double-click import works later.
- Test on a clean Ubuntu 24.04+ VM with PipeWire/PulseAudio and on a distro with
  plain ALSA.
- Runtime dependencies: ALSA/PulseAudio, a working OpenGL/Vulkan driver for
  iced's renderer (or the tiny-skia fallback).
- Flatpak later if distribution requires sandboxing.

### CI

Add `.github/workflows/desktop.yml`:

- matrix: `macos-latest`, `ubuntu-latest`;
- Linux packages: `libasound2-dev`, `libxkbcommon-dev`, `libwayland-dev`,
  `libudev-dev`, `pkg-config` (adjust to the final UI/audio backends);
- `cargo build -p oniris-desktop`, `cargo test -p oniris-desktop`,
  `cargo clippy -p oniris-desktop -- -D warnings`;
- package artifacts on version tags;
- keep the existing Rust core workflow separate and exclude the desktop crate
  from it (`--workspace --exclude oniris-desktop` or explicit `-p` list).

Audio-device tests are not reliable in CI. Test the engine by rendering to WAV
headlessly; leave real playback to manual QA.

## Phases

### D0 — Scaffold and playback (1–2 days)

- Add `oniris-desktop` crate.
- iced window with a dark theme and a minimal layout.
- cpal output stream, engine in the callback, play/pause.
- Load one generated procedural pack from `packs/`.
- Exit criteria: audio plays on macOS; pause/resume works; no underruns in a
  30-minute run.

### D1 — Library and settings (2–3 days)

- Pack discovery from the desktop data dir and bundled resources.
- Selection, master volume, timer, procedural-only filter.
- Credits/licenses panel.
- Settings persistence.
- Exit criteria: desktop matches the Android main screen functionally; settings
  survive restart.

### D2 — Import and presets (2–3 days)

- Import `.onirispack` with `oniris-pack::unpack_archive`.
- Drag-and-drop or file dialog.
- Pack details, delete/uninstall, open data folder.
- Presets: save/load/delete.
- Exit criteria: a user can install a pack, mix it, save a preset, and restart.

### D3 — OS integration (2–3 days)

- Tray icon and menu.
- MPRIS/Now Playing media controls.
- Close behavior (stop vs keep playing in tray).
- Keyboard shortcuts.
- Exit criteria: playback continues/minimizes as configured; media keys work on
  Linux and macOS.

### D4 — Packaging and CI (2–3 days)

- macOS `.app` + `.dmg`; Linux AppImage + `.deb`.
- Desktop CI matrix.
- Icons, `.desktop`, MIME type, license/credits screen.
- Exit criteria: artifacts install and run on clean macOS and Ubuntu machines.

### D5 — Hardening (ongoing)

- Device hotplug, sample-rate changes, audio errors.
- Accessibility and keyboard-only navigation.
- Long-run CPU/memory soak.
- Windows port and store packaging.

## Testing and QA

- **Unit:** settings serialization, library discovery, import path safety,
  controller state transitions.
- **Headless audio:** reuse the core/CLI render tests; compare stats for fixed
  seeds.
- **Manual:** macOS CoreAudio and Ubuntu PipeWire/PulseAudio/ALSA; speakers,
  wired headphones, Bluetooth; device switch during playback; sleep timer.
- **Soak:** 2–8 hours of playback; log underruns, CPU, memory, and device
  errors.
- **Accessibility:** keyboard navigation, screen-reader labels; iced's
  accessibility support should be verified early. If it is insufficient, this
  may force a UI toolkit change before D1.

## Risks and open questions

- **Toolkit risk:** iced 0.14 is current and MSRV-compatible, but it is
  experimental. Validate accessibility and Linux rendering before investing in
  the UI.
- **Audio backend risk:** ALSA/PipeWire device naming and sample-rate quirks;
  test on real Linux hardware, not just containers.
- **Real-time safety:** pack loading must never run in the callback. The
  prepared-engine swap or a core preload API is mandatory.
- **Two UIs:** Android Compose and desktop iced will diverge. Define a feature
  matrix and keep the core/pack semantics shared.
- **Background behavior:** closing the window on macOS/Linux does not have an
  Android-like foreground service. Decide between stop-on-close and a tray app.
- **Packaging/signing:** macOS notarization and Linux AppImage runtime libraries
  are release-time work, not D0 work.
- **Desktop is not a phone:** battery is less critical, but CPU/thermal and
  audio glitch behavior still matter.
- **Scope:** do not port every Android feature at once. MVP is library + mix +
  timer + procedural filter + credits.

## Recommended first step

Build a thin vertical slice:

1. Add the `oniris-desktop` crate with iced + cpal + rtrb.
2. Open a window with Play/Pause and a pack selector.
3. Load `packs/rainy-night-procedural` and play it through the default device.
4. Show a decoded-memory and underrun indicator.

That proves the UI stack, the audio callback, and the shared core on this
machine. Then add the library, settings, and import in D1–D2.

Do not start with packaging, tray integration, or media keys. They add platform
complexity before the core desktop loop is proven.

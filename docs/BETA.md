# Oniris 1.0.0-beta2 — tester guide

This is an internal, release-mode Android beta for real-device testing. It is
not a Play Store build.

## What is included

- Android 8.0+ (API 26), target SDK 36.
- Native audio engine for `arm64-v8a`, `armeabi-v7a`, and `x86_64`.
- Four bundled CC0 procedural packs: Pure Noise, Rainy Night, Deep Space Cabin,
  and Enchanted Forest.
- Pack library, `.onirispack` import, per-pack and master volume, four built-in
  presets plus user-saved presets, sleep timer, live mix changes, foreground
  playback, audio focus, headphone-disconnect pause, and lock-screen controls.
- No accounts, ads, analytics, or network requirement for playback.

## Requirements

- An Android 8.0+ phone, tablet, or emulator.
- About 30 MB of free space for the app and decoded audio.
- Optional: `adb` for command-line install and logs.

## Install

1. Download `oniris-beta.apk` from the GitHub release assets.
2. Verify the SHA-256 printed in the release notes if you want to check the
   download:

   ```bash
   shasum -a 256 oniris-beta.apk
   ```

3. Install from the device:

   - Copy the APK to the device and open it with Files/Downloads.
   - Allow "install unknown apps" for the app you use to open it, then install.
   - If a previous debug build is installed, uninstall `dev.oniris` first
     because it uses a different signing key.

4. Or install over USB:

   ```bash
   adb install -r oniris-beta.apk
   ```

5. Open Oniris and allow notifications when prompted. The notification is
   required for foreground playback and lock-screen controls.

## First run

1. Pick a preset: **Rainy Night**, **Deep Space**, **Forest**, or **Pure
   Noise**. Each preset sets a 45-minute timer.
2. Press **Play**. The app fades in and keeps playing with the screen off.
3. Optional: open **Sound packs** to select up to four packs and adjust each
   pack's volume.
4. Optional: change the sleep timer or save the current mix with **Save mix**.

## Using Oniris

- **Presets** — Apply a built-in or saved mix. **Save mix** stores the current
  pack selection, per-pack volumes, master volume, and timer.
- **Sound packs** — Toggle packs into the mix. The engine accepts up to four
  active packs. The switch changes the live mix; remaining packs crossfade.
- **Preview** — Tap **Preview** on a pack card to hear a 30-second excerpt. A
  preview uses a separate engine and is disabled while the main mix is playing.
- **Per-pack volume** — Expand a selected pack row to set its level from 0 to
  100%. Changes are click-free and apply while playing.
- **Master volume** — Overall output level.
- **Sleep timer** — Off, 15, 30, 45, 60, or 90 minutes. The final 60 seconds
  fade out.
- **Play/Pause** — Starts or stops the render thread. Pause is immediate in this
  beta; a fade-out pause is planned.
- **Background playback** — Use the notification or lock-screen controls. The
  active mix name is shown as media metadata.
- **Quick Settings tile** — Add Oniris from the Quick Settings tile editor, then
  tap it to play or pause the last mix without opening the app.
- **Headphone disconnect** — Playback pauses when headphones are unplugged.
- **Import a pack** — Tap **Import**, choose an `.onirispack` file, and wait for
  the validation message. The archive is checked and decoded before it is added
  to the library.
- **Procedural only** — Hides recorded packs and limits the library to fully
  synthesized content.
- **Licenses & credits** — Scroll to the bottom of the main screen for the
  per-pack attribution entries.

## Generating a pack to import

From a development machine:

```bash
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  gen-procedural-packs /tmp/oniris-packs --pack enchanted-forest
cargo run --manifest-path core/Cargo.toml -p oniris-cli -- \
  pack /tmp/oniris-packs/enchanted-forest /tmp/enchanted-forest.onirispack
adb push /tmp/enchanted-forest.onirispack /sdcard/Download/
```

Then use **Import** in the app and choose the file from Downloads.

## Suggested test pass

- Play each built-in preset for at least 10 minutes.
- Preview each pack; confirm it starts within a second, fades in, stops at 30
  seconds, and stops when you press Play.
- Lock the screen and verify notification/lock-screen play, pause, stop, and
  metadata.
- Add the Oniris Quick Settings tile and verify it starts/pauses the last mix.
- Change pack selection and per-pack volume during playback; listen for gaps,
  clicks, or level jumps.
- Run a 15-minute sleep timer and confirm the final minute fades to silence.
- Save a mix, restart the app, and apply the saved preset.
- Import an `.onirispack`; confirm it appears in the library and plays.
- Unplug headphones and confirm playback pauses.
- Switch apps, turn the screen off, and confirm playback continues.
- If possible, record battery/underrun notes for an overnight run.

## Known limitations

- **Procedural content only.** Field-recorded launch packs are not included yet.
- **No in-app download browser.** Pack import is manual; previews are included.
- **MediaSessionCompat, not Media3.** Controls and metadata work, but the
  planned Media3 adapter is not implemented.
- **Pause is immediate.** The sleep-timer fade is implemented; a short pause
  fade is still planned.
- **No timer remaining or pack art in the notification.**
- **No on-device battery/thermal/underrun validation yet.** The design targets
  under 2% CPU and under 3%/h drain, but this build has not completed the
  eight-hour soak.
- **Not tested across the OEM matrix.** Samsung/Xiaomi/Oppo background-kill
  behavior still needs real-device checks.
- **Internal signing key.** This APK is signed with a local beta key, not a Play
  Store release key. Future beta updates must be signed with the same key.
- **No crash reporting or telemetry.** Logs are available with
  `adb logcat -s Oniris`.
- **Android only.** No desktop or web client in this release.

## Reporting an issue

Please include:

- Device model and Android version.
- The exact steps and the preset/pack names.
- Whether the screen was on or off and whether headphones/Bluetooth were used.
- A log capture if possible:

  ```bash
  adb logcat -d -s Oniris > oniris-log.txt
  ```

## Build metadata

- Version: `1.0.0-beta2` (`versionCode 3`)
- Package: `dev.oniris`
- ABIs: `arm64-v8a`, `armeabi-v7a`, `x86_64`
- Signed with the internal `Oniris Beta` test key.

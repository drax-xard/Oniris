# Bundled packs

Put development packs here to ship them inside the APK:

```
assets/packs/<pack-id>/manifest.json
assets/packs/<pack-id>/audio/*.wav
```

`PackRepository` copies each pack to the app's internal `files/packs` directory
on first launch and skips packs that are already installed. Packs pushed with
`adb push` into `files/packs` do not need to be bundled.

None of the generated test packs are committed. Create them with `make packs`
and copy the ones you want to ship.

# Localization

**Status:** scaffolding in place. English is the only shipped language; adding
another one is a resource-file drop plus one line of configuration.

## Where strings live

All user-facing text is in `android/app/src/main/res/values/strings.xml`, the
default (unqualified) locale. Kotlin reads it through `stringResource()` in
Compose and `getString()` outside it (services, the playback controller).

Three rules that matter in this file:

- **Positional placeholders only.** `%1$s`, `%2$d` — never bare `%s`/`%d`, so a
  translator can reorder arguments. A literal percent sign is `%%`.
- **Plurals use `<plurals>`.** Counts are phrased with `pluralStringResource()`
  and a quantity; English needs `one`/`other`, but languages can add `few`,
  `many`, etc.
- **Quoted values keep their whitespace.** AAPT strips leading/trailing spaces
  from unquoted values, so separators are written `" · "` with the quotes.

## Add a language

1. Copy `values/strings.xml` to `values-<tag>/strings.xml` (e.g. `values-fr`).
2. Translate the values, leaving placeholders and `translatable="false"` entries
   alone. Partial translations are fine: missing keys fall back to `values/`
   per string, so a half-translated language still runs.
3. Add the quantities your language needs to each `<plurals>`.
4. Add `<locale android:name="fr" />` to `res/xml/locales_config.xml`. Without
   this line the strings still load when the device language matches, but the
   Android 13+ per-app language picker will not offer the language.
5. Check for overflow: German and Russian run long, so look at the Play button,
   the timer chips, and the drawer.

### Testing a locale without changing the device

```bash
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell cmd locale set-app-locales dev.oniris --user 0 --locales fr
adb shell am force-stop dev.oniris && adb shell am start -n dev.oniris/.MainActivity
# reset
adb shell cmd locale set-app-locales dev.oniris --user 0 --locales en
```

The same switch is in the UI at Settings > Apps > Oniris > Language on
Android 13+. A quick check is `adb shell uiautomator dump` and grepping the
`text=` attributes, which catches layouts that silently drop separators.

## What is deliberately not translated

- **Pack content.** Each pack's `manifest.json` carries `name` and `description`
  maps keyed by language; the app renders the pack's own text. Translating the
  app must not touch pack names.
- **Log messages.** `Log.w`/`Log.e` strings stay English so bug reports are
  greppable (`PackRepository`, `AudioRenderThread`).
- **Numbers.** Gain trims format through `String.format(Locale.getDefault(), …)`
  so the decimal separator follows the locale ("-2,8 dB" in German).
- **Identifiers.** Preference keys, wake-lock tags, notification channel IDs, and
  intent actions are code, not copy.

## Adding a string

Add it to `values/strings.xml` first, then reference it. If you find a literal
in Kotlin that reaches the UI, that is a bug: move it into the resource file.
The one exception is text that is data rather than copy (file paths, pack ids).

package dev.oniris.data

import androidx.datastore.core.DataMigration
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.stringPreferencesKey

/**
 * Converts settings written by the pre-media-session build.
 *
 * That build stored per-pack volumes as a `stringSet` of `"id=gain"` entries;
 * this one stores the same key as a JSON string. Reading the old value through
 * the new key throws `ClassCastException: UnmodifiableSet cannot be cast to
 * String`, which crashed the app on launch for anyone upgrading rather than
 * installing fresh.
 *
 * DataStore only offers `String`, `Int`, `Long`, `Float`, `Boolean` and
 * `Set<String>` values, so "the value is a set where a string is expected" is
 * exactly the shape of a legacy read.
 */
internal object LegacySettingsMigration : DataMigration<Preferences> {
    private val PACK_GAINS = stringPreferencesKey(PACK_GAINS_KEY)

    override suspend fun shouldMigrate(currentData: Preferences): Boolean =
        currentData.asMap()[PACK_GAINS] is Set<*>

    override suspend fun migrate(currentData: Preferences): Preferences =
        currentData.toMutablePreferences().apply {
            this[PACK_GAINS] = legacyPackGainsToJson(currentData.asMap()[PACK_GAINS])
        }

    override suspend fun cleanUp() = Unit
}

internal const val PACK_GAINS_KEY = "pack_gains"

/** `{"id=gain", ...}` -> `{"id":gain, ...}`. Unparseable entries are dropped. */
internal fun legacyPackGainsToJson(raw: Any?): String {
    val entries = raw as? Set<*> ?: return "{}"
    val gains = buildMap {
        for (entry in entries) {
            val text = entry as? String ?: continue
            val id = text.substringBefore('=')
            val value = text.substringAfter('=', "").toFloatOrNull() ?: continue
            if (id.isNotBlank()) put(id, value.coerceIn(0f, 1f))
        }
    }
    return jsonObjectOf(gains)
}

/**
 * Minimal object encoder: `org.json` is a stub in JVM unit tests unless a real
 * implementation is added to the test classpath, and this conversion is worth
 * testing without that dependency.
 */
internal fun jsonObjectOf(values: Map<String, Float>): String =
    values.entries.joinToString(separator = ",", prefix = "{", postfix = "}") { (key, value) ->
        "\"${escapeJson(key)}\":$value"
    }

private fun escapeJson(text: String): String = buildString {
    for (character in text) {
        when (character) {
            '"' -> append("\\\"")
            '\\' -> append("\\\\")
            '\n' -> append("\\n")
            '\r' -> append("\\r")
            '\t' -> append("\\t")
            else -> append(character)
        }
    }
}

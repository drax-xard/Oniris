package dev.oniris.data

import androidx.datastore.preferences.core.mutablePreferencesOf
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LegacySettingsMigrationTest {

    private fun legacyPrefs(entries: Set<String>) = mutablePreferencesOf(
        stringSetPreferencesKey(PACK_GAINS_KEY) to entries,
    )

    @Test
    fun migratesALegacyGainSetIntoJson() = runBlocking {
        val legacy = legacyPrefs(setOf("rainy-night=0.5", "pure-noise=1.4", "broken-entry"))
        assertTrue(LegacySettingsMigration.shouldMigrate(legacy))

        val migrated = LegacySettingsMigration.migrate(legacy)
        val raw = migrated[stringPreferencesKey(PACK_GAINS_KEY)]
        assertTrue("expected a string value, got $raw", raw is String)

        // org.json is a stub in JVM unit tests, so compare the encoded string.
        assertEquals("""{"rainy-night":0.5,"pure-noise":1.0}""", raw)
        assertFalse(LegacySettingsMigration.shouldMigrate(migrated))
    }

    @Test
    fun leavesCurrentSettingsAlone() = runBlocking {
        val current = mutablePreferencesOf(
            stringPreferencesKey(PACK_GAINS_KEY) to """{"rainy-night":0.5}""",
        )
        assertFalse(LegacySettingsMigration.shouldMigrate(current))
    }

    @Test
    fun handlesMissingAndEmptyValues() = runBlocking {
        assertFalse(LegacySettingsMigration.shouldMigrate(mutablePreferencesOf()))
        assertEquals("{}", legacyPackGainsToJson(null))
        assertEquals("{}", legacyPackGainsToJson(setOf("", "=0.5", "id=")))
    }
}

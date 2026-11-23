package dev.oniris.data

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

data class Settings(
    val selectedPackIds: Set<String> = emptySet(),
    /** Linear master volume, 0..1. */
    val masterVolume: Float = 0.8f,
    /** Sleep timer in minutes; 0 means off. */
    val timerMinutes: Int = 0,
    /** Linear per-pack trim keyed by pack id; a missing entry means unity. */
    val packGains: Map<String, Float> = emptyMap(),
)

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore("oniris_settings")

class SettingsStore(context: Context) {
    private val dataStore = context.applicationContext.dataStore

    val settings: Flow<Settings> = dataStore.data.map { preferences ->
        Settings(
            selectedPackIds = preferences[SELECTED_PACKS].orEmpty(),
            masterVolume = preferences[MASTER_VOLUME] ?: 0.8f,
            timerMinutes = preferences[TIMER_MINUTES] ?: 0,
            packGains = parseGains(preferences[PACK_GAINS]),
        )
    }

    suspend fun setSelectedPacks(ids: Set<String>) {
        dataStore.edit { it[SELECTED_PACKS] = ids }
    }

    suspend fun setMasterVolume(volume: Float) {
        dataStore.edit { it[MASTER_VOLUME] = volume.coerceIn(0f, 1f) }
    }

    suspend fun setTimerMinutes(minutes: Int) {
        dataStore.edit { it[TIMER_MINUTES] = minutes.coerceAtLeast(0) }
    }

    /** Store a linear trim for one pack; 1.0 is unity gain. */
    suspend fun setPackGain(packId: String, gain: Float) {
        val clamped = gain.coerceIn(0f, MAX_PACK_GAIN)
        dataStore.edit { preferences ->
            val entries = preferences[PACK_GAINS].orEmpty()
                .filterNot { it.substringBefore('=') == packId }
                .toMutableSet()
            entries.add("$packId=$clamped")
            preferences[PACK_GAINS] = entries
        }
    }

    private fun parseGains(entries: Set<String>?): Map<String, Float> =
        entries.orEmpty().mapNotNull { entry ->
            val id = entry.substringBefore('=', "")
            val value = entry.substringAfter('=', "").toFloatOrNull()
            if (id.isBlank() || value == null) null else id to value.coerceIn(0f, MAX_PACK_GAIN)
        }.toMap()

    private companion object {
        val SELECTED_PACKS = stringSetPreferencesKey("selected_packs")
        val MASTER_VOLUME = floatPreferencesKey("master_volume")
        val TIMER_MINUTES = intPreferencesKey("timer_minutes")
        val PACK_GAINS = stringSetPreferencesKey("pack_gains")

        /** Matches the engine's per-pack gain clamp (0..2 linear) with headroom to spare. */
        const val MAX_PACK_GAIN = 1.5f
    }
}

package dev.oniris.data

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import org.json.JSONObject

data class Settings(
    val selectedPackIds: Set<String> = emptySet(),
    /** Linear master volume, 0..1. */
    val masterVolume: Float = 0.8f,
    /** Per-pack linear volume, 0..1. Missing packs default to 1.0. */
    val packGains: Map<String, Float> = emptyMap(),
    /** User-saved named mixes. Built-in presets live in the UI layer. */
    val presets: List<MixPreset> = emptyList(),
    /** Sleep timer in minutes; 0 means off. */
    val timerMinutes: Int = 0,
    /** Hide recorded packs and use only fully synthesized packs. */
    val proceduralOnly: Boolean = false,
)

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore("oniris_settings")

class SettingsStore(context: Context) {
    private val dataStore = context.applicationContext.dataStore

    val settings: Flow<Settings> = dataStore.data.map { preferences ->
        Settings(
            selectedPackIds = preferences[SELECTED_PACKS].orEmpty(),
            masterVolume = preferences[MASTER_VOLUME] ?: 0.8f,
            packGains = decodePackGains(preferences[PACK_GAINS]),
            presets = decodePresets(preferences[PRESETS]),
            timerMinutes = preferences[TIMER_MINUTES] ?: 0,
            proceduralOnly = preferences[PROCEDURAL_ONLY] ?: false,
        )
    }

    suspend fun setSelectedPacks(ids: Set<String>) {
        dataStore.edit { it[SELECTED_PACKS] = ids }
    }

    suspend fun setMasterVolume(volume: Float) {
        dataStore.edit { it[MASTER_VOLUME] = volume.coerceIn(0f, 1f) }
    }

    suspend fun setPackGain(id: String, volume: Float) {
        dataStore.edit { preferences ->
            val gains = decodePackGains(preferences[PACK_GAINS]).toMutableMap()
            gains[id] = volume.coerceIn(0f, 1f)
            preferences[PACK_GAINS] = JSONObject(gains).toString()
        }
    }

    suspend fun setPackGains(gains: Map<String, Float>) {
        dataStore.edit { preferences ->
            preferences[PACK_GAINS] = JSONObject(
                gains.mapValues { (_, value) -> value.coerceIn(0f, 1f) },
            ).toString()
        }
    }

    suspend fun savePreset(preset: MixPreset) {
        dataStore.edit { preferences ->
            val presets = decodePresets(preferences[PRESETS])
                .filterNot { it.id == preset.id }
                .plus(preset)
            preferences[PRESETS] = encodePresets(presets)
        }
    }

    suspend fun deletePreset(id: String) {
        dataStore.edit { preferences ->
            val presets = decodePresets(preferences[PRESETS]).filterNot { it.id == id }
            preferences[PRESETS] = encodePresets(presets)
        }
    }

    /** Apply all parts of a preset in one DataStore transaction. */
    suspend fun applyPreset(preset: MixPreset) {
        dataStore.edit { preferences ->
            preferences[SELECTED_PACKS] = preset.packIds.toSet()
            preferences[PACK_GAINS] = JSONObject(
                preset.packGains.mapValues { (_, value) -> value.coerceIn(0f, 1f) },
            ).toString()
            preferences[MASTER_VOLUME] = preset.masterVolume.coerceIn(0f, 1f)
            preferences[TIMER_MINUTES] = preset.timerMinutes.coerceAtLeast(0)
        }
    }

    suspend fun setTimerMinutes(minutes: Int) {
        dataStore.edit { it[TIMER_MINUTES] = minutes.coerceAtLeast(0) }
    }

    suspend fun setProceduralOnly(enabled: Boolean) {
        dataStore.edit { it[PROCEDURAL_ONLY] = enabled }
    }

    private companion object {
        val SELECTED_PACKS = stringSetPreferencesKey("selected_packs")
        val MASTER_VOLUME = floatPreferencesKey("master_volume")
        val PACK_GAINS = stringPreferencesKey("pack_gains")
        val PRESETS = stringPreferencesKey("presets")
        val TIMER_MINUTES = intPreferencesKey("timer_minutes")
        val PROCEDURAL_ONLY = booleanPreferencesKey("procedural_only")
    }
}

private fun decodePackGains(json: String?): Map<String, Float> {
    if (json.isNullOrBlank()) return emptyMap()
    return runCatching {
        val values = JSONObject(json)
        buildMap {
            values.keys().forEach { id ->
                put(id, values.optDouble(id, 1.0).toFloat().coerceIn(0f, 1f))
            }
        }
    }.getOrDefault(emptyMap())
}

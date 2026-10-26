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
)

private val Context.dataStore: DataStore<Preferences> by preferencesDataStore("oniris_settings")

class SettingsStore(context: Context) {
    private val dataStore = context.applicationContext.dataStore

    val settings: Flow<Settings> = dataStore.data.map { preferences ->
        Settings(
            selectedPackIds = preferences[SELECTED_PACKS].orEmpty(),
            masterVolume = preferences[MASTER_VOLUME] ?: 0.8f,
            timerMinutes = preferences[TIMER_MINUTES] ?: 0,
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

    private companion object {
        val SELECTED_PACKS = stringSetPreferencesKey("selected_packs")
        val MASTER_VOLUME = floatPreferencesKey("master_volume")
        val TIMER_MINUTES = intPreferencesKey("timer_minutes")
    }
}

package dev.oniris

import android.app.Application
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PlaybackController

class OnirisApplication : Application() {
    lateinit var packs: PackRepository
        private set

    lateinit var settings: SettingsStore
        private set

    override fun onCreate() {
        super.onCreate()
        packs = PackRepository(this)
        settings = SettingsStore(this)
        PlaybackController.init(this)
    }
}

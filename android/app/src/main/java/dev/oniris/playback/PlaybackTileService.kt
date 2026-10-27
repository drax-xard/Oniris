package dev.oniris.playback

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import dev.oniris.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/** Quick Settings tile that starts/stops the last mix like the main Play button. */
class PlaybackTileService : TileService() {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var stateJob: Job? = null

    override fun onStartListening() {
        super.onStartListening()
        stateJob?.cancel()
        stateJob = scope.launch {
            PlaybackController.isPlaying.collect { updateTile() }
        }
    }

    override fun onStopListening() {
        stateJob?.cancel()
        stateJob = null
        super.onStopListening()
    }

    override fun onClick() {
        super.onClick()
        if (PlaybackController.isPlaying.value) {
            PlaybackService.pause(this)
        } else {
            PlaybackService.play(this)
        }
        updateTile()
    }

    override fun onTileAdded() {
        super.onTileAdded()
        updateTile()
    }

    override fun onDestroy() {
        stateJob?.cancel()
        scope.cancel()
        super.onDestroy()
    }

    private fun updateTile() {
        val tile = qsTile ?: return
        tile.state = if (PlaybackController.isPlaying.value) {
            Tile.STATE_ACTIVE
        } else {
            Tile.STATE_INACTIVE
        }
        tile.label = getString(R.string.quick_tile_label)
        tile.contentDescription = getString(
            if (PlaybackController.isPlaying.value) {
                R.string.quick_tile_playing
            } else {
                R.string.quick_tile_idle
            },
        )
        tile.updateTile()
    }
}

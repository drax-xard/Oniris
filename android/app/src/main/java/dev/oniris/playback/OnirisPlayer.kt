package dev.oniris.playback

import android.content.Context
import android.os.Looper
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import dev.oniris.OnirisApplication
import dev.oniris.R
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Media3 player façade over the Rust engine and [PlaybackController].
 *
 * Media3 owns the MediaSession, notification, lock-screen controls, media
 * buttons, and foreground lifecycle. This player only translates its commands
 * into controller calls and exposes the controller state back to Media3.
 */
@androidx.annotation.OptIn(markerClass = [UnstableApi::class])
class OnirisPlayer(
    private val context: Context,
    private val scope: CoroutineScope,
) : SimpleBasePlayer(Looper.getMainLooper()) {
    private var playbackState = Player.STATE_IDLE
    private var playWhenReady = false
    private var title = ""
    private var playerError: PlaybackException? = null
    private var loadJob: Job? = null
    private var released = false

    private val stateJob = scope.launch {
        combine(
            PlaybackController.isPlaying,
            PlaybackController.mixTitle,
        ) { playing, mixTitle -> playing to mixTitle }
            .collect { (playing, mixTitle) ->
                title = mixTitle
                if (playing) {
                    playbackState = Player.STATE_READY
                    playerError = null
                } else if (playbackState == Player.STATE_BUFFERING) {
                    // A play request is still loading packs.
                } else if (playWhenReady && playbackState == Player.STATE_READY) {
                    // The sleep timer finished or playback stopped itself.
                    playWhenReady = false
                }
                invalidateState()
            }
    }

    override fun getState(): State {
        val metadata = MediaMetadata.Builder()
            .setTitle(title.ifBlank { context.getString(R.string.app_name) })
            .setArtist(context.getString(R.string.app_name))
            .setAlbumTitle("Sleep soundscape")
            .build()
        val item = MediaItem.Builder()
            .setMediaId(MEDIA_ID)
            .setMediaMetadata(metadata)
            .build()
        val itemData = MediaItemData.Builder(MEDIA_ID)
            .setMediaItem(item)
            .setMediaMetadata(metadata)
            .setDurationUs(C.TIME_UNSET)
            .build()

        return State.Builder()
            .setAvailableCommands(COMMANDS)
            .setPlayWhenReady(playWhenReady, Player.PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
            .setPlaybackState(playbackState)
            .setPlaylist(listOf(itemData))
            .setCurrentMediaItemIndex(0)
            .setContentPositionMs(C.TIME_UNSET)
            .setAudioAttributes(AUDIO_ATTRIBUTES)
            .setPlayerError(playerError)
            .setIsLoading(playbackState == Player.STATE_BUFFERING)
            .build()
    }

    override fun handlePrepare(): ListenableFuture<*> {
        if (!released && playbackState == Player.STATE_IDLE) {
            playbackState = Player.STATE_READY
            playerError = null
            invalidateState()
        }
        return Futures.immediateVoidFuture()
    }

    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> {
        if (released) return Futures.immediateVoidFuture()
        if (playWhenReady) {
            requestPlay()
        } else {
            requestPause()
        }
        return Futures.immediateVoidFuture()
    }

    override fun handleStop(): ListenableFuture<*> {
        playWhenReady = false
        playerError = null
        playbackState = Player.STATE_IDLE
        PlaybackController.stop()
        invalidateState()
        return Futures.immediateVoidFuture()
    }

    override fun handleRelease(): ListenableFuture<*> {
        released = true
        stateJob.cancel()
        PlaybackController.stop()
        return Futures.immediateVoidFuture()
    }

    private fun requestPlay() {
        playWhenReady = true
        playerError = null
        if (PlaybackController.isPlaying.value) {
            playbackState = Player.STATE_READY
            invalidateState()
            return
        }
        if (loadJob?.isActive == true) {
            invalidateState()
            return
        }

        playbackState = Player.STATE_BUFFERING
        invalidateState()
        loadJob = scope.launch {
            val started = withContext(Dispatchers.IO) {
                runCatching {
                    restoreSelectionIfNeeded()
                    PlaybackController.play()
                }.getOrDefault(false)
            }
            if (released) return@launch

            if (!this@OnirisPlayer.playWhenReady) {
                if (playbackState == Player.STATE_IDLE) {
                    PlaybackController.stop()
                } else if (started) {
                    PlaybackController.pause()
                }
                playbackState = if (playbackState == Player.STATE_IDLE) {
                    Player.STATE_IDLE
                } else {
                    Player.STATE_READY
                }
                invalidateState()
                return@launch
            }

            if (started) {
                playbackState = Player.STATE_READY
            } else {
                playbackState = Player.STATE_IDLE
                this@OnirisPlayer.playWhenReady = false
                playerError = PlaybackException(
                    "Could not start playback",
                    null,
                    PlaybackException.ERROR_CODE_UNSPECIFIED,
                )
            }
            invalidateState()
        }
    }

    private fun requestPause() {
        playWhenReady = false
        PlaybackController.pause()
        if (playbackState == Player.STATE_BUFFERING) {
            playbackState = Player.STATE_READY
        }
        invalidateState()
    }

    /**
     * Restore the last mix when a media button, tile, or notification starts
     * playback after the process was recreated.
     */
    private suspend fun restoreSelectionIfNeeded() {
        if (PlaybackController.hasSelection) return
        val app = context.applicationContext as OnirisApplication
        val settings = app.settings.settings.first()
        val selection = app.packs.listPacks()
            .filter {
                it.id in settings.selectedPackIds &&
                    (!settings.proceduralOnly || it.procedural)
            }
            .map { PackSelection(it.id, it.name, it.directory) }

        PlaybackController.setPacks(selection)
        PlaybackController.setMasterGain(settings.masterVolume)
        PlaybackController.setPackGains(settings.packGains)
        PlaybackController.setSleepTimer(settings.timerMinutes)
    }

    private companion object {
        const val MEDIA_ID = "oniris"

        val COMMANDS: Player.Commands = Player.Commands.Builder()
            .addAll(
                Player.COMMAND_PLAY_PAUSE,
                Player.COMMAND_STOP,
                Player.COMMAND_PREPARE,
                Player.COMMAND_RELEASE,
                Player.COMMAND_GET_CURRENT_MEDIA_ITEM,
                Player.COMMAND_GET_TIMELINE,
                Player.COMMAND_GET_METADATA,
                Player.COMMAND_GET_AUDIO_ATTRIBUTES,
            )
            .build()

        val AUDIO_ATTRIBUTES: AudioAttributes = AudioAttributes.Builder()
            .setUsage(C.USAGE_MEDIA)
            .setContentType(C.AUDIO_CONTENT_TYPE_MUSIC)
            .build()
    }
}

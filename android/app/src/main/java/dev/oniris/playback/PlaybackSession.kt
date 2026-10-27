package dev.oniris.playback

import android.content.ComponentName
import android.content.Context
import android.util.Log
import androidx.core.content.ContextCompat
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken

/**
 * Small helper for UI and the Quick Settings tile to send play/pause/stop to
 * the Media3 session without holding a long-lived controller connection.
 */
object PlaybackSession {
    fun play(context: Context) {
        withController(context) { it.play() }
    }

    fun pause(context: Context) {
        withController(context) { it.pause() }
    }

    fun stop(context: Context) {
        withController(context) { it.stop() }
    }

    private fun withController(context: Context, action: (MediaController) -> Unit) {
        val appContext = context.applicationContext
        val token = SessionToken(
            appContext,
            ComponentName(appContext, PlaybackService::class.java),
        )
        val future = MediaController.Builder(appContext, token).buildAsync()
        future.addListener(
            {
                runCatching {
                    val controller = future.get()
                    action(controller)
                    controller.release()
                }.onFailure { Log.w(TAG, "media controller command failed", it) }
            },
            ContextCompat.getMainExecutor(appContext),
        )
    }

    private const val TAG = "Oniris"
}

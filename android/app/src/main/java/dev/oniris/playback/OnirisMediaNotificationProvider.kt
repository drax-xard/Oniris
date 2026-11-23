package dev.oniris.playback

import android.content.Context
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import com.google.common.collect.ImmutableList
import dev.oniris.R

/**
 * Media3's default notification has play/pause but no stop. Oniris needs an
 * explicit stop action so a finished session can be dismissed from the lock
 * screen.
 */
@androidx.annotation.OptIn(markerClass = [UnstableApi::class])
class OnirisMediaNotificationProvider(context: Context) : DefaultMediaNotificationProvider(
    context,
    { NOTIFICATION_ID },
    CHANNEL_ID,
    R.string.notification_channel,
) {
    private val appContext = context.applicationContext

    init {
        setSmallIcon(R.drawable.ic_notification)
    }

    override fun getMediaButtons(
        session: MediaSession,
        playerCommands: Player.Commands,
        mediaButtonPreferences: ImmutableList<CommandButton>,
        showPauseButton: Boolean,
    ): ImmutableList<CommandButton> {
        val buttons = super.getMediaButtons(
            session,
            playerCommands,
            mediaButtonPreferences,
            showPauseButton,
        )
        if (!playerCommands.contains(Player.COMMAND_STOP)) return buttons

        return ImmutableList.builder<CommandButton>()
            .addAll(buttons)
            .add(
                CommandButton.Builder(CommandButton.ICON_STOP)
                    .setPlayerCommand(Player.COMMAND_STOP)
                    .setDisplayName(appContext.getString(R.string.notification_stop))
                    .build(),
            )
            .build()
    }

    private companion object {
        const val CHANNEL_ID = "oniris_playback"
        const val NOTIFICATION_ID = 41
    }
}

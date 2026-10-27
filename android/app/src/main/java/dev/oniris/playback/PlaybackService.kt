package dev.oniris.playback

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.ServiceInfo
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Build
import android.os.IBinder
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import androidx.media.app.NotificationCompat.MediaStyle
import dev.oniris.MainActivity
import dev.oniris.OnirisApplication
import dev.oniris.R
import dev.oniris.playback.PackSelection
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Foreground media service. It owns the notification and the MediaSession;
 * playback itself lives in PlaybackController.
 */
class PlaybackService : Service() {
    private lateinit var mediaSession: MediaSessionCompat
    private lateinit var audioManager: AudioManager
    private var focusRequest: AudioFocusRequest? = null
    private var resumeOnFocusGain = false
    private var ducked = false
    private var gainBeforeDuck: Float? = null
    private var foregroundStarted = false

    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
    private var stateJob: Job? = null
    private var playbackJob: Job? = null

    private val noisyReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) {
                pausePlayback()
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        createChannel()
        audioManager = getSystemService(Context.AUDIO_SERVICE) as AudioManager
        mediaSession = MediaSessionCompat(this, "Oniris").apply {
            setCallback(
                object : MediaSessionCompat.Callback() {
                    override fun onPlay() = startPlayback()
                    override fun onPause() = pausePlayback()
                    override fun onStop() = stopPlayback()
                },
            )
            setMetadata(
                MediaMetadataCompat.Builder()
                    .putString(MediaMetadataCompat.METADATA_KEY_TITLE, "Oniris")
                    .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, "Sleep soundscape")
                    .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, -1L)
                    .build(),
            )
            setPlaybackState(playbackState(PlaybackStateCompat.STATE_NONE))
            setActive(true)
        }

        ContextCompat.registerReceiver(
            this,
            noisyReceiver,
            IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY),
            ContextCompat.RECEIVER_NOT_EXPORTED,
        )

        stateJob = serviceScope.launch {
            combine(
                PlaybackController.isPlaying,
                PlaybackController.mixTitle,
            ) { playing, title -> playing to title }
                .collect { (playing, _) ->
                    if (!foregroundStarted) return@collect
                    updateMetadata()
                    when {
                        playing -> notifyPlaying()
                        playbackJob?.isActive == true -> {
                            // Packs are still loading; keep the buffering
                            // notification instead of flashing "Paused".
                        }

                        else -> {
                            updateMediaState(PlaybackStateCompat.STATE_PAUSED)
                            notifyPaused()
                        }
                    }
                }
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_PLAY -> startPlayback()
            ACTION_PAUSE -> pausePlayback()
            ACTION_STOP -> stopPlayback()
        }
        return START_NOT_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onTaskRemoved(rootIntent: Intent?) {
        if (!PlaybackController.isPlaying.value) {
            stopPlayback()
        }
    }

    override fun onDestroy() {
        stateJob?.cancel()
        serviceScope.cancel()
        runCatching { unregisterReceiver(noisyReceiver) }
        abandonFocus()
        PlaybackController.stop()
        mediaSession.release()
        super.onDestroy()
    }

    private fun startPlayback() {
        PackPreviewController.stop()
        if (playbackJob?.isActive == true) return
        if (!requestFocus()) {
            stopSelfSafely()
            return
        }

        // startForegroundService() requires startForeground() promptly, so show
        // the notification before the (potentially slow) pack load.
        mediaSession.setActive(true)
        startForegroundCompat(buildNotification(playing = true))
        updateMediaState(PlaybackStateCompat.STATE_BUFFERING)

        playbackJob = serviceScope.launch {
            val started = withContext(Dispatchers.IO) {
                restoreSelectionIfNeeded()
                PlaybackController.play()
            }
            if (started && PlaybackController.isPlaying.value) {
                updateMetadata()
                updateMediaState(PlaybackStateCompat.STATE_PLAYING)
                notifyPlaying()
            } else if (started) {
                // Paused by the user while packs were loading.
                updateMetadata()
                updateMediaState(PlaybackStateCompat.STATE_PAUSED)
                notifyPaused()
            } else {
                abandonFocus()
                stopSelfSafely()
            }
        }
    }

    /**
     * Rebuild the mix from settings when the process was recreated by a
     * notification action or the system.
     */
    private suspend fun restoreSelectionIfNeeded() {
        if (PlaybackController.hasSelection) return

        val app = application as OnirisApplication
        val settings = app.settings.settings.first()
        val selection = app.packs.listPacks()
            .filter { it.id in settings.selectedPackIds }
            .map { PackSelection(it.id, it.name, it.directory) }

        PlaybackController.setPacks(selection)
        PlaybackController.setMasterGain(settings.masterVolume)
        PlaybackController.setPackGains(settings.packGains)
        PlaybackController.setSleepTimer(settings.timerMinutes)
    }

    private fun pausePlayback() {
        PlaybackController.pause()
        updateMediaState(PlaybackStateCompat.STATE_PAUSED)
        if (foregroundStarted) {
            notifyPaused()
        }
    }

    private fun stopPlayback() {
        PlaybackController.stop()
        updateMediaState(PlaybackStateCompat.STATE_STOPPED)
        mediaSession.setActive(false)
        abandonFocus()
        if (foregroundStarted) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            foregroundStarted = false
        }
        stopSelf()
    }

    private fun stopSelfSafely() {
        abandonFocus()
        if (foregroundStarted) {
            stopForeground(STOP_FOREGROUND_REMOVE)
            foregroundStarted = false
        }
        stopSelf()
    }

    private fun startForegroundCompat(notification: Notification) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK,
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        foregroundStarted = true
    }

    private fun notifyPlaying() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, buildNotification(playing = true))
    }

    private fun notifyPaused() {
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, buildNotification(playing = false))
    }

    private fun buildNotification(playing: Boolean): Notification {
        val contentIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val toggleIntent = if (playing) {
            PendingIntent.getService(
                this,
                1,
                Intent(this, PlaybackService::class.java).setAction(ACTION_PAUSE),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        } else {
            PendingIntent.getForegroundService(
                this,
                1,
                Intent(this, PlaybackService::class.java).setAction(ACTION_PLAY),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        }
        val stopIntent = PendingIntent.getService(
            this,
            2,
            Intent(this, PlaybackService::class.java).setAction(ACTION_STOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(
                if (playing) {
                    PlaybackController.mixTitle.value.ifBlank {
                        getString(R.string.notification_playing)
                    }
                } else {
                    getString(R.string.notification_paused)
                },
            )
            .setContentIntent(contentIntent)
            .setOngoing(playing)
            .setSilent(true)
            .setCategory(NotificationCompat.CATEGORY_TRANSPORT)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .addAction(
                R.drawable.ic_notification,
                getString(if (playing) R.string.pause else R.string.play),
                toggleIntent,
            )
            .addAction(
                R.drawable.ic_notification,
                getString(R.string.stop),
                stopIntent,
            )
            .setStyle(
                MediaStyle()
                    .setMediaSession(mediaSession.sessionToken)
                    .setShowActionsInCompactView(0, 1),
            )
            .build()
    }

    private fun updateMediaState(state: Int) {
        mediaSession.setPlaybackState(playbackState(state))
    }

    private fun updateMetadata() {
        val title = PlaybackController.mixTitle.value.ifBlank { getString(R.string.app_name) }
        mediaSession.setMetadata(
            MediaMetadataCompat.Builder()
                .putString(MediaMetadataCompat.METADATA_KEY_TITLE, title)
                .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, getString(R.string.app_name))
                .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, "Sleep soundscape")
                .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, -1L)
                .build(),
        )
    }

    private fun playbackState(state: Int): PlaybackStateCompat {
        return PlaybackStateCompat.Builder()
            .setActions(
                PlaybackStateCompat.ACTION_PLAY or
                    PlaybackStateCompat.ACTION_PAUSE or
                    PlaybackStateCompat.ACTION_PLAY_PAUSE or
                    PlaybackStateCompat.ACTION_STOP,
            )
            .setState(
                state,
                PlaybackStateCompat.PLAYBACK_POSITION_UNKNOWN,
                if (state == PlaybackStateCompat.STATE_PLAYING) 1f else 0f,
            )
            .build()
    }

    private fun requestFocus(): Boolean {
        val attributes = AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
            .build()
        val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
            .setAudioAttributes(attributes)
            .setOnAudioFocusChangeListener { change -> onFocusChange(change) }
            .build()
        focusRequest = request
        return audioManager.requestAudioFocus(request) ==
            AudioManager.AUDIOFOCUS_REQUEST_GRANTED
    }

    private fun abandonFocus() {
        focusRequest?.let { audioManager.abandonAudioFocusRequest(it) }
        focusRequest = null
    }

    private fun onFocusChange(change: Int) {
        when (change) {
            AudioManager.AUDIOFOCUS_LOSS -> {
                resumeOnFocusGain = false
                restoreDuckedGain()
                pausePlayback()
            }

            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> {
                resumeOnFocusGain = PlaybackController.isPlaying.value
                restoreDuckedGain()
                pausePlayback()
            }

            AudioManager.AUDIOFOCUS_LOSS_TRANSIENT_CAN_DUCK -> {
                if (!ducked) {
                    ducked = true
                    val current = PlaybackController.masterGain()
                    gainBeforeDuck = current
                    PlaybackController.setMasterGain(current * 0.2f)
                }
            }

            AudioManager.AUDIOFOCUS_GAIN -> {
                if (ducked) {
                    ducked = false
                    restoreDuckedGain()
                }
                if (resumeOnFocusGain) {
                    resumeOnFocusGain = false
                    startPlayback()
                }
            }
        }
    }

    private fun restoreDuckedGain() {
        ducked = false
        val previous = gainBeforeDuck ?: return
        gainBeforeDuck = null
        PlaybackController.setMasterGain(previous)
    }

    private fun createChannel() {
        val channel = NotificationChannel(
            CHANNEL_ID,
            getString(R.string.notification_channel),
            NotificationManager.IMPORTANCE_LOW,
        ).apply {
            setSound(null, null)
            enableVibration(false)
            setShowBadge(false)
        }
        getSystemService(NotificationManager::class.java)
            .createNotificationChannel(channel)
    }

    companion object {
        const val ACTION_PLAY = "dev.oniris.action.PLAY"
        const val ACTION_PAUSE = "dev.oniris.action.PAUSE"
        const val ACTION_STOP = "dev.oniris.action.STOP"

        private const val CHANNEL_ID = "oniris_playback"
        private const val NOTIFICATION_ID = 41

        fun play(context: Context) {
            ContextCompat.startForegroundService(
                context,
                Intent(context, PlaybackService::class.java).setAction(ACTION_PLAY),
            )
        }

        fun pause(context: Context) {
            context.startService(
                Intent(context, PlaybackService::class.java).setAction(ACTION_PAUSE),
            )
        }

        fun stop(context: Context) {
            context.startService(
                Intent(context, PlaybackService::class.java).setAction(ACTION_STOP),
            )
        }
    }
}

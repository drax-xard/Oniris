package dev.oniris.playback

import android.annotation.SuppressLint
import android.content.Context
import android.os.PowerManager
import dev.oniris.audio.AudioRenderThread
import dev.oniris.audio.NativeEngine
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

data class PackSelection(val id: String, val dir: File)

/**
 * Process-wide playback control. The UI and the media service share this
 * object; only the service owns the foreground notification.
 *
 * Control calls are serialized by [lock]. Playback loading is deliberately
 * synchronous: the service calls [play] from a background dispatcher.
 */
object PlaybackController {
    private val lock = Any()

    private var appContext: Context? = null
    private var engine: NativeEngine? = null
    private var renderThread: AudioRenderThread? = null
    private var wakeLock: PowerManager.WakeLock? = null

    private var selections: List<PackSelection> = emptyList()
    private var loadedSelections: List<PackSelection> = emptyList()
    private var masterGain: Float = 0.8f
    private var timerMinutes: Int = 0

    private val _isPlaying = MutableStateFlow(false)
    val isPlaying: StateFlow<Boolean> = _isPlaying.asStateFlow()

    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    val available: Boolean
        get() = engine?.available == true

    fun init(context: Context) {
        synchronized(lock) {
            if (engine != null) return
            appContext = context.applicationContext
            engine = NativeEngine()
        }
    }

    fun setPacks(value: List<PackSelection>) {
        synchronized(lock) {
            selections = value
        }
    }

    val hasSelection: Boolean
        get() = synchronized(lock) { selections.isNotEmpty() }

    fun play(): Boolean = synchronized(lock) {
        val context = appContext ?: return fail("Playback is not initialized")
        var current = engine ?: return fail("Native engine is not initialized")
        if (!current.available) return fail("Native engine is not built for this device")
        if (selections.isEmpty()) return fail("Add at least one sound pack first")
        if (_isPlaying.value) return true

        if (loadedSelections != selections) {
            renderThread?.stop()
            renderThread = null
            current.stop()
            current.close()
            current = NativeEngine()
            engine = current
            if (!current.available) return fail("Native engine is not built for this device")

            for (selection in selections) {
                if (!current.loadPack(selection.dir.absolutePath)) {
                    return fail("Could not load ${selection.id}")
                }
            }

            val json = JSONObject().apply {
                put("masterGain", 1.0)
                put(
                    "packs",
                    JSONArray().apply {
                        selections.forEach { selection ->
                            put(
                                JSONObject()
                                    .put("id", selection.id)
                                    .put("gainDb", 0.0)
                                    .put("muted", false),
                            )
                        }
                    },
                )
            }.toString()

            if (!current.setMix(json)) {
                current.stop()
                return fail("Could not apply the mix")
            }
            loadedSelections = selections
            masterGain = 0.8f
        }

        current.play()
        current.setMasterGain(masterGain)
        applyTimer(current)
        acquireWakeLock(context)

        val thread = AudioRenderThread(current) {
            // Runs on the audio thread; do not join here.
            _isPlaying.value = false
            releaseWakeLock()
        }
        renderThread = thread
        thread.start()
        _isPlaying.value = true
        _message.value = null
        true
    }

    /** Stop immediately; the UI restarts playback from silence. */
    fun pause() {
        synchronized(lock) {
            renderThread?.stop()
            renderThread = null
            engine?.stop()
            releaseWakeLock()
            _isPlaying.value = false
        }
    }

    fun stop() {
        synchronized(lock) {
            renderThread?.stop()
            renderThread = null
            engine?.stop()
            releaseWakeLock()
            _isPlaying.value = false
        }
    }

    fun setMasterGain(gain: Float) {
        synchronized(lock) {
            masterGain = gain.coerceIn(0f, 1f)
            engine?.setMasterGain(masterGain)
        }
    }

    fun setPackGainDb(id: String, gainDb: Float) {
        synchronized(lock) {
            engine?.setPackGainDb(id, gainDb)
        }
    }

    fun setSleepTimer(minutes: Int) {
        synchronized(lock) {
            timerMinutes = minutes.coerceAtLeast(0)
            engine?.let(::applyTimer)
        }
    }

    fun clearSleepTimer() {
        synchronized(lock) {
            timerMinutes = 0
            engine?.clearSleepTimer()
        }
    }

    fun decodedBytes(): Long = synchronized(lock) {
        engine?.decodedBytes() ?: 0L
    }

    fun clearMessage() {
        _message.value = null
    }

    private fun applyTimer(engine: NativeEngine) {
        if (timerMinutes > 0) {
            engine.setSleepTimer(timerMinutes * 60_000L, 60_000L)
        } else {
            engine.clearSleepTimer()
        }
    }

    @SuppressLint("WakelockTimeout")
    private fun acquireWakeLock(context: Context) {
        if (wakeLock != null) return
        val powerManager = context.getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "Oniris:Playback",
        ).apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.let { lock ->
            if (lock.isHeld) lock.release()
        }
        wakeLock = null
    }

    private fun fail(message: String): Boolean {
        _message.value = message
        return false
    }
}

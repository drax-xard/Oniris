package dev.oniris.playback

import android.annotation.SuppressLint
import android.content.Context
import android.os.PowerManager
import androidx.annotation.StringRes
import dev.oniris.R
import dev.oniris.audio.AudioRenderThread
import dev.oniris.audio.NativeEngine
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

data class PackSelection(val id: String, val name: String, val dir: File)

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
    private val loadedPackIds = mutableSetOf<String>()
    private var masterGain: Float = 0.8f
    private var packGains: Map<String, Float> = emptyMap()
    private var timerMinutes: Int = 0

    @Volatile
    private var pausing = false

    private val _isPlaying = MutableStateFlow(false)
    val isPlaying: StateFlow<Boolean> = _isPlaying.asStateFlow()

    private val _mixPackNames = MutableStateFlow<List<String>>(emptyList())
    val mixPackNames: StateFlow<List<String>> = _mixPackNames.asStateFlow()

    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    val available: Boolean
        get() = engine?.available == true

    fun init(context: Context) {
        synchronized(lock) {
            if (engine != null) return
            appContext = context.applicationContext
            engine = NativeEngine()
            loadedPackIds.clear()
        }
    }

    fun setPacks(value: List<PackSelection>) {
        synchronized(lock) {
            selections = value
            _mixPackNames.value = value.map { it.name }
            if (_isPlaying.value) {
                engine?.let(::applyMix)
            }
        }
    }

    val hasSelection: Boolean
        get() = synchronized(lock) { selections.isNotEmpty() }

    fun play(): Boolean = synchronized(lock) {
        val context = appContext ?: return fail(R.string.playback_not_initialized)
        val current = engine ?: return fail(R.string.playback_engine_not_initialized)
        if (!current.available) return fail(R.string.playback_engine_unavailable)
        if (selections.isEmpty()) return fail(R.string.playback_need_pack)
        if (pausing && renderThread?.isRunning == true) {
            // Reverse a pause fade that is still running.
            pausing = false
            current.play()
            _isPlaying.value = true
            _message.value = null
            return true
        }
        pausing = false
        if (_isPlaying.value) return true
        if (!stopRenderThread()) {
            return fail(R.string.playback_stopping)
        }

        if (!applyMix(current)) return false

        current.play()
        current.setMasterGain(masterGain)
        applyTimer(current)
        acquireWakeLock(context)

        val thread = AudioRenderThread(current) {
            // Runs on the audio thread; do not join here.
            pausing = false
            _isPlaying.value = false
            releaseWakeLock()
        }
        renderThread = thread
        _isPlaying.value = true
        thread.start()
        _message.value = null
        true
    }

    /**
     * Load any newly selected packs and crossfade to the current selection.
     * The engine keeps removed packs decoded so a later re-add is instant.
     */
    private fun applyMix(current: NativeEngine): Boolean {
        for (selection in selections) {
            if (selection.id in loadedPackIds) continue
            if (!current.loadPack(selection.dir.absolutePath)) {
                return fail(R.string.error_could_not_load_pack, selection.name)
            }
            loadedPackIds += selection.id
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
                                .put("gainDb", volumeToGainDb(packGains[selection.id] ?: 1f))
                                .put("muted", false),
                        )
                    }
                },
            )
        }.toString()

        if (!current.setMix(json)) {
            return fail(R.string.playback_could_not_apply_mix)
        }
        for (selection in selections) {
            current.setPackGain(selection.id, packGains[selection.id] ?: 1f)
        }
        loadedSelections = selections
        return true
    }

    /** Fade out over the engine's pause ramp, then let the render thread exit. */
    fun pause() {
        synchronized(lock) {
            val current = engine ?: return
            if (!_isPlaying.value) return
            if (renderThread == null) {
                current.stop()
                pausing = false
                _isPlaying.value = false
                releaseWakeLock()
                return
            }
            pausing = true
            _isPlaying.value = false
            current.pause()
        }
    }

    fun stop() {
        synchronized(lock) {
            pausing = false
            if (stopRenderThread()) {
                engine?.stop()
            }
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

    fun masterGain(): Float = synchronized(lock) { masterGain }

    fun setPackGain(id: String, volume: Float) {
        synchronized(lock) {
            packGains = packGains + (id to volume.coerceIn(0f, 1f))
            if (loadedSelections.any { it.id == id }) {
                engine?.setPackGain(id, packGains.getValue(id))
            }
        }
    }

    fun setPackGains(values: Map<String, Float>) {
        synchronized(lock) {
            packGains = values.mapValues { (_, volume) -> volume.coerceIn(0f, 1f) }
            for (selection in loadedSelections) {
                engine?.setPackGain(selection.id, packGains[selection.id] ?: 1f)
            }
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

    /** Mix JSON is expressed in dB; zero volume maps to the floor, then the
     *  exact linear gain (including mute) is applied through setPackGain. */
    private fun volumeToGainDb(volume: Float): Double {
        val value = volume.coerceIn(0f, 1f)
        return if (value <= 0.0001f) -60.0 else 20.0 * kotlin.math.log10(value.toDouble())
    }

    /**
     * Returns true when no render thread is left running. On failure the
     * reference is kept so a later call can join it before freeing memory.
     */
    private fun stopRenderThread(): Boolean {
        val current = renderThread ?: return true
        if (current.stop()) {
            renderThread = null
            return true
        }
        return false
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

    /**
     * Publish a user-facing message for the UI to show.
     *
     * Without an application context there is nothing that could render the
     * message, so a null message (no snackbar) is the correct outcome.
     */
    private fun fail(@StringRes messageRes: Int, vararg args: Any): Boolean {
        _message.value = appContext?.getString(messageRes, *args)
        return false
    }
}

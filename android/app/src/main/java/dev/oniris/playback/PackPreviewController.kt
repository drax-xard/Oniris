package dev.oniris.playback

import dev.oniris.audio.AudioRenderThread
import dev.oniris.audio.NativeEngine
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject

/**
 * Short, low-risk pack previews for the library.
 *
 * Previews use their own engine and AudioTrack, and are only allowed while the
 * main mix is not playing. The preview ends with the engine's sleep timer so the
 * fade-out and render-thread shutdown stay sample-accurate.
 */
object PackPreviewController {
    private const val PREVIEW_MS = 30_000L
    private const val FADE_IN_MS = 800L
    private const val FADE_OUT_MS = 2_000L

    private val lock = Any()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    private var engine: NativeEngine? = null
    private var thread: AudioRenderThread? = null
    private var cleanupJob: Job? = null
    private var generation = 0

    private val _activePackId = MutableStateFlow<String?>(null)
    val activePackId: StateFlow<String?> = _activePackId.asStateFlow()

    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    fun toggle(selection: PackSelection) {
        if (_activePackId.value == selection.id) {
            stop()
            return
        }
        if (PlaybackController.isPlaying.value) {
            _message.value = "Pause playback to preview a pack"
            return
        }

        stop()
        val previousCleanup = cleanupJob
        val request = synchronized(lock) { ++generation }
        scope.launch {
            previousCleanup?.join()
            val error = withContext(Dispatchers.IO) { startEngine(selection, request) }
            if (error != null) {
                _message.value = error
            }
        }
    }

    fun stop() {
        val oldThread: AudioRenderThread?
        val oldEngine: NativeEngine?
        synchronized(lock) {
            generation += 1
            oldThread = thread
            oldEngine = engine
            thread = null
            engine = null
            _activePackId.value = null
        }
        if (oldThread == null && oldEngine == null) return
        cleanupJob = scope.launch(Dispatchers.IO) {
            oldThread?.stop()
            oldEngine?.stop()
            oldEngine?.close()
        }
    }

    fun clearMessage() {
        _message.value = null
    }

    private fun startEngine(selection: PackSelection, request: Int): String? {
        val preview = NativeEngine()
        if (!preview.available) {
            preview.close()
            return "The native audio engine is not available"
        }
        if (!preview.loadPack(selection.dir.absolutePath)) {
            preview.close()
            return "Could not load ${selection.name}"
        }

        val mix = JSONObject().apply {
            put("masterGain", 1.0)
            put(
                "packs",
                JSONArray().put(
                    JSONObject()
                        .put("id", selection.id)
                        .put("gainDb", 0.0)
                        .put("muted", false),
                ),
            )
        }.toString()
        if (!preview.setMix(mix)) {
            preview.close()
            return "Could not start the preview"
        }

        preview.setFadeIn(FADE_IN_MS)
        preview.setMasterGain(1f)
        preview.play()
        preview.setSleepTimer(PREVIEW_MS, FADE_OUT_MS)

        synchronized(lock) {
            if (request != generation) {
                preview.stop()
                preview.close()
                return null
            }
            engine = preview
            thread = AudioRenderThread(preview) {
                scope.launch { finish(preview) }
            }.also { it.start() }
            _activePackId.value = selection.id
        }
        return null
    }

    private fun finish(finished: NativeEngine) {
        synchronized(lock) {
            if (engine !== finished) return
            engine = null
            thread = null
            _activePackId.value = null
        }
        finished.stop()
        finished.close()
    }
}

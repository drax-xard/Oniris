package dev.oniris.audio

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.os.Process
import android.util.Log
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.concurrent.thread
import kotlin.math.max

/**
 * Owns the AudioTrack and the native render loop.
 *
 * One blocking write per 4096-frame block (~85 ms at 48 kHz). No decoders run
 * here; the native engine mixes pre-decoded PCM.
 */
class AudioRenderThread(
    private val engine: NativeEngine,
    private val sampleRate: Int = 48_000,
    private val onFinished: () -> Unit = {},
) {
    @Volatile
    private var running = false
    private var thread: Thread? = null

    val isRunning: Boolean
        get() = running

    fun start() {
        if (running) return
        running = true
        thread = thread(name = "oniris-audio", priority = Thread.MAX_PRIORITY) { loop() }
    }

    private fun loop() {
        Process.setThreadPriority(Process.THREAD_PRIORITY_URGENT_AUDIO)

        val frames = 4096
        val bytesPerFrame = 4 // stereo 16-bit
        var track: AudioTrack? = null

        try {
            val minBuffer = AudioTrack.getMinBufferSize(
                sampleRate,
                AudioFormat.CHANNEL_OUT_STEREO,
                AudioFormat.ENCODING_PCM_16BIT,
            )
            val bufferSize = max(minBuffer, frames * bytesPerFrame * 3)

            track = AudioTrack.Builder()
                .setAudioAttributes(
                    AudioAttributes.Builder()
                        .setUsage(AudioAttributes.USAGE_MEDIA)
                        .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                        .build(),
                )
                .setAudioFormat(
                    AudioFormat.Builder()
                        .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                        .setSampleRate(sampleRate)
                        .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                        .build(),
                )
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setBufferSizeInBytes(bufferSize)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_POWER_SAVING)
                .build()

            if (track.state != AudioTrack.STATE_INITIALIZED) {
                Log.e(TAG, "AudioTrack is not initialized")
                return
            }

            val buffer = ByteBuffer.allocateDirect(frames * bytesPerFrame)
                .order(ByteOrder.LITTLE_ENDIAN)

            track.play()
            while (running) {
                buffer.clear()
                val rendered = engine.render(buffer, frames)
                if (rendered <= 0) {
                    Thread.sleep(5)
                    continue
                }

                buffer.position(0)
                buffer.limit(rendered * bytesPerFrame)
                val written = track.write(buffer, buffer.remaining(), AudioTrack.WRITE_BLOCKING)
                if (written < 0) {
                    Log.e(TAG, "AudioTrack.write failed: $written")
                    break
                }

                if (engine.isFinished() || engine.isPaused()) break
            }
        } catch (error: Throwable) {
            Log.e(TAG, "audio render loop failed", error)
        } finally {
            running = false
            track?.let {
                runCatching { it.pause() }
                runCatching { it.flush() }
                it.release()
            }
            onFinished()
        }
    }

    /**
     * Stop the render loop. Returns false if the thread did not exit within
     * 1.5 s (for example a wedged audio HAL); the caller must then leave the
     * native engine alive rather than freeing memory the thread may still use.
     */
    fun stop(): Boolean {
        running = false
        val current = thread ?: return true
        current.join(1_500)
        thread = null
        return !current.isAlive
    }

    private companion object {
        const val TAG = "Oniris"
    }
}

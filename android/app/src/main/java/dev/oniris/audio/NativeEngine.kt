package dev.oniris.audio

import java.nio.ByteBuffer

/**
 * JNI bridge to the Rust engine.
 *
 * Control calls and [render] are synchronized because the native engine is not
 * internally synchronized. A render call is short (one 4096-frame block), so
 * control calls wait at most a few milliseconds. A later refinement can move
 * controls to a lock-free command queue.
 */
class NativeEngine(sampleRate: Int = 48_000) : AutoCloseable {
    private var handle: Long = 0L

    val available: Boolean

    init {
        available = try {
            System.loadLibrary("oniris_jni")
            handle = nativeCreate(sampleRate)
            handle != 0L
        } catch (_: Throwable) {
            false
        }
    }

    @Synchronized
    fun loadPack(path: String): Boolean =
        handle != 0L && nativeLoadPack(handle, path) == 0

    @Synchronized
    fun setMix(json: String): Boolean =
        handle != 0L && nativeSetMix(handle, json) == 0

    @Synchronized
    fun setPackGain(id: String, gain: Float): Boolean =
        handle != 0L && nativeSetPackGain(handle, id, gain) == 0

    @Synchronized
    fun setMasterGain(gain: Float) {
        if (handle != 0L) nativeSetMasterGain(handle, gain)
    }

    @Synchronized
    fun play() {
        if (handle != 0L) nativePlay(handle)
    }

    @Synchronized
    fun pause() {
        if (handle != 0L) nativePause(handle)
    }

    @Synchronized
    fun stop() {
        if (handle != 0L) nativeStop(handle)
    }

    @Synchronized
    fun setSleepTimer(durationMs: Long, fadeMs: Long) {
        if (handle != 0L) nativeSetSleepTimer(handle, durationMs, fadeMs)
    }

    @Synchronized
    fun clearSleepTimer() {
        if (handle != 0L) nativeClearSleepTimer(handle)
    }

    @Synchronized
    fun render(buffer: ByteBuffer, frames: Int): Int =
        if (handle == 0L) 0 else nativeRender(handle, buffer, frames)

    @Synchronized
    fun isFinished(): Boolean = handle != 0L && nativeIsFinished(handle)

    @Synchronized
    fun decodedBytes(): Long = if (handle == 0L) 0L else nativeDecodedBytes(handle)

    override fun close() {
        synchronized(this) {
            if (handle != 0L) {
                nativeDestroy(handle)
                handle = 0L
            }
        }
    }

    private external fun nativeCreate(sampleRate: Int): Long
    private external fun nativeDestroy(handle: Long)
    private external fun nativeLoadPack(handle: Long, path: String): Int
    private external fun nativeSetMix(handle: Long, json: String): Int
    private external fun nativeSetPackGain(handle: Long, id: String, gainDb: Float): Int
    private external fun nativeSetMasterGain(handle: Long, gain: Float)
    private external fun nativePlay(handle: Long)
    private external fun nativePause(handle: Long)
    private external fun nativeStop(handle: Long)
    private external fun nativeSetSleepTimer(handle: Long, durationMs: Long, fadeMs: Long)
    private external fun nativeClearSleepTimer(handle: Long)
    private external fun nativeRender(handle: Long, buffer: ByteBuffer, frames: Int): Int
    private external fun nativeIsFinished(handle: Long): Boolean
    private external fun nativeDecodedBytes(handle: Long): Long
}

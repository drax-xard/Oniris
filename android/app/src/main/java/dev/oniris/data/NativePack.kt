package dev.oniris.data

/**
 * JNI bridge for pack archive import.
 *
 * The Rust side validates the zip, extracts it into a temporary directory, and
 * decodes every asset before publishing it under the packs directory. This
 * keeps the safety rules identical to `oniris verify-pack`/`unpack`.
 */
object NativePack {
    private val loaded = try {
        System.loadLibrary("oniris_jni")
        true
    } catch (_: Throwable) {
        false
    }

    val available: Boolean
        get() = loaded

    /**
     * Install [archivePath] into [packsDir] and return the manifest id.
     * Throws [RuntimeException] when the archive is invalid or the pack is
     * already installed.
     */
    fun installArchive(archivePath: String, packsDir: String): String? {
        if (!loaded) return null
        return nativeInstallArchive(archivePath, packsDir)
    }

    private external fun nativeInstallArchive(archivePath: String, packsDir: String): String?
}

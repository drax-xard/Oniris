package dev.oniris.data

import android.content.Context
import android.net.Uri
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.util.UUID

/** A pack that passed the minimal on-device manifest checks. */
data class PackInfo(
    val id: String,
    val name: String,
    val description: String,
    val tags: List<String>,
    val bedCount: Int,
    val eventCount: Int,
    val license: String,
    val credits: List<Credit>,
    val procedural: Boolean,
    val directory: File,
)

/** Attribution for one pack asset, or for the pack as a whole. */
data class Credit(
    val asset: String?,
    val title: String,
    val author: String,
    val source: String,
    val license: String,
    val modified: Boolean,
)

/**
 * Finds packs under `filesDir/packs` and installs packs bundled in
 * `assets/packs` on first launch.
 *
 * Full validation (path safety, decode, loop bounds) happens in the Rust core
 * when the pack is loaded. This repository only reads enough of the manifest to
 * render the library.
 */
class PackRepository(context: Context) {
    private val appContext = context.applicationContext
    private val packsDir = File(appContext.filesDir, "packs")
    private val assets = appContext.assets

    suspend fun listPacks(): List<PackInfo> = withContext(Dispatchers.IO) {
        installBundledPacks()
        scan()
    }

    /**
     * Import an `.onirispack` chosen through the system document picker.
     *
     * The archive is copied into the cache first (the Rust extractor needs a
     * filesystem path), then validated and installed by [NativePack]. A failed
     * import leaves the existing library untouched.
     */
    suspend fun importArchive(uri: Uri): PackInfo = withContext(Dispatchers.IO) {
        val archive = File(appContext.cacheDir, "import-${UUID.randomUUID()}.onirispack")
        try {
            copyToCache(uri, archive)
            val id = NativePack.installArchive(archive.absolutePath, packsDir.absolutePath)
                ?: throw IOException("The native pack importer is not available on this device")
            readPack(File(packsDir, id))
                ?: throw IOException("The imported pack could not be read")
        } finally {
            archive.delete()
        }
    }

    private fun copyToCache(uri: Uri, destination: File) {
        appContext.contentResolver.openInputStream(uri)?.use { source ->
            destination.outputStream().use { target ->
                val buffer = ByteArray(COPY_BUFFER_BYTES)
                var total = 0L
                while (true) {
                    val read = source.read(buffer)
                    if (read < 0) break
                    total += read
                    if (total > MAX_ARCHIVE_BYTES) {
                        throw IOException("The selected pack is larger than the import limit")
                    }
                    target.write(buffer, 0, read)
                }
            }
        } ?: throw IOException("Could not open the selected file")
    }

    private fun installBundledPacks() {
        packsDir.mkdirs()
        val bundled = runCatching { assets.list(BUNDLED_ROOT) }.getOrNull() ?: return
        for (id in bundled) {
            val destination = File(packsDir, id)
            if (destination.exists()) continue
            runCatching { copyAssetDirectory("$BUNDLED_ROOT/$id", destination) }
                .onFailure { Log.w(TAG, "could not install bundled pack $id", it) }
        }
    }

    private fun copyAssetDirectory(assetPath: String, destination: File) {
        val children = assets.list(assetPath) ?: return
        if (children.isEmpty()) {
            destination.parentFile?.mkdirs()
            assets.open(assetPath).use { input ->
                destination.outputStream().use { output -> input.copyTo(output) }
            }
            return
        }

        destination.mkdirs()
        for (child in children) {
            copyAssetDirectory("$assetPath/$child", File(destination, child))
        }
    }

    private fun scan(): List<PackInfo> {
        packsDir.mkdirs()
        return packsDir.listFiles()
            .orEmpty()
            .filter { it.isDirectory }
            .mapNotNull { readPack(it) }
            .sortedBy { it.name.lowercase() }
    }

    private fun readPack(directory: File): PackInfo? {
        val manifestFile = File(directory, "manifest.json")
        if (!manifestFile.isFile) return null

        return runCatching {
            val json = JSONObject(manifestFile.readText())
            val id = json.optString("id").takeIf { it.isNotBlank() } ?: return null
            if (id != directory.name) {
                Log.w(TAG, "pack ${directory.name} declares id $id; skipping")
                return null
            }
            val names = json.optJSONObject("name")
            val descriptions = json.optJSONObject("description")
            val tagsJson = json.optJSONArray("tags")
            val tags = buildList {
                if (tagsJson != null) {
                    for (index in 0 until tagsJson.length()) {
                        add(tagsJson.optString(index))
                    }
                }
            }

            val creditsJson = json.optJSONArray("credits")
            val credits = buildList {
                if (creditsJson != null) {
                    for (index in 0 until creditsJson.length()) {
                        val credit = creditsJson.optJSONObject(index) ?: continue
                        val title = credit.optString("title").trim()
                        val author = credit.optString("author").trim()
                        val license = credit.optString("license").trim()
                        if (title.isEmpty() || author.isEmpty() || license.isEmpty()) continue
                        add(
                            Credit(
                                asset = credit.optString("asset").trim().takeIf { it.isNotEmpty() },
                                title = title,
                                author = author,
                                source = credit.optString("source").trim(),
                                license = license,
                                modified = credit.optBoolean("modified", false),
                            ),
                        )
                    }
                }
            }

            PackInfo(
                id = id,
                name = names?.optString("en")?.takeIf { it.isNotBlank() } ?: id,
                description = descriptions?.optString("en").orEmpty(),
                tags = tags.filter { it.isNotBlank() },
                bedCount = json.optJSONArray("beds")?.length() ?: 0,
                eventCount = json.optJSONArray("events")?.length() ?: 0,
                license = json.optString("license").trim(),
                credits = credits,
                procedural = json.optBoolean("procedural", false),
                directory = directory,
            )
        }.onFailure { Log.w(TAG, "could not read ${directory.name}", it) }.getOrNull()
    }

    companion object {
        private const val TAG = "Oniris"
        private const val BUNDLED_ROOT = "packs"
        private const val COPY_BUFFER_BYTES = 64 * 1024
        private const val MAX_ARCHIVE_BYTES = 1024L * 1024 * 1024
    }
}

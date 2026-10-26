package dev.oniris.data

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File

/** A pack that passed the minimal on-device manifest checks. */
data class PackInfo(
    val id: String,
    val name: String,
    val description: String,
    val tags: List<String>,
    val bedCount: Int,
    val eventCount: Int,
    val directory: File,
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
    private val packsDir = File(context.filesDir, "packs")
    private val assets = context.assets

    suspend fun listPacks(): List<PackInfo> = withContext(Dispatchers.IO) {
        installBundledPacks()
        scan()
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

            PackInfo(
                id = id,
                name = names?.optString("en")?.takeIf { it.isNotBlank() } ?: id,
                description = descriptions?.optString("en").orEmpty(),
                tags = tags.filter { it.isNotBlank() },
                bedCount = json.optJSONArray("beds")?.length() ?: 0,
                eventCount = json.optJSONArray("events")?.length() ?: 0,
                directory = directory,
            )
        }.onFailure { Log.w(TAG, "could not read ${directory.name}", it) }.getOrNull()
    }

    companion object {
        private const val TAG = "Oniris"
        private const val BUNDLED_ROOT = "packs"
    }
}

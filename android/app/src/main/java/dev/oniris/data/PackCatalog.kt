package dev.oniris.data

import android.content.Context
import android.os.Build
import android.util.Log
import dev.oniris.R
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/** One downloadable pack, as advertised by `packs/index.json` in the repo. */
data class CatalogEntry(
    val id: String,
    val name: String,
    val description: String,
    val version: String,
    val license: String,
    val credits: String,
    val tags: List<String>,
    val bedCount: Int,
    val eventCount: Int,
    val sizeBytes: Long,
    val sha256: String,
    val url: String,
    val minAppVersionCode: Int,
)

/**
 * Fetches the pack catalog and downloads pack archives.
 *
 * The catalog is a plain JSON file in the repository (`packs/index.json`), served
 * from the raw CDN: no API token, no rate limit, and it is versioned with the
 * packs it describes. The archive's SHA-256 is the only thing standing between a
 * user and an arbitrary file, so [download] refuses to hand back bytes that do
 * not match the catalog entry.
 */
class PackCatalog(context: Context) {
    private val appContext = context.applicationContext
    private val cacheFile = File(appContext.cacheDir, CACHE_FILE)
    private val downloadDir = File(appContext.cacheDir, "downloads").apply { mkdirs() }

    /**
     * Current catalog. Serves the on-disk copy when it is younger than
     * [CACHE_TTL_MS]; any network failure falls back to the cached copy so the
     * library still shows something when the device is offline.
     */
    suspend fun entries(forceRefresh: Boolean = false): List<CatalogEntry> = withContext(Dispatchers.IO) {
        val cached = readCache()
        if (!forceRefresh && cached != null && isFresh()) return@withContext cached
        try {
            val body = httpGet(CATALOG_URL)
            val parsed = parse(body)
            runCatching { cacheFile.writeText(body) }
            parsed
        } catch (error: Exception) {
            Log.w(TAG, "catalog fetch failed", error)
            cached ?: throw IOException(
                appContext.getString(R.string.error_catalog_unavailable),
            )
        }
    }

    /** Downloads and verifies [entry], reporting progress in 0..1. */
    suspend fun download(entry: CatalogEntry, onProgress: (Float) -> Unit = {}): File =
        withContext(Dispatchers.IO) {
            val target = File(downloadDir, "${entry.id}-v${entry.version}.onirispack")
            val partial = File(downloadDir, "${target.name}.part")
            partial.delete()
            val digest = MessageDigest.getInstance("SHA-256")
            var written = 0L
            try {
                val connection = openConnection(entry.url)
                try {
                    val declared = connection.contentLengthLong
                    val expected = if (declared > 0) declared else entry.sizeBytes
                    connection.inputStream.use { input ->
                        partial.outputStream().use { output ->
                            val buffer = ByteArray(BUFFER_BYTES)
                            while (true) {
                                val read = input.read(buffer)
                                if (read < 0) break
                                digest.update(buffer, 0, read)
                                written += read
                                if (written > MAX_DOWNLOAD_BYTES) {
                                    throw IOException(
                                        appContext.getString(R.string.error_pack_too_large),
                                    )
                                }
                                output.write(buffer, 0, read)
                                if (expected > 0) {
                                    onProgress((written.toFloat() / expected).coerceIn(0f, 1f))
                                }
                            }
                        }
                    }
                } finally {
                    connection.disconnect()
                }

                val actual = digest.digest().joinToString("") { "%02x".format(it) }
                if (!actual.equals(entry.sha256, ignoreCase = true)) {
                    throw IOException(
                        appContext.getString(R.string.error_catalog_checksum, entry.name),
                    )
                }
                if (target.exists()) target.delete()
                if (!partial.renameTo(target)) {
                    throw IOException(appContext.getString(R.string.error_catalog_download, entry.name))
                }
                onProgress(1f)
                target
            } finally {
                partial.delete()
            }
        }

    private fun isFresh(): Boolean =
        cacheFile.isFile && System.currentTimeMillis() - cacheFile.lastModified() < CACHE_TTL_MS

    private fun readCache(): List<CatalogEntry>? = runCatching {
        if (!cacheFile.isFile) null else parse(cacheFile.readText())
    }.getOrNull()

    private fun parse(body: String): List<CatalogEntry> {
        val root = JSONObject(body)
        val array = root.optJSONArray("packs") ?: return emptyList()
        val appVersionCode = appVersionCode()
        val result = mutableListOf<CatalogEntry>()
        for (index in 0 until array.length()) {
            val json = array.optJSONObject(index) ?: continue
            val id = json.optString("id").trim()
            val url = json.optString("url").trim()
            val digest = json.optString("sha256").trim()
            if (id.isEmpty() || url.isEmpty() || digest.length != 64) {
                Log.w(TAG, "catalog entry $index is incomplete; skipping")
                continue
            }
            val minCode = json.optInt("minAppVersionCode", 1)
            if (appVersionCode > 0 && appVersionCode < minCode) {
                Log.i(TAG, "pack $id needs app version code $minCode; hiding")
                continue
            }
            val tagsJson = json.optJSONArray("tags")
            val tags = buildList {
                if (tagsJson != null) {
                    for (tagIndex in 0 until tagsJson.length()) add(tagsJson.optString(tagIndex))
                }
            }
            result.add(
                CatalogEntry(
                    id = id,
                    name = json.optJSONObject("name")?.optString("en")?.takeIf { it.isNotBlank() } ?: id,
                    description = json.optJSONObject("description")?.optString("en").orEmpty(),
                    version = json.optString("version").trim(),
                    license = json.optString("license").trim(),
                    credits = json.optString("credits").trim(),
                    tags = tags.filter { it.isNotBlank() },
                    bedCount = json.optInt("beds", 0),
                    eventCount = json.optInt("events", 0),
                    sizeBytes = json.optLong("sizeBytes", 0L),
                    sha256 = digest,
                    url = url,
                    minAppVersionCode = minCode,
                ),
            )
        }
        return result
    }

    @Suppress("DEPRECATION")
    private fun appVersionCode(): Int {
        val info = appContext.packageManager.getPackageInfo(appContext.packageName, 0)
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.longVersionCode.toInt()
        } else {
            info.versionCode
        }
    }

    private fun httpGet(url: String): String {
        val connection = openConnection(url)
        return try {
            val status = connection.responseCode
            if (status != HttpURLConnection.HTTP_OK) {
                throw IOException("HTTP $status for $url")
            }
            connection.inputStream.bufferedReader().use { it.readText() }
        } finally {
            connection.disconnect()
        }
    }

    private fun openConnection(url: String): HttpURLConnection =
        (URL(url).openConnection() as HttpURLConnection).apply {
            connectTimeout = CONNECT_TIMEOUT_MS
            readTimeout = READ_TIMEOUT_MS
            instanceFollowRedirects = true
            setRequestProperty("Accept", "application/json, application/octet-stream")
            setRequestProperty("User-Agent", "Oniris-Android")
        }

    companion object {
        private const val TAG = "Oniris"
        private const val CATALOG_URL =
            "https://raw.githubusercontent.com/drax-xard/Oniris/main/packs/index.json"
        private const val CACHE_FILE = "pack-catalog.json"
        private const val CACHE_TTL_MS = 60L * 60L * 1000L
        private const val CONNECT_TIMEOUT_MS = 10_000
        private const val READ_TIMEOUT_MS = 20_000
        private const val BUFFER_BYTES = 64 * 1024
        private const val MAX_DOWNLOAD_BYTES = 512L * 1024 * 1024
    }
}

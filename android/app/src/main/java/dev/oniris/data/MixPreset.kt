package dev.oniris.data

import org.json.JSONArray
import org.json.JSONObject

/** A named mix: pack selection, per-pack levels, master level, and timer. */
data class MixPreset(
    val id: String,
    val name: String,
    val packIds: List<String>,
    val packGains: Map<String, Float> = emptyMap(),
    val masterVolume: Float = 0.8f,
    val timerMinutes: Int = 45,
    val builtIn: Boolean = false,
)

internal fun MixPreset.toJson(): JSONObject = JSONObject().apply {
    put("id", id)
    put("name", name)
    put("packIds", JSONArray(packIds))
    put(
        "packGains",
        JSONObject(packGains.mapValues { (_, gain) -> gain.coerceIn(0f, 1f) }),
    )
    put("masterVolume", masterVolume.coerceIn(0f, 1f).toDouble())
    put("timerMinutes", timerMinutes.coerceAtLeast(0))
}

internal fun decodePreset(json: JSONObject): MixPreset? {
    val id = json.optString("id").trim().takeIf { it.isNotEmpty() } ?: return null
    val name = json.optString("name").trim().takeIf { it.isNotEmpty() } ?: return null
    val packIdsJson = json.optJSONArray("packIds") ?: return null
    val packIds = buildList {
        for (index in 0 until packIdsJson.length()) {
            val packId = packIdsJson.optString(index).trim()
            if (packId.isNotEmpty() && packId !in this) add(packId)
        }
    }
    if (packIds.isEmpty()) return null

    val gainsJson = json.optJSONObject("packGains")
    val packGains = buildMap {
        if (gainsJson != null) {
            gainsJson.keys().forEach { packId ->
                put(packId, gainsJson.optDouble(packId, 1.0).toFloat().coerceIn(0f, 1f))
            }
        }
    }

    return MixPreset(
        id = id,
        name = name,
        packIds = packIds,
        packGains = packGains,
        masterVolume = json.optDouble("masterVolume", 0.8).toFloat().coerceIn(0f, 1f),
        timerMinutes = json.optInt("timerMinutes", 45).coerceAtLeast(0),
    )
}

internal fun decodePresets(json: String?): List<MixPreset> {
    if (json.isNullOrBlank()) return emptyList()
    return runCatching {
        val array = JSONArray(json)
        buildList {
            for (index in 0 until array.length()) {
                val preset = array.optJSONObject(index)?.let(::decodePreset)
                if (preset != null) add(preset)
            }
        }
    }.getOrDefault(emptyList())
}

internal fun encodePresets(presets: List<MixPreset>): String =
    JSONArray(presets.map { it.toJson() }).toString()

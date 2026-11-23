package dev.oniris.ui.screens

import androidx.annotation.StringRes
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.data.MixPreset
import dev.oniris.ui.components.PresetRow

/**
 * A built-in mix preset. Display names live in string resources so they can be
 * localized; [MixPreset.name] stays empty for these and is resolved with
 * [presetDisplayName] at render time.
 */
private data class BuiltInPreset(
    val id: String,
    @StringRes val nameRes: Int,
    val packIds: List<String>,
    val timerMinutes: Int = 45,
)

private val builtInPresets = listOf(
    BuiltInPreset(
        id = "builtin-rainy-night",
        nameRes = R.string.preset_rainy_night,
        packIds = listOf("rainy-night-procedural"),
    ),
    BuiltInPreset(
        id = "builtin-deep-space",
        nameRes = R.string.preset_deep_space,
        packIds = listOf("deep-space-cabin"),
    ),
    BuiltInPreset(
        id = "builtin-forest",
        nameRes = R.string.preset_forest,
        packIds = listOf("enchanted-forest"),
    ),
    BuiltInPreset(
        id = "builtin-sea-voyage",
        nameRes = R.string.preset_sea_voyage,
        packIds = listOf("windy-sea-voyage"),
    ),
    BuiltInPreset(
        id = "builtin-pure-noise",
        nameRes = R.string.preset_pure_noise,
        packIds = listOf("pure-noise"),
    ),
)

private val builtInPresetNameRes: Map<String, Int> =
    builtInPresets.associate { it.id to it.nameRes }

private fun BuiltInPreset.toMixPreset(): MixPreset = MixPreset(
    id = id,
    // Resolved from nameRes for display; built-ins are never persisted.
    name = "",
    packIds = packIds,
    timerMinutes = timerMinutes,
    builtIn = true,
)

@Composable
private fun presetDisplayName(preset: MixPreset): String {
    val nameRes = if (preset.builtIn) builtInPresetNameRes[preset.id] else null
    return if (nameRes != null) stringResource(nameRes) else preset.name
}

/**
 * Built-in and saved mixes. [onApplyPreset] receives the preset already
 * narrowed to installed packs, its resolved display name, and how many of its
 * packs are missing, so the caller can report the outcome.
 */
@Composable
fun PresetsScreen(
    userPresets: List<MixPreset>,
    availablePackIds: Set<String>,
    canSaveMix: Boolean,
    onSaveMix: () -> Unit,
    onApplyPreset: (preset: MixPreset, displayName: String, missingCount: Int) -> Unit,
    onDeletePreset: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val presets = builtInPresets.map { it.toMixPreset() } + userPresets
    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(
                onClick = onSaveMix,
                enabled = canSaveMix,
            ) {
                Text(stringResource(R.string.action_save_mix))
            }
        }

        presets.forEach { preset ->
            val displayName = presetDisplayName(preset)
            val installed = preset.packIds.filter { it in availablePackIds }
            PresetRow(
                preset = preset,
                displayName = displayName,
                available = installed.isNotEmpty(),
                onApply = {
                    onApplyPreset(
                        preset.copy(
                            packIds = installed,
                            packGains = preset.packGains.filterKeys { it in installed },
                        ),
                        displayName,
                        preset.packIds.size - installed.size,
                    )
                },
                onDelete = if (preset.builtIn) null else ({ onDeletePreset(preset.id) }),
            )
            Spacer(Modifier.height(8.dp))
        }

        Spacer(Modifier.height(16.dp))
    }
}

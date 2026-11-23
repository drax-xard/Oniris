package dev.oniris.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.oniris.data.PackInfo
import dev.oniris.ui.components.Notice
import kotlin.math.log10

/** Master volume plus per-pack trim for the packs currently in the mix. */
@Composable
fun MixerScreen(
    activePacks: List<PackInfo>,
    masterVolume: Float,
    packGains: Map<String, Float>,
    onMasterVolumeChange: (Float) -> Unit,
    onPackGainChange: (packId: String, gain: Float) -> Unit,
    onOpenLibrary: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Surface(
            shape = MaterialTheme.shapes.large,
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 1.dp,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(Modifier.padding(horizontal = 16.dp, vertical = 16.dp)) {
                LevelRow(
                    label = "Master",
                    value = masterVolume,
                    valueRange = 0f..1f,
                    footer = "${(masterVolume * 100).toInt()}%",
                    onValueChange = onMasterVolumeChange,
                )
            }
        }

        Spacer(Modifier.height(24.dp))

        Text(
            text = "Layers",
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(4.dp))
        Text(
            text = if (activePacks.size == 1) {
                "1 pack in the mix"
            } else {
                "${activePacks.size} packs in the mix"
            },
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))

        if (activePacks.isEmpty()) {
            Notice("Nothing in the mix yet.")
            Spacer(Modifier.height(12.dp))
            Button(onClick = onOpenLibrary) {
                Text("Open the library")
            }
        } else {
            activePacks.forEach { pack ->
                val gain = packGains[pack.id] ?: 1f
                Surface(
                    shape = MaterialTheme.shapes.large,
                    color = MaterialTheme.colorScheme.surface,
                    tonalElevation = 1.dp,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.padding(horizontal = 16.dp, vertical = 12.dp)) {
                        LevelRow(
                            label = pack.name,
                            value = gain,
                            valueRange = 0f..1.5f,
                            footer = gainLabel(gain),
                            onValueChange = { value -> onPackGainChange(pack.id, value) },
                        )
                    }
                }
                Spacer(Modifier.height(10.dp))
            }
            Spacer(Modifier.height(6.dp))
            Text(
                text = "0 dB keeps a pack at its authored level; drag down to tuck a layer " +
                    "under the others.",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }

        Spacer(Modifier.height(24.dp))
    }
}

@Composable
private fun LevelRow(
    label: String,
    value: Float,
    valueRange: ClosedFloatingPointRange<Float>,
    footer: String,
    onValueChange: (Float) -> Unit,
) {
    Row(
        Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.titleSmall,
            modifier = Modifier.weight(1f),
        )
        Spacer(Modifier.width(8.dp))
        Text(
            text = footer,
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
    Slider(
        value = value,
        onValueChange = onValueChange,
        valueRange = valueRange,
    )
}

private fun gainLabel(gain: Float): String {
    if (gain <= 0.001f) return "−∞ dB"
    val db = 20f * log10(gain)
    val rounded = (db * 10).toInt() / 10f
    return when {
        rounded > 0f -> "+$rounded dB"
        rounded == 0f -> "0 dB"
        else -> "$rounded dB"
    }.replace("+0.0 dB", "0 dB")
}

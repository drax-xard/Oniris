package dev.oniris.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice
import kotlin.math.roundToInt

/** One included pack as the mixer sees it. */
data class MixChannel(
    val id: String,
    val name: String,
    val gain: Float,
)

/** Master level plus one slider per included pack. */
@Composable
fun MixerScreen(
    masterVolume: Float,
    channels: List<MixChannel>,
    onMasterVolumeChange: (Float) -> Unit,
    onChannelGainChange: (id: String, gain: Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    val masterVolumeDescription = stringResource(R.string.master_volume_content_description)
    val masterVolumeState = stringResource(
        R.string.percent_state,
        (masterVolume * 100).roundToInt(),
    )

    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Text(
            stringResource(R.string.section_volume),
            style = MaterialTheme.typography.titleMedium,
        )
        Slider(
            value = masterVolume,
            onValueChange = onMasterVolumeChange,
            valueRange = 0f..1f,
            modifier = Modifier.semantics {
                contentDescription = masterVolumeDescription
                stateDescription = masterVolumeState
            },
        )

        Spacer(Modifier.height(20.dp))

        if (channels.isEmpty()) {
            Notice(stringResource(R.string.notice_no_included_packs))
        } else {
            channels.forEach { channel ->
                ChannelRow(
                    channel = channel,
                    onGainChange = { gain -> onChannelGainChange(channel.id, gain) },
                )
                Spacer(Modifier.height(10.dp))
            }
        }

        Spacer(Modifier.height(16.dp))
    }
}

@Composable
private fun ChannelRow(
    channel: MixChannel,
    onGainChange: (Float) -> Unit,
) {
    val gainPercent = (channel.gain * 100).roundToInt()
    val volumeDescription =
        stringResource(R.string.pack_volume_content_description, channel.name)
    val volumeState = stringResource(R.string.percent_state, gainPercent)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(horizontal = 16.dp, vertical = 12.dp)) {
            Text(
                text = channel.name,
                style = MaterialTheme.typography.titleSmall,
            )
            Spacer(Modifier.height(8.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = stringResource(R.string.pack_volume_label, gainPercent),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.width(92.dp),
                )
                Slider(
                    value = channel.gain,
                    onValueChange = onGainChange,
                    valueRange = 0f..1f,
                    modifier = Modifier
                        .weight(1f)
                        .semantics {
                            contentDescription = volumeDescription
                            stateDescription = volumeState
                        },
                )
            }
        }
    }
}

package dev.oniris.ui.screens

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
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice

private val timerOptions = listOf(
    0 to R.string.timer_off,
    15 to R.string.timer_15_min,
    30 to R.string.timer_30_min,
    45 to R.string.timer_45_min,
    60 to R.string.timer_60_min,
    90 to R.string.timer_90_min,
)

/**
 * Transport, status line, sleep timer, and a summary of the selected mix.
 * The timer sits directly under the play button so starting a night is
 * play, then tap a duration.
 */
@Composable
fun PlayScreen(
    isPlaying: Boolean,
    engineAvailable: Boolean,
    hasSelection: Boolean,
    proceduralOnly: Boolean,
    decodedMegabytes: Long,
    timerMinutes: Int,
    mixPackNames: List<String>,
    onTogglePlay: () -> Unit,
    onTimerChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        if (!engineAvailable) {
            // Settings carries the full explanation; this is the short version
            // so a disabled Play button is never a mystery.
            Text(
                text = stringResource(R.string.notice_engine_unavailable_short),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(8.dp))
        }

        Button(
            onClick = onTogglePlay,
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp),
            enabled = engineAvailable && hasSelection,
        ) {
            Text(
                stringResource(if (isPlaying) R.string.action_pause else R.string.action_play),
            )
        }

        Spacer(Modifier.height(10.dp))

        Text(
            text = statusLine(
                isPlaying = isPlaying,
                hasSelection = hasSelection,
                proceduralOnly = proceduralOnly,
                decodedMegabytes = decodedMegabytes,
            ),
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(24.dp))

        Text(
            stringResource(R.string.section_sleep_timer),
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(8.dp))
        timerOptions.chunked(3).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                row.forEach { (minutes, labelRes) ->
                    FilterChip(
                        selected = timerMinutes == minutes,
                        onClick = { onTimerChange(minutes) },
                        label = { Text(stringResource(labelRes)) },
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
        }

        Spacer(Modifier.height(16.dp))

        Text(
            stringResource(R.string.mix_summary_title),
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(8.dp))
        if (mixPackNames.isEmpty()) {
            Notice(stringResource(R.string.mix_summary_empty))
        } else {
            Surface(
                shape = MaterialTheme.shapes.large,
                color = MaterialTheme.colorScheme.surface,
                tonalElevation = 1.dp,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(Modifier.padding(horizontal = 16.dp, vertical = 14.dp)) {
                    mixPackNames.forEachIndexed { index, name ->
                        if (index > 0) Spacer(Modifier.height(6.dp))
                        Text(name, style = MaterialTheme.typography.titleSmall)
                    }
                }
            }
        }

        Spacer(Modifier.height(24.dp))
    }
}

@Composable
private fun statusLine(
    isPlaying: Boolean,
    hasSelection: Boolean,
    proceduralOnly: Boolean,
    decodedMegabytes: Long,
): String = when {
    isPlaying -> stringResource(R.string.status_playing, decodedMegabytes)
    !hasSelection -> if (proceduralOnly) {
        stringResource(R.string.status_select_procedural_pack)
    } else {
        stringResource(R.string.status_select_pack)
    }
    else -> stringResource(R.string.status_ready)
}

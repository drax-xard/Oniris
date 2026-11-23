package dev.oniris.ui.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.oniris.ui.components.Notice

private val timerOptions = listOf(
    0 to "Off",
    15 to "15 min",
    30 to "30 min",
    45 to "45 min",
    60 to "1 hour",
    90 to "1.5 hours",
)

/** Sleep timer. The engine fades the mix out over the last minute. */
@Composable
fun TimerScreen(
    timerMinutes: Int,
    onTimerChange: (Int) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Text(
            text = if (timerMinutes == 0) "Timer off" else "Stop after ${timerLabel(timerMinutes)}",
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(10.dp))

        timerOptions.chunked(2).forEach { row ->
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                row.forEach { (minutes, label) ->
                    FilterChip(
                        selected = timerMinutes == minutes,
                        onClick = { onTimerChange(minutes) },
                        label = { Text(label) },
                        modifier = Modifier.weight(1f),
                    )
                }
                if (row.size == 1) {
                    Spacer(Modifier.weight(1f))
                }
            }
            Spacer(Modifier.height(8.dp))
        }

        Spacer(Modifier.height(20.dp))

        Notice(
            if (timerMinutes == 0) {
                "No timer. Playback keeps going until you stop it."
            } else {
                "After ${timerLabel(timerMinutes)} the mix fades out over 60 seconds and " +
                    "the session pauses. Changing the timer applies to the running session."
            },
        )

        Spacer(Modifier.height(12.dp))

        Text(
            text = "The fade is sample-accurate in the audio engine, so it finishes " +
                "exactly at the deadline rather than on a slow tick.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(24.dp))
    }
}

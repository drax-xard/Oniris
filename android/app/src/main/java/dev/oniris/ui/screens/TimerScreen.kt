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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice

/** Timer options in the UI: minutes paired with their label resource. */
private val timerOptions: List<Pair<Int, Int>> = listOf(
    0 to R.string.timer_option_off,
    15 to R.string.timer_option_15,
    30 to R.string.timer_option_30,
    45 to R.string.timer_option_45,
    60 to R.string.timer_option_60,
    90 to R.string.timer_option_90,
)

/** Inline timer length: "off" / "45 min" / "1 h". */
@Composable
internal fun timerLabel(minutes: Int): String = when (minutes) {
    0 -> stringResource(R.string.timer_short_off)
    15 -> stringResource(R.string.timer_short_15)
    30 -> stringResource(R.string.timer_short_30)
    45 -> stringResource(R.string.timer_short_45)
    60 -> stringResource(R.string.timer_short_60)
    90 -> stringResource(R.string.timer_short_90)
    else -> stringResource(R.string.timer_short_minutes, minutes)
}

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
            text = if (timerMinutes == 0) {
                stringResource(R.string.timer_title_off)
            } else {
                stringResource(R.string.timer_title_stop_after, timerLabel(timerMinutes))
            },
            style = MaterialTheme.typography.titleMedium,
        )
        Spacer(Modifier.height(10.dp))

        timerOptions.chunked(2).forEach { row ->
            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                row.forEach { (minutes, labelRes) ->
                    FilterChip(
                        selected = timerMinutes == minutes,
                        onClick = { onTimerChange(minutes) },
                        label = { Text(stringResource(labelRes)) },
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
                stringResource(R.string.timer_notice_off)
            } else {
                stringResource(R.string.timer_notice_set, timerLabel(timerMinutes))
            },
        )

        Spacer(Modifier.height(12.dp))

        Text(
            text = stringResource(R.string.timer_fade_note),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(24.dp))
    }
}

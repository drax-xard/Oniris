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
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice

/**
 * Transport screen: what is loaded, whether it is running, and one big control.
 */
@Composable
fun PlayScreen(
    packNames: List<String>,
    isPlaying: Boolean,
    available: Boolean,
    decodedBytes: Long,
    timerMinutes: Int,
    onPlayPause: () -> Unit,
    onStop: () -> Unit,
    onOpenLibrary: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val separator = stringResource(R.string.list_separator)
    val status = buildList {
        add(pluralStringResource(R.plurals.play_layers, packNames.size, packNames.size))
        add(
            when {
                isPlaying -> stringResource(
                    R.string.play_status_offline,
                    (decodedBytes / (1024 * 1024)).toInt(),
                )

                packNames.isEmpty() -> stringResource(R.string.play_status_silent)
                else -> stringResource(R.string.play_status_idle)
            },
        )
        add(stringResource(R.string.play_status_timer, timerLabel(timerMinutes)))
    }.joinToString(separator)

    Column(
        modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        if (!available) {
            Notice(stringResource(R.string.play_engine_missing))
            Spacer(Modifier.height(16.dp))
        }

        Surface(
            shape = MaterialTheme.shapes.extraLarge,
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 2.dp,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(horizontal = 20.dp, vertical = 24.dp),
                horizontalAlignment = Alignment.Start,
            ) {
                Text(
                    text = stringResource(
                        when {
                            isPlaying -> R.string.play_state_playing
                            packNames.isEmpty() -> R.string.play_state_nothing_loaded
                            else -> R.string.play_state_ready
                        },
                    ),
                    style = MaterialTheme.typography.headlineMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.height(6.dp))
                Text(
                    text = if (packNames.isEmpty()) {
                        stringResource(R.string.play_empty_hint)
                    } else {
                        packNames.joinToString(separator)
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(14.dp))
                Text(
                    text = status,
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        Spacer(Modifier.height(20.dp))

        Button(
            onClick = onPlayPause,
            modifier = Modifier
                .fillMaxWidth()
                .height(64.dp),
            enabled = available && packNames.isNotEmpty(),
        ) {
            Text(
                text = stringResource(if (isPlaying) R.string.pause else R.string.play),
                style = MaterialTheme.typography.titleMedium,
            )
        }

        Spacer(Modifier.height(8.dp))

        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(
                onClick = onStop,
                enabled = isPlaying,
            ) {
                Text(stringResource(R.string.stop))
            }
            TextButton(onClick = onOpenLibrary) {
                Text(
                    stringResource(
                        if (packNames.isEmpty()) {
                            R.string.play_choose_packs
                        } else {
                            R.string.play_edit_packs
                        },
                    ),
                )
            }
        }

        if (packNames.isEmpty()) {
            Spacer(Modifier.height(16.dp))
            Notice(stringResource(R.string.play_no_selection))
        }

        Spacer(Modifier.height(24.dp))
    }
}

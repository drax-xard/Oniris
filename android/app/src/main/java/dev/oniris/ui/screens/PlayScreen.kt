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
import androidx.compose.ui.unit.dp
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
    Column(
        modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        if (!available) {
            Notice(
                "The native audio engine is not loaded on this build. Browsing works, " +
                    "but playback needs liboniris_jni.so built for this device.",
            )
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
                    text = when {
                        isPlaying -> "Playing"
                        packNames.isEmpty() -> "Nothing loaded"
                        else -> "Ready"
                    },
                    style = MaterialTheme.typography.headlineMedium,
                    color = MaterialTheme.colorScheme.onSurface,
                )
                Spacer(Modifier.height(6.dp))
                Text(
                    text = if (packNames.isEmpty()) {
                        "Pick a few packs in the Library."
                    } else {
                        packNames.joinToString(" · ")
                    },
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(14.dp))
                Text(
                    text = buildString {
                        append(if (packNames.size == 1) "1 layer" else "${packNames.size} layers")
                        append(" · ")
                        append(
                            when {
                                isPlaying -> "${decodedBytes / (1024 * 1024)} MB decoded · offline"
                                packNames.isEmpty() -> "silent"
                                else -> "idle"
                            },
                        )
                        append(" · timer ")
                        append(timerLabel(timerMinutes))
                    },
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
                text = if (isPlaying) "Pause" else "Play",
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
                Text("Stop")
            }
            TextButton(onClick = onOpenLibrary) {
                Text(if (packNames.isEmpty()) "Choose packs" else "Edit packs")
            }
        }

        if (packNames.isEmpty()) {
            Spacer(Modifier.height(16.dp))
            Notice("The library is empty of a selection. Nothing plays until at least one pack is on.")
        }

        Spacer(Modifier.height(24.dp))
    }
}

@Composable
internal fun timerLabel(minutes: Int): String = when (minutes) {
    0 -> "off"
    60 -> "1 h"
    90 -> "1.5 h"
    else -> "$minutes min"
}

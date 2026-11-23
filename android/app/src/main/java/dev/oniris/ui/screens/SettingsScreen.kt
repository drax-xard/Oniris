package dev.oniris.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import dev.oniris.ui.components.Notice

/** Engine diagnostics and app information. */
@Composable
fun SettingsScreen(
    engineAvailable: Boolean,
    decodedBytes: Long,
    installedPacks: Int,
    versionName: String?,
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
            Column(Modifier.padding(horizontal = 16.dp, vertical = 8.dp)) {
                InfoRow(
                    label = "Audio engine",
                    value = if (engineAvailable) "Native (JNI) ready" else "Not available",
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = "Decoded memory",
                    value = "${decodedBytes / (1024 * 1024)} MB · 96 MB budget",
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = "Installed packs",
                    value = installedPacks.toString(),
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = "Version",
                    value = versionName ?: "unknown",
                )
            }
        }

        Spacer(Modifier.height(16.dp))

        Notice(
            "Playback is fully offline: packs are decoded once from app-private " +
                "storage and mixed from memory, with no network, analytics, or timers " +
                "running while the screen is off.",
        )

        Spacer(Modifier.height(12.dp))

        Text(
            text = "Battery optimization help, attribution, and a true-peak readout " +
                "land in a later pass.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(24.dp))
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            modifier = Modifier.weight(1f),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

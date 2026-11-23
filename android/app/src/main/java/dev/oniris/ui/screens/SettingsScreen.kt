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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice

/**
 * Mirrors `DEFAULT_MEMORY_LIMIT` in `oniris-core` (96 MB). Kept here only for
 * display; the engine is the one that enforces it.
 */
private const val MEMORY_BUDGET_MB = 96

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
                    label = stringResource(R.string.settings_audio_engine),
                    value = stringResource(
                        if (engineAvailable) {
                            R.string.settings_engine_ready
                        } else {
                            R.string.settings_engine_missing
                        },
                    ),
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = stringResource(R.string.settings_decoded_memory),
                    value = stringResource(
                        R.string.settings_decoded_value,
                        (decodedBytes / (1024 * 1024)).toInt(),
                        MEMORY_BUDGET_MB,
                    ),
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = stringResource(R.string.settings_installed_packs),
                    value = installedPacks.toString(),
                )
                HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f))
                InfoRow(
                    label = stringResource(R.string.settings_version),
                    value = versionName ?: stringResource(R.string.settings_version_unknown),
                )
            }
        }

        Spacer(Modifier.height(16.dp))

        Notice(stringResource(R.string.settings_offline_notice))

        Spacer(Modifier.height(12.dp))

        Text(
            text = stringResource(R.string.settings_later_note),
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

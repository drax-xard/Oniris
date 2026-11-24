package dev.oniris.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import java.util.Locale
import dev.oniris.R
import dev.oniris.data.CatalogEntry
import dev.oniris.data.MixPreset
import dev.oniris.data.PackInfo

/** A preset card: name, its summary line, and Apply/Delete. */
@Composable
fun PresetRow(
    preset: MixPreset,
    displayName: String,
    available: Boolean,
    onApply: () -> Unit,
    onDelete: (() -> Unit)?,
    modifier: Modifier = Modifier,
) {
    val separator = stringResource(R.string.separator)
    val applyDescription = stringResource(R.string.preset_apply_content_description, displayName)
    val deleteDescription = stringResource(R.string.preset_delete_content_description, displayName)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(text = displayName, style = MaterialTheme.typography.titleSmall)
                val details = buildList {
                    add(
                        pluralStringResource(
                            R.plurals.pack_count,
                            preset.packIds.size,
                            preset.packIds.size,
                        ),
                    )
                    add(
                        if (preset.timerMinutes > 0) {
                            stringResource(R.string.preset_timer, preset.timerMinutes)
                        } else {
                            stringResource(R.string.preset_no_timer)
                        },
                    )
                    if (preset.builtIn) add(stringResource(R.string.preset_built_in))
                    if (!available) add(stringResource(R.string.preset_missing_packs))
                }.joinToString(separator)
                Text(
                    text = details,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            TextButton(
                onClick = onApply,
                enabled = available,
                modifier = Modifier.semantics {
                    contentDescription = applyDescription
                },
            ) {
                Text(stringResource(R.string.action_apply))
            }
            if (onDelete != null) {
                TextButton(
                    onClick = onDelete,
                    modifier = Modifier.semantics {
                        contentDescription = deleteDescription
                    },
                ) {
                    Text(stringResource(R.string.action_delete))
                }
            }
        }
    }
}

/** A library row: pack details, the include switch, and the preview button. */
@Composable
fun PackRow(
    pack: PackInfo,
    selected: Boolean,
    previewing: Boolean,
    onPreview: () -> Unit,
    onSelectedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val separator = stringResource(R.string.separator)
    val tagSeparator = stringResource(R.string.tag_separator)
    val includeDescription = stringResource(R.string.pack_include_content_description, pack.name)
    val stateIncluded = stringResource(R.string.state_included)
    val stateNotIncluded = stringResource(R.string.state_not_included)
    val previewDescription = stringResource(R.string.preview_content_description, pack.name)
    val stopPreviewDescription =
        stringResource(R.string.preview_stop_content_description, pack.name)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    text = pack.name,
                    style = MaterialTheme.typography.titleMedium,
                )
                if (pack.description.isNotBlank()) {
                    Text(
                        text = pack.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Spacer(Modifier.height(6.dp))
                val details = buildList {
                    add(
                        pluralStringResource(
                            R.plurals.pack_bed_count,
                            pack.bedCount,
                            pack.bedCount,
                        ),
                    )
                    add(
                        pluralStringResource(
                            R.plurals.pack_event_count,
                            pack.eventCount,
                            pack.eventCount,
                        ),
                    )
                    if (pack.procedural) add(stringResource(R.string.pack_procedural))
                    if (pack.tags.isNotEmpty()) {
                        add(pack.tags.take(3).joinToString(tagSeparator))
                    }
                }.joinToString(separator)
                Text(
                    text = details,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.width(12.dp))
            Column(horizontalAlignment = Alignment.End) {
                Switch(
                    checked = selected,
                    onCheckedChange = onSelectedChange,
                    modifier = Modifier.semantics {
                        contentDescription = includeDescription
                        stateDescription = if (selected) stateIncluded else stateNotIncluded
                    },
                )
                TextButton(
                    onClick = onPreview,
                    modifier = Modifier.semantics {
                        contentDescription = if (previewing) {
                            stopPreviewDescription
                        } else {
                            previewDescription
                        }
                    },
                ) {
                    Text(
                        stringResource(
                            if (previewing) R.string.action_stop else R.string.action_preview,
                        ),
                    )
                }
            }
        }
    }
}

/**
 * A catalog row: the pack's identity, what is installed, and a download button.
 * Shows [progress] while a download is running and never offers a second
 * download while one is in flight.
 */
@Composable
fun CatalogRow(
    entry: CatalogEntry,
    installedVersion: String?,
    downloading: Boolean,
    progress: Float,
    onDownload: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val separator = stringResource(R.string.separator)
    val tagSeparator = stringResource(R.string.tag_separator)
    val downloadDescription = stringResource(R.string.catalog_download_content_description, entry.name)
    val sizeMegabytes = String.format(Locale.getDefault(), "%.1f", entry.sizeBytes / (1024.0 * 1024.0))
    val sizeLabel = stringResource(R.string.catalog_size_mb, sizeMegabytes)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(text = entry.name, style = MaterialTheme.typography.titleMedium)
                if (entry.description.isNotBlank()) {
                    Text(
                        text = entry.description,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Spacer(Modifier.height(6.dp))
                val details = buildList {
                    add(
                        pluralStringResource(
                            R.plurals.pack_bed_count,
                            entry.bedCount,
                            entry.bedCount,
                        ),
                    )
                    add(
                        pluralStringResource(
                            R.plurals.pack_event_count,
                            entry.eventCount,
                            entry.eventCount,
                        ),
                    )
                    if (entry.tags.isNotEmpty()) add(entry.tags.take(3).joinToString(tagSeparator))
                }.joinToString(separator)
                Text(
                    text = stringResource(
                        R.string.catalog_details,
                        entry.version,
                        entry.license,
                        sizeLabel,
                        details,
                    ),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                val status = when {
                    installedVersion == null -> null
                    installedVersion == entry.version ->
                        stringResource(R.string.catalog_installed, installedVersion)
                    else -> stringResource(R.string.catalog_update, installedVersion, entry.version)
                }
                if (status != null) {
                    Text(
                        text = status,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
                if (downloading) {
                    Spacer(Modifier.height(6.dp))
                    LinearProgressIndicator(
                        progress = { progress },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
            Spacer(Modifier.width(12.dp))
            TextButton(
                onClick = onDownload,
                enabled = !downloading,
                modifier = Modifier.semantics { contentDescription = downloadDescription },
            ) {
                Text(
                    if (downloading) {
                        stringResource(
                            R.string.catalog_downloading,
                            (progress * 100).toInt(),
                        )
                    } else {
                        stringResource(R.string.catalog_download)
                    },
                )
            }
        }
    }
}

/** A muted card for empty states and engine notices. */
@Composable
fun Notice(text: String, modifier: Modifier = Modifier) {
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        modifier = modifier
            .fillMaxWidth()
            .padding(top = 12.dp),
    ) {
        Column(Modifier.padding(16.dp)) {
            HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f))
            Spacer(Modifier.height(10.dp))
            Text(
                text = text,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

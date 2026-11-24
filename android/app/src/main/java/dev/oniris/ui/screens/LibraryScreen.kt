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
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.data.CatalogEntry
import dev.oniris.data.PackInfo
import dev.oniris.ui.components.CatalogRow
import dev.oniris.ui.components.Notice
import dev.oniris.ui.components.PackRow

/**
 * Installed packs: import and refresh, the procedural-only filter, and one row
 * per visible pack with its include switch and preview.
 */
@Composable
fun LibraryScreen(
    packs: List<PackInfo>,
    availablePacks: List<PackInfo>,
    loading: Boolean,
    importing: Boolean,
    importEnabled: Boolean,
    proceduralOnly: Boolean,
    includedPackIds: Set<String>,
    previewingId: String?,
    catalog: List<CatalogEntry>,
    catalogChecking: Boolean,
    catalogError: String?,
    downloadingId: String?,
    downloadProgress: Float,
    onImport: () -> Unit,
    onRefresh: () -> Unit,
    onProceduralOnlyChange: (Boolean) -> Unit,
    onPreview: (PackInfo) -> Unit,
    onIncludedChange: (PackInfo, Boolean) -> Unit,
    onCheckCatalog: () -> Unit,
    onDownload: (CatalogEntry) -> Unit,
    modifier: Modifier = Modifier,
) {
    // Semantics descriptions are plain lambdas, so resolve them here.
    val proceduralOnlyDescription = stringResource(R.string.procedural_only_content_description)
    val stateOn = stringResource(R.string.state_on)
    val stateOff = stringResource(R.string.state_off)

    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(
                onClick = onImport,
                enabled = !importing && importEnabled,
            ) {
                Text(
                    stringResource(
                        if (importing) R.string.action_importing else R.string.action_import,
                    ),
                )
            }
            TextButton(
                onClick = onRefresh,
                enabled = !loading && !importing,
            ) {
                Text(stringResource(R.string.action_refresh))
            }
        }

        Spacer(Modifier.height(4.dp))
        Row(
            Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    text = stringResource(R.string.procedural_only_title),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Text(
                    text = stringResource(R.string.procedural_only_description),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(
                checked = proceduralOnly,
                onCheckedChange = onProceduralOnlyChange,
                modifier = Modifier.semantics {
                    contentDescription = proceduralOnlyDescription
                    stateDescription = if (proceduralOnly) stateOn else stateOff
                },
            )
        }
        Spacer(Modifier.height(10.dp))

        when {
            loading -> {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(vertical = 24.dp),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator()
                }
            }

            packs.isEmpty() -> Notice(stringResource(R.string.notice_no_packs))

            availablePacks.isEmpty() -> Notice(stringResource(R.string.notice_no_procedural_packs))

            else -> {
                availablePacks.forEach { pack ->
                    PackRow(
                        pack = pack,
                        selected = pack.id in includedPackIds,
                        previewing = previewingId == pack.id,
                        onPreview = { onPreview(pack) },
                        onSelectedChange = { checked -> onIncludedChange(pack, checked) },
                    )
                    Spacer(Modifier.height(10.dp))
                }
            }
        }
        if (!proceduralOnly) {
            Spacer(Modifier.height(24.dp))
            Row(
                Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text(
                        text = stringResource(R.string.catalog_section_title),
                        style = MaterialTheme.typography.titleSmall,
                    )
                    Text(
                        text = stringResource(R.string.catalog_section_description),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                TextButton(
                    onClick = onCheckCatalog,
                    enabled = !catalogChecking && downloadingId == null,
                ) {
                    Text(
                        stringResource(
                            if (catalogChecking) R.string.catalog_checking else R.string.catalog_check,
                        ),
                    )
                }
            }
            Spacer(Modifier.height(10.dp))
            when {
                catalogError != null && catalog.isEmpty() ->
                    Notice(stringResource(R.string.catalog_check_failed))

                catalog.isEmpty() && catalogChecking -> Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(vertical = 16.dp),
                    horizontalArrangement = Arrangement.Center,
                ) {
                    CircularProgressIndicator()
                }

                catalog.isEmpty() -> Notice(stringResource(R.string.catalog_empty))

                else -> catalog.forEach { entry ->
                    CatalogRow(
                        entry = entry,
                        installedVersion = packs.firstOrNull { it.id == entry.id }?.version,
                        downloading = downloadingId == entry.id,
                        progress = if (downloadingId == entry.id) downloadProgress else 0f,
                        onDownload = { onDownload(entry) },
                    )
                    Spacer(Modifier.height(10.dp))
                }
            }
        }

        Spacer(Modifier.height(16.dp))
    }
}

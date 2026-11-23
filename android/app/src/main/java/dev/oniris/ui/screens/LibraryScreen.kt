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
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.data.PackInfo
import dev.oniris.ui.components.Notice
import dev.oniris.ui.components.PackCard

/** Pack library: browse what is installed and toggle it into the mix. */
@Composable
fun LibraryScreen(
    packs: List<PackInfo>,
    loading: Boolean,
    selectedIds: Set<String>,
    onSelectedChange: (packId: String, selected: Boolean) -> Unit,
    onRefresh: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Row(
            Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = when {
                    loading -> stringResource(R.string.library_scanning)
                    packs.isEmpty() -> stringResource(R.string.library_none_installed)
                    else -> stringResource(
                        R.string.library_selected_count,
                        packs.count { it.id in selectedIds },
                        packs.size,
                    )
                },
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = onRefresh, enabled = !loading) {
                Text(stringResource(R.string.action_refresh))
            }
        }

        Spacer(Modifier.height(8.dp))

        when {
            loading -> Row(
                Modifier
                    .fillMaxWidth()
                    .padding(vertical = 24.dp),
                horizontalArrangement = Arrangement.Center,
            ) {
                CircularProgressIndicator()
            }

            packs.isEmpty() -> Notice(stringResource(R.string.library_empty_notice))

            else -> {
                packs.forEach { pack ->
                    PackCard(
                        pack = pack,
                        selected = pack.id in selectedIds,
                        onSelectedChange = { checked -> onSelectedChange(pack.id, checked) },
                    )
                    Spacer(Modifier.height(10.dp))
                }
            }
        }

        Spacer(Modifier.height(24.dp))
    }
}

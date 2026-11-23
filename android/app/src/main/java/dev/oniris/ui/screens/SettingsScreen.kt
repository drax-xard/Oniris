package dev.oniris.ui.screens

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.oniris.R
import dev.oniris.ui.components.Notice

/** Engine diagnostics and the credits entry point. */
@Composable
fun SettingsScreen(
    engineAvailable: Boolean,
    hasCredits: Boolean,
    onShowLicenses: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        if (!engineAvailable) {
            Notice(stringResource(R.string.notice_engine_unavailable))
        }

        if (hasCredits) {
            Spacer(Modifier.height(24.dp))
            HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f))
            Spacer(Modifier.height(8.dp))
            TextButton(
                onClick = onShowLicenses,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(stringResource(R.string.licenses_title))
            }
        } else {
            Notice(stringResource(R.string.notice_no_credits))
        }

        Spacer(Modifier.height(24.dp))
    }
}

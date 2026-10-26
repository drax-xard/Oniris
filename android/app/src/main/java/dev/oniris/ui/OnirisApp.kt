package dev.oniris.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.oniris.data.PackInfo
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PackSelection
import dev.oniris.playback.PlaybackController
import dev.oniris.playback.PlaybackService
import kotlinx.coroutines.launch

private val timerOptions = listOf(
    0 to "Off",
    15 to "15 min",
    30 to "30 min",
    45 to "45 min",
    60 to "1 h",
    90 to "1.5 h",
)

@Composable
fun OnirisApp(
    repository: PackRepository,
    settingsStore: SettingsStore,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val settings by settingsStore.settings.collectAsStateWithLifecycle(initialValue = null)
    val isPlaying by PlaybackController.isPlaying.collectAsStateWithLifecycle()
    val message by PlaybackController.message.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }

    var packs by remember { mutableStateOf<List<PackInfo>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var refreshToken by remember { mutableStateOf(0) }

    LaunchedEffect(refreshToken) {
        loading = true
        packs = repository.listPacks()
        loading = false
    }

    val current = settings
    val selectedIds = current?.selectedPackIds ?: emptySet()

    LaunchedEffect(packs, selectedIds) {
        val selection = packs
            .filter { it.id in selectedIds }
            .map { PackSelection(it.id, it.directory) }
        val wasPlaying = PlaybackController.isPlaying.value
        PlaybackController.setPacks(selection)
        if (wasPlaying) {
            PlaybackController.stop()
            if (selection.isEmpty()) {
                PlaybackService.stop(context)
            } else {
                PlaybackService.play(context)
            }
        }
    }

    LaunchedEffect(current?.masterVolume) {
        current?.let { PlaybackController.setMasterGain(it.masterVolume) }
    }

    LaunchedEffect(current?.timerMinutes) {
        current?.let { PlaybackController.setSleepTimer(it.timerMinutes) }
    }

    LaunchedEffect(message) {
        message?.let {
            snackbarHostState.showSnackbar(it)
            PlaybackController.clearMessage()
        }
    }

    val background = Brush.verticalGradient(
        listOf(
            MaterialTheme.colorScheme.background,
            MaterialTheme.colorScheme.surfaceVariant,
            MaterialTheme.colorScheme.background,
        ),
    )

    Scaffold(
        modifier = modifier.fillMaxSize(),
        containerColor = MaterialTheme.colorScheme.background,
        snackbarHost = { SnackbarHost(snackbarHostState) },
    ) { padding ->
        Box(
            Modifier
                .fillMaxSize()
                .background(background)
                .padding(padding),
        ) {
            Column(
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp, vertical = 16.dp),
            ) {
                Text(
                    text = "Oniris",
                    style = MaterialTheme.typography.displaySmall,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(
                    text = "Layer a night of sound and let it run.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                if (!PlaybackController.available) {
                    Spacer(Modifier.height(16.dp))
                    Notice("The native audio engine is not loaded. Build liboniris_jni.so for this device to play audio.")
                }

                Spacer(Modifier.height(28.dp))

                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "Sound packs",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(
                        onClick = { refreshToken += 1 },
                        enabled = !loading,
                    ) {
                        Text("Refresh")
                    }
                }

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

                    packs.isEmpty() -> Notice(
                        "No packs installed yet. Generate the development packs with " +
                            "`make packs`, then push one into the app's files/packs directory " +
                            "or bundle it under assets/packs.",
                    )

                    else -> {
                        packs.forEach { pack ->
                            PackRow(
                                pack = pack,
                                selected = pack.id in selectedIds,
                                onSelectedChange = { checked ->
                                    scope.launch {
                                        val next = if (checked) {
                                            selectedIds + pack.id
                                        } else {
                                            selectedIds - pack.id
                                        }
                                        settingsStore.setSelectedPacks(next)
                                    }
                                },
                            )
                            Spacer(Modifier.height(10.dp))
                        }
                    }
                }

                Spacer(Modifier.height(20.dp))

                Text("Volume", style = MaterialTheme.typography.titleMedium)
                Slider(
                    value = current?.masterVolume ?: 0.8f,
                    onValueChange = { value ->
                        scope.launch { settingsStore.setMasterVolume(value) }
                    },
                    valueRange = 0f..1f,
                )

                Spacer(Modifier.height(12.dp))

                Text("Sleep timer", style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.height(8.dp))
                timerOptions.chunked(3).forEach { row ->
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        row.forEach { (minutes, label) ->
                            FilterChip(
                                selected = (current?.timerMinutes ?: 0) == minutes,
                                onClick = {
                                    scope.launch { settingsStore.setTimerMinutes(minutes) }
                                },
                                label = { Text(label) },
                            )
                        }
                    }
                    Spacer(Modifier.height(8.dp))
                }

                Spacer(Modifier.height(20.dp))

                Button(
                    onClick = {
                        if (isPlaying) {
                            PlaybackService.pause(context)
                        } else {
                            PlaybackService.play(context)
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(56.dp),
                    enabled = PlaybackController.available && selectedIds.isNotEmpty(),
                ) {
                    Text(if (isPlaying) "Pause" else "Play")
                }

                Spacer(Modifier.height(10.dp))

                Text(
                    text = if (isPlaying) {
                        "Playing · offline · no ads, no accounts"
                    } else if (selectedIds.isEmpty()) {
                        "Select at least one pack"
                    } else {
                        "Ready"
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                Spacer(Modifier.height(24.dp))
            }
        }
    }
}

@Composable
private fun PackRow(
    pack: PackInfo,
    selected: Boolean,
    onSelectedChange: (Boolean) -> Unit,
) {
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = Modifier.fillMaxWidth(),
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
                val details = buildString {
                    append(pack.bedCount)
                    append(if (pack.bedCount == 1) " bed" else " beds")
                    append(" · ")
                    append(pack.eventCount)
                    append(if (pack.eventCount == 1) " event" else " events")
                    if (pack.tags.isNotEmpty()) {
                        append(" · ")
                        append(pack.tags.take(3).joinToString(", "))
                    }
                }
                Text(
                    text = details,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.width(12.dp))
            Switch(checked = selected, onCheckedChange = onSelectedChange)
        }
    }
}

@Composable
private fun Notice(text: String) {
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        modifier = Modifier
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

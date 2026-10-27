package dev.oniris.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
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
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
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
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.oniris.data.MixPreset
import dev.oniris.data.NativePack
import dev.oniris.data.PackInfo
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PackPreviewController
import dev.oniris.playback.PackSelection
import dev.oniris.playback.PlaybackController
import dev.oniris.playback.PlaybackService
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlin.math.roundToInt
import java.util.UUID

private const val MAX_MIX_PACKS = 4

private val builtInPresets = listOf(
    MixPreset(
        id = "builtin-rainy-night",
        name = "Rainy Night",
        packIds = listOf("rainy-night-procedural"),
        timerMinutes = 45,
        builtIn = true,
    ),
    MixPreset(
        id = "builtin-deep-space",
        name = "Deep Space",
        packIds = listOf("deep-space-cabin"),
        timerMinutes = 45,
        builtIn = true,
    ),
    MixPreset(
        id = "builtin-forest",
        name = "Forest",
        packIds = listOf("enchanted-forest"),
        timerMinutes = 45,
        builtIn = true,
    ),
    MixPreset(
        id = "builtin-pure-noise",
        name = "Pure Noise",
        packIds = listOf("pure-noise"),
        timerMinutes = 45,
        builtIn = true,
    ),
)

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
    val previewingId by PackPreviewController.activePackId.collectAsStateWithLifecycle()
    val previewMessage by PackPreviewController.message.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }

    var packs by remember { mutableStateOf<List<PackInfo>>(emptyList()) }
    var loading by remember { mutableStateOf(true) }
    var importing by remember { mutableStateOf(false) }
    var refreshToken by remember { mutableIntStateOf(0) }
    var showSavePreset by remember { mutableStateOf(false) }
    var presetName by remember { mutableStateOf("") }

    val importLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            importing = true
            try {
                val pack = repository.importArchive(uri)
                snackbarHostState.showSnackbar("Imported ${pack.name}")
                refreshToken += 1
            } catch (error: Exception) {
                snackbarHostState.showSnackbar(error.message ?: "Could not import the pack")
            } finally {
                importing = false
            }
        }
    }

    LaunchedEffect(refreshToken) {
        loading = true
        packs = repository.listPacks()
        loading = false
    }

    val current = settings
    val selectedIds = current?.selectedPackIds ?: emptySet()
    val proceduralOnly = current?.proceduralOnly ?: false
    val packGains = current?.packGains ?: emptyMap()
    val availablePacks = if (proceduralOnly) packs.filter { it.procedural } else packs
    val availableIds = availablePacks.map { it.id }.toSet()
    val activeSelectedIds = selectedIds.intersect(availableIds)
    val installedIds = packs.map { it.id }.toSet()
    val presets = builtInPresets + (current?.presets ?: emptyList())

    LaunchedEffect(packs, selectedIds, proceduralOnly) {
        val selection = availablePacks
            .filter { it.id in selectedIds }
            .map { PackSelection(it.id, it.name, it.directory) }
        val wasPlaying = PlaybackController.isPlaying.value
        if (wasPlaying && selection.isEmpty()) {
            PlaybackController.setPacks(selection)
            PlaybackController.stop()
            PlaybackService.stop(context)
        } else if (wasPlaying) {
            withContext(Dispatchers.IO) {
                PlaybackController.setPacks(selection)
            }
        } else {
            PlaybackController.setPacks(selection)
        }
    }

    LaunchedEffect(current?.masterVolume) {
        current?.let { PlaybackController.setMasterGain(it.masterVolume) }
    }

    LaunchedEffect(current?.packGains) {
        current?.let { PlaybackController.setPackGains(it.packGains) }
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

    LaunchedEffect(previewMessage) {
        previewMessage?.let {
            snackbarHostState.showSnackbar(it)
            PackPreviewController.clearMessage()
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
                        text = "Presets",
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(
                        onClick = {
                            presetName = ""
                            showSavePreset = true
                        },
                        enabled = activeSelectedIds.isNotEmpty(),
                    ) {
                        Text("Save mix")
                    }
                }

                presets.forEach { preset ->
                    val installed = preset.packIds.filter { it in availableIds }
                    PresetRow(
                        preset = preset,
                        available = installed.isNotEmpty(),
                        onApply = {
                            if (installed.isEmpty()) {
                                scope.launch {
                                    snackbarHostState.showSnackbar(
                                        "Install a pack from ${preset.name} first",
                                    )
                                }
                            } else {
                                scope.launch {
                                    settingsStore.applyPreset(
                                        preset.copy(
                                            packIds = installed,
                                            packGains = preset.packGains
                                                .filterKeys { it in installed },
                                        ),
                                    )
                                    val missing = preset.packIds.size - installed.size
                                    snackbarHostState.showSnackbar(
                                        if (missing > 0) {
                                            "Applied ${preset.name}; $missing pack(s) missing"
                                        } else {
                                            "Applied ${preset.name}"
                                        },
                                    )
                                }
                            }
                        },
                        onDelete = if (preset.builtIn) null else {
                            { scope.launch { settingsStore.deletePreset(preset.id) } }
                        },
                    )
                    Spacer(Modifier.height(8.dp))
                }

                Spacer(Modifier.height(24.dp))

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
                        onClick = {
                            importLauncher.launch(
                                arrayOf(
                                    "application/zip",
                                    "application/octet-stream",
                                    "application/x-onirispack",
                                    "application/x-zip-compressed",
                                ),
                            )
                        },
                        enabled = !importing && NativePack.available,
                    ) {
                        Text(if (importing) "Importing…" else "Import")
                    }
                    TextButton(
                        onClick = { refreshToken += 1 },
                        enabled = !loading && !importing,
                    ) {
                        Text("Refresh")
                    }
                }

                Spacer(Modifier.height(4.dp))
                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(
                            text = "Procedural only",
                            style = MaterialTheme.typography.bodyMedium,
                        )
                        Text(
                            text = "Show only fully synthesized soundscapes; recorded packs stay hidden.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Switch(
                        checked = proceduralOnly,
                        onCheckedChange = { enabled ->
                            scope.launch { settingsStore.setProceduralOnly(enabled) }
                        },
                        modifier = Modifier.semantics {
                            contentDescription = "Procedural packs only"
                            stateDescription = if (proceduralOnly) "On" else "Off"
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

                    packs.isEmpty() -> Notice(
                        "No packs installed yet. Import an .onirispack with the Import button, " +
                            "generate development packs with `make packs`, or bundle them under assets/packs.",
                    )

                    proceduralOnly && availablePacks.isEmpty() -> Notice(
                        "No procedural packs installed. Generate them with `make procedural`, " +
                            "then refresh or push them into the app's files/packs directory.",
                    )

                    else -> {
                        availablePacks.forEach { pack ->
                            PackRow(
                                pack = pack,
                                selected = pack.id in activeSelectedIds,
                                gain = packGains[pack.id] ?: 1f,
                                previewing = previewingId == pack.id,
                                onPreview = {
                                    PackPreviewController.toggle(
                                        PackSelection(pack.id, pack.name, pack.directory),
                                    )
                                },
                                onGainChange = { value ->
                                    PlaybackController.setPackGain(pack.id, value)
                                    scope.launch { settingsStore.setPackGain(pack.id, value) }
                                },
                                onSelectedChange = { checked ->
                                    scope.launch {
                                        if (checked && selectedIds.size >= MAX_MIX_PACKS) {
                                            snackbarHostState.showSnackbar(
                                                "Mix up to $MAX_MIX_PACKS packs",
                                            )
                                            return@launch
                                        }
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
                    modifier = Modifier.semantics {
                        contentDescription = "Master volume"
                        stateDescription =
                            "${((current?.masterVolume ?: 0.8f) * 100).roundToInt()} percent"
                    },
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
                        PackPreviewController.stop()
                        if (isPlaying) {
                            PlaybackService.pause(context)
                        } else {
                            PlaybackService.play(context)
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(56.dp),
                    enabled = PlaybackController.available && activeSelectedIds.isNotEmpty(),
                ) {
                    Text(if (isPlaying) "Pause" else "Play")
                }

                Spacer(Modifier.height(10.dp))

                Text(
                    text = if (isPlaying) {
                        val megabytes = PlaybackController.decodedBytes() / (1024 * 1024)
                        "Playing · $megabytes MB decoded · offline"
                    } else if (activeSelectedIds.isEmpty()) {
                        if (proceduralOnly) {
                            "Select a procedural pack"
                        } else {
                            "Select at least one pack"
                        }
                    } else {
                        "Ready"
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                if (packs.any { it.credits.isNotEmpty() }) {
                    Spacer(Modifier.height(24.dp))
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f))
                    Spacer(Modifier.height(16.dp))
                    Text("Licenses & credits", style = MaterialTheme.typography.titleMedium)
                    packs.filter { it.credits.isNotEmpty() }.forEach { pack ->
                        Spacer(Modifier.height(10.dp))
                        Text(
                            text = pack.name,
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        pack.credits.forEach { credit ->
                            val line = buildString {
                                append(credit.title)
                                append(" — ")
                                append(credit.author)
                                append(" (")
                                append(credit.license)
                                if (credit.modified) append(", edited")
                                append(")")
                            }
                            Text(
                                text = line,
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            if (credit.source.isNotBlank()) {
                                Text(
                                    text = credit.source,
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }

                Spacer(Modifier.height(24.dp))
            }
        }
    }

    if (showSavePreset) {
        AlertDialog(
            onDismissRequest = { showSavePreset = false },
            title = { Text("Save current mix") },
            text = {
                OutlinedTextField(
                    value = presetName,
                    onValueChange = { presetName = it },
                    label = { Text("Preset name") },
                    singleLine = true,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        val name = presetName.trim()
                        if (name.isEmpty()) return@TextButton
                        val preset = MixPreset(
                            id = "user-${UUID.randomUUID()}",
                            name = name,
                            packIds = activeSelectedIds.toList(),
                            packGains = packGains.filterKeys { it in activeSelectedIds },
                            masterVolume = current?.masterVolume ?: 0.8f,
                            timerMinutes = current?.timerMinutes ?: 45,
                        )
                        scope.launch {
                            settingsStore.savePreset(preset)
                            showSavePreset = false
                            presetName = ""
                            snackbarHostState.showSnackbar("Saved $name")
                        }
                    },
                    enabled = presetName.isNotBlank(),
                ) {
                    Text("Save")
                }
            },
            dismissButton = {
                TextButton(onClick = { showSavePreset = false }) {
                    Text("Cancel")
                }
            },
        )
    }
}

@Composable
private fun PresetRow(
    preset: MixPreset,
    available: Boolean,
    onApply: () -> Unit,
    onDelete: (() -> Unit)?,
) {
    Surface(
        shape = MaterialTheme.shapes.large,
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 1.dp,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(text = preset.name, style = MaterialTheme.typography.titleSmall)
                val details = buildString {
                    append(preset.packIds.size)
                    append(if (preset.packIds.size == 1) " pack" else " packs")
                    append(" · ")
                    append(if (preset.timerMinutes > 0) "${preset.timerMinutes} min timer" else "no timer")
                    if (preset.builtIn) append(" · built-in")
                    if (!available) append(" · missing packs")
                }
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
                    contentDescription = "Apply ${preset.name}"
                },
            ) {
                Text("Apply")
            }
            if (onDelete != null) {
                TextButton(
                    onClick = onDelete,
                    modifier = Modifier.semantics {
                        contentDescription = "Delete ${preset.name}"
                    },
                ) {
                    Text("Delete")
                }
            }
        }
    }
}

@Composable
private fun PackRow(
    pack: PackInfo,
    selected: Boolean,
    gain: Float,
    previewing: Boolean,
    onPreview: () -> Unit,
    onGainChange: (Float) -> Unit,
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
                    if (pack.procedural) {
                        append(" · procedural")
                    }
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
                if (selected) {
                    Spacer(Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = "Volume ${(gain * 100).roundToInt()}%",
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.width(92.dp),
                        )
                        Slider(
                            value = gain,
                            onValueChange = onGainChange,
                            valueRange = 0f..1f,
                            modifier = Modifier
                                .weight(1f)
                                .semantics {
                                    contentDescription = "${pack.name} volume"
                                    stateDescription = "${(gain * 100).roundToInt()} percent"
                                },
                        )
                    }
                }
            }
            Spacer(Modifier.width(12.dp))
            Column(horizontalAlignment = Alignment.End) {
                Switch(
                    checked = selected,
                    onCheckedChange = onSelectedChange,
                    modifier = Modifier.semantics {
                        contentDescription = "Include ${pack.name} in the mix"
                        stateDescription = if (selected) "Included" else "Not included"
                    },
                )
                TextButton(
                    onClick = onPreview,
                    modifier = Modifier.semantics {
                        contentDescription = if (previewing) {
                            "Stop preview of ${pack.name}"
                        } else {
                            "Preview ${pack.name}"
                        }
                    },
                ) {
                    Text(if (previewing) "Stop" else "Preview")
                }
            }
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

package dev.oniris.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.annotation.StringRes
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.oniris.R
import dev.oniris.data.MixPreset
import dev.oniris.data.NativePack
import dev.oniris.data.PackInfo
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PackPreviewController
import dev.oniris.playback.PackSelection
import dev.oniris.playback.PlaybackController
import dev.oniris.playback.PlaybackSession
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlin.math.roundToInt
import java.util.UUID

private const val MAX_MIX_PACKS = 4

/**
 * A built-in mix preset. Display names live in string resources so they can be
 * localized; [MixPreset.name] stays empty for these and is resolved with
 * [presetDisplayName] at render time.
 */
private data class BuiltInPreset(
    val id: String,
    @StringRes val nameRes: Int,
    val packIds: List<String>,
    val timerMinutes: Int = 45,
)

private val builtInPresets = listOf(
    BuiltInPreset(
        id = "builtin-rainy-night",
        nameRes = R.string.preset_rainy_night,
        packIds = listOf("rainy-night-procedural"),
    ),
    BuiltInPreset(
        id = "builtin-deep-space",
        nameRes = R.string.preset_deep_space,
        packIds = listOf("deep-space-cabin"),
    ),
    BuiltInPreset(
        id = "builtin-forest",
        nameRes = R.string.preset_forest,
        packIds = listOf("enchanted-forest"),
    ),
    BuiltInPreset(
        id = "builtin-sea-voyage",
        nameRes = R.string.preset_sea_voyage,
        packIds = listOf("windy-sea-voyage"),
    ),
    BuiltInPreset(
        id = "builtin-pure-noise",
        nameRes = R.string.preset_pure_noise,
        packIds = listOf("pure-noise"),
    ),
)

private val builtInPresetNameRes: Map<String, Int> =
    builtInPresets.associate { it.id to it.nameRes }

private fun BuiltInPreset.toMixPreset(): MixPreset = MixPreset(
    id = id,
    // Resolved from nameRes for display; built-ins are never persisted.
    name = "",
    packIds = packIds,
    timerMinutes = timerMinutes,
    builtIn = true,
)

@Composable
private fun presetDisplayName(preset: MixPreset): String {
    val nameRes = if (preset.builtIn) builtInPresetNameRes[preset.id] else null
    return if (nameRes != null) stringResource(nameRes) else preset.name
}

private val timerOptions = listOf(
    0 to R.string.timer_off,
    15 to R.string.timer_15_min,
    30 to R.string.timer_30_min,
    45 to R.string.timer_45_min,
    60 to R.string.timer_60_min,
    90 to R.string.timer_90_min,
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
    var showLicenses by remember { mutableStateOf(false) }
    var presetName by remember { mutableStateOf("") }

    val importLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            importing = true
            try {
                val pack = repository.importArchive(uri)
                snackbarHostState.showSnackbar(
                    context.getString(R.string.message_imported, pack.name),
                )
                refreshToken += 1
            } catch (error: Exception) {
                snackbarHostState.showSnackbar(
                    error.message ?: context.getString(R.string.message_import_failed),
                )
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
    val presets = builtInPresets.map { it.toMixPreset() } + (current?.presets ?: emptyList())

    LaunchedEffect(packs, selectedIds, proceduralOnly) {
        val selection = availablePacks
            .filter { it.id in selectedIds }
            .map { PackSelection(it.id, it.name, it.directory) }
        val wasPlaying = PlaybackController.isPlaying.value
        if (wasPlaying && selection.isEmpty()) {
            PlaybackController.setPacks(selection)
            PlaybackController.stop()
            PlaybackSession.stop(context)
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

    // Semantics descriptions are plain lambdas, so resolve them here.
    val proceduralOnlyDescription = stringResource(R.string.procedural_only_content_description)
    val stateOn = stringResource(R.string.state_on)
    val stateOff = stringResource(R.string.state_off)
    val masterVolumeDescription = stringResource(R.string.master_volume_content_description)

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
                    text = stringResource(R.string.app_name),
                    style = MaterialTheme.typography.displaySmall,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(
                    text = stringResource(R.string.app_tagline),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                if (!PlaybackController.available) {
                    Spacer(Modifier.height(16.dp))
                    Notice(stringResource(R.string.notice_engine_unavailable))
                }

                Spacer(Modifier.height(28.dp))

                Row(
                    Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = stringResource(R.string.section_presets),
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
                        Text(stringResource(R.string.action_save_mix))
                    }
                }

                presets.forEach { preset ->
                    val displayName = presetDisplayName(preset)
                    val installed = preset.packIds.filter { it in availableIds }
                    PresetRow(
                        preset = preset,
                        available = installed.isNotEmpty(),
                        onApply = {
                            if (installed.isEmpty()) {
                                scope.launch {
                                    snackbarHostState.showSnackbar(
                                        context.getString(R.string.preset_install_first, displayName),
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
                                            context.resources.getQuantityString(
                                                R.plurals.preset_applied_missing,
                                                missing,
                                                displayName,
                                                missing,
                                            )
                                        } else {
                                            context.getString(R.string.preset_applied, displayName)
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
                        text = stringResource(R.string.section_sound_packs),
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
                        Text(
                            stringResource(
                                if (importing) R.string.action_importing else R.string.action_import,
                            ),
                        )
                    }
                    TextButton(
                        onClick = { refreshToken += 1 },
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
                        onCheckedChange = { enabled ->
                            scope.launch { settingsStore.setProceduralOnly(enabled) }
                        },
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

                    proceduralOnly && availablePacks.isEmpty() -> Notice(
                        stringResource(R.string.notice_no_procedural_packs),
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
                                                context.getString(
                                                    R.string.message_mix_limit,
                                                    MAX_MIX_PACKS,
                                                ),
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

                val masterVolume = current?.masterVolume ?: 0.8f
                val masterVolumeState = stringResource(
                    R.string.percent_state,
                    (masterVolume * 100).roundToInt(),
                )
                Text(
                    stringResource(R.string.section_volume),
                    style = MaterialTheme.typography.titleMedium,
                )
                Slider(
                    value = masterVolume,
                    onValueChange = { value ->
                        scope.launch { settingsStore.setMasterVolume(value) }
                    },
                    valueRange = 0f..1f,
                    modifier = Modifier.semantics {
                        contentDescription = masterVolumeDescription
                        stateDescription = masterVolumeState
                    },
                )

                Spacer(Modifier.height(12.dp))

                Text(
                    stringResource(R.string.section_sleep_timer),
                    style = MaterialTheme.typography.titleMedium,
                )
                Spacer(Modifier.height(8.dp))
                timerOptions.chunked(3).forEach { row ->
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        row.forEach { (minutes, labelRes) ->
                            FilterChip(
                                selected = (current?.timerMinutes ?: 0) == minutes,
                                onClick = {
                                    scope.launch { settingsStore.setTimerMinutes(minutes) }
                                },
                                label = { Text(stringResource(labelRes)) },
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
                            PlaybackSession.pause(context)
                        } else {
                            PlaybackSession.play(context)
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(56.dp),
                    enabled = PlaybackController.available && activeSelectedIds.isNotEmpty(),
                ) {
                    Text(
                        stringResource(if (isPlaying) R.string.action_pause else R.string.action_play),
                    )
                }

                Spacer(Modifier.height(10.dp))

                Text(
                    text = if (isPlaying) {
                        val megabytes = PlaybackController.decodedBytes() / (1024 * 1024)
                        stringResource(R.string.status_playing, megabytes)
                    } else if (activeSelectedIds.isEmpty()) {
                        if (proceduralOnly) {
                            stringResource(R.string.status_select_procedural_pack)
                        } else {
                            stringResource(R.string.status_select_pack)
                        }
                    } else {
                        stringResource(R.string.status_ready)
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                if (packs.any { it.credits.isNotEmpty() }) {
                    Spacer(Modifier.height(24.dp))
                    HorizontalDivider(color = MaterialTheme.colorScheme.outline.copy(alpha = 0.4f))
                    Spacer(Modifier.height(8.dp))
                    TextButton(
                        onClick = { showLicenses = true },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text(stringResource(R.string.licenses_title))
                    }
                }

                Spacer(Modifier.height(24.dp))
            }
        }
    }

    if (showSavePreset) {
        AlertDialog(
            onDismissRequest = { showSavePreset = false },
            title = { Text(stringResource(R.string.save_preset_title)) },
            text = {
                OutlinedTextField(
                    value = presetName,
                    onValueChange = { presetName = it },
                    label = { Text(stringResource(R.string.preset_name_label)) },
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
                            snackbarHostState.showSnackbar(
                                context.getString(R.string.preset_saved, name),
                            )
                        }
                    },
                    enabled = presetName.isNotBlank(),
                ) {
                    Text(stringResource(R.string.action_save))
                }
            },
            dismissButton = {
                TextButton(onClick = { showSavePreset = false }) {
                    Text(stringResource(R.string.action_cancel))
                }
            },
        )
    }

    if (showLicenses) {
        val creditedPacks = packs.filter { it.credits.isNotEmpty() }
        AlertDialog(
            onDismissRequest = { showLicenses = false },
            title = { Text(stringResource(R.string.licenses_title)) },
            text = {
                Column(
                    Modifier
                        .heightIn(max = 400.dp)
                        .verticalScroll(rememberScrollState()),
                ) {
                    creditedPacks.forEachIndexed { index, pack ->
                        if (index > 0) Spacer(Modifier.height(16.dp))
                        Text(
                            text = pack.name,
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.primary,
                        )
                        Text(
                            text = pack.license,
                            style = MaterialTheme.typography.labelSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        pack.credits.forEach { credit ->
                            Spacer(Modifier.height(6.dp))
                            val line = if (credit.modified) {
                                stringResource(
                                    R.string.credit_line_edited,
                                    credit.title,
                                    credit.author,
                                    credit.license,
                                )
                            } else {
                                stringResource(
                                    R.string.credit_line,
                                    credit.title,
                                    credit.author,
                                    credit.license,
                                )
                            }
                            Text(
                                text = line,
                                style = MaterialTheme.typography.bodySmall,
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
            },
            confirmButton = {
                TextButton(onClick = { showLicenses = false }) {
                    Text(stringResource(R.string.action_close))
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
    val displayName = presetDisplayName(preset)
    val separator = stringResource(R.string.separator)
    val applyDescription = stringResource(R.string.preset_apply_content_description, displayName)
    val deleteDescription = stringResource(R.string.preset_delete_content_description, displayName)
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
    val separator = stringResource(R.string.separator)
    val tagSeparator = stringResource(R.string.tag_separator)
    val gainPercent = (gain * 100).roundToInt()
    val volumeDescription = stringResource(R.string.pack_volume_content_description, pack.name)
    val volumeState = stringResource(R.string.percent_state, gainPercent)
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
                if (selected) {
                    Spacer(Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            text = stringResource(R.string.pack_volume_label, gainPercent),
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
                                    contentDescription = volumeDescription
                                    stateDescription = volumeState
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

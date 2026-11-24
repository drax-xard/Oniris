package dev.oniris.ui

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.NavigationDrawerItemDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PermanentDrawerSheet
import androidx.compose.material3.PermanentNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.oniris.R
import dev.oniris.data.CatalogEntry
import dev.oniris.data.MixPreset
import dev.oniris.data.NativePack
import dev.oniris.data.PackCatalog
import dev.oniris.data.PackInfo
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PackPreviewController
import dev.oniris.playback.PackSelection
import dev.oniris.playback.PlaybackController
import dev.oniris.playback.PlaybackSession
import dev.oniris.ui.navigation.OnirisDestination
import dev.oniris.ui.screens.LibraryScreen
import dev.oniris.ui.screens.MixChannel
import dev.oniris.ui.screens.MixerScreen
import dev.oniris.ui.screens.PlayScreen
import dev.oniris.ui.screens.PresetsScreen
import dev.oniris.ui.screens.SettingsScreen
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.UUID

private const val MAX_MIX_PACKS = 4

/** Below this width the drawer is modal; at or above it stays on screen. */
private const val WIDE_LAYOUT_MIN_WIDTH_DP = 600

private const val PERMANENT_DRAWER_WIDTH_DP = 280

private const val BYTES_PER_MEGABYTE = 1024 * 1024

private val IMPORT_MIME_TYPES = arrayOf(
    "application/zip",
    "application/octet-stream",
    "application/x-onirispack",
    "application/x-zip-compressed",
)

/**
 * App shell: owns the settings flow, the pack list, the import launcher, the
 * playback effects, the dialogs, and the destination selection. Each
 * destination screen is stateless and only receives values and callbacks.
 */
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
    val mixPackNames by PlaybackController.mixPackNames.collectAsStateWithLifecycle()
    val previewingId by PackPreviewController.activePackId.collectAsStateWithLifecycle()
    val previewMessage by PackPreviewController.message.collectAsStateWithLifecycle()
    val snackbarHostState = remember { SnackbarHostState() }

    var destination by rememberSaveable { mutableStateOf(OnirisDestination.Play) }
    val drawerState = rememberDrawerState(DrawerValue.Closed)

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

    val catalog = remember { PackCatalog(context) }
    var catalogEntries by remember { mutableStateOf<List<CatalogEntry>>(emptyList()) }
    var catalogChecking by remember { mutableStateOf(false) }
    var catalogError by remember { mutableStateOf<String?>(null) }
    var downloadingId by remember { mutableStateOf<String?>(null) }
    var downloadProgress by remember { mutableStateOf(0f) }

    fun checkCatalog(forceRefresh: Boolean) {
        if (catalogChecking) return
        scope.launch {
            catalogChecking = true
            catalogError = null
            try {
                catalogEntries = catalog.entries(forceRefresh = forceRefresh)
            } catch (error: Exception) {
                catalogError = error.message ?: context.getString(R.string.error_catalog_unavailable)
            } finally {
                catalogChecking = false
            }
        }
    }

    LaunchedEffect(Unit) { checkCatalog(forceRefresh = false) }

    fun downloadPack(entry: CatalogEntry) {
        if (downloadingId != null) return
        if (packs.firstOrNull { it.id == entry.id }?.version == entry.version) return
        scope.launch {
            downloadingId = entry.id
            downloadProgress = 0f
            try {
                val archive = catalog.download(entry) { fraction -> downloadProgress = fraction }
                val pack = repository.installDownloaded(archive)
                snackbarHostState.showSnackbar(
                    context.getString(R.string.message_imported, pack.name),
                )
                refreshToken += 1
            } catch (error: Exception) {
                snackbarHostState.showSnackbar(
                    error.message ?: context.getString(R.string.message_import_failed),
                )
            } finally {
                downloadingId = null
                downloadProgress = 0f
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

    val isWideLayout = LocalConfiguration.current.screenWidthDp >= WIDE_LAYOUT_MIN_WIDTH_DP

    // Back closes an open drawer first; from any other destination it returns
    // to Play before the activity itself handles the gesture.
    BackHandler(enabled = drawerState.isOpen || destination != OnirisDestination.Play) {
        if (drawerState.isOpen) {
            scope.launch { drawerState.close() }
        } else {
            destination = OnirisDestination.Play
        }
    }

    val navigate: (OnirisDestination) -> Unit = { target ->
        destination = target
        if (drawerState.isOpen) scope.launch { drawerState.close() }
    }

    val background = Brush.verticalGradient(
        listOf(
            MaterialTheme.colorScheme.background,
            MaterialTheme.colorScheme.surfaceVariant,
            MaterialTheme.colorScheme.background,
        ),
    )

    val masterVolume = current?.masterVolume ?: 0.8f
    val timerMinutes = current?.timerMinutes ?: 0
    val mixerChannels = availablePacks
        .filter { it.id in activeSelectedIds }
        .map { MixChannel(id = it.id, name = it.name, gain = packGains[it.id] ?: 1f) }

    val screens: @Composable (Modifier) -> Unit = { screenModifier ->
        when (destination) {
            OnirisDestination.Play -> PlayScreen(
                isPlaying = isPlaying,
                engineAvailable = PlaybackController.available,
                hasSelection = activeSelectedIds.isNotEmpty(),
                proceduralOnly = proceduralOnly,
                decodedMegabytes = if (isPlaying) {
                    PlaybackController.decodedBytes() / BYTES_PER_MEGABYTE
                } else {
                    0L
                },
                timerMinutes = timerMinutes,
                mixPackNames = mixPackNames,
                onTogglePlay = {
                    PackPreviewController.stop()
                    if (isPlaying) {
                        PlaybackSession.pause(context)
                    } else {
                        PlaybackSession.play(context)
                    }
                },
                onTimerChange = { minutes ->
                    scope.launch { settingsStore.setTimerMinutes(minutes) }
                },
                modifier = screenModifier,
            )

            OnirisDestination.Presets -> PresetsScreen(
                userPresets = current?.presets ?: emptyList(),
                availablePackIds = availableIds,
                canSaveMix = activeSelectedIds.isNotEmpty(),
                onSaveMix = {
                    presetName = ""
                    showSavePreset = true
                },
                onApplyPreset = { preset, displayName, missingCount ->
                    scope.launch {
                        if (preset.packIds.isEmpty()) {
                            snackbarHostState.showSnackbar(
                                context.getString(R.string.preset_install_first, displayName),
                            )
                            return@launch
                        }
                        settingsStore.applyPreset(preset)
                        snackbarHostState.showSnackbar(
                            if (missingCount > 0) {
                                context.resources.getQuantityString(
                                    R.plurals.preset_applied_missing,
                                    missingCount,
                                    displayName,
                                    missingCount,
                                )
                            } else {
                                context.getString(R.string.preset_applied, displayName)
                            },
                        )
                    }
                },
                onDeletePreset = { id ->
                    scope.launch { settingsStore.deletePreset(id) }
                },
                modifier = screenModifier,
            )

            OnirisDestination.Library -> LibraryScreen(
                packs = packs,
                availablePacks = availablePacks,
                loading = loading,
                importing = importing,
                importEnabled = NativePack.available,
                proceduralOnly = proceduralOnly,
                includedPackIds = activeSelectedIds,
                previewingId = previewingId,
                catalog = catalogEntries,
                catalogChecking = catalogChecking,
                catalogError = catalogError,
                downloadingId = downloadingId,
                downloadProgress = downloadProgress,
                onCheckCatalog = { checkCatalog(forceRefresh = true) },
                onDownload = { entry -> downloadPack(entry) },
                onImport = { importLauncher.launch(IMPORT_MIME_TYPES) },
                onRefresh = { refreshToken += 1 },
                onProceduralOnlyChange = { enabled ->
                    scope.launch { settingsStore.setProceduralOnly(enabled) }
                },
                onPreview = { pack ->
                    PackPreviewController.toggle(
                        PackSelection(pack.id, pack.name, pack.directory),
                    )
                },
                onIncludedChange = { pack, checked ->
                    scope.launch {
                        if (checked && selectedIds.size >= MAX_MIX_PACKS) {
                            snackbarHostState.showSnackbar(
                                context.getString(R.string.message_mix_limit, MAX_MIX_PACKS),
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
                modifier = screenModifier,
            )

            OnirisDestination.Mixer -> MixerScreen(
                masterVolume = masterVolume,
                channels = mixerChannels,
                onMasterVolumeChange = { value ->
                    scope.launch { settingsStore.setMasterVolume(value) }
                },
                onChannelGainChange = { id, value ->
                    PlaybackController.setPackGain(id, value)
                    scope.launch { settingsStore.setPackGain(id, value) }
                },
                modifier = screenModifier,
            )

            OnirisDestination.Settings -> SettingsScreen(
                engineAvailable = PlaybackController.available,
                hasCredits = packs.any { it.credits.isNotEmpty() },
                onShowLicenses = { showLicenses = true },
                modifier = screenModifier,
            )
        }
    }

    val shell: @Composable () -> Unit = {
        Box(
            Modifier
                .fillMaxSize()
                .background(background),
        ) {
            Scaffold(
                modifier = Modifier.fillMaxSize(),
                containerColor = Color.Transparent,
                topBar = {
                    OnirisTopBar(
                        destination = destination,
                        onOpenNavigation = if (isWideLayout) {
                            null
                        } else {
                            { scope.launch { drawerState.open() } }
                        },
                    )
                },
                snackbarHost = { SnackbarHost(snackbarHostState) },
            ) { padding ->
                screens(
                    Modifier
                        .fillMaxSize()
                        .padding(padding),
                )
            }
        }
    }

    if (isWideLayout) {
        PermanentNavigationDrawer(
            drawerContent = {
                PermanentDrawerSheet(Modifier.width(PERMANENT_DRAWER_WIDTH_DP.dp)) {
                    OnirisDrawerContent(destination = destination, onNavigate = navigate)
                }
            },
            modifier = modifier.fillMaxSize(),
        ) {
            shell()
        }
    } else {
        ModalNavigationDrawer(
            drawerState = drawerState,
            drawerContent = {
                ModalDrawerSheet {
                    OnirisDrawerContent(destination = destination, onNavigate = navigate)
                }
            },
            modifier = modifier.fillMaxSize(),
        ) {
            shell()
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

/** Destination title and subtitle, plus the drawer button on compact widths. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun OnirisTopBar(
    destination: OnirisDestination,
    onOpenNavigation: (() -> Unit)?,
) {
    TopAppBar(
        title = {
            Column {
                Text(
                    text = stringResource(destination.labelRes),
                    style = MaterialTheme.typography.titleLarge,
                )
                Text(
                    text = stringResource(destination.subtitleRes),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        },
        navigationIcon = {
            if (onOpenNavigation != null) {
                IconButton(onClick = onOpenNavigation) {
                    Icon(
                        imageVector = Icons.Filled.Menu,
                        contentDescription = stringResource(R.string.action_open_navigation),
                    )
                }
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.Transparent),
    )
}

/** Shared drawer body for both the modal and the permanent sheet. */
@Composable
private fun ColumnScope.OnirisDrawerContent(
    destination: OnirisDestination,
    onNavigate: (OnirisDestination) -> Unit,
) {
    Spacer(Modifier.height(24.dp))
    Text(
        text = stringResource(R.string.app_name),
        style = MaterialTheme.typography.headlineSmall,
        color = MaterialTheme.colorScheme.primary,
        modifier = Modifier.padding(horizontal = 28.dp),
    )
    Text(
        text = stringResource(R.string.app_tagline),
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(horizontal = 28.dp, vertical = 4.dp),
    )
    Spacer(Modifier.height(16.dp))
    Column(
        Modifier
            .fillMaxWidth()
            .weight(1f)
            .verticalScroll(rememberScrollState()),
    ) {
        OnirisDestination.entries.forEach { item ->
            NavigationDrawerItem(
                label = { Text(stringResource(item.labelRes)) },
                icon = { Icon(imageVector = item.icon, contentDescription = null) },
                selected = item == destination,
                onClick = { onNavigate(item) },
                modifier = Modifier.padding(NavigationDrawerItemDefaults.ItemPadding),
            )
        }
    }
}

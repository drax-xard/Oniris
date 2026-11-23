package dev.oniris.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.NavigationDrawerItem
import androidx.compose.material3.NavigationDrawerItemDefaults
import androidx.compose.material3.PermanentDrawerSheet
import androidx.compose.material3.PermanentNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.material3.rememberDrawerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import dev.oniris.R
import dev.oniris.data.PackInfo
import dev.oniris.data.PackRepository
import dev.oniris.data.SettingsStore
import dev.oniris.playback.PackSelection
import dev.oniris.playback.PlaybackController
import dev.oniris.playback.PlaybackService
import dev.oniris.ui.navigation.OnirisDestination
import dev.oniris.ui.screens.LibraryScreen
import dev.oniris.ui.screens.MixerScreen
import dev.oniris.ui.screens.PlayScreen
import dev.oniris.ui.screens.SettingsScreen
import dev.oniris.ui.screens.TimerScreen
import kotlinx.coroutines.launch

/**
 * App shell: one screen per [OnirisDestination] behind a side navigation
 * drawer, plus the state and side effects shared by all screens.
 *
 * The drawer is permanent on tablets and tablets-in-landscape (>= 600 dp) and a
 * modal drawer behind a hamburger below that.
 */
@OptIn(ExperimentalMaterial3Api::class)
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
    var destination by rememberSaveable { mutableStateOf(OnirisDestination.Play) }
    val drawerState = rememberDrawerState(DrawerValue.Closed)

    val wideLayout = LocalConfiguration.current.screenWidthDp >= 600

    LaunchedEffect(refreshToken) {
        loading = true
        packs = repository.listPacks()
        loading = false
    }

    val current = settings
    val selectedIds = current?.selectedPackIds ?: emptySet()
    val packGains = current?.packGains ?: emptyMap()
    val selectedPacks = remember(packs, selectedIds) { packs.filter { it.id in selectedIds } }

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

    LaunchedEffect(packGains) {
        PlaybackController.setPackGains(packGains)
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

    val versionName = remember {
        runCatching {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName
        }.getOrNull()
    }

    val select: (OnirisDestination) -> Unit = { target ->
        destination = target
        if (!wideLayout) scope.launch { drawerState.close() }
    }

    BackHandler(enabled = drawerState.isOpen || destination != OnirisDestination.Play) {
        if (drawerState.isOpen) {
            scope.launch { drawerState.close() }
        } else {
            destination = OnirisDestination.Play
        }
    }

    val background = Brush.verticalGradient(
        listOf(
            MaterialTheme.colorScheme.background,
            MaterialTheme.colorScheme.surfaceVariant,
            MaterialTheme.colorScheme.background,
        ),
    )

    val content: @Composable () -> Unit = {
        Scaffold(
            modifier = modifier.fillMaxSize(),
            containerColor = MaterialTheme.colorScheme.background,
            snackbarHost = { SnackbarHost(snackbarHostState) },
            topBar = {
                TopAppBar(
                    title = {
                        Column {
                            Text(
                                text = stringResource(destination.labelRes),
                                style = MaterialTheme.typography.titleLarge,
                            )
                            Text(
                                text = stringResource(destination.subtitleRes),
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    },
                    navigationIcon = {
                        if (!wideLayout) {
                            IconButton(onClick = { scope.launch { drawerState.open() } }) {
                                Icon(
                                    Icons.Filled.Menu,
                                    contentDescription = stringResource(R.string.nav_open),
                                )
                            }
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.background,
                    ),
                )
            },
        ) { padding ->
            Box(
                Modifier
                    .fillMaxSize()
                    .background(background)
                    .padding(padding),
            ) {
                when (destination) {
                    OnirisDestination.Play -> PlayScreen(
                        packNames = selectedPacks.map { it.name },
                        isPlaying = isPlaying,
                        available = PlaybackController.available,
                        decodedBytes = PlaybackController.decodedBytes(),
                        timerMinutes = current?.timerMinutes ?: 0,
                        onPlayPause = {
                            if (isPlaying) {
                                PlaybackService.pause(context)
                            } else {
                                PlaybackService.play(context)
                            }
                        },
                        onStop = { PlaybackService.stop(context) },
                        onOpenLibrary = { select(OnirisDestination.Library) },
                    )

                    OnirisDestination.Library -> LibraryScreen(
                        packs = packs,
                        loading = loading,
                        selectedIds = selectedIds,
                        onSelectedChange = { id, checked ->
                            scope.launch {
                                val next = if (checked) selectedIds + id else selectedIds - id
                                settingsStore.setSelectedPacks(next)
                            }
                        },
                        onRefresh = { refreshToken += 1 },
                    )

                    OnirisDestination.Mixer -> MixerScreen(
                        activePacks = selectedPacks,
                        masterVolume = current?.masterVolume ?: 0.8f,
                        packGains = packGains,
                        onMasterVolumeChange = { value ->
                            scope.launch { settingsStore.setMasterVolume(value) }
                        },
                        onPackGainChange = { id, gain ->
                            scope.launch { settingsStore.setPackGain(id, gain) }
                        },
                        onOpenLibrary = { select(OnirisDestination.Library) },
                    )

                    OnirisDestination.Timer -> TimerScreen(
                        timerMinutes = current?.timerMinutes ?: 0,
                        onTimerChange = { minutes ->
                            scope.launch { settingsStore.setTimerMinutes(minutes) }
                        },
                    )

                    OnirisDestination.Settings -> SettingsScreen(
                        engineAvailable = PlaybackController.available,
                        decodedBytes = PlaybackController.decodedBytes(),
                        installedPacks = packs.size,
                        versionName = versionName,
                    )
                }
            }
        }
    }

    val drawer: @Composable () -> Unit = {
        Column(Modifier.padding(vertical = 24.dp)) {
            Column(Modifier.padding(horizontal = 28.dp)) {
                Text(
                    text = stringResource(R.string.app_name),
                    style = MaterialTheme.typography.displaySmall,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(
                    text = stringResource(R.string.drawer_tagline),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.height(24.dp))
            OnirisDestination.entries.forEach { item ->
                NavigationDrawerItem(
                    label = { Text(stringResource(item.labelRes)) },
                    icon = { Icon(item.icon, contentDescription = null) },
                    selected = item == destination,
                    onClick = { select(item) },
                    modifier = Modifier.padding(NavigationDrawerItemDefaults.ItemPadding),
                )
            }
        }
    }

    if (wideLayout) {
        PermanentNavigationDrawer(
            drawerContent = {
                PermanentDrawerSheet(Modifier.width(280.dp)) { drawer() }
            },
        ) {
            content()
        }
    } else {
        ModalNavigationDrawer(
            drawerState = drawerState,
            drawerContent = {
                ModalDrawerSheet { drawer() }
            },
        ) {
            content()
        }
    }
}

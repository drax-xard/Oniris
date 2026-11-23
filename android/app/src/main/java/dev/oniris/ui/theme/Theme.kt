package dev.oniris.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val Night = Color(0xFF0B1210)
private val NightSurface = Color(0xFF111A17)
private val NightSurfaceHigh = Color(0xFF1B2622)
private val NightSurfaceHighest = Color(0xFF232F2B)
private val NightSurfaceLow = Color(0xFF0E1614)
private val Moon = Color(0xFF9CC7B8)
private val MoonDeep = Color(0xFF24443B)
private val MoonPale = Color(0xFFC7E9DD)
private val Mist = Color(0xFFE2EAE6)
private val MistDim = Color(0xFFB6C8BE)
private val NightOnMoon = Color(0xFF10201B)

/**
 * Always-dark palette. Every Material 3 role is set explicitly: the drawer
 * sheets, filter chips, sliders, and selected navigation items all read from
 * `surfaceContainer*`, `secondaryContainer`, and `outlineVariant`, and would
 * otherwise fall back to the default purple scheme.
 */
private val colors = darkColorScheme(
    primary = Moon,
    onPrimary = NightOnMoon,
    primaryContainer = MoonDeep,
    onPrimaryContainer = MoonPale,
    inversePrimary = MoonDeep,
    secondary = MistDim,
    onSecondary = NightOnMoon,
    secondaryContainer = Color(0xFF1F3A32),
    onSecondaryContainer = MoonPale,
    tertiary = Color(0xFFA7BFA0),
    onTertiary = NightOnMoon,
    tertiaryContainer = Color(0xFF29402F),
    onTertiaryContainer = Color(0xFFDDEBD5),
    background = Night,
    onBackground = Mist,
    surface = NightSurface,
    onSurface = Mist,
    surfaceVariant = NightSurfaceHigh,
    onSurfaceVariant = MistDim,
    surfaceTint = Moon,
    inverseSurface = Mist,
    inverseOnSurface = Night,
    error = Color(0xFFF2B8B5),
    onError = Color(0xFF601410),
    errorContainer = Color(0xFF8C1D18),
    onErrorContainer = Color(0xFFF9DEDC),
    outline = Color(0xFF5E756C),
    outlineVariant = Color(0xFF33453D),
    scrim = Color(0xFF000000),
    surfaceBright = NightSurfaceHighest,
    surfaceDim = Night,
    surfaceContainerLowest = Night,
    surfaceContainerLow = NightSurfaceLow,
    surfaceContainer = Color(0xFF0F1917),
    surfaceContainerHigh = NightSurfaceHigh,
    surfaceContainerHighest = NightSurfaceHighest,
)

@Composable
fun OnirisTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = colors, content = content)
}

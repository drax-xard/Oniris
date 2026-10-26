package dev.oniris.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val Night = Color(0xFF0B1210)
private val NightSurface = Color(0xFF111A17)
private val NightSurfaceHigh = Color(0xFF1B2622)
private val Moon = Color(0xFF9CC7B8)
private val MoonDeep = Color(0xFF24443B)
private val Mist = Color(0xFFE2EAE6)
private val MistDim = Color(0xFFB6C8BE)
private val NightOnMoon = Color(0xFF10201B)

private val colors = darkColorScheme(
    primary = Moon,
    onPrimary = NightOnMoon,
    primaryContainer = MoonDeep,
    onPrimaryContainer = Color(0xFFC7E9DD),
    secondary = MistDim,
    onSecondary = NightOnMoon,
    background = Night,
    onBackground = Mist,
    surface = NightSurface,
    onSurface = Mist,
    surfaceVariant = NightSurfaceHigh,
    onSurfaceVariant = MistDim,
    outline = Color(0xFF5E756C),
)

@Composable
fun OnirisTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = colors, content = content)
}

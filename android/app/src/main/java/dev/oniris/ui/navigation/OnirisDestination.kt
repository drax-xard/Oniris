package dev.oniris.ui.navigation

import androidx.annotation.StringRes
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Star
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.PathBuilder
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp
import dev.oniris.R

/**
 * Three rails with a knob on each, drawn here because the core icon set has no
 * mixer glyph. Like every vector icon it is tinted by the host `Icon`.
 */
private val mixerIcon: ImageVector = ImageVector.Builder(
    name = "OnirisMixer",
    defaultWidth = 24.dp,
    defaultHeight = 24.dp,
    viewportWidth = 24f,
    viewportHeight = 24f,
).apply {
    path(
        stroke = SolidColor(Color.Black),
        strokeLineWidth = 1.8f,
        strokeLineCap = StrokeCap.Round,
    ) {
        moveTo(4f, 6f)
        lineTo(20f, 6f)
        moveTo(4f, 12f)
        lineTo(20f, 12f)
        moveTo(4f, 18f)
        lineTo(20f, 18f)
    }
    path(fill = SolidColor(Color.Black)) {
        knob(left = 7f, top = 4.2f)
        knob(left = 13.4f, top = 10.2f)
        knob(left = 9.6f, top = 16.2f)
    }
}.build()

/** A 2.6 x 3.6 slider knob whose top-left corner is at ([left], [top]). */
private fun PathBuilder.knob(left: Float, top: Float) {
    moveTo(left, top)
    lineTo(left + 2.6f, top)
    lineTo(left + 2.6f, top + 3.6f)
    lineTo(left, top + 3.6f)
    close()
}

/** The five top-level surfaces of the app, in drawer order. */
enum class OnirisDestination(
    @StringRes val labelRes: Int,
    @StringRes val subtitleRes: Int,
    val icon: ImageVector,
) {
    Play(
        labelRes = R.string.destination_play,
        subtitleRes = R.string.destination_play_subtitle,
        icon = Icons.Filled.PlayArrow,
    ),
    Presets(
        labelRes = R.string.destination_presets,
        subtitleRes = R.string.destination_presets_subtitle,
        icon = Icons.Filled.Star,
    ),
    Library(
        labelRes = R.string.destination_library,
        subtitleRes = R.string.destination_library_subtitle,
        icon = Icons.AutoMirrored.Filled.List,
    ),
    Mixer(
        labelRes = R.string.destination_mixer,
        subtitleRes = R.string.destination_mixer_subtitle,
        icon = mixerIcon,
    ),
    Settings(
        labelRes = R.string.destination_settings,
        subtitleRes = R.string.destination_settings_subtitle,
        icon = Icons.Filled.Settings,
    ),
}

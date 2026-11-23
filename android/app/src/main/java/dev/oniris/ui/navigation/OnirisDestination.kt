package dev.oniris.ui.navigation

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Settings
import androidx.annotation.StringRes
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp
import dev.oniris.R

/**
 * Top-level sections of the app. One screen each, selected from the side
 * navigation drawer. Labels are string resources so they follow the UI locale.
 */
enum class OnirisDestination(
    @StringRes val labelRes: Int,
    @StringRes val subtitleRes: Int,
    val icon: ImageVector,
) {
    Play(R.string.nav_play, R.string.nav_play_subtitle, Icons.Filled.PlayArrow),
    Library(R.string.nav_library, R.string.nav_library_subtitle, Icons.AutoMirrored.Filled.List),
    Mixer(R.string.nav_mixer, R.string.nav_mixer_subtitle, OnirisIcons.Mixer),
    Timer(R.string.nav_timer, R.string.nav_timer_subtitle, Icons.Filled.DateRange),
    Settings(R.string.nav_settings, R.string.nav_settings_subtitle, Icons.Filled.Settings),
}

/**
 * Icons that `material-icons-core` does not ship. Drawn as a [ImageVector] so
 * the app does not pull in `material-icons-extended` for a handful of glyphs.
 */
object OnirisIcons {
    /** Three rails with slider knobs. */
    val Mixer: ImageVector by lazy {
        ImageVector.Builder(
            name = "OnirisMixer",
            defaultWidth = 24.dp,
            defaultHeight = 24.dp,
            viewportWidth = 24f,
            viewportHeight = 24f,
        ).apply {
            rail(6f)
            rail(12f)
            rail(18f)
            knob(8f, 6f)
            knob(15f, 12f)
            knob(11f, 18f)
        }.build()
    }

    private fun ImageVector.Builder.rail(y: Float) = path(
        stroke = SolidColor(Color.Black),
        strokeLineWidth = 1.8f,
        strokeLineCap = StrokeCap.Round,
    ) {
        moveTo(4f, y)
        lineTo(20f, y)
    }

    private fun ImageVector.Builder.knob(x: Float, y: Float) = path(fill = SolidColor(Color.Black)) {
        moveTo(x - 1.7f, y - 2.8f)
        lineTo(x + 1.7f, y - 2.8f)
        lineTo(x + 1.7f, y + 2.8f)
        lineTo(x - 1.7f, y + 2.8f)
        close()
    }
}

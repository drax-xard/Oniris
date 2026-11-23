package dev.oniris.ui.navigation

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Settings
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.path
import androidx.compose.ui.unit.dp

/**
 * Top-level sections of the app. One screen each, selected from the side
 * navigation drawer.
 */
enum class OnirisDestination(
    val label: String,
    val subtitle: String,
    val icon: ImageVector,
) {
    Play("Play", "Transport and current mix", Icons.Filled.PlayArrow),
    Library("Library", "Choose sound packs", Icons.AutoMirrored.Filled.List),
    Mixer("Mixer", "Balance the active packs", OnirisIcons.Mixer),
    Timer("Timer", "Let it stop on its own", Icons.Filled.DateRange),
    Settings("Settings", "Engine and app details", Icons.Filled.Settings),
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

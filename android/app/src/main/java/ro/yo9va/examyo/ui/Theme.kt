package ro.yo9va.examyo.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val Green = Color(0xFF2EA050)
val Red = Color(0xFFD2463C)

private val LightColors = lightColorScheme(
    primary = Color(0xFF205CAA),
    onPrimary = Color.White,
    primaryContainer = Color(0xFFD6E3F7),
    onPrimaryContainer = Color(0xFF0B2A52),
    secondaryContainer = Color(0xFFDDE6F2),
)

private val DarkColors = darkColorScheme(
    primary = Color(0xFF9CC2F5),
    onPrimary = Color(0xFF0B2A52),
    primaryContainer = Color(0xFF1D4A82),
    onPrimaryContainer = Color(0xFFD6E3F7),
)

@Composable
fun ExamTheme(theme: String?, content: @Composable () -> Unit) {
    val dark = when (theme) {
        "light" -> false
        "dark" -> true
        else -> isSystemInDarkTheme()
    }
    MaterialTheme(colorScheme = if (dark) DarkColors else LightColors, content = content)
}

@Composable
fun isDark(): Boolean = MaterialTheme.colorScheme.background.let { it.red + it.green + it.blue < 1.5f }

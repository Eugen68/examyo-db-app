package ro.yo9va.examyo

import android.graphics.Bitmap
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.ExperimentalTestApi
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performTextInput
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import java.io.File

/**
 * Parcurge aplicația ca un utilizator și salvează capturi de ecran
 * (în CI ajung pe ramura „capturi-android”, ca să poată fi verificate fără telefon).
 */
@RunWith(AndroidJUnit4::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [34], qualifiers = "w411dp-h891dp-xxhdpi")
@OptIn(ExperimentalTestApi::class)
class ScreenshotTest {
    @get:Rule
    val rule = createAndroidComposeRule<MainActivity>()

    private val dir = File(System.getProperty("screenshots.dir") ?: "build/screenshots").apply { mkdirs() }

    private fun shot(name: String) {
        rule.waitForIdle()
        val bmp = rule.onRoot().captureToImage().asAndroidBitmap()
        File(dir, "$name.png").outputStream().use { bmp.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    @Test
    fun walkThrough() {
        rule.waitUntilAtLeastOneExists(hasText("Adaugă și începe"), timeoutMillis = 30_000)
        shot("01-profiluri")

        rule.onNodeWithText("Nume").performTextInput("Eugen")
        rule.onNodeWithText("Indicativ (opțional)").performTextInput("yo9va")
        rule.onNodeWithText("Adaugă și începe").performClick()
        shot("02-acasa")

        rule.onNodeWithText("Clasa I").performClick()
        shot("03-probe")

        rule.onAllNodesWithText("Începe").onFirst().performClick()
        shot("04-intrebare")

        rule.onNodeWithText("A)").performClick()
        shot("05-selectat")
        rule.onNodeWithText("Verifică").performClick()
        rule.onNodeWithText("Rezolvarea").performClick()
        shot("06-rezolvare")

        rule.onNodeWithText("Următoarea ▶").performClick()
        shot("07-urmatoarea")

        rule.onNodeWithText("←").performClick()
        rule.waitUntilAtLeastOneExists(hasText("Statisticile mele"), timeoutMillis = 5_000)
        rule.onNodeWithText("Statisticile mele").performScrollTo().performClick()
        shot("08-statistici")

        rule.onNodeWithText("←").performClick()
        rule.onNodeWithText("←").performClick()
        rule.waitUntilAtLeastOneExists(hasText("întunecată"), timeoutMillis = 5_000)
        rule.onNodeWithText("întunecată").performScrollTo().performClick()
        shot("09-profiluri-intunecat")
    }
}

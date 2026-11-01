package ro.yo9va.examyo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import ro.yo9va.examyo.ui.ExamApp

class MainActivity : ComponentActivity() {
    private val vm: ExamViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { ExamApp(vm) }
    }
}

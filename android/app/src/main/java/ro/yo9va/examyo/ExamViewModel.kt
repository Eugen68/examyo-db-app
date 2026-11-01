package ro.yo9va.examyo

import android.app.Application
import android.net.Uri
import android.os.SystemClock
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import ro.yo9va.examyo.data.Bank
import ro.yo9va.examyo.data.ExamClass
import ro.yo9va.examyo.data.Probe
import ro.yo9va.examyo.data.Profile
import ro.yo9va.examyo.data.STORE_FILE
import ro.yo9va.examyo.data.Store
import ro.yo9va.examyo.data.StoreFile
import ro.yo9va.examyo.data.now
import java.io.File

sealed interface Screen {
    data object Profiles : Screen
    data class Home(val pid: String) : Screen
    data class Practice(
        val pid: String,
        val cls: ExamClass,
        val probe: Probe,
        val wrongOnly: Boolean,
        val list: List<Int>,
        val pos: Int,
        /** order[poziție afișată] = indicele original al răspunsului. */
        val order: List<Int> = shuffled(),
        val selected: Int? = null,
        val answered: Boolean = false,
        val showExplanation: Boolean = false,
    ) : Screen

    data class Done(val pid: String, val cls: ExamClass, val probe: Probe, val wrongOnly: Boolean, val count: Int) : Screen
    data class Stats(val pid: String, val cls: ExamClass?) : Screen
    data object AllStats : Screen
}

private fun shuffled() = listOf(0, 1, 2, 3).shuffled()

class ExamViewModel(app: Application) : AndroidViewModel(app) {
    private val file = StoreFile(File(app.filesDir, STORE_FILE))

    var bank by mutableStateOf<Bank?>(null)
        private set
    var store by mutableStateOf(Store())
        private set
    var screen by mutableStateOf<Screen>(Screen.Profiles)
        private set
    var message by mutableStateOf<String?>(null)

    private var lastAction = SystemClock.elapsedRealtime()

    init {
        val (s, warning) = file.load()
        store = s
        message = warning
        viewModelScope.launch {
            bank = withContext(Dispatchers.Default) {
                Bank.load(app.assets.open("all.json").bufferedReader().use { it.readText() })
            }
        }
    }

    private fun update(newStore: Store) {
        store = newStore
        try {
            file.save(newStore)
        } catch (e: Exception) {
            message = e.message ?: "Nu pot salva progresul."
        }
    }

    /** Secunde de la ultima acțiune; pauzele lungi sunt plafonate la salvare. */
    private fun touch(): Long {
        val t = SystemClock.elapsedRealtime()
        val s = (t - lastAction) / 1000
        lastAction = t
        return s
    }

    // ---------------------------------------------------------------- profiluri

    fun openProfile(id: String) {
        update(store.copy(lastProfile = id))
        screen = Screen.Home(id)
    }

    fun addProfile(name: String, callsign: String) {
        if (name.isBlank()) return
        val p = Profile.new(name, callsign, now())
        update(store.copy(profiles = store.profiles + p))
        openProfile(p.id)
    }

    fun deleteProfile(id: String) {
        update(
            store.copy(
                profiles = store.profiles.filter { it.id != id },
                lastProfile = store.lastProfile.takeIf { it != id },
            )
        )
    }

    fun setClass(pid: String, c: ExamClass) = update(store.updateProfile(pid) { it.copy(lastClass = c) })

    fun setTheme(theme: String) = update(store.copy(theme = theme))

    fun showStats(pid: String) {
        screen = Screen.Stats(pid, store.profile(pid)?.lastClass)
    }

    fun showStatsClass(pid: String, c: ExamClass) {
        screen = Screen.Stats(pid, c)
    }

    fun showAllStats() {
        screen = Screen.AllStats
    }

    fun goHome(pid: String) {
        screen = Screen.Home(pid)
    }

    fun back() {
        screen = when (val s = screen) {
            Screen.Profiles -> Screen.Profiles
            is Screen.Home -> Screen.Profiles
            is Screen.Practice -> Screen.Home(s.pid)
            is Screen.Done -> Screen.Home(s.pid)
            is Screen.Stats -> Screen.Home(s.pid)
            Screen.AllStats -> Screen.Profiles
        }
    }

    // ---------------------------------------------------------------- parcurgere

    fun start(pid: String, c: ExamClass, p: Probe, fromStart: Boolean, wrongOnly: Boolean) {
        val bank = bank ?: return
        val profile = store.profile(pid) ?: return
        val list: List<Int>
        var pos = 0
        if (wrongOnly) {
            list = profile.wrong(bank, c, p)
        } else {
            list = bank.set(c, p)
            pos = if (fromStart) 0 else profile.position(c, p)
            if (pos >= list.size) pos = 0
            update(store.updateProfile(pid) { it.withPosition(c, p, pos) })
        }
        if (list.isEmpty()) return
        touch()
        screen = Screen.Practice(pid, c, p, wrongOnly, list, pos)
    }

    fun select(displayIndex: Int) {
        val s = screen as? Screen.Practice ?: return
        if (!s.answered) screen = s.copy(selected = displayIndex)
    }

    fun verify() {
        val s = screen as? Screen.Practice ?: return
        val bank = bank ?: return
        val sel = s.selected ?: return
        if (s.answered) return
        val q = bank.questions[s.list[s.pos]]
        val correct = s.order[sel] == q.correct
        val secs = touch()
        update(store.updateProfile(s.pid) {
            val r = it.record(q.key, correct, now()).addTime(secs)
            if (s.wrongOnly) r else r.withPosition(s.cls, s.probe, s.pos + 1)
        })
        screen = s.copy(answered = true)
    }

    fun toggleExplanation() {
        val s = screen as? Screen.Practice ?: return
        if (s.answered) screen = s.copy(showExplanation = !s.showExplanation)
    }

    fun next() {
        val s = screen as? Screen.Practice ?: return
        if (s.pos + 1 >= s.list.size) {
            if (!s.wrongOnly) update(store.updateProfile(s.pid) { it.withPosition(s.cls, s.probe, s.list.size) })
            screen = Screen.Done(s.pid, s.cls, s.probe, s.wrongOnly, s.list.size)
        } else {
            goTo(s, s.pos + 1)
        }
    }

    fun prev() {
        val s = screen as? Screen.Practice ?: return
        if (s.pos > 0) goTo(s, s.pos - 1)
    }

    private fun goTo(s: Screen.Practice, pos: Int) {
        val secs = touch()
        update(store.updateProfile(s.pid) {
            val r = it.addTime(secs)
            if (s.wrongOnly) r else r.withPosition(s.cls, s.probe, pos)
        })
        screen = Screen.Practice(s.pid, s.cls, s.probe, s.wrongOnly, s.list, pos)
    }

    // ---------------------------------------------------------------- export / import

    fun exportTo(uri: Uri) {
        try {
            getApplication<Application>().contentResolver.openOutputStream(uri, "wt")!!.use {
                it.write(Store.json.encodeToString(Store.serializer(), store).toByteArray())
            }
            message = "Progresul a fost exportat."
        } catch (e: Exception) {
            message = "Exportul nu a reușit: ${e.message}"
        }
    }

    fun importFrom(uri: Uri) {
        try {
            val text = getApplication<Application>().contentResolver.openInputStream(uri)!!.use {
                it.readBytes().toString(Charsets.UTF_8)
            }
            val other = Store.parse(text)
            update(store.merge(other))
            message = "Am importat ${other.profiles.size} utilizator(i). La cei existenți a rămas varianta cea mai recentă."
        } catch (e: Exception) {
            message = "Fișierul nu e un progres ExamYO valid."
        }
    }
}

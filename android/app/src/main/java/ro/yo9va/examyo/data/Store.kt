package ro.yo9va.examyo.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File
import kotlin.random.Random

// Port al src/store.rs. Formatul JSON e identic cu ExamYO-date.json de pe Windows.

const val STORE_FILE = "ExamYO-date.json"

/** O pauză mai lungă de atât între două acțiuni nu intră în timpul de studiu. */
const val MAX_IDLE_SECS = 300L

@Serializable
data class QStat(
    val attempts: Int = 0,
    val correct: Int = 0,
    @SerialName("first_correct") val firstCorrect: Boolean = false,
    @SerialName("last_correct") val lastCorrect: Boolean = false,
    @SerialName("last_ts") val lastTs: Long = 0,
)

@Serializable
data class Profile(
    val id: String,
    val name: String,
    val callsign: String = "",
    val created: Long = 0,
    @SerialName("last_active") val lastActive: Long = 0,
    @SerialName("last_class") val lastClass: ExamClass? = null,
    @SerialName("time_secs") val timeSecs: Long = 0,
    val answers: Map<String, QStat> = emptyMap(),
    val positions: Map<String, Int> = emptyMap(),
) {
    val displayName: String get() = if (callsign.isEmpty()) name else "$name ($callsign)"

    fun record(key: String, correct: Boolean, now: Long): Profile {
        val old = answers[key] ?: QStat(firstCorrect = correct)
        val s = old.copy(
            attempts = old.attempts + 1,
            correct = old.correct + if (correct) 1 else 0,
            lastCorrect = correct,
            lastTs = now,
        )
        return copy(answers = answers + (key to s), lastActive = now)
    }

    fun addTime(secs: Long): Profile = copy(timeSecs = timeSecs + secs.coerceIn(0, MAX_IDLE_SECS))

    fun position(c: ExamClass, p: Probe): Int = positions[positionKey(c, p)] ?: 0

    fun withPosition(c: ExamClass, p: Probe, pos: Int): Profile =
        copy(positions = positions + (positionKey(c, p) to pos))

    fun stats(bank: Bank, c: ExamClass, p: Probe): Stats = Stats.of(this, bank, bank.set(c, p))

    fun classStats(bank: Bank, c: ExamClass): Stats {
        val seen = HashSet<String>()
        val all = ArrayList<Int>()
        for (p in c.probes) for (i in bank.set(c, p)) if (seen.add(bank.questions[i].key)) all += i
        return Stats.of(this, bank, all)
    }

    /** Întrebările din probă la care ultimul răspuns a fost greșit. */
    fun wrong(bank: Bank, c: ExamClass, p: Probe): List<Int> =
        bank.set(c, p).filter { i -> answers[bank.questions[i].key]?.let { !it.lastCorrect } == true }

    companion object {
        fun new(name: String, callsign: String, now: Long) = Profile(
            id = "%x%04x".format(now, Random.nextInt(0, 0x10000)),
            name = name.trim(),
            callsign = callsign.trim().uppercase(),
            created = now,
            lastActive = now,
        )
    }
}

fun positionKey(c: ExamClass, p: Probe) = "${c.name}/${p.name}"

data class Stats(val total: Int, val seen: Int, val firstCorrect: Int, val wrongNow: Int) {
    val firstCorrectPct: Float? get() = if (seen > 0) 100f * firstCorrect / seen else null
    val seenFraction: Float get() = if (total == 0) 0f else seen.toFloat() / total

    companion object {
        fun of(p: Profile, bank: Bank, set: List<Int>): Stats {
            var seen = 0
            var first = 0
            var wrong = 0
            for (i in set) {
                val q = p.answers[bank.questions[i].key] ?: continue
                seen++
                if (q.firstCorrect) first++
                if (!q.lastCorrect) wrong++
            }
            return Stats(set.size, seen, first, wrong)
        }
    }
}

@Serializable
data class Store(
    val version: Int = 1,
    val profiles: List<Profile> = emptyList(),
    @SerialName("last_profile") val lastProfile: String? = null,
    val theme: String? = null,
) {
    fun profile(id: String) = profiles.firstOrNull { it.id == id }

    fun updateProfile(id: String, f: (Profile) -> Profile): Store =
        copy(profiles = profiles.map { if (it.id == id) f(it) else it })

    /**
     * Combină cu progresul adus de pe alt dispozitiv: profilurile noi se adaugă,
     * iar la cele comune rămâne varianta cu activitatea cea mai recentă.
     */
    fun merge(other: Store): Store {
        val byId = LinkedHashMap<String, Profile>()
        for (p in profiles) byId[p.id] = p
        for (p in other.profiles) {
            val mine = byId[p.id]
            if (mine == null || p.lastActive > mine.lastActive) byId[p.id] = p
        }
        return copy(profiles = byId.values.toList())
    }

    companion object {
        val json = Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
            prettyPrint = true
        }

        fun parse(text: String): Store = json.decodeFromString(serializer(), text)
    }
}

/** Citirea și scrierea fișierului de progres. */
class StoreFile(private val file: File) {
    /** Dacă fișierul e stricat, îl păstrează sub alt nume și pornește de la zero, cu un avertisment. */
    fun load(): Pair<Store, String?> {
        if (!file.exists()) return Store() to null
        return try {
            Store.parse(file.readText()) to null
        } catch (e: Exception) {
            val backup = File(file.parentFile, file.nameWithoutExtension + ".stricat-${now()}.json")
            file.renameTo(backup)
            Store() to "Fișierul de progres era deteriorat. L-am păstrat ca ${backup.name} și am pornit de la zero."
        }
    }

    fun save(store: Store) {
        val tmp = File(file.parentFile, file.name + ".tmp")
        tmp.writeText(Store.json.encodeToString(Store.serializer(), store))
        if (!tmp.renameTo(file)) {
            file.delete()
            check(tmp.renameTo(file)) { "Nu pot salva progresul." }
        }
    }
}

fun now(): Long = System.currentTimeMillis() / 1000

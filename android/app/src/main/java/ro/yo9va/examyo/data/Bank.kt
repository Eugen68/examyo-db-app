package ro.yo9va.examyo.data

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.util.Locale

// Port în Kotlin al src/data.rs din varianta de Windows. Cheile întrebărilor trebuie să fie
// identice cu cele din Rust, ca fișierul de progres să poată fi mutat între PC și telefon.

enum class ExamClass(val roman: String) {
    I("I"), II("II"), III("III"), IV("IV");

    val title: String get() = "Clasa $roman"

    /** Probele examenului pentru clasă (clasa a IV-a nu are radiotehnică). */
    val probes: List<Probe>
        get() = if (this == IV) listOf(Probe.Legislatie, Probe.Sanctiuni, Probe.Operare, Probe.Ntsm)
        else Probe.entries
}

/** Numele constantelor trebuie să rămână ca în Rust: apar în cheile de poziție („I/Radiotehnica”). */
enum class Probe(val title: String) {
    Radiotehnica("Radiotehnică"),
    Legislatie("Legislație"),
    Sanctiuni("Sancțiuni"),
    Operare("Reguli de trafic"),
    Ntsm("Protecția muncii"),
}

private data class Part(val slug: String, val level: String)

/** Din ce baze se compune o probă, în ordinea de parcurgere: întâi clasa III, apoi II, apoi I. */
private fun composition(c: ExamClass, p: Probe): List<Part> {
    val r3 = Part("radiotehnica-clasa-3", "Clasa III")
    val r2 = Part("radiotehnica-clasa-2", "Clasa II")
    val r1 = Part("radiotehnica-clasa-1", "Clasa I")
    val l3 = Part("legislatie-clasa-3", "Clasa III")
    val l12 = Part("legislatie-clasa-1-2", "Clasa I–II")
    val l4 = Part("legislatie-clasa-4", "Clasa IV")
    val o3 = Part("operare-clasa-3", "Clasa III")
    val o12 = Part("operare-clasa-1-2", "Clasa I–II")
    val o4 = Part("operare-clasa-4", "Clasa IV")
    val n13 = Part("ntsm-clasa-1-3", "")
    val n4 = Part("ntsm-clasa-4", "")
    val s = Part("sanctiuni", "")
    return when (p) {
        Probe.Radiotehnica -> when (c) {
            ExamClass.I -> listOf(r3, r2, r1)
            ExamClass.II -> listOf(r3, r2)
            ExamClass.III -> listOf(r3)
            ExamClass.IV -> emptyList()
        }
        Probe.Legislatie -> when (c) {
            ExamClass.I, ExamClass.II -> listOf(l3, l12)
            ExamClass.III -> listOf(l3)
            ExamClass.IV -> listOf(l4)
        }
        Probe.Operare -> when (c) {
            ExamClass.I, ExamClass.II -> listOf(o3, o12)
            ExamClass.III -> listOf(o3)
            ExamClass.IV -> listOf(o4)
        }
        Probe.Ntsm -> if (c == ExamClass.IV) listOf(n4) else listOf(n13)
        Probe.Sanctiuni -> listOf(s)
    }
}

data class Figure(val name: String, val width: Int?)

data class Question(
    val key: String,
    val sourceId: String,
    val level: String,
    val ancomCode: String?,
    val text: String,
    val figure: Figure?,
    val answers: List<String>,
    val correct: Int,
    val explanation: String?,
    val explanationFigure: Figure?,
    val contributor: String?,
    val explanationAuthor: String?,
)

@Serializable
private class RawAll(val source: RawSource, val databases: List<RawDb>)

@Serializable
private class RawSource(@SerialName("upstream_commit") val upstreamCommit: String? = null)

@Serializable
private class RawDb(val slug: String, val questions: List<RawQ>)

@Serializable
private class RawFig(val file: String, val width: Int? = null)

@Serializable
private class RawQ(
    val id: String,
    val question: String,
    val image: RawFig? = null,
    val answers: List<String>,
    @SerialName("correct_index") val correctIndex: Int,
    val explanation: String? = null,
    @SerialName("explanation_image") val explanationImage: RawFig? = null,
    val contributor: String? = null,
    @SerialName("explanation_author") val explanationAuthor: String? = null,
)

private fun RawFig?.toFigure() = this?.let { Figure(it.file.removePrefix("images/"), it.width) }

class Bank private constructor(
    val questions: List<Question>,
    private val sets: Map<Pair<ExamClass, Probe>, List<Int>>,
    val upstreamCommit: String?,
) {
    /** Indicii întrebărilor dintr-o probă, în ordinea de parcurgere, fără dubluri. */
    fun set(c: ExamClass, p: Probe): List<Int> = sets[c to p].orEmpty()

    companion object {
        private val json = Json { ignoreUnknownKeys = true }

        fun levels(c: ExamClass, p: Probe): List<String> =
            composition(c, p).map { it.level }.filter { it.isNotEmpty() }

        fun load(text: String): Bank {
            val raw = json.decodeFromString<RawAll>(text)
            val questions = ArrayList<Question>()
            val bySlug = HashMap<String, List<Int>>()
            for (db in raw.databases) {
                val idx = ArrayList<Int>(db.questions.size)
                for (q in db.questions) {
                    require(q.answers.size == 4) { "${q.id}: nu are exact 4 răspunsuri" }
                    require(q.correctIndex in 0..3) { "${q.id}: răspuns corect invalid" }
                    val (code, body) = splitAncomCode(q.question)
                    idx += questions.size
                    questions += Question(
                        key = questionKey(body, q.answers),
                        sourceId = q.id,
                        level = "",
                        ancomCode = code,
                        text = body,
                        figure = q.image.toFigure(),
                        answers = q.answers,
                        correct = q.correctIndex,
                        explanation = q.explanation,
                        explanationFigure = q.explanationImage.toFigure(),
                        contributor = q.contributor,
                        explanationAuthor = q.explanationAuthor,
                    )
                }
                bySlug[db.slug] = idx
            }

            val sets = HashMap<Pair<ExamClass, Probe>, List<Int>>()
            for (c in ExamClass.entries) {
                for (p in c.probes) {
                    val seen = HashSet<String>()
                    val list = ArrayList<Int>()
                    for (part in composition(c, p)) {
                        val ids = bySlug[part.slug] ?: error("Lipsește baza „${part.slug}” din date")
                        for (i in ids) {
                            if (seen.add(questions[i].key)) {
                                list += i
                                if (questions[i].level.isEmpty() && part.level.isNotEmpty()) {
                                    questions[i] = questions[i].copy(level = part.level)
                                }
                            }
                        }
                    }
                    sets[c to p] = list
                }
            }
            return Bank(questions, sets, raw.source.upstreamCommit)
        }
    }
}

/** „01A11/ Rigiditatea …” → ("01A11", "Rigiditatea …"). Aceeași regulă ca în Rust. */
fun splitAncomCode(text: String): Pair<String?, String> {
    var i = 0
    fun digits(max: Int): Int {
        val s = i
        while (i < text.length && i - s < max && text[i] in '0'..'9') i++
        return i - s
    }
    val ok = digits(2) == 2 &&
        i < text.length && text[i] in 'A'..'Z' &&
        run { i++; digits(3) >= 1 } &&
        run {
            if (i < text.length && text[i] in 'a'..'z') i++
            i < text.length && text[i] == '/'
        }
    return if (ok) text.substring(0, i) to text.substring(i + 1).trimStart() else null to text
}

private fun isAlnum(cp: Int): Boolean {
    // Echivalentul lui char::is_alphanumeric din Rust (Alphabetic sau Nd/Nl/No).
    if (Character.isAlphabetic(cp)) return true
    return when (Character.getType(cp).toByte()) {
        Character.DECIMAL_DIGIT_NUMBER, Character.LETTER_NUMBER, Character.OTHER_NUMBER -> true
        else -> false
    }
}

private fun normalize(s: String, out: StringBuilder) {
    var i = 0
    while (i < s.length) {
        var cp = s.codePointAt(i)
        i += Character.charCount(cp)
        cp = when (cp) {
            'ă'.code, 'â'.code, 'Ă'.code, 'Â'.code -> 'a'.code
            'î'.code, 'Î'.code -> 'i'.code
            'ș'.code, 'ş'.code, 'Ș'.code, 'Ş'.code -> 's'.code
            'ț'.code, 'ţ'.code, 'Ț'.code, 'Ţ'.code -> 't'.code
            else -> cp
        }
        val lower = String(Character.toChars(cp)).lowercase(Locale.ROOT)
        var j = 0
        while (j < lower.length) {
            val c = lower.codePointAt(j)
            j += Character.charCount(c)
            if (isAlnum(c)) out.appendCodePoint(c)
        }
    }
}

/** Cheie stabilă (FNV-1a pe 64 de biți) din textul întrebării și al răspunsurilor. */
fun questionKey(text: String, answers: List<String>): String {
    val sb = StringBuilder()
    normalize(text, sb)
    for (a in answers) {
        sb.append('|')
        normalize(a, sb)
    }
    var h = 0xcbf29ce484222325UL
    for (b in sb.toString().toByteArray(Charsets.UTF_8)) {
        h = h xor b.toUByte().toULong()
        h *= 0x100000001b3UL
    }
    return h.toString(16).padStart(16, '0')
}

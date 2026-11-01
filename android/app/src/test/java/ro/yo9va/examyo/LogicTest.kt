package ro.yo9va.examyo

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import ro.yo9va.examyo.data.Bank
import ro.yo9va.examyo.data.ExamClass
import ro.yo9va.examyo.data.MAX_IDLE_SECS
import ro.yo9va.examyo.data.Probe
import ro.yo9va.examyo.data.Profile
import ro.yo9va.examyo.data.Store
import ro.yo9va.examyo.data.questionKey
import ro.yo9va.examyo.data.splitAncomCode
import java.io.File

class LogicTest {
    private val bank: Bank by lazy {
        val f = listOf("../../examyo-db/data/all.json", "../examyo-db/data/all.json").map(::File).first { it.exists() }
        Bank.load(f.readText())
    }

    @Test
    fun everyClassHasItsProbes() {
        for (c in ExamClass.entries) for (p in c.probes) assertTrue("$c $p goală", bank.set(c, p).isNotEmpty())
        assertFalse(Probe.Radiotehnica in ExamClass.IV.probes)
    }

    @Test
    fun higherClassesIncludeLowerOnes() {
        fun keys(c: ExamClass, p: Probe) = bank.set(c, p).map { bank.questions[it].key }.toSet()
        for (p in listOf(Probe.Radiotehnica, Probe.Legislatie, Probe.Operare)) {
            assertTrue(keys(ExamClass.II, p).containsAll(keys(ExamClass.III, p)))
            assertTrue(keys(ExamClass.I, p).containsAll(keys(ExamClass.II, p)))
        }
    }

    @Test
    fun noDuplicatesAndLowerLevelsFirst() {
        for (c in ExamClass.entries) for (p in c.probes) {
            val s = bank.set(c, p)
            assertEquals("$c $p are dubluri", s.size, s.map { bank.questions[it].key }.toSet().size)
        }
        val r1 = bank.set(ExamClass.I, Probe.Radiotehnica)
        assertTrue(bank.questions[r1.first()].sourceId.startsWith("radiotehnica-clasa-3/"))
        assertTrue(bank.questions[r1.last()].sourceId.startsWith("radiotehnica-clasa-1/"))
    }

    @Test
    fun ancomCodeAndKeys() {
        assertEquals("01A11" to "Rigiditatea", splitAncomCode("01A11/ Rigiditatea"))
        assertEquals(null to "12V este", splitAncomCode("12V este"))
        val a = listOf("Unu", "Doi", "Trei", "Patru")
        assertEquals(questionKey("Ce înseamnă știința?", a), questionKey("Ce inseamna stiinta ?", a))
    }

    /** Cheile și compunerea probelor trebuie să fie identice cu cele din varianta Rust (Windows). */
    @Test
    fun sameResultsAsRust() {
        val path = System.getenv("RUST_KEYS")
        assumeTrue("RUST_KEYS nu e setat", !path.isNullOrEmpty() && File(path).exists())
        val lines = File(path!!).readLines()
        val q = lines.filter { it.startsWith("Q\t") }.map { it.split("\t") }
        assertEquals(bank.questions.size, q.size)
        for ((i, row) in q.withIndex()) {
            val mine = bank.questions[i]
            assertEquals(row[1], mine.sourceId)
            assertEquals("cheie diferită la ${row[1]}", row[2], mine.key)
            assertEquals(row.getOrElse(3) { "" }, mine.ancomCode ?: "")
        }
        for (row in lines.filter { it.startsWith("S\t") }.map { it.split("\t") }) {
            val (c, p) = row[1].split("/")
            val set = bank.set(ExamClass.valueOf(c), Probe.valueOf(p))
            assertEquals(row[1], row[2].toInt(), set.size)
            assertEquals(row[3], bank.questions[set.first()].sourceId)
        }
    }

    @Test
    fun profileRecordsAndCapsIdleTime() {
        var p = Profile.new("Ion", "yo3abc", 0)
        assertEquals("YO3ABC", p.callsign)
        p = p.record("k", false, 1).record("k", true, 2)
        val s = p.answers.getValue("k")
        assertTrue(!s.firstCorrect && s.lastCorrect)
        assertEquals(2, s.attempts)
        assertEquals(10 + MAX_IDLE_SECS, p.addTime(10).addTime(10_000).timeSecs)
    }

    /** Un fișier scris de varianta Rust trebuie citit corect, și invers. */
    @Test
    fun readsRustStoreFile() {
        val rust = """
            {
              "version": 1,
              "profiles": [
                {
                  "id": "6ae6d9ccead3",
                  "name": "Eugen",
                  "callsign": "YO9VA",
                  "created": 1793513932,
                  "last_active": 1793513951,
                  "last_class": "I",
                  "time_secs": 61,
                  "answers": {
                    "4b2475f2135f93e0": { "attempts": 2, "correct": 1, "first_correct": false, "last_correct": true, "last_ts": 1793513951 }
                  },
                  "positions": { "I/Radiotehnica": 80 }
                }
              ],
              "last_profile": "6ae6d9ccead3",
              "theme": "light"
            }
        """.trimIndent()
        val s = Store.parse(rust)
        val p = s.profiles.single()
        assertEquals(ExamClass.I, p.lastClass)
        assertEquals(80, p.position(ExamClass.I, Probe.Radiotehnica))
        assertTrue(p.answers.getValue("4b2475f2135f93e0").lastCorrect)
        assertEquals("light", s.theme)

        val back = Store.json.encodeToString(Store.serializer(), s)
        assertTrue(back.contains("\"last_class\": \"I\""))
        assertTrue(back.contains("\"I/Radiotehnica\": 80"))
        assertEquals(s, Store.parse(back))
    }

    @Test
    fun mergeKeepsNewestProfile() {
        val a = Profile.new("A", "", 10)
        val b = Profile.new("B", "", 20)
        val mine = Store(profiles = listOf(a.copy(lastActive = 100)))
        val other = Store(profiles = listOf(a.copy(lastActive = 200, timeSecs = 5), b))
        val m = mine.merge(other)
        assertEquals(2, m.profiles.size)
        assertEquals(5, m.profile(a.id)!!.timeSecs)
        assertNotNull(m.profile(b.id))
        assertNull(Store().profile("x"))
    }
}

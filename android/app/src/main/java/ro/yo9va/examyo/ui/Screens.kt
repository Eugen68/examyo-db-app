@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)

package ro.yo9va.examyo.ui

import android.graphics.BitmapFactory
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import ro.yo9va.examyo.BuildConfig
import ro.yo9va.examyo.ExamViewModel
import ro.yo9va.examyo.Screen
import ro.yo9va.examyo.data.Bank
import ro.yo9va.examyo.data.ExamClass
import ro.yo9va.examyo.data.Figure
import ro.yo9va.examyo.data.Probe
import ro.yo9va.examyo.data.Profile
import ro.yo9va.examyo.data.Question
import ro.yo9va.examyo.data.Stats
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private const val SITE = "http://examyo.scienceontheweb.net"
private const val FEEDBACK = "http://examyo.scienceontheweb.net/cgi-bin/troubleticket.cgi?type=0"
private val LETTERS = listOf("A", "B", "C", "D")

// ------------------------------------------------------------------ rădăcina

@Composable
fun ExamApp(vm: ExamViewModel) {
    ExamTheme(vm.store.theme) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            val bank = vm.bank
            if (bank == null) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            } else {
                BackHandler(enabled = vm.screen != Screen.Profiles) { vm.back() }
                when (val s = vm.screen) {
                    Screen.Profiles -> ProfilesScreen(vm, bank)
                    is Screen.Home -> HomeScreen(vm, bank, s.pid)
                    is Screen.Practice -> PracticeScreen(vm, bank, s)
                    is Screen.Done -> DoneScreen(vm, bank, s)
                    is Screen.Stats -> StatsScreen(vm, bank, s)
                    Screen.AllStats -> AllStatsScreen(vm, bank)
                }
            }
            vm.message?.let { msg ->
                AlertDialog(
                    onDismissRequest = { vm.message = null },
                    confirmButton = { TextButton(onClick = { vm.message = null }) { Text("OK") } },
                    text = { Text(msg) },
                )
            }
        }
    }
}

// ------------------------------------------------------------------ utilitare

/** Profilul a dispărut (de ex. după un import): ne întoarcem, fără efecte în timpul compunerii. */
@Composable
private fun GoBack(vm: ExamViewModel) {
    LaunchedEffect(Unit) { vm.back() }
}

private fun fmtDate(ts: Long): String =
    if (ts <= 0) "–" else SimpleDateFormat("dd.MM.yyyy HH:mm", Locale.ROOT).format(Date(ts * 1000))

private fun fmtDuration(secs: Long): String {
    val h = secs / 3600
    val m = (secs % 3600) / 60
    return when {
        h == 0L && m == 0L -> "sub un minut"
        h == 0L -> "$m min"
        else -> "$h h $m min"
    }
}

private fun fmtPct(p: Float?): String = p?.let { "%.0f%%".format(it) } ?: "–"

@Composable
private fun Weak(text: String, size: Int = 14) =
    Text(text, fontSize = size.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)

@Composable
private fun Bold(text: String, size: Int = 16, color: Color = Color.Unspecified) =
    Text(text, fontSize = size.sp, fontWeight = FontWeight.Bold, color = color)

@Composable
private fun ScreenScaffold(
    title: String,
    subtitle: String? = null,
    onBack: (() -> Unit)? = null,
    bottomBar: @Composable () -> Unit = {},
    content: @Composable (PaddingValues) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(title, fontSize = 18.sp, fontWeight = FontWeight.Bold, maxLines = 1)
                        if (subtitle != null) Weak(subtitle, 13)
                    }
                },
                navigationIcon = {
                    if (onBack != null) TextButton(onClick = onBack) { Text("←", fontSize = 22.sp) }
                },
            )
        },
        bottomBar = bottomBar,
        content = content,
    )
}

/** Coloană derulabilă, cu lățime maximă (pe tabletă textul nu se întinde pe tot ecranul). */
@Composable
private fun ScrollColumn(padding: PaddingValues, content: @Composable () -> Unit) {
    Box(
        Modifier
            .fillMaxSize()
            .padding(padding)
            .verticalScroll(rememberScrollState()),
        contentAlignment = Alignment.TopCenter,
    ) {
        Column(
            Modifier
                .widthIn(max = 760.dp)
                .fillMaxWidth()
                .padding(horizontal = 16.dp, vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) { content() }
    }
}

// ------------------------------------------------------------------ profiluri

@Composable
private fun ProfilesScreen(vm: ExamViewModel, bank: Bank) {
    val uri = LocalUriHandler.current
    var name by rememberSaveable { mutableStateOf("") }
    var call by rememberSaveable { mutableStateOf("") }
    var confirmDelete by remember { mutableStateOf<Profile?>(null) }
    val export = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/json")) {
        it?.let(vm::exportTo)
    }
    val import = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) {
        it?.let(vm::importFrom)
    }

    Scaffold { padding ->
        ScrollColumn(padding) {
            Spacer(Modifier.height(8.dp))
            Bold("ExamYO", 34)
            Weak("Pregătire pentru examenul de radioamator", 17)
            Weak("realizat de Eugen, YO9VA", 14)
            Spacer(Modifier.height(8.dp))

            val profiles = vm.store.profiles.sortedByDescending { it.lastActive }
            if (profiles.isNotEmpty()) {
                Bold("Cine învață acum?", 18)
                for (p in profiles) {
                    Card(onClick = { vm.openProfile(p.id) }, modifier = Modifier.fillMaxWidth()) {
                        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                            Column(Modifier.weight(1f)) {
                                Bold(p.displayName, 18)
                                val cls = p.lastClass
                                val line = if (cls == null) "clasă nealeasă" else {
                                    val s = p.classStats(bank, cls)
                                    "${cls.title} · ${s.seen} din ${s.total} parcurse"
                                }
                                Weak(line, 15)
                                Weak("Ultima activitate: ${fmtDate(p.lastActive)}", 13)
                            }
                            TextButton(onClick = { confirmDelete = p }) { Text("Șterge") }
                        }
                    }
                }
                Spacer(Modifier.height(4.dp))
            }

            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Bold("Utilizator nou", 18)
                    OutlinedTextField(
                        value = name,
                        onValueChange = { name = it },
                        label = { Text("Nume") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words, imeAction = ImeAction.Next),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    OutlinedTextField(
                        value = call,
                        onValueChange = { call = it },
                        label = { Text("Indicativ (opțional)") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = {
                            if (name.isNotBlank()) vm.addProfile(name, call).also { name = ""; call = "" }
                        }),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Button(
                        onClick = { vm.addProfile(name, call); name = ""; call = "" },
                        enabled = name.isNotBlank(),
                    ) { Text("Adaugă și începe") }
                }
            }

            if (profiles.isNotEmpty()) {
                OutlinedButton(onClick = vm::showAllStats) { Text("Statistici pentru toți utilizatorii") }
            }

            Spacer(Modifier.height(6.dp))
            Bold("Progresul pe alt dispozitiv", 16)
            Weak("Exportă fișierul ExamYO-date.json și importă-l pe telefon sau în ExamYO de pe PC (pune-l lângă ExamYO.exe).", 14)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { export.launch("ExamYO-date.json") }) { Text("Exportă progresul") }
                OutlinedButton(onClick = { import.launch(arrayOf("application/json", "application/octet-stream", "text/plain")) }) {
                    Text("Importă progresul")
                }
            }

            Spacer(Modifier.height(6.dp))
            Bold("Temă", 16)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                val current = vm.store.theme ?: "system"
                for ((key, label) in listOf("system" to "ca sistemul", "light" to "luminoasă", "dark" to "întunecată")) {
                    FilterChip(selected = current == key, onClick = { vm.setTheme(key) }, label = { Text(label) })
                }
            }

            HorizontalDivider(Modifier.padding(vertical = 8.dp))
            Weak("Întrebările și rezolvările: Francisc Toth, YO6OWN, și contributorii lor.", 13)
            TextButton(onClick = { uri.openUri(SITE) }, contentPadding = PaddingValues(0.dp)) {
                Text("examyo.scienceontheweb.net", fontSize = 13.sp)
            }
            val commit = bank.upstreamCommit?.take(7) ?: "?"
            Weak("Licență GNU GPL v2+ · versiunea ${BuildConfig.VERSION_NAME} · date upstream $commit", 12)
            Spacer(Modifier.navigationBarsPadding())
        }
    }

    confirmDelete?.let { p ->
        AlertDialog(
            onDismissRequest = { confirmDelete = null },
            title = { Text("Ștergi utilizatorul?") },
            text = { Text("${p.displayName} și tot progresul lui vor fi șterse. Operația nu poate fi anulată.") },
            confirmButton = {
                TextButton(onClick = { vm.deleteProfile(p.id); confirmDelete = null }) { Text("Șterge", color = Red) }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = null }) { Text("Renunță") } },
        )
    }
}

// ------------------------------------------------------------------ acasă

@Composable
private fun HomeScreen(vm: ExamViewModel, bank: Bank, pid: String) {
    val profile = vm.store.profile(pid) ?: return GoBack(vm)
    ScreenScaffold(title = profile.displayName, subtitle = "ExamYO", onBack = vm::back) { padding ->
        ScrollColumn(padding) {
            Bold("Clasa pentru care te pregătești", 17)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (c in ExamClass.entries) {
                    FilterChip(
                        selected = profile.lastClass == c,
                        onClick = { vm.setClass(pid, c) },
                        label = { Text(c.title, fontSize = 16.sp, fontWeight = FontWeight.Bold) },
                    )
                }
            }
            val cls = profile.lastClass
            if (cls == null) {
                Weak("Alege clasa ca să vezi probele.", 15)
                return@ScrollColumn
            }
            if (cls == ExamClass.I || cls == ExamClass.II) {
                Weak("Include și întrebările claselor inferioare. Le iei la rând, începând cu cele mai ușoare.", 13)
            }
            for (p in cls.probes) ProbeCard(vm, bank, profile, cls, p)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = { vm.showStats(pid) }) { Text("Statisticile mele") }
                TextButton(onClick = vm::back) { Text("Schimbă utilizatorul") }
            }
        }
    }
}

@Composable
private fun ProbeCard(vm: ExamViewModel, bank: Bank, profile: Profile, cls: ExamClass, probe: Probe) {
    val s = profile.stats(bank, cls, probe)
    val pos = profile.position(cls, probe)
    val levels = Bank.levels(cls, probe)
    OutlinedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Bold(probe.title, 20)
            if (levels.size > 1) Weak(levels.joinToString(" → "), 13)
            Text(
                "${s.total} întrebări · parcurse ${s.seen} · corecte din prima ${fmtPct(s.firstCorrectPct)} · greșite acum ${s.wrongNow}",
                fontSize = 15.sp,
            )
            LinearProgressIndicator(progress = { s.seenFraction }, modifier = Modifier.fillMaxWidth())
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                val main = when {
                    pos == 0 && s.seen == 0 -> "Începe"
                    pos < s.total -> "Continuă de la ${pos + 1}"
                    else -> "Terminată — de la început"
                }
                Button(onClick = { vm.start(profile.id, cls, probe, fromStart = pos >= s.total, wrongOnly = false) }) {
                    Text(main)
                }
                if (pos in 1 until s.total) {
                    OutlinedButton(onClick = { vm.start(profile.id, cls, probe, fromStart = true, wrongOnly = false) }) {
                        Text("De la început")
                    }
                }
                OutlinedButton(
                    onClick = { vm.start(profile.id, cls, probe, fromStart = false, wrongOnly = true) },
                    enabled = s.wrongNow > 0,
                ) { Text("Doar greșelile (${s.wrongNow})") }
            }
        }
    }
}

// ------------------------------------------------------------------ parcurgere

@Composable
private fun PracticeScreen(vm: ExamViewModel, bank: Bank, s: Screen.Practice) {
    val q = bank.questions[s.list[s.pos]]
    val mode = if (s.wrongOnly) " · doar greșelile" else ""
    ScreenScaffold(
        title = "${s.cls.title} · ${s.probe.title}",
        subtitle = "Întrebarea ${s.pos + 1} din ${s.list.size}$mode",
        onBack = vm::back,
        bottomBar = {
            Surface(tonalElevation = 3.dp) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .navigationBarsPadding()
                        .padding(horizontal = 12.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    OutlinedButton(onClick = vm::prev, enabled = s.pos > 0) { Text("◀") }
                    Spacer(Modifier.weight(1f))
                    if (s.answered) {
                        OutlinedButton(onClick = vm::toggleExplanation) {
                            Text(if (s.showExplanation) "Ascunde rezolvarea" else "Rezolvarea")
                        }
                        Button(onClick = vm::next) { Text("Următoarea ▶") }
                    } else {
                        TextButton(onClick = vm::next) { Text("Sari peste") }
                        Button(onClick = vm::verify, enabled = s.selected != null) { Text("Verifică") }
                    }
                }
            }
        },
    ) { padding ->
        Column(Modifier.padding(padding)) {
            LinearProgressIndicator(
                progress = { s.pos.toFloat() / s.list.size },
                modifier = Modifier.fillMaxWidth().height(3.dp),
            )
            ScrollColumn(PaddingValues(0.dp)) {
                val tag = listOfNotNull(q.level.ifEmpty { null }, q.ancomCode?.let { "subiect ANCOM $it" })
                if (tag.isNotEmpty()) Weak(tag.joinToString(" · "), 13)
                Text(q.text, fontSize = 19.sp, lineHeight = 26.sp)
                q.figure?.let { FigureView(it) }
                Spacer(Modifier.height(4.dp))
                for (d in 0..3) AnswerCard(vm, s, q, d)

                if (s.answered) {
                    val chosen = s.order[s.selected!!]
                    if (chosen == q.correct) {
                        Bold("✔ Corect!", 19, Green)
                    } else {
                        val shown = s.order.indexOf(q.correct)
                        Bold("✘ Greșit. Răspunsul corect este ${LETTERS[shown]}.", 19, Red)
                    }
                    if (s.showExplanation) ExplanationCard(q)
                    q.contributor?.let { Weak("Întrebare propusă de: $it", 13) }
                } else {
                    Weak("Atinge un răspuns, apoi „Verifică”.", 13)
                }
                Spacer(Modifier.height(12.dp))
            }
        }
    }
}

@Composable
private fun AnswerCard(vm: ExamViewModel, s: Screen.Practice, q: Question, d: Int) {
    val orig = s.order[d]
    val isCorrect = orig == q.correct
    val isSelected = s.selected == d
    val dark = isDark()
    val cs = MaterialTheme.colorScheme
    val (fill, border) = when {
        s.answered && isCorrect -> (if (dark) Color(0xFF1C4A28) else Color(0xFFD6F1DB)) to BorderStroke(2.dp, Green)
        s.answered && isSelected -> (if (dark) Color(0xFF5A2220) else Color(0xFFFADAD6)) to BorderStroke(2.dp, Red)
        isSelected -> cs.primaryContainer to BorderStroke(2.dp, cs.primary)
        else -> cs.surfaceVariant.copy(alpha = 0.5f) to BorderStroke(1.dp, cs.outlineVariant)
    }
    val mark = when {
        s.answered && isCorrect -> "  ✔"
        s.answered && isSelected -> "  ✘"
        else -> ""
    }
    Surface(
        onClick = { vm.select(d) },
        enabled = !s.answered,
        shape = RoundedCornerShape(10.dp),
        color = fill,
        border = border,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(horizontal = 14.dp, vertical = 12.dp)) {
            Bold("${LETTERS[d]})", 17)
            Spacer(Modifier.width(10.dp))
            Text(q.answers[orig] + mark, fontSize = 17.sp, lineHeight = 23.sp)
        }
    }
}

@Composable
private fun ExplanationCard(q: Question) {
    val uri = LocalUriHandler.current
    Card(
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer.copy(alpha = 0.6f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Bold("Rezolvare", 17)
            if (q.explanation != null) {
                Text(q.explanation, fontSize = 16.sp, lineHeight = 22.sp)
                q.explanationFigure?.let { FigureView(it) }
                q.explanationAuthor?.let { Weak("— $it", 13) }
            } else {
                Text("Pentru această întrebare nu există încă o rezolvare.", fontSize = 15.sp)
                TextButton(onClick = { uri.openUri(FEEDBACK) }, contentPadding = PaddingValues(0.dp)) {
                    Text("Dacă o știi, o poți propune autorului")
                }
            }
        }
    }
}

@Composable
private fun FigureView(f: Figure) {
    val context = LocalContext.current
    val bitmap = remember(f.name) {
        runCatching {
            context.assets.open("images/${f.name}").use { BitmapFactory.decodeStream(it) }?.asImageBitmap()
        }.getOrNull()
    }
    if (bitmap == null) {
        Weak("[figura ${f.name} lipsește]")
        return
    }
    BoxWithConstraints(Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        val scale = if ((f.width ?: 400) < 300) 1.4f else 1.15f
        val wanted = ((f.width ?: bitmap.width) * scale).dp
        val w = if (wanted > maxWidth - 16.dp) maxWidth - 16.dp else wanted
        // Figurile sunt desene negre pe transparent: fundal alb ca să se vadă și în tema întunecată.
        Box(
            Modifier
                .background(Color.White, RoundedCornerShape(6.dp))
                .padding(8.dp)
        ) {
            Image(
                bitmap = bitmap,
                contentDescription = "Figura întrebării",
                modifier = Modifier.width(w).aspectRatio(bitmap.width.toFloat() / bitmap.height),
            )
        }
    }
}

// ------------------------------------------------------------------ final de probă

@Composable
private fun DoneScreen(vm: ExamViewModel, bank: Bank, d: Screen.Done) {
    val profile = vm.store.profile(d.pid) ?: return GoBack(vm)
    val s = profile.stats(bank, d.cls, d.probe)
    ScreenScaffold(title = "${d.cls.title} · ${d.probe.title}", subtitle = profile.displayName, onBack = vm::back) { padding ->
        ScrollColumn(padding) {
            Bold("🎉 Gata!", 30)
            Text(
                if (d.wrongOnly) "Ai trecut prin cele ${d.count} întrebări greșite din proba ${d.probe.title}."
                else "Ai ajuns la capătul probei ${d.probe.title} (${d.count} întrebări).",
                fontSize = 17.sp,
            )
            OutlinedCard(Modifier.fillMaxWidth()) {
                Column(Modifier.padding(14.dp)) { StatsLines(s) }
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (s.wrongNow > 0) {
                    Button(onClick = { vm.start(d.pid, d.cls, d.probe, fromStart = false, wrongOnly = true) }) {
                        Text("Reia greșelile (${s.wrongNow})")
                    }
                }
                OutlinedButton(onClick = { vm.start(d.pid, d.cls, d.probe, fromStart = true, wrongOnly = false) }) {
                    Text("Ia proba de la început")
                }
                TextButton(onClick = { vm.goHome(d.pid) }) { Text("Înapoi la probe") }
            }
        }
    }
}

@Composable
private fun StatsLines(s: Stats) {
    Text("Parcurse: ${s.seen} din ${s.total}", fontSize = 16.sp)
    Text("Corecte din prima: ${s.firstCorrect} (${fmtPct(s.firstCorrectPct)})", fontSize = 16.sp)
    Text("Greșite la ultima încercare: ${s.wrongNow}", fontSize = 16.sp)
}

// ------------------------------------------------------------------ statistici

@Composable
private fun StatsScreen(vm: ExamViewModel, bank: Bank, st: Screen.Stats) {
    val profile = vm.store.profile(st.pid) ?: return GoBack(vm)
    ScreenScaffold(title = "Statistici", subtitle = profile.displayName, onBack = vm::back) { padding ->
        ScrollColumn(padding) {
            Bold(profile.displayName, 22)
            Weak("Utilizator din ${fmtDate(profile.created)}", 14)
            Weak("Ultima activitate: ${fmtDate(profile.lastActive)}", 14)
            Weak("Timp de studiu: ${fmtDuration(profile.timeSecs)}", 14)
            val attempts = profile.answers.values.sumOf { it.attempts }
            Weak("${profile.answers.size} întrebări diferite, $attempts răspunsuri date în total", 14)
            Spacer(Modifier.height(4.dp))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (c in ExamClass.entries) {
                    FilterChip(selected = st.cls == c, onClick = { vm.showStatsClass(st.pid, c) }, label = { Text(c.title) })
                }
            }
            val cls = st.cls ?: run {
                Weak("Alege o clasă ca să vezi statisticile pe probe.", 15)
                return@ScrollColumn
            }
            var weakest: Pair<Probe, Float>? = null
            for (p in cls.probes) {
                val s = profile.stats(bank, cls, p)
                StatsCard(p.title, s, bold = false)
                val pct = s.firstCorrectPct
                if (pct != null && s.seen >= 10 && (weakest == null || pct < weakest.second)) weakest = p to pct
            }
            StatsCard("Total ${cls.title}", profile.classStats(bank, cls), bold = true)
            weakest?.let { (p, pct) ->
                Text("Cel mai slab: ${p.title} (${"%.0f".format(pct)}% corecte din prima)", color = Red, fontSize = 15.sp)
            }
            Weak("„Corecte din prima” = la câte întrebări din cele parcurse ai răspuns corect la prima încercare.", 13)
        }
    }
}

@Composable
private fun StatsCard(title: String, s: Stats, bold: Boolean) {
    OutlinedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Bold(title, if (bold) 18 else 16)
            Text(
                "Parcurse ${s.seen} din ${s.total} · corecte din prima ${fmtPct(s.firstCorrectPct)} · greșite acum ${s.wrongNow}",
                fontSize = 15.sp,
            )
            LinearProgressIndicator(progress = { s.seenFraction }, modifier = Modifier.fillMaxWidth())
        }
    }
}

@Composable
private fun AllStatsScreen(vm: ExamViewModel, bank: Bank) {
    ScreenScaffold(title = "Toți utilizatorii", subtitle = "Pentru clasa aleasă de fiecare", onBack = vm::back) { padding ->
        ScrollColumn(padding) {
            for (p in vm.store.profiles.sortedBy { it.name.lowercase() }) {
                Card(onClick = { vm.showStats(p.id) }, modifier = Modifier.fillMaxWidth()) {
                    Column(Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Bold(p.displayName, 17)
                        val c = p.lastClass
                        if (c != null) {
                            val s = p.classStats(bank, c)
                            Text(
                                "${c.title} · parcurse ${s.seen}/${s.total} · corecte din prima ${fmtPct(s.firstCorrectPct)}",
                                fontSize = 15.sp,
                            )
                            LinearProgressIndicator(progress = { s.seenFraction }, modifier = Modifier.fillMaxWidth())
                        } else {
                            Weak("clasă nealeasă", 15)
                        }
                        Weak("Timp de studiu ${fmtDuration(p.timeSecs)} · ultima activitate ${fmtDate(p.lastActive)}", 13)
                    }
                }
            }
        }
    }
}

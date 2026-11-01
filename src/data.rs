//! Întrebările: încărcare, compunerea probelor pe clase și eliminarea dublurilor.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ------------------------------------------------------------------ clase și probe

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Class {
    I,
    II,
    III,
    IV,
}

impl Class {
    pub const ALL: [Class; 4] = [Class::I, Class::II, Class::III, Class::IV];

    pub fn roman(self) -> &'static str {
        match self {
            Class::I => "I",
            Class::II => "II",
            Class::III => "III",
            Class::IV => "IV",
        }
    }

    pub fn name(self) -> String {
        format!("Clasa {}", self.roman())
    }

    /// Probele examenului pentru clasa respectivă (clasa a IV-a nu are radiotehnică).
    pub fn probes(self) -> &'static [Probe] {
        match self {
            Class::IV => &[Probe::Legislatie, Probe::Sanctiuni, Probe::Operare, Probe::Ntsm],
            _ => &Probe::ALL,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Probe {
    Radiotehnica,
    Legislatie,
    Sanctiuni,
    Operare,
    Ntsm,
}

impl Probe {
    pub const ALL: [Probe; 5] =
        [Probe::Radiotehnica, Probe::Legislatie, Probe::Sanctiuni, Probe::Operare, Probe::Ntsm];

    pub fn name(self) -> &'static str {
        match self {
            Probe::Radiotehnica => "Radiotehnică",
            Probe::Legislatie => "Legislație",
            Probe::Sanctiuni => "Sancțiuni",
            Probe::Operare => "Reguli de trafic",
            Probe::Ntsm => "Protecția muncii",
        }
    }
}

/// Din ce baze se compune o probă pentru o clasă, în ordinea de parcurgere:
/// întâi întrebările de nivel inferior (clasa III), apoi II, apoi I.
fn composition(class: Class, probe: Probe) -> &'static [(&'static str, &'static str)] {
    use Class::*;
    use Probe::*;
    const R3: (&str, &str) = ("radiotehnica-clasa-3", "Clasa III");
    const R2: (&str, &str) = ("radiotehnica-clasa-2", "Clasa II");
    const R1: (&str, &str) = ("radiotehnica-clasa-1", "Clasa I");
    const L3: (&str, &str) = ("legislatie-clasa-3", "Clasa III");
    const L12: (&str, &str) = ("legislatie-clasa-1-2", "Clasa I–II");
    const L4: (&str, &str) = ("legislatie-clasa-4", "Clasa IV");
    const O3: (&str, &str) = ("operare-clasa-3", "Clasa III");
    const O12: (&str, &str) = ("operare-clasa-1-2", "Clasa I–II");
    const O4: (&str, &str) = ("operare-clasa-4", "Clasa IV");
    const N13: (&str, &str) = ("ntsm-clasa-1-3", "");
    const N4: (&str, &str) = ("ntsm-clasa-4", "");
    const S: (&str, &str) = ("sanctiuni", "");
    match (class, probe) {
        (I, Radiotehnica) => &[R3, R2, R1],
        (II, Radiotehnica) => &[R3, R2],
        (III, Radiotehnica) => &[R3],
        (IV, Radiotehnica) => &[],
        (I | II, Legislatie) => &[L3, L12],
        (III, Legislatie) => &[L3],
        (IV, Legislatie) => &[L4],
        (I | II, Operare) => &[O3, O12],
        (III, Operare) => &[O3],
        (IV, Operare) => &[O4],
        (I | II | III, Ntsm) => &[N13],
        (IV, Ntsm) => &[N4],
        (_, Sanctiuni) => &[S],
    }
}

// ------------------------------------------------------------------ întrebări

#[derive(Clone, Debug)]
pub struct Figure {
    /// Numele fișierului din examyo-db/images.
    pub name: String,
    /// Lățimea în pixeli la care o afișa simulatorul original.
    pub width: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Question {
    /// Cheie stabilă, calculată din textul întrebării și al răspunsurilor.
    /// Progresul se salvează după ea, deci supraviețuiește renumerotărilor din upstream
    /// și leagă aceeași întrebare apărută în mai multe baze.
    pub key: String,
    /// Identificatorul din examyo-db, de exemplu „radiotehnica-clasa-1/0”.
    pub source_id: String,
    /// Nivelul de proveniență afișat la întrebare („Clasa III”, „Clasa I–II” …).
    pub level: &'static str,
    /// Codul subiectului din lista oficială ANCOM (de ex. „01A11”), când întrebarea îl are.
    pub ancom_code: Option<String>,
    pub text: String,
    pub figure: Option<Figure>,
    pub answers: [String; 4],
    pub correct: usize,
    pub explanation: Option<String>,
    pub explanation_figure: Option<Figure>,
    pub contributor: Option<String>,
    pub explanation_author: Option<String>,
}

#[derive(Deserialize)]
struct RawAll {
    source: RawSource,
    databases: Vec<RawDb>,
}

#[derive(Deserialize)]
struct RawSource {
    upstream_commit: Option<String>,
}

#[derive(Deserialize)]
struct RawDb {
    slug: String,
    questions: Vec<RawQ>,
}

#[derive(Deserialize)]
struct RawFig {
    file: String,
    width: Option<u32>,
}

#[derive(Deserialize)]
struct RawQ {
    id: String,
    question: String,
    image: Option<RawFig>,
    answers: Vec<String>,
    correct_index: usize,
    explanation: Option<String>,
    explanation_image: Option<RawFig>,
    contributor: Option<String>,
    explanation_author: Option<String>,
}

fn figure(raw: Option<RawFig>) -> Option<Figure> {
    raw.map(|f| Figure {
        name: f.file.trim_start_matches("images/").to_string(),
        width: f.width,
    })
}

/// Toate întrebările, plus listele (fără dubluri) pentru fiecare clasă și probă.
pub struct Bank {
    pub questions: Vec<Question>,
    sets: HashMap<(Class, Probe), Vec<usize>>,
    pub upstream_commit: Option<String>,
}

impl Bank {
    pub fn load(json: &[u8]) -> Result<Bank, String> {
        let raw: RawAll = serde_json::from_slice(json).map_err(|e| format!("Date invalide: {e}"))?;
        let mut questions = Vec::new();
        let mut by_slug: HashMap<String, Vec<usize>> = HashMap::new();

        for db in raw.databases {
            let mut idx = Vec::with_capacity(db.questions.len());
            for q in db.questions {
                let answers: [String; 4] = q
                    .answers
                    .try_into()
                    .map_err(|_| format!("{}: nu are exact 4 răspunsuri", q.id))?;
                if q.correct_index > 3 {
                    return Err(format!("{}: răspuns corect invalid", q.id));
                }
                idx.push(questions.len());
                let (ancom_code, text) = split_ancom_code(&q.question);
                questions.push(Question {
                    key: question_key(&text, &answers),
                    source_id: q.id,
                    level: "",
                    ancom_code,
                    text,
                    figure: figure(q.image),
                    answers,
                    correct: q.correct_index,
                    explanation: q.explanation,
                    explanation_figure: figure(q.explanation_image),
                    contributor: q.contributor,
                    explanation_author: q.explanation_author,
                });
            }
            by_slug.insert(db.slug, idx);
        }

        let mut sets = HashMap::new();
        for class in Class::ALL {
            for &probe in class.probes() {
                let mut seen = HashSet::new();
                let mut list = Vec::new();
                for (slug, level) in composition(class, probe) {
                    let ids = by_slug
                        .get(*slug)
                        .ok_or_else(|| format!("Lipsește baza „{slug}” din date"))?;
                    for &i in ids {
                        if seen.insert(questions[i].key.clone()) {
                            list.push((i, *level));
                        }
                    }
                }
                sets.insert((class, probe), list);
            }
        }

        // O bază are mereu același nivel în orice compunere, deci îl putem nota pe întrebare.
        let sets = sets
            .into_iter()
            .map(|(k, v): ((Class, Probe), Vec<(usize, &'static str)>)| {
                for &(i, level) in &v {
                    if questions[i].level.is_empty() {
                        questions[i].level = level;
                    }
                }
                (k, v.into_iter().map(|(i, _)| i).collect())
            })
            .collect();

        Ok(Bank { questions, sets, upstream_commit: raw.source.upstream_commit })
    }

    /// Indicii întrebărilor dintr-o probă, în ordinea de parcurgere, fără dubluri.
    pub fn set(&self, class: Class, probe: Probe) -> &[usize] {
        self.sets.get(&(class, probe)).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Ce niveluri conține proba, pentru afișare („Clasa III, Clasa II, Clasa I”).
    pub fn levels(class: Class, probe: Probe) -> Vec<&'static str> {
        composition(class, probe).iter().map(|(_, l)| *l).filter(|l| !l.is_empty()).collect()
    }
}

/// Unele întrebări încep cu codul subiectului ANCOM: „01A11/ Rigiditatea …”.
/// Îl separăm de text ca să-l afișăm discret, deasupra întrebării.
fn split_ancom_code(text: &str) -> (Option<String>, String) {
    let b = text.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize, max: usize| {
        let s = *i;
        while *i < b.len() && *i - s < max && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - s
    };
    let ok = digits(&mut i, 2) == 2
        && i < b.len()
        && b[i].is_ascii_uppercase()
        && {
            i += 1;
            digits(&mut i, 3) >= 1
        }
        && {
            if i < b.len() && b[i].is_ascii_lowercase() {
                i += 1;
            }
            i < b.len() && b[i] == b'/'
        };
    if ok {
        (Some(text[..i].to_string()), text[i + 1..].trim_start().to_string())
    } else {
        (None, text.to_string())
    }
}

// ------------------------------------------------------------------ chei stabile

/// Normalizează textul ca două variante ale aceleiași întrebări (cu și fără diacritice,
/// cu spații sau punctuație diferite) să dea aceeași cheie.
fn normalize(s: &str, out: &mut String) {
    for ch in s.chars() {
        let ch = match ch {
            'ă' | 'â' | 'Ă' | 'Â' => 'a',
            'î' | 'Î' => 'i',
            'ș' | 'ş' | 'Ș' | 'Ş' => 's',
            'ț' | 'ţ' | 'Ț' | 'Ţ' => 't',
            c => c,
        };
        for c in ch.to_lowercase() {
            if c.is_alphanumeric() {
                out.push(c);
            }
        }
    }
}

pub fn question_key(text: &str, answers: &[String; 4]) -> String {
    let mut s = String::new();
    normalize(text, &mut s);
    for a in answers {
        s.push('|');
        normalize(a, &mut s);
    }
    // FNV-1a pe 64 de biți: stabil între versiuni de compilator, spre deosebire de DefaultHasher.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank() -> Bank {
        Bank::load(crate::embedded_json()).expect("datele se încarcă")
    }

    #[test]
    fn every_class_has_its_probes() {
        let b = bank();
        for c in Class::ALL {
            for &p in c.probes() {
                assert!(!b.set(c, p).is_empty(), "{c:?} {p:?} e goală");
            }
        }
        assert!(!Class::IV.probes().contains(&Probe::Radiotehnica));
    }

    #[test]
    fn higher_classes_include_lower_ones() {
        let b = bank();
        let keys = |c, p| -> HashSet<String> {
            b.set(c, p).iter().map(|&i| b.questions[i].key.clone()).collect()
        };
        for p in [Probe::Radiotehnica, Probe::Legislatie, Probe::Operare] {
            let (k1, k2, k3) = (keys(Class::I, p), keys(Class::II, p), keys(Class::III, p));
            assert!(k3.is_subset(&k2), "{p:?}: III ⊄ II");
            assert!(k2.is_subset(&k1), "{p:?}: II ⊄ I");
        }
    }

    #[test]
    fn no_duplicates_and_lower_levels_first() {
        let b = bank();
        for c in Class::ALL {
            for &p in c.probes() {
                let s = b.set(c, p);
                let uniq: HashSet<_> = s.iter().map(|&i| &b.questions[i].key).collect();
                assert_eq!(uniq.len(), s.len(), "{c:?} {p:?} are dubluri");
            }
        }
        let r1 = b.set(Class::I, Probe::Radiotehnica);
        assert!(b.questions[r1[0]].source_id.starts_with("radiotehnica-clasa-3/"));
        assert!(b.questions[*r1.last().unwrap()].source_id.starts_with("radiotehnica-clasa-1/"));
    }

    #[test]
    fn all_figures_are_embedded() {
        let b = bank();
        for q in &b.questions {
            for f in [&q.figure, &q.explanation_figure].into_iter().flatten() {
                assert!(crate::embedded_image(&f.name).is_some(), "lipsește {}", f.name);
            }
        }
    }

    #[test]
    fn ancom_code_is_split_off() {
        assert_eq!(
            split_ancom_code("01A11/ Rigiditatea dielectricilor"),
            (Some("01A11".into()), "Rigiditatea dielectricilor".into())
        );
        assert_eq!(split_ancom_code("23B70/Aproximativ"), (Some("23B70".into()), "Aproximativ".into()));
        assert_eq!(split_ancom_code("12V este tensiunea"), (None, "12V este tensiunea".into()));
        let b = bank();
        assert_eq!(b.questions.iter().filter(|q| q.ancom_code.is_some()).count(), 253);
    }

    #[test]
    fn key_ignores_diacritics_and_spacing() {
        let a = ["Unu".into(), "Doi".into(), "Trei".into(), "Patru".into()];
        assert_eq!(
            question_key("Care este tensiunea?", &a),
            question_key("Care  este tensiunea ?", &a)
        );
        assert_eq!(question_key("Ce înseamnă știința?", &a), question_key("Ce inseamna stiinta?", &a));
    }
}

//! Profilurile utilizatorilor și progresul lor, salvate local într-un fișier JSON
//! aflat lângă executabil (aplicația e portabilă, deci progresul „călătorește” cu ea).

use crate::data::{Bank, Class, Probe};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const STORE_FILE: &str = "ExamYO-date.json";

/// Cât timp maxim se socotește între două acțiuni; o pauză mai lungă nu intră în timpul de studiu.
pub const MAX_IDLE_SECS: u64 = 300;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QStat {
    pub attempts: u32,
    pub correct: u32,
    /// Prima încercare a fost corectă.
    pub first_correct: bool,
    /// Ultima încercare a fost corectă (cele greșite revin în „Doar greșelile”).
    pub last_correct: bool,
    pub last_ts: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub callsign: String,
    pub created: i64,
    pub last_active: i64,
    #[serde(default)]
    pub last_class: Option<Class>,
    /// Timp de studiu, în secunde.
    #[serde(default)]
    pub time_secs: u64,
    /// Statistici pe întrebare, după cheia stabilă a întrebării.
    #[serde(default)]
    pub answers: BTreeMap<String, QStat>,
    /// Unde a rămas la parcurgere, pe clasă și probă.
    #[serde(default)]
    pub positions: BTreeMap<String, usize>,
}

pub fn position_key(class: Class, probe: Probe) -> String {
    format!("{class:?}/{probe:?}")
}

impl Profile {
    pub fn new(name: &str, callsign: &str, now: i64) -> Profile {
        Profile {
            id: format!("{now:x}{:04x}", fastrand::u16(..)),
            name: name.trim().to_string(),
            callsign: callsign.trim().to_uppercase(),
            created: now,
            last_active: now,
            ..Default::default()
        }
    }

    pub fn display_name(&self) -> String {
        if self.callsign.is_empty() {
            self.name.clone()
        } else {
            format!("{} ({})", self.name, self.callsign)
        }
    }

    pub fn record(&mut self, key: &str, correct: bool, now: i64) {
        let s = self.answers.entry(key.to_string()).or_insert_with(|| QStat {
            first_correct: correct,
            ..Default::default()
        });
        s.attempts += 1;
        if correct {
            s.correct += 1;
        }
        s.last_correct = correct;
        s.last_ts = now;
        self.last_active = now;
    }

    pub fn add_time(&mut self, secs: u64) {
        self.time_secs += secs.min(MAX_IDLE_SECS);
    }

    pub fn position(&self, class: Class, probe: Probe) -> usize {
        self.positions.get(&position_key(class, probe)).copied().unwrap_or(0)
    }

    pub fn set_position(&mut self, class: Class, probe: Probe, pos: usize) {
        self.positions.insert(position_key(class, probe), pos);
    }

    pub fn stats(&self, bank: &Bank, class: Class, probe: Probe) -> Stats {
        Stats::of(self, bank, bank.set(class, probe))
    }

    pub fn class_stats(&self, bank: &Bank, class: Class) -> Stats {
        let mut all: Vec<usize> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for &p in class.probes() {
            for &i in bank.set(class, p) {
                if seen.insert(&bank.questions[i].key) {
                    all.push(i);
                }
            }
        }
        Stats::of(self, bank, &all)
    }

    /// Întrebările dintr-o probă la care ultimul răspuns a fost greșit.
    pub fn wrong(&self, bank: &Bank, class: Class, probe: Probe) -> Vec<usize> {
        bank.set(class, probe)
            .iter()
            .copied()
            .filter(|&i| self.answers.get(&bank.questions[i].key).is_some_and(|s| !s.last_correct))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    pub total: usize,
    /// Întrebări la care a răspuns cel puțin o dată.
    pub seen: usize,
    pub first_correct: usize,
    /// Întrebări la care ultimul răspuns e greșit.
    pub wrong_now: usize,
}

impl Stats {
    fn of(p: &Profile, bank: &Bank, set: &[usize]) -> Stats {
        let mut s = Stats { total: set.len(), ..Default::default() };
        for &i in set {
            if let Some(q) = p.answers.get(&bank.questions[i].key) {
                s.seen += 1;
                s.first_correct += q.first_correct as usize;
                s.wrong_now += !q.last_correct as usize;
            }
        }
        s
    }

    /// Procentul de răspunsuri corecte din prima, din cele parcurse.
    pub fn first_correct_pct(&self) -> Option<f32> {
        (self.seen > 0).then(|| 100.0 * self.first_correct as f32 / self.seen as f32)
    }

    pub fn seen_fraction(&self) -> f32 {
        if self.total == 0 { 0.0 } else { self.seen as f32 / self.total as f32 }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    #[serde(default = "store_version")]
    pub version: u32,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub last_profile: Option<String>,
    /// Tema aleasă: "system", "light" sau "dark".
    #[serde(default)]
    pub theme: Option<String>,
}

fn store_version() -> u32 {
    1
}

impl Store {
    /// Fișierul de date stă lângă executabil.
    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(STORE_FILE)
    }

    /// Încarcă datele. Dacă fișierul e stricat, îl păstrează cu altă extensie
    /// (ca să nu se piardă nimic) și pornește de la zero, cu un avertisment.
    pub fn load(path: &Path) -> (Store, Option<String>) {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return (Store { version: 1, ..Default::default() }, None)
            }
            Err(e) => {
                return (
                    Store { version: 1, ..Default::default() },
                    Some(format!("Nu pot citi {}: {e}", path.display())),
                )
            }
        };
        match serde_json::from_slice::<Store>(&bytes) {
            Ok(s) => (s, None),
            Err(e) => {
                let backup = path.with_extension(format!("stricat-{}.json", now()));
                let _ = std::fs::rename(path, &backup);
                (
                    Store { version: 1, ..Default::default() },
                    Some(format!(
                        "Fișierul de date era deteriorat ({e}). L-am păstrat ca {} și am pornit de la zero.",
                        backup.display()
                    )),
                )
            }
        }
    }

    /// Scrie atomic: întâi într-un fișier temporar, apoi îl redenumește.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .and_then(|_| std::fs::rename(&tmp, path))
            .map_err(|e| format!("Nu pot salva progresul în {}: {e}", path.display()))
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile_mut(&mut self, id: &str) -> Option<&mut Profile> {
        self.profiles.iter_mut().find(|p| p.id == id)
    }
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_tracks_first_and_last() {
        let mut p = Profile::new("Ion", "yo3abc", 0);
        assert_eq!(p.callsign, "YO3ABC");
        p.record("k", false, 1);
        p.record("k", true, 2);
        let s = &p.answers["k"];
        assert!(!s.first_correct && s.last_correct);
        assert_eq!((s.attempts, s.correct), (2, 1));
    }

    #[test]
    fn idle_time_is_capped() {
        let mut p = Profile::new("Ion", "", 0);
        p.add_time(10);
        p.add_time(10_000);
        assert_eq!(p.time_secs, 10 + MAX_IDLE_SECS);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("examyo-test-{}", fastrand::u32(..)));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(STORE_FILE);
        let mut s = Store::default();
        let mut p = Profile::new("Maria", "", 5);
        p.last_class = Some(Class::III);
        p.set_position(Class::III, Probe::Operare, 7);
        s.profiles.push(p);
        s.save(&path).unwrap();
        let (l, warn) = Store::load(&path);
        assert!(warn.is_none());
        assert_eq!(l.profiles[0].position(Class::III, Probe::Operare), 7);
        assert_eq!(l.profiles[0].last_class, Some(Class::III));

        std::fs::write(&path, b"{ stricat").unwrap();
        let (l, warn) = Store::load(&path);
        assert!(l.profiles.is_empty() && warn.is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

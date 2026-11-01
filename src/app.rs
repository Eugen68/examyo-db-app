//! Interfața (egui): profiluri, alegerea clasei, parcurgerea întrebărilor și statistici.

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Key, Layout, Margin, Modifiers,
    ProgressBar, RichText, Sense, Stroke, TextStyle, Ui, UiBuilder, Vec2,
};
use examyo_core::data::{Bank, Class, Figure, Probe, Question};
use examyo_core::store::{now, Profile, Stats, Store};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

const CONTENT_WIDTH: f32 = 920.0;
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];
const SITE: &str = "http://examyo.scienceontheweb.net";
const FEEDBACK: &str = "http://examyo.scienceontheweb.net/cgi-bin/troubleticket.cgi?type=0";

// ------------------------------------------------------------------ stare

pub struct ExamApp {
    bank: Bank,
    store: Store,
    path: PathBuf,
    screen: Screen,
    message: Option<String>,
    new_name: String,
    new_call: String,
    confirm_delete: Option<String>,
    stats_class: Option<Class>,
}

enum Screen {
    Profiles,
    Home(String),
    Practice(Practice),
    Done { pid: String, class: Class, probe: Probe, wrong_only: bool, count: usize },
    Stats(String),
    AllStats,
}

struct Practice {
    pid: String,
    class: Class,
    probe: Probe,
    wrong_only: bool,
    list: Vec<usize>,
    pos: usize,
    /// Ordinea de afișare a răspunsurilor: order[poziție afișată] = indicele original.
    order: [usize; 4],
    selected: Option<usize>,
    answered: bool,
    show_explanation: bool,
    last_action: Instant,
}

impl Practice {
    fn new(pid: String, class: Class, probe: Probe, list: Vec<usize>, pos: usize, wrong_only: bool) -> Self {
        let mut p = Practice {
            pid,
            class,
            probe,
            wrong_only,
            list,
            pos: 0,
            order: [0, 1, 2, 3],
            selected: None,
            answered: false,
            show_explanation: false,
            last_action: Instant::now(),
        };
        p.go(pos);
        p
    }

    fn go(&mut self, pos: usize) {
        self.pos = pos;
        self.order = [0, 1, 2, 3];
        fastrand::shuffle(&mut self.order);
        self.selected = None;
        self.answered = false;
        self.show_explanation = false;
    }

    /// Secunde de la ultima acțiune (plafonate la salvare, ca pauzele lungi să nu conteze).
    fn touch(&mut self) -> u64 {
        let s = self.last_action.elapsed().as_secs();
        self.last_action = Instant::now();
        s
    }
}

enum Action {
    OpenProfile(String),
    AddProfile,
    DeleteProfile(String),
    ToProfiles,
    SetClass(String, Class),
    Start { pid: String, class: Class, probe: Probe, from_start: bool, wrong_only: bool },
    ToHome(String),
    ToStats(String),
    ToAllStats,
    SetTheme(egui::ThemePreference),
}

// ------------------------------------------------------------------ inițializare

impl ExamApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_style(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let bank = Bank::load(examyo_core::embedded_json()).expect("datele incluse în aplicație sunt valide");
        let path = Store::default_path();
        let (store, message) = Store::load(&path);
        cc.egui_ctx.set_theme(match store.theme.as_deref() {
            Some("light") => egui::ThemePreference::Light,
            Some("dark") => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
        });
        ExamApp {
            bank,
            store,
            path,
            screen: Screen::Profiles,
            message,
            new_name: String::new(),
            new_call: String::new(),
            confirm_delete: None,
            stats_class: None,
        }
    }

    fn save(&mut self) {
        if let Err(e) = self.store.save(&self.path) {
            self.message = Some(e);
        }
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::OpenProfile(id) => {
                self.store.last_profile = Some(id.clone());
                self.save();
                self.screen = Screen::Home(id);
            }
            Action::AddProfile => {
                let name = self.new_name.trim();
                if name.is_empty() {
                    return;
                }
                let p = Profile::new(name, &self.new_call, now());
                let id = p.id.clone();
                self.store.profiles.push(p);
                self.new_name.clear();
                self.new_call.clear();
                self.apply(Action::OpenProfile(id));
            }
            Action::DeleteProfile(id) => {
                self.store.profiles.retain(|p| p.id != id);
                if self.store.last_profile.as_deref() == Some(&id) {
                    self.store.last_profile = None;
                }
                self.save();
            }
            Action::ToProfiles => self.screen = Screen::Profiles,
            Action::SetClass(pid, class) => {
                if let Some(p) = self.store.profile_mut(&pid) {
                    p.last_class = Some(class);
                }
                self.save();
            }
            Action::Start { pid, class, probe, from_start, wrong_only } => {
                let Some(profile) = self.store.profile_mut(&pid) else { return };
                let (list, pos) = if wrong_only {
                    (profile.wrong(&self.bank, class, probe), 0)
                } else {
                    let list = self.bank.set(class, probe).to_vec();
                    let pos = if from_start { 0 } else { profile.position(class, probe) };
                    let pos = if pos >= list.len() { 0 } else { pos };
                    profile.set_position(class, probe, pos);
                    (list, pos)
                };
                if list.is_empty() {
                    return;
                }
                self.save();
                self.screen = Screen::Practice(Practice::new(pid, class, probe, list, pos, wrong_only));
            }
            Action::ToHome(pid) => self.screen = Screen::Home(pid),
            Action::ToStats(pid) => {
                self.stats_class = self.store.profile(&pid).and_then(|p| p.last_class);
                self.screen = Screen::Stats(pid);
            }
            Action::ToAllStats => self.screen = Screen::AllStats,
            Action::SetTheme(pref) => {
                self.store.theme = Some(
                    match pref {
                        egui::ThemePreference::Light => "light",
                        egui::ThemePreference::Dark => "dark",
                        egui::ThemePreference::System => "system",
                    }
                    .into(),
                );
                self.save();
            }
        }
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "dejavu".into(),
        Arc::new(egui::FontData::from_static(include_bytes!("../assets/fonts/DejaVuSans.ttf"))),
    );
    fonts.font_data.insert(
        "dejavu-bold".into(),
        Arc::new(egui::FontData::from_static(include_bytes!("../assets/fonts/DejaVuSans-Bold.ttf"))),
    );
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "dejavu".into());
    fonts
        .families
        .insert(
            FontFamily::Name("bold".into()),
            vec!["dejavu-bold".into(), "dejavu".into(), "NotoEmoji-Regular".into(), "emoji-icon-font".into()],
        );
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::new(26.0, FontFamily::Name("bold".into()))),
            (TextStyle::Body, FontId::proportional(17.0)),
            (TextStyle::Button, FontId::proportional(16.0)),
            (TextStyle::Small, FontId::proportional(13.5)),
            (TextStyle::Monospace, FontId::monospace(15.0)),
        ]
        .into();
        // Text mai contrastant decât implicit: întrebările se citesc mult.
        let (text, strong) = if s.visuals.dark_mode {
            (Color32::from_gray(215), Color32::from_gray(245))
        } else {
            (Color32::from_gray(35), Color32::from_gray(10))
        };
        s.visuals.widgets.noninteractive.fg_stroke.color = text;
        s.visuals.widgets.inactive.fg_stroke.color = text;
        s.visuals.widgets.hovered.fg_stroke.color = strong;
        s.visuals.widgets.active.fg_stroke.color = strong;
        s.spacing.item_spacing = Vec2::new(10.0, 8.0);
        s.spacing.button_padding = Vec2::new(14.0, 7.0);
        s.spacing.interact_size.y = 30.0;
    });
}

fn bold(text: impl Into<String>) -> RichText {
    RichText::new(text).family(FontFamily::Name("bold".into()))
}

fn fmt_date(ts: i64) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(ts, 0).single() {
        Some(d) if ts > 0 => d.format("%d.%m.%Y %H:%M").to_string(),
        _ => "–".into(),
    }
}

fn fmt_duration(secs: u64) -> String {
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    match (h, m) {
        (0, 0) => "sub un minut".into(),
        (0, m) => format!("{m} min"),
        (h, m) => format!("{h} h {m} min"),
    }
}

fn fmt_pct(p: Option<f32>) -> String {
    p.map(|p| format!("{p:.0}%")).unwrap_or_else(|| "–".into())
}

/// Conținut centrat, cu lățime maximă, ca textul să nu se întindă pe tot ecranul.
fn centered<R>(ui: &mut Ui, max_width: f32, add: impl FnOnce(&mut Ui) -> R) -> R {
    // Lăsăm loc pentru bara de derulare.
    let avail = ui.available_width() - 20.0;
    let w = avail.min(max_width);
    ui.horizontal_top(|ui| {
        ui.add_space(((avail - w) / 2.0).max(0.0));
        ui.vertical(|ui| {
            ui.set_width(w);
            add(ui)
        })
        .inner
    })
    .inner
}

fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .inner_margin(Margin::same(14))
        .corner_radius(CornerRadius::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

// ------------------------------------------------------------------ bucla principală

impl eframe::App for ExamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(msg) = self.message.clone() {
            egui::TopBottomPanel::bottom("mesaj").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(Color32::from_rgb(210, 70, 50), format!("⚠ {msg}"));
                    if ui.button("OK").clicked() {
                        self.message = None;
                    }
                });
            });
        }

        let mut actions = Vec::new();
        let ExamApp { bank, store, screen, new_name, new_call, confirm_delete, stats_class, path, message } = self;

        match screen {
            Screen::Profiles => profiles_screen(ctx, bank, store, new_name, new_call, confirm_delete, &mut actions),
            Screen::Home(pid) => home_screen(ctx, bank, store, pid, &mut actions),
            Screen::Practice(p) => {
                if let Some(next) = practice_screen(ctx, bank, store, p, path, message) {
                    *screen = next;
                }
            }
            Screen::Done { pid, class, probe, wrong_only, count } => {
                done_screen(ctx, bank, store, pid, *class, *probe, *wrong_only, *count, &mut actions)
            }
            Screen::Stats(pid) => stats_screen(ctx, bank, store, pid, stats_class, &mut actions),
            Screen::AllStats => all_stats_screen(ctx, bank, store, &mut actions),
        }

        for a in actions {
            self.apply(a);
        }
    }
}

fn top_bar(ctx: &egui::Context, title: &str, add_right: impl FnOnce(&mut Ui)) {
    egui::TopBottomPanel::top("sus")
        .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(Margin::symmetric(16, 10)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(bold("ExamYO").size(20.0));
                if !title.is_empty() {
                    ui.label(RichText::new(title).size(17.0));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), add_right);
            });
        });
}

// ------------------------------------------------------------------ profiluri

fn profiles_screen(
    ctx: &egui::Context,
    bank: &Bank,
    store: &Store,
    new_name: &mut String,
    new_call: &mut String,
    confirm_delete: &mut Option<String>,
    actions: &mut Vec<Action>,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            centered(ui, 640.0, |ui| {
                ui.add_space(28.0);
                ui.label(bold("ExamYO").size(36.0));
                ui.label(RichText::new("Pregătire pentru examenul de radioamator").size(19.0).weak());
                ui.add_space(22.0);

                let mut profiles: Vec<&Profile> = store.profiles.iter().collect();
                profiles.sort_by_key(|p| std::cmp::Reverse(p.last_active));

                if !profiles.is_empty() {
                    ui.label(bold("Cine învață acum?").size(18.0));
                    for p in profiles {
                        card(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(bold(p.display_name()).size(18.0));
                                    let class = p.last_class.map(|c| c.name()).unwrap_or_else(|| "clasă nealeasă".into());
                                    let progress = p
                                        .last_class
                                        .map(|c| {
                                            let s = p.class_stats(bank, c);
                                            format!(" · {} din {} întrebări parcurse", s.seen, s.total)
                                        })
                                        .unwrap_or_default();
                                    ui.label(RichText::new(format!("{class}{progress}")).weak());
                                    ui.label(
                                        RichText::new(format!("Ultima activitate: {}", fmt_date(p.last_active)))
                                            .small()
                                            .weak(),
                                    );
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.button("🗑").on_hover_text("Șterge utilizatorul").clicked() {
                                        *confirm_delete = Some(p.id.clone());
                                    }
                                    if ui.button(bold("Continuă ▶")).clicked() {
                                        actions.push(Action::OpenProfile(p.id.clone()));
                                    }
                                });
                            });
                        });
                    }
                    ui.add_space(16.0);
                }

                card(ui, |ui| {
                    ui.label(bold("Utilizator nou").size(18.0));
                    let mut submit = false;
                    egui::Grid::new("nou").num_columns(2).spacing([12.0, 10.0]).show(ui, |ui| {
                        ui.label("Nume:");
                        let r = ui.add(egui::TextEdit::singleline(new_name).desired_width(300.0).hint_text("ex. Ion Popescu"));
                        submit |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                        ui.end_row();
                        ui.label("Indicativ:");
                        let r = ui.add(
                            egui::TextEdit::singleline(new_call).desired_width(230.0).hint_text("opțional, ex. YO3ABC"),
                        );
                        submit |= r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                        ui.end_row();
                    });
                    let ok = !new_name.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new(bold("Adaugă și începe"))).clicked() || (submit && ok) {
                        actions.push(Action::AddProfile);
                    }
                });

                if !store.profiles.is_empty() {
                    ui.add_space(10.0);
                    if ui.button("📊 Statistici pentru toți utilizatorii").clicked() {
                        actions.push(Action::ToAllStats);
                    }
                }

                ui.add_space(30.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Temă:").small().weak());
                    let current = ui.ctx().options(|o| o.theme_preference);
                    for (pref, name) in [
                        (egui::ThemePreference::System, "ca sistemul"),
                        (egui::ThemePreference::Light, "luminoasă"),
                        (egui::ThemePreference::Dark, "întunecată"),
                    ] {
                        if ui.selectable_label(current == pref, RichText::new(name).small()).clicked() {
                            ui.ctx().set_theme(pref);
                            actions.push(Action::SetTheme(pref));
                        }
                    }
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new("Întrebările și rezolvările: Francisc Toth, YO6OWN, și contributorii lor —").small().weak());
                    ui.hyperlink_to(RichText::new("examyo.scienceontheweb.net").small(), SITE);
                });
                let commit = bank.upstream_commit.as_deref().map(|c| &c[..c.len().min(7)]).unwrap_or("?");
                ui.label(
                    RichText::new(format!(
                        "Licență GNU GPL v2+ · versiunea {} · date upstream {commit} · progresul se salvează lângă ExamYO.exe",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .small()
                    .weak(),
                );
            });
        });
    });

    if let Some(id) = confirm_delete.clone() {
        let name = store.profile(&id).map(|p| p.display_name()).unwrap_or_default();
        let modal = egui::Modal::new(egui::Id::new("sterge")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(bold("Ștergi utilizatorul?").size(19.0));
            ui.label(format!("{name} și tot progresul lui vor fi șterse. Operația nu poate fi anulată."));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Șterge").color(Color32::from_rgb(200, 60, 60))).clicked() {
                    actions.push(Action::DeleteProfile(id.clone()));
                    *confirm_delete = None;
                }
                if ui.button("Renunță").clicked() {
                    *confirm_delete = None;
                }
            });
        });
        if modal.should_close() {
            *confirm_delete = None;
        }
    }
}

// ------------------------------------------------------------------ acasă: clasă și probe

fn home_screen(ctx: &egui::Context, bank: &Bank, store: &Store, pid: &str, actions: &mut Vec<Action>) {
    let Some(profile) = store.profile(pid) else {
        actions.push(Action::ToProfiles);
        return;
    };

    top_bar(ctx, "", |ui| {
        if ui.button("Schimbă utilizatorul").clicked() {
            actions.push(Action::ToProfiles);
        }
        if ui.button("📊 Statisticile mele").clicked() {
            actions.push(Action::ToStats(pid.to_string()));
        }
        ui.label(bold(profile.display_name()));
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            centered(ui, CONTENT_WIDTH, |ui| {
                ui.add_space(12.0);
                ui.label(bold("Clasa pentru care te pregătești").size(18.0));
                ui.horizontal(|ui| {
                    for c in Class::ALL {
                        let sel = profile.last_class == Some(c);
                        let b = egui::Button::new(bold(c.name()).size(17.0)).selected(sel).min_size(Vec2::new(110.0, 40.0));
                        if ui.add(b).clicked() && !sel {
                            actions.push(Action::SetClass(pid.to_string(), c));
                        }
                    }
                });

                let Some(class) = profile.last_class else {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Alege clasa ca să vezi probele.").weak());
                    return;
                };
                if matches!(class, Class::I | Class::II) {
                    ui.label(
                        RichText::new("Include și întrebările claselor inferioare. Le iei la rând, începând cu cele mai ușoare.")
                            .small()
                            .weak(),
                    );
                }
                ui.add_space(14.0);

                for &probe in class.probes() {
                    probe_card(ui, bank, profile, pid, class, probe, actions);
                    ui.add_space(6.0);
                }
            });
        });
    });
}

fn probe_card(ui: &mut Ui, bank: &Bank, profile: &Profile, pid: &str, class: Class, probe: Probe, actions: &mut Vec<Action>) {
    let s = profile.stats(bank, class, probe);
    let pos = profile.position(class, probe);
    let levels = Bank::levels(class, probe);
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(bold(probe.name()).size(20.0));
            if levels.len() > 1 {
                ui.label(RichText::new(levels.join(" → ")).small().weak());
            }
        });
        ui.label(format!(
            "{} întrebări · parcurse {} · corecte din prima {} · greșite acum {}",
            s.total,
            s.seen,
            fmt_pct(s.first_correct_pct()),
            s.wrong_now
        ));
        ui.add(ProgressBar::new(s.seen_fraction()).desired_height(8.0));
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            let start = |from_start, wrong_only| Action::Start {
                pid: pid.to_string(),
                class,
                probe,
                from_start,
                wrong_only,
            };
            let main = if pos == 0 && s.seen == 0 {
                "Începe ▶".to_string()
            } else if pos < s.total {
                format!("Continuă de la întrebarea {} ▶", pos + 1)
            } else {
                "Ai terminat — ia de la început ▶".to_string()
            };
            if ui.button(bold(main)).clicked() {
                actions.push(start(pos >= s.total, false));
            }
            if pos > 0 && pos < s.total && ui.button("Ia de la început").clicked() {
                actions.push(start(true, false));
            }
            if ui
                .add_enabled(s.wrong_now > 0, egui::Button::new(format!("Doar greșelile ({})", s.wrong_now)))
                .clicked()
            {
                actions.push(start(false, true));
            }
        });
    });
}

// ------------------------------------------------------------------ parcurgere

/// Întoarce un ecran nou când utilizatorul iese din parcurgere.
fn practice_screen(
    ctx: &egui::Context,
    bank: &Bank,
    store: &mut Store,
    p: &mut Practice,
    path: &PathBuf,
    message: &mut Option<String>,
) -> Option<Screen> {
    let Some(profile_name) = store.profile(&p.pid).map(|x| x.display_name()) else {
        return Some(Screen::Profiles);
    };
    let total = p.list.len();
    let q: &Question = &bank.questions[p.list[p.pos]];

    // --- intrări de la tastatură
    let mut verify = false;
    let mut next = false;
    let mut prev = false;
    let mut back = false;
    ctx.input_mut(|i| {
        if !p.answered {
            let keys = [
                (Key::Num1, Key::A),
                (Key::Num2, Key::B),
                (Key::Num3, Key::C),
                (Key::Num4, Key::D),
            ];
            for (d, (k1, k2)) in keys.into_iter().enumerate() {
                if i.consume_key(Modifiers::NONE, k1) || i.consume_key(Modifiers::NONE, k2) {
                    p.selected = Some(d);
                }
            }
        }
        if i.consume_key(Modifiers::NONE, Key::Enter) {
            if p.answered {
                next = true;
            } else if p.selected.is_some() {
                verify = true;
            }
        }
        if p.answered && i.consume_key(Modifiers::NONE, Key::E) {
            p.show_explanation = !p.show_explanation;
        }
        next |= i.consume_key(Modifiers::NONE, Key::ArrowRight);
        prev |= i.consume_key(Modifiers::NONE, Key::ArrowLeft);
        back |= i.consume_key(Modifiers::NONE, Key::Escape);
    });

    // --- bara de sus
    let mode = if p.wrong_only { " · doar greșelile" } else { "" };
    top_bar(ctx, &format!("· {} · {}{mode}", p.class.name(), p.probe.name()), |ui| {
        ui.label(bold(format!("Întrebarea {} din {total}", p.pos + 1)));
        ui.label(RichText::new(&profile_name).weak());
        if ui.button("◀ Înapoi la probe").clicked() {
            back = true;
        }
    });
    egui::TopBottomPanel::top("progres").show_separator_line(false).show(ctx, |ui| {
        ui.add(ProgressBar::new((p.pos as f32) / total as f32).desired_height(4.0));
    });

    // --- butoanele de jos
    egui::TopBottomPanel::bottom("navigare")
        .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(Margin::symmetric(16, 12)))
        .show(ctx, |ui| {
            centered(ui, CONTENT_WIDTH, |ui| {
                ui.horizontal(|ui| {
                    if ui.add_enabled(p.pos > 0, egui::Button::new("◀ Anterioara")).clicked() {
                        prev = true;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if p.answered {
                            if ui.button(bold("Următoarea ▶")).on_hover_text("Enter").clicked() {
                                next = true;
                            }
                            let label = if p.show_explanation { "Ascunde rezolvarea" } else { "Arată rezolvarea" };
                            if ui.button(label).on_hover_text("E").clicked() {
                                p.show_explanation = !p.show_explanation;
                            }
                        } else {
                            if ui
                                .add_enabled(p.selected.is_some(), egui::Button::new(bold("Verifică răspunsul")))
                                .on_hover_text("Enter")
                                .clicked()
                            {
                                verify = true;
                            }
                            if ui.button("Sari peste ▶").clicked() {
                                next = true;
                            }
                        }
                    });
                });
            });
        });

    // --- întrebarea
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().id_salt(("intrebare", p.pos)).show(ui, |ui| {
            centered(ui, CONTENT_WIDTH, |ui| {
                ui.add_space(14.0);
                let tag: Vec<String> = [
                    (!q.level.is_empty()).then(|| q.level.to_string()),
                    q.ancom_code.as_ref().map(|c| format!("subiect ANCOM {c}")),
                ]
                .into_iter()
                .flatten()
                .collect();
                if !tag.is_empty() {
                    ui.label(RichText::new(tag.join(" · ")).small().weak());
                }
                ui.label(RichText::new(&q.text).size(20.0));
                if let Some(f) = &q.figure {
                    ui.add_space(6.0);
                    figure_ui(ui, f);
                }
                ui.add_space(12.0);

                for d in 0..4 {
                    if answer_ui(ui, p, q, d) && !p.answered {
                        p.selected = Some(d);
                    }
                    ui.add_space(2.0);
                }

                if p.answered {
                    ui.add_space(8.0);
                    let chosen = p.order[p.selected.unwrap()];
                    if chosen == q.correct {
                        ui.label(bold("✔ Corect!").size(19.0).color(green_stroke()));
                    } else {
                        let shown = p.order.iter().position(|&o| o == q.correct).unwrap();
                        ui.label(
                            bold(format!("✘ Greșit. Răspunsul corect este {}.", LETTERS[shown]))
                                .size(19.0)
                                .color(red_stroke()),
                        );
                    }
                    if p.show_explanation {
                        ui.add_space(6.0);
                        explanation_ui(ui, q);
                    }
                    if let Some(c) = &q.contributor {
                        ui.add_space(10.0);
                        ui.label(RichText::new(format!("Întrebare propusă de: {c}")).small().weak());
                    }
                } else {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Alege un răspuns (click sau tastele 1–4 / A–D), apoi „Verifică răspunsul”.")
                            .small()
                            .weak(),
                    );
                }
                ui.add_space(20.0);
            });
        });
    });

    // --- efectele acțiunilor
    let mut changed = false;
    if verify && !p.answered {
        if let Some(sel) = p.selected {
            let correct = p.order[sel] == q.correct;
            let secs = p.touch();
            if let Some(profile) = store.profile_mut(&p.pid) {
                profile.record(&q.key, correct, now());
                profile.add_time(secs);
                if !p.wrong_only {
                    profile.set_position(p.class, p.probe, p.pos + 1);
                }
            }
            p.answered = true;
            changed = true;
        }
    }

    let mut result = None;
    if back {
        result = Some(Screen::Home(p.pid.clone()));
    } else if next {
        if p.pos + 1 >= total {
            result = Some(Screen::Done {
                pid: p.pid.clone(),
                class: p.class,
                probe: p.probe,
                wrong_only: p.wrong_only,
                count: total,
            });
            if !p.wrong_only {
                if let Some(profile) = store.profile_mut(&p.pid) {
                    profile.set_position(p.class, p.probe, total);
                }
                changed = true;
            }
        } else {
            p.go(p.pos + 1);
            changed |= remember_position(store, p);
        }
    } else if prev && p.pos > 0 {
        p.go(p.pos - 1);
        changed |= remember_position(store, p);
    }

    if changed {
        if let Err(e) = store.save(path) {
            *message = Some(e);
        }
    }
    result
}

fn remember_position(store: &mut Store, p: &mut Practice) -> bool {
    let secs = p.touch();
    if let Some(profile) = store.profile_mut(&p.pid) {
        profile.add_time(secs);
        if !p.wrong_only {
            profile.set_position(p.class, p.probe, p.pos);
        }
        return true;
    }
    false
}

fn green_stroke() -> Color32 {
    Color32::from_rgb(46, 160, 80)
}
fn red_stroke() -> Color32 {
    Color32::from_rgb(210, 70, 60)
}

/// Desenează un răspuns; întoarce true dacă s-a dat click pe el.
fn answer_ui(ui: &mut Ui, p: &Practice, q: &Question, d: usize) -> bool {
    let orig = p.order[d];
    let dark = ui.visuals().dark_mode;
    let is_correct = orig == q.correct;
    let is_selected = p.selected == Some(d);

    let resp = ui
        .scope_builder(UiBuilder::new().id_salt(("raspuns", d)).sense(if p.answered { Sense::hover() } else { Sense::click() }), |ui| {
            let response = ui.response();
            let v = ui.visuals();
            let (fill, stroke) = if p.answered && is_correct {
                (if dark { Color32::from_rgb(28, 74, 40) } else { Color32::from_rgb(214, 241, 219) }, Stroke::new(2.0_f32, green_stroke()))
            } else if p.answered && is_selected {
                (if dark { Color32::from_rgb(90, 34, 32) } else { Color32::from_rgb(250, 218, 214) }, Stroke::new(2.0_f32, red_stroke()))
            } else if is_selected {
                (v.selection.bg_fill, Stroke::new(2.0_f32, v.selection.stroke.color))
            } else if response.hovered() && !p.answered {
                (v.widgets.hovered.weak_bg_fill, v.widgets.hovered.bg_stroke)
            } else {
                (v.widgets.inactive.weak_bg_fill, Stroke::new(1.0_f32, v.widgets.noninteractive.bg_stroke.color))
            };
            egui::Frame::new()
                .fill(fill)
                .stroke(stroke)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(14, 11))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        ui.label(bold(format!("{})", LETTERS[d])).size(18.0));
                        let mark = if p.answered && is_correct {
                            "  ✔"
                        } else if p.answered && is_selected {
                            "  ✘"
                        } else {
                            ""
                        };
                        ui.add(egui::Label::new(RichText::new(format!("{}{mark}", q.answers[orig])).size(18.0)).wrap());
                    });
                });
        })
        .response;
    resp.clicked()
}

fn explanation_ui(ui: &mut Ui, q: &Question) {
    let v = ui.visuals();
    egui::Frame::new()
        .fill(v.faint_bg_color)
        .stroke(Stroke::new(1.0_f32, v.widgets.noninteractive.bg_stroke.color))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(bold("Rezolvare").size(18.0));
            match &q.explanation {
                Some(text) => {
                    ui.add(egui::Label::new(RichText::new(text).size(16.5)).wrap());
                    if let Some(f) = &q.explanation_figure {
                        figure_ui(ui, f);
                    }
                    if let Some(a) = &q.explanation_author {
                        ui.label(RichText::new(format!("— {a}")).small().weak());
                    }
                }
                None => {
                    ui.label("Pentru această întrebare nu există încă o rezolvare.");
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Dacă o știi, o poți propune autorului:").small().weak());
                        ui.hyperlink_to(RichText::new("Colectare erori și sugestii").small(), FEEDBACK);
                    });
                }
            }
        });
}

fn figure_ui(ui: &mut Ui, f: &Figure) {
    let Some(bytes) = examyo_core::embedded_image(&f.name) else {
        ui.label(RichText::new(format!("[figura {} lipsește]", f.name)).weak());
        return;
    };
    let max_w = ui.available_width();
    ui.vertical_centered(|ui| {
        // Figurile sunt desene negre pe fundal transparent: fundal alb ca să se vadă și în modul întunecat.
        egui::Frame::new()
            .fill(Color32::WHITE)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                let scale = if f.width.unwrap_or(400) < 300 { 1.5 } else { 1.25 };
                ui.add(
                    egui::Image::from_bytes(format!("bytes://{}", f.name), bytes)
                        .fit_to_original_size(scale)
                        .max_width(max_w - 16.0),
                );
            });
    });
}

// ------------------------------------------------------------------ finalul unei probe

#[allow(clippy::too_many_arguments)]
fn done_screen(
    ctx: &egui::Context,
    bank: &Bank,
    store: &Store,
    pid: &str,
    class: Class,
    probe: Probe,
    wrong_only: bool,
    count: usize,
    actions: &mut Vec<Action>,
) {
    let Some(profile) = store.profile(pid) else {
        actions.push(Action::ToProfiles);
        return;
    };
    let s = profile.stats(bank, class, probe);
    top_bar(ctx, &format!("· {} · {}", class.name(), probe.name()), |ui| {
        ui.label(RichText::new(profile.display_name()).weak());
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        centered(ui, 640.0, |ui| {
            ui.add_space(40.0);
            ui.label(bold("🎉 Gata!").size(30.0));
            let what = if wrong_only {
                format!("Ai reluat cele {count} întrebări greșite din proba {}.", probe.name())
            } else {
                format!("Ai ajuns la capătul probei {} ({count} întrebări).", probe.name())
            };
            ui.label(RichText::new(what).size(18.0));
            ui.add_space(10.0);
            card(ui, |ui| stats_lines(ui, &s));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if s.wrong_now > 0 && ui.button(bold(format!("Reia greșelile ({})", s.wrong_now))).clicked() {
                    actions.push(Action::Start { pid: pid.into(), class, probe, from_start: false, wrong_only: true });
                }
                if ui.button("Ia proba de la început").clicked() {
                    actions.push(Action::Start { pid: pid.into(), class, probe, from_start: true, wrong_only: false });
                }
                if ui.button("Înapoi la probe").clicked() {
                    actions.push(Action::ToHome(pid.into()));
                }
            });
        });
    });
}

fn stats_lines(ui: &mut Ui, s: &Stats) {
    ui.label(format!("Parcurse: {} din {}", s.seen, s.total));
    ui.label(format!("Corecte din prima: {} ({})", s.first_correct, fmt_pct(s.first_correct_pct())));
    ui.label(format!("Greșite la ultima încercare: {}", s.wrong_now));
}

// ------------------------------------------------------------------ statistici

fn stats_screen(
    ctx: &egui::Context,
    bank: &Bank,
    store: &Store,
    pid: &str,
    stats_class: &mut Option<Class>,
    actions: &mut Vec<Action>,
) {
    let Some(profile) = store.profile(pid) else {
        actions.push(Action::ToProfiles);
        return;
    };
    top_bar(ctx, "· Statistici", |ui| {
        if ui.button("◀ Înapoi la probe").clicked() {
            actions.push(Action::ToHome(pid.into()));
        }
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            centered(ui, CONTENT_WIDTH, |ui| {
                ui.add_space(12.0);
                ui.label(bold(profile.display_name()).size(24.0));
                ui.label(RichText::new(format!(
                    "Utilizator din {} · ultima activitate {} · timp de studiu {}",
                    fmt_date(profile.created),
                    fmt_date(profile.last_active),
                    fmt_duration(profile.time_secs)
                ))
                .weak());
                let attempts: u32 = profile.answers.values().map(|a| a.attempts).sum();
                ui.label(RichText::new(format!(
                    "{} întrebări diferite, {attempts} răspunsuri date în total",
                    profile.answers.len()
                ))
                .weak());
                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    for c in Class::ALL {
                        if ui.selectable_label(*stats_class == Some(c), c.name()).clicked() {
                            *stats_class = Some(c);
                        }
                    }
                });
                let Some(class) = *stats_class else {
                    ui.label(RichText::new("Alege o clasă ca să vezi statisticile pe probe.").weak());
                    return;
                };
                ui.add_space(8.0);

                card(ui, |ui| {
                    egui::Grid::new("statistici").striped(true).num_columns(5).spacing([24.0, 8.0]).show(ui, |ui| {
                        for h in ["Proba", "Întrebări", "Parcurse", "Corecte din prima", "Greșite acum"] {
                            ui.label(bold(h));
                        }
                        ui.end_row();
                        let mut weakest: Option<(Probe, f32)> = None;
                        for &probe in class.probes() {
                            let s = profile.stats(bank, class, probe);
                            stats_row(ui, probe.name(), &s, false);
                            if let Some(pct) = s.first_correct_pct().filter(|_| s.seen >= 10) {
                                if weakest.is_none_or(|(_, w)| pct < w) {
                                    weakest = Some((probe, pct));
                                }
                            }
                        }
                        stats_row(ui, "Total", &profile.class_stats(bank, class), true);
                        ui.end_row();
                        if let Some((probe, pct)) = weakest {
                            ui.label("");
                            ui.end_row();
                            ui.label(RichText::new(format!("Cel mai slab: {} ({pct:.0}% corecte din prima)", probe.name())).color(red_stroke()));
                            ui.end_row();
                        }
                    });
                });
                ui.label(
                    RichText::new("„Corecte din prima” = la câte întrebări din cele parcurse ai răspuns corect la prima încercare.")
                        .small()
                        .weak(),
                );
            });
        });
    });
}

fn stats_row(ui: &mut Ui, name: &str, s: &Stats, total: bool) {
    let t = |x: String| if total { bold(x) } else { RichText::new(x) };
    ui.label(t(name.to_string()));
    ui.label(t(s.total.to_string()));
    ui.label(t(format!("{} ({:.0}%)", s.seen, 100.0 * s.seen_fraction())));
    ui.label(t(match s.first_correct_pct() {
        Some(p) => format!("{} ({p:.0}%)", s.first_correct),
        None => "–".into(),
    }));
    ui.label(t(s.wrong_now.to_string()));
    if !total {
        ui.end_row();
    }
}

fn all_stats_screen(ctx: &egui::Context, bank: &Bank, store: &Store, actions: &mut Vec<Action>) {
    top_bar(ctx, "· Toți utilizatorii", |ui| {
        if ui.button("◀ Înapoi").clicked() {
            actions.push(Action::ToProfiles);
        }
    });
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            centered(ui, 1100.0, |ui| {
                ui.add_space(12.0);
                ui.label(bold("Progresul tuturor utilizatorilor").size(22.0));
                ui.label(RichText::new("Pentru clasa aleasă de fiecare.").weak());
                ui.add_space(8.0);
                let mut profiles: Vec<&Profile> = store.profiles.iter().collect();
                profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                card(ui, |ui| {
                    egui::Grid::new("toti").striped(true).num_columns(7).spacing([22.0, 8.0]).show(ui, |ui| {
                        for h in ["Utilizator", "Clasa", "Parcurse", "Corecte din prima", "Timp de studiu", "Ultima activitate", ""] {
                            ui.label(bold(h));
                        }
                        ui.end_row();
                        for p in profiles {
                            ui.label(p.display_name());
                            match p.last_class {
                                Some(c) => {
                                    let s = p.class_stats(bank, c);
                                    ui.label(c.roman());
                                    ui.label(format!("{} / {} ({:.0}%)", s.seen, s.total, 100.0 * s.seen_fraction()));
                                    ui.label(fmt_pct(s.first_correct_pct()));
                                }
                                None => {
                                    ui.label("–");
                                    ui.label("–");
                                    ui.label("–");
                                }
                            }
                            ui.label(fmt_duration(p.time_secs));
                            ui.label(fmt_date(p.last_active));
                            if ui.small_button("Detalii").clicked() {
                                actions.push(Action::ToStats(p.id.clone()));
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        });
    });
}

// ------------------------------------------------------------------ iconiță

/// Iconița ferestrei, desenată din cod: un pilon de antenă cu unde, pe fond albastru.
pub fn icon() -> egui::IconData {
    const N: usize = 64;
    let mut rgba = vec![0u8; N * N * 4];
    let c = (N as f32 - 1.0) / 2.0;
    for y in 0..N {
        for x in 0..N {
            let (fx, fy) = (x as f32 - c, y as f32 - c);
            let r = (fx * fx + fy * fy).sqrt();
            let i = (y * N + x) * 4;
            if r > 31.5 {
                continue;
            }
            let mut px = [32u8, 92, 170, 255];
            // pilonul
            let mast = fx.abs() < 2.2 && fy > -8.0 && fy < 22.0;
            let legs = fy > 6.0 && fy < 22.0 && ((fx - (fy - 6.0) * 0.45).abs() < 1.8 || (fx + (fy - 6.0) * 0.45).abs() < 1.8);
            // undele, deasupra vârfului
            let (wx, wy) = (fx, fy + 10.0);
            let wr = (wx * wx + wy * wy).sqrt();
            let up = wy < -1.0 && wx.abs() < -wy * 1.3;
            let waves = up && [7.0, 13.0, 19.0].iter().any(|&k| (wr - k).abs() < 1.6);
            if mast || legs || waves || wr < 3.0 {
                px = [255, 255, 255, 255];
            }
            rgba[i..i + 4].copy_from_slice(&px);
        }
    }
    egui::IconData { rgba, width: N as u32, height: N as u32 }
}

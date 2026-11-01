// Fără fereastră de consolă pe Windows în varianta finală.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ExamYO – pregătire examen radioamator · realizat de Eugen, YO9VA")
            .with_inner_size([1100.0, 820.0])
            .with_min_inner_size([760.0, 560.0])
            .with_icon(app::icon()),
        ..Default::default()
    };
    eframe::run_native("ExamYO", options, Box::new(|cc| Ok(Box::new(app::ExamApp::new(cc)))))
}

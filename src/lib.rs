//! Logica aplicației, separată de interfață, ca să poată fi refolosită
//! (de exemplu într-o variantă de Android).

pub mod data;
pub mod store;

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// Conținutul fișierului all.json din examyo-db, inclus la compilare.
pub fn embedded_json() -> &'static [u8] {
    embedded::ALL_JSON
}

/// Octeții unei figuri incluse la compilare, după numele fișierului.
pub fn embedded_image(name: &str) -> Option<&'static [u8]> {
    embedded::IMAGES
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| embedded::IMAGES[i].1)
}

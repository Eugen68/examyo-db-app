//! Include în executabil întrebările (examyo-db/data/all.json) și figurile lor,
//! ca aplicația să fie un singur .exe portabil.

use std::{env, fs, path::Path};

fn main() {
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let db = Path::new(&root).join("examyo-db");
    let json = db.join("data").join("all.json");
    let images = db.join("images");

    if !json.exists() {
        panic!(
            "Lipsește {}. Rulează: git submodule update --init examyo-db",
            json.display()
        );
    }
    println!("cargo:rerun-if-changed={}", json.display());
    println!("cargo:rerun-if-changed={}", images.display());

    let mut names: Vec<_> = fs::read_dir(&images)
        .expect("lipsește examyo-db/images")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();

    let mut out = String::new();
    out.push_str(&format!(
        "pub static ALL_JSON: &[u8] = include_bytes!({:?});\n",
        json.display().to_string()
    ));
    out.push_str("pub static IMAGES: &[(&str, &[u8])] = &[\n");
    for n in &names {
        out.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            n,
            images.join(n).display().to_string()
        ));
    }
    out.push_str("];\n");

    let dest = Path::new(&env::var("OUT_DIR").unwrap()).join("embedded.rs");
    fs::write(dest, out).unwrap();

    // Pe Windows: iconița și detaliile din „Proprietăți” ale fișierului .exe.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/examyo.ico");
        winresource::WindowsResource::new()
            .set_icon("assets/examyo.ico")
            .compile()
            .expect("nu pot include iconița în executabil");
    }
}

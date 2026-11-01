//! Exportă cheile întrebărilor și mărimea probelor, ca testele variantei de Android
//! să verifice că dau exact aceleași rezultate (progresul e portabil între PC și telefon).

use examyo_core::data::{Bank, Class};

fn main() {
    let b = Bank::load(examyo_core::embedded_json()).unwrap();
    for q in &b.questions {
        println!("Q\t{}\t{}\t{}", q.source_id, q.key, q.ancom_code.as_deref().unwrap_or(""));
    }
    for c in Class::ALL {
        for &p in c.probes() {
            let first = b.set(c, p).first().map(|&i| b.questions[i].source_id.as_str()).unwrap_or("");
            println!("S\t{c:?}/{p:?}\t{}\t{first}", b.set(c, p).len());
        }
    }
}

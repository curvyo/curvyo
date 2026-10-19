//! Times fontdb listing all system faces (and optionally an extra directory).
use std::time::Instant;

fn main() {
    for round in 0..3 {
        let t = Instant::now();
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let sys = t.elapsed();
        let n_sys = db.len();
        let extra = std::env::args().nth(1).map(|dir| {
            let t = Instant::now();
            db.load_fonts_dir(&dir);
            (t.elapsed(), db.len() - n_sys)
        });
        let families: std::collections::BTreeSet<String> = db
            .faces()
            .filter_map(|f| f.families.first().map(|(n, _)| n.clone()))
            .collect();
        print!(
            "round {round}: system faces {n_sys} in {:.1} ms",
            sys.as_secs_f64() * 1e3
        );
        if let Some((d, n)) = extra {
            print!("; extra dir faces {n} in {:.1} ms", d.as_secs_f64() * 1e3);
        }
        println!("; families {}", families.len());
    }
}

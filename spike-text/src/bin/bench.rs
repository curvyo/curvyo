//! Native benchmark: shape + outline 500 characters; kerning and ligature checks.
use std::time::Instant;

/// 500 characters of Latin text with kerning pairs (AV, To, Wa, Ty) and ligatures (fi, fl, ff).
pub fn text500() -> String {
    let base = "AVATAR Toffee: the office waffle flavour fits five fine fjords. \
                Typewriters, LAVA, Yacht, We fly off. fi fl ff ffi ffl! ";
    let mut s = String::new();
    while s.chars().count() < 500 {
        s.push_str(base);
    }
    s.chars().take(500).collect()
}

fn stats(mut v: Vec<f64>) -> (f64, f64, f64) {
    v.sort_by(f64::total_cmp);
    (v[0], v[v.len() / 2], v[v.len() * 99 / 100])
}

fn main() {
    let path = std::env::args().nth(1).expect("font path");
    let bytes = std::fs::read(&path).expect("read font");
    let text = text500();
    assert_eq!(text.chars().count(), 500);

    // Correctness: kerning and ligature.
    let face = spike_text::Face::new(bytes.clone(), 0).expect("parse");
    let av = face.shape_and_outline("AV", 20.0).expect("shape");
    let kern_shaped: i32 = av.advances.iter().sum();
    let nominal = face.nominal_advance('A').unwrap() + face.nominal_advance('V').unwrap();
    println!(
        "AV shaped advance {kern_shaped} vs nominal A+V {nominal} (font units): kern {}",
        kern_shaped as f32 - nominal
    );
    let fi = face.shape_and_outline("fi", 20.0).expect("shape");
    println!(
        "\"fi\" -> {} glyph(s) {:?}",
        fi.glyph_ids.len(),
        fi.glyph_ids
    );
    let one = face.shape_and_outline(&text, 20.0).expect("shape");
    println!(
        "500 chars -> {} glyphs, {} path floats, {} draw errors",
        one.glyph_ids.len(),
        one.path.len(),
        one.draw_errors
    );

    let n = 300;
    let mut cold = Vec::new();
    for _ in 0..n {
        let t = Instant::now();
        let r = spike_text::shape_cold(&bytes, &text).expect("cold");
        std::hint::black_box(r);
        cold.push(t.elapsed().as_secs_f64() * 1e3);
    }
    let mut warm = Vec::new();
    for _ in 0..n {
        let t = Instant::now();
        let r = face.shape_and_outline(&text, 20.0).expect("warm");
        std::hint::black_box(r);
        warm.push(t.elapsed().as_secs_f64() * 1e3);
    }
    let (a, b, c) = stats(cold);
    println!("harfrust+skrifa cold (parse+shape+outline) ms: min {a:.3} median {b:.3} p99 {c:.3}");
    let (a, b, c) = stats(warm);
    println!("harfrust+skrifa warm (shape+outline)       ms: min {a:.3} median {b:.3} p99 {c:.3}");

    let rface = rustybuzz_face(&bytes);
    if let Some(f) = rface {
        let mut v = Vec::new();
        for _ in 0..n {
            let t = Instant::now();
            std::hint::black_box(spike_text::rb::shape_and_outline(&f, &text));
            v.push(t.elapsed().as_secs_f64() * 1e3);
        }
        let (a, b, c) = stats(v);
        println!(
            "rustybuzz+ttf-parser warm                  ms: min {a:.3} median {b:.3} p99 {c:.3}"
        );
    }
}

fn rustybuzz_face(bytes: &[u8]) -> Option<rustybuzz::Face<'_>> {
    rustybuzz::Face::from_slice(bytes, 0)
}

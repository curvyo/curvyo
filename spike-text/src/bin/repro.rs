//! Re-runs one saved case and prints the panic message (fallback pair only).
fn main() {
    let p = std::env::args().nth(1).expect("case file");
    let data = std::fs::read(&p).expect("read");
    let text =
        "AVATAR Toffee: the office waffle flavour fits five fine fjords. Typewriters, LAVA, Yacht.";
    let a = std::panic::catch_unwind(|| {
        if let Ok(f) = spike_text::Face::new(data.clone(), 0) {
            let _ = f.shape_and_outline(text, 20.0);
            let _ = f.outline_all();
        }
    });
    println!("harfrust+skrifa panicked: {}", a.is_err());
    let r = std::panic::catch_unwind(|| {
        if let Some(face) = rustybuzz::Face::from_slice(&data, 0) {
            let _ = spike_text::rb::shape_and_outline(&face, text);
        }
    });
    println!("rustybuzz shape+outline panicked: {}", r.is_err());
}

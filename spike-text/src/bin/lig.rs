//! Prints glyph ids for ligature candidates under harfrust and rustybuzz.
fn main() {
    let path = std::env::args().nth(1).expect("font path");
    let bytes = std::fs::read(&path).expect("read");
    let face = spike_text::Face::new(bytes.clone(), 0).expect("parse");
    let rb = rustybuzz::Face::from_slice(&bytes, 0).expect("rb");
    for s in ["fi", "fl", "ff", "ffi", "ffl", "AV", "To", "Wa"] {
        let h = face.shape_and_outline(s, 20.0).expect("shape");
        let mut b = rustybuzz::UnicodeBuffer::new();
        b.push_str(s);
        b.guess_segment_properties();
        let g = rustybuzz::shape(&rb, &[], b);
        let rg: Vec<u32> = g.glyph_infos().iter().map(|i| i.glyph_id).collect();
        let ra: Vec<i32> = g.glyph_positions().iter().map(|p| p.x_advance).collect();
        println!(
            "{s}: harfrust {:?} adv {:?} | rustybuzz {rg:?} adv {ra:?}",
            h.glyph_ids, h.advances
        );
    }
}

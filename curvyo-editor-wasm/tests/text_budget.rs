//! Criterion 3 of `specs/0074-text-objects`: a typed character appears in the next drawn frame,
//! at most 50 ms after the key, on a text of 500 characters. Measured as the session's work for
//! one key and the frame after it. Asserted in release builds; a debug build prints the numbers.
//! Run: `cargo test --release -p curvyo-editor-wasm --test text_budget -- --nocapture`.
//! Machine of the plan: dev VM `impa` (8 vCPU Xeon).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use curvyo_document_core::Point;
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{FontBlock, TextInput};

const BUDGET: Duration = Duration::from_millis(50);

#[test]
fn ac03_a_key_on_a_500_character_text_is_drawn_within_50_ms() {
    let fonts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
    let file = "source-serif-4/SourceSerif4-Regular.ttf";
    let mut session = Session::new(1);
    session.set_font_list(
        FontBlock::Included,
        vec![(
            file.to_owned(),
            "Source Sans 3".to_owned(),
            "Regular".to_owned(),
            0,
        )],
        Vec::new(),
    );
    session.add_font_file(file, &std::fs::read(fonts.join(file)).unwrap());
    session.set_tool(Tool::Text);
    session.pointer_down(Point::new(10.0, 20.0), false);
    session.pointer_up(Point::new(10.0, 20.0), false, false);
    let line = "AVATAR Toffee: the office waffle flavour fits five fine fjords. ";
    let text: String = line.chars().cycle().take(500).collect();
    session.text_input(TextInput::Insert(text));
    let _ = session.draw_list();
    let mut worst = Duration::ZERO;
    for c in "Typing!".chars() {
        let started = Instant::now();
        session.text_input(TextInput::Insert(c.to_string()));
        let frame = session.draw_list();
        worst = worst.max(started.elapsed());
        assert!(frame.triangle_count() > 0);
    }
    println!("one key and its frame, 500 characters: worst {worst:?} (budget {BUDGET:?})");
    if !cfg!(debug_assertions) {
        assert!(worst <= BUDGET, "{worst:?} is over {BUDGET:?}");
    }
}

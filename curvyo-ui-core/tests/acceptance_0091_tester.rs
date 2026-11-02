//! Tester cases for `specs/0091-trace-text-recognition` round 2, the garbage filter of Frage
//! 115 B / 117 B (Terms, "Accepted line"; criterion 4): a line without a sure letter becomes text
//! only with at least 2 characters, most of them letters or digits, a line confidence at or above
//! the threshold (0.8, adrs.md), and not one short pattern repeated. Each case sits on one side of
//! one boundary; the other conditions hold. Written from the specification.

use curvyo_ui_core::OcrLine;

fn line(text: &str, confidence: f64) -> OcrLine {
    OcrLine {
        quad: [[0.0, 0.0], [100.0, 0.0], [100.0, 20.0], [0.0, 20.0]],
        text: text.to_owned(),
        confidence: vec![confidence; text.chars().count()],
        positions: Vec::new(),
    }
}

#[test]
fn filter_one_character_fails_two_pass() {
    assert!(!line("A", 0.99).plausible(), "one character");
    assert!(!line("7", 0.99).plausible(), "one digit");
    assert!(
        !line(" A ", 0.99).plausible(),
        "one character between spaces"
    );
    assert!(line("Ab", 0.99).plausible(), "two characters");
    assert!(line("A7", 0.99).plausible(), "a letter and a digit");
}

#[test]
fn filter_mostly_symbols_fails_mostly_letters_pass() {
    assert!(!line("+-*/a", 0.99).plausible(), "1 of 5 letters");
    assert!(!line("a+", 0.99).plausible(), "half letters is not most");
    assert!(!line("ab+-", 0.99).plausible(), "2 of 4 is not most");
    assert!(line("ab+", 0.99).plausible(), "2 of 3 letters");
    assert!(
        line("Tel. 0421", 0.99).plausible(),
        "7 of 8 letters or digits"
    );
    assert!(
        line("12.50 €", 0.99).plausible(),
        "4 of 6 letters or digits"
    );
}

#[test]
fn filter_confidence_boundary_at_0_8() {
    assert!(
        !line("Zoll", 0.79).plausible(),
        "0.79 is under the threshold"
    );
    assert!(
        !line("Zoll", 0.799_999).plausible(),
        "just under the threshold"
    );
    assert!(line("Zoll", 0.80).plausible(), "0.80 is at the threshold");
    // The confidence comes from the OCR as f32: 0.8 f32 widened is a hair above 0.8.
    assert!(
        line("Zoll", f64::from(0.8_f32)).plausible(),
        "0.8 as the OCR's f32"
    );
    // The threshold is on the line's mean: one weak letter among strong ones passes.
    let mut mixed = line("Zoll", 0.95);
    mixed.confidence[1] = 0.5;
    assert!(mixed.plausible(), "mean 0.8375");
    let mut weak = line("Zoll", 0.95);
    weak.confidence[1] = 0.2;
    assert!(!weak.plausible(), "mean 0.7625");
}

#[test]
fn filter_a_short_repeated_pattern_fails() {
    for text in [
        "llll", "ililil", "TOTOTO", "OO", "0000", "abcabc", "lI lI lI",
    ] {
        assert!(
            !line(text, 0.99).plausible(),
            "{text:?} is one short pattern repeated"
        );
    }
    for text in ["Zoll", "Ill", "ilil x", "TOTOTOX", "Mono", "abcdabcd"] {
        assert!(
            line(text, 0.99).plausible(),
            "{text:?} is no repeated short pattern"
        );
    }
}

#[test]
fn filter_the_ring_and_the_dot_grid_readings_fail() {
    // Criterion 18: the 0025 ring read as "O" and the dot grid read as "TOTOTO" stay shapes even
    // when the OCR is sure of them.
    assert!(!line("O", 1.0).plausible());
    assert!(!line("TOTOTO", 1.0).plausible());
}

#[test]
fn filter_an_empty_or_unconfident_line_fails() {
    assert!(!line("", 0.99).plausible());
    let mut none = line("Zoll", 0.99);
    none.confidence.clear();
    assert!(!none.plausible(), "no confidences");
}

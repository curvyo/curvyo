//! Upside-down and steep text in Trace end to end (`specs/0091-trace-text-recognition` slice 3,
//! criteria 30, 31, 33 and 35): the slice 3 fixtures read, traced with the text pass, fitted to
//! the included faces and created. Angles are the text's turn in document space (y down), as
//! criterion 8's test: a line reading bottom to top ("+90°" on screen) is -90. The 180° poster
//! is the `#[ignore]` harness that prints the numbers of the PR:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --lib trace_text_turned_tests -- --ignored --nocapture
//! ```

use super::trace_text_colour_tests::{colour_params, run as colour_run};
use super::trace_text_tests::{POSTER, Run, levenshtein, model, run, wrong_sure};

/// The turn of `placement` in degrees.
fn degrees(placement: [f64; 6]) -> f64 {
    placement[1].atan2(placement[0]).to_degrees()
}

/// How far `got` is from `want`, degrees in -180 to 180.
fn off(got: f64, want: f64) -> f64 {
    (got - want + 540.0).rem_euclid(360.0) - 180.0
}

/// The text of `run` nearest `want`, which must be within one letter of it.
fn text_near<'a>(run: &'a Run, want: &str) -> &'a (String, Vec<usize>, f64, [f64; 6]) {
    let text = run
        .texts
        .iter()
        .min_by_key(|t| levenshtein(&t.0, want))
        .unwrap_or_else(|| panic!("no text for {want:?}: {run:?}"));
    assert!(levenshtein(&text.0, want) <= 1, "{want:?}: {run:?}");
    text
}

/// Each `(string, degrees)` of `wanted` is a text of `run` within one letter and 1°, with no
/// wrong sure letter.
fn assert_turned(run: &Run, wanted: &[(&str, f64)]) {
    assert_eq!(run.texts.len(), wanted.len(), "{run:?}");
    for (want, angle) in wanted {
        let text = text_near(run, want);
        let got = degrees(text.3);
        assert!(off(got, *angle).abs() <= 1.0, "{want:?} turned {got}");
        assert_eq!(wrong_sure(&text.0, &text.1, want), 0, "{text:?}");
    }
}

/// Criteria 31 and 33 (and 35: one step, one undo, the same twice): the steep picture gives both
/// lines reading the right way round, the mixed one its upright line unturned and its
/// upside-down line turned by 180°.
#[test]
fn steep_and_upside_down_lines_are_texts_the_right_way_round() {
    let model = model();
    let steep = run(&model, "turned/steep.png");
    assert_turned(
        &steep,
        &[("Steil nach oben", -90.0), ("Steil nach unten", 90.0)],
    );
    let mixed = run(&model, "turned/mixed.png");
    assert_turned(&mixed, &[("Oben richtig", 0.0), ("Kopfstand 180", 180.0)]);
    let again = run(&model, "turned/mixed.png");
    assert_eq!(
        (again.texts, again.faces, again.paths),
        (mixed.texts, mixed.faces, mixed.paths),
        "criterion 13"
    );
}

/// Criterion 35 in Multicolour: the mixed picture gives the same two texts, turned the same.
#[test]
fn the_mixed_picture_reads_the_same_in_multicolour() {
    let model = model();
    let (colour, _) = colour_run(&model, "turned/mixed.png", &colour_params());
    let plain = run(&model, "turned/mixed.png");
    let strings = |texts: Vec<(&String, [f64; 6])>| {
        texts
            .into_iter()
            .map(|(s, m)| (s.clone(), degrees(m).round()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        strings(colour.texts.iter().map(|t| (&t.0, t.4)).collect()),
        strings(plain.texts.iter().map(|t| (&t.0, t.3)).collect()),
    );
    assert!(colour.texts_on_top, "{colour:?}");
}

/// Criteria 30 and 34: the reference poster turned by 180° gives the upright poster's strings
/// (within one letter; "BÄCKEREI" exact, lead 2026-11-03: the turned crop is padded at both
/// ends, so its last letter is not lost), faces and sure letters, 0 wrong sure letters, each text turned by 180°
/// from its upright one; prints the times of both.
#[test]
#[ignore = "harness: prints the numbers of the PR"]
fn the_upside_down_poster_reads_as_the_upright_one() {
    let model = model();
    let upright = run(&model, "poster.png");
    let turned = run(&model, "turned/poster_180.png");
    for (name, r) in [("upright", &upright), ("180°", &turned)] {
        println!(
            "{name}: {} texts, {} paths; thread CPU OCR {:.2} s, text pass and trace {:.2} s, fit {:.2} s",
            r.texts.len(),
            r.paths,
            r.ocr_seconds,
            r.pass_seconds,
            r.fit_seconds
        );
        for (t, face) in r.texts.iter().zip(&r.faces) {
            println!(
                "  {:?} {face} guessed {:?} turned {:.2}",
                t.0,
                t.1,
                degrees(t.3)
            );
        }
    }
    assert_eq!(turned.paths, upright.paths, "the shapes whole");
    assert!(
        turned
            .texts
            .iter()
            .any(|t| t.0 == "BÄCKEREI" && t.1.is_empty()),
        "{turned:?}"
    );
    let mut wrong = 0;
    for (k, up) in upright.texts.iter().enumerate() {
        let (j, down) = turned
            .texts
            .iter()
            .enumerate()
            .min_by_key(|(_, t)| levenshtein(&t.0, &up.0))
            .unwrap();
        assert!(levenshtein(&down.0, &up.0) <= 1, "{up:?} vs {down:?}");
        assert_eq!(turned.faces[j], upright.faces[k], "{:?}", up.0);
        if down.0 == up.0 {
            assert_eq!(down.1, up.1, "the same sure letters of {:?}", up.0);
        }
        let turn = off(degrees(down.3), degrees(up.3) + 180.0);
        assert!(turn.abs() <= 1.0, "{:?} turned {turn} from 180", up.0);
        let truth = POSTER
            .iter()
            .min_by_key(|l| levenshtein(&down.0, l))
            .unwrap();
        wrong += wrong_sure(&down.0, &down.1, truth);
    }
    assert_eq!(wrong, 0, "wrong sure letters");
}

/// The process's CPU time in seconds: natively `rten` runs on `rayon`'s threads, which the
/// thread's CPU time misses (in the wasm worker it all runs on one thread, ADR 0028 §4).
#[allow(clippy::cast_precision_loss)] // seconds and nanoseconds of a test run
fn process_seconds() -> f64 {
    #[cfg(unix)]
    {
        let t = rustix::time::clock_gettime(rustix::time::ClockId::ProcessCPUTime);
        t.tv_sec as f64 + t.tv_nsec as f64 * 1e-9
    }
    #[cfg(not(unix))]
    {
        0.0
    }
}

/// Criterion 34: the CPU time of the OCR (run with `--test-threads=1` and
/// `RAYON_NUM_THREADS=1`, so the process's CPU time is this read's) on the upright poster, the
/// 180° poster, the steep and the mixed picture and the customer's screenshot, where only unsure
/// or steep lines are read twice.
#[test]
#[ignore = "harness: prints the numbers of the PR"]
fn the_extra_reads_cost() {
    let model = model();
    for file in [
        "poster.png",
        "turned/poster_180.png",
        "turned/steep.png",
        "turned/mixed.png",
        "customer/font-manager-screenshot.png",
    ] {
        let (mut session, image) = super::trace_text_tests::session_with(file);
        let source = session.trace_source(image, false).unwrap();
        let started = process_seconds();
        let lines = vectrace::ocr::accept::trace_read_text(
            &model,
            &source.rgba,
            source.width,
            source.height,
            &mut |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
        let seconds = process_seconds() - started;
        println!("{file}: {} lines, OCR {seconds:.2} s CPU", lines.len());
    }
}

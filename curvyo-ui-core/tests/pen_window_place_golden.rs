//! Golden files of `pen_window.json` (`specs/0158-pen-display` slice 2a criterion 16, 2b criteria
//! 4 and 5, ADR 0096 A4, `CLAUDE.md` §5): format 2 reads and writes byte-equal; format 1 still
//! reads, with both switches off; damaged, newer and unknown-field files read as none; an
//! off-screen place opens centred on the main screen.
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_ui_core::pen_window_place::{
    Opening, OpeningSize, PenWindowFlags, PenWindowPlace, ScreenRect, opening, read_pen_window,
    read_pen_window_flags, write_pen_window,
};

const GOLDEN: &str = include_str!("fixtures/pen_window/pen_window_v2.json");
const GOLDEN_V1: &str = include_str!("fixtures/pen_window/pen_window_v1.json");

const KAMVAS_PLACE: PenWindowPlace = PenWindowPlace {
    width: 1920,
    height: 1080,
    position: Some((2560, 0)),
    maximised: true,
};

#[test]
fn the_golden_reads_and_writes_byte_equal() {
    let place = read_pen_window(GOLDEN).unwrap();
    let flags = read_pen_window_flags(GOLDEN);
    assert_eq!(place, KAMVAS_PLACE);
    assert_eq!(
        flags,
        PenWindowFlags {
            follow_view: true,
            canvas_only: false,
        }
    );
    assert_eq!(write_pen_window(&place, flags), GOLDEN);
}

#[test]
fn the_format_1_golden_still_reads_with_both_switches_off() {
    assert_eq!(read_pen_window(GOLDEN_V1), Some(KAMVAS_PLACE));
    assert_eq!(read_pen_window_flags(GOLDEN_V1), PenWindowFlags::default());
}

#[test]
fn damaged_newer_and_unknown_field_files_read_as_none() {
    for text in [
        include_str!("fixtures/pen_window/pen_window_damaged.json"),
        include_str!("fixtures/pen_window/pen_window_newer.json"),
        include_str!("fixtures/pen_window/pen_window_unknown_field.json"),
    ] {
        assert_eq!(read_pen_window(text), None);
    }
}

#[test]
fn an_off_screen_file_opens_centred_on_the_main_screen() {
    let saved = read_pen_window(include_str!(
        "fixtures/pen_window/pen_window_off_screen.json"
    ));
    let screen = ScreenRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1200,
    };
    assert_eq!(
        opening(saved.as_ref(), &[screen], Some(screen)),
        Opening {
            size: OpeningSize::Physical(1800, 1000),
            position: Some((60, 100)),
            maximised: false,
        }
    );
}

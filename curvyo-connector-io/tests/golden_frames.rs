//! Golden files for the frames of this crate: the four `PairSpake` shapes and the four reasons of
//! `PairRefused` (addendum A6).
#![allow(clippy::unwrap_used)]

use curvyo_connector_io::frames::{Frame, IoFrame, RefusalReason, decode, encode};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path).unwrap().trim_end().to_owned()
}

fn spake(round: u8, client_name: Option<&str>) -> IoFrame {
    IoFrame::PairSpake {
        round,
        client_name: client_name.map(str::to_owned),
        message: "00".repeat(if round == 1 { 33 } else { 32 }),
    }
}

#[test]
fn every_frame_matches_its_golden_file_both_ways() {
    let cases = [
        ("pair_spake_app_1.json", spake(1, Some("Studio laptop"))),
        ("pair_spake_connector_1.json", spake(1, None)),
        ("pair_spake_app_2.json", spake(2, None)),
        ("pair_spake_connector_2.json", spake(2, None)),
        (
            "pair_refused_wrong_code.json",
            IoFrame::PairRefused {
                reason: RefusalReason::WrongCode,
                attempts_left: 2,
            },
        ),
        (
            "pair_refused_code_expired.json",
            IoFrame::PairRefused {
                reason: RefusalReason::CodeExpired,
                attempts_left: 1,
            },
        ),
        (
            "pair_refused_no_code.json",
            IoFrame::PairRefused {
                reason: RefusalReason::NoCode,
                attempts_left: 0,
            },
        ),
    ];
    for (file, frame) in cases {
        let golden = fixture(file);
        assert_eq!(encode(&Frame::Io(frame.clone())).unwrap(), golden, "{file}");
        assert_eq!(decode(&golden).unwrap(), Frame::Io(frame), "{file}");
    }
}

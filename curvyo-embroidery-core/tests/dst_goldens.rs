//! The DST golden files of `specs/0160-embroidery` criteria 12, 14 to 18 (`tests/fixtures/
//! embroidery_*.dst`): fixed designs on the EU7 page (hoop centre at 95, 145 mm), written, read
//! back by the crate's own reader and compared. `CURVYO_UPDATE_GOLDEN=1` rewrites the files
//! natively; review the diff. The goldens are embedded with `include_bytes!`, so the same
//! comparison runs in Node on `wasm32-unknown-unknown` (`adrs.md` EM9):
//!
//! ```text
//! cargo test --target wasm32-unknown-unknown -p curvyo-embroidery-core --test dst_goldens
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::items_after_statements,
    clippy::type_complexity,
    missing_docs
)]

use curvyo_document_core::{Point, Vec2};
use curvyo_embroidery_core::{
    DST_MAX_STITCH, EmbroiderySettings, Extents, ItemOutline, Mm, OpKind, PlanItem, PlanOperation,
    StitchKind, StitchPlan, StitchesPerMin, ThreadColour, build_plan, extents, read_dst, summarise,
    write_dst,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::wasm_bindgen_test as test;

const CENTRE: Point = Point { x: 95.0, y: 145.0 };

fn line(points: &[(f64, f64)], closed: bool) -> ItemOutline {
    ItemOutline {
        anchors: points
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed,
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> ItemOutline {
    line(&[(x, y), (x + w, y), (x + w, y + h), (x, y + h)], true)
}

fn op(kind: OpKind, thread: &str, items: Vec<(Vec<ItemOutline>, f64)>) -> PlanOperation {
    PlanOperation {
        kind,
        settings: EmbroiderySettings::default(),
        thread: ThreadColour::parse(thread).unwrap(),
        items: items
            .into_iter()
            .enumerate()
            .map(|(i, (outlines, stroke))| PlanItem {
                outlines,
                stroke_width: Mm::new(stroke).unwrap(),
                stacking: u32::try_from(i).unwrap(),
            })
            .collect(),
    }
}

const RED: &str = "#C8102E";
const BLUE: &str = "#1F4E9C";

fn running() -> Vec<PlanOperation> {
    vec![op(
        OpKind::Running,
        RED,
        vec![(vec![rect(75.0, 130.0, 40.0, 30.0)], 0.3)],
    )]
}

fn tatami() -> Vec<PlanOperation> {
    vec![op(
        OpKind::Tatami,
        RED,
        vec![(vec![rect(80.0, 135.0, 30.0, 20.0)], 0.3)],
    )]
}

fn satin() -> Vec<PlanOperation> {
    let column = line(&[(75.0, 145.0), (115.0, 145.0)], false);
    vec![op(OpKind::Satin, BLUE, vec![(vec![column], 3.0)])]
}

/// The design of criterion 19: a running outline with the 10 mm centre cross of `adrs.md` EM10,
/// a tatami fill inside it, both red, and a blue satin line.
fn mvp_two_colours() -> Vec<PlanOperation> {
    let cross_h = line(&[(90.0, 145.0), (100.0, 145.0)], false);
    let cross_v = line(&[(95.0, 140.0), (95.0, 150.0)], false);
    vec![
        op(
            OpKind::Running,
            RED,
            vec![
                (vec![rect(65.0, 115.0, 60.0, 60.0)], 0.3),
                (vec![cross_h], 0.3),
                (vec![cross_v], 0.3),
            ],
        ),
        op(
            OpKind::Tatami,
            RED,
            vec![(vec![rect(75.0, 120.0, 40.0, 15.0)], 0.3)],
        ),
        op(
            OpKind::Satin,
            BLUE,
            vec![(vec![line(&[(75.0, 165.0), (115.0, 165.0)], false)], 3.0)],
        ),
    ]
}

fn plan(ops: &[PlanOperation]) -> StitchPlan {
    build_plan(ops, CENTRE, DST_MAX_STITCH)
}

const GOLDENS: [(&str, &[u8], fn() -> Vec<PlanOperation>); 4] = [
    (
        "embroidery_running.dst",
        include_bytes!("fixtures/embroidery_running.dst"),
        running,
    ),
    (
        "embroidery_tatami.dst",
        include_bytes!("fixtures/embroidery_tatami.dst"),
        tatami,
    ),
    (
        "embroidery_satin.dst",
        include_bytes!("fixtures/embroidery_satin.dst"),
        satin,
    ),
    (
        "embroidery_mvp_two_colours.dst",
        include_bytes!("fixtures/embroidery_mvp_two_colours.dst"),
        mvp_two_colours,
    ),
];

fn written(name: &str, design: fn() -> Vec<PlanOperation>) -> Vec<u8> {
    let label = name
        .trim_start_matches("embroidery_")
        .trim_end_matches(".dst");
    write_dst(label, &plan(&design()).stitches).unwrap()
}

#[test]
fn criterion_14_16_the_four_designs_write_their_golden_files() {
    for (name, golden, design) in GOLDENS {
        let bytes = written(name, design);
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name);
            std::fs::write(path, &bytes).unwrap();
            continue;
        }
        assert_eq!(bytes, golden, "{name}");
    }
}

#[test]
fn criterion_17_the_goldens_read_back_to_the_stitch_list_and_its_extents() {
    for (name, golden, design) in GOLDENS {
        let read = read_dst(golden).unwrap();
        let stitches = plan(&design()).stitches;
        assert_eq!(read.stitches, stitches, "{name}: every needle point");
        assert_eq!(read.extents, extents(&read.stitches), "{name}");
        assert_eq!(read.extents, extents(&stitches), "{name}");
        assert_eq!(read.records * 3 + 512, golden.len(), "{name}");
    }
}

#[test]
fn criterion_12_the_summary_of_the_golden_design_equals_its_read_back() {
    let speed = StitchesPerMin::new(600).unwrap();
    let stitches = plan(&mvp_two_colours()).stitches;
    let read = read_dst(GOLDENS[3].1).unwrap();
    let ours = summarise(&stitches, speed);
    assert_eq!(ours, summarise(&read.stitches, speed));
    assert_eq!(ours.colours, 2, "one colour stop (criterion 19)");
    assert_eq!(read.colour_changes, 1);
}

#[test]
fn criterion_14_a_10_mm_square_centred_in_the_hoop_reaches_50_each_way() {
    let ops = vec![op(
        OpKind::Running,
        RED,
        vec![(vec![rect(90.0, 140.0, 10.0, 10.0)], 0.3)],
    )];
    let all = Extents {
        plus_x: 50,
        minus_x: 50,
        plus_y: 50,
        minus_y: 50,
    };
    assert_eq!(extents(&plan(&ops).stitches), all);
}

#[test]
fn criterion_14_an_off_centre_square_counts_up_and_right_from_the_centre() {
    // 5 to 15 mm right of the centre and 5 to 15 mm above it (the page's Y points down).
    let ops = vec![op(
        OpKind::Running,
        RED,
        vec![(vec![rect(100.0, 130.0, 10.0, 10.0)], 0.3)],
    )];
    let e = extents(&plan(&ops).stitches);
    assert_eq!(
        e,
        Extents {
            plus_x: 150,
            minus_x: 0,
            plus_y: 150,
            minus_y: 0
        }
    );
}

#[test]
fn criterion_15_a_30_mm_travel_is_three_jumps() {
    let ops = vec![op(
        OpKind::Running,
        RED,
        vec![(vec![line(&[(125.0, 145.0), (135.0, 145.0)], false)], 0.3)],
    )];
    let stitches = plan(&ops).stitches;
    let kinds: Vec<StitchKind> = stitches.iter().take(4).map(|s| s.kind).collect();
    assert_eq!(
        kinds,
        [
            StitchKind::Jump,
            StitchKind::Jump,
            StitchKind::Jump,
            StitchKind::Normal
        ]
    );
    let bytes = write_dst("Travel", &stitches).unwrap();
    assert_eq!(
        &bytes[512..515],
        [0b0000_1001, 0b0000_0100, 0b1000_0111],
        "+100 = 81 + 27 - 9 + 1, a jump"
    );
}

#[test]
fn criterion_18_two_writes_are_byte_identical() {
    for (name, _, design) in GOLDENS {
        assert_eq!(written(name, design), written(name, design), "{name}");
    }
}

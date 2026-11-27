//! `specs/0161-cnc-output` criteria 6, 8, 9, 12 to 14: the Profile toolpath, its passes and order,
//! and the GRBL goldens `cnc_grbl_profile_outside.nc`, `cnc_grbl_profile_inside.nc` and
//! `cnc_grbl_two_parts.nc`. `CURVYO_BLESS=1` rewrites the goldens.
#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::many_single_char_names)]

use curvyo_cnc_core::{
    CncJob, Depth, Direction, GrblHeader, ItemLoops, ItemOutline, JobOperation, MAX_LINE, MoveKind,
    ProfileSettings, Side, Stock, Tool, build_job, profile_loops, write_grbl,
};
use curvyo_document_core::{Length, Point, Vec2};

const TOL: f64 = 0.01;
const K: f64 = 0.552_284_749_830_793_4;

fn rect(x: f64, y: f64, w: f64, h: f64) -> ItemOutline {
    let p = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
    ItemOutline {
        anchors: p
            .iter()
            .map(|&(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
            .collect(),
        closed: true,
    }
}

fn circle(cx: f64, cy: f64, r: f64) -> ItemOutline {
    let k = K * r;
    let anchors = vec![
        (
            Point::new(cx + r, cy),
            Vec2::new(0.0, -k),
            Vec2::new(0.0, k),
        ),
        (
            Point::new(cx, cy + r),
            Vec2::new(k, 0.0),
            Vec2::new(-k, 0.0),
        ),
        (
            Point::new(cx - r, cy),
            Vec2::new(0.0, k),
            Vec2::new(0.0, -k),
        ),
        (
            Point::new(cx, cy - r),
            Vec2::new(-k, 0.0),
            Vec2::new(k, 0.0),
        ),
    ];
    ItemOutline {
        anchors,
        closed: true,
    }
}

fn loops(outlines: &[ItemOutline], side: Side) -> Vec<curvyo_cnc_core::ToolLoop> {
    match profile_loops(outlines, side, 3.175 / 2.0) {
        ItemLoops::Loops(l) => l,
        other => panic!("{other:?}"),
    }
}

/// The distance from `p` to the rectangle `(x, y, w, h)`'s boundary outside it.
fn outside_distance(p: Point, (x, y, w, h): (f64, f64, f64, f64)) -> f64 {
    let dx = (x - p.x).max(p.x - x - w).max(0.0);
    let dy = (y - p.y).max(p.y - y - h).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

#[test]
fn c6_outside_offsets_a_rectangle_outward_by_the_tool_radius_with_round_corners() {
    let l = loops(&[rect(20.0, 20.0, 50.0, 30.0)], Side::Outside);
    assert_eq!(l.len(), 1);
    for p in &l[0].points {
        let d = outside_distance(*p, (20.0, 20.0, 50.0, 30.0));
        assert!((d - 1.5875).abs() <= TOL, "{p:?} at {d}");
    }
}

#[test]
fn c6_inside_on_a_20_mm_circle_gives_16_825_mm() {
    let l = loops(&[circle(50.0, 50.0, 10.0)], Side::Inside);
    assert_eq!(l.len(), 1);
    for p in &l[0].points {
        let r = ((p.x - 50.0).powi(2) + (p.y - 50.0).powi(2)).sqrt();
        assert!((2.0 * r - 16.825).abs() <= 2.0 * TOL, "{r}");
    }
}

#[test]
fn c6_a_square_with_a_hole_keeps_part_and_hole_size() {
    let l = loops(
        &[rect(0.0, 0.0, 50.0, 50.0), circle(25.0, 25.0, 10.0)],
        Side::Outside,
    );
    assert_eq!(l.len(), 2);
    let hole = l.iter().find(|l| !l.outer).unwrap();
    for p in &hole.points {
        let r = ((p.x - 25.0).powi(2) + (p.y - 25.0).powi(2)).sqrt();
        assert!((2.0 * r - 16.825).abs() <= 2.0 * TOL, "{r}");
    }
    let outer = l.iter().find(|l| l.outer).unwrap();
    for p in &outer.points {
        let d = outside_distance(*p, (0.0, 0.0, 50.0, 50.0));
        assert!((d - 1.5875).abs() <= TOL);
    }
}

fn op(side: Side, depth: f64, items: Vec<Vec<ItemOutline>>) -> JobOperation {
    let tool = Tool::default_with_id("default");
    let mut settings = ProfileSettings::new(&tool, 10.0);
    settings.side = side;
    settings.depth = Depth::new(depth).unwrap();
    JobOperation {
        settings,
        tool,
        items,
    }
}

fn gcode(page: &str, ops: &[JobOperation]) -> (CncJob, String) {
    let stock = Stock::default();
    let job = build_job(ops, &stock, Length::from_mm(5.0));
    let tool = &ops[0].tool;
    let header = GrblHeader {
        page,
        tool,
        stock: &stock,
        safe: Length::from_mm(5.0),
    };
    let text = write_grbl(&job, &header, tool.rpm);
    (job, text)
}

fn golden(name: &str, text: &str) {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("CURVYO_BLESS").is_some() {
        std::fs::write(&path, text).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap();
    assert!(want == text, "{name} differs from its golden");
}

fn two_parts() -> Vec<JobOperation> {
    vec![op(
        Side::Outside,
        3.0,
        vec![
            vec![rect(20.0, 20.0, 40.0, 30.0), circle(40.0, 35.0, 6.0)],
            vec![rect(90.0, 20.0, 40.0, 30.0), circle(110.0, 35.0, 6.0)],
        ],
    )]
}

#[test]
fn c12_c14_the_goldens_are_written_byte_identical() {
    let (_, outside) = gcode(
        "Outside",
        &[op(
            Side::Outside,
            3.0,
            vec![vec![rect(20.0, 20.0, 50.0, 30.0)]],
        )],
    );
    golden("cnc_grbl_profile_outside.nc", &outside);
    let (_, inside) = gcode(
        "Inside",
        &[op(Side::Inside, 3.0, vec![vec![circle(50.0, 50.0, 10.0)]])],
    );
    golden("cnc_grbl_profile_inside.nc", &inside);
    let (_, two) = gcode("Two parts", &two_parts());
    golden("cnc_grbl_two_parts.nc", &two);
    assert_eq!(gcode("Two parts", &two_parts()).1, two, "byte-identical");
}

#[test]
fn c12_c13_the_lines_are_grbl_1_1_lines() {
    let (_, text) = gcode("Two parts", &two_parts());
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[4], "G21 G90 G17 G94");
    assert_eq!(lines[5], "M3 S12000");
    assert_eq!(lines[6], "G0 Z5");
    assert_eq!(&lines[lines.len() - 3..], ["M5", "G0 Z5", "M30"]);
    for line in &lines {
        assert!(line.len() <= MAX_LINE, "{line}");
        let code = line.split('(').next().unwrap();
        assert!(!code.contains(['?', '!', '~']), "{line}");
        assert!(!code.contains("M6") && !code.contains("G81") && !code.contains("G43"));
        for word in code.split_whitespace() {
            assert!(!word.contains('e'), "{word}");
            if let Some((_, decimals)) = word.split_once('.') {
                assert!(decimals.len() <= 3, "{word}");
            }
        }
    }
}

#[test]
fn c8_depth_10_with_step_down_3_runs_four_passes_ending_at_minus_10() {
    let mut o = op(
        Side::Outside,
        10.0,
        vec![vec![rect(20.0, 20.0, 50.0, 30.0)]],
    );
    o.tool.step_down = curvyo_cnc_core::StepDown::new(3.0).unwrap();
    let (job, _) = gcode("p", &[o]);
    let depths: Vec<f64> = job
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge)
        .map(|m| m.to.z())
        .collect();
    assert_eq!(depths, [-2.5, -5.0, -7.5, -10.0]);
}

#[test]
fn c9_holes_run_before_the_outline_that_holds_them() {
    let (job, _) = gcode("p", &two_parts());
    let plunges: Vec<_> = job
        .moves
        .iter()
        .filter(|m| m.kind == MoveKind::Plunge && m.pass == 1)
        .map(|m| m.to)
        .collect();
    assert_eq!(plunges.len(), 4);
    // Page y 35 is work y 115 on the 150 mm stock: the hole of the first part first.
    let near_hole = |p: curvyo_cnc_core::WorkPoint, cx: f64| {
        ((p.x() - cx).powi(2) + (p.y() - 115.0).powi(2)).sqrt() < 6.0
    };
    assert!(near_hole(plunges[0], 40.0), "{:?}", plunges[0]);
    assert!(!near_hole(plunges[1], 40.0) && !near_hole(plunges[1], 110.0));
    assert!(near_hole(plunges[2], 110.0));
}

fn work_area_of_pass(job: &CncJob) -> f64 {
    let cut: Vec<_> = job
        .moves
        .iter()
        .filter(|m| m.pass == 1 && m.kind != MoveKind::Rapid)
        .map(|m| m.to)
        .collect();
    let mut a = 0.0;
    for i in 0..cut.len() {
        let (p, q) = (cut[i], cut[(i + 1) % cut.len()]);
        a += p.x() * q.y() - q.x() * p.y();
    }
    a / 2.0
}

#[test]
fn c5_conventional_outside_is_counter_clockwise_and_climb_reverses_it() {
    let mut o = op(Side::Outside, 1.0, vec![vec![rect(20.0, 20.0, 50.0, 30.0)]]);
    assert!(work_area_of_pass(&gcode("p", std::slice::from_ref(&o)).0) > 0.0);
    o.settings.direction = Direction::Climb;
    assert!(work_area_of_pass(&gcode("p", std::slice::from_ref(&o)).0) < 0.0);
    let mut i = op(Side::Inside, 1.0, vec![vec![circle(50.0, 50.0, 10.0)]]);
    i.settings.direction = Direction::Climb;
    assert!(work_area_of_pass(&gcode("p", &[i]).0) > 0.0);
}

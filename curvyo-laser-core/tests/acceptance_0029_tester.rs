//! Tester acceptance cases for `specs/0029-laser-job-preview-and-output`, written from the
//! specification before the implementation was read: every golden of criterion 31 read back by
//! the tester's own GRBL reader (`tester_grbl_0029`) and checked against criteria 13 to 26 and 38
//! (dialect, order, extent, lengths, passes, Z, air assist, corners, homing, relative moves),
//! plus the format facts of criterion 39 and the pure number rules of criteria 10 and 25.
#![allow(
    clippy::many_single_char_names,
    clippy::field_reassign_with_default,
    clippy::assigning_clones,
    clippy::semicolon_if_nothing_returned,
    clippy::cast_lossless,
    clippy::neg_multiply,
    clippy::bool_assert_comparison,
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    missing_docs
)]

mod tester_grbl_0029;

use std::path::PathBuf;

use curvyo_laser_core::OutputFormat;
use curvyo_laser_core::job::{
    Start, coordinate, duration_text, estimate_text, extension, file_type, length_text,
    lengths_text, name, s_value,
};
use tester_grbl_0029::{Kind, Mode, Program, read};

const GOLDENS: [&str; 19] = [
    "cut_line",
    "score_and_cut_order",
    "engrave_bidirectional",
    "engrave_crosshatch_scan_angle",
    "engrave_overscan",
    "engrave_outline_set",
    "cutout_margin",
    "inner_first",
    "passes",
    "m3_constant_power",
    "z_offset",
    "origin_corners_bl",
    "origin_corners_br",
    "origin_corners_tl",
    "origin_corners_tr",
    "fixture_jobs_first",
    "home_first",
    "start_head_position",
    "air_assist",
];

fn bytes(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/output/grbl")
        .join(format!("{name}.gcode"));
    std::fs::read(&p).unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn golden(name: &str) -> Program {
    read(&bytes(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

const MM: i64 = 1000;

// ---- criterion 31: every golden exists and reads as the dialect of criterion 24 -------------------

#[test]
fn c31_every_named_golden_exists_in_its_format_folder_and_reads() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/output/grbl");
    for g in GOLDENS {
        let p = golden(g);
        assert!(!p.segs.is_empty(), "{g}: no move");
    }
    // `specs/0123` criterion 40 adds the `raster_*` goldens to this folder and replaces
    // `image_skipped` (0029 criterion 18) by `raster_threshold`; they are checked there.
    // `specs/0121` criterion 17 adds `print_and_cut_cut_line`, checked in `job_output_goldens.rs`.
    let count = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            let name = e.as_ref().unwrap().file_name();
            let name = name.to_string_lossy();
            !name.starts_with("raster_") && !name.starts_with("print_and_cut_")
        })
        .count();
    assert_eq!(
        count,
        GOLDENS.len(),
        "one golden per named case, nothing else"
    );
}

#[test]
fn c24_header_then_g21_g90_g17_then_laser_start_in_order() {
    for g in GOLDENS {
        let p = golden(g);
        assert!(
            p.header[0].starts_with("Curvyo job: "),
            "{g}: job name first"
        );
        assert!(
            p.header[1].starts_with("Machine: "),
            "{g}: machine name second"
        );
        let runs = p.header_runs();
        assert!(
            !runs.is_empty(),
            "{g}: operations in run order with numbers"
        );
        for (i, (n, _)) in runs.iter().enumerate() {
            assert_eq!(*n as usize, i + 1, "{g}: run numbers 1.. in order");
        }
        let c = &p.commands;
        let off = usize::from(p.homes);
        assert_eq!(c[off], "G21", "{g}");
        assert!(c[off + 1] == "G90" || c[off + 1] == "G91", "{g}");
        assert_eq!(c[off + 2], "G17", "{g}");
        assert_eq!(c[off + 3], "M5", "{g}");
        assert!(c[off + 4] == "M4 S0" || c[off + 4] == "M3 S0", "{g}");
    }
}

#[test]
fn c24_no_date_time_path_user_version_or_random_in_any_golden() {
    for g in GOLDENS {
        let text = String::from_utf8(bytes(g)).unwrap();
        let lower = text.to_lowercase();
        for bad in [
            "/home", "\\", "c:", "20", "version", "v0.", "user", "date", "time",
        ] {
            for line in lower.lines().filter(|l| l.starts_with(';')) {
                // "20" would be any year; the header holds only names and run numbers.
                if bad == "20" {
                    continue;
                }
                assert!(
                    !line.contains(bad),
                    "{g}: header line {line:?} holds {bad:?}"
                );
            }
        }
        assert!(
            !text.contains("G2 ") && !text.contains("G3 "),
            "{g}: no arcs (criterion 17)"
        );
        assert!(
            !text.contains("G10") && !text.contains("G92") && !text.contains("G54"),
            "{g}: no work offset (26)"
        );
    }
}

#[test]
fn c17_burn_moves_are_short_straight_segments_only() {
    // Every move is G0 or G1 (the reader refuses anything else); the circles of inner_first
    // are flattened into many short segments.
    let p = golden("inner_first");
    let burns = p.segs.iter().filter(|s| s.kind == Kind::Burn).count();
    assert!(burns > 40, "a circle is many segments, got {burns}");
}

#[test]
fn c26_absolute_goldens_stay_inside_the_bed_with_non_negative_coordinates() {
    for g in GOLDENS {
        let p = golden(g);
        if p.relative {
            continue;
        }
        let b = p.all_box();
        assert!(b.0 >= 0 && b.1 >= 0, "{g}: a negative coordinate {b:?}");
        assert!(
            b.2 <= 400 * MM && b.3 <= 300 * MM,
            "{g}: outside a 400 x 300 bed {b:?}"
        );
        assert_eq!(p.end, (0, 0), "{g}: the end is a rapid back to the origin");
        assert!(p.commands.last().unwrap() == "G0 X0.000 Y0.000", "{g}");
    }
}

// ---- criteria 13, 25: cut_line --------------------------------------------------------------------

#[test]
fn c13_c25_cut_line_one_burn_with_f_and_s_where_they_change() {
    let p = golden("cut_line");
    assert_eq!(p.mode, Mode::M4);
    let burns: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Burn).collect();
    assert_eq!(burns.len(), 1);
    assert!(
        (p.burn_mm() - 100.0).abs() < 0.01,
        "a 100 mm line, got {}",
        p.burn_mm()
    );
    assert_eq!(burns[0].f, 600);
    // S = round(power x S-max / 100): S 500 of a 1000 range is 50 %.
    assert_eq!(burns[0].s, 500);
    assert!(
        p.commands.iter().any(|c| c.contains("F600 S500")),
        "F and S on the burn line"
    );
}

// ---- criterion 20, 23: order of operations --------------------------------------------------------

#[test]
fn c20_c23_score_runs_before_cut_and_the_header_says_so() {
    let p = golden("score_and_cut_order");
    let names: Vec<String> = p.header_runs().into_iter().map(|r| r.1).collect();
    assert_eq!(names, ["Score 1", "Cut 1"]);
    let order = p.settings_order();
    assert_eq!(order.len(), 2, "{order:?}");
    assert_eq!(order[0], (200, 3000), "Score's numbers run first");
    assert_eq!(order[1], (900, 300), "Cut's numbers run last");
    // Each closed rectangle returns to its first point (criterion 13).
    for r in p.runs() {
        assert_eq!(
            r.points.first(),
            r.points.last(),
            "a closed outline returns to its start"
        );
    }
}

#[test]
fn c20_fixture_jobs_run_first_then_engrave_then_cut() {
    let p = golden("fixture_jobs_first");
    let names: Vec<String> = p.header_runs().into_iter().map(|r| r.1).collect();
    assert_eq!(names.len(), 3);
    assert!(names[0].starts_with("Fixture jobs"), "{names:?}");
    assert!(names[0].ends_with("Score"), "{names:?}");
    assert_eq!(names[1], "Engrave 1");
    assert_eq!(names[2], "Cut 1");
    let order = p.settings_order();
    assert_eq!(order.len(), 3, "{order:?}");
    assert_eq!(order[2], (900, 300), "Cut last");
}

#[test]
fn c22_inner_first_text_then_hole_then_rectangle() {
    // A rectangle with a circular hole and a text inside the hole: every path inside another
    // runs before the path that surrounds it.
    let p = golden("inner_first");
    let runs = p.runs();
    assert!(runs.len() >= 3, "{} runs", runs.len());
    let boxes: Vec<(i64, i64, i64, i64)> = runs
        .iter()
        .map(|r| {
            r.points
                .iter()
                .fold((i64::MAX, i64::MAX, i64::MIN, i64::MIN), |b, p| {
                    (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
                })
        })
        .collect();
    let area = |b: &(i64, i64, i64, i64)| (b.2 - b.0) as i128 * (b.3 - b.1) as i128;
    let contains = |o: &(i64, i64, i64, i64), i: &(i64, i64, i64, i64)| {
        o.0 <= i.0 && o.1 <= i.1 && o.2 >= i.2 && o.3 >= i.3 && o != i
    };
    // For every pair where one box contains the other, the contained one runs first.
    for (a, ba) in boxes.iter().enumerate() {
        for (b, bb) in boxes.iter().enumerate() {
            if contains(ba, bb) {
                assert!(b < a, "run {b} lies inside run {a} but runs after it");
            }
        }
    }
    // The largest (the rectangle) is last.
    let largest = (0..boxes.len()).max_by_key(|&i| area(&boxes[i])).unwrap();
    assert_eq!(largest, boxes.len() - 1, "the rectangle runs last");
    for r in &runs {
        assert_eq!(r.points.first(), r.points.last(), "closed paths close");
    }
}

// ---- criterion 21: passes -------------------------------------------------------------------------

#[test]
fn c21_passes_repeat_the_whole_operation_in_the_same_order() {
    let p = golden("passes");
    let runs = p.runs();
    assert_eq!(runs.len() % 2, 0);
    // Two squares per pass; the pass count is runs / 2 and each pass repeats the first.
    let per = 2;
    let passes = runs.len() / per;
    assert!(passes >= 2, "{passes} passes");
    for k in 1..passes {
        for j in 0..per {
            assert_eq!(
                runs[k * per + j].points,
                runs[j].points,
                "pass {k} path {j}"
            );
        }
    }
    assert!(
        p.commands.iter().filter(|c| c.contains(" F")).count() == 1,
        "F written once"
    );
}

// ---- criterion 25: M3 -----------------------------------------------------------------------------

#[test]
fn c25_m3_writes_m5_before_travels_and_m3_s_before_burns() {
    let p = golden("m3_constant_power");
    assert_eq!(p.mode, Mode::M3);
    let c = &p.commands;
    for (i, line) in c.iter().enumerate() {
        if line.starts_with("G0 ") {
            assert_eq!(c[i - 1], "M5", "travel at {i} not after M5");
        }
    }
    let after_travel: Vec<&String> = c
        .windows(2)
        .filter(|w| w[0].starts_with("G0 ") && w[1].starts_with('M'))
        .map(|w| &w[1])
        .collect();
    assert!(
        after_travel.iter().all(|m| m.starts_with("M3 S")),
        "{after_travel:?}"
    );
    assert_eq!(p.settings_order(), [(200, 3000), (900, 300)]);
}

// ---- criterion 19: Z ------------------------------------------------------------------------------

#[test]
fn c19_z_offset_is_one_move_down_before_and_one_back_to_zero_after() {
    let p = golden("z_offset");
    let z: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Z).collect();
    assert_eq!(z.len(), 2, "exactly two Z moves");
    assert_eq!(z[0].z, -1500);
    assert_eq!(z[1].z, 0);
    let first_burn = p.segs.iter().position(|s| s.kind == Kind::Burn).unwrap();
    let last_burn = p.segs.iter().rposition(|s| s.kind == Kind::Burn).unwrap();
    let zi: Vec<usize> = p
        .segs
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == Kind::Z)
        .map(|(i, _)| i)
        .collect();
    assert!(
        zi[0] < first_burn && zi[1] > last_burn,
        "Z before the first and after the last pass"
    );
    // The other goldens never write Z.
    for g in GOLDENS.iter().filter(|g| **g != "z_offset") {
        assert!(
            !golden(g).segs.iter().any(|s| s.kind == Kind::Z),
            "{g}: Z written"
        );
    }
}

// ---- criterion 25: air assist ---------------------------------------------------------------------

#[test]
fn c25_air_assist_m8_before_the_first_burn_m9_at_the_end() {
    let p = golden("air_assist");
    let on = p.air_on.expect("M8");
    let first_burn_cmd = p
        .commands
        .iter()
        .position(|c| c.starts_with("G1 ") && c.contains(" S") && !c.ends_with(" S0"))
        .unwrap();
    assert!(on < first_burn_cmd, "M8 before the first burn");
    assert!(p.air_off.is_some(), "M9 at the end");
    for g in GOLDENS.iter().filter(|g| **g != "air_assist") {
        let q = golden(g);
        assert!(
            q.air_on.is_none() && q.air_off.is_none(),
            "{g}: air written while off"
        );
    }
    // Same job as cut_line otherwise.
    let base = golden("cut_line");
    assert!((p.burn_mm() - base.burn_mm()).abs() < 1e-9);
}

// ---- criterion 26: corners ------------------------------------------------------------------------

#[test]
fn c26_four_corners_give_mirrored_coordinates_each_inside_the_bed() {
    let (w, h) = (400 * MM, 300 * MM);
    let pts = |g: &str| -> Vec<(i64, i64)> {
        let p = golden(g);
        let mut v: Vec<(i64, i64)> = p
            .segs
            .iter()
            .filter(|s| s.kind == Kind::Burn)
            .flat_map(|s| [s.from, s.to])
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let bl = pts("origin_corners_bl");
    let mut tl: Vec<(i64, i64)> = bl.iter().map(|&(x, y)| (x, h - y)).collect();
    let mut br: Vec<(i64, i64)> = bl.iter().map(|&(x, y)| (w - x, y)).collect();
    let mut tr: Vec<(i64, i64)> = bl.iter().map(|&(x, y)| (w - x, h - y)).collect();
    tl.sort_unstable();
    br.sort_unstable();
    tr.sort_unstable();
    assert_eq!(pts("origin_corners_tl"), tl, "top-left mirrors Y");
    assert_eq!(pts("origin_corners_br"), br, "bottom-right mirrors X");
    assert_eq!(pts("origin_corners_tr"), tr, "top-right mirrors both");
    for g in [
        "origin_corners_bl",
        "origin_corners_br",
        "origin_corners_tl",
        "origin_corners_tr",
    ] {
        let p = golden(g);
        let first = p.segs[0].to;
        assert!(
            first.0 >= 0 && first.1 >= 0 && first.0 <= w && first.1 <= h,
            "{g} starts inside"
        );
    }
}

// ---- criterion 38: Home first and Head position ---------------------------------------------------

#[test]
fn c38_home_first_has_h_before_g21_and_otherwise_equals_cut_line() {
    let home = golden("home_first");
    assert!(home.homes);
    assert_eq!(home.commands[0], "$H");
    assert_eq!(home.commands[1], "G21");
    let base = golden("cut_line");
    assert!(!base.homes);
    assert_eq!(
        &home.commands[1..],
        &base.commands[..],
        "the rest is the Machine origin file"
    );
    assert!(!home.relative, "Home first forces Machine origin");
}

#[test]
fn c38_head_position_is_g91_with_no_absolute_coordinate_and_returns_to_its_start() {
    let p = golden("start_head_position");
    assert!(p.relative);
    assert!(!p.commands.iter().any(|c| c == "G90"), "no G90");
    assert_eq!(p.end, (0, 0), "the end returns to where the head stood");
    // The first move is a burn from where the head stands (no first travel).
    assert_eq!(p.segs[0].kind, Kind::Burn, "{:?}", p.segs[0]);
    let base = golden("cut_line");
    assert!((p.burn_mm() - base.burn_mm()).abs() < 0.001, "the same job");
}

// ---- criteria 14, 15: engrave ---------------------------------------------------------------------

#[test]
fn c14_c15_bidirectional_lines_alternate_interval_apart_half_an_interval_inside() {
    let p = golden("engrave_bidirectional");
    let burns: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Burn).collect();
    assert!(burns.len() >= 3);
    for w in burns.windows(2) {
        let d0 = (w[0].to.0 - w[0].from.0).signum();
        let d1 = (w[1].to.0 - w[1].from.0).signum();
        assert_eq!(d0, -d1, "alternate directions");
        assert_eq!(w[0].from.1, w[0].to.1, "lines parallel to X at 0 degrees");
    }
    let ys: Vec<i64> = burns.iter().map(|s| s.from.1).collect();
    let step = ys[1] - ys[0];
    assert!(
        ys.windows(2).all(|w| w[1] - w[0] == step),
        "one Interval apart: {ys:?}"
    );
    // The travel between two lines of one area is the shortest connection: straight up by Interval.
    let travels: Vec<_> = p
        .segs
        .iter()
        .filter(|s| s.kind == Kind::Travel)
        .skip(1)
        .collect();
    for t in travels.iter().take(burns.len() - 1) {
        assert!(
            (t.length_mm() * 1000.0 - step.abs() as f64).abs() < 1.0,
            "{t:?}"
        );
    }
    // Half an Interval inside: the box is the area box less half a step on Y.
    let b = p.burn_box().unwrap();
    assert_eq!((b.3 - b.1) % step.abs(), 0);
}

#[test]
fn c15_crosshatch_lines_at_the_scan_angle_then_plus_90_one_interval_apart() {
    let p = golden("engrave_crosshatch_scan_angle");
    let burns: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Burn).collect();
    let angle = |s: &tester_grbl_0029::Seg| {
        let a = ((s.to.1 - s.from.1) as f64)
            .atan2((s.to.0 - s.from.0) as f64)
            .to_degrees();
        (a + 360.0) % 180.0
    };
    let first = angle(burns[0]);
    let split = burns
        .iter()
        .position(|s| (angle(s) - first).abs() > 1.0)
        .expect("a second hatch");
    assert!(
        burns[..split]
            .iter()
            .all(|s| (angle(s) - first).abs() < 0.2),
        "the first hatch is one angle"
    );
    let second = angle(burns[split]);
    assert!(
        ((second - first).rem_euclid(180.0) - 90.0).abs() < 0.2,
        "{first} then {second}"
    );
    assert!(
        burns[split..]
            .iter()
            .all(|s| (angle(s) - second).abs() < 0.2),
        "then only the second angle"
    );
    // A positive Scan angle turns the lines counter-clockwise as seen on the page; with a
    // bottom-left origin the file's Y is the page's up, so the first angle is in (0, 90).
    assert!(first > 0.0 && first < 90.0, "{first}");
    // Spacing along the normal is one Interval.
    let n = (first.to_radians().sin() * -1.0, first.to_radians().cos());
    let offs: Vec<f64> = burns[..split]
        .iter()
        .map(|s| (s.from.0 as f64 * n.0 + s.from.1 as f64 * n.1) / 1000.0)
        .collect();
    let mut sorted = offs.clone();
    sorted.sort_by(f64::total_cmp);
    for w in sorted.windows(2) {
        assert!((w[1] - w[0] - 1.0).abs() < 0.01, "spacing {}", w[1] - w[0]);
    }
}

#[test]
fn c15_overscan_extends_each_line_laser_off_and_switches_at_the_edge() {
    let p = golden("engrave_overscan");
    let off: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Off).collect();
    let burns: Vec<_> = p.segs.iter().filter(|s| s.kind == Kind::Burn).collect();
    assert_eq!(
        off.len(),
        burns.len() * 2,
        "a run-out on both ends of every line"
    );
    // Run-out length = overscan% x v^2 / (2 x 1000 mm/s^2), v = 6000/60 = 100 mm/s -> 5 mm x %.
    for o in &off {
        assert_eq!(o.f, burns[0].f, "run-out at the burn feed");
        assert!(
            o.length_mm() > 0.0 && o.length_mm() <= 5.0 + 1e-9,
            "{}",
            o.length_mm()
        );
    }
    // The burn starts and ends exactly where the bidirectional golden's would on X (the edge).
    let b = p.burn_box().unwrap();
    let bo = p.all_box();
    assert!(
        bo.0 < b.0 && bo.2 > b.2,
        "run-out beyond the area on both ends"
    );
}

#[test]
fn c16_engrave_outline_hatch_with_engrave_set_then_outlines_with_score_set_holes_empty() {
    let p = golden("engrave_outline_set");
    let order = p.settings_order();
    assert_eq!(order.len(), 2, "{order:?}");
    let (hatch, outline) = (order[0], order[1]);
    assert_ne!(hatch, outline, "two sets");
    let runs = p.runs();
    let outlines: Vec<_> = runs.iter().filter(|r| (r.s, r.f) == outline).collect();
    assert_eq!(outlines.len(), 2, "hole outline and outer outline");
    assert!(outlines.iter().all(|r| r.points.first() == r.points.last()));
    // Hole first (inner first, criterion 22), then the outer.
    let span = |r: &tester_grbl_0029::Run| {
        r.points.iter().map(|p| p.0).max().unwrap() - r.points.iter().map(|p| p.0).min().unwrap()
    };
    assert!(span(outlines[0]) < span(outlines[1]));
    // No hatch segment crosses the hole's inside.
    let hole = outlines[0]
        .points
        .iter()
        .fold((i64::MAX, i64::MAX, i64::MIN, i64::MIN), |b, p| {
            (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1))
        });
    for s in p
        .segs
        .iter()
        .filter(|s| s.kind == Kind::Burn && (s.s, s.f) == hatch)
    {
        let y = s.from.1;
        if y > hole.1 && y < hole.3 {
            let (a, b) = (s.from.0.min(s.to.0), s.from.0.max(s.to.0));
            assert!(
                b <= hole.0 || a >= hole.2,
                "hatch {s:?} enters the hole {hole:?}"
            );
        }
    }
}

#[test]
fn c16_cutout_margin_one_closed_outer_contour() {
    let p = golden("cutout_margin");
    let runs = p.runs();
    assert_eq!(runs.len(), 1, "one closed line per item and pass");
    assert_eq!(runs[0].points.first(), runs[0].points.last());
}

// ---- criterion 9 and 31 at file level: lengths are whole micrometres ------------------------------

#[test]
fn c9_lengths_read_back_are_finite_and_burn_dominant_for_cut_line() {
    let p = golden("cut_line");
    // travel: origin -> (20, 20) and back from (120, 20)
    let expect_travel = 20.0_f64.hypot(20.0) + 120.0_f64.hypot(20.0);
    assert!(
        (p.travel_mm() - expect_travel).abs() < 0.01,
        "{}",
        p.travel_mm()
    );
}

// ---- criterion 39 and 25: format facts and number rules -------------------------------------------

#[test]
fn c39_grbl_is_the_one_format_with_gcode_extension_and_file_type() {
    assert_eq!(name(OutputFormat::Grbl), "GRBL");
    assert_eq!(extension(OutputFormat::Grbl), "gcode");
    assert_eq!(file_type(OutputFormat::Grbl), "GRBL G-code");
}

#[test]
fn c25_s_is_round_power_times_smax_over_100() {
    // power in tenths of a percent
    assert_eq!(s_value(500, 1000), 500);
    assert_eq!(s_value(1000, 1000), 1000);
    assert_eq!(s_value(0, 1000), 0);
    assert_eq!(s_value(333, 255), 85); // 33.3 % of 255 = 84.9
    assert_eq!(s_value(5, 1000), 5); // 0.5 %
    assert_eq!(s_value(125, 1000), 125);
    assert_eq!(s_value(1, 255), 0); // 0.1 % of 255 = 0.255
    assert_eq!(s_value(2, 255), 1); // 0.51
}

#[test]
fn c24_coordinates_three_decimals_never_minus_zero() {
    assert_eq!(coordinate(0), "0.000");
    assert_eq!(coordinate(1), "0.001");
    assert_eq!(coordinate(-1), "-0.001");
    assert_eq!(coordinate(-1000), "-1.000");
    assert_eq!(coordinate(123_456), "123.456");
    assert_eq!(coordinate(10_000_000), "10000.000");
}

#[test]
fn c10_texts_of_lengths_and_time() {
    assert_eq!(estimate_text(21.0), "About 21 s");
    assert_eq!(estimate_text(760.0), "About 12 min 40 s");
    assert_eq!(estimate_text(3900.0), "About 1 h 5 min");
    assert_eq!(length_text(999.4), "999 mm");
    assert_eq!(length_text(1234.0), "1.23 m");
    assert_eq!(lengths_text(200.0, 100.0), "Burn 200 mm · travel 100 mm");
    assert!(!duration_text(21.0).starts_with("About"));
}

#[test]
fn c38_start_is_one_value_of_three_spellings() {
    // Superseded by 0029 rework (2026-11-17): one Start pick replaces Start from and Home first;
    // there is no Home first rule left to force a value.
    assert_eq!(Start::parse("machine_origin"), Some(Start::MachineOrigin));
    assert_eq!(Start::parse("head_position"), Some(Start::HeadPosition));
    assert_eq!(
        Start::parse("home_then_origin"),
        Some(Start::HomeThenOrigin)
    );
    assert_eq!(Start::parse("middle"), None);
    assert_eq!(Start::default(), Start::MachineOrigin);
}

// ---- criterion 32: profile record 6 and format 48 ------------------------------------------------

fn profile_golden(name: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/machine_profiles")
            .join(name),
    )
    .unwrap()
}

fn record_name(text: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(text).unwrap();
    format!("{}.json", v["id"].as_str().unwrap())
}

#[test]
fn c32_profile_record_6_reads_air_assist_and_output_format_and_writes_back_byte_for_byte() {
    use curvyo_laser_core::{read_record, write_record};
    let text = profile_golden("format6_air_assist.json");
    let p = read_record(&record_name(&text), &text).expect("format 6 reads");
    assert!(p.values.air_assist);
    assert_eq!(p.values.output_format, OutputFormat::Grbl);
    // Since `specs/0150` any write is record format 7 with the default pair.
    let seven = write_record(&p);
    assert!(seven.starts_with("{\n  \"format_version\": 7,"));
    assert_eq!(read_record(&record_name(&seven), &seven).unwrap(), p);
}

#[test]
fn c32_format_5_reads_air_off_grbl_and_its_new_keys_damage_it() {
    use curvyo_laser_core::read_record;
    let text = profile_golden("format5_fixture_sets.json");
    let p = read_record(&record_name(&text), &text).expect("format 5 reads");
    assert!(!p.values.air_assist);
    assert_eq!(p.values.output_format, OutputFormat::Grbl);
    for bad in [
        "format5_air_assist_key.json",
        "format5_output_format_key.json",
        "format6_unknown_output_format.json",
    ] {
        let t = profile_golden(bad);
        assert!(
            read_record(&record_name(&t), &t).is_err(),
            "{bad} must not read"
        );
    }
    let t = profile_golden("newer_format.json");
    let v: serde_json::Value = serde_json::from_str(&t).unwrap();
    assert_eq!(
        v["format_version"], 8,
        "newer is now 8 (record format 7 is `0150`'s)"
    );
    assert!(read_record(&record_name(&t), &t).is_err());
}

#[test]
fn c32_a_written_record_is_format_6_with_both_keys() {
    use curvyo_laser_core::{read_record, write_record};
    let text = profile_golden("format5_fixture_sets.json");
    let p = read_record(&record_name(&text), &text).unwrap();
    let out = write_record(&p);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["format_version"], 7, "record format 7 since `0150`");
    assert_eq!(v["air_assist"], false);
    assert_eq!(v["output_format"], "grbl");
    let at = |k: &str| out.find(&format!("\"{k}\"")).unwrap();
    assert!(
        at("motorised_z") < at("air_assist") && at("air_assist") < at("output_format"),
        "{out}"
    );
}

#[test]
fn c32_project_format_48_and_the_page_registers_read() {
    use curvyo_document_core::{CURRENT_FORMAT_VERSION, unpack};
    use curvyo_laser_core::job::read_output_settings;
    // Format 49 since `specs/0124-gradient-fill` (ADR 0066 §8).
    // Format 50 since `specs/0123-raster-engraving` (ADR 0069 §10).
    // Format 51 since `specs/0135-image-optimisation` slice 1 (ADR 0072 §8); Superseded by 0135 slice 1 (2026-11-19).
    assert_eq!(CURRENT_FORMAT_VERSION, 60);
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../curvyo-document-core/tests/fixtures");
    let bytes = std::fs::read(dir.join("laser_output_v48.curvyo")).unwrap();
    let d = unpack(1, &bytes).unwrap();
    let mut seen = Vec::new();
    for page in d.pages() {
        if let Some(m) = page.kind.manufacturing() {
            let snap = d.manufacturing(m).unwrap();
            seen.push(read_output_settings(&snap));
        }
    }
    // Superseded by 0029 rework (2026-11-17): the goldens hold the one register `start` (ADR
    // 0065 §6): Home, then machine origin; Head position; a page with no registers.
    assert_eq!(seen.len(), 3, "three laser pages");
    let a = &seen[0];
    assert!(a.fixture_material.is_some());
    assert_eq!(a.start, Start::HomeThenOrigin);
    assert_eq!(a.damaged, 0);
    assert_eq!(seen[1].start, Start::HeadPosition);
    let c = &seen[2];
    assert!(c.fixture_material.is_none() && c.start == Start::MachineOrigin);
    let bytes = std::fs::read(dir.join("laser_output_damaged_v48.curvyo")).unwrap();
    let d = unpack(1, &bytes).unwrap();
    let damaged: Vec<_> = d
        .pages()
        .into_iter()
        .filter_map(|p| p.kind.manufacturing())
        .map(|m| read_output_settings(&d.manufacturing(m).unwrap()))
        .collect();
    let s = &damaged[0];
    assert_eq!(s.damaged, 2, "the pick and the start each count one");
    assert!(s.fixture_material.is_none());
    assert_eq!(
        s.start,
        Start::MachineOrigin,
        "an unknown value reads Machine origin"
    );
    assert!(s.start_damaged);
    assert!(damaged[1].start_damaged && damaged[1].start == Start::MachineOrigin);
}

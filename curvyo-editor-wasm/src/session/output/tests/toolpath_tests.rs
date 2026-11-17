//! The job cache and the Toolpath view: one build per change, the drawn moves, the clone and pin cases and the page label toggle (criteria 8, 9, 11, 12, 17, 36).

#![allow(
    clippy::assert_is_empty,
    clippy::case_sensitive_file_extension_comparisons
)]

use super::{cut, laser_with_square, rows, view};
use crate::session::Session;
use crate::session::align_test_support::draw_rect;

#[test]
fn criterion_36_the_job_is_built_once_per_change_and_reused_between_frames() {
    let (mut s, square) = laser_with_square();
    let id = cut(&mut s, &[square]);
    let builds = |s: &Session| s.output.job.builds.get();
    let _ = view(&s);
    let before = builds(&s);
    let _ = view(&s);
    let _ = s.play_view();
    assert_eq!(builds(&s), before, "an unchanged page builds no job");
    s.edit_operation_setting(id, "main", "power", 30.0).unwrap();
    let _ = view(&s);
    assert_eq!(builds(&s), before + 1);
}

#[test]
fn criterion_36_a_library_change_rebuilds_the_job_by_its_generation() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let builds = |s: &Session| s.output.job.builds.get();
    let _ = view(&s);
    let before = builds(&s);
    let generation = s.materials.library_generation();
    s.load_materials(&[], None);
    assert_ne!(s.materials.library_generation(), generation);
    let _ = view(&s);
    assert_eq!(builds(&s), before + 1, "a new library generation rebuilds");
    let _ = view(&s);
    assert_eq!(
        builds(&s),
        before + 1,
        "an unchanged generation reuses the job"
    );
}

#[test]
fn criterion_8_the_toolpath_view_is_per_page_and_exclusive_with_the_operations_view() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let page = s.active_page().id;
    assert!(s.operations_view_on());
    let steps = s.undo_len();
    assert!(s.toggle_toolpath_view(&page));
    assert_eq!(s.undo_len(), steps, "no step");
    assert!(s.toolpath_view_on());
    assert!(!s.operations_view_on(), "Route releases ListTree");
    assert_eq!(view(&s)["toolpath_view"], true);
    // ListTree while Route is pressed releases Route and is pressed.
    assert!(s.toggle_operations_view(&page));
    assert!(!s.toolpath_view_on());
    assert!(s.operations_view_on());
    // Releasing Route gives ListTree back the state it had.
    assert!(s.toggle_operations_view(&page));
    assert!(s.toggle_toolpath_view(&page));
    assert!(s.toggle_toolpath_view(&page));
    assert!(!s.operations_view_on());
}

#[test]
fn criteria_8_and_12_the_toolpath_draws_and_a_frame_with_no_change_rebuilds_nothing() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let page = s.active_page().id;
    assert!(s.toolpath_overlay().is_none(), "not pressed: nothing");
    s.toggle_toolpath_view(&page);
    let first = s.toolpath_overlay().unwrap();
    assert!(!first.triangles.is_empty());
    let (looks, lists) = (s.output.draw.looks.get(), s.output.draw.lists.get());
    assert_eq!(s.toolpath_overlay().unwrap(), first);
    assert_eq!(s.output.draw.lists.get(), lists, "the same frame is reused");
    s.set_play_position(0.5);
    let _ = s.toolpath_overlay();
    assert_eq!(
        s.output.draw.lists.get(),
        lists + 1,
        "a slider move redraws"
    );
    assert_eq!(s.output.draw.looks.get(), looks, "but keeps the look");
    let labels = s.toolpath_ring_labels();
    assert_eq!(labels.len(), 1);
    // Superseded by 0029 rework (2026-11-17): the ring shows the row's digit (P26).
    assert_eq!(labels[0].digit, Some(1));
}

#[test]
fn criterion_9_the_drawn_burn_length_is_the_jobs() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let job = s.laser_job().unwrap();
    let (look, _) = s.toolpath_look(&job);
    let burn: f64 = look
        .segments
        .iter()
        .filter(|seg| seg.stroke == curvyo_render_core::ToolpathStroke::Cut)
        .map(|seg| seg.from.vector_to(seg.to).length())
        .sum();
    assert!(
        (burn - job.timeline.burn_mm()).abs() < 0.01,
        "{burn} vs {}",
        job.timeline.burn_mm()
    );
    assert!((burn - 80.0).abs() < 0.01);
}

#[test]
fn criterion_11_a_hidden_item_that_runs_is_drawn_and_warned() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    s.recording_unguarded(|this| this.document.set_node_visible(&[square], false).unwrap());
    let v = view(&s);
    let texts: Vec<&str> = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap())
        .collect();
    assert!(texts.contains(&"1 item runs while hidden."), "{texts:?}");
    // Superseded by 0029 rework (2026-11-17): the run is the row's line 2.
    assert_eq!(rows(&s)["rows"][0]["run"]["digit"], 1);
}

#[test]
fn criterion_17_a_pin_circle_runs_within_0_01_mm_of_the_true_circle_in_the_file_and_the_view() {
    use curvyo_document_core::{FixtureJobSwitch, Point};
    use curvyo_laser_core::job::{MoveKind, coordinate};
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let o = s.active_page().origin;
    assert!(s.arm_job_fixture("pin"));
    s.pointer_down(Point::new(o.x + 100.0, o.y + 100.0), false);
    s.pointer_up(Point::new(o.x + 100.0, o.y + 100.0), false, false);
    s.end_fixture_placement();
    let pin = s.selected_fixture().unwrap();
    // 40 mm: the fixture look's 48 chords would lie 0.043 mm inside the circle.
    assert!(s.set_fixture_grid_value(pin, "diameter_mm", 40.0));
    let centre = s.fixture_centres_for_tests(pin)[0];
    let centre = Point::new(centre.x - o.x, centre.y - o.y);
    let jobs = s.fixture_jobs_view().unwrap();
    let diameters = jobs.groups[0]
        .jobs
        .iter()
        .find(|j| j.role == Some("diameters"))
        .unwrap()
        .id
        .clone();
    let job = s.fixture_job_named(&diameters).unwrap();
    for other in jobs.groups[0].jobs.iter().filter(|j| j.output) {
        let id = s.fixture_job_named(&other.id).unwrap();
        s.set_fixture_job_switch(id, FixtureJobSwitch::Output, false);
    }
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    let birch = s.add_material("Birch 3 mm", 3.0, &[1; 16]).unwrap();
    let laser = s.active_profile_id().unwrap();
    for (n, kind) in ["cut", "score", "engrave"].into_iter().enumerate() {
        let seed = [u8::try_from(n + 2).unwrap(); 16];
        s.add_material_set(&birch, &laser, kind, &seed).unwrap();
    }
    assert!(s.set_fixture_material(&birch));
    let built = s.laser_job().unwrap();
    let file = String::from_utf8(s.export_job("").unwrap().bytes).unwrap();
    let job = &built.built.job;
    let fixture_block = job
        .blocks
        .iter()
        .position(|b| b.ring == curvyo_laser_core::job::Ring::Fixture)
        .unwrap();
    let mut points = Vec::new();
    for m in job.moves.iter().filter(|m| m.block == fixture_block) {
        if matches!(m.kind, MoveKind::Burn { .. }) {
            let machine = job.frame.machine_point(m.to);
            let words = format!("X{} Y{}", coordinate(machine.x), coordinate(machine.y));
            assert!(file.contains(&words), "the file runs {words}");
            points.push(m.to.to_mm());
        }
    }
    assert!(points.len() > 32, "flattened finer than the fixture look");
    let radius = 20.0;
    for pair in points.windows(2) {
        let d = |p: Point| (p.x - centre.x).hypot(p.y - centre.y);
        assert!((d(pair[0]) - radius).abs() <= 0.01, "{}", d(pair[0]));
        let mid = Point::new(
            f64::midpoint(pair[0].x, pair[1].x),
            f64::midpoint(pair[0].y, pair[1].y),
        );
        assert!(radius - d(mid) <= 0.01, "chord sagitta {}", radius - d(mid));
    }
    // The Toolpath view draws the same moves.
    let (look, _) = s.toolpath_look(&built);
    for segment in look.segments.iter().filter(|g| {
        job.moves[g.index].block == fixture_block
            && matches!(job.moves[g.index].kind, MoveKind::Burn { .. })
    }) {
        let at = job.moves[segment.index].to.to_mm();
        assert!(
            (segment.to.x - o.x - at.x).abs() < 1e-9 && (segment.to.y - o.y - at.y).abs() < 1e-9
        );
    }
}

#[test]
fn criterion_8_a_clone_of_a_group_does_not_draw_its_own_look_in_the_toolpath_view() {
    let (mut s, a) = laser_with_square();
    let o = s.active_page().origin;
    let b = draw_rect(&mut s, (o.x + 40.0, o.y + 10.0), (o.x + 60.0, o.y + 30.0));
    s.selection.set(&[a, b]);
    s.group_selection();
    let _ = s.linked_clone();
    let clone = s.selection.ids()[0];
    cut(&mut s, &[clone]);
    let objects = s.objects();
    assert!(
        objects.iter().any(|object| object.id() == clone),
        "the clone is drawn"
    );
    let page = s.active_page().id;
    s.toggle_toolpath_view(&page);
    let (out, _) = s.operation_looked_artwork(&objects, s.view()).unwrap();
    assert!(
        out.iter()
            .all(|object| ![clone, a, b].contains(&object.id())),
        "the Toolpath view draws the job instead"
    );
}

#[test]
fn part_d_the_page_label_carries_the_route_toggle_and_ops_view_reads_unpressed_while_it_is() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let page = s.active_page().id;
    let row = |s: &Session| -> serde_json::Value {
        let rows: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
        rows.into_iter()
            .find(|row| row["id"] == page.as_str())
            .unwrap()
    };
    assert_eq!(row(&s)["toolpathView"], false);
    assert_eq!(row(&s)["operationsView"], true);
    s.toggle_toolpath_view(&page);
    assert_eq!(row(&s)["toolpathView"], true);
    assert_eq!(row(&s)["operationsView"], false);
}

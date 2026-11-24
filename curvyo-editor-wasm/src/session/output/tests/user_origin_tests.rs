//! The user origin of `specs/0118-machine-control` slice 2c in the session (criteria 29 to 32;
//! adrs O6, O7; UX notes W41 to W48): Set origin here, the pick and its row, the Toolpath view at
//! the shift, the canvas marks and the bed check's distance.

use super::{cut, laser_with_square, undo_label, view};

fn labels(v: &serde_json::Value) -> Vec<String> {
    v["start"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["label"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn c29_set_origin_here_stores_the_point_and_picks_user_origin_in_one_step() {
    let (mut s, _) = laser_with_square();
    assert_eq!(labels(&view(&s)).len(), 3, "absent until a point is stored");
    assert!(view(&s)["start"].get("user_origin").is_none());
    assert!(s.set_user_origin(120.0, 85.5));
    assert_eq!(undo_label(&mut s), "Set origin here");
    let v = view(&s);
    assert_eq!(v["start"]["value"], "user_origin");
    assert_eq!(labels(&v).last().unwrap(), "User origin");
    assert_eq!(v["start"]["user_origin"]["x"], 120.0);
    assert_eq!(v["start"]["user_origin"]["y"], 85.5);
    assert!(
        !s.set_user_origin(120.004, 85.5),
        "the same point writes no step"
    );
    assert!(!s.set_user_origin(10_000.1, 0.0), "beyond 10 m");
    assert!(!s.set_user_origin(f64::NAN, 0.0));
}

#[test]
fn w43_the_typed_pair_is_its_own_step_user_origin() {
    let (mut s, _) = laser_with_square();
    assert!(s.type_user_origin(120.0, 85.5));
    assert_eq!(undo_label(&mut s), "User origin");
    assert!(s.set_user_origin(10.0, 20.0));
    assert_eq!(undo_label(&mut s), "Set origin here");
    assert!(s.set_user_origin(30.0, 40.0));
    assert!(s.type_user_origin(31.0, 40.0));
    assert_eq!(undo_label(&mut s), "User origin");
    assert!(
        !s.type_user_origin(31.004, 40.0),
        "the same point writes no step"
    );
}

#[test]
fn c30_another_pick_keeps_the_point_and_hides_the_row() {
    let (mut s, _) = laser_with_square();
    assert!(!s.set_start("user_origin"), "no point yet");
    assert!(s.set_user_origin(100.0, 50.0));
    assert!(s.set_start("machine_origin"));
    let v = view(&s);
    assert_eq!(labels(&v).len(), 4, "User origin stays in the list");
    assert!(
        v["start"].get("user_origin").is_none(),
        "no row with another pick"
    );
    assert!(s.set_start("user_origin"));
    assert_eq!(undo_label(&mut s), "Start: User origin");
}

#[test]
fn c31_the_toolpath_is_drawn_where_the_job_burns_and_the_items_stay() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let page = s.active_manufacturing().unwrap().page.unwrap();
    let min_x = |s: &crate::session::Session| {
        let job = s.laser_job().unwrap();
        let (look, _) = s.toolpath_look(&job);
        look.segments
            .iter()
            .filter(|seg| seg.stroke == curvyo_render_core::ToolpathStroke::Cut)
            .flat_map(|seg| [seg.from.x, seg.to.x])
            .fold(f64::INFINITY, f64::min)
    };
    assert!((min_x(&s) - (page.origin.x + 10.0)).abs() < 1e-6);
    assert!(s.set_user_origin(100.0, 50.0));
    // Bottom-left machine: the box's left edge sits on X 100 of the bed.
    assert!((min_x(&s) - (page.origin.x + 100.0)).abs() < 1e-6);
    assert!(crate::session::output::shifted(
        &s.laser_job().unwrap().built.job
    ));
}

#[test]
fn c32_the_cross_and_the_dashed_box_show_only_with_user_origin() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    assert_eq!(s.user_origin_marks_json(), r#"{"cross":null,"box":null}"#);
    assert!(s.set_user_origin(100.0, 50.0));
    let _ = view(&s);
    let marks = s.user_origin_marks();
    let (cx, cy) = marks.cross.expect("the cross");
    let [bx, by, w, h] = marks.burn_box.expect("the dashed box");
    let scale = s.view().scale();
    assert!((w - 20.0 * scale).abs() < 1e-6 && (h - 20.0 * scale).abs() < 1e-6);
    assert!((bx - cx).abs() < 1e-6, "its anchor corner is on the cross");
    assert!((by + h - cy).abs() < 1e-6);
    let page = s.active_page().id;
    s.toggle_toolpath_view(&page);
    assert!(
        s.user_origin_marks().burn_box.is_none(),
        "the moves show the place"
    );
    assert!(s.set_start("machine_origin"));
    assert_eq!(s.user_origin_marks().cross, None);
}

#[test]
fn c31_the_bed_check_names_the_distance() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let width = s
        .active_manufacturing()
        .unwrap()
        .page
        .unwrap()
        .size
        .width
        .as_mm();
    assert!(s.set_user_origin(width - 8.0, 50.0));
    let v = view(&s);
    let texts: Vec<&str> = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap())
        .collect();
    assert!(
        texts.iter().any(|t| t.ends_with("mm bed by 12 mm.")),
        "{texts:?}"
    );
    assert_eq!(v["export"]["enabled"], false);
}

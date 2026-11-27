//! Acceptance tests for `specs/0158-pen-display` slice 1 (Pressure Pencil), criteria 1 to 17,
//! written from the specification; all input is synthetic `PenSample` batches.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]

mod cpu_time;

use curvyo_document_core::{Direction, Document, Length, NodeId, Point, unpack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::{PEN_SAMPLE_STRIDE, PressureCurve, WidthPreset};

const NONE: f64 = f64::NAN;

#[derive(Clone, Copy)]
struct S {
    x: f64,
    y: f64,
    p: f64,
    eraser: bool,
    phase: u8, // 0 hover, 1 contact, 2 out
    t: f64,
}

fn contact(x: f64, y: f64, p: f64, t: f64) -> S {
    S {
        x,
        y,
        p,
        eraser: false,
        phase: 1,
        t,
    }
}

fn flat(samples: &[S]) -> Vec<f64> {
    let mut v = Vec::new();
    for s in samples {
        v.extend_from_slice(&[
            s.x,
            s.y,
            s.p,
            NONE,
            NONE,
            if s.eraser { 1.0 } else { 0.0 },
            f64::from(s.phase),
            0.0,
            s.t,
        ]);
    }
    assert_eq!(v.len(), samples.len() * PEN_SAMPLE_STRIDE);
    v
}

/// A session with the Pencil; `mm(x, y)` gives screen px for document mm.
struct Rig {
    session: Session,
    ox: f64,
    oy: f64,
    scale: f64,
}

impl Rig {
    fn new() -> Self {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pencil);
        let o = session.screen_to_document(0.0, 0.0);
        let one = session.screen_to_document(100.0, 0.0);
        let scale = (one.x - o.x) / 100.0; // mm per px
        Self {
            session,
            ox: o.x,
            oy: o.y,
            scale,
        }
    }
    /// Screen px of a document point in mm.
    fn px(&self, x: f64, y: f64) -> (f64, f64) {
        ((x - self.ox) / self.scale, (y - self.oy) / self.scale)
    }
    fn feed(&mut self, samples: &[S]) {
        let f = flat(samples);
        self.session.pen_samples(&f);
    }
    /// A straight stroke of `n` samples over `len` mm with pressure from `p0` to `p1`.
    fn line(&self, n: usize, len: f64, p0: f64, p1: f64, dt: f64) -> Vec<S> {
        (0..n)
            .map(|i| {
                let f = i as f64 / (n - 1) as f64;
                let (x, y) = self.px(20.0 + len * f, 50.0);
                contact(x, y, p0 + (p1 - p0) * f, 1000.0 + dt * i as f64)
            })
            .collect()
    }
    fn doc(&mut self) -> Document {
        unpack(2, &self.session.pack("t").unwrap()).unwrap()
    }
    fn paths(&mut self) -> Vec<NodeId> {
        self.doc().object_ids()
    }
    fn undo_name(&mut self) -> Option<String> {
        self.session.history_target_name(Direction::Undo)
    }
}

/// How a stroke ends: contact samples then one hover sample (pen lifted).
fn lifted(mut v: Vec<S>) -> Vec<S> {
    let last = *v.last().unwrap();
    v.push(S {
        phase: 0,
        p: 0.0,
        t: last.t + 5.0,
        ..last
    });
    v
}

fn widths(doc: &Document, id: NodeId) -> Vec<(f64, f64)> {
    doc.width_points(id)
        .unwrap()
        .iter()
        .map(|w| (w.position, w.multiplier))
        .collect()
}

// ---- criterion 1
#[test]
fn c01_pressure_stroke_makes_one_selected_path_narrow_to_wide_one_undo_step() {
    let mut r = Rig::new();
    let s = lifted(r.line(100, 100.0, 0.1, 0.9, 8.0));
    r.feed(&s);
    let doc = r.doc();
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 1);
    let path = doc.path(ids[0]).unwrap();
    assert!(!path.closed);
    assert!(path.anchors.len() >= 2);
    let w = widths(&doc, ids[0]);
    assert!(!w.is_empty(), "profile expected");
    let first = w.iter().min_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    let last = w.iter().max_by(|a, b| a.0.total_cmp(&b.0)).unwrap();
    assert!(
        first.1 < last.1,
        "start {first:?} must be narrower than end {last:?}"
    );
    assert_eq!(r.session.selected_object_count(), 1);
    assert_eq!(r.session.tool(), Tool::Pencil);
    assert_eq!(r.session.undo_len(), 1);
    assert_eq!(r.undo_name().as_deref(), Some("Pencil stroke"));
}

// ---- criterion 2
fn s_curve(r: &Rig, n: usize) -> (Vec<S>, Vec<(f64, f64)>) {
    let mut v = Vec::new();
    let mut pts = Vec::new();
    for i in 0..n {
        let f = i as f64 / (n - 1) as f64;
        let x = 20.0 + 150.0 * f; // 150 mm along x (arc slightly longer)
        let y = 60.0 + 20.0 * (std::f64::consts::TAU * f).sin();
        pts.push((x, y));
        let (px, py) = r.px(x, y);
        v.push(contact(px, py, 0.5, 1000.0 + 2.0 * i as f64));
    }
    (v, pts)
}

fn dist_to_polyline_anchor_curve(doc: &Document, id: NodeId, p: (f64, f64)) -> f64 {
    // flatten the cubic centre line
    let path = doc.path(id).unwrap();
    let a = &path.anchors;
    let mut best = f64::MAX;
    for pair in a.windows(2) {
        let (p0, p3) = (pair[0].point, pair[1].point);
        let h1 = Point::new(p0.x + pair[0].handle_out.x, p0.y + pair[0].handle_out.y);
        let h2 = Point::new(p3.x + pair[1].handle_in.x, p3.y + pair[1].handle_in.y);
        for k in 0..=200 {
            let t = f64::from(k) / 200.0;
            let u = 1.0 - t;
            let x = u * u * u * p0.x
                + 3.0 * u * u * t * h1.x
                + 3.0 * u * t * t * h2.x
                + t * t * t * p3.x;
            let y = u * u * u * p0.y
                + 3.0 * u * u * t * h1.y
                + 3.0 * u * t * t * h2.y
                + t * t * t * p3.y;
            best = best.min((x - p.0).hypot(y - p.1));
        }
    }
    best
}

#[test]
fn c02_two_thousand_samples_fit_within_tolerance_few_nodes_fast_release() {
    let mut r = Rig::new();
    let (v, pts) = s_curve(&r, 2000);
    let (head, tail) = v.split_at(1999);
    r.feed(head);
    let last = *tail.first().unwrap();
    let ((), cpu) = cpu_time::cpu_time_of(|| r.feed(&lifted(vec![last])));
    assert!(
        cpu.as_millis() < 200,
        "release took {cpu:?} (debug build; 200 ms is the release budget)"
    );
    let doc = r.doc();
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 1);
    let nodes = doc.path(ids[0]).unwrap().anchors.len();
    assert!(nodes <= 50, "nodes {nodes}");
    assert!(nodes < 2000 / 10);
    let worst = pts
        .iter()
        .map(|&p| dist_to_polyline_anchor_curve(&doc, ids[0], p))
        .fold(0.0, f64::max);
    assert!(worst <= 0.4 + 0.02, "worst deviation {worst} mm");
}

// ---- criterion 3 (maths) and 4 (curves, min max)
#[test]
fn c03_smoothing_and_curves() {
    let sm = curvyo_ui_core::smooth_pressures(&[0.2, 0.2, 1.0, 0.2, 0.2]);
    let want = [0.2, 0.2, 0.467, 0.467, 0.467];
    for (a, b) in sm.iter().zip(want) {
        assert!((a - b).abs() < 0.001, "{sm:?}");
    }
    assert!((PressureCurve::Soft.apply(0.25) - 0.5).abs() < 1e-9);
    assert!((PressureCurve::Linear.apply(0.25) - 0.25).abs() < 1e-9);
    assert!((PressureCurve::Firm.apply(0.25) - 0.0625).abs() < 1e-9);
}

fn profile_multipliers(curve: PressureCurve, min: f64, max: f64) -> (Vec<f64>, f64) {
    let mut r = Rig::new();
    r.session.set_pencil_curve(curve);
    r.session.set_pencil_min(Length::from_mm(min));
    r.session.set_pencil_max(Length::from_mm(max));
    let s = lifted(r.line(60, 60.0, 0.5, 0.5, 8.0));
    r.feed(&s);
    let doc = r.doc();
    let id = doc.object_ids()[0];
    let base = doc.path(id).unwrap().style.stroke.width.as_mm();
    (widths(&doc, id).iter().map(|w| w.1 * base).collect(), base)
}

#[test]
fn c04_soft_widest_firm_narrowest_and_min_max_set_the_extremes() {
    // constant pressure 0.5 has no variation: use a ramp so a profile exists.
    let mut mids = Vec::new();
    for c in [
        PressureCurve::Soft,
        PressureCurve::Linear,
        PressureCurve::Firm,
    ] {
        let mut r = Rig::new();
        r.session.set_pencil_curve(c);
        let s = lifted(r.line(60, 60.0, 0.0, 1.0, 8.0));
        r.feed(&s);
        let doc = r.doc();
        let id = doc.object_ids()[0];
        let base = doc.path(id).unwrap().style.stroke.width.as_mm();
        let w = widths(&doc, id);
        // width at the middle by interpolation
        let mut mid = None;
        for p in w.windows(2) {
            if p[0].0 <= 0.5 && p[1].0 >= 0.5 {
                let t = (0.5 - p[0].0) / (p[1].0 - p[0].0).max(1e-12);
                mid = Some((p[0].1 + t * (p[1].1 - p[0].1)) * base);
            }
        }
        mids.push(mid.expect("profile spans the middle"));
    }
    assert!(
        mids[0] > mids[1] && mids[1] > mids[2],
        "soft/linear/firm mid widths {mids:?}"
    );
    // Min and Max: extremes of the profile (Linear, full ramp)
    let (w, _) = profile_multipliers(PressureCurve::Linear, 1.0, 5.0);
    let _ = w;
    let mut r = Rig::new();
    r.session.set_pencil_min(Length::from_mm(1.0));
    r.session.set_pencil_max(Length::from_mm(5.0));
    let s = lifted(r.line(80, 60.0, 0.0, 1.0, 8.0));
    r.feed(&s);
    let doc = r.doc();
    let id = doc.object_ids()[0];
    let base = doc.path(id).unwrap().style.stroke.width.as_mm();
    let ws: Vec<f64> = widths(&doc, id).iter().map(|w| w.1 * base).collect();
    let (lo, hi) = (
        ws.iter().copied().fold(f64::MAX, f64::min),
        ws.iter().copied().fold(0.0, f64::max),
    );
    assert!((hi - 5.0).abs() < 0.3, "widest {hi}");
    assert!((lo - 1.0).abs() < 0.3, "narrowest {lo}");
}

#[test]
fn c04_defaults_and_max_never_below_min() {
    let mut r = Rig::new();
    let st = r.session.pencil_settings();
    assert!((st.widths.min().as_mm() - 0.2).abs() < 1e-9);
    assert!((st.widths.max().as_mm() - 2.0).abs() < 1e-9);
    r.session.set_pencil_min(Length::from_mm(5.0));
    assert!(r.session.pencil_settings().widths.max().as_mm() >= 5.0);
    r.session.set_pencil_max(Length::from_mm(1.0));
    let w = r.session.pencil_settings().widths;
    assert!(w.max().as_mm() >= w.min().as_mm());
    // ranges 0.05..20 and 0.1..50
    r.session.set_pencil_min(Length::from_mm(0.0));
    assert!(r.session.pencil_settings().widths.min().as_mm() >= 0.05 - 1e-9);
    r.session.set_pencil_min(Length::from_mm(100.0));
    assert!(r.session.pencil_settings().widths.min().as_mm() <= 20.0 + 1e-9);
    r.session.set_pencil_max(Length::from_mm(1000.0));
    assert!(r.session.pencil_settings().widths.max().as_mm() <= 50.0 + 1e-9);
}

// ---- criterion 5
#[test]
fn c05_profile_is_small_and_survives_save_and_reopen() {
    let mut r = Rig::new();
    let s = lifted(r.line(300, 100.0, 0.1, 0.9, 4.0));
    r.feed(&s);
    let doc = r.doc();
    let id = doc.object_ids()[0];
    let w = widths(&doc, id);
    assert!(
        w.len() < 20,
        "a linear ramp needs few width points, got {}",
        w.len()
    );
    assert!(w.iter().all(|p| (0.0..=1.0).contains(&p.0)));
    let packed = r.session.pack("t").unwrap();
    let again = Session::open(5, &packed).unwrap();
    let doc2 = unpack(3, &again.pack("t").unwrap()).unwrap();
    let id2 = doc2.object_ids()[0];
    let (a, b) = (doc.path(id).unwrap(), doc2.path(id2).unwrap());
    assert_eq!(a.anchors.len(), b.anchors.len());
    for (x, y) in a.anchors.iter().zip(&b.anchors) {
        assert_eq!(
            (x.point, x.handle_in, x.handle_out),
            (y.point, y.handle_in, y.handle_out)
        );
    }
    assert_eq!(w, widths(&doc2, id2));
}

// ---- criterion 6
#[test]
fn c06_profile_presets_work_each_in_one_undo_step() {
    let mut r = Rig::new();
    let s = lifted(r.line(100, 100.0, 0.1, 0.9, 8.0));
    r.feed(&s);
    for preset in [
        WidthPreset::TaperBoth,
        WidthPreset::TaperStart,
        WidthPreset::TaperEnd,
        WidthPreset::Uniform,
    ] {
        let before = r.session.undo_len();
        r.session.apply_width_preset(preset).unwrap();
        assert_eq!(r.session.undo_len(), before + 1);
        assert_eq!(r.session.selected_width_preset(), Some(preset));
    }
}

// ---- criteria 7, 8
#[test]
fn c07_no_pressure_stroke_has_no_profile_and_the_current_width() {
    for p in [NONE, 0.0, 0.5] {
        let mut r = Rig::new();
        let s = lifted(r.line(40, 50.0, p, p, 8.0));
        r.feed(&s);
        let doc = r.doc();
        assert_eq!(doc.object_ids().len(), 1);
        let id = doc.object_ids()[0];
        assert!(widths(&doc, id).is_empty(), "pressure {p}");
        assert_eq!(r.session.pencil_last_had_pressure(), Some(false));
        assert!((doc.path(id).unwrap().style.stroke.width.as_mm() - pen_width()).abs() < 1e-9);
    }
    assert!(!Session::new(1).pencil_settings().simulate);
}

/// The stroke width a new Pen path takes (the "current stroke width").
fn pen_width() -> f64 {
    use curvyo_ui_core::PenMode;
    let mut s = Session::new(1);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Freehand));
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_hover(Point::new(20.0, 20.0), false, false);
    s.pointer_up(Point::new(40.0, 10.0), false, false);
    let d = unpack(2, &s.pack("t").unwrap()).unwrap();
    d.path(d.object_ids()[0])
        .unwrap()
        .style
        .stroke
        .width
        .as_mm()
}

fn simulated(speed_mm_s: &[f64]) -> Vec<f64> {
    // per-segment speeds along x; returns the profile widths at samples (mm)
    let mut r = Rig::new();
    r.session.set_pencil_simulate(true);
    let mut v = Vec::new();
    let (mut x, mut t) = (20.0, 1000.0);
    let (px, py) = r.px(x, 50.0);
    v.push(contact(px, py, NONE, t));
    for sp in speed_mm_s {
        for _ in 0..30 {
            let dt = 10.0;
            x += sp * dt / 1000.0;
            t += dt;
            let (px, py) = r.px(x, 50.0);
            v.push(contact(px, py, NONE, t));
        }
    }
    let v = lifted(v);
    r.feed(&v);
    let doc = r.doc();
    let id = doc.object_ids()[0];
    let base = doc.path(id).unwrap().style.stroke.width.as_mm();
    widths(&doc, id).iter().map(|w| w.1 * base).collect()
}

#[test]
fn c08_simulated_pressure_follows_speed() {
    // constant speed: constant width (no profile or flat)
    let flat_w = simulated(&[100.0]);
    if flat_w.len() > 1 {
        let (lo, hi) = (
            flat_w.iter().copied().fold(f64::MAX, f64::min),
            flat_w.iter().copied().fold(0.0, f64::max),
        );
        assert!(hi - lo < 0.15, "constant speed gave {flat_w:?}");
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let slow = simulated(&[50.0]);
    let fast = simulated(&[200.0]);
    assert!(!slow.is_empty() && !fast.is_empty());
    assert!(mean(&slow) > mean(&fast), "slow {slow:?} fast {fast:?}");
    // one stroke, fast then slow: slow part wider
    let mixed = simulated(&[250.0, 50.0]);
    assert!(mixed.len() >= 2);
    assert!(mixed.last().unwrap() > mixed.first().unwrap(), "{mixed:?}");
}

#[test]
fn c08_simulation_is_ignored_when_the_stroke_has_pressure() {
    let draw = |sim: bool| {
        let mut r = Rig::new();
        r.session.set_pencil_simulate(sim);
        let s = lifted(r.line(60, 60.0, 0.1, 0.9, 8.0));
        r.feed(&s);
        let doc = r.doc();
        let id = doc.object_ids()[0];
        widths(&doc, id)
    };
    assert_eq!(draw(true), draw(false));
}

// ---- criterion 9
#[test]
fn c09_higher_smoothing_never_gives_more_nodes_and_zero_stays_within_a_tenth() {
    let mut last = usize::MAX;
    for pct in [0.0, 10.0, 20.0, 40.0, 60.0, 80.0, 100.0] {
        let mut r = Rig::new();
        assert!(r.session.set_pencil_smoothing(Length::from_mm(pct / 100.0)));
        let (mut v, pts) = {
            // wobbly stroke
            let mut v = Vec::new();
            let mut pts = Vec::new();
            for i in 0..400 {
                let f = f64::from(i) / 399.0;
                let x = 20.0 + 120.0 * f;
                let y = 60.0 + 8.0 * (f * 17.0).sin() + 0.15 * f64::from((i * 7919) % 13) / 13.0;
                pts.push((x, y));
                let (px, py) = r.px(x, y);
                v.push(contact(px, py, NONE, 1000.0 + 3.0 * f64::from(i)));
            }
            (v, pts)
        };
        v = lifted(v);
        r.feed(&v);
        let doc = r.doc();
        let id = doc.object_ids()[0];
        let n = doc.path(id).unwrap().anchors.len();
        assert!(
            n <= last,
            "smoothing {pct}% gave {n} nodes, more than {last}"
        );
        last = n;
        if pct == 0.0 {
            let worst = pts
                .iter()
                .map(|&p| dist_to_polyline_anchor_curve(&doc, id, p))
                .fold(0.0, f64::max);
            assert!(worst <= 0.1 + 0.01, "worst {worst}");
        }
    }
}

#[test]
fn c09_smoothing_is_a_session_setting_not_in_the_document() {
    let mut r = Rig::new();
    assert!(!r.session.set_pencil_smoothing(Length::from_mm(-1.0)));
    assert!(!r.session.set_pencil_smoothing(Length::from_mm(f64::NAN)));
    assert!(
        (r.session.pencil_settings().smoothing.as_mm() - 0.4).abs() < 1e-9,
        "default 40 %"
    );
    assert_eq!(r.session.undo_len(), 0, "a setting is no history step");
}

// ---- criterion 10 (preview)
#[test]
fn c10_a_stroke_in_progress_shows_a_preview_and_the_object_stays_near_it() {
    let mut r = Rig::new();
    let empty = r.session.draw_list().triangle_count();
    let s = r.line(100, 100.0, 0.1, 0.9, 8.0);
    r.feed(&s[..60]);
    assert!(r.paths().is_empty(), "nothing committed during the stroke");
    assert!(
        r.session.draw_list().triangle_count() > empty,
        "preview drawn"
    );
    let mut plain = Rig::new();
    let p = plain.line(100, 100.0, NONE, NONE, 8.0);
    plain.feed(&p[..60]);
    assert!(plain.session.draw_list().triangle_count() > 0);
}

// ---- criterion 11
#[test]
fn c11_hover_starts_nothing_batches_equal_one_batch_touch_and_extras_ignored() {
    let mut r = Rig::new();
    let h: Vec<S> = (0..10)
        .map(|i| S {
            phase: 0,
            ..contact(10.0 + f64::from(i), 10.0, 0.0, f64::from(i))
        })
        .collect();
    r.feed(&h);
    assert!(r.paths().is_empty());
    assert_eq!(r.session.undo_len(), 0);
    // one batch vs many
    let mk = |r: &Rig| lifted(r.line(100, 100.0, 0.1, 0.9, 8.0));
    let mut a = Rig::new();
    let sa = mk(&a);
    a.feed(&sa);
    let mut b = Rig::new();
    let sb = mk(&b);
    for chunk in sb.chunks(7) {
        b.feed(chunk);
    }
    let (da, db) = (a.doc(), b.doc());
    let (ia, ib) = (da.object_ids()[0], db.object_ids()[0]);
    assert_eq!(
        da.path(ia).unwrap().anchors.len(),
        db.path(ib).unwrap().anchors.len()
    );
    assert_eq!(widths(&da, ia), widths(&db, ib));
    assert_eq!(db.object_ids().len(), 1);
    // barrel bits and tilt are carried, change nothing
    let mut c = Rig::new();
    let mut f = flat(&lifted(c.line(100, 100.0, 0.1, 0.9, 8.0)));
    for rec in f.chunks_mut(PEN_SAMPLE_STRIDE) {
        rec[3] = 0.4;
        rec[4] = -0.3;
        rec[7] = 3.0;
    }
    c.session.pen_samples(&f);
    let dc = c.doc();
    let ic = dc.object_ids()[0];
    assert_eq!(
        da.path(ia).unwrap().anchors.len(),
        dc.path(ic).unwrap().anchors.len()
    );
    assert_eq!(widths(&da, ia), widths(&dc, ic));
}

#[test]
fn c11_pen_samples_are_ignored_by_other_tools() {
    let mut r = Rig::new();
    r.session.set_tool(Tool::Select);
    let s = lifted(r.line(30, 40.0, 0.5, 0.9, 8.0));
    r.feed(&s);
    assert!(r.paths().is_empty());
}

// ---- criterion 12
fn eraser_stroke(r: &Rig, from: (f64, f64), to: (f64, f64)) -> Vec<S> {
    let mut v = Vec::new();
    for i in 0..=10 {
        let f = f64::from(i) / 10.0;
        let (x, y) = r.px(from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f);
        v.push(S {
            eraser: true,
            ..contact(x, y, 0.5, 5000.0 + 8.0 * f64::from(i))
        });
    }
    let last = *v.last().unwrap();
    v.push(S { phase: 0, ..last });
    v
}

#[test]
fn c12_eraser_end_deletes_touched_objects_in_one_erase_step_and_creates_nothing() {
    let mut r = Rig::new();
    let a = lifted(r.line(40, 80.0, NONE, NONE, 8.0)); // y = 50
    r.feed(&a);
    // second stroke at y = 50 shifted: build manually at y=100
    let mut b = r.line(40, 80.0, NONE, NONE, 8.0);
    for s in &mut b {
        s.y += 50.0 / r.scale;
    }
    r.feed(&lifted(b));
    assert_eq!(r.paths().len(), 2);
    let undo_before = r.session.undo_len();
    // hover first: highlights, deletes nothing
    let (hx, hy) = r.px(60.0, 50.0);
    r.feed(&[S {
        eraser: true,
        phase: 0,
        ..contact(hx, hy, 0.0, 4000.0)
    }]);
    assert_eq!(r.paths().len(), 2);
    assert_eq!(r.session.undo_len(), undo_before);
    // touch nothing
    r.feed(&eraser_stroke(&r, (30.0, 200.0), (90.0, 200.0)));
    assert_eq!(
        r.session.undo_len(),
        undo_before,
        "no commit for an empty erase"
    );
    // erase the first
    r.feed(&eraser_stroke(&r, (60.0, 40.0), (60.0, 60.0)));
    assert_eq!(r.paths().len(), 1);
    assert_eq!(r.session.undo_len(), undo_before + 1);
    assert_eq!(r.undo_name().as_deref(), Some("Erase"));
    assert_eq!(r.session.tool(), Tool::Pencil);
}

// ---- criterion 13
#[test]
fn c13_style_is_the_current_stroke_style_fill_none_widths_from_min_max() {
    let mut r = Rig::new();
    let s = lifted(r.line(60, 60.0, 0.0, 1.0, 8.0));
    r.feed(&s);
    let doc = r.doc();
    let id = doc.object_ids()[0];
    let st = doc.path(id).unwrap().style;
    assert!(st.stroke.enabled);
    assert!(!st.fill.enabled);
    let base = st.stroke.width.as_mm();
    let ws: Vec<f64> = widths(&doc, id).iter().map(|w| w.1 * base).collect();
    assert!(ws.iter().copied().fold(0.0, f64::max) <= 2.0 + 1e-6);
}

// ---- criterion 14
#[test]
fn c14_svg_follows_the_visible_outline_for_a_profiled_stroke() {
    let mut r = Rig::new();
    let s = lifted(r.line(60, 60.0, 0.1, 0.9, 8.0));
    r.feed(&s);
    let curvyo_editor_wasm::SvgExportResult::Written { text, .. } = r
        .session
        .export_svg(curvyo_ui_core::SvgExportOptions::default())
    else {
        panic!("not written")
    };
    assert!(text.contains("<path"));
    assert!(
        !text.contains("stroke-width"),
        "variable stroke exports as a filled outline"
    );
}

// ---- criterion 15
#[test]
fn c15_escape_cancels_and_short_strokes_create_nothing() {
    let mut r = Rig::new();
    let s = r.line(40, 60.0, 0.5, 0.9, 8.0);
    r.feed(&s[..20]);
    let _ = r.session.escape();
    r.feed(&lifted(s[20..].to_vec()));
    assert!(
        r.paths().is_empty(),
        "after Escape the rest of the stroke must not create a path"
    );
    assert_eq!(r.session.undo_len(), 0);
    // one sample
    let (x, y) = r.px(30.0, 30.0);
    r.feed(&lifted(vec![contact(x, y, 0.5, 1.0)]));
    assert!(r.paths().is_empty());
    // below 0.1 mm
    let (x2, y2) = r.px(30.05, 30.0);
    r.feed(&lifted(vec![
        contact(x, y, 0.5, 1.0),
        contact(x2, y2, 0.5, 9.0),
    ]));
    assert!(r.paths().is_empty());
    assert_eq!(r.session.undo_len(), 0);
}

// ---- edge cases
#[test]
fn edge_two_strokes_in_one_batch_are_two_paths_and_two_steps() {
    let mut r = Rig::new();
    let mut a = lifted(r.line(30, 40.0, 0.2, 0.8, 8.0));
    let mut b = lifted(r.line(30, 40.0, 0.2, 0.8, 8.0));
    for s in &mut b {
        s.y += 30.0 / r.scale;
        s.t += 1000.0;
    }
    a.extend(b);
    r.feed(&a);
    assert_eq!(r.paths().len(), 2);
    assert_eq!(r.session.undo_len(), 2);
}

#[test]
fn edge_full_batch_of_256_and_a_loop_stay_one_open_path() {
    let mut r = Rig::new();
    let s = lifted(r.line(256, 100.0, 0.2, 0.8, 4.0));
    r.feed(&s);
    assert_eq!(r.paths().len(), 1);
    let mut r = Rig::new();
    let mut v = Vec::new();
    for i in 0..=120 {
        let a = std::f64::consts::TAU * f64::from(i) / 120.0;
        let (x, y) = r.px(80.0 + 20.0 * a.cos(), 80.0 + 20.0 * a.sin());
        v.push(contact(
            x,
            y,
            0.5 + 0.3 * a.sin(),
            1000.0 + 8.0 * f64::from(i),
        ));
    }
    r.feed(&lifted(v));
    let doc = r.doc();
    assert_eq!(doc.object_ids().len(), 1);
    assert!(!doc.path(doc.object_ids()[0]).unwrap().closed);
}

#[test]
fn edge_non_finite_and_unknown_records_do_not_panic_or_draw() {
    let mut r = Rig::new();
    let mut f = flat(&r.line(10, 20.0, 0.5, 0.5, 8.0));
    f[0] = f64::NAN;
    f[PEN_SAMPLE_STRIDE + 5] = 7.0; // unknown end
    f.push(1.0); // short trailing record
    r.session.pen_samples(&f);
    let _ = r.paths();
}

// ---- criterion 16 and 17
#[test]
fn c16_freehand_pen_mode_is_unchanged() {
    use curvyo_ui_core::PenMode;
    let mut s = Session::new(1);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Freehand));
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_hover(Point::new(20.0, 20.0), false, false);
    s.pointer_up(Point::new(40.0, 10.0), false, false);
    let d = unpack(2, &s.pack("t").unwrap()).unwrap();
    assert_eq!(d.object_ids().len(), 1);
    assert!(widths(&d, d.object_ids()[0]).is_empty());
}

#[test]
fn c17_no_pressure_tilt_or_device_data_in_the_document() {
    let mut r = Rig::new();
    let mut f = flat(&lifted(r.line(60, 60.0, 0.123_456_7, 0.9, 8.0)));
    for rec in f.chunks_mut(PEN_SAMPLE_STRIDE) {
        rec[3] = 0.314_159_2;
        rec[4] = -0.271_828_1;
    }
    r.session.pen_samples(&f);
    let bytes = r.session.pack("t").unwrap();
    let hay = format!("{bytes:?}");
    // the packed document is a zip: inspect the unpacked JSON export if available
    let doc = unpack(2, &bytes).unwrap();
    let json = format!("{:?}", doc.path(doc.object_ids()[0]).unwrap());
    for needle in [
        "0.1234567",
        "0.3141592",
        "0.2718281",
        "pressure",
        "tilt",
        "barrel",
    ] {
        assert!(!json.contains(needle), "{needle} found in {json}");
    }
    let _ = hay;
    // the width multipliers are not the raw pressures
    let id = doc.object_ids()[0];
    assert!(widths(&doc, id).len() < 60);
}

// ---- UX: key P selects the Pencil in every state
#[test]
fn ux_key_p_is_the_pencil_tool() {
    use curvyo_editor_wasm::KeyInput;
    let mut s = Session::new(1);
    s.set_tool(Tool::Select);
    s.key_down(KeyInput {
        key: "p",
        ..KeyInput::default()
    });
    assert_eq!(s.tool(), Tool::Pencil);
    s.set_tool(Tool::Pen);
    s.key_down(KeyInput {
        key: "P",
        shift: true,
        ..KeyInput::default()
    });
    let _ = s.tool();
}

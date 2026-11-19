//! The raster job of `specs/0123-raster-engraving` (ADR 0068, ADR 0069) built from `job_input`
//! fixtures with no session: what runs, the sweeps, order, passes, time, GRBL and the checks.

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::cast_precision_loss
)]

mod job_fixtures;

use std::sync::Arc;

use curvyo_document_core::{Color, GradientBox, GradientFill, PlacedGradient, Point};
use curvyo_laser_core::FeedRate;
use curvyo_laser_core::job::{
    Built, Cause, GateFacts, JobInput, LeafGeometry, MoveKind, Names, PlanCache, Severity,
    Timeline, build_job, check_text, estimate_text, gate, lengths_text, prepare_job, write,
};
use curvyo_laser_core::operations::OperationKind::{Cut, Engrave, EngraveOutline, Image, Score};
use curvyo_laser_core::settings::{Dither, ImageSettings, OperationOptions, Role};
use job_fixtures::{Page, circle, grey_image, image, own_engrave, picture, plain, rect};

fn build(input: &JobInput) -> Built {
    build_job(input, &[], &mut PlanCache::default())
}

fn with(dither: Dither) -> OperationOptions {
    OperationOptions {
        image: ImageSettings {
            dither,
            ..ImageSettings::DEFAULT
        },
        ..plain()
    }
}

fn black(w: u32, h: u32) -> curvyo_image_core::WorkingImage {
    grey_image(w, h, |_, _| 0)
}

/// A page with one block of `kind` ("Image 1") of `leaves` at 0.1 mm, 6,000 mm/min, 50 %.
fn image_page(
    kind: curvyo_laser_core::operations::OperationKind,
    options: OperationOptions,
    leaves: Vec<LeafGeometry>,
) -> Page {
    let mut page = Page::new();
    page.values().max_feed = FeedRate::new(60_000).unwrap();
    page.operation(
        kind,
        "Image 1",
        own_engrave(kind, 50.0, 6000, 0.1),
        options,
        leaves,
    );
    page
}

fn sweeps(built: &Built) -> Vec<(u32, Role)> {
    built
        .job
        .moves
        .iter()
        .filter_map(|m| match m.kind {
            MoveKind::Sweep { sweep, role, .. } => Some((sweep, role)),
            _ => None,
        })
        .collect()
}

fn gradient_ellipse() -> LeafGeometry {
    let outline = circle(30.0, 30.0, 10.0);
    let frame = GradientBox::axis_aligned(Point::new(20.0, 20.0), Point::new(40.0, 40.0));
    LeafGeometry::Gradient {
        outlines: vec![outline],
        gradient: PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &frame),
    }
}

#[test]
fn c1_an_image_kind_image_is_rastered_and_a_cut_image_writes_its_frame() {
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![picture(10.0, 10.0, 5.0, 2.0, black(5, 2))],
    );
    let built = build(&page.input);
    assert_eq!(
        sweeps(&built).len(),
        20,
        "one sweep per row of a solid image"
    );
    assert!(
        built
            .job
            .moves
            .iter()
            .all(|m| !matches!(m.kind, MoveKind::Burn { .. })),
        "no frame in Image"
    );
    let mut cut = Page::new();
    cut.operation(
        Cut,
        "Cut 1",
        job_fixtures::own(Cut, 90.0, 300, 1),
        plain(),
        vec![picture(10.0, 10.0, 5.0, 2.0, black(5, 2))],
    );
    let built = build(&cut.input);
    assert!(sweeps(&built).is_empty(), "no pixel in a Cut");
    assert!(built.job.burns());
}

/// A turned and skewed frame: a parallelogram.
fn slanted() -> [Point; 4] {
    [
        Point::new(20.0, 10.0),
        Point::new(48.0, 18.0),
        Point::new(42.0, 40.0),
        Point::new(14.0, 32.0),
    ]
}

fn ring_of(corners: [Point; 4]) -> LeafGeometry {
    let zero = curvyo_document_core::Vec2::ZERO;
    LeafGeometry::Outlines(vec![curvyo_laser_core::job::LeafOutline {
        anchors: corners.iter().map(|&p| (p, zero, zero)).collect(),
        closed: true,
    }])
}

/// Criterion 1 (Frage 271 A): in every kind but Image an image runs, move for move, as a
/// rectangle path on its frame, its pixels never read.
#[test]
fn c1_an_image_in_a_vector_kind_runs_as_the_rectangle_of_its_frame() {
    let image = LeafGeometry::Image {
        frame: slanted(),
        asset: curvyo_document_core::AssetHash::of(b"slanted"),
        pixels: Some(Arc::new(black(8, 8))),
    };
    let unreadable = LeafGeometry::Image {
        frame: slanted(),
        asset: curvyo_document_core::AssetHash::of(b"gone"),
        pixels: None,
    };
    for kind in [Cut, Score, Engrave, EngraveOutline] {
        let run = |leaf: LeafGeometry| {
            let mut page = Page::new();
            page.operation(
                kind,
                "Op 1",
                if kind.has_interval() {
                    own_engrave(kind, 50.0, 6000, 0.5)
                } else {
                    job_fixtures::own(kind, 50.0, 6000, 1)
                },
                plain(),
                vec![leaf],
            );
            build(&page.input).job.moves
        };
        let rectangle = run(ring_of(slanted()));
        assert!(!rectangle.is_empty(), "{kind:?}");
        assert_eq!(run(image.clone()), rectangle, "{kind:?}");
        assert_eq!(
            run(unreadable.clone()),
            rectangle,
            "{kind:?}: no pixel needed"
        );
    }
}

#[test]
fn c1_engrave_outline_hatches_an_image_then_traces_its_frame_with_the_score_set() {
    let page = image_page(
        EngraveOutline,
        plain(),
        vec![picture(10.0, 10.0, 5.0, 2.0, black(5, 2))],
    );
    let built = build(&page.input);
    assert!(sweeps(&built).is_empty(), "no dots outside Image");
    let roles: Vec<Role> = built
        .job
        .moves
        .iter()
        .filter_map(|m| match m.kind {
            MoveKind::Burn { role, .. } => Some(role),
            _ => None,
        })
        .collect();
    let last_main = roles.iter().rposition(|r| *r == Role::Main).unwrap();
    let first_outline = roles.iter().position(|r| *r == Role::Outline).unwrap();
    assert!(last_main < first_outline, "hatch first, then the frame");
}

#[test]
fn c3_a_gradient_shape_rasters_in_image_and_runs_its_outline_in_score() {
    let page = image_page(
        Image,
        with(Dither::FloydSteinberg),
        vec![gradient_ellipse()],
    );
    let built = build(&page.input);
    assert_ne!(sweeps(&built).len(), 0);
    assert!(
        built
            .job
            .moves
            .iter()
            .all(|m| !matches!(m.kind, MoveKind::Burn { .. })),
        "no outline"
    );
    for (index, sweep) in built.job.sweeps.iter().enumerate() {
        for p in [sweep.start, sweep.end] {
            let (dx, dy) = (p.x as f64 / 1000.0 - 30.0, p.y as f64 / 1000.0 - 30.0);
            assert!(
                dx * dx + dy * dy <= 10.06 * 10.06,
                "sweep {index} leaves the ellipse"
            );
        }
    }
    let mut cut = Page::new();
    cut.operation(
        Score,
        "Score 1",
        job_fixtures::own(Score, 20.0, 3000, 1),
        plain(),
        vec![gradient_ellipse()],
    );
    let built = build(&cut.input);
    assert!(
        sweeps(&built).is_empty() && built.job.burns(),
        "only the outline"
    );
}

/// Criterion 3: an Engrave hatches a gradient-filled square exactly as the same solid square.
#[test]
fn c3_an_engrave_of_a_gradient_square_is_the_hatch_of_the_solid_square() {
    let square = rect(10.0, 10.0, 20.0, 20.0);
    let frame = GradientBox::axis_aligned(Point::new(10.0, 10.0), Point::new(30.0, 30.0));
    let gradient = LeafGeometry::Gradient {
        outlines: vec![square.clone()],
        gradient: PlacedGradient::new(&GradientFill::first_for(Color::BLACK, 255), &frame),
    };
    let moves = |leaf| {
        build(&image_page(Engrave, plain(), vec![leaf]).input)
            .job
            .moves
    };
    let solid = moves(LeafGeometry::Outlines(vec![square]));
    assert!(solid.len() > 10);
    assert_eq!(moves(gradient), solid);
}

/// Criterion 48: a leaf in an Image operation that is no picture burns nothing; the session
/// names it in the gate's facts and the check warns once, naming the operation.
#[test]
fn c48_a_plain_shape_in_image_burns_nothing_and_warns() {
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![rect(10.0, 10.0, 20.0, 20.0).into_leaf()],
    );
    let built = build(&page.input);
    assert!(
        built
            .job
            .moves
            .iter()
            .all(|m| !matches!(m.kind, MoveKind::Burn { .. } | MoveKind::Sweep { .. }))
    );
    let op = page.input.blocks[0].id;
    let check = |items: Vec<curvyo_document_core::NodeId>| {
        let timeline = Timeline::of(&built.job);
        let facts = GateFacts {
            not_pictures: vec![(op, items)],
            ..GateFacts::default()
        };
        let g = gate(&page.input, &built, &timeline, &facts);
        let names = Names::of(page.input.blocks.iter().map(|b| (b.id, b.name.clone())));
        g.issues
            .iter()
            .find(|i| matches!(i.cause, Cause::NotPictures { .. }))
            .map(|i| (i.cause.severity(), check_text(i, &names).whole()))
            .unwrap()
    };
    let leaf = page.input.blocks[0].leaves[0].node;
    assert_eq!(
        check(vec![leaf]),
        (
            Severity::Warn,
            "1 item in Image 1 is not a picture and does not burn.".to_owned()
        )
    );
    assert_eq!(
        check(vec![leaf, op]).1,
        "2 items in Image 1 are not pictures and do not burn."
    );
}

#[test]
fn c15_a_half_white_image_writes_no_move_over_the_white_half() {
    let img = grey_image(200, 100, |x, _| if x < 100 { 0 } else { 255 });
    let page = image_page(
        Image,
        with(Dither::FloydSteinberg),
        vec![picture(0.0, 0.0, 20.0, 10.0, img)],
    );
    let built = build(&page.input);
    let mut from = built.job.home;
    for m in &built.job.moves[..built.job.moves.len() - 1] {
        assert!(m.to.x <= 10_000, "{m:?}");
        from = m.to;
    }
    let _ = from;
}

#[test]
fn c16_bidirectional_rows_alternate_and_unidirectional_rows_travel_back() {
    let img = black(10, 2);
    let built = build(
        &image_page(
            Image,
            with(Dither::Threshold),
            vec![picture(0.0, 0.0, 10.0, 2.0, img.clone())],
        )
        .input,
    );
    let s = &built.job.sweeps;
    assert!(s[0].start.x < s[0].end.x && s[1].start.x > s[1].end.x);
    let travel_between: Vec<i128> = built
        .job
        .moves
        .windows(2)
        .filter(|w| matches!(w[0].kind, MoveKind::Sweep { .. }) && w[1].kind == MoveKind::Travel)
        .map(|w| w[0].to.distance_squared(w[1].to))
        .take(3)
        .collect();
    assert_eq!(travel_between, vec![100 * 100; 3], "a one-Interval step");
    let mut options = with(Dither::Threshold);
    options.bidirectional = false;
    let built = build(&image_page(Image, options, vec![picture(0.0, 0.0, 10.0, 2.0, img)]).input);
    assert!(built.job.sweeps.iter().all(|s| s.start.x < s.end.x));
}

#[test]
fn c16_overscan_adds_run_outs_and_crosshatch_is_ignored() {
    let mut options = with(Dither::Threshold);
    options.overscan = curvyo_laser_core::settings::Overscan::new(50).unwrap();
    options.crosshatch = true;
    let built = build(
        &image_page(
            Image,
            options,
            vec![picture(50.0, 50.0, 10.0, 1.0, black(10, 1))],
        )
        .input,
    );
    let rows = sweeps(&built).len();
    assert_eq!(rows, 10, "crosshatch adds no second set");
    let run_outs = built
        .job
        .moves
        .iter()
        .filter(|m| matches!(m.kind, MoveKind::RunOut(_)))
        .count();
    assert_eq!(run_outs, 2 * rows);
}

#[test]
fn c17_passes_repeat_the_same_sweeps() {
    let mut page = Page::new();
    page.operation(
        Image,
        "Image 1",
        job_fixtures::settings(Image, serde_json::json!({"main": {"power_pct": 50, "speed_mm_min": 6000, "passes": 3, "interval_mm": 0.1}})),
        with(Dither::FloydSteinberg),
        vec![picture(0.0, 0.0, 2.0, 2.0, grey_image(4, 4, |x, y| u8::try_from(x * 40 + y * 20).unwrap()))],
    );
    let built = build(&page.input);
    let list = sweeps(&built);
    assert_eq!(list.len(), 3 * built.job.sweeps.len());
    let n = built.job.sweeps.len();
    assert_eq!(list[..n], list[n..2 * n], "the same dots every pass");
}

#[test]
fn c21_a_solid_100_by_10_square_reads_its_time_and_lengths() {
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![picture(0.0, 290.0, 100.0, 10.0, black(100, 10))],
    );
    let built = build(&page.input);
    let timeline = Timeline::of(&built.job);
    assert!(
        (timeline.burn_mm() - 10_000.0).abs() < 0.01,
        "{}",
        timeline.burn_mm()
    );
    assert_eq!(estimate_text(timeline.total_s()), "About 1 min 40 s");
    assert!(lengths_text(timeline.burn_mm(), 9.9).starts_with("Burn 10.00 m"));
    let steps: f64 = built
        .job
        .moves
        .windows(2)
        .filter(|w| {
            matches!(w[0].kind, MoveKind::Sweep { .. })
                && w[1].kind == MoveKind::Travel
                && w[1].block == 0
        })
        .map(|w| w[0].to.distance_mm(w[1].to))
        .sum();
    assert!((steps - 9.9).abs() < 1e-6, "{steps}");
    let head = timeline.head_at(&built.job, 0.5);
    assert!((head.elapsed_s - timeline.total_s() / 2.0).abs() < 0.5);
}

#[test]
fn c19_m4_writes_one_g1_per_run_and_m3_switches_per_run() {
    let img = grey_image(10, 1, |x, _| if x % 4 < 2 { 0 } else { 255 });
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![picture(10.0, 10.0, 10.0, 1.0, img)],
    );
    let built = build(&page.input);
    let text =
        String::from_utf8(write(page.input.machine.values.output_format, &built.job)).unwrap();
    let runs: usize = built.job.sweeps.iter().map(|s| s.runs().count()).sum();
    let g1 = text.lines().filter(|l| l.starts_with("G1 ")).count();
    assert_eq!(g1, runs, "{text}");
    assert!(text.contains(" S500") && text.contains(" S0"));
    let mut page = page;
    page.values().laser_mode = false;
    let built = build(&page.input);
    let text =
        String::from_utf8(write(page.input.machine.values.output_format, &built.job)).unwrap();
    let burned: usize = built
        .job
        .sweeps
        .iter()
        .map(|s| s.runs().filter(|r| r.level > 0).count())
        .sum();
    assert_eq!(
        text.lines().filter(|l| l.starts_with("M3 S500")).count(),
        burned
    );
    assert!(
        !text
            .lines()
            .any(|l| l.starts_with("G1") && l.ends_with(" S0")),
        "M3 gaps are G0"
    );
}

fn checks(page: &Page) -> Vec<(Cause, String)> {
    let built = build(&page.input);
    let timeline = Timeline::of(&built.job);
    let g = gate(&page.input, &built, &timeline, &GateFacts::default());
    let names = Names::of(page.input.blocks.iter().map(|b| (b.id, b.name.clone())));
    g.issues
        .iter()
        .map(|i| (i.cause.clone(), check_text(i, &names).whole()))
        .collect()
}

#[test]
fn c33_grey_on_m3_blocks_and_c32_lowest_above_power_warns() {
    let mut options = with(Dither::Grey);
    options.image.lowest_power_tenths = 600;
    let mut page = image_page(
        Image,
        options,
        vec![picture(10.0, 10.0, 2.0, 2.0, black(2, 2))],
    );
    page.values().laser_mode = false;
    let list = checks(&page);
    let texts: Vec<&str> = list.iter().map(|(_, t)| t.as_str()).collect();
    assert!(
        texts.contains(&"Image 1 needs laser mode M4 for Grey."),
        "{texts:?}"
    );
    assert!(texts.contains(&"Image 1: Lowest power is above Power."));
    let grey = list
        .iter()
        .find(|(c, _)| matches!(c, Cause::GreyNeedsM4 { .. }))
        .unwrap();
    assert_eq!(grey.0.severity(), Severity::Block);
}

#[test]
fn c34_too_many_dots_blocks_with_the_count() {
    let mut page = Page::new();
    page.operation(
        Image,
        "Image 1",
        own_engrave(Image, 50.0, 6000, 0.01),
        with(Dither::Threshold),
        vec![picture(0.0, 0.0, 400.0, 300.0, black(4, 3))],
    );
    let list = checks(&page);
    let dots = list
        .iter()
        .find(|(c, _)| matches!(c, Cause::TooManyDots { .. }))
        .unwrap();
    assert_eq!(
        dots.1,
        "An item would need 1,200,000,000 dots; the limit is 50,000,000."
    );
    assert_eq!(dots.0.severity(), Severity::Block);
    assert!(
        !list.iter().any(|(c, _)| *c == Cause::NothingRuns),
        "the dot limit stands in for Nothing runs (UX review U5): {list:?}"
    );
    let built = build(&page.input);
    let timeline = Timeline::of(&built.job);
    let g = gate(&page.input, &built, &timeline, &GateFacts::default());
    assert_eq!(g.runs[0].time_s, None, "the row reads '–', not '0 s'");
}

#[test]
fn c35_c36_coarse_overlapping_reduced_and_unreadable_images_warn() {
    let mut reduced = black(2, 2);
    reduced.original_size = (4, 4);
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![
            picture(10.0, 10.0, 10.0, 10.0, black(2, 2)),
            picture(15.0, 15.0, 10.0, 10.0, reduced),
            image(100.0, 100.0, 5.0, 5.0),
        ],
    );
    let texts: Vec<String> = checks(&page).into_iter().map(|(_, t)| t).collect();
    let position = |t: &str| {
        texts
            .iter()
            .position(|x| x == t)
            .unwrap_or_else(|| panic!("{t}: {texts:?}"))
    };
    let coarse = position("2 images are coarser than their Interval.");
    let overlap = position("2 items overlap and burn twice.");
    let red = position("1 image was reduced and loses detail.");
    let unread = position("1 image cannot be read and is skipped.");
    assert!(
        coarse < overlap && overlap < red && red < unread,
        "{texts:?}"
    );
}

#[test]
fn c31_a_frame_partly_outside_the_bed_blocks() {
    let img = grey_image(4, 4, |x, _| if x == 0 { 0 } else { 255 });
    let page = image_page(
        Image,
        with(Dither::Threshold),
        vec![picture(390.0, 10.0, 20.0, 20.0, img)],
    );
    let list = checks(&page);
    assert!(
        list.iter()
            .any(|(c, t)| matches!(c, Cause::OutsideBed { .. })
                && t == "1 item lies outside the 400 × 300 mm bed."),
        "{list:?}"
    );
}

#[test]
fn c39_two_exports_are_byte_identical_and_the_cache_reuses_the_pyramid() {
    let img = Arc::new(grey_image(64, 64, |x, y| {
        u8::try_from((x * 3 + y * 2) % 256).unwrap()
    }));
    let leaf = LeafGeometry::Image {
        frame: job_fixtures::frame(5.0, 5.0, 20.0, 20.0),
        asset: curvyo_document_core::AssetHash::of(b"ramp"),
        pixels: Some(img),
    };
    let page = image_page(
        Image,
        with(Dither::Jarvis),
        vec![leaf, rect(40.0, 5.0, 10.0, 10.0).into_leaf()],
    );
    let mut cache = PlanCache::default();
    let a = write(
        page.input.machine.values.output_format,
        &build_job(&page.input, &[], &mut cache).job,
    );
    assert_eq!(cache.pyramids(), 1);
    let b = write(
        page.input.machine.values.output_format,
        &build(&page.input).job,
    );
    assert_eq!(a, b);
}

trait IntoLeaf {
    fn into_leaf(self) -> LeafGeometry;
}

impl IntoLeaf for curvyo_laser_core::job::LeafOutline {
    fn into_leaf(self) -> LeafGeometry {
        LeafGeometry::Outlines(vec![self])
    }
}

/// ADR 0069 §12 (`specs/0123` criteria 12, 37 d): the sweeps worked a few dots at a time across
/// calls, and resumed after another item's edit, give the job built at once.
#[test]
fn c37d_a_job_prepared_in_steps_equals_the_job_built_at_once() {
    for dither in [
        Dither::FloydSteinberg,
        Dither::Stucki,
        Dither::Ordered,
        Dither::Grey,
    ] {
        let img = grey_image(120, 80, |x, y| u8::try_from((x * 2 + y) % 256).unwrap());
        let page = image_page(
            Image,
            with(dither),
            vec![picture(0.0, 0.0, 12.0, 8.0, img), gradient_ellipse()],
        );
        let at_once = build(&page.input);
        let mut cache = PlanCache::default();
        assert!(
            !prepare_job(&page.input, &[], &mut cache, 0),
            "nothing worked yet"
        );
        let mut steps = 0;
        while !prepare_job(&page.input, &[], &mut cache, 500) {
            steps += 1;
            assert!(steps < 10_000);
        }
        assert!(steps > 3, "{dither:?} took {steps} steps");
        assert!(prepare_job(&page.input, &[], &mut cache, 0), "all held");
        assert_eq!(
            build_job(&page.input, &[], &mut cache),
            at_once,
            "{dither:?}"
        );
    }
}

/// A cache made with [`PlanCache::sharing_pyramids`] reads the grey pyramids of the cache it was
/// made from: one pyramid per asset for the job and the Operations view's preview (architect
/// review S3).
#[test]
fn c37_two_caches_sharing_pyramids_hold_one_pyramid_per_asset() {
    let img = Arc::new(grey_image(64, 64, |x, y| {
        u8::try_from((x * 3 + y * 2) % 256).unwrap()
    }));
    let leaf = LeafGeometry::Image {
        frame: job_fixtures::frame(5.0, 5.0, 20.0, 20.0),
        asset: curvyo_document_core::AssetHash::of(b"ramp"),
        pixels: Some(img),
    };
    let page = image_page(Image, with(Dither::Jarvis), vec![leaf]);
    let mut job = PlanCache::default();
    let mut preview = job.sharing_pyramids();
    let _ = build_job(&page.input, &[], &mut job);
    assert_eq!(job.pyramids(), 1);
    assert_eq!(preview.pyramids(), 1, "the preview sees the job's pyramid");
    let _ = build_job(&page.input, &[], &mut preview);
    assert_eq!(job.pyramids(), 1, "no second pyramid");
}

/// Criterion 47 d: an Image row's numbers line ends with its dither, Grey with its range.
#[test]
fn c47_the_image_rows_numbers_name_the_dither() {
    let line = |options: OperationOptions| {
        let page = image_page(
            Image,
            options,
            vec![picture(10.0, 10.0, 2.0, 2.0, black(2, 2))],
        );
        let built = build(&page.input);
        let timeline = Timeline::of(&built.job);
        let g = gate(&page.input, &built, &timeline, &GateFacts::default());
        curvyo_laser_core::job::numbers_text(&g.runs[0].numbers[0])
    };
    assert_eq!(
        line(with(Dither::FloydSteinberg)),
        "50 %, 6,000 mm/min, 1 ×, 0.1 mm · Floyd"
    );
    let mut grey = with(Dither::Grey);
    grey.image.lowest_power_tenths = 105;
    assert_eq!(
        line(grey),
        "50 %, 6,000 mm/min, 1 ×, 0.1 mm · Grey 10.5–50 %"
    );
}

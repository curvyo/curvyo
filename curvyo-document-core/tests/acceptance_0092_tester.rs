//! Tester's acceptance tests for `specs/0092-cooperative-editing` criterion 31 (the page frame
//! as one register, ADR 0031 decision 9a) and the format 34 compatibility of older files.
//! Written from the specification before reading the implementation.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, Length, NodeId, ObjectSnapshot, Page, Point,
    RectBounds, Shape, pack, unpack,
};

const EPS: f64 = 1e-9;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn rect(document: &Document, x: f64, y: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    })
}

fn rect_origin(document: &Document, id: NodeId) -> Point {
    let Some(ObjectSnapshot::Primitive(primitive)) = document.object(id) else {
        panic!("a primitive");
    };
    let Shape::Rect { bounds, .. } = primitive.shape else {
        panic!("a rect");
    };
    bounds.origin
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < EPS && (a.y - b.y).abs() < EPS
}

fn frame(page: &Page) -> (f64, f64, f64, f64) {
    (
        page.origin.x,
        page.origin.y,
        page.size.width.as_mm(),
        page.size.height.as_mm(),
    )
}

fn same_frame(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> bool {
    (a.0 - b.0).abs() < EPS
        && (a.1 - b.1).abs() < EPS
        && (a.2 - b.2).abs() < EPS
        && (a.3 - b.3).abs() < EPS
}

fn centre(f: (f64, f64, f64, f64)) -> Point {
    Point::new(f.0 + f.2 / 2.0, f.1 + f.3 / 2.0)
}

/// Sends `from`'s changes that `to` lacks into `to`.
fn send(from: &Document, to: &Document) {
    let bytes = from.export_updates(&to.version()).unwrap();
    to.import_updates(&bytes).unwrap();
}

fn page_frame(document: &Document, id: &str) -> (f64, f64, f64, f64) {
    frame(document.pages().iter().find(|p| p.id == id).unwrap())
}

/// A tiny deterministic generator for the sweep below (no proptest in this crate).
struct Lcg(u64);

impl Lcg {
    fn next_f64(&mut self, lo: f64, hi: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = f64::from(u32::try_from(self.0 >> 40).unwrap()) / f64::from(1u32 << 24);
        lo + unit * (hi - lo)
    }
}

// ------------------------------------------------------------------ criterion 31

/// Criterion 31, swept: two peers resize the same page to arbitrary sizes. Both converge to the
/// same frame, which is one of the two whole frames (never A's width with B's height or origin),
/// keeps the original centre, and no object moves; the merge does not depend on its order.
#[test]
fn t31_concurrent_resizes_sweep_converge_to_one_whole_frame_around_the_centre() {
    let mut rng = Lcg(0x0092);
    for round in 0..40u64 {
        let base = Document::new(1);
        let ids = [rect(&base, 5.0, 7.0), rect(&base, -30.0, 120.0)];
        let bytes = pack(&base, "t").unwrap();
        let start = frame(&base.pages()[0]);
        let a = unpack(10 + round * 2, &bytes).unwrap();
        let b = unpack(11 + round * 2, &bytes).unwrap();
        let size_a = DocumentSize::from_mm(rng.next_f64(10.0, 900.0), rng.next_f64(10.0, 900.0));
        let size_b = DocumentSize::from_mm(rng.next_f64(10.0, 900.0), rng.next_f64(10.0, 900.0));
        a.resize(size_a).unwrap();
        b.resize(size_b).unwrap();
        let fa = frame(&a.pages()[0]);
        let fb = frame(&b.pages()[0]);
        assert!(near(centre(fa), centre(start)), "a resize keeps the centre");
        // Merge in one direction first, then the other: both orders end equal.
        send(&a, &b);
        send(&b, &a);
        let ma = frame(&a.pages()[0]);
        let mb = frame(&b.pages()[0]);
        assert!(same_frame(ma, mb), "round {round}: {ma:?} != {mb:?}");
        assert!(
            same_frame(ma, fa) || same_frame(ma, fb),
            "round {round}: {ma:?} is neither {fa:?} nor {fb:?}"
        );
        assert!(near(centre(ma), centre(start)));
        assert_eq!(a.size(), a.pages()[0].size, "document size reads the frame");
        for id in ids {
            assert!(near(rect_origin(&a, id), rect_origin(&base, id)));
            assert!(near(rect_origin(&b, id), rect_origin(&base, id)));
            // Read relative to the frame origin, both peers show the same position.
            let ra = rect_origin(&a, id);
            let rb = rect_origin(&b, id);
            assert!(near(
                Point::new(ra.x - ma.0, ra.y - ma.1),
                Point::new(rb.x - mb.0, rb.y - mb.1)
            ));
        }
    }
}

/// Criterion 31: a resize in A concurrent with a fit in B of the same page converges to one of
/// the two whole frames, and the fit moves no object.
#[test]
fn t31_resize_concurrent_with_fit_converges_to_one_whole_frame() {
    for (pa, pb) in [(2, 3), (3, 2)] {
        let base = Document::new(1);
        let one = rect(&base, 40.0, 50.0);
        let two = rect(&base, 100.0, 90.0);
        let bytes = pack(&base, "t").unwrap();
        let a = unpack(pa, &bytes).unwrap();
        let b = unpack(pb, &bytes).unwrap();
        a.resize(DocumentSize::from_mm(500.0, 600.0)).unwrap();
        assert_eq!(
            b.fit_to_content((Point::new(40.0, 50.0), Point::new(120.0, 100.0))),
            Ok(true)
        );
        let fb = frame(&b.pages()[0]);
        assert!(same_frame(fb, (40.0, 50.0, 80.0, 50.0)), "{fb:?}");
        let fa = frame(&a.pages()[0]);
        send(&a, &b);
        send(&b, &a);
        let merged = frame(&a.pages()[0]);
        assert!(same_frame(merged, frame(&b.pages()[0])));
        assert!(
            same_frame(merged, fa) || same_frame(merged, fb),
            "{merged:?}"
        );
        for d in [&a, &b] {
            assert!(near(rect_origin(d, one), Point::new(40.0, 50.0)));
            assert!(near(rect_origin(d, two), Point::new(100.0, 90.0)));
        }
    }
}

/// Criterion 31 with three peers: three concurrent resizes of one page end in one of the three
/// whole frames in every replica, whatever the delivery order.
#[test]
fn t31_three_concurrent_resizes_end_in_one_whole_frame() {
    let base = Document::new(1);
    rect(&base, 0.0, 0.0);
    let bytes = pack(&base, "t").unwrap();
    let peers: Vec<Document> = (2..5).map(|p| unpack(p, &bytes).unwrap()).collect();
    let sizes = [(300.0, 200.0), (150.0, 450.0), (700.0, 80.0)];
    let mut frames = Vec::new();
    for (peer, (w, h)) in peers.iter().zip(sizes) {
        peer.resize(DocumentSize::from_mm(w, h)).unwrap();
        frames.push(frame(&peer.pages()[0]));
    }
    // Ring delivery, two rounds, so every replica sees every change in a different order.
    for _ in 0..2 {
        for i in 0..3 {
            send(&peers[i], &peers[(i + 1) % 3]);
        }
    }
    let merged = frame(&peers[0].pages()[0]);
    for peer in &peers {
        assert!(same_frame(frame(&peer.pages()[0]), merged));
    }
    assert!(frames.iter().any(|f| same_frame(*f, merged)), "{merged:?}");
}

/// Criterion 31 on a file written before format 34: both peers open a version 33 file (no frame
/// register yet) and resize its page at the same time; they still converge to one whole frame.
#[test]
fn t31_concurrent_resizes_of_a_version_33_file_converge_to_one_whole_frame() {
    let bytes = fixture("tiling_v33.curvyo");
    let a = unpack(21, &bytes).unwrap();
    let b = unpack(22, &bytes).unwrap();
    let ids = a.object_ids();
    let before: Vec<_> = ids.iter().map(|id| a.object(*id)).collect();
    let start = frame(&a.pages()[0]);
    a.resize(DocumentSize::from_mm(333.0, 111.0)).unwrap();
    b.resize(DocumentSize::from_mm(123.0, 456.0)).unwrap();
    let (fa, fb) = (frame(&a.pages()[0]), frame(&b.pages()[0]));
    send(&a, &b);
    send(&b, &a);
    let merged = frame(&a.pages()[0]);
    assert!(same_frame(merged, frame(&b.pages()[0])));
    assert!(
        same_frame(merged, fa) || same_frame(merged, fb),
        "{merged:?}"
    );
    assert!(near(centre(merged), centre(start)));
    let after: Vec<_> = ids.iter().map(|id| a.object(*id)).collect();
    assert_eq!(before, after, "no object changes through a resize");
}

/// Criterion 31: a resize in A and a fit in B of a version 33 file, one on each of two pages
/// (the multi-page fixture of format 26), are both kept.
#[test]
fn t31_resizes_of_different_pages_of_an_old_file_are_both_kept() {
    let bytes = fixture("pages_v26.curvyo");
    let a = unpack(31, &bytes).unwrap();
    let b = unpack(32, &bytes).unwrap();
    let pages = a.pages();
    assert!(pages.len() >= 2, "the fixture has several pages");
    let (first, second) = (pages[0].id.clone(), pages[1].id.clone());
    a.resize_page(&first, DocumentSize::from_mm(250.0, 260.0))
        .unwrap();
    b.resize_page(&second, DocumentSize::from_mm(90.0, 95.0))
        .unwrap();
    let (fa, fb) = (page_frame(&a, &first), page_frame(&b, &second));
    send(&a, &b);
    send(&b, &a);
    for d in [&a, &b] {
        assert!(same_frame(page_frame(d, &first), fa));
        assert!(same_frame(page_frame(d, &second), fb));
    }
}

/// Criterion 31: a resize concurrent with a rename of the same page keeps both.
#[test]
fn t31_resize_concurrent_with_a_rename_keeps_both() {
    let base = Document::new(1);
    let bytes = pack(&base, "t").unwrap();
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    let id = a.pages()[0].id.clone();
    a.resize(DocumentSize::from_mm(321.0, 123.0)).unwrap();
    assert!(b.rename_page(&id, "Cut sheet").unwrap());
    send(&a, &b);
    send(&b, &a);
    for d in [&a, &b] {
        let page = d.pages()[0].clone();
        assert_eq!(page.name, "Cut sheet");
        assert_eq!(page.size, DocumentSize::from_mm(321.0, 123.0));
    }
}

/// Criterion 31 (and 11's "no half-deleted object" for pages): a resize of a page another peer
/// deletes at the same time converges without a half-deleted page.
#[test]
fn t31_resize_of_a_page_deleted_by_a_peer_converges() {
    let base = Document::new(1);
    let second = base.add_page();
    let bytes = pack(&base, "t").unwrap();
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    a.resize_page(&second.id, DocumentSize::from_mm(80.0, 80.0))
        .unwrap();
    b.delete_page(&second.id).unwrap();
    send(&a, &b);
    send(&b, &a);
    assert_eq!(a.pages(), b.pages());
    for page in a.pages() {
        let f = frame(&page);
        assert!(f.0.is_finite() && f.1.is_finite() && f.2 > 0.0 && f.3 > 0.0);
    }
    // The merged state survives a save and reopen.
    let reopened = unpack(9, &pack(&a, "t").unwrap()).unwrap();
    assert_eq!(reopened.pages(), a.pages());
}

/// Criterion 31: after a concurrent resize, the next sequential resize by either peer wins in
/// both replicas (the register is still live after a conflict).
#[test]
fn t31_a_resize_after_a_conflict_wins_everywhere() {
    let base = Document::new(1);
    let bytes = pack(&base, "t").unwrap();
    let a = unpack(2, &bytes).unwrap();
    let b = unpack(3, &bytes).unwrap();
    a.resize(DocumentSize::from_mm(300.0, 300.0)).unwrap();
    b.resize(DocumentSize::from_mm(100.0, 100.0)).unwrap();
    send(&a, &b);
    send(&b, &a);
    let loser_is_a = a.pages()[0].size != DocumentSize::from_mm(300.0, 300.0);
    let loser = if loser_is_a { &a } else { &b };
    loser.resize(DocumentSize::from_mm(222.0, 111.0)).unwrap();
    send(&a, &b);
    send(&b, &a);
    for d in [&a, &b] {
        assert_eq!(d.pages()[0].size, DocumentSize::from_mm(222.0, 111.0));
    }
}

// ------------------------------------------------------------------ format 34 compatibility

/// Every valid fixture of an older format still opens with format 34; its pages keep their
/// frames through a save and reopen, and each frame is finite and positive.
#[test]
fn format_34_opens_every_older_valid_fixture_and_keeps_its_frames() {
    assert_eq!(CURRENT_FORMAT_VERSION, 34);
    let valid = [
        "pages_v26.curvyo",
        "tiling_v33.curvyo",
        "pattern_v32.curvyo",
        "dimension_v31.curvyo",
        "grid_v30.curvyo",
        "sketch_plate_v29.curvyo",
        "snapping_v27.curvyo",
        "image_v25.curvyo",
        "text_layout_v24.curvyo",
        "text_clones_v23.curvyo",
        "groups_v11.curvyo",
        "background_v10.curvyo",
        "primitives_v3.curvyo",
        "paths_v2.curvyo",
        "format_version_1.curvyo",
        "valid.curvyo",
    ];
    for name in valid {
        let opened = unpack(40, &fixture(name)).unwrap_or_else(|e| panic!("{name} opens: {e:?}"));
        let pages = opened.pages();
        assert!(!pages.is_empty(), "{name}");
        for page in &pages {
            let f = frame(page);
            assert!(
                f.0.is_finite() && f.1.is_finite() && f.2 > 0.0 && f.3 > 0.0,
                "{name}: {f:?}"
            );
        }
        assert_eq!(opened.size(), pages[0].size, "{name}");
        let resaved = unpack(41, &pack(&opened, "t").unwrap()).unwrap();
        assert_eq!(resaved.pages(), pages, "{name} keeps its frames");
    }
}

/// An old multi-page file keeps every page's origin when it is resaved at format 34, and a
/// resize of its second page keeps that page's centre.
#[test]
fn format_34_keeps_old_page_origins_and_centres_on_resize() {
    let opened = unpack(50, &fixture("pages_v26.curvyo")).unwrap();
    let pages = opened.pages();
    let second = pages[1].clone();
    let start = frame(&second);
    opened
        .resize_page(
            &second.id,
            DocumentSize::from_mm(start.2 + 40.0, start.3 + 10.0),
        )
        .unwrap();
    let after = page_frame(&opened, &second.id);
    assert!(near(centre(after), centre(start)), "{start:?} -> {after:?}");
    let first_after = frame(&opened.pages()[0]);
    assert!(
        same_frame(first_after, frame(&pages[0])),
        "page 1 untouched"
    );
}

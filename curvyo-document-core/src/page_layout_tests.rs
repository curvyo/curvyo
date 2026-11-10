//! Tests of the page auto-layout (`specs/0109-page-autolayout` criteria 1 to 13, 22 to 24, 32 to 40).

use proptest::prelude::*;

use super::*;
use crate::{PageGrid, PageKind};

const A4: (f64, f64) = (210.0, 297.0);

fn page(id: &str, x: f64, y: f64, w: f64, h: f64) -> Page {
    Page {
        id: id.to_owned(),
        name: id.to_owned(),
        origin: Point::new(x, y),
        size: DocumentSize::from_mm(w, h),
        grid: PageGrid::DEFAULT,
        kind: PageKind::Drawing,
    }
}

fn frame_of(p: &Page) -> Frame {
    Frame {
        origin: p.origin,
        size: p.size,
    }
}

/// `p` resized to `w` x `h` about its centre, as `resize_page` does (ADR 0031 9a).
fn resized(p: &Page, w: f64, h: f64) -> Frame {
    frame_of(p).resized(DocumentSize::from_mm(w, h))
}

/// `p` turned to the other orientation about its centre.
fn turned(p: &Page) -> Frame {
    resized(p, p.size.height.as_mm(), p.size.width.as_mm())
}

/// `pages` with the layout applied and `changed` set to `new`.
fn apply(pages: &[Page], changed: Option<(&str, Frame)>, layout: &Layout) -> Vec<Page> {
    pages
        .iter()
        .map(|p| {
            let mut p = p.clone();
            if let Some((id, frame)) = changed
                && p.id == id
            {
                p.origin = frame.origin;
                p.size = frame.size;
            }
            if let Some((_, by)) = layout.moves.iter().find(|(id, _)| *id == p.id) {
                p.origin = p.origin.translated(*by);
            }
            p
        })
        .collect()
}

/// A row of `n` pages of `size` laid out by Add page (`next_page_origin`).
fn add_page_row(n: usize, (w, h): (f64, f64)) -> Vec<Page> {
    let mut pages: Vec<Page> = Vec::new();
    for i in 0..n {
        let origin = crate::pages::next_page_origin(&pages, DocumentSize::from_mm(w, h));
        pages.push(page(&format!("p{i}"), origin.x, origin.y, w, h));
    }
    pages
}

/// A grid of `rows` x `cols` pages of `size` with the Add page gap.
fn grid(rows: usize, cols: usize, (w, h): (f64, f64)) -> Vec<Page> {
    let gap = page_gap(DocumentSize::from_mm(w, h));
    let mut pages = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = (c as f64 * (w + gap), r as f64 * (h + gap));
            pages.push(page(&format!("r{r}c{c}"), x, y, w, h));
        }
    }
    pages
}

fn moved(layout: &Layout, id: &str) -> Vec2 {
    layout
        .moves
        .iter()
        .find(|(m, _)| m == id)
        .map_or(Vec2::ZERO, |(_, by)| *by)
}

fn assert_vec(actual: Vec2, x: f64, y: f64) {
    assert!(
        (actual.x - x).abs() < 1e-6 && (actual.y - y).abs() < 1e-6,
        "{actual:?} is not ({x}, {y})"
    );
}

fn assert_same_frames(a: &[Page], b: &[Page]) {
    for (p, q) in a.iter().zip(b) {
        assert_eq!(p.id, q.id);
        let (r, s) = (Rect::of_page(p), Rect::of_page(q));
        for (u, v) in [(r.x0, s.x0), (r.y0, s.y0), (r.x1, s.x1), (r.y1, s.y1)] {
            assert!((u - v).abs() < 1e-6, "{} at {r:?}, expected {s:?}", p.id);
        }
    }
}

/// Every pair of `after` that shares area also did in `before`; `gone` is left out.
fn no_new_overlap(before: &[Page], after: &[Page], gone: Option<&str>) -> bool {
    let live = |p: &&Page| Some(p.id.as_str()) != gone;
    let b: Vec<_> = before.iter().filter(live).map(Rect::of_page).collect();
    let a: Vec<_> = after.iter().filter(live).map(Rect::of_page).collect();
    (0..a.len())
        .all(|i| (i + 1..a.len()).all(|j| !a[i].shares_area(a[j]) || b[i].shares_area(b[j])))
}

fn distance(a: &Page, b: &Page) -> f64 {
    Rect::of_page(a).distance(Rect::of_page(b))
}

// --- reflow ---------------------------------------------------------------------------------

#[test]
fn c1_landscape_page_one_moves_the_laser_page_beside_it_by_the_width_gain() {
    let mut pages = add_page_row(1, A4);
    let origin = crate::pages::next_page_origin(&pages, DocumentSize::from_mm(600.0, 400.0));
    pages.push(page("laser", origin.x, origin.y, 600.0, 400.0));
    let new = turned(&pages[0]);
    let layout = reflow(&pages, "p0", new);
    assert_vec(moved(&layout, "laser"), 43.5, 0.0);
    let after = apply(&pages, Some(("p0", new)), &layout);
    assert!((after[0].origin.x + 297.0 - (pages[0].origin.x + 210.0 + 43.5)).abs() < 1e-6);
    assert!((distance(&after[0], &after[1]) - 29.7).abs() < 1e-6);
    assert!(no_new_overlap(&pages, &after, None));
    assert!(!layout.capped && !layout.fell_back);
}

#[test]
fn c2_a_neighbour_beside_each_edge_moves_by_the_edge_change_and_keeps_its_distance() {
    for distance_to in [29.7, 55.0, 0.0, 3000.0] {
        for grow in [true, false] {
            let d = if grow { 40.0 } else { -40.0 };
            let p = page("p", 0.0, 0.0, 200.0, 200.0);
            let new = resized(&p, 200.0 + 2.0 * d, 200.0 + 2.0 * d);
            let neighbours = [
                (
                    "right",
                    page("n", 200.0 + distance_to, 0.0, 100.0, 100.0),
                    d,
                    0.0,
                ),
                (
                    "left",
                    page("n", -100.0 - distance_to, 0.0, 100.0, 100.0),
                    -d,
                    0.0,
                ),
                (
                    "bottom",
                    page("n", 0.0, 200.0 + distance_to, 100.0, 100.0),
                    0.0,
                    d,
                ),
                (
                    "top",
                    page("n", 0.0, -100.0 - distance_to, 100.0, 100.0),
                    0.0,
                    -d,
                ),
            ];
            for (side, n, x, y) in neighbours {
                let pages = vec![p.clone(), n];
                let layout = reflow(&pages, "p", new);
                assert_vec(moved(&layout, "n"), x, y);
                let after = apply(&pages, Some(("p", new)), &layout);
                assert!(
                    (distance(&after[0], &after[1]) - distance_to).abs() < 1e-6,
                    "{side} {grow} {distance_to}"
                );
            }
        }
    }
}

#[test]
fn c3_a_row_spreads_by_half_the_width_gain_on_both_sides_and_keeps_every_distance() {
    for n in [2, 5, 8] {
        for turn in [0, n / 2, n - 1] {
            let pages = add_page_row(n, A4);
            let new = turned(&pages[turn]);
            let layout = reflow(&pages, &pages[turn].id, new);
            for (i, p) in pages.iter().enumerate() {
                let expected = match i.cmp(&turn) {
                    std::cmp::Ordering::Less => -43.5,
                    std::cmp::Ordering::Equal => 0.0,
                    std::cmp::Ordering::Greater => 43.5,
                };
                assert_vec(moved(&layout, &p.id), expected, 0.0);
            }
            let after = apply(&pages, Some((&pages[turn].id, new)), &layout);
            for i in 1..n {
                let d = distance(&after[i - 1], &after[i]);
                assert!((d - 29.7).abs() < 1e-6, "n {n} turn {turn} pair {i}: {d}");
            }
        }
    }
}

#[test]
fn c3_coaster_becoming_a4_moves_key_ring_and_board_right_in_the_same_row() {
    let mut pages: Vec<Page> = Vec::new();
    for (id, w, h) in [
        ("coaster", 100.0, 100.0),
        ("key", 50.0, 30.0),
        ("board", 300.0, 200.0),
    ] {
        let origin = crate::pages::next_page_origin(&pages, DocumentSize::from_mm(w, h));
        pages.push(page(id, origin.x, origin.y, w, h));
    }
    let new = resized(&pages[0], A4.0, A4.1);
    let layout = reflow(&pages, "coaster", new);
    assert_vec(moved(&layout, "key"), 55.0, 0.0);
    assert_vec(moved(&layout, "board"), 55.0, 0.0);
    assert!(!layout.fell_back);
}

#[test]
fn c4_a_page_that_gets_smaller_pulls_its_neighbours_back() {
    let pages = add_page_row(3, A4);
    let landscape = turned(&pages[1]);
    let first = reflow(&pages, "p1", landscape);
    let turned_pages = apply(&pages, Some(("p1", landscape)), &first);
    let back = turned(&turned_pages[1]);
    let second = reflow(&turned_pages, "p1", back);
    assert_vec(moved(&second, "p0"), 43.5, 0.0);
    assert_vec(moved(&second, "p2"), -43.5, 0.0);
    assert!(!second.capped && !second.fell_back);
}

/// Portrait, Landscape, Portrait (or the reverse) on every page of `pages` restores every frame.
fn assert_round_trips(pages: &[Page]) {
    for p in pages {
        let new = turned(p);
        let first = reflow(pages, &p.id, new);
        let after = apply(pages, Some((&p.id, new)), &first);
        assert!(no_new_overlap(pages, &after, None), "{} first", p.id);
        assert!(!first.capped && !first.fell_back, "{}", p.id);
        let changed = after.iter().find(|q| q.id == p.id).unwrap();
        let back = turned(changed);
        let second = reflow(&after, &p.id, back);
        let restored = apply(&after, Some((&p.id, back)), &second);
        assert!(no_new_overlap(&after, &restored, None), "{} second", p.id);
        assert_same_frames(&restored, pages);
    }
}

#[test]
fn c5_rows_and_grids_of_one_format_round_trip_on_every_page() {
    for (rows, cols) in [(1, 2), (1, 8), (2, 8)] {
        assert_round_trips(&grid(rows, cols, A4));
        assert_round_trips(&grid(rows, cols, (A4.1, A4.0)));
    }
}

#[test]
fn c5_in_the_grid_a_b_d_e_turning_a_moves_b_right_and_d_up_and_leaves_e() {
    let pages = grid(2, 2, A4);
    let new = turned(&pages[0]);
    let layout = reflow(&pages, "r0c0", new);
    assert_vec(moved(&layout, "r0c1"), 43.5, 0.0);
    assert_vec(moved(&layout, "r1c0"), 0.0, -43.5);
    assert_vec(moved(&layout, "r1c1"), 0.0, 0.0);
}

#[test]
fn c6_in_the_grid_a_to_f_turning_b_moves_a_c_and_e_and_leaves_d_and_f() {
    let pages = grid(2, 3, A4);
    let new = turned(&pages[1]);
    let layout = reflow(&pages, "r0c1", new);
    assert_vec(moved(&layout, "r0c0"), -43.5, 0.0);
    assert_vec(moved(&layout, "r0c2"), 43.5, 0.0);
    assert_vec(moved(&layout, "r1c1"), 0.0, -43.5);
    assert_vec(moved(&layout, "r1c0"), 0.0, 0.0);
    assert_vec(moved(&layout, "r1c2"), 0.0, 0.0);
    let after = apply(&pages, Some(("r0c1", new)), &layout);
    assert!((distance(&after[1], &after[4]) - 29.7).abs() < 1e-6);
    assert_round_trips(&pages);
}

#[test]
fn c7_a_diagonal_page_stays_and_a_far_page_beside_an_edge_moves() {
    let p = page("p", 0.0, 0.0, 200.0, 200.0);
    let diagonal = page("diag", 300.0, 300.0, 100.0, 100.0);
    let far = page("far", 5000.0, 50.0, 100.0, 100.0);
    let pages = vec![p.clone(), diagonal, far];
    let layout = reflow(&pages, "p", resized(&p, 260.0, 200.0));
    assert_vec(moved(&layout, "diag"), 0.0, 0.0);
    assert_vec(moved(&layout, "far"), 30.0, 0.0);
}

#[test]
fn c8_a_corner_page_beside_both_edges_moves_on_both_axes() {
    let p = page("p", 0.0, 0.0, 200.0, 200.0);
    let corner = page("corner", 230.0, 220.0, 100.0, 100.0);
    let pages = vec![p.clone(), corner];
    // Grows by 40 on each side: the corner page then lies beside both the right and the bottom
    // edge's extent (the larger frame).
    let new = resized(&p, 280.0, 280.0);
    let layout = reflow(&pages, "p", new);
    assert_vec(moved(&layout, "corner"), 40.0, 40.0);
}

#[test]
fn c9_a_page_overlapping_the_changed_page_before_does_not_move() {
    let p = page("p", 0.0, 0.0, 200.0, 200.0);
    let over = page("over", 150.0, 50.0, 100.0, 100.0);
    let pages = vec![p.clone(), over];
    let layout = reflow(&pages, "p", resized(&p, 300.0, 200.0));
    assert_eq!(layout.moves, []);
}

#[test]
fn c10_a_pull_stops_where_its_page_would_touch_a_page_outside_the_chain() {
    let p = page("p", 0.0, 0.0, 200.0, 100.0);
    let tall = page("tall", 220.0, 0.0, 100.0, 300.0);
    let blocker = page("blocker", 205.0, 150.0, 10.0, 50.0);
    let pages = vec![p.clone(), tall, blocker];
    let new = resized(&p, 100.0, 100.0);
    let layout = reflow(&pages, "p", new);
    assert_vec(moved(&layout, "tall"), -5.0, 0.0);
    assert_vec(moved(&layout, "blocker"), 0.0, 0.0);
    assert!(layout.capped);
    let after = apply(&pages, Some(("p", new)), &layout);
    assert!(no_new_overlap(&pages, &after, None));
}

#[test]
fn c11_a_pair_that_comes_to_overlap_sends_the_later_page_to_a_new_row() {
    let before = [
        Rect::of_page(&page("a", 0.0, 0.0, 100.0, 100.0)),
        Rect::of_page(&page("b", 200.0, 0.0, 100.0, 100.0)),
        Rect::of_page(&page("c", 400.0, 0.0, 100.0, 100.0)),
    ];
    let mut current = before;
    current[1] = Rect::of_page(&page("b", 450.0, 0.0, 100.0, 100.0));
    assert!(settle(&before, &mut current, None, None));
    // "c" is the later page of the pair: it starts a new row below the lowest page.
    assert_eq!(
        current[1],
        Rect::of_page(&page("b", 450.0, 0.0, 100.0, 100.0))
    );
    assert_eq!(
        current[2],
        Rect::of_page(&page("c", 0.0, 120.0, 100.0, 100.0))
    );
    let mut kept = before;
    kept[1] = current[1];
    assert!(settle(&before, &mut kept, Some(2), None));
    assert_eq!(kept[1], Rect::of_page(&page("b", 0.0, 120.0, 100.0, 100.0)));
}

#[test]
fn c13_no_page_beside_a_moving_edge_moves_nothing() {
    let p = page("p", 0.0, 0.0, 200.0, 200.0);
    let pages = vec![p.clone(), page("q", 0.0, 500.0, 100.0, 100.0)];
    let layout = reflow(&pages, "p", resized(&p, 300.0, 200.0));
    assert_eq!(layout.moves, []);
}

#[test]
fn c32_a_corner_anchored_resize_moves_only_the_pages_beside_right_and_bottom() {
    let p = page("p", 0.0, 0.0, 200.0, 200.0);
    let pages = vec![
        p.clone(),
        page("right", 230.0, 0.0, 100.0, 100.0),
        page("left", -130.0, 0.0, 100.0, 100.0),
        page("below", 0.0, 230.0, 100.0, 100.0),
        page("above", 0.0, -130.0, 100.0, 100.0),
    ];
    let new = Frame {
        origin: p.origin,
        size: DocumentSize::from_mm(260.0, 250.0),
    };
    let layout = reflow(&pages, "p", new);
    assert_vec(moved(&layout, "right"), 60.0, 0.0);
    assert_vec(moved(&layout, "below"), 0.0, 50.0);
    assert_vec(moved(&layout, "left"), 0.0, 0.0);
    assert_vec(moved(&layout, "above"), 0.0, 0.0);
}

// --- push_apart -----------------------------------------------------------------------------

#[test]
fn c22_a_dropped_page_pushes_the_page_it_lands_on_to_its_gap() {
    let pages = vec![
        page("a", 0.0, 0.0, A4.0, A4.1),
        page("b", 300.0, 0.0, A4.0, A4.1),
    ];
    let new = Frame {
        origin: Point::new(200.0, 0.0),
        size: pages[0].size,
    };
    let layout = push_apart(&pages, "a", new);
    assert_vec(moved(&layout, "b"), 139.7, 0.0);
    assert_vec(moved(&layout, "a"), 0.0, 0.0);
    let after = apply(&pages, Some(("a", new)), &layout);
    assert!((distance(&after[0], &after[1]) - 29.7).abs() < 1e-6);
}

#[test]
fn c22_pushed_pages_push_on_in_a_chain() {
    let pages = add_page_row(3, A4);
    let new = Frame {
        origin: Point::new(pages[1].origin.x - 10.0, 0.0),
        size: pages[0].size,
    };
    let layout = push_apart(&pages, "p0", new);
    let after = apply(&pages, Some(("p0", new)), &layout);
    assert!(no_new_overlap(&pages, &after, None));
    assert!((distance(&after[0], &after[1]) - 29.7).abs() < 1e-6);
    assert!((distance(&after[1], &after[2]) - 29.7).abs() < 1e-6);
    assert!(!layout.fell_back);
}

#[test]
fn c22_a_drop_between_two_pages_pushes_both_away() {
    let pages = vec![
        page("a", 0.0, 0.0, 100.0, 100.0),
        page("b", 130.0, 0.0, 100.0, 100.0),
        page("x", 0.0, 500.0, 60.0, 100.0),
    ];
    let new = Frame {
        origin: Point::new(85.0, 0.0),
        size: pages[2].size,
    };
    let layout = push_apart(&pages, "x", new);
    assert_vec(moved(&layout, "a"), -35.0, 0.0);
    assert_vec(moved(&layout, "b"), 35.0, 0.0);
}

#[test]
fn c23_a_drop_that_only_comes_close_moves_nothing() {
    let pages = vec![
        page("a", 0.0, 0.0, 100.0, 100.0),
        page("b", 300.0, 0.0, 100.0, 100.0),
    ];
    let new = Frame {
        origin: Point::new(200.0, 0.0),
        size: pages[0].size,
    };
    assert_eq!(push_apart(&pages, "a", new).moves, []);
}

#[test]
fn c24_a_pair_overlapping_at_press_is_not_pushed() {
    let pages = vec![
        page("drawing", 0.0, 0.0, 300.0, 300.0),
        page("laser", 50.0, 50.0, 100.0, 100.0),
    ];
    let new = Frame {
        origin: Point::new(80.0, 60.0),
        size: pages[1].size,
    };
    assert_eq!(push_apart(&pages, "laser", new).moves, []);
}

#[test]
fn c22_a_page_pushed_back_onto_the_dropped_page_goes_to_a_new_row() {
    let pages = vec![
        page("d", 1000.0, 1000.0, 100.0, 100.0),
        page("r", 40.0, 0.0, 80.0, 100.0),
        page("q", 125.0, -500.0, 10.0, 1000.0),
    ];
    let new = Frame {
        origin: Point::new(0.0, 0.0),
        size: pages[0].size,
    };
    let layout = push_apart(&pages, "d", new);
    assert!(layout.fell_back);
    let after = apply(&pages, Some(("d", new)), &layout);
    assert!(no_new_overlap(&pages, &after, None));
    assert_vec(moved(&layout, "r"), 80.0, 0.0);
    // The fallback row starts below every page, at the leftmost edge.
    assert_eq!(after[2].origin, Point::new(0.0, 120.0));
}

// --- close_gap ------------------------------------------------------------------------------

#[test]
fn c35_deleting_the_middle_of_three_a4_pages_moves_the_last_239_7_mm_left() {
    let pages = add_page_row(3, A4);
    let layout = close_gap(&pages, "p1");
    assert_vec(moved(&layout, "p2"), -239.7, 0.0);
    assert_vec(moved(&layout, "p0"), 0.0, 0.0);
    let after = apply(&pages, None, &layout);
    assert!((distance(&after[0], &after[2]) - 29.7).abs() < 1e-6);
}

#[test]
fn c38_deleting_the_first_page_moves_the_rest_of_the_row_and_the_last_moves_nothing() {
    let pages = add_page_row(3, A4);
    let first = close_gap(&pages, "p0");
    assert_vec(moved(&first, "p1"), -239.7, 0.0);
    assert_vec(moved(&first, "p2"), -239.7, 0.0);
    assert_eq!(close_gap(&pages, "p2").moves, []);
}

#[test]
fn c38_a_dragged_distance_inside_the_chain_stays() {
    let mut pages = add_page_row(3, A4);
    pages.push(page("d", pages[2].origin.x + 210.0 + 55.0, 0.0, A4.0, A4.1));
    let layout = close_gap(&pages, "p1");
    assert_vec(moved(&layout, "p2"), -239.7, 0.0);
    assert_vec(moved(&layout, "d"), -239.7, 0.0);
}

#[test]
fn c38_a_page_in_a_column_closes_upwards() {
    let gap = 29.7;
    let pages = vec![
        page("a", 0.0, 0.0, A4.0, A4.1),
        page("b", 0.0, 297.0 + gap, A4.0, A4.1),
        page("c", 0.0, 2.0 * (297.0 + gap), A4.0, A4.1),
    ];
    let layout = close_gap(&pages, "a");
    assert_vec(moved(&layout, "b"), 0.0, -326.7);
    assert_vec(moved(&layout, "c"), 0.0, -326.7);
}

#[test]
fn c39_the_grid_closes_along_its_row_only() {
    let pages = grid(2, 3, A4);
    let b = close_gap(&pages, "r0c1");
    assert_eq!(b.moves.len(), 1);
    assert_vec(moved(&b, "r0c2"), -239.7, 0.0);
    assert_eq!(close_gap(&pages, "r0c2").moves, []);
    let e = close_gap(&pages, "r1c1");
    assert_eq!(e.moves.len(), 1);
    assert_vec(moved(&e, "r1c2"), -239.7, 0.0);
    let a = close_gap(&pages, "r0c0");
    assert_eq!(a.moves.len(), 2);
    assert_vec(moved(&a, "r0c1"), -239.7, 0.0);
    assert_vec(moved(&a, "r0c2"), -239.7, 0.0);
}

#[test]
fn c40_a_page_overlapping_the_deleted_page_does_not_move_and_stops_the_chain() {
    let mut pages = add_page_row(3, A4);
    pages.push(page("over", 300.0, 100.0, 50.0, 50.0));
    let layout = close_gap(&pages, "p1");
    assert_vec(moved(&layout, "over"), 0.0, 0.0);
    // The last page stops where it touches the page left in the hole (criterion 40).
    assert_vec(moved(&layout, "p2"), -(2.0 * 239.7 - 350.0), 0.0);
    assert!(layout.capped);
}

#[test]
fn c40_a_closing_pull_stops_before_a_page_outside_the_chain() {
    let pages = vec![
        page("a", 0.0, 0.0, 100.0, 100.0),
        page("b", 120.0, 0.0, 100.0, 100.0),
        page("c", 240.0, 0.0, 100.0, 300.0),
        page("q", 150.0, 150.0, 10.0, 50.0),
    ];
    let layout = close_gap(&pages, "b");
    assert_vec(moved(&layout, "c"), -80.0, 0.0);
    assert!(layout.capped);
}

// --- properties -----------------------------------------------------------------------------

/// Up to 12 pages in a 4-column grid of cells, each page somewhere inside its cell: no two
/// share area.
fn layouts() -> impl Strategy<Value = Vec<Page>> {
    (
        prop::collection::vec(
            (30.0..350.0_f64, 30.0..350.0_f64, 0.0..1.0_f64, 0.0..1.0_f64),
            2..12,
        ),
        0.0..60.0_f64,
    )
        .prop_map(|(cells, spacing)| {
            let cell = 400.0 + spacing;
            cells
                .into_iter()
                .enumerate()
                .map(|(i, (w, h, fx, fy))| {
                    #[allow(clippy::cast_precision_loss)]
                    let (cx, cy) = ((i % 4) as f64 * cell, (i / 4) as f64 * cell);
                    page(
                        &format!("p{i}"),
                        cx + fx * (400.0 - w),
                        cy + fy * (400.0 - h),
                        w,
                        h,
                    )
                })
                .collect()
        })
}

proptest! {
    #[test]
    fn reflow_never_moves_the_changed_page_and_never_overlaps(
        pages in layouts(),
        pick in 0..100usize,
        size in (30.0..500.0_f64, 30.0..500.0_f64),
    ) {
        let p = &pages[pick % pages.len()];
        let new = resized(p, size.0, size.1);
        let layout = reflow(&pages, &p.id, new);
        prop_assert!(layout.moves.iter().all(|(id, _)| *id != p.id));
        prop_assert_eq!(&layout, &reflow(&pages, &p.id, new));
        let after = apply(&pages, Some((&p.id, new)), &layout);
        prop_assert!(no_new_overlap(&pages, &after, None));
    }

    /// A turn and the turn back restore every frame when the first turn capped no pull and sent
    /// no page to the new row (criteria 5, 10, 11). Only for turns: after a resize that grows or
    /// shrinks both sides, a page beyond a corner (criterion 8) or one pulled under the frame can
    /// face another edge's extent on the way back.
    #[test]
    fn a_turn_and_the_turn_back_restore_every_frame(pages in layouts(), pick in 0..100usize) {
        let p = &pages[pick % pages.len()];
        let new = turned(p);
        let layout = reflow(&pages, &p.id, new);
        let after = apply(&pages, Some((&p.id, new)), &layout);
        prop_assert!(no_new_overlap(&pages, &after, None));
        if !layout.capped && !layout.fell_back {
            let back = frame_of(p);
            let second = reflow(&after, &p.id, back);
            prop_assert!(!second.capped && !second.fell_back);
            let restored = apply(&after, Some((&p.id, back)), &second);
            assert_same_frames(&restored, &pages);
        }
    }

    #[test]
    fn push_apart_never_moves_the_dropped_page_and_never_overlaps(
        pages in layouts(),
        pick in 0..100usize,
        by in (-500.0..500.0_f64, -500.0..500.0_f64),
    ) {
        let p = &pages[pick % pages.len()];
        let new = Frame { origin: p.origin.translated(Vec2::new(by.0, by.1)), size: p.size };
        let layout = push_apart(&pages, &p.id, new);
        prop_assert!(layout.moves.iter().all(|(id, _)| *id != p.id));
        prop_assert_eq!(&layout, &push_apart(&pages, &p.id, new));
        let after = apply(&pages, Some((&p.id, new)), &layout);
        prop_assert!(no_new_overlap(&pages, &after, None));
    }

    #[test]
    fn close_gap_never_overlaps(pages in layouts(), pick in 0..100usize) {
        let id = pages[pick % pages.len()].id.clone();
        let layout = close_gap(&pages, &id);
        prop_assert_eq!(&layout, &close_gap(&pages, &id));
        let after = apply(&pages, None, &layout);
        prop_assert!(no_new_overlap(&pages, &after, Some(&id)));
    }
}

//! The one read of which page an object belongs to (ADR 0043 §1): a sketch page by ownership,
//! every other page by position.
//!
//! A sketch page's only member is its live owning sketch, wherever its outline lies, and that
//! sketch is a member of no other page. For every other object sketch frames do not exist: it
//! belongs to the drawing and output pages its bounds reach, and to none (the pasteboard)
//! otherwise. The frame move, Fit page, the export's Active page only scope, placement members
//! and the page selection call this read; none keeps its own overlap test.

use std::collections::HashMap;

use curvyo_document_core::{Document, NodeId, Page, PageKind, Point};

/// How far an object's bounds must reach onto a frame to count, per consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The box touches or overlaps the closed frame (ADR 0024 §1: Fit page, placements, the page
    /// selection).
    Touches,
    /// The box shares area with the frame, or a box without width or height lies in the closed
    /// frame, within 1e-6 mm (`0024` criterion 130 (b): the export's Active page only).
    SharesArea,
    /// The box lies wholly in the closed frame (`0059` criterion 9: what a frame move carries).
    Inside,
}

/// How close a box may come to a frame and still share area with it (`0024` criterion 130 (b)).
const AREA_TOLERANCE_MM: f64 = 1e-6;

impl Reach {
    /// Whether the box `min`..`max` reaches `page` this far.
    #[must_use]
    pub fn holds(self, page: &Page, min: Point, max: Point) -> bool {
        match self {
            Self::Touches => page.overlaps(min, max),
            Self::Inside => page.contains(min) && page.contains(max),
            Self::SharesArea => {
                let e = AREA_TOLERANCE_MM;
                let (x0, y0) = (page.origin.x, page.origin.y);
                let (x1, y1) = (x0 + page.size.width.as_mm(), y0 + page.size.height.as_mm());
                if max.x - min.x <= e || max.y - min.y <= e {
                    max.x >= x0 - e && min.x <= x1 + e && max.y >= y0 - e && min.y <= y1 + e
                } else {
                    max.x > x0 + e && min.x < x1 - e && max.y > y0 + e && min.y < y1 - e
                }
            }
        }
    }
}

/// The owning sketches of the live sketch pages, read once for a set of membership questions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageMembership {
    /// Each fixed sketch and the id of the sketch page that owns it.
    owners: HashMap<NodeId, String>,
}

impl PageMembership {
    /// The owners of the sketch pages among `pages` of `document`.
    #[must_use]
    pub fn of(document: &Document, pages: &[Page]) -> Self {
        Self::from_owners(
            pages
                .iter()
                .filter(|p| p.kind == PageKind::Sketch)
                .filter_map(|page| {
                    document
                        .page_sketch(&page.id)
                        .map(|owner| (owner, page.id.clone()))
                }),
        )
    }

    /// The read for the given `(sketch, sketch page id)` owners.
    #[must_use]
    pub fn from_owners(owners: impl IntoIterator<Item = (NodeId, String)>) -> Self {
        Self {
            owners: owners.into_iter().collect(),
        }
    }

    /// Whether `id` is the owning sketch of a sketch page.
    #[must_use]
    pub fn is_fixed(&self, id: NodeId) -> bool {
        self.owners.contains_key(&id)
    }

    /// Whether the object `id` with the bounds `min`..`max` is a member of `page`.
    #[must_use]
    pub fn is_member(
        &self,
        page: &Page,
        id: NodeId,
        (min, max): (Point, Point),
        reach: Reach,
    ) -> bool {
        if page.kind == PageKind::Sketch {
            return self.owners.get(&id) == Some(&page.id);
        }
        !self.is_fixed(id) && reach.holds(page, min, max)
    }

    /// The pages of `pages` the object `id` with the bounds `bounds` belongs to, in list order;
    /// none for an object on the pasteboard.
    pub fn pages_of<'p>(
        &'p self,
        pages: &'p [Page],
        id: NodeId,
        bounds: (Point, Point),
        reach: Reach,
    ) -> impl Iterator<Item = &'p Page> + 'p {
        pages
            .iter()
            .filter(move |page| self.is_member(page, id, bounds, reach))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::{DocumentSize, PageGrid};

    use super::*;

    fn page(id: &str, x: f64, kind: PageKind) -> Page {
        Page {
            id: id.into(),
            name: id.into(),
            origin: Point::new(x, 0.0),
            size: DocumentSize::from_mm(100.0, 100.0),
            grid: PageGrid::DEFAULT,
            kind,
        }
    }

    fn pages() -> Vec<Page> {
        vec![
            page("drawing", 0.0, PageKind::Drawing),
            page("sketch", 100.0, PageKind::Sketch),
        ]
    }

    fn node(counter: i32) -> NodeId {
        serde_json::from_value(serde_json::json!({"peer": "1", "counter": counter})).unwrap()
    }

    fn sketch() -> NodeId {
        node(1)
    }

    fn rect() -> NodeId {
        node(2)
    }

    fn read() -> PageMembership {
        PageMembership::from_owners([(sketch(), "sketch".to_owned())])
    }

    #[test]
    fn a_rectangle_inside_a_sketch_frame_belongs_to_no_page() {
        let pages = pages();
        let inside = (Point::new(120.0, 20.0), Point::new(150.0, 40.0));
        for reach in [Reach::Touches, Reach::SharesArea, Reach::Inside] {
            assert_eq!(read().pages_of(&pages, rect(), inside, reach).count(), 0);
        }
    }

    #[test]
    fn a_sketch_across_a_drawing_page_edge_is_a_member_of_its_sketch_page_only() {
        let pages = pages();
        let across = (Point::new(60.0, 20.0), Point::new(250.0, 40.0));
        for reach in [Reach::Touches, Reach::SharesArea, Reach::Inside] {
            let membership = read();
            let ids: Vec<_> = membership
                .pages_of(&pages, sketch(), across, reach)
                .map(|p| p.id.as_str())
                .collect();
            assert_eq!(ids, ["sketch"]);
        }
    }

    #[test]
    fn an_object_over_both_belongs_to_the_drawing_page_only() {
        let pages = pages();
        let both = (Point::new(90.0, 20.0), Point::new(110.0, 40.0));
        let membership = read();
        let ids: Vec<_> = membership
            .pages_of(&pages, rect(), both, Reach::Touches)
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(ids, ["drawing"]);
        assert!(!read().is_member(&pages[0], rect(), both, Reach::Inside));
    }

    #[test]
    fn the_reaches_differ_at_the_edge() {
        let drawing = &pages()[0];
        let touching = (Point::new(100.0, 10.0), Point::new(120.0, 20.0));
        assert!(Reach::Touches.holds(drawing, touching.0, touching.1));
        assert!(!Reach::SharesArea.holds(drawing, touching.0, touching.1));
        let flat = (Point::new(100.0, 10.0), Point::new(100.0, 20.0));
        assert!(Reach::SharesArea.holds(drawing, flat.0, flat.1));
    }
}

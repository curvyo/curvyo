//! The one read of which page an object belongs to (ADR 0043 §1, ADR 0051 §4): a sketch page by
//! ownership, every other page by position.
//!
//! A sketch page's only member is its live owning sketch, wherever its outline lies, and that
//! sketch is a member of no other page. For every other object sketch frames do not exist: it
//! belongs to the drawing and output pages its bounds reach, and to none (the pasteboard)
//! otherwise. Fit page, the export's Active page only scope, placement members and the page
//! selection call this read by reach; a page that moves (a drag, a size change, a delete) carries
//! the top-level objects whose centre it owns. None keeps its own overlap test.

use std::collections::HashMap;

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Page, PageContents, PageKind, Point};

use crate::{
    Units, VariableArtworkLookup, object_bounds::object_outline_bounds_with_artwork,
    object_outline_bounds,
};

/// How far an object's bounds must reach onto a frame to count, per consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The box touches or overlaps the closed frame (ADR 0024 §1: Fit page, placements, the page
    /// selection).
    Touches,
    /// The box shares area with the frame, or a box without width or height lies in the closed
    /// frame, within 1e-6 mm (`0024` criterion 130 (b): the export's Active page only).
    SharesArea,
}

/// How close a box may come to a frame and still share area with it (`0024` criterion 130 (b)).
const AREA_TOLERANCE_MM: f64 = 1e-6;

impl Reach {
    /// Whether the box `min`..`max` reaches `page` this far.
    #[must_use]
    pub fn holds(self, page: &Page, min: Point, max: Point) -> bool {
        match self {
            Self::Touches => page.overlaps(min, max),
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
    /// Each placed page and the id of the laser page that owns it (`specs/0106` criterion 1).
    placements: HashMap<NodeId, String>,
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
            placements: HashMap::new(),
        }
    }

    /// The read with the given `(placed page, laser page id)` owners added: a placed page is a
    /// member of its laser page by ownership and of no other page (`specs/0106` criterion 1).
    #[must_use]
    pub fn with_placements(
        mut self,
        placements: impl IntoIterator<Item = (NodeId, String)>,
    ) -> Self {
        self.placements = placements.into_iter().collect();
        self
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
        if let Some(owner) = self.placements.get(&id) {
            return *owner == page.id;
        }
        !self.is_fixed(id) && reach.holds(page, min, max)
    }

    /// The page of `pages` that owns the top-level object `id` whose outline box has its centre
    /// at `centre` (ADR 0051 §4, `specs/0109` criteria 14 to 20): a sketch page's own sketch, else
    /// the drawing, unknown or output page whose closed frame holds the centre (1e-9 mm). Sketch
    /// frames do not count; an output page wins over drawing pages; any other tie, or no frame,
    /// is no owner and the object stays.
    #[must_use]
    pub fn owner_by_centre<'p>(
        &self,
        pages: &'p [Page],
        id: NodeId,
        centre: Point,
    ) -> Option<&'p Page> {
        if let Some(owner) = self.owners.get(&id).or_else(|| self.placements.get(&id)) {
            return pages.iter().find(|p| p.id == *owner);
        }
        let mut output = (0, None);
        let mut drawing = (0, None);
        for page in pages
            .iter()
            .filter(|p| p.kind != PageKind::Sketch && p.contains(centre))
        {
            let slot = if matches!(page.kind, PageKind::Output { .. }) {
                &mut output
            } else {
                &mut drawing
            };
            *slot = (slot.0 + 1, Some(page));
        }
        match (output, drawing) {
            ((1, page), _) | ((0, _), (1, page)) => page,
            _ => None,
        }
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

/// What every page of `pages` owns by centre (ADR 0051 §4): the leaves of each top-level object
/// of `document` (a leaf, or a whole group, layers transparent) whose outline-box centre the page
/// holds, read from `leaves` (every leaf, hidden, locked, guides and dimensions included). Placed
/// pages are left out: they follow their laser page's frame (ADR 0042 §4).
#[must_use]
pub fn page_contents(
    document: &Document,
    pages: &[Page],
    membership: &PageMembership,
    leaves: &[ObjectSnapshot],
) -> PageContents {
    contents_by(document, pages, membership, leaves, &mut |leaf| {
        Some(object_outline_bounds(leaf))
    })
    .unwrap_or_default()
}

/// [`page_contents`] measuring a variable-width stroke by the box of its visible outline from
/// `artwork`; `None` while `artwork` has no outline for one of them.
#[must_use]
pub fn page_contents_with_artwork(
    document: &Document,
    pages: &[Page],
    membership: &PageMembership,
    leaves: &[ObjectSnapshot],
    artwork: &VariableArtworkLookup<'_>,
) -> Option<PageContents> {
    contents_by(document, pages, membership, leaves, &mut |leaf| {
        object_outline_bounds_with_artwork(leaf, artwork)
    })
}

/// The ownership read behind [`page_contents`], with each leaf's box from `bounds`; `None` as soon
/// as `bounds` has none for a leaf.
fn contents_by(
    document: &Document,
    pages: &[Page],
    membership: &PageMembership,
    leaves: &[ObjectSnapshot],
    bounds: &mut dyn FnMut(&ObjectSnapshot) -> Option<(Point, Point)>,
) -> Option<PageContents> {
    let units = Units::build(document, None);
    let mut order: Vec<NodeId> = Vec::new();
    let mut boxes: HashMap<NodeId, ((Point, Point), Vec<NodeId>)> = HashMap::new();
    for leaf in leaves {
        let id = leaf.id();
        if membership.placements.contains_key(&id) {
            continue;
        }
        let unit = if membership.is_fixed(id) {
            id
        } else {
            units.unit_of(id)
        };
        let (min, max) = bounds(leaf)?;
        let entry = boxes.entry(unit).or_insert_with(|| {
            order.push(unit);
            ((min, max), Vec::new())
        });
        let (lo, hi) = &mut entry.0;
        *lo = Point::new(lo.x.min(min.x), lo.y.min(min.y));
        *hi = Point::new(hi.x.max(max.x), hi.y.max(max.y));
        entry.1.push(id);
    }
    let mut contents = PageContents::new();
    for unit in order {
        let Some(((min, max), ids)) = boxes.remove(&unit) else {
            continue;
        };
        let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        if let Some(page) = membership.owner_by_centre(pages, unit, centre) {
            contents.entry(page.id.clone()).or_default().extend(ids);
        }
    }
    Some(contents)
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
        for reach in [Reach::Touches, Reach::SharesArea] {
            assert_eq!(read().pages_of(&pages, rect(), inside, reach).count(), 0);
        }
    }

    #[test]
    fn a_sketch_across_a_drawing_page_edge_is_a_member_of_its_sketch_page_only() {
        let pages = pages();
        let across = (Point::new(60.0, 20.0), Point::new(250.0, 40.0));
        for reach in [Reach::Touches, Reach::SharesArea] {
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
    }

    #[test]
    fn a_placed_page_is_a_member_of_its_laser_page_only() {
        let kind = PageKind::Output {
            manufacturing: node(9),
            process: "curvyo.laser".to_owned(),
        };
        let laser = page("laser", 300.0, kind);
        let pages = vec![pages()[0].clone(), laser];
        let placed = node(3);
        let membership = read().with_placements([(placed, "laser".to_owned())]);
        let over_drawing = (Point::new(10.0, 10.0), Point::new(20.0, 20.0));
        let ids: Vec<_> = membership
            .pages_of(&pages, placed, over_drawing, Reach::Touches)
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(ids, ["laser"]);
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

    fn laser(id: &str, x: f64) -> Page {
        page(
            id,
            x,
            PageKind::Output {
                manufacturing: node(9),
                process: "curvyo.laser".to_owned(),
            },
        )
    }

    fn owner(pages: &[Page], id: NodeId, x: f64, y: f64) -> Option<String> {
        read()
            .owner_by_centre(pages, id, Point::new(x, y))
            .map(|p| p.id.clone())
    }

    #[test]
    fn the_centre_owner_is_the_page_holding_the_centre_and_an_edge_counts() {
        let pages = pages();
        assert_eq!(
            owner(&pages, rect(), 50.0, 50.0).as_deref(),
            Some("drawing")
        );
        assert_eq!(
            owner(&pages, rect(), 100.0, 50.0).as_deref(),
            Some("drawing")
        );
        assert_eq!(
            owner(&pages, rect(), 50.0, 100.0 + 5e-10).as_deref(),
            Some("drawing")
        );
        assert_eq!(owner(&pages, rect(), 50.0, 150.0), None);
    }

    #[test]
    fn a_centre_over_a_sketch_page_has_no_owner_and_the_sketch_belongs_to_its_page() {
        let pages = pages();
        assert_eq!(owner(&pages, rect(), 150.0, 50.0), None);
        assert_eq!(
            owner(&pages, sketch(), -500.0, 50.0).as_deref(),
            Some("sketch")
        );
    }

    #[test]
    fn a_laser_page_wins_over_a_drawing_page_and_two_of_a_kind_own_nothing() {
        let both = vec![
            page("drawing", 0.0, PageKind::Drawing),
            laser("laser", 50.0),
        ];
        assert_eq!(owner(&both, rect(), 75.0, 50.0).as_deref(), Some("laser"));
        let drawings = vec![
            page("a", 0.0, PageKind::Drawing),
            page("b", 50.0, PageKind::Drawing),
        ];
        assert_eq!(owner(&drawings, rect(), 75.0, 50.0), None);
        let lasers = vec![laser("a", 0.0), laser("b", 50.0)];
        assert_eq!(owner(&lasers, rect(), 75.0, 50.0), None);
    }
}

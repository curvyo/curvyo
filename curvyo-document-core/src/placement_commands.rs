//! The commands that write placed pages (ADR 0045 §7, `specs/0100-bring-content-in` criteria
//! 13, 17 to 19 and 24): placing a drawing page on a laser page, transforming and copying a
//! placement on its own laser page, and removing the placements of a deleted page.

use loro::TreeParentId;

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::manufacturing::ManufacturingError;
use crate::manufacturing_codec::{PlacementLink, SHAPE_PLACEMENT, write_placement_link};
use crate::pages::Page;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;
use crate::units::{Point, Vec2};

/// The label of placing a drawing page on a laser page (criteria 13 and 15).
pub const PLACE_PAGE_COMMIT_LABEL: &str = "place_page";

/// The smallest overlap, in mm², of a placed page's box with its laser page that keeps it on the
/// page: an edge-only touch is off the page (criterion 18).
pub const PLACED_PAGE_MIN_OVERLAP_MM2: f64 = 1e-9;

/// One placement with the pages it joins: where it is and where it may go.
#[derive(Debug, Clone)]
pub struct PlacedPage {
    /// The placement node.
    pub id: NodeId,
    /// The laser page that owns it.
    pub output: Page,
    /// The drawing page it shows; `None` when that page is gone (it draws nothing).
    pub source: Option<Page>,
    /// Its stored matrix: source page-local mm to output page-local mm.
    pub matrix: Affine,
}

impl PlacedPage {
    /// The map from global source coordinates to global output coordinates (ADR 0032 §6):
    /// output origin + matrix × (point − source origin).
    #[must_use]
    pub fn global_map(&self) -> Option<Affine> {
        let source = self.source.as_ref()?;
        Some(global_map(&self.output, source, self.matrix))
    }

    /// The source frame's corners under [`PlacedPage::global_map`]: its box (criterion 17).
    #[must_use]
    pub fn frame_corners(&self) -> Option<[Point; 4]> {
        let source = self.source.as_ref()?;
        Some(frame_corners(&self.output, source, self.matrix))
    }
}

fn global_map(output: &Page, source: &Page, matrix: Affine) -> Affine {
    Affine::translation(Vec2::new(output.origin.x, output.origin.y))
        .compose(matrix)
        .compose(Affine::translation(Vec2::new(
            -source.origin.x,
            -source.origin.y,
        )))
}

fn frame_corners(output: &Page, source: &Page, matrix: Affine) -> [Point; 4] {
    let map = global_map(output, source, matrix);
    let (x, y) = (source.origin.x, source.origin.y);
    let (w, h) = (source.size.width.as_mm(), source.size.height.as_mm());
    [
        Point::new(x, y),
        Point::new(x + w, y),
        Point::new(x + w, y + h),
        Point::new(x, y + h),
    ]
    .map(|p| map.apply_point(p))
}

/// How far a point lies inside one side of a frame (negative: outside).
type HalfPlane = fn(Point, f64) -> f64;

/// Whether the quadrilateral `corners` (a source frame under its matrix, convex) overlaps the
/// frame of `page` by more than [`PLACED_PAGE_MIN_OVERLAP_MM2`]: the turned frame itself is
/// clipped to the page, not its bounding box (criterion 18), so an edge-only touch or a corner
/// that only the bounds reach is off the page.
#[must_use]
pub fn overlaps_page(corners: &[Point; 4], page: &Page) -> bool {
    let (x0, y0) = (page.origin.x, page.origin.y);
    let (x1, y1) = (x0 + page.size.width.as_mm(), y0 + page.size.height.as_mm());
    let mut polygon: Vec<Point> = corners.to_vec();
    // Sutherland-Hodgman against the four half-planes of the frame.
    let edges: [(HalfPlane, f64); 4] = [
        (|p, x| p.x - x, x0),
        (|p, x| x - p.x, x1),
        (|p, y| p.y - y, y0),
        (|p, y| y - p.y, y1),
    ];
    for (inside, limit) in edges {
        let mut clipped = Vec::with_capacity(polygon.len() + 1);
        for i in 0..polygon.len() {
            let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            let (da, db) = (inside(a, limit), inside(b, limit));
            if da >= 0.0 {
                clipped.push(a);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                clipped.push(Point::new(
                    (b.x - a.x).mul_add(t, a.x),
                    (b.y - a.y).mul_add(t, a.y),
                ));
            }
        }
        polygon = clipped;
        if polygon.len() < 3 {
            return false;
        }
    }
    let twice: f64 = (0..polygon.len())
        .map(|i| {
            let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
            a.x.mul_add(b.y, -(b.x * a.y))
        })
        .sum();
    twice.abs() / 2.0 > PLACED_PAGE_MIN_OVERLAP_MM2
}

impl Document {
    /// Every placement of every laser page, with its pages, in tree order.
    #[must_use]
    pub fn placed_pages(&self) -> Vec<PlacedPage> {
        let pages = self.pages();
        self.manufacturings()
            .into_iter()
            .filter_map(|m| Some((m.page.clone()?, m.placements)))
            .flat_map(|(output, placements)| {
                let pages = &pages;
                placements.into_iter().map(move |placement| PlacedPage {
                    id: placement.id,
                    output: output.clone(),
                    source: pages.iter().find(|p| p.id == placement.page).cloned(),
                    matrix: placement.matrix,
                })
            })
            .collect()
    }

    /// The placement `id`, `None` when it is not a live placement.
    #[must_use]
    pub fn placed_page(&self, id: NodeId) -> Option<PlacedPage> {
        self.placed_pages().into_iter().find(|p| p.id == id)
    }

    /// Places the drawing page `source` on the laser page `output` with `matrix` (source
    /// page-local to output page-local), above its earlier placements, in one commit labelled
    /// [`PLACE_PAGE_COMMIT_LABEL`] (ADR 0045 §7). Returns the placement.
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] when `output` is not a laser page or `source` not a
    /// drawing page; [`ManufacturingError::OffPage`] when the placed frame would not overlap
    /// the laser page; [`ManufacturingError::OutOfRange`] for a matrix past ±1e7. Nothing is
    /// written.
    pub fn place_page(
        &self,
        output: &str,
        source: &str,
        matrix: Affine,
    ) -> Result<NodeId, ManufacturingError> {
        let pages = self.pages();
        let out = pages
            .iter()
            .find(|p| p.id == output)
            .ok_or(ManufacturingError::NotFound)?;
        let manufacturing = out
            .kind
            .manufacturing()
            .ok_or(ManufacturingError::NotFound)?;
        let src = pages
            .iter()
            .find(|p| p.id == source && p.kind == crate::PageKind::Drawing)
            .ok_or(ManufacturingError::NotFound)?;
        if !matrix.is_within_limit() {
            return Err(ManufacturingError::OutOfRange);
        }
        if !overlaps_page(&frame_corners(out, src, matrix), out) {
            return Err(ManufacturingError::OffPage);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree
            .create(TreeParentId::Node(tree_id_of(manufacturing)))
            .map_err(|_| ManufacturingError::NotFound)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NotFound)?;
        meta.insert(KEY_SHAPE, SHAPE_PLACEMENT)
            .map_err(|_| ManufacturingError::NotFound)?;
        write_placement_link(
            &meta,
            &PlacementLink {
                page: source.to_owned(),
                matrix,
            },
        );
        self.commit_with_label(PLACE_PAGE_COMMIT_LABEL);
        Ok(NodeId::from_parts(node.peer, node.counter))
    }

    /// Maps each placement of `moves` by its global map T (a move, turn, scale, skew or flip in
    /// document space), writing only its `placement_link` whole (T conjugated into page-local
    /// coordinates), in one commit labelled `label` (ADR 0045 §7, criterion 17).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an id that is not a live placement of a live page;
    /// [`ManufacturingError::OffPage`] when a result would not overlap its own laser page
    /// (criterion 18); [`ManufacturingError::OutOfRange`] for a matrix past ±1e7 or not finite.
    /// The whole command is refused; nothing is written.
    pub fn transform_placements(
        &self,
        moves: &[(NodeId, Affine)],
        label: &str,
    ) -> Result<(), ManufacturingError> {
        let placed = self.placed_pages();
        let mut planned = Vec::with_capacity(moves.len());
        for &(id, t) in moves {
            let p = placed
                .iter()
                .find(|p| p.id == id)
                .ok_or(ManufacturingError::NotFound)?;
            let source = p.source.as_ref().ok_or(ManufacturingError::NotFound)?;
            let to_page = Affine::translation(Vec2::new(p.output.origin.x, p.output.origin.y));
            let from_page = Affine::translation(Vec2::new(-p.output.origin.x, -p.output.origin.y));
            let matrix = from_page.compose(t).compose(to_page).compose(p.matrix);
            if !matrix.is_finite() || !matrix.is_within_limit() {
                return Err(ManufacturingError::OutOfRange);
            }
            if !overlaps_page(&frame_corners(&p.output, source, matrix), &p.output) {
                return Err(ManufacturingError::OffPage);
            }
            planned.push((id, source.id.clone(), matrix));
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for (id, page, matrix) in planned {
            let meta = tree
                .get_meta(tree_id_of(id))
                .map_err(|_| ManufacturingError::NotFound)?;
            write_placement_link(&meta, &PlacementLink { page, matrix });
        }
        self.commit_with_label(label);
        Ok(())
    }

    /// Copies each placement of `ids` onto its own laser page with its matrix, moved by `offset`
    /// in document space, above the placements there, in one commit labelled `label` (Duplicate
    /// and the Ctrl copy of a move, criterion 19). Returns the copies in the order of `ids`.
    ///
    /// # Errors
    /// As [`Document::transform_placements`]; nothing is written.
    pub fn duplicate_placements(
        &self,
        ids: &[NodeId],
        offset: Vec2,
        label: &str,
    ) -> Result<Vec<NodeId>, ManufacturingError> {
        let placed = self.placed_pages();
        let t = Affine::translation(offset);
        let mut planned = Vec::with_capacity(ids.len());
        for &id in ids {
            let p = placed
                .iter()
                .find(|p| p.id == id)
                .ok_or(ManufacturingError::NotFound)?;
            let source = p.source.as_ref().ok_or(ManufacturingError::NotFound)?;
            let matrix = t.compose(p.matrix);
            if !matrix.is_within_limit() {
                return Err(ManufacturingError::OutOfRange);
            }
            if !overlaps_page(&frame_corners(&p.output, source, matrix), &p.output) {
                return Err(ManufacturingError::OffPage);
            }
            let parent = p
                .output
                .kind
                .manufacturing()
                .ok_or(ManufacturingError::NotFound)?;
            planned.push((parent, source.id.clone(), matrix));
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut made = Vec::with_capacity(planned.len());
        for (parent, page, matrix) in planned {
            let node = tree
                .create(TreeParentId::Node(tree_id_of(parent)))
                .map_err(|_| ManufacturingError::NotFound)?;
            let meta = tree
                .get_meta(node)
                .map_err(|_| ManufacturingError::NotFound)?;
            meta.insert(KEY_SHAPE, SHAPE_PLACEMENT)
                .map_err(|_| ManufacturingError::NotFound)?;
            write_placement_link(&meta, &PlacementLink { page, matrix });
            made.push(NodeId::from_parts(node.peer, node.counter));
        }
        self.commit_with_label(label);
        Ok(made)
    }
}

//! What output pages draw in the plane (`0093` criteria 6, 8, 9, 10; `0098` criteria 10, 12).

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, ManufacturingSnapshot, NodeId, ObjectSnapshot, Page, Point, Vec2, ViewTransform,
};
use curvyo_render_core::{DrawList, GridStroke, RgbaColor, build_artwork};

use super::super::Session;

/// Below this the linear part of a placement matrix is not a uniform scale (tolerance of the
/// tessellation scale only; the vertices are mapped exactly).
const UNIFORM_EPS: f64 = 1e-9;

/// The member indexes of one placement, for the drawing read and page they were computed from.
#[derive(Debug, Default)]
pub(in crate::session) struct PlacedCache(RefCell<Vec<PlacedEntry>>);

#[derive(Debug)]
struct PlacedEntry {
    drawn: Arc<[ObjectSnapshot]>,
    page: Page,
    members: Arc<[usize]>,
}

impl Session {
    /// The visible fixtures of every output page, added to that page's entry of `grids` (one
    /// entry per page of `pages`): they are painted with the page's grid, under everything
    /// placed or drawn on it. Lines one device pixel wide in the grid ink.
    pub(in crate::session) fn add_output_fixtures(
        &self,
        pages: &[Page],
        grids: &mut Vec<DrawList>,
    ) {
        let view = self.view();
        let ink = curvyo_render_core::grid_ink(self.shown_background());
        for manufacturing in &self.document.manufacturings() {
            let Some(page) = manufacturing.page.as_ref() else {
                continue;
            };
            let Some(index) = pages.iter().position(|shown| shown.id == page.id) else {
                continue;
            };
            let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
            for fixture in manufacturing.fixtures.iter().filter(|f| f.visible) {
                let mut drawing =
                    curvyo_ui_core::fixture_drawing(&fixture.kind, &fixture.params, page.size);
                for line in &mut drawing.lines {
                    line.start = to_plane.apply_point(line.start);
                    line.end = to_plane.apply_point(line.end);
                }
                if grids.len() < pages.len() {
                    grids.resize_with(pages.len(), DrawList::default);
                }
                grids[index].extend(fixture_lines(&drawing, ink, view, self.device_pixel_ratio));
            }
        }
    }

    /// Each placement's drawing page on its output page, under the drawing (ADR 0042 §4,
    /// `adrs.md` decision 2), through one composed matrix: the drawing's resolved outlines
    /// reused, never copied. Artwork layers only, so the drawing after it keeps its layers and
    /// images. Drawn in every workbench, in global coordinates.
    pub(in crate::session) fn output_pages_frame(&self) -> DrawList {
        let mut list = DrawList::default();
        let manufacturings = self.document.manufacturings();
        if manufacturings.is_empty() {
            return list;
        }
        let view = self.view();
        let read = self.document_read();
        for manufacturing in &manufacturings {
            let Some(page) = manufacturing.page.as_ref() else {
                continue;
            };
            let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
            for (_, map, members) in self.placements_of(manufacturing) {
                let map = to_plane.compose(map);
                let objects: Vec<ObjectSnapshot> = members
                    .iter()
                    .map(|&index| read.drawn[index].clone())
                    .collect();
                let tessellation =
                    ViewTransform::new(view.scale() * linear_scale(map), Point::new(0.0, 0.0));
                let mut art = build_artwork(&objects, tessellation);
                art.map_positions(|p| map.apply_point(p));
                list.extend(art);
            }
        }
        list
    }

    /// The marks over every output page: a selected placement's box and the machine origin
    /// marker (view only, never artwork).
    pub(in crate::session) fn output_pages_overlay(&self) -> DrawList {
        let mut list = DrawList::default();
        let manufacturings = self.document.manufacturings();
        if manufacturings.is_empty() {
            return list;
        }
        let view = self.view();
        let read = self.document_read();
        let selected = self.selected_placement();
        for manufacturing in &manufacturings {
            let Some(page) = manufacturing.page.as_ref() else {
                continue;
            };
            let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
            for (id, map, members) in self.placements_of(manufacturing) {
                if selected == Some(id)
                    && let Some((min, max)) =
                        curvyo_ui_core::placed_bounds(&read.drawn, &members, to_plane.compose(map))
                {
                    let corners = [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)];
                    list.extend(curvyo_render_core::build_offset_grown_box(corners, view));
                }
            }
            if self.process_enabled(&manufacturing.process)
                && let Some(copy) = manufacturing.profile.as_ref()
            {
                let read = curvyo_laser_core::read_copy(Some(copy), None);
                list.extend(origin_marker(read.copy.values.origin, page, view));
            }
        }
        list
    }

    /// The placement whose placed content lies under `point`, the topmost first.
    pub(in crate::session) fn placement_at(&self, point: Point) -> Option<NodeId> {
        let read = self.document_read();
        self.document
            .manufacturings()
            .iter()
            .rev()
            .find_map(|manufacturing| {
                let page = manufacturing.page.as_ref()?;
                let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
                self.placements_of(manufacturing)
                    .into_iter()
                    .rev()
                    .find(|(_, map, members)| {
                        curvyo_ui_core::placed_bounds(&read.drawn, members, to_plane.compose(*map))
                            .is_some_and(|(min, max)| {
                                (min.x..=max.x).contains(&point.x)
                                    && (min.y..=max.y).contains(&point.y)
                            })
                    })
                    .map(|(id, _, _)| id)
            })
    }

    /// The union of the placed content's bounds on the output page, page-local (the overhang
    /// notice of the Engraved grid).
    pub(in crate::session) fn placed_bounds_of(
        &self,
        manufacturing: &ManufacturingSnapshot,
    ) -> Option<(Point, Point)> {
        let read = self.document_read();
        self.placements_of(manufacturing)
            .into_iter()
            .filter_map(|(_, map, members)| {
                curvyo_ui_core::placed_bounds(&read.drawn, &members, map)
            })
            .reduce(|(a, b), (c, d)| {
                (
                    Point::new(a.x.min(c.x), a.y.min(c.y)),
                    Point::new(b.x.max(d.x), b.y.max(d.y)),
                )
            })
    }

    /// Each placement whose drawing page exists: its id, the map to the output page's local mm
    /// and the indexes of its members in the drawn leaves. A placement of a deleted page draws
    /// nothing (ADR 0032 §7).
    fn placements_of(
        &self,
        manufacturing: &ManufacturingSnapshot,
    ) -> Vec<(NodeId, Affine, Arc<[usize]>)> {
        let pages = self.document.pages();
        let read = self.document_read();
        manufacturing
            .placements
            .iter()
            .filter_map(|placement| {
                let source = pages.iter().find(|page| page.id == placement.page)?.clone();
                let members = self.placed_members(&read.drawn, &read.guides, &source);
                let map = curvyo_ui_core::placement_map(&source, placement.matrix);
                Some((placement.id, map, members))
            })
            .collect()
    }

    fn placed_members(
        &self,
        drawn: &Arc<[ObjectSnapshot]>,
        guides: &std::collections::HashSet<curvyo_document_core::NodeId>,
        page: &Page,
    ) -> Arc<[usize]> {
        let cache = &self.manufacturing.placed.0;
        if let Some(entry) = cache
            .borrow()
            .iter()
            .find(|entry| Arc::ptr_eq(&entry.drawn, drawn) && entry.page == *page)
        {
            return Arc::clone(&entry.members);
        }
        let membership = self.page_membership();
        let members: Arc<[usize]> =
            curvyo_ui_core::placement_members(drawn, page, guides, &membership).into();
        let mut entries = cache.borrow_mut();
        entries.retain(|entry| Arc::ptr_eq(&entry.drawn, drawn));
        entries.push(PlacedEntry {
            drawn: Arc::clone(drawn),
            page: page.clone(),
            members: Arc::clone(&members),
        });
        members
    }
}

/// The machine origin marker at the copy's corner of the output page (`0026` criterion 11):
/// drawn above the placed content and the drawing, view only.
fn origin_marker(
    corner: curvyo_laser_core::OriginCorner,
    page: &Page,
    view: ViewTransform,
) -> DrawList {
    let (width, height) = (page.size.width.as_mm(), page.size.height.as_mm());
    let x = page.origin.x + if corner.is_right() { width } else { 0.0 };
    let y = page.origin.y + if corner.is_bottom() { height } else { 0.0 };
    let inward = (
        if corner.is_right() { -1.0 } else { 1.0 },
        if corner.is_bottom() { -1.0 } else { 1.0 },
    );
    curvyo_render_core::build_origin_marker(Point::new(x, y), inward, width.min(height), view)
}

/// How much a placement map scales lengths (for the tessellation tolerance only): the square
/// root of its determinant's magnitude, 1 for a degenerate map.
fn linear_scale(map: Affine) -> f64 {
    let det = map.a.mul_add(map.d, -(map.b * map.c)).abs();
    if det > UNIFORM_EPS { det.sqrt() } else { 1.0 }
}

/// The lines of one fixture, each one device pixel wide in the page's grid ink: a reference on
/// the bed, not artwork (its stored line width is the engraving's, used by later job slices).
fn fixture_lines(
    drawing: &curvyo_extension_core::FixtureDrawing,
    ink: RgbaColor,
    view: ViewTransform,
    device_pixel_ratio: f64,
) -> DrawList {
    let strokes: Vec<GridStroke> = drawing
        .lines
        .iter()
        .map(|line| GridStroke {
            start: line.start,
            end: line.end,
            emphasis: 1.0,
            alpha: 1.0,
        })
        .collect();
    curvyo_render_core::build_grid_lines(&strokes, ink, view, device_pixel_ratio)
}

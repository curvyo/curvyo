//! The frame of a manufacturing workbench (criteria 4, 6, 8, 9, 10): the output page, its visible
//! fixtures below, then each placement's drawing page through one composed matrix, the drawing's
//! resolved outlines reused and never copied.

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, ManufacturingSnapshot, ObjectSnapshot, Page, PageGrid, Point, ViewTransform,
};
use curvyo_render_core::{
    DrawList, GridStroke, RgbaColor, build_artwork, build_artwork_with_looks, build_page_areas,
};

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
    /// The frame of the active manufacturing workbench; an empty list when it is gone.
    pub(in crate::session) fn manufacturing_frame(&self) -> DrawList {
        let Some(manufacturing) = self.active_manufacturing() else {
            return DrawList::default();
        };
        let view = self.view();
        let area = manufacturing.work_area();
        let output = Page {
            id: String::new(),
            name: manufacturing.name.clone(),
            origin: Point::new(0.0, 0.0),
            size: area,
            grid: PageGrid::DEFAULT,
        };
        let background = self.shown_background();
        let mut list = build_page_areas(
            std::slice::from_ref(&output),
            background,
            view,
            self.device_pixel_ratio,
        );
        let ink = curvyo_render_core::grid_ink(background);
        for fixture in manufacturing.fixtures.iter().filter(|f| f.visible) {
            let drawing = curvyo_ui_core::fixture_drawing(&fixture.kind, &fixture.params, area);
            list.extend(fixture_lines(&drawing, ink, view, self.device_pixel_ratio));
        }
        let read = self.document_read();
        for (_, map, members) in self.placements_of(&manufacturing) {
            let objects: Vec<ObjectSnapshot> = members
                .iter()
                .map(|&index| read.drawn[index].clone())
                .collect();
            let tessellation =
                ViewTransform::new(view.scale() * linear_scale(map), Point::new(0.0, 0.0));
            // Role view draws each leaf by its layer's role (`0027` criterion 6), else as drawn.
            let mut art = match self.role_looks(&manufacturing, &objects) {
                Some(looks) => build_artwork_with_looks(&objects, &looks, tessellation),
                None => build_artwork(&objects, tessellation),
            };
            art.map_positions(|p| map.apply_point(p));
            list.extend(art);
        }
        if let Some(active) = self.active_copy() {
            list.extend(origin_marker(active.read.copy.values.origin, area, view));
        }
        list
    }

    /// The union of the placed content's bounds on the output page (criterion 8).
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

    /// Each placement whose drawing page exists: the page, the map to the output page and the
    /// indexes of its members in the drawn leaves. A placement of a deleted page draws nothing
    /// (ADR 0032 §7).
    fn placements_of(
        &self,
        manufacturing: &ManufacturingSnapshot,
    ) -> Vec<(Page, Affine, Arc<[usize]>)> {
        let pages = self.document.pages();
        let read = self.document_read();
        manufacturing
            .placements
            .iter()
            .filter_map(|placement| {
                let source = pages.iter().find(|page| page.id == placement.page)?.clone();
                let members = self.placed_members(&read.drawn, &read.guides, &source);
                let map = curvyo_ui_core::placement_map(&source, placement.matrix);
                Some((source, map, members))
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
        let members: Arc<[usize]> = curvyo_ui_core::placement_members(drawn, page, guides).into();
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
/// drawn above the placed content, view only.
fn origin_marker(
    corner: curvyo_laser_core::OriginCorner,
    area: curvyo_document_core::DocumentSize,
    view: ViewTransform,
) -> DrawList {
    let (width, height) = (area.width.as_mm(), area.height.as_mm());
    let x = if corner.is_right() { width } else { 0.0 };
    let y = if corner.is_bottom() { height } else { 0.0 };
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

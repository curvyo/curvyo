//! What output pages draw in the plane (`0093` criteria 6, 8, 9, 10; `0098` criteria 10, 12).

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, ManufacturingSnapshot, NodeId, ObjectSnapshot, Page, Point, Vec2, ViewTransform,
};
use curvyo_render_core::{DrawList, GridStroke, build_artwork};

use super::super::Session;

/// The alpha of a placed page drawn off its laser page during a drag, in percent (UX notes §5).
const OFF_PAGE_PERCENT: u16 = 40;

/// The alpha of a placed page's frame mark (UX notes §5: 65 %).
const FRAME_MARK_ALPHA: u8 = 166;

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
        let live = self.placed_live();
        let looks = self.operation_look_table();
        for manufacturing in &manufacturings {
            let Some(page) = manufacturing.page.as_ref() else {
                continue;
            };
            let to_plane = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
            for (id, map, members) in self.placements_of(manufacturing) {
                let objects: Vec<ObjectSnapshot> = members
                    .iter()
                    .map(|&index| read.drawn[index].clone())
                    .collect();
                // The members of this placed page in their looks (`0101` criteria 11, 14, 15, 18).
                let objects = match looks
                    .as_ref()
                    .filter(|t| t.manufacturing == manufacturing.id)
                {
                    Some(table) => self.looked_placed_members(table, id, &objects, view),
                    None => objects,
                };
                // A placed page in a Select drag is drawn where a release puts it, a copy beside
                // the original; off its laser page at 40 % (`specs/0100` UX notes §5).
                let mut drawn = vec![(Affine::IDENTITY, false)];
                if let Some(&(t, copy, off)) =
                    live.iter().find(|(lid, ..)| *lid == id).map(|(_, l)| l)
                {
                    if copy {
                        drawn.push((t, off));
                    } else {
                        drawn = vec![(t, off)];
                    }
                }
                for (t, faint) in drawn {
                    let map = t.compose(to_plane.compose(map));
                    let tessellation =
                        ViewTransform::new(view.scale() * linear_scale(map), Point::new(0.0, 0.0));
                    let mut art = build_artwork(&objects, tessellation);
                    art.map_positions(|p| map.apply_point(p));
                    if faint {
                        for vertex in &mut art.triangles {
                            // 40 % of an alpha in 0..=255, integer arithmetic.
                            vertex.color.a =
                                u8::try_from(u16::from(vertex.color.a) * OFF_PAGE_PERCENT / 100)
                                    .unwrap_or(u8::MAX);
                        }
                    }
                    list.extend(art);
                }
            }
        }
        list
    }

    /// The live map of each placed page in the Select edit in flight: the map from where it is to
    /// where a release puts it, whether that is a copy, and whether it would be off its page.
    pub(in crate::session) fn placed_live(&self) -> Vec<(NodeId, (Affine, bool, bool))> {
        let objects = self.objects();
        let Some(live) = self.select_live_edit_in(&objects) else {
            return Vec::new();
        };
        live.objects
            .iter()
            .filter_map(|moved| {
                let ObjectSnapshot::Image(after) = moved else {
                    return None;
                };
                let placed = self.document.placed_page(after.id)?;
                let Some(ObjectSnapshot::Image(before)) =
                    objects.iter().find(|o| o.id() == after.id)
                else {
                    return None;
                };
                let t = after.placement.compose(before.placement.inverse()?);
                let corners = [
                    Point::new(0.0, 0.0),
                    Point::new(1.0, 0.0),
                    Point::new(1.0, 1.0),
                    Point::new(0.0, 1.0),
                ]
                .map(|p| after.placement.apply_point(p));
                let off = !curvyo_document_core::overlaps_page(&corners, &placed.output);
                Some((after.id, (t, live.copy, off)))
            })
            .collect()
    }

    /// The frame mark of every placed page (UX notes §5): its source frame under its matrix, one
    /// device pixel wide in the toolbar ink at 65 %; a mark, never output.
    pub(in crate::session) fn placed_frame_marks(&self) -> DrawList {
        let placed = self.document.placed_pages();
        if placed.is_empty() {
            return DrawList::default();
        }
        let mut ink = curvyo_render_core::grid_ink(self.shown_background());
        ink.a = FRAME_MARK_ALPHA;
        let strokes: Vec<GridStroke> = placed
            .iter()
            .filter_map(|p| self.placed_frame(p.id))
            .flat_map(|c| {
                (0..4).map(move |i| GridStroke {
                    start: c[i],
                    end: c[(i + 1) % 4],
                    emphasis: 1.0,
                    alpha: 1.0,
                })
            })
            .collect();
        curvyo_render_core::build_grid_lines(&strokes, ink, self.view(), self.device_pixel_ratio)
    }

    /// The marks over every output page: a selected placement's box and the machine origin
    /// marker (view only, never artwork).
    pub(in crate::session) fn output_pages_overlay(&self) -> DrawList {
        let mut list = self.placed_frame_marks();
        let manufacturings = self.document.manufacturings();
        if manufacturings.is_empty() {
            return list;
        }
        let view = self.view();
        for manufacturing in &manufacturings {
            let Some(page) = manufacturing.page.as_ref() else {
                continue;
            };
            if self.process_enabled(&manufacturing.process)
                && let Some(copy) = manufacturing.profile.as_ref()
            {
                let read = curvyo_laser_core::read_copy(Some(copy), None);
                list.extend(origin_marker(read.copy.values.origin, page, view));
            }
        }
        list
    }

    /// The frame corners of placed page `id` as drawn now (with a drag in flight).
    pub(in crate::session) fn placed_frame(&self, id: NodeId) -> Option<[Point; 4]> {
        self.document.placed_page(id)?.frame_corners()
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

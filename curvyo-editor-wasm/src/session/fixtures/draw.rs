//! What the fixtures draw (`specs/0102-laser-fixtures` criteria 5 to 7, 16 to 18, 20 to 23, 35 d
//! and 37; UX notes §5 and §6): each laser page's fixtures as one look for `curvyo-render-core`,
//! the selection box and cues over everything, and the grid layout cached by its inputs.

use std::cell::RefCell;
use std::sync::Arc;

use curvyo_document_core::{DocumentSize, Point};
use curvyo_laser_core::{GridLayout, GridSpec, Slot, TapeEdge, layout};
use curvyo_render_core::{
    BoardLook, DrawList, FixtureLook, GridLook, ScrewLook, SlotLook, StopLook, TapeLook, TickLook,
    TransformDecorationInput,
};

use super::super::Session;
use super::fit::outline_of;
use super::handles::handle_glyphs;
use super::placed::{PageFixtures, PlacedFixture, Shape};
use super::slots::slot_outside;

/// Minor lines fade in between these spacings, in device px (UX notes §5.1).
pub(super) const FADE_PX: (f64, f64) = (4.0, 12.0);
/// The width of a tape's band, in mm (criterion 17).
const BAND_MM: f64 = 5.0;

/// The grid's layout for the inputs it was built from: a frame with no change does no work.
#[derive(Debug, Default)]
pub(super) struct GridCache(RefCell<Option<(GridSpec, DocumentSize, Arc<GridLayout>)>>);

impl GridCache {
    /// The layout of `spec` over `area`, built on a miss.
    pub(super) fn layout(&self, spec: GridSpec, area: DocumentSize) -> Arc<GridLayout> {
        if let Some((s, a, cached)) = self.0.borrow().as_ref()
            && *s == spec
            && *a == area
        {
            return Arc::clone(cached);
        }
        let built = Arc::new(layout(spec, area));
        *self.0.borrow_mut() = Some((spec, area, Arc::clone(&built)));
        built
    }
}

/// The four corners of the rectangle `x0, y0` to `x1, y1` moved by `o`.
fn rect(o: Point, (x0, y0): (f64, f64), (x1, y1): (f64, f64)) -> [Point; 4] {
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].map(|(x, y)| Point::new(o.x + x, o.y + y))
}

/// The band and inner edge of the tape on `edge` of a `w` by `h` work area at `o`.
fn band(edge: TapeEdge, o: Point, w: f64, h: f64) -> ([Point; 4], (Point, Point)) {
    let b = BAND_MM;
    let (corners, inner) = match edge {
        TapeEdge::Bottom => (rect(o, (0.0, h - b), (w, h)), ((0.0, h - b), (w, h - b))),
        TapeEdge::Top => (rect(o, (0.0, 0.0), (w, b)), ((0.0, b), (w, b))),
        TapeEdge::Left => (rect(o, (0.0, 0.0), (b, h)), ((b, 0.0), (b, h))),
        TapeEdge::Right => (rect(o, (w - b, 0.0), (w, h)), ((w - b, 0.0), (w - b, h))),
    };
    let at = |(x, y): (f64, f64)| Point::new(o.x + x, o.y + y);
    (corners, (at(inner.0), at(inner.1)))
}

impl Session {
    /// The look of `fixtures`' Active, shown fixtures, a fixture being dragged at its live place.
    pub(super) fn fixture_look(&self, fixtures: &PageFixtures) -> FixtureLook {
        let mut look = FixtureLook::default();
        let page = &fixtures.page;
        for fixture in fixtures.fixtures.iter().filter(|f| f.shown()) {
            let fixture = self
                .fixture_live_edit(fixture.id)
                .map_or_else(|| fixture.clone(), |edit| fixture.with_edit(page, edit));
            let corners = fixture.corners(page);
            match &fixture.shape {
                Shape::Grid(spec) => {
                    look.grid = Some(self.grid_look(*spec, page.size, page.origin));
                }
                Shape::Board { width, height } => look.boards.push(BoardLook {
                    corners,
                    slots: board_slots(&fixture, *width, *height),
                }),
                Shape::Clamp { .. } => look.clamps.push(corners),
                Shape::Stop {
                    leg_a,
                    leg_b,
                    thickness,
                } => look.stops.push(stop_look(
                    &fixture,
                    leg_a.as_mm(),
                    leg_b.as_mm(),
                    thickness.as_mm(),
                )),
                Shape::Screw { thread, head } => look.screws.push(ScrewLook {
                    centre: fixture.global.apply_point(Point::new(0.0, 0.0)),
                    head_mm: head.as_mm(),
                    thread_mm: thread.as_mm(),
                }),
                Shape::Unknown => {}
            }
        }
        look
    }

    /// The grid's lines and tapes in document space, with the fade of UX notes §5.1.
    fn grid_look(&self, spec: GridSpec, area: DocumentSize, origin: Point) -> GridLook {
        let grid = self.fixtures.grid_cache.layout(spec, area);
        let device = self.view().scale() * self.device_pixel_ratio.max(f64::MIN_POSITIVE);
        let spacing_px = spec.spacing.as_mm() * device;
        let minor_alpha = ((spacing_px - FADE_PX.0) / (FADE_PX.1 - FADE_PX.0)).clamp(0.0, 1.0);
        let major_px = spacing_px * f64::from(spec.major_every.max(1));
        let shift = |p: Point| Point::new(origin.x + p.x, origin.y + p.y);
        let lines = |segments: &[curvyo_extension_core::LineSegment]| -> Vec<(Point, Point)> {
            segments
                .iter()
                .map(|s| (shift(s.start), shift(s.end)))
                .collect()
        };
        let short_ticks = 5.0 * device >= FADE_PX.0;
        let (w, h) = (area.width.as_mm(), area.height.as_mm());
        let tapes = grid
            .tapes
            .iter()
            .map(|tape| {
                let (band, inner_edge) = band(tape.edge, origin, w, h);
                let ticks = tape
                    .ticks
                    .iter()
                    .filter(|t| t.long || short_ticks)
                    .map(|t| TickLook {
                        start: shift(t.start),
                        end: shift(t.end),
                        long: t.long,
                    })
                    .collect();
                TapeLook {
                    band,
                    inner_edge,
                    ticks,
                }
            })
            .collect();
        GridLook {
            minor: if spacing_px >= FADE_PX.0 { lines(&grid.minor) } else { Vec::new() },
            major: if major_px >= FADE_PX.0 { lines(&grid.major) } else { Vec::new() },
            #[allow(clippy::cast_possible_truncation)] // 0..=1
            minor_alpha: minor_alpha as f32,
            tapes,
        }
    }

    /// The fixtures of every laser page drawn in its page grid's layer (criterion 7: under
    /// everything placed or drawn on the page), one list per page of `pages`.
    pub(in crate::session) fn fixture_page_lists(
        &self,
        pages: &[curvyo_document_core::Page],
    ) -> Vec<(usize, DrawList)> {
        let view = self.view();
        self.available_page_fixtures()
            .iter()
            .filter_map(|fixtures| {
                let index = pages.iter().position(|p| p.id == fixtures.page.id)?;
                let look = self.fixture_look(fixtures);
                Some((
                    index,
                    curvyo_render_core::build_fixture_look(&look, view, self.device_pixel_ratio),
                ))
            })
            .collect()
    }

    /// Puts each laser page's fixtures in front of its entry of `grids` (one entry per page of
    /// `pages`): the bed grid lies above the page's view grid of `0090`, and the layer keeps the
    /// first fragment (UX notes §5.1, §5.5).
    pub(in crate::session) fn add_fixture_lists(
        &self,
        pages: &[curvyo_document_core::Page],
        grids: &mut Vec<DrawList>,
    ) {
        // Only laser pages hold fixtures: a drawing without one reads no manufacturing.
        if !pages
            .iter()
            .any(|p| matches!(p.kind, curvyo_document_core::PageKind::Output { .. }))
        {
            return;
        }
        for (index, mut list) in self.fixture_page_lists(pages) {
            if grids.len() < pages.len() {
                grids.resize_with(pages.len(), DrawList::default);
            }
            list.triangles.append(&mut grids[index].triangles);
            grids[index] = list;
        }
    }

    /// Over everything: the selected fixture's box and the rectangle's handles it keeps, its
    /// selected slot, the lit slot of a fit and the turned ghost of a fitted move (UX notes §6.2,
    /// §7.2).
    pub(in crate::session) fn fixture_overlay(&self) -> DrawList {
        let view = self.view();
        let mut list = DrawList::default();
        if let Some((slot, fit)) = self.current_fit() {
            if slot.drawn {
                list.extend(curvyo_render_core::build_slot_cue(
                    &slot.outline,
                    true,
                    view,
                ));
            }
            if fit.turned
                && let Some(ghost) = self.fitted_objects()
            {
                list.extend(curvyo_render_core::build_drag_ghost(&ghost, view));
            }
        }
        let Some((fixtures, fixture)) =
            self.selected_fixture().and_then(|id| self.page_fixture(id))
        else {
            return list;
        };
        let page = &fixtures.page;
        let fixture = self
            .fixture_live_edit(fixture.id)
            .map_or_else(|| fixture.clone(), |edit| fixture.with_edit(page, edit));
        if let Some(ghost) = self.fixture_ghost() {
            list.extend(ghost);
        }
        list.extend(curvyo_render_core::build_fixture_box(
            fixture.corners(page),
            view,
        ));
        let active = self
            .fixture_drag_handle()
            .or_else(|| self.fixture_entry_handle());
        let input = TransformDecorationInput {
            handles: handle_glyphs(
                &fixture,
                page,
                &self.transform_handle_tolerances(),
                active,
                self.held.shift && self.fixtures.drag.is_none(),
            ),
            device_pixel_ratio: self.device_pixel_ratio,
            ..TransformDecorationInput::default()
        };
        list.extend(curvyo_render_core::build_transform_draw_list(view, &input));
        if let Some(slot) = self
            .selected_slot()
            .and_then(|id| fixture.slots.iter().find(|s| s.id == id))
        {
            let outline = outline_of(&slot.slot, fixture.global);
            list.extend(curvyo_render_core::build_slot_cue(&outline, false, view));
        }
        list
    }
}

/// A stop's two legs from its outer corner, `leg_a` along +x and `leg_b` along +y, `thickness`
/// wide, placed.
fn stop_look(stop: &PlacedFixture, leg_a: f64, leg_b: f64, thickness: f64) -> StopLook {
    let place = |corners: [Point; 4]| corners.map(|p| stop.global.apply_point(p));
    let origin = Point::new(0.0, 0.0);
    StopLook {
        legs: [
            place(rect(origin, (0.0, 0.0), (leg_a, thickness))),
            place(rect(origin, (0.0, thickness), (thickness, leg_b))),
        ],
    }
}

/// A holder's slots as drawn, an outside one dashed (criterion 20).
fn board_slots(
    holder: &PlacedFixture,
    width: curvyo_document_core::Length,
    height: curvyo_document_core::Length,
) -> Vec<SlotLook> {
    holder
        .slots
        .iter()
        .map(|s| SlotLook {
            outline: outline_of(&s.slot, holder.global),
            outside: slot_outside(&s.slot, width, height),
        })
        .collect()
}

/// Whether `slot` is large enough on screen for its name (UX notes §5.2: 40 device px).
pub(super) fn slot_label_fits(slot: &Slot, device_px_per_mm: f64) -> bool {
    let (w, h) = slot.extent();
    w.as_mm().min(h.as_mm()) * device_px_per_mm >= 40.0
}

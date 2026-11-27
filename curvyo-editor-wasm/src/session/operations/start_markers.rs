//! The start markers of the selected items' closed outlines (`specs/0139` criteria 19, 23;
//! `ux-notes.md` 5.2, Frage 415 C): which show, a drag along the path that never rests in a tab,
//! Delete back to Automatic, and their look.

use curvyo_document_core::{AUTOMATIC_START_COMMIT_LABEL, MOVE_START_POINT_COMMIT_LABEL, Point};
use curvyo_laser_core::tabs::{OutlineEntry, TabSpan};
use curvyo_render_core::{DrawList, RgbaColor, Vertex, thick_line};

use super::super::Session;
use super::tab_outlines::TabOutline;

/// `--accent`, the selection colour of every handle (as `print_and_cut_overlay.rs`).
const ACCENT: RgbaColor = RgbaColor::opaque(0x2F, 0x6F, 0xEE);
/// A path shorter than this on screen shows no marker (C5).
const MIN_PATH_PX: f64 = 48.0;
/// A press and release closer than this on screen is a click, not a drag.
const DEAD_ZONE_PX: f64 = 3.0;
/// A marker snaps to a node within this along the path (criterion 19).
const SNAP_PX: f64 = 8.0;
/// The triangle: 10 px from its base to its tip, 8 px across its base (UX 5.2).
const LENGTH_PX: f64 = 10.0;
const BASE_PX: f64 = 8.0;
const RING_PX: f64 = 1.5;

/// The outline whose start marker is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StartRef {
    pub(super) op: curvyo_document_core::NodeId,
    pub(super) key: String,
    pub(super) reference: String,
}

/// A start marker drag in flight.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StartDrag {
    pub(super) outline: TabOutline,
    pub(super) arc: f64,
    pub(super) snapped: bool,
    pub(super) limited: bool,
    pub(super) from: Point,
    pub(super) live: bool,
}

/// `arc` moved out of the tab of `spans` that covers it, to that tab's nearer edge, and whether
/// it moved (criterion 19: a start never rests inside a tab).
fn clear_of_tabs(outline: &TabOutline, spans: &[TabSpan], arc: f64) -> (f64, bool) {
    let m = &outline.measured;
    let length = m.length();
    for span in spans {
        let d = (arc - span.start()).rem_euclid(length);
        if d > 0.0 && d < span.width {
            let edge = if d < span.width / 2.0 {
                span.start()
            } else {
                span.end()
            };
            return (m.wrap(edge), true);
        }
    }
    (arc, false)
}

/// Where the burn of a set start at `arc` begins: the end of a tab that covers it.
fn burn_start(outline: &TabOutline, arc: f64) -> f64 {
    let m = &outline.measured;
    outline
        .spans
        .iter()
        .find(|span| {
            let d = (arc - span.start()).rem_euclid(m.length());
            d < span.width - 0.5
        })
        .map_or(arc, |span| m.wrap(span.end()))
}

impl Session {
    /// The start markers drawn now and whether each is set: a set one whenever its item is
    /// selected, an Automatic one only while the Tabs tool is on (Frage 415 C), where the job
    /// enters (C7); none in the Toolpath view, during a transform drag or on short paths.
    pub(super) fn start_markers(&self) -> Vec<(TabOutline, f64, bool)> {
        if self.toolpath_view_on() || self.select.drag_in_flight() {
            return Vec::new();
        }
        let scale = self.view().scale();
        let tool = self.tabs_tool();
        self.tab_outlines(None)
            .into_iter()
            .filter(|o| o.measured.closed() && o.measured.length() / 1000.0 * scale >= MIN_PATH_PX)
            .filter_map(|o| match (o.entry, o.entered) {
                (Some(OutlineEntry::Start(at)), _) => {
                    let arc = burn_start(&o, o.measured.arc_of(at));
                    Some((o, arc, true))
                }
                (_, Some(arc)) if tool => Some((o, arc, false)),
                _ => None,
            })
            .collect()
    }

    fn start_marker_at(&self, point: Point) -> Option<(TabOutline, f64)> {
        let radius = self.transform_handle_tolerances().param_hit.as_mm();
        self.start_markers()
            .into_iter()
            .map(|(o, arc, _)| {
                let d = o.to_document(arc).vector_to(point).length();
                (o, arc, d)
            })
            .filter(|(_, _, d)| *d <= radius)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(o, arc, _)| (o, arc))
    }

    /// A press on a start marker: it becomes current and its drag starts. Whether it took it.
    pub(super) fn start_press(&mut self, point: Point) -> bool {
        let Some((outline, arc)) = self.start_marker_at(point) else {
            self.operations.tabs.start_current = None;
            return false;
        };
        self.operations.tabs.current = None;
        self.operations.tabs.start_current = Some(StartRef {
            op: outline.op,
            key: outline.key.clone(),
            reference: outline.reference.clone(),
        });
        self.operations.tabs.start_drag = Some(StartDrag {
            outline,
            arc,
            snapped: false,
            limited: false,
            from: point,
            live: false,
        });
        true
    }

    /// Whether the pointer at `point` is over a start marker or one is held.
    pub(super) fn start_marker_hovered(&self, point: Point) -> bool {
        self.operations.tabs.start_drag.is_some() || self.start_marker_at(point).is_some()
    }

    /// The pointer moved to `point` with a start marker held: it slides along the path, snaps to
    /// a node within 8 px unless Alt is held, and stops at a tab's edge. Whether one is held.
    pub(super) fn start_drag_move(&mut self, point: Point) -> bool {
        let scale = self.view().scale();
        let alt = self.held.alt;
        let Some(drag) = self.operations.tabs.start_drag.as_mut() else {
            return false;
        };
        if !drag.live {
            if drag.from.vector_to(point).length() * scale < DEAD_ZONE_PX {
                return true;
            }
            drag.live = true;
        }
        let m = &drag.outline.measured;
        let (mut arc, _) = m.nearest(drag.outline.to_page_um(point));
        let mut snapped = false;
        if !alt {
            let snap = SNAP_PX / scale * 1000.0;
            let length = m.length();
            let near = m
                .vertex_arcs()
                .iter()
                .map(|&node| {
                    let d = (node - arc).abs();
                    (d.min(length - d), node)
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((d, node)) = near
                && d <= snap
            {
                arc = node;
                snapped = true;
            }
        }
        let (arc, limited) = clear_of_tabs(&drag.outline, &drag.outline.spans, arc);
        drag.arc = arc;
        drag.snapped = snapped && !limited;
        drag.limited = limited;
        true
    }

    /// The release of a start marker drag: one step "Move start point" when it moved.
    pub(super) fn start_drag_end(&mut self, point: Point) -> bool {
        if !self.start_drag_move(point) {
            return false;
        }
        let Some(drag) = self.operations.tabs.start_drag.take() else {
            return false;
        };
        if !drag.live {
            return true;
        }
        let fraction = drag.outline.measured.fraction_of(drag.arc);
        let entry = Some(OutlineEntry::Start(fraction));
        if drag.outline.entry == entry {
            return true;
        }
        let writes = self.entry_writes(&[(&drag.outline, entry)]);
        self.write_item_records(drag.outline.op, &writes, MOVE_START_POINT_COMMIT_LABEL);
        true
    }

    /// Escape or a lost pointer during a start marker drag: nothing is written.
    pub(super) fn cancel_start_drag(&mut self) -> bool {
        self.operations.tabs.start_drag.take().is_some()
    }

    /// Delete or Backspace on a current set start marker: the outline returns to Automatic, one
    /// step "Automatic start" (C8). Whether a set marker was current.
    pub(super) fn delete_current_start(&mut self) -> bool {
        let Some(current) = self.operations.tabs.start_current.clone() else {
            return false;
        };
        let Some(outline) = self
            .tab_outlines(Some(current.op))
            .into_iter()
            .find(|o| o.key == current.key && o.reference == current.reference)
            .filter(|o| o.entry.is_some())
        else {
            return false;
        };
        let writes = self.entry_writes(&[(&outline, None)]);
        self.write_item_records(outline.op, &writes, AUTOMATIC_START_COMMIT_LABEL);
        self.operations.tabs.start_current = None;
        true
    }

    /// The readout of a start marker drag: "Start · 12.50 mm" along the path, " · node".
    pub(super) fn start_readout(&self) -> Option<super::super::shapes::LiveReadout> {
        let drag = self.operations.tabs.start_drag.as_ref()?;
        let mut text = format!("Start · {:.2} mm", drag.arc / 1000.0);
        if drag.snapped {
            text.push_str(" · node");
        }
        Some(super::super::shapes::LiveReadout {
            text,
            anchor: self.pointer_position?,
        })
    }

    /// The start markers over the drawing (UX 5.2): a triangle whose base sits on the start and
    /// whose tip points along the cut, solid `--accent` when set or held, white with an accent
    /// ring when Automatic.
    pub(in crate::session) fn start_markers_draw_list(&self) -> DrawList {
        let scale = self.view().scale();
        let mut list = DrawList::default();
        if scale <= 0.0 {
            return list;
        }
        let current = self.operations.tabs.start_current.as_ref();
        let drag = self.operations.tabs.start_drag.as_ref();
        for (outline, arc, set) in self.start_markers() {
            let held = drag.is_some_and(|d| {
                d.outline.reference == outline.reference && d.outline.key == outline.key
            });
            let is_current = current.is_some_and(|c| {
                c.op == outline.op && c.key == outline.key && c.reference == outline.reference
            });
            let arc = if held {
                drag.map_or(arc, |d| d.arc)
            } else {
                arc
            };
            let solid = set || held || is_current;
            push_marker(&mut list, &outline, arc, scale, solid);
        }
        list
    }
}

fn push_marker(list: &mut DrawList, outline: &TabOutline, arc: f64, scale: f64, solid: bool) {
    let base = outline.to_document(arc);
    let (dx, dy) = outline.measured.tangent_at(arc);
    let (len, half) = (LENGTH_PX / scale, BASE_PX / 2.0 / scale);
    let tip = Point::new(base.x + dx * len, base.y + dy * len);
    let left = Point::new(base.x - dy * half, base.y + dx * half);
    let right = Point::new(base.x + dy * half, base.y - dx * half);
    let fill = if solid { ACCENT } else { RgbaColor::WHITE };
    for (position, color) in [(tip, fill), (left, fill), (right, fill)] {
        list.triangles.push(Vertex { position, color });
    }
    if !solid {
        let ring = RING_PX / scale;
        for (a, b) in [(tip, left), (left, right), (right, tip)] {
            list.extend(thick_line(a, b, ring, ACCENT));
        }
    }
}

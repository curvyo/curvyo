//! What the Find centre lines preview draws (`specs/0117-centre-lines` criterion 15b, UX notes
//! §4): the subjects washed to 35 %, their kept parts and inner areas at full strength, and over
//! them the result's painted outline, its centre lines and their nodes; never in the document.

use curvyo_document_core::{
    AnchorId, AnchorKind, AnchorSnapshot, Angle, Color, CurveKind, Fill, ObjectSnapshot, Opacity,
    PathSnapshot, Point, Stroke, Style, SubpathSnapshot, Vec2, ViewTransform,
};
use curvyo_geometry_core::OutlineTriple;
use curvyo_render_core::{DrawList, PreviewAnchor};
use curvyo_ui_core::INNER_AREA_COLOR;

use super::Session;
use super::trace_mesh::MeshFrame;

/// The wash over a subject under the preview: the canvas colour at 65 %, as "Trace preview".
const CENTRE_WASH: f64 = 0.65;

/// The half strength of a preview older than the values (UX notes §4).
const STALE_STRENGTH: f64 = 0.5;

/// A filled path of `rings` in `color` at `opacity`, no stroke, carrying `id` (never a node).
fn filled_path(
    id: curvyo_document_core::NodeId,
    rings: &[Vec<OutlineTriple>],
    color: Color,
    opacity: f64,
) -> Option<PathSnapshot> {
    let mut next = 0_u64;
    let mut subpaths = rings.iter().filter(|r| r.len() >= 2).map(|ring| {
        let anchors = ring
            .iter()
            .map(|&(point, handle_in, handle_out)| {
                next += 1;
                AnchorSnapshot {
                    id: AnchorId::new(0, next),
                    point,
                    handle_in,
                    handle_out,
                    kind: AnchorKind::Corner,
                }
            })
            .collect();
        SubpathSnapshot {
            closed: true,
            anchors,
        }
    });
    let first = subpaths.next()?;
    Some(PathSnapshot {
        id,
        closed: true,
        style: Style {
            stroke: Stroke {
                enabled: false,
                ..Stroke::default()
            },
            fill: Fill {
                enabled: true,
                color,
                opacity: Opacity::new(opacity).unwrap_or(Opacity::OPAQUE),
                ..Fill::default()
            },
        },
        anchors: first.anchors,
        extra_subpaths: subpaths.collect(),
        curve_kind: CurveKind::Bezier,
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: Angle::from_radians(0.0),
    })
}

/// A polygon as corner anchors.
fn polygon(points: &[Point]) -> Vec<OutlineTriple> {
    points
        .iter()
        .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
        .collect()
}

impl Session {
    /// The artwork under the preview: right above each subject its wash, then its kept parts in
    /// its own colour and its inner areas, at full strength.
    pub(super) fn centre_lines_artwork(&self, objects: &mut Vec<ObjectSnapshot>) {
        let Some(bar) = &self.centre else {
            return;
        };
        let white = Color {
            r: 255,
            g: 255,
            b: 255,
        };
        let mut inserts: Vec<(usize, Vec<ObjectSnapshot>)> = Vec::new();
        for (index, subject) in bar.found.subjects.iter().enumerate() {
            let Some(at) = objects.iter().position(|o| o.id() == subject.id) else {
                continue;
            };
            let rings: Vec<Vec<OutlineTriple>> =
                subject.outlines.iter().map(|(a, _)| a.clone()).collect();
            let mut added = Vec::new();
            added.extend(
                filled_path(subject.id, &rings, white, CENTRE_WASH).map(ObjectSnapshot::Path),
            );
            let lines = bar
                .preview
                .as_ref()
                .and_then(|p| p.get(index))
                .and_then(|r| r.lines.as_ref());
            if let Some(lines) = lines {
                let fill = subject.style.fill.color;
                let opacity = subject.style.fill.opacity.get();
                for area in &lines.inner_areas {
                    added.extend(
                        filled_path(subject.id, &[polygon(area)], INNER_AREA_COLOR, 1.0)
                            .map(ObjectSnapshot::Path),
                    );
                }
                for region in &lines.filled {
                    added.extend(
                        filled_path(subject.id, region, fill, opacity).map(ObjectSnapshot::Path),
                    );
                }
            }
            inserts.push((at + 1, added));
        }
        inserts.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
        for (at, added) in inserts {
            objects.splice(at..at, added);
        }
    }

    /// The preview over the artwork: the painted outline of each result path, its centre line
    /// and its nodes; at half strength while the values have moved on.
    pub(super) fn centre_lines_draw_into(&self, view: ViewTransform, list: &mut DrawList) {
        let Some(bar) = &self.centre else {
            return;
        };
        let Some(preview) = &bar.preview else {
            return;
        };
        let strength = if bar.stale { STALE_STRENGTH } else { 1.0 };
        let frame = MeshFrame {
            scale: view.scale(),
            stale: bar.stale,
            visible: None,
            moving: self.trace.moving,
        };
        let build = || {
            let mut outlines: Vec<Vec<PreviewAnchor>> = Vec::new();
            let mut lines: Vec<(Vec<PreviewAnchor>, bool)> = Vec::new();
            let mut nodes: Vec<Point> = Vec::new();
            for result in preview {
                outlines.extend(result.painted.iter().flatten().map(|ring| polygon(ring)));
                for path in result.lines.iter().flat_map(|l| &l.paths) {
                    lines.push((path.anchors.clone(), path.closed));
                    nodes.extend(path.anchors.iter().map(|a| a.0));
                }
            }
            let mut mesh = curvyo_render_core::build_preview_outline(&outlines, view, strength);
            mesh.extend(curvyo_render_core::build_centre_line_preview(
                &lines, &nodes, view, strength,
            ));
            (mesh, None)
        };
        // The preview has no artwork layers and no images: its triangles are overlay as they are.
        bar.mesh.with(frame, build, |mesh| {
            list.triangles.extend_from_slice(&mesh.triangles);
        });
    }

    /// Whether the Node tool's own marks (nodes, handles, boxes, marquee) stay away: the bar
    /// is open (UX notes §4).
    pub(super) fn centre_lines_hides_marks(&self) -> bool {
        self.centre.is_some()
    }
}

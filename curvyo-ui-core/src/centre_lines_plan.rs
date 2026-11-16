//! Planning Find centre lines on the selection (`specs/0117-centre-lines` criteria 1, 3, 4, 12,
//! 16 and 18; ADR 0063 §8): the subjects, the refusals, the node limit, and the drawing that
//! replaces each subject.
//!
//! Pure: it reads snapshots and returns subjects or a refusal, and turns a kernel result into
//! drawing nodes. The write is the session's; the sentences are the host's.

use std::collections::HashSet;

use curvyo_document_core::{
    AnchorKind, Color, DrawingAnchor, DrawingNode, DrawingOutline, DrawingRegisters, Fill, LineCap,
    LineJoin, NodeId, ObjectSnapshot, Point, Stroke, Style,
};
use curvyo_geometry_core::{CentreLines, CentreRole, OutlineTriple};

use crate::boolean::{is_open, outlines_of};

/// The node limit of criterion 18: above it the operation is refused with the count (the job
/// runs in the worker, ADR 0063 amendment 2026-11-18).
pub const CENTRE_LINES_NODE_LIMIT: usize = 100_000;

/// The commit label of Find centre lines; its history name is "Find centre lines".
pub const CENTRE_LINES_COMMIT_LABEL: &str = "centre_lines";

/// The fill of an inner area (CENTRE-INNERFILL default A): a neutral light grey.
pub const INNER_AREA_COLOR: Color = Color {
    r: 0xD9,
    g: 0xD9,
    b: 0xD9,
};

/// Why Find centre lines was refused. Nothing changes and no history entry is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CentreLinesRefusal {
    /// The first object that is not a subject is a text: "Convert text to path first."
    Text,
    /// It is an image: "Trace the image first."
    Image,
    /// Anything else: "Select filled shapes."
    NotFilled,
    /// More nodes than [`CENTRE_LINES_NODE_LIMIT`]: "Too large: n nodes."
    TooLarge {
        /// The node count of the subjects.
        nodes: usize,
    },
}

/// One subject: a filled closed shape as it is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct CentreSubject {
    /// Its id.
    pub id: NodeId,
    /// Its outlines `(anchors, closed)`.
    pub outlines: Vec<(Vec<OutlineTriple>, bool)>,
    /// Its style (the fill colour becomes the stroke colour).
    pub style: Style,
    /// Whether it is a linked clone.
    pub clone: bool,
}

/// The subjects of a selection and what is skipped.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CentreSubjects {
    /// Subjects in stacking order.
    pub subjects: Vec<CentreSubject>,
    /// Selected objects that are not subjects.
    pub skipped: usize,
    /// Clones of a subject that will become ordinary paths.
    pub followers: Vec<NodeId>,
}

/// Whether `object` is a subject: a filled closed path, compound path, primitive or clone.
#[must_use]
pub fn is_centre_subject(object: &ObjectSnapshot) -> bool {
    let fillable = matches!(
        object,
        ObjectSnapshot::Path(_) | ObjectSnapshot::Clone(_) | ObjectSnapshot::Primitive(_)
    );
    let fill = &object.style().fill;
    fillable && fill.enabled && fill.opacity.get() > 0.0 && !is_open(object)
}

/// The refusal for a selection without a subject, naming the first reason (criterion 4).
fn refusal_of(objects: &[&ObjectSnapshot]) -> CentreLinesRefusal {
    match objects.first() {
        Some(ObjectSnapshot::Text(_)) => CentreLinesRefusal::Text,
        Some(ObjectSnapshot::Image(_)) => CentreLinesRefusal::Image,
        _ => CentreLinesRefusal::NotFilled,
    }
}

/// Whether the button is enabled: some selected leaf is a subject (criterion 1).
#[must_use]
pub fn centre_lines_available(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> bool {
    let wanted: HashSet<NodeId> = selected.iter().copied().collect();
    objects
        .iter()
        .any(|o| wanted.contains(&o.id()) && !skipped(o.id()) && is_centre_subject(o))
}

/// The subjects among the selected leaves (`objects` in stacking order), or why there are none.
///
/// # Errors
/// [`CentreLinesRefusal`] when no selected leaf is a subject, or the subjects have more than
/// [`CENTRE_LINES_NODE_LIMIT`] nodes.
pub fn centre_line_subjects(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> Result<CentreSubjects, CentreLinesRefusal> {
    let wanted: HashSet<NodeId> = selected.iter().copied().collect();
    let chosen: Vec<&ObjectSnapshot> = objects
        .iter()
        .filter(|o| wanted.contains(&o.id()))
        .collect();
    let mut result = CentreSubjects::default();
    let mut others = Vec::new();
    for object in &chosen {
        if !skipped(object.id()) && is_centre_subject(object) {
            result.subjects.push(CentreSubject {
                id: object.id(),
                outlines: outlines_of(object),
                style: object.style().clone(),
                clone: matches!(object, ObjectSnapshot::Clone(_)),
            });
        } else {
            others.push(*object);
        }
    }
    if result.subjects.is_empty() {
        return Err(refusal_of(&others));
    }
    result.skipped = others.len();
    let nodes: usize = result
        .subjects
        .iter()
        .flat_map(|s| &s.outlines)
        .map(|(anchors, _)| anchors.len())
        .sum();
    if nodes > CENTRE_LINES_NODE_LIMIT {
        return Err(CentreLinesRefusal::TooLarge { nodes });
    }
    let ids: HashSet<NodeId> = result.subjects.iter().map(|s| s.id).collect();
    result.followers = objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Clone(clone)
                if ids.contains(&clone.chain.origin) && !ids.contains(&clone.id()) =>
            {
                Some(clone.id())
            }
            _ => None,
        })
        .collect();
    Ok(result)
}

/// One centre path's width profile as `(position, multiplier)`, in path order.
pub type ProfilePoints = Vec<(f64, f64)>;

/// The drawing that replaces a subject, and the profiles of its paths in leaf order.
#[derive(Debug, Clone, PartialEq)]
pub struct CentreDrawing {
    /// One path, or a group named after the subject (criterion 3).
    pub node: DrawingNode,
    /// The width profile of every leaf of `node` in order; empty for filled leaves.
    pub profiles: Vec<ProfilePoints>,
}

fn anchors_of(points: impl Iterator<Item = OutlineTriple>) -> Vec<DrawingAnchor> {
    points
        .map(|(point, handle_in, handle_out)| DrawingAnchor {
            point,
            handle_in,
            handle_out,
            kind: AnchorKind::Corner,
        })
        .collect()
}

fn named(name: &str) -> DrawingRegisters {
    DrawingRegisters {
        name: Some(name.to_string()),
        ..DrawingRegisters::default()
    }
}

/// A filled path of polygons with `style`.
fn filled(outlines: &[Vec<OutlineTriple>], style: Style, name: &str) -> DrawingNode {
    DrawingNode::Path {
        registers: named(name),
        outlines: outlines
            .iter()
            .map(|ring| DrawingOutline {
                closed: true,
                anchors: anchors_of(ring.iter().copied()),
            })
            .collect(),
        style,
    }
}

/// A polygon as corner anchors.
fn corners(ring: &[Point]) -> Vec<OutlineTriple> {
    ring.iter()
        .map(|p| {
            (
                *p,
                curvyo_document_core::Vec2::ZERO,
                curvyo_document_core::Vec2::ZERO,
            )
        })
        .collect()
}

/// The style of a centre path: Fill off, stroke in the subject's fill colour and opacity.
fn centre_style(subject: &Style, width: curvyo_document_core::Length, cap: LineCap) -> Style {
    Style {
        stroke: Stroke {
            enabled: true,
            width,
            color: subject.fill.color,
            opacity: subject.fill.opacity,
            join: LineJoin::Round,
            cap,
            ..Stroke::default()
        },
        fill: Fill {
            enabled: false,
            color: subject.fill.color,
            ..Fill::default()
        },
    }
}

/// The name of a subject's result group: "Centre lines of <name>", or "Centre lines" for an
/// unnamed subject.
fn group_name(name: Option<&str>) -> String {
    match name.map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => format!("Centre lines of {name}"),
        None => "Centre lines".to_string(),
    }
}

/// The drawing for `subject` from its kernel result, or `None` when it stays as it is (every
/// region a blob). Bottom to top: inner areas, kept regions, main paths, branches.
#[must_use]
pub fn centre_lines_drawing(
    subject: &CentreSubject,
    lines: &CentreLines,
    name: Option<&str>,
) -> Option<CentreDrawing> {
    if lines.all_filled() {
        return None;
    }
    let mut children = Vec::new();
    let mut profiles = Vec::new();
    let inner_style = Style {
        stroke: Stroke {
            enabled: false,
            ..Stroke::default()
        },
        fill: Fill {
            enabled: true,
            color: INNER_AREA_COLOR,
            ..Fill::default()
        },
    };
    for area in &lines.inner_areas {
        children.push(filled(
            std::slice::from_ref(&corners(area)),
            inner_style.clone(),
            "Inner area",
        ));
        profiles.push(Vec::new());
    }
    for region in &lines.filled {
        children.push(filled(region, subject.style.clone(), "Filled"));
        profiles.push(Vec::new());
    }
    for path in &lines.paths {
        let (cap, label) = match path.role {
            CentreRole::Main => (LineCap::Round, "Centre line"),
            CentreRole::Branch => (LineCap::Butt, "Branch"),
        };
        children.push(DrawingNode::Path {
            registers: named(label),
            outlines: vec![DrawingOutline {
                closed: path.closed,
                anchors: anchors_of(path.anchors.iter().copied()),
            }],
            style: centre_style(&subject.style, path.width, cap),
        });
        let points = path
            .profile
            .positions()
            .map(|at| (at, path.profile.sample(at).unwrap_or(1.0)))
            .collect();
        profiles.push(points);
    }
    let node = if children.len() == 1 {
        children.remove(0)
    } else {
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some(group_name(name)),
                ..DrawingRegisters::default()
            },
            layer: false,
            children,
        }
    };
    Some(CentreDrawing { node, profiles })
}

#[cfg(test)]
mod tests {
    use super::group_name;

    #[test]
    fn the_result_group_is_named_centre_lines_of_its_subject() {
        assert_eq!(group_name(None), "Centre lines");
        assert_eq!(group_name(Some("  ")), "Centre lines");
        assert_eq!(group_name(Some("Whale")), "Centre lines of Whale");
    }
}

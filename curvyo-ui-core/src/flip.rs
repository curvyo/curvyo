//! The flip of a selection and its no-op test (`specs/0056-mirror-and-negative-scale/`
//! criteria 2, 3 and 18): the signed scale map `(-1, 1)` or `(1, -1)` about the centre of
//! the selection's true bounding box, in the document axes, applied with the same
//! [`apply`](crate::group_transform) as a group drag and written as one commit; a flip
//! whose every result looks like its start writes nothing.

use curvyo_document_core::{
    Document, NodeId, ObjectSnapshot, OutlineAnchor, Point, Shape, Vec2, outline_of_rotated,
};

use crate::conversion::is_circle;
use crate::group_transform::{GroupMap, commit_group, map_all};
use crate::object_bounds::object_outline_bounds;
use crate::object_selection::ObjectSelection;
use crate::transform_commit::same_within_tolerance;
use crate::transform_drag::{ScaleModes, StrokeScaling};

/// Two points of a drawn outline closer than this (millimetres) are the same point
/// (`specs/0009-unified-object-editing/` criterion 12).
const SAME_POINT_MM: f64 = 1e-9;

/// The commit label of a horizontal flip ("Flip horizontally" in the History).
pub(crate) const FLIP_HORIZONTAL_COMMIT_LABEL: &str = "flip_horizontal";

/// The commit label of a vertical flip ("Flip vertically" in the History).
pub(crate) const FLIP_VERTICAL_COMMIT_LABEL: &str = "flip_vertical";

/// Which way a flip mirrors the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipAxis {
    /// Left to right, about the vertical line through the box centre.
    Horizontal,
    /// Top to bottom, about the horizontal line through the box centre.
    Vertical,
}

impl FlipAxis {
    const fn label(self) -> &'static str {
        match self {
            Self::Horizontal => FLIP_HORIZONTAL_COMMIT_LABEL,
            Self::Vertical => FLIP_VERTICAL_COMMIT_LABEL,
        }
    }

    /// The flip about `centre` as the scale map of a group gesture.
    const fn map(self, centre: Point) -> GroupMap {
        let (sx, sy) = match self {
            Self::Horizontal => (-1.0, 1.0),
            Self::Vertical => (1.0, -1.0),
        };
        GroupMap::Scale {
            pivot: centre,
            sx,
            sy,
        }
    }
}

/// What [`flip_selection`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipOutcome {
    /// One commit was written.
    Flipped,
    /// Every object looks the same flipped: nothing was written (criterion 3).
    LooksTheSame,
    /// Nothing is selected, or no selected object exists: nothing was written.
    NothingSelected,
}

/// Flips the selected objects of `objects` by `axis` about the centre of their true
/// bounding box and writes the result as one commit (criterion 2). The stroke width and
/// the corner radii are never rewritten (criterion 23: the magnitudes are 1).
pub fn flip_selection(
    document: &Document,
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    axis: FlipAxis,
) -> FlipOutcome {
    let Some((map, starts, results)) = flip_results(objects, selection.ids(), axis) else {
        return FlipOutcome::NothingSelected;
    };
    if results
        .iter()
        .zip(&starts)
        .all(|(new, old)| looks_the_same(new, old))
    {
        return FlipOutcome::LooksTheSame;
    }
    if commit_group(
        document,
        &results,
        StrokeScaling::Keep,
        axis.label(),
        Some(&map),
    ) {
        FlipOutcome::Flipped
    } else {
        FlipOutcome::NothingSelected
    }
}

/// The flip's map, the selected objects of `objects` named by `ids` (in the order of
/// `objects`) and their flipped snapshots; `None` when none of `ids` is in `objects`.
fn flip_results(
    objects: &[ObjectSnapshot],
    ids: &[NodeId],
    axis: FlipAxis,
) -> Option<(GroupMap, Vec<ObjectSnapshot>, Vec<ObjectSnapshot>)> {
    let wanted: std::collections::HashSet<NodeId> = ids.iter().copied().collect();
    let starts: Vec<ObjectSnapshot> = objects
        .iter()
        .filter(|object| wanted.contains(&object.id()))
        .cloned()
        .collect();
    let (min, max) =
        starts
            .iter()
            .map(object_outline_bounds)
            .reduce(|(a_min, a_max), (b_min, b_max)| {
                (
                    Point::new(a_min.x.min(b_min.x), a_min.y.min(b_min.y)),
                    Point::new(a_max.x.max(b_max.x), a_max.y.max(b_max.y)),
                )
            })?;
    let centre = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    let map = axis.map(centre);
    let converted = vec![None; starts.len()];
    let results = map_all(&starts, &converted, &map, ScaleModes::default());
    Some((map, starts, results))
}

/// Whether `a` is drawn the same as `b` (criterion 3): the same snapshot within the
/// tolerances of `0009` criterion 12, or the same circle at any rotation, or the same
/// outlines (a symmetric path or polygon whose anchors were permuted by the mirror).
fn looks_the_same(a: &ObjectSnapshot, b: &ObjectSnapshot) -> bool {
    if same_within_tolerance(a, b) {
        return true;
    }
    if let (ObjectSnapshot::Primitive(pa), ObjectSnapshot::Primitive(pb)) = (a, b)
        && let (Shape::Ellipse { frame: fa, .. }, Shape::Ellipse { frame: fb, .. }) =
            (pa.shape, pb.shape)
        && is_circle(&fa)
        && is_circle(&fb)
    {
        return same_point(fa.center, fb.center)
            && (fa.rx.as_mm() - fb.rx.as_mm()).abs() <= SAME_POINT_MM;
    }
    outlines_match(&outlines(a), &outlines(b))
}

/// One drawn outline: its cubic segments (start, two control points, end) and whether it
/// is closed.
struct Outline {
    closed: bool,
    segments: Vec<[Point; 4]>,
}

fn outlines(object: &ObjectSnapshot) -> Vec<Outline> {
    match object {
        ObjectSnapshot::Primitive(primitive) => {
            let outline = outline_of_rotated(&primitive.shape, primitive.rotation);
            let anchors: Vec<(Point, Vec2, Vec2)> = outline
                .iter()
                .map(|a: &OutlineAnchor| (a.point, a.handle_in, a.handle_out))
                .collect();
            vec![outline_of(&anchors, outline.closed)]
        }
        ObjectSnapshot::Path(path)
        | ObjectSnapshot::Clone(curvyo_document_core::CloneSnapshot { resolved: path, .. }) => path
            .subpaths()
            .map(|subpath| {
                let anchors: Vec<(Point, Vec2, Vec2)> = subpath
                    .anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect();
                outline_of(&anchors, subpath.closed)
            })
            .collect(),
    }
}

fn outline_of(anchors: &[(Point, Vec2, Vec2)], closed: bool) -> Outline {
    let count = anchors.len();
    let pairs = if closed {
        count
    } else {
        count.saturating_sub(1)
    };
    let segments = (0..pairs)
        .map(|i| {
            let (p0, _, out) = anchors[i];
            let (p3, into, _) = anchors[(i + 1) % count];
            [p0, p0.translated(out), p3.translated(into), p3]
        })
        .collect();
    Outline { closed, segments }
}

/// Whether the two outline sets match one to one: an open outline segment by segment in
/// order (its start and end stay where they were, so its markers do), a closed one as a
/// set of segments, each forward or reversed.
fn outlines_match(a: &[Outline], b: &[Outline]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|outline| {
        let found = b.iter().enumerate().position(|(index, other)| {
            !used[index]
                && other.closed == outline.closed
                && other.segments.len() == outline.segments.len()
                && if outline.closed {
                    segments_match_as_sets(&outline.segments, &other.segments)
                } else {
                    outline
                        .segments
                        .iter()
                        .zip(&other.segments)
                        .all(|(s, t)| same_segment(s, t))
                }
        });
        found.inspect(|&index| used[index] = true).is_some()
    })
}

fn segments_match_as_sets(a: &[[Point; 4]], b: &[[Point; 4]]) -> bool {
    let mut used = vec![false; b.len()];
    a.iter().all(|segment| {
        let reversed = [segment[3], segment[2], segment[1], segment[0]];
        let found = b.iter().enumerate().position(|(index, other)| {
            !used[index] && (same_segment(segment, other) || same_segment(&reversed, other))
        });
        found.inspect(|&index| used[index] = true).is_some()
    })
}

fn same_segment(a: &[Point; 4], b: &[Point; 4]) -> bool {
    a.iter().zip(b).all(|(p, q)| same_point(*p, *q))
}

fn same_point(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() <= SAME_POINT_MM && (a.y - b.y).abs() <= SAME_POINT_MM
}

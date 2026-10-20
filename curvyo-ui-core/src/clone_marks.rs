//! The canvas marks of a selected linked clone and of what it follows (`specs/0049-linked-clone`
//! criteria 57 to 62, `adrs.md` decision 12): which objects are traced in which role and form,
//! where their tags go, and the edge chips of marked objects off screen. View state, pure.

use std::collections::{HashMap, HashSet};

use curvyo_document_core::{NodeId, ObjectSnapshot, Point, ViewTransform};

use crate::object_bounds::object_outline_bounds;

/// At most this many selected clones are traced (criterion 61).
pub const MAX_CLONE_TRACES: usize = 200;
/// At most this many originals (direct and root) are traced (criteria 58, 61).
pub const MAX_ORIGINAL_TRACES: usize = 50;
/// At most this many faint clones of a selected original are traced (criterion 59).
pub const MAX_FAINT_TRACES: usize = 50;
/// Tags are drawn only while at most this many objects are marked (criterion 61).
pub const MAX_TAGGED_OBJECTS: usize = 12;
/// A tag needs an object at least this large on screen, in both directions (px).
pub const MIN_TAGGED_BOX_PX: f64 = 24.0;
/// Below this size on screen (px, both directions) an object is marked by a ring.
pub const RING_BELOW_PX: f64 = 6.0;
/// At most this many edge chips (criterion 58).
pub const MAX_EDGE_CHIPS: usize = 8;
/// A tag's height (px), its distance from the object's box, and its raised distance.
pub const TAG_HEIGHT_PX: f64 = 20.0;
const TAG_GAP_PX: f64 = 10.0;
const TAG_RAISED_GAP_PX: f64 = 36.0;
/// The width of a tag: the glyph and padding plus an average 12 px character.
const TAG_FIXED_PX: f64 = 28.0;
const TAG_CHAR_PX: f64 = 6.5;

/// The role an object is marked in, strongest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkRole {
    /// A selected clone: 2 px dashed (6 on, 3 off).
    Clone,
    /// The direct original of a selected clone: 2 px solid.
    Original,
    /// The root original of a chain, when it is not the direct original: 1 px solid.
    Root,
    /// A clone of a selected original (the Proposal of criterion 59): 1 px dashed at 65 %.
    FaintClone,
}

/// How a trace is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkForm {
    /// The object's outline, traced.
    Trace,
    /// An 8 px ring around the centre of an object under 6 px on screen.
    Ring,
}

/// One traced object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkTrace {
    /// The object; the renderer draws its outline from the read.
    pub id: NodeId,
    /// Its strongest role.
    pub role: MarkRole,
    /// Trace or ring.
    pub form: MarkForm,
}

/// A screen rectangle in px: top-left corner and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
}

impl MarkRect {
    fn meets(&self, other: &Self) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }

    fn inside(&self, w: f64, h: f64) -> bool {
        self.x >= 0.0 && self.y >= 0.0 && self.x + self.w <= w && self.y + self.h <= h
    }
}

/// A text tag attached to a marked object.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkTag {
    /// The object.
    pub id: NodeId,
    /// "Clone", "Original", "Root", "Clone + Original", "Original (12 clones)".
    pub text: String,
    /// Where the tag sits on screen.
    pub rect: MarkRect,
}

/// A chip at the viewport edge pointing to a marked original off screen.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeChip {
    /// "Original" or "Root".
    pub text: &'static str,
    /// The chip's centre on screen (px), on the viewport edge.
    pub at: (f64, f64),
    /// The direction to the object, radians from the x axis (screen, y down).
    pub toward: f64,
}

/// Everything the frame draws for linked clones.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CloneMarks {
    /// Traces and rings, back to front in role order.
    pub traces: Vec<MarkTrace>,
    /// Tags.
    pub tags: Vec<MarkTag>,
    /// Edge chips.
    pub chips: Vec<EdgeChip>,
}

/// What [`clone_marks`] needs about the frame.
#[derive(Debug, Clone, Copy)]
pub struct MarkFrame<'a> {
    /// The view.
    pub view: ViewTransform,
    /// The viewport size (px).
    pub viewport: (f64, f64),
    /// The screen rectangles a tag may not meet: drawn handle glyphs, the centre handle,
    /// reference marks, refusal labels, DOM chips.
    pub obstacles: &'a [MarkRect],
    /// A canvas drag or a preview outline is in the frame: no marks at all (criterion 59).
    pub suppressed: bool,
    /// The effectively hidden objects (`0050`): a hidden original is not traced, and its clone's
    /// tag reads "Clone (original hidden)" (criterion 58).
    pub hidden: &'a HashSet<NodeId>,
}

/// The marks for `selected` (the selection, Select or Node tool) over `objects` (the read).
#[must_use]
pub fn clone_marks(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    frame: &MarkFrame<'_>,
) -> CloneMarks {
    if frame.suppressed || selected.is_empty() {
        return CloneMarks::default();
    }
    let by_id: HashMap<NodeId, &ObjectSnapshot> =
        objects.iter().map(|object| (object.id(), object)).collect();
    let chosen: HashSet<NodeId> = selected.iter().copied().collect();
    let mut roles = roles_of(objects, &by_id, &chosen, frame);
    roles.retain(|(id, _)| !frame.hidden.contains(id));
    // Tags are placed in role order, so a selected clone gets the first free candidate even when
    // it lies on what it follows (UX review must-fix 4); stacking order within a role.
    roles.sort_by_key(|(_, kinds)| kinds[0]);
    // An original or root whose outline is a selected clone's (a fresh clone lies on it) is not
    // traced: its solid line would hide the clone's dash. Its tag stays.
    let covered: Vec<&curvyo_document_core::PathSnapshot> = selected
        .iter()
        .filter_map(|id| match by_id.get(id) {
            Some(ObjectSnapshot::Clone(clone)) => Some(&clone.resolved),
            _ => None,
        })
        .collect();
    let mut marks = CloneMarks::default();
    let tagged = roles.len() <= MAX_TAGGED_OBJECTS;
    let mut placed: Vec<MarkRect> = Vec::new();
    for (id, kinds) in &roles {
        let Some(object) = by_id.get(id) else {
            continue;
        };
        let (min, max) = screen_box(object, frame.view);
        let visible = MarkRect {
            x: min.0,
            y: min.1,
            w: max.0 - min.0,
            h: max.1 - min.1,
        }
        .meets(&MarkRect {
            x: 0.0,
            y: 0.0,
            w: frame.viewport.0,
            h: frame.viewport.1,
        });
        let strongest = kinds[0];
        if !visible {
            if matches!(strongest, MarkRole::Original | MarkRole::Root)
                && marks.chips.len() < MAX_EDGE_CHIPS
            {
                marks
                    .chips
                    .push(edge_chip(strongest, min, max, frame.viewport));
            }
            continue;
        }
        let small = max.0 - min.0 < RING_BELOW_PX && max.1 - min.1 < RING_BELOW_PX;
        let under_a_clone = matches!(strongest, MarkRole::Original | MarkRole::Root)
            && outline_of(object)
                .is_some_and(|outline| covered.iter().any(|clone| same_outline(outline, clone)));
        if !under_a_clone {
            marks.traces.push(MarkTrace {
                id: *id,
                role: strongest,
                form: if small {
                    MarkForm::Ring
                } else {
                    MarkForm::Trace
                },
            });
        }
        let large = max.0 - min.0 >= MIN_TAGGED_BOX_PX && max.1 - min.1 >= MIN_TAGGED_BOX_PX;
        if tagged && large && strongest != MarkRole::FaintClone {
            let mut text = tag_text(kinds, objects, *id, &chosen);
            if text == "Clone"
                && let Some(ObjectSnapshot::Clone(clone)) = by_id.get(id)
                && frame.hidden.contains(&clone.chain.origin)
            {
                "Clone (original hidden)".clone_into(&mut text);
            } else if text == "Clone"
                && let Some(ObjectSnapshot::Clone(clone)) = by_id.get(id)
                && let Some(own) = crate::clone_panel::overrides_words(clone)
            {
                // `specs/0057-clone-overrides` criterion 29: "Clone, own placement and style".
                text = format!("Clone, {own}");
            }
            if let Some(rect) = place_tag(&text, min, max, frame, &placed) {
                placed.push(rect);
                marks.tags.push(MarkTag {
                    id: *id,
                    text,
                    rect,
                });
            }
        }
    }
    marks
        .traces
        .sort_by_key(|trace| std::cmp::Reverse(trace.role));
    marks
}

/// The drawn outline of a path or a clone; `None` for a primitive.
fn outline_of(object: &ObjectSnapshot) -> Option<&curvyo_document_core::PathSnapshot> {
    match object {
        ObjectSnapshot::Path(path) => Some(path),
        ObjectSnapshot::Clone(clone) => Some(&clone.resolved),
        ObjectSnapshot::Primitive(_) | ObjectSnapshot::Text(_) => None,
    }
}

/// The largest distance (mm) at which two anchors of a traced outline count as one.
const SAME_OUTLINE_MM: f64 = 1e-6;

/// Whether `a` and `b` draw the same outline: the same anchors, points and handles within
/// [`SAME_OUTLINE_MM`].
fn same_outline(
    a: &curvyo_document_core::PathSnapshot,
    b: &curvyo_document_core::PathSnapshot,
) -> bool {
    let near = |p: f64, q: f64| (p - q).abs() <= SAME_OUTLINE_MM;
    a.all_anchors().count() == b.all_anchors().count()
        && a.all_anchors().zip(b.all_anchors()).all(|(x, y)| {
            near(x.point.x, y.point.x)
                && near(x.point.y, y.point.y)
                && near(x.handle_in.x, y.handle_in.x)
                && near(x.handle_in.y, y.handle_in.y)
                && near(x.handle_out.x, y.handle_out.x)
                && near(x.handle_out.y, y.handle_out.y)
        })
}

/// Every marked object with its roles (strongest first), within the caps, in stacking order.
fn roles_of(
    objects: &[ObjectSnapshot],
    by_id: &HashMap<NodeId, &ObjectSnapshot>,
    chosen: &HashSet<NodeId>,
    frame: &MarkFrame<'_>,
) -> Vec<(NodeId, Vec<MarkRole>)> {
    let mut roles: HashMap<NodeId, Vec<MarkRole>> = HashMap::new();
    let mut screen = Visibility {
        frame,
        hulls: HashMap::new(),
    };
    let mut on_screen = |object: &ObjectSnapshot| screen.on_screen(object);
    let mut clones = 0;
    let mut originals: Vec<NodeId> = Vec::new();
    let mut any_clone = false;
    for object in objects {
        let ObjectSnapshot::Clone(clone) = object else {
            continue;
        };
        if !chosen.contains(&clone.id()) {
            continue;
        }
        any_clone = true;
        if clones < MAX_CLONE_TRACES && on_screen(object) {
            clones += 1;
            roles.entry(clone.id()).or_default().push(MarkRole::Clone);
        }
        for (target, role) in [
            (clone.chain.origin, MarkRole::Original),
            (clone.chain.root, MarkRole::Root),
        ] {
            if role == MarkRole::Root && target == clone.chain.origin {
                continue;
            }
            if !originals.contains(&target) {
                if originals.len() >= MAX_ORIGINAL_TRACES {
                    continue;
                }
                originals.push(target);
            }
            let entry = roles.entry(target).or_default();
            if !entry.contains(&role) {
                entry.push(role);
            }
        }
    }
    if !any_clone {
        let mut faint = 0;
        for object in objects {
            if let ObjectSnapshot::Clone(clone) = object
                && chosen.contains(&clone.chain.origin)
                && faint < MAX_FAINT_TRACES
                && on_screen(object)
            {
                faint += 1;
                roles
                    .entry(clone.id())
                    .or_default()
                    .push(MarkRole::FaintClone);
                let entry = roles.entry(clone.chain.origin).or_default();
                if !entry.contains(&MarkRole::Original) {
                    entry.push(MarkRole::Original);
                }
            }
        }
    }
    let mut ordered: Vec<(NodeId, Vec<MarkRole>)> = objects
        .iter()
        .filter_map(|object| {
            let mut kinds = roles.remove(&object.id())?;
            kinds.sort();
            Some((object.id(), kinds))
        })
        .collect();
    // Originals that are not in the read (an orphan's target) have nothing to trace.
    ordered.retain(|(id, _)| by_id.contains_key(id));
    ordered
}

/// The text of a tag: the roles, joined ("Clone + Original"); a selected original that clones
/// follow and no selected clone reads "Original (12 clones)".
fn tag_text(
    kinds: &[MarkRole],
    objects: &[ObjectSnapshot],
    id: NodeId,
    chosen: &HashSet<NodeId>,
) -> String {
    if kinds == [MarkRole::Original] && chosen.contains(&id) {
        let count = objects
            .iter()
            .filter(|object| matches!(object, ObjectSnapshot::Clone(c) if c.chain.origin == id))
            .count();
        return format!(
            "Original ({count} {})",
            if count == 1 { "clone" } else { "clones" }
        );
    }
    kinds
        .iter()
        .filter_map(|role| match role {
            MarkRole::Clone => Some("Clone"),
            MarkRole::Original => Some("Original"),
            MarkRole::Root => Some("Root"),
            MarkRole::FaintClone => None,
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

/// Whether objects meet the viewport. A clone's box is its root's control-point hull under the
/// composed map, the hull computed once per root: O(1) per clone, so a scan over 10,000 clones
/// stays cheap (criterion 61).
struct Visibility<'a, 'b> {
    frame: &'a MarkFrame<'b>,
    hulls: HashMap<*const curvyo_document_core::PathSnapshot, (Point, Point)>,
}

impl Visibility<'_, '_> {
    fn on_screen(&mut self, object: &ObjectSnapshot) -> bool {
        let view = self.frame.view;
        let (min, max) = match object {
            ObjectSnapshot::Clone(clone) => {
                let root = &clone.chain.root_outline;
                let (lo, hi) = *self
                    .hulls
                    .entry(std::sync::Arc::as_ptr(root))
                    .or_insert_with(|| hull(root));
                [
                    Point::new(lo.x, lo.y),
                    Point::new(hi.x, lo.y),
                    Point::new(lo.x, hi.y),
                    Point::new(hi.x, hi.y),
                ]
                .map(|p| view.document_to_screen(clone.chain.composed.apply_point(p)))
                .iter()
                .fold(
                    (
                        (f64::INFINITY, f64::INFINITY),
                        (f64::NEG_INFINITY, f64::NEG_INFINITY),
                    ),
                    |(lo, hi), c| {
                        (
                            (lo.0.min(c.0), lo.1.min(c.1)),
                            (hi.0.max(c.0), hi.1.max(c.1)),
                        )
                    },
                )
            }
            _ => screen_box(object, view),
        };
        let (w, h) = self.frame.viewport;
        max.0 >= 0.0 && max.1 >= 0.0 && min.0 <= w && min.1 <= h
    }
}

/// The box of a path's points and handle ends: it holds the drawn outline.
fn hull(path: &curvyo_document_core::PathSnapshot) -> (Point, Point) {
    let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
    let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for anchor in path.all_anchors() {
        for p in [
            anchor.point,
            anchor.point.translated(anchor.handle_in),
            anchor.point.translated(anchor.handle_out),
        ] {
            lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
        }
    }
    (lo, hi)
}

/// The object's drawn outline box on screen: `(min, max)` corners in px.
fn screen_box(object: &ObjectSnapshot, view: ViewTransform) -> ((f64, f64), (f64, f64)) {
    let (min, max) = object_outline_bounds(object);
    let a = view.document_to_screen(min);
    let b = view.document_to_screen(max);
    ((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1)))
}

/// The first free candidate place of a tag (criterion 60 (c)), or `None` when every one meets an
/// obstacle, another tag or the viewport edge: then the tag is dropped and the trace stays.
fn place_tag(
    text: &str,
    min: (f64, f64),
    max: (f64, f64),
    frame: &MarkFrame<'_>,
    placed: &[MarkRect],
) -> Option<MarkRect> {
    #[allow(clippy::cast_precision_loss)] // a tag's text is a few characters
    let w = TAG_FIXED_PX + TAG_CHAR_PX * text.chars().count() as f64;
    let h = TAG_HEIGHT_PX;
    let above = min.1 - TAG_GAP_PX - h;
    let below = max.1 + TAG_GAP_PX;
    let left = min.0;
    let right = max.0 - w;
    let candidates = [
        (left, above),
        (right, above),
        (left, below),
        (right, below),
        (left, min.1 - TAG_RAISED_GAP_PX - h),
        (left, max.1 + TAG_RAISED_GAP_PX),
    ];
    candidates
        .into_iter()
        .map(|(x, y)| MarkRect { x, y, w, h })
        .find(|rect| {
            rect.inside(frame.viewport.0, frame.viewport.1)
                && !frame.obstacles.iter().any(|o| rect.meets(o))
                && !placed.iter().any(|o| rect.meets(o))
        })
}

/// The chip for an object whose box lies wholly outside the viewport: on the nearest edge,
/// pointing toward the object's centre.
fn edge_chip(role: MarkRole, min: (f64, f64), max: (f64, f64), viewport: (f64, f64)) -> EdgeChip {
    let centre = (f64::midpoint(min.0, max.0), f64::midpoint(min.1, max.1));
    let middle = (viewport.0 / 2.0, viewport.1 / 2.0);
    let at = (
        centre.0.clamp(0.0, viewport.0),
        centre.1.clamp(0.0, viewport.1),
    );
    EdgeChip {
        text: if role == MarkRole::Root {
            "Root"
        } else {
            "Original"
        },
        at,
        toward: (centre.1 - middle.1).atan2(centre.0 - middle.0),
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, Vec2};

    use super::*;

    fn square(d: &Document, n: u64, x: f64, size: f64) -> NodeId {
        let c = |k: u64, dx: f64, dy: f64| {
            NewAnchor::corner(AnchorId::new(1, n * 10 + k), Point::new(x + dx, dy))
        };
        d.create_path(
            &[
                c(0, 0.0, 0.0),
                c(1, size, 0.0),
                c(2, size, size),
                c(3, 0.0, size),
            ],
            true,
        )
    }

    static NO_HIDDEN: std::sync::LazyLock<HashSet<NodeId>> = std::sync::LazyLock::new(HashSet::new);

    fn frame(obstacles: &[MarkRect]) -> MarkFrame<'_> {
        MarkFrame {
            view: ViewTransform::new(4.0, Point::new(0.0, 0.0)),
            viewport: (1200.0, 800.0),
            obstacles,
            suppressed: false,
            hidden: &NO_HIDDEN,
        }
    }

    /// B → A → O with B selected: B dashed "Clone", A "Original", O "Root" (criterion 58).
    fn chain() -> (Document, NodeId, NodeId, NodeId) {
        let d = Document::new(1);
        let o = square(&d, 1, 10.0, 20.0);
        let a = d.linked_clone(&[o], &[]).unwrap()[0];
        d.translate_objects(&[a], Vec2::new(60.0, 0.0)).unwrap();
        let b = d.linked_clone(&[a], &[]).unwrap()[0];
        d.translate_objects(&[b], Vec2::new(0.0, 60.0)).unwrap();
        (d, o, a, b)
    }

    #[test]
    fn criterion_58_a_chain_marks_the_clone_its_original_and_its_root() {
        let (d, o, a, b) = chain();
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[b], &frame(&[]));
        let role = |id| marks.traces.iter().find(|t| t.id == id).map(|t| t.role);
        assert_eq!(role(b), Some(MarkRole::Clone));
        assert_eq!(role(a), Some(MarkRole::Original));
        assert_eq!(role(o), Some(MarkRole::Root));
        let text = |id| {
            marks
                .tags
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.text.clone())
        };
        assert_eq!(text(b).as_deref(), Some("Clone"));
        assert_eq!(text(a).as_deref(), Some("Original"));
        assert_eq!(text(o).as_deref(), Some("Root"));
    }

    /// UX review must-fix 4: right after Linked clone the clone lies on its original. The
    /// clone keeps its tag (tags are placed in role order), and the original's solid trace is
    /// not drawn over the clone's dashed one (its tag stays).
    #[test]
    fn criterion_58_a_fresh_clone_on_its_original_keeps_its_tag_and_its_dash() {
        let d = Document::new(1);
        let o = square(&d, 1, 10.0, 20.0);
        let c = d.linked_clone(&[o], &[]).unwrap()[0];
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[c], &frame(&[]));
        assert!(marks.tags.iter().any(|t| t.id == c && t.text == "Clone"));
        assert!(marks.tags.iter().any(|t| t.id == o && t.text == "Original"));
        assert_eq!(
            marks.traces.iter().map(|t| t.id).collect::<Vec<_>>(),
            [c],
            "only the dashed trace"
        );
        // A chain lying on itself: neither the original nor the root is traced.
        let b = d.linked_clone(&[c], &[]).unwrap()[0];
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[b], &frame(&[]));
        assert_eq!(marks.traces.iter().map(|t| t.id).collect::<Vec<_>>(), [b]);
        assert!(marks.tags.iter().any(|t| t.id == b && t.text == "Clone"));
    }

    #[test]
    fn criterion_57_no_clone_selected_no_marks_and_a_drag_hides_them() {
        let (d, o, _, b) = chain();
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[], &frame(&[]));
        assert_eq!(marks, CloneMarks::default());
        let mut suppressed = frame(&[]);
        suppressed.suppressed = true;
        assert_eq!(
            clone_marks(&objects, &[b], &suppressed),
            CloneMarks::default()
        );
        // The Proposal of criterion 59: a selected original tags its clone count.
        let marks = clone_marks(&objects, &[o], &frame(&[]));
        assert!(marks.tags.iter().any(|t| t.text == "Original (1 clone)"));
        assert!(marks.traces.iter().any(|t| t.role == MarkRole::FaintClone));
    }

    #[test]
    fn criterion_58_one_object_with_two_roles_gets_the_strongest_trace_and_one_tag() {
        let (d, _, a, b) = chain();
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[a, b], &frame(&[]));
        let a_trace = marks.traces.iter().find(|t| t.id == a).unwrap();
        assert_eq!(a_trace.role, MarkRole::Clone);
        let a_tags: Vec<_> = marks.tags.iter().filter(|t| t.id == a).collect();
        assert_eq!(a_tags.len(), 1);
        assert_eq!(a_tags[0].text, "Clone + Original");
    }

    #[test]
    fn criterion_60_tags_never_meet_obstacles_or_each_other() {
        let (d, _, a, b) = chain();
        let objects = d.read_objects().objects;
        for k in 0..200 {
            #[allow(clippy::cast_precision_loss)]
            let shift = f64::from(k) * 3.0;
            let obstacles = [MarkRect {
                x: shift,
                y: 150.0,
                w: 40.0,
                h: 40.0,
            }];
            let marks = clone_marks(&objects, &[a, b], &frame(&obstacles));
            for (i, tag) in marks.tags.iter().enumerate() {
                assert!(!tag.rect.meets(&obstacles[0]));
                assert!(tag.rect.inside(1200.0, 800.0));
                for other in &marks.tags[i + 1..] {
                    assert!(!tag.rect.meets(&other.rect));
                }
            }
        }
    }

    #[test]
    fn criterion_61_many_clones_cap_the_traces_and_drop_the_tags() {
        let d = Document::new(1);
        let o = square(&d, 1, 0.0, 1.0);
        let sources = vec![o; 1];
        let mut all = Vec::new();
        for _ in 0..250 {
            all.push(d.linked_clone(&sources, &[]).unwrap()[0]);
        }
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &all, &frame(&[]));
        let clones = marks
            .traces
            .iter()
            .filter(|t| t.role == MarkRole::Clone)
            .count();
        assert_eq!(clones, MAX_CLONE_TRACES);
        // The original lies under the clones, so its solid trace is left out (must-fix 4).
        assert_eq!(marks.traces.len(), MAX_CLONE_TRACES, "no original trace");
        assert_eq!(marks.tags, Vec::new());
        // At a zoom where the square is under 6 px it is a ring.
        let mut small = frame(&[]);
        small.view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let marks = clone_marks(&objects, &all[..1], &small);
        assert!(marks.traces.iter().all(|t| t.form == MarkForm::Ring));
    }

    #[test]
    fn criterion_58_an_original_off_screen_gets_an_edge_chip() {
        let (d, o, _, b) = chain();
        d.translate_objects(&[o], Vec2::new(-1000.0, 0.0)).unwrap();
        let objects = d.read_objects().objects;
        let marks = clone_marks(&objects, &[b], &frame(&[]));
        assert_eq!(marks.chips.len(), 2, "A follows O off screen too");
        assert!(marks.chips.iter().all(|c| c.at.0 == 0.0));
        let _ = b;
    }
}

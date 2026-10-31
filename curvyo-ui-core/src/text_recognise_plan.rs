//! What Recognise text reads from the selection and writes back (`specs/0088-trace-recognition`
//! criteria 2, 3, 6 and 7; `adrs.md` decisions 1, 6 and 11).
//!
//! The subjects are the closed outlines of the selected paths, filled or not (a closed outline is
//! read as filled), one outline per subpath, in document millimetres; images, paths without a
//! closed outline and other objects are counted and left as they are. The write turns each recognised line into a text with the style of the path it
//! replaces, placed directly in front of it, and keeps every outline that was not recognised in a
//! piece of its own path at the path's place. Pure: the session writes it in one step.

use std::collections::HashSet;

use curvyo_document_core::{
    Affine, FontRef, Length, MAX_TEXT_SIZE, MIN_TEXT_SIZE, NewAnchor, NodeId, ObjectSnapshot,
    PieceOutlines, Style, SubpathRef, TextLayoutValues,
};
use curvyo_text_core::RecognisedRun;
use kurbo::{BezPath, Point, Vec2};

/// The selected outlines Recognise text reads, and what it leaves alone.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecogniseSubjects {
    /// One closed subpath each, document millimetres.
    pub outlines: Vec<BezPath>,
    /// Per outline: its path and the subpath's index in it.
    pub sources: Vec<(NodeId, usize)>,
    /// Selected images, left as they are.
    pub images: usize,
    /// Selected paths without a closed outline, left as they are.
    pub open_paths: usize,
    /// Other selected objects (texts, clones, shapes), left as they are.
    pub others: usize,
}

/// The selected leaves of `objects`, not skipped (hidden, locked, guides), in stacking order.
fn selected_leaves<'a>(
    objects: &'a [ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> impl Iterator<Item = &'a ObjectSnapshot> {
    let wanted: HashSet<NodeId> = selected.iter().copied().collect();
    objects
        .iter()
        .filter(move |o| wanted.contains(&o.id()) && !skipped(o.id()))
}

fn readable(object: &ObjectSnapshot) -> bool {
    match object {
        ObjectSnapshot::Path(path) => path.subpaths().any(|s| s.closed),
        _ => false,
    }
}

/// Whether the Recognise text button is in the bar: some selected leaf is a path with a closed
/// outline, filled or not (UX notes §1; lead decision 2026-10-31, permit over refuse).
#[must_use]
pub fn recognise_available(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> bool {
    selected_leaves(objects, selected, skipped).any(readable)
}

/// A subpath as a Bézier path in document millimetres.
fn bez_of(subpath: &SubpathRef<'_>) -> BezPath {
    let mut path = BezPath::new();
    let at = |p: curvyo_document_core::Point| Point::new(p.x, p.y);
    let Some(first) = subpath.anchors.first() else {
        return path;
    };
    path.move_to(at(first.point));
    let count = subpath.anchors.len();
    for i in 0..count {
        let (from, to) = (&subpath.anchors[i], &subpath.anchors[(i + 1) % count]);
        let straight = from.handle_out.x == 0.0
            && from.handle_out.y == 0.0
            && to.handle_in.x == 0.0
            && to.handle_in.y == 0.0;
        if straight {
            path.line_to(at(to.point));
        } else {
            let c1 = at(from.point) + Vec2::new(from.handle_out.x, from.handle_out.y);
            let c2 = at(to.point) + Vec2::new(to.handle_in.x, to.handle_in.y);
            path.curve_to(c1, c2, at(to.point));
        }
    }
    path.close_path();
    path
}

/// The subjects of the selection (UX notes §8: what cannot be read is skipped and counted).
#[must_use]
pub fn recognise_subjects(
    objects: &[ObjectSnapshot],
    selected: &[NodeId],
    skipped: &dyn Fn(NodeId) -> bool,
) -> RecogniseSubjects {
    let mut subjects = RecogniseSubjects::default();
    for object in selected_leaves(objects, selected, skipped) {
        match object {
            ObjectSnapshot::Image(_) => subjects.images += 1,
            ObjectSnapshot::Path(path) if readable(object) => {
                for (index, subpath) in path.subpaths().enumerate() {
                    if subpath.closed && subpath.anchors.len() >= 2 {
                        subjects.outlines.push(bez_of(&subpath));
                        subjects.sources.push((path.id, index));
                    }
                }
            }
            ObjectSnapshot::Path(_) => subjects.open_paths += 1,
            _ => subjects.others += 1,
        }
    }
    subjects
}

/// One text Recognise text writes.
#[derive(Debug, Clone, PartialEq)]
pub struct RecognisedText {
    /// The index of its run in the runs planned.
    pub run: usize,
    /// The path whose style it takes and in front of which it goes.
    pub source: NodeId,
    /// That path's style.
    pub style: Style,
    /// The string.
    pub string: String,
    /// The face.
    pub font: FontRef,
    /// The cap height.
    pub size: Length,
    /// Text space to the document.
    pub placement: Affine,
    /// The letter and word spacing (times Size, as stored).
    pub layout: TextLayoutValues,
    /// The character positions of the guessed letters.
    pub guessed: Vec<usize>,
}

/// The whole write of one run, for one step.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecogniseWrite {
    /// The texts, in line order.
    pub texts: Vec<RecognisedText>,
    /// The paths that lose outlines (they are replaced).
    pub removed: Vec<NodeId>,
    /// What stays of each of them: its outlines that were not recognised, at its place.
    pub pieces: Vec<(NodeId, PieceOutlines)>,
    /// Subject outlines left as paths.
    pub left: usize,
}

/// The write for `runs` (each with its face) over `subjects`: lines whose size is out of the
/// text range stay paths.
#[must_use]
pub fn plan_recognise_write(
    objects: &[ObjectSnapshot],
    subjects: &RecogniseSubjects,
    runs: &[(RecognisedRun, FontRef)],
) -> RecogniseWrite {
    let range = MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm();
    let mut used = vec![false; subjects.outlines.len()];
    let mut write = RecogniseWrite::default();
    for (index, (run, font)) in runs.iter().enumerate() {
        let size = run.size.get();
        let Some(&first) = run.outlines.first() else {
            continue;
        };
        if !range.contains(&size) || run.outlines.iter().any(|&o| o >= used.len()) {
            continue;
        }
        let source = subjects.sources[first].0;
        let Some(object) = objects.iter().find(|o| o.id() == source) else {
            continue;
        };
        for &o in &run.outlines {
            used[o] = true;
        }
        let p = run.placement.as_coeffs();
        write.texts.push(RecognisedText {
            run: index,
            source,
            style: object.style().clone(),
            string: run.text.clone(),
            font: font.clone(),
            size: Length::from_mm(size),
            placement: Affine::new(p[0], p[1], p[2], p[3], p[4], p[5]),
            layout: TextLayoutValues {
                letter_spacing: run.letter_spacing.get() / size,
                word_spacing: run.word_spacing.get() / size,
                ..TextLayoutValues::DEFAULT
            },
            guessed: run.guessed.clone(),
        });
    }
    write.left = used.iter().filter(|u| !**u).count();
    // Every path that lost an outline is replaced by what stays of it.
    let mut touched: Vec<NodeId> = Vec::new();
    for (index, &(id, _)) in subjects.sources.iter().enumerate() {
        if used[index] && !touched.contains(&id) {
            touched.push(id);
        }
    }
    for id in touched {
        let Some(ObjectSnapshot::Path(path)) = objects.iter().find(|o| o.id() == id) else {
            continue;
        };
        let gone: HashSet<usize> = subjects
            .sources
            .iter()
            .zip(&used)
            .filter(|((source, _), u)| *source == id && **u)
            .map(|((_, subpath), _)| *subpath)
            .collect();
        let rest: PieceOutlines = path
            .subpaths()
            .enumerate()
            .filter(|(index, _)| !gone.contains(index))
            .map(|(_, s)| (s.anchors.to_vec(), s.closed))
            .collect::<Vec<(Vec<NewAnchor>, bool)>>();
        write.removed.push(id);
        if !rest.is_empty() {
            write.pieces.push((id, rest));
        }
    }
    write
}

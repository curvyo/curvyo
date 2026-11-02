//! The per-shape memo of pattern pieces (`specs/0071-pattern-fill` criteria 2, 16, 19; ADR 0029
//! §6). An entry is keyed by the frame-local outline, the pattern and the line width, compared in
//! full; a move or rotation of the shape is a hit that only maps the pieces. The memo never changes
//! a result: a miss computes exactly what the outputs compute. It lives here, not in the session,
//! because the session does not depend on the kernel (ADR 0011).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, PatternFill};
use curvyo_geometry_core::{
    PatternError, PatternFrame, PatternOutline, PatternPiece, pattern_pieces,
};

use crate::pattern_output::{PATTERN_TOLERANCE, pattern_subject};

/// What a shape's pattern came to, as the panel's state line reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternState {
    /// The shape is open: nothing is drawn (criterion 23).
    Open,
    /// This many pieces were made.
    Pieces(usize),
    /// Over the hard limit: not generated, about this many pieces (criterion 18).
    TooMany(usize),
    /// Over the work limit: not generated, whatever the pieces (criterion 18, acceptance report
    /// finding 2).
    TooDetailed,
}

#[derive(Debug, Clone, PartialEq)]
struct Key {
    local: Vec<PatternOutline>,
    fill: PatternFill,
    line_width: Length,
}

#[derive(Debug, Clone)]
struct Entry {
    key: Key,
    pieces: Arc<Result<Vec<PatternPiece>, PatternError>>,
    /// The last frame-local pieces that were made, kept over a result over the limit.
    last_ok: Option<Arc<Vec<PatternPiece>>>,
}

/// The memo: one entry per shape with a pattern.
#[derive(Debug, Default)]
pub struct PatternCache {
    entries: HashMap<NodeId, Entry>,
    /// How many times pieces were made (a miss), for the tests that show a hit.
    made: usize,
}

impl PatternCache {
    /// The pieces of `object`'s pattern in document coordinates and its state; `None` when it
    /// paints no pattern. Over the hard limit there are no pieces, unless `keep_last`: then the
    /// last pieces made for this shape are given (a field dragged past the limit shows the last
    /// valid pattern, UX notes §7); the state still says it is over.
    pub fn pieces_of(
        &mut self,
        object: &ObjectSnapshot,
        keep_last: bool,
    ) -> Option<(Vec<PatternPiece>, PatternState)> {
        let subject = pattern_subject(object)?;
        let open = subject.outlines.iter().any(|(_, closed)| !closed);
        let frame = PatternFrame::of(&subject.outlines, subject.rotation);
        let key = Key {
            local: frame.to_local(&subject.outlines),
            fill: subject.fill,
            line_width: subject.line_width,
        };
        let id = object.id();
        let (pieces, last_ok) = match self.entries.get(&id) {
            Some(entry) if entry.key == key => (Arc::clone(&entry.pieces), entry.last_ok.clone()),
            previous => {
                let earlier = previous.and_then(|entry| entry.last_ok.clone());
                self.made += 1;
                let made = Arc::new(pattern_pieces(
                    &key.local,
                    &key.fill,
                    key.line_width,
                    PATTERN_TOLERANCE,
                ));
                let last_ok = match made.as_ref() {
                    Ok(local) => Some(Arc::new(local.clone())),
                    Err(_) => earlier,
                };
                self.entries.insert(
                    id,
                    Entry {
                        key,
                        pieces: Arc::clone(&made),
                        last_ok: last_ok.clone(),
                    },
                );
                (made, last_ok)
            }
        };
        Some(match pieces.as_ref() {
            Ok(_) if open => (Vec::new(), PatternState::Open),
            Ok(local) => (frame.to_document(local), PatternState::Pieces(local.len())),
            Err(error) => {
                let kept = last_ok
                    .filter(|_| keep_last)
                    .map_or_else(Vec::new, |local| frame.to_document(&local));
                let state = match error {
                    PatternError::TooMany { estimate } => PatternState::TooMany(*estimate),
                    PatternError::TooDetailed => PatternState::TooDetailed,
                };
                (kept, state)
            }
        })
    }

    /// Drops the entries of shapes not in `ids` (ADR 0029 §6: once per read, so every part of a
    /// draw that expands patterns finds the entries the others made).
    pub fn retain_only(&mut self, ids: &HashSet<NodeId>) {
        self.entries.retain(|id, _| ids.contains(id));
    }

    /// How many times pieces were made rather than found.
    #[must_use]
    pub const fn made(&self) -> usize {
        self.made
    }
}

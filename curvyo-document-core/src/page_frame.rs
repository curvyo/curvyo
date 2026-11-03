//! The page frame register `page.<id>.frame`: a page's origin and size as one map value, always
//! written whole, so concurrent resizes converge to one whole frame (ADR 0031 decision 9a).
//!
//! Version 33 files store four registers per page instead (`x`, `y`, `width`, `height`, the first
//! page's size in the root's `size_width_mm`/`size_height_mm`); a page without a frame is read
//! from them, and nothing is rewritten until the page is next edited.

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};

use crate::document::ROOT_MAP;
use crate::document_size::{MAX_DOCUMENT_MM, MIN_DOCUMENT_MM};
use crate::units::{DocumentSize, Point};

/// The field name of the frame register after `page.<id>.`.
pub(crate) const FRAME_FIELD: &str = "frame";
const FIELDS: [&str; 4] = ["x", "y", "width", "height"];

/// A page's stored frame: its global top-left corner and its size, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub(crate) origin: Point,
    pub(crate) size: DocumentSize,
}

impl Frame {
    /// This frame resized to `size` around its centre.
    pub(crate) fn resized(self, size: DocumentSize) -> Self {
        let dx = (self.size.width.as_mm() - size.width.as_mm()) / 2.0;
        let dy = (self.size.height.as_mm() - size.height.as_mm()) / 2.0;
        Self {
            origin: Point::new(self.origin.x + dx, self.origin.y + dy),
            size,
        }
    }
}

fn frame_key(id: &str) -> String {
    format!("page.{id}.{FRAME_FIELD}")
}

/// Writes the whole frame of page `id`, without a commit.
pub(crate) fn write_frame(root: &LoroMap, id: &str, frame: Frame) {
    let map: std::collections::HashMap<String, LoroValue> = FIELDS
        .iter()
        .zip([
            frame.origin.x,
            frame.origin.y,
            frame.size.width.as_mm(),
            frame.size.height.as_mm(),
        ])
        .map(|(field, value)| ((*field).to_owned(), LoroValue::Double(value)))
        .collect();
    // invariant: attached root map, a plain map value.
    #[allow(clippy::unwrap_used)]
    root.insert(&frame_key(id), LoroValue::Map(map.into()))
        .unwrap();
}

/// The frame register of page `id` when it holds a valid frame; `None` when there is none (a
/// version 33 page) or it is damaged.
pub(crate) fn read_frame(root: &LoroMap, id: &str) -> Option<Frame> {
    match root.get(&frame_key(id))? {
        ValueOrContainer::Value(value) => parse(&value),
        ValueOrContainer::Container(_) => None,
    }
}

/// A map of four finite numbers with both sides in the document size range.
fn parse(value: &LoroValue) -> Option<Frame> {
    let LoroValue::Map(map) = value else {
        return None;
    };
    if map.len() != FIELDS.len() {
        return None;
    }
    let number = |field: &str| match map.get(field)? {
        LoroValue::Double(n) if n.is_finite() => Some(*n),
        _ => None,
    };
    let side =
        |field: &str| number(field).filter(|mm| (MIN_DOCUMENT_MM..=MAX_DOCUMENT_MM).contains(mm));
    Some(Frame {
        origin: Point::new(number("x")?, number("y")?),
        size: DocumentSize::from_mm(side("width")?, side("height")?),
    })
}

/// Whether every frame register of `loro` is well formed (the damage rule checked on open).
pub(crate) fn validate(loro: &LoroDoc) -> bool {
    let mut valid = true;
    loro.get_map(ROOT_MAP).for_each(|key, value| {
        let is_frame = key
            .strip_prefix("page.")
            .and_then(|rest| rest.rsplit_once('.'))
            .is_some_and(|(_, field)| field == FRAME_FIELD);
        if is_frame {
            valid &= matches!(value, ValueOrContainer::Value(v) if parse(&v).is_some());
        }
    });
    valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_round_trips_and_resizes_around_its_centre() {
        let loro = LoroDoc::new();
        let root = loro.get_map(ROOT_MAP);
        let frame = Frame {
            origin: Point::new(-3.0, 4.5),
            size: DocumentSize::from_mm(100.0, 50.0),
        };
        assert_eq!(read_frame(&root, "p"), None);
        write_frame(&root, "p", frame);
        assert_eq!(read_frame(&root, "p"), Some(frame));
        assert!(validate(&loro));
        let resized = frame.resized(DocumentSize::from_mm(120.0, 40.0));
        assert_eq!(resized.origin, Point::new(-13.0, 9.5));
    }

    #[test]
    fn a_frame_with_an_extra_field_is_damaged() {
        let loro = LoroDoc::new();
        let root = loro.get_map(ROOT_MAP);
        let map: std::collections::HashMap<String, LoroValue> =
            ["x", "y", "width", "height", "depth"]
                .iter()
                .map(|f| ((*f).to_owned(), LoroValue::Double(10.0)))
                .collect();
        root.insert("page.p.frame", LoroValue::Map(map.into()))
            .unwrap();
        assert_eq!(read_frame(&root, "p"), None);
        assert!(!validate(&loro));
    }
}

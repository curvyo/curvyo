//! The stored shape of an image object (ADR 0017 §1), read, written and validated here.
//!
//! ```text
//! meta map of an image:
//!   shape           : "image"
//!   image_asset     : "<64 lowercase hex>"   SHA-256 of the asset bytes, written once
//!   image_placement : [a, b, c, d, e, f]     the unit square to document mm, finite, within
//!                                            ±1e7; a singular map is allowed and draws nothing
//!   + `name`, `visible`, `locked`, `opacity`; no style keys, no outline
//!   + the trace mask `trace_*` (`trace_mask_codec.rs`, ADR 0061)
//! ```

use loro::{LoroMap, LoroValue, ValueOrContainer};

use crate::affine::Affine;
use crate::assets::AssetHash;
use crate::image_model::ImageSnapshot;
use crate::path_codec::as_f64;
use crate::path_model::NodeId;

/// The `shape` tag of an image node.
pub(crate) const SHAPE_IMAGE: &str = "image";
const KEY_ASSET: &str = "image_asset";
const KEY_PLACEMENT: &str = "image_placement";

/// Whether `meta` is an image node's.
pub(crate) fn is_image_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_IMAGE
    )
}

/// Writes an image node's registers. Writing an asset other than the stored one removes the
/// node's trace mask in the same commit, so a mask never moves silently onto other pixels
/// (ADR 0061 §8, Frage 5 ASSET-SWAP).
pub(crate) fn write_image(meta: &LoroMap, asset: &AssetHash, placement: Affine) {
    if read_asset(meta).is_some_and(|stored| stored != *asset) {
        crate::trace_mask_codec::remove_all(meta);
    }
    crate::shape_codec::write_shape_tag(meta, SHAPE_IMAGE);
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_ASSET, asset.as_str()).unwrap();
    write_placement(meta, placement);
}

/// Writes the placement register.
pub(crate) fn write_placement(meta: &LoroMap, placement: Affine) {
    // invariant: see `write_image`.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_PLACEMENT, placement.to_array().to_vec())
        .unwrap();
}

fn read_asset(meta: &LoroMap) -> Option<AssetHash> {
    match meta.get(KEY_ASSET) {
        Some(ValueOrContainer::Value(LoroValue::String(text))) => AssetHash::parse(&text),
        _ => None,
    }
}

/// The placement register when it holds six finite numbers within the matrix limit.
pub(crate) fn read_placement(meta: &LoroMap) -> Option<Affine> {
    let Some(ValueOrContainer::Value(LoroValue::List(list))) = meta.get(KEY_PLACEMENT) else {
        return None;
    };
    if list.len() != 6 {
        return None;
    }
    let mut entries = [0.0; 6];
    for (entry, value) in entries.iter_mut().zip(list.iter()) {
        *entry = as_f64(value)?;
    }
    let placement = Affine::from_array(entries);
    placement.is_within_limit().then_some(placement)
}

/// The image `id` as stored, or `None` when a register cannot be read.
pub(crate) fn read_image_snapshot(id: NodeId, meta: &LoroMap) -> Option<ImageSnapshot> {
    Some(ImageSnapshot::new(
        id,
        read_asset(meta)?,
        read_placement(meta)?,
    ))
}

/// Whether an image node's registers can be read (open-file validation).
pub(crate) fn validate_image_node(meta: &LoroMap) -> bool {
    read_asset(meta).is_some() && read_placement(meta).is_some()
}

/// Moves an image node's frame by `offset` in place.
pub(crate) fn translate_image_meta(meta: &LoroMap, offset: crate::units::Vec2) {
    if let Some(placement) = read_placement(meta) {
        write_placement(meta, Affine::translation(offset).compose(placement));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_mask_codec::{read_trace_mask, write_part, write_region};
    use crate::trace_mask_model::{NewTraceRegion, TraceRegionKind, UnitPoint};

    /// ADR 0061 §8 (Frage 5 ASSET-SWAP): writing another asset drops the mask; writing the same
    /// asset again (a moved tiling instance) keeps it.
    #[test]
    fn a_new_asset_drops_the_trace_mask_and_the_same_asset_keeps_it() {
        let doc = loro::LoroDoc::new();
        let meta = doc.get_map("meta");
        let placement = Affine::new(10.0, 0.0, 0.0, 10.0, 0.0, 0.0);
        write_image(&meta, &AssetHash::of(b"a"), placement);
        write_part(&meta, "0", 0, false);
        let region = NewTraceRegion {
            kind: TraceRegionKind::Rect,
            subtract: false,
            points: vec![UnitPoint::new(0.0, 0.0), UnitPoint::new(1.0, 1.0)],
        };
        write_region(&meta, "0", "1@1", 0, &region);
        write_image(
            &meta,
            &AssetHash::of(b"a"),
            Affine::translation(crate::units::Vec2::new(1.0, 0.0)),
        );
        assert_eq!(read_trace_mask(&meta).parts[0].regions.len(), 1);
        write_image(&meta, &AssetHash::of(b"b"), placement);
        assert!(read_trace_mask(&meta).is_empty());
        assert_eq!(read_asset(&meta), Some(AssetHash::of(b"b")));
    }
}

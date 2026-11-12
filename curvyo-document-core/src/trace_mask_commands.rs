//! The trace mask commands of one image, one commit each (ADR 0061 §3, §4, §6): draw, change and
//! delete a region, Clear mask, Inside or Outside, Add part, Remove part and a part's settings.

use loro::LoroMap;

use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::trace_mask_codec::{
    FIRST_TRACE_PART, dangling_keys, delete, find_region_key, highest_orders, part_key,
    read_trace_mask, region_keys_of, write_part, write_region, write_settings,
};
use crate::trace_mask_model::{
    ADD_TRACE_PART_LABEL, CLEAR_TRACE_MASK_LABEL, DELETE_TRACE_REGION_LABEL,
    EDIT_TRACE_SETTINGS_LABEL, ImplicitTracePart, NewTraceRegion, REMOVE_TRACE_PART_LABEL,
    TRACE_MASK_INSIDE_LABEL, TRACE_MASK_OUTSIDE_LABEL, TraceMask, TraceMaskError, TracePartRef,
    TracePartSettings, TraceRegionEdit, UnitPoint,
};

impl Document {
    /// The trace mask stored on the image `image` (empty for an untouched one); `None` when
    /// `image` is not a live image.
    #[must_use]
    pub fn trace_mask(&self, image: NodeId) -> Option<TraceMask> {
        self.image_meta(image)
            .ok()
            .map(|meta| read_trace_mask(&meta))
    }

    /// Draws `region` into `part` of `image` in one step "Draw rectangle" or "Draw lasso"; the
    /// implicit part is stored as part `0` in the same step. Returns the new region's id.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchPart`] for a stored part that is
    /// gone, [`TraceMaskError::InvalidPoints`] for points that do not fit the kind.
    pub fn draw_trace_region(
        &self,
        image: NodeId,
        part: TracePartRef<'_>,
        region: &NewTraceRegion,
    ) -> Result<String, TraceMaskError> {
        let meta = self.image_meta(image)?;
        check_points(region)?;
        let id = self.new_trace_key_id();
        let part = store_part(&meta, part)?;
        let (_, regions) = highest_orders(&meta);
        write_region(&meta, &part, &id, regions.map_or(0, |o| o + 1), region);
        self.commit_with_label(region.kind.draw_label());
        Ok(id)
    }

    /// Replaces the points of the region `region` of `image` (a move, a resize or a moved lasso
    /// corner, `edit` naming the step); its kind, order and Subtract stay. `Ok(false)` and no step
    /// when the points are unchanged.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchRegion`],
    /// [`TraceMaskError::InvalidPoints`].
    pub fn set_trace_region_points(
        &self,
        image: NodeId,
        region: &str,
        points: &[UnitPoint],
        edit: TraceRegionEdit,
    ) -> Result<bool, TraceMaskError> {
        let meta = self.image_meta(image)?;
        let mask = read_trace_mask(&meta);
        let (part, stored) = mask.region(region).ok_or(TraceMaskError::NoSuchRegion)?;
        let changed = NewTraceRegion {
            kind: stored.kind,
            subtract: stored.subtract,
            points: points.to_vec(),
        };
        check_points(&changed)?;
        if stored.points == changed.points {
            return Ok(false);
        }
        write_region(&meta, &part.id, region, stored.order, &changed);
        self.commit_with_label(edit.label());
        Ok(true)
    }

    /// Deletes the region `region` of `image` in one step "Delete region", with any dangling keys.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchRegion`].
    pub fn delete_trace_region(&self, image: NodeId, region: &str) -> Result<(), TraceMaskError> {
        let meta = self.image_meta(image)?;
        let key = find_region_key(&meta, region).ok_or(TraceMaskError::NoSuchRegion)?;
        delete(&meta, &key);
        sweep_dangling(&meta);
        self.commit_with_label(DELETE_TRACE_REGION_LABEL);
        Ok(())
    }

    /// Clear mask (criterion 20): every region of `part` goes in one step "Clear mask"; the part
    /// and its settings stay. `Ok(false)` and no step when it has none.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchPart`].
    pub fn clear_trace_mask(&self, image: NodeId, part: &str) -> Result<bool, TraceMaskError> {
        let meta = self.image_meta(image)?;
        require_part(&meta, part)?;
        let keys = region_keys_of(&meta, part);
        let dangling = dangling_keys(&meta);
        if keys.is_empty() && dangling.is_empty() {
            return Ok(false);
        }
        for key in keys.iter().chain(&dangling) {
            delete(&meta, key);
        }
        self.commit_with_label(CLEAR_TRACE_MASK_LABEL);
        Ok(!keys.is_empty())
    }

    /// Inside (`false`) or Outside for the stored part `part`, one step "Mask: Inside" or
    /// "Mask: Outside"; `Ok(false)` and no step when it already is.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchPart`].
    pub fn set_trace_part_outside(
        &self,
        image: NodeId,
        part: &str,
        outside: bool,
    ) -> Result<bool, TraceMaskError> {
        let meta = self.image_meta(image)?;
        require_part(&meta, part)?;
        let mask = read_trace_mask(&meta);
        let stored = mask.part(part).ok_or(TraceMaskError::NoSuchPart)?;
        if stored.outside == outside {
            return Ok(false);
        }
        write_part(&meta, part, stored.order, outside);
        self.commit_with_label(if outside {
            TRACE_MASK_OUTSIDE_LABEL
        } else {
            TRACE_MASK_INSIDE_LABEL
        });
        Ok(true)
    }

    /// Add part: a new last part of `image`, Inside, with `settings`, in one step "Add part"; an
    /// untouched image stores its `implicit` part as part `0` first. Returns the new part's id.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`].
    pub fn add_trace_part(
        &self,
        image: NodeId,
        implicit: &ImplicitTracePart,
        settings: &TracePartSettings,
    ) -> Result<String, TraceMaskError> {
        let meta = self.image_meta(image)?;
        let id = self.new_trace_key_id();
        store_part(&meta, TracePartRef::Implicit(implicit))?;
        let (parts, _) = highest_orders(&meta);
        write_part(&meta, &id, parts.map_or(0, |o| o + 1), false);
        write_settings(&meta, &id, settings);
        self.commit_with_label(ADD_TRACE_PART_LABEL);
        Ok(id)
    }

    /// Remove part: `part` with its settings and regions, and any dangling keys, in one step
    /// "Remove part".
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchPart`],
    /// [`TraceMaskError::LastPart`] when it is the image's only part.
    pub fn remove_trace_part(&self, image: NodeId, part: &str) -> Result<(), TraceMaskError> {
        let meta = self.image_meta(image)?;
        require_part(&meta, part)?;
        if read_trace_mask(&meta).parts.len() < 2 {
            return Err(TraceMaskError::LastPart);
        }
        delete(&meta, &part_key(part));
        sweep_dangling(&meta);
        self.commit_with_label(REMOVE_TRACE_PART_LABEL);
        Ok(())
    }

    /// Writes the settings of the stored part `part` whole, one step "Change trace settings";
    /// `Ok(false)` and no step when they are unchanged.
    ///
    /// # Errors
    /// [`TraceMaskError::NotAnImage`], [`TraceMaskError::NoSuchPart`].
    pub fn set_trace_part_settings(
        &self,
        image: NodeId,
        part: &str,
        settings: &TracePartSettings,
    ) -> Result<bool, TraceMaskError> {
        let meta = self.image_meta(image)?;
        require_part(&meta, part)?;
        let mask = read_trace_mask(&meta);
        if mask.part(part).is_some_and(|p| p.settings == *settings) {
            return Ok(false);
        }
        write_settings(&meta, part, settings);
        self.commit_with_label(EDIT_TRACE_SETTINGS_LABEL);
        Ok(true)
    }

    /// The meta map of the live image `image`.
    fn image_meta(&self, image: NodeId) -> Result<LoroMap, TraceMaskError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(image);
        if !node_exists(&tree, node)
            || crate::manufacturing_codec::is_manufacturing_node(&tree, node)
        {
            return Err(TraceMaskError::NotAnImage);
        }
        let meta = tree
            .get_meta(node)
            .map_err(|_| TraceMaskError::NotAnImage)?;
        if crate::image_codec::is_image_meta(&meta) {
            Ok(meta)
        } else {
            Err(TraceMaskError::NotAnImage)
        }
    }

    /// A new part or region id (ADR 0061 §1): `<counter>@<peer>` of the next operation of this
    /// peer, so it is unique across peers and never reused.
    fn new_trace_key_id(&self) -> String {
        let peer = self.loro().peer_id();
        let counter = self.loro().oplog_vv().get(&peer).copied().unwrap_or(0);
        let pending = i32::try_from(self.loro().get_pending_txn_len()).unwrap_or(i32::MAX);
        format!("{}@{peer}", counter.saturating_add(pending))
    }
}

/// The stored id of `part`, storing the implicit part as part `0` when no part is stored yet.
fn store_part(meta: &LoroMap, part: TracePartRef<'_>) -> Result<String, TraceMaskError> {
    match part {
        TracePartRef::Stored(id) => {
            require_part(meta, id)?;
            Ok(id.to_owned())
        }
        TracePartRef::Implicit(implicit) => {
            let (parts, _) = highest_orders(meta);
            if parts.is_none() {
                write_part(meta, FIRST_TRACE_PART, 0, implicit.outside);
                write_settings(meta, FIRST_TRACE_PART, &implicit.settings);
                return Ok(FIRST_TRACE_PART.to_owned());
            }
            // A peer stored parts meanwhile: the first of them takes the gesture.
            read_trace_mask(meta)
                .parts
                .first()
                .map(|p| p.id.clone())
                .ok_or(TraceMaskError::NoSuchPart)
        }
    }
}

fn require_part(meta: &LoroMap, part: &str) -> Result<(), TraceMaskError> {
    if meta.get(&part_key(part)).is_some() {
        Ok(())
    } else {
        Err(TraceMaskError::NoSuchPart)
    }
}

fn check_points(region: &NewTraceRegion) -> Result<(), TraceMaskError> {
    if region.kind.accepts(region.points.len()) && region.points.iter().all(|p| p.is_storable()) {
        Ok(())
    } else {
        Err(TraceMaskError::InvalidPoints)
    }
}

fn sweep_dangling(meta: &LoroMap) {
    for key in dangling_keys(meta) {
        delete(meta, &key);
    }
}

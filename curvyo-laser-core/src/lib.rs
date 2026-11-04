//! The laser workbench extension (ADR 0034 §1): its manifest, the outlines of its fixtures, the
//! text of its notices and its machine profiles (ADR 0038, ADR 0039). It receives document data
//! and record texts as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod engraved_grid;
mod machine_profile;
mod machine_profile_copy;
mod machine_profile_file;
mod machine_profiles;
mod manifest;
mod overhang;

pub use engraved_grid::{ENGRAVED_GRID, engraved_grid};
pub use machine_profile::{
    BaudRate, Connection, FeedRate, GENERIC_LASER_ID, GENERIC_LASER_NAME, Limited,
    MAX_CONNECTION_CHARS, MAX_NAME_CHARS, MAX_WORK_MM, MIN_WORK_MM, MachineProfile, OriginCorner,
    PowerValue, ProfileValues, clamp_address, clamp_baud, clamp_feed, clamp_name, clamp_port,
    clamp_power, clamp_work_area, clamp_work_side, grouped, is_profile_id, new_profile_id,
    save_as_name,
};
pub use machine_profile_copy::{
    CopyRead, DAMAGED_NAME_NOTICE, ProfileCopy, differs, read_copy, write_copy,
};
pub use machine_profile_file::{
    MAX_RECORD_BYTES, RECORD_SUFFIX, RecordError, read_choice, read_record, record_file_name,
    record_id, write_choice, write_record,
};
pub use machine_profiles::{LibraryWrite, MAX_RECORDS, MachineProfiles};
pub use manifest::{PROCESS_ID, manifest};
pub use overhang::{OVERHANG_NOTICE, OVERHANG_TOLERANCE, overhang_text};

use curvyo_document_core::DocumentSize;
use curvyo_extension_core::{FixtureDrawing, NoticeContext, StoredParams};

/// The outline of a fixture of kind `kind` (the part after `curvyo.laser/`) with the stored
/// values `params` over a work area; an unknown kind draws nothing.
#[must_use]
pub fn fixture_drawing(
    kind: &str,
    params: &StoredParams,
    work_area: DocumentSize,
) -> FixtureDrawing {
    match kind {
        ENGRAVED_GRID => engraved_grid(params, work_area),
        _ => FixtureDrawing::empty(),
    }
}

/// The text of the notice `id`, `None` while it does not apply or for an unknown id.
#[must_use]
pub fn notice(id: &str, context: &NoticeContext) -> Option<String> {
    match id {
        OVERHANG_NOTICE => overhang_text(context),
        _ => None,
    }
}

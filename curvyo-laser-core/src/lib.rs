//! The laser workbench extension (ADR 0034 §1): its manifest, the outlines of its fixtures, the
//! text of its notices, its machine profiles (ADR 0038, ADR 0039) and the rules of its operations
//! (ADR 0046 §5). It receives document data and record texts as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod fixture_fit;
mod fixture_kind;
mod fixture_slot;
mod fixture_snap;
mod fixture_stored;
mod grid_layout;
mod machine_profile;
mod machine_profile_copy;
mod machine_profile_file;
mod machine_profile_fixtures;
mod machine_profiles;
mod manifest;
mod operation_cut_out;
mod operation_kind;
mod operation_kind_run;
mod operation_naming;
mod operation_nearest;
mod operation_sequence;
mod operation_summary;
mod overhang;

/// The laser operations of `specs/0101-laser-operations` as pure rules over stored records
/// (ADR 0046 §5).
pub mod operations {
    pub use crate::operation_cut_out::{CutOutError, cut_out_contours};
    pub use crate::operation_kind::{KindRead, OperationKind, Section};
    pub use crate::operation_kind_run::{KindRun, LeafShape, Lines, kind_run};
    pub use crate::operation_naming::next_operation_name;
    pub use crate::operation_nearest::{Assignment, ChainScope, OperationList, OperationRule};
    pub use crate::operation_sequence::{cuts_last, insert_index, may_move, run_sequence};
    pub use crate::operation_summary::{Hidden, Leaf, LeafStatus, Summary, leaf_status, summarize};
}

pub use fixture_fit::{FitMatch, fit_slot};
pub use fixture_kind::{
    FixtureKind, KIND_PREFIX, MAJOR_EVERY_RANGE, MmRange, ScrewPart, SlotShape, StopPart,
    board_side_range, grid_spacing_range, origin_offset_range, screw_diameters, screw_head_range,
    screw_thread_range, slot_size_range, stop_leg_range, stop_legs, stop_thickness_range,
};
pub use fixture_slot::Slot;
pub use fixture_snap::{Placement, board_points, stop_points};
pub use fixture_stored::{SLOT_CIRCLE, SLOT_RECT, grid_spec, params_damage, read_slot, screw_spec};
pub use grid_layout::{
    GridLayout, GridSpec, Lattice, Number, Tape, TapeEdge, TapeEdges, Tick, layout,
};
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
pub use machine_profile_fixtures::{MAX_SET_FIXTURES, MAX_SET_SLOTS};
pub use machine_profiles::{LibraryWrite, MAX_RECORDS, MachineProfiles};
pub use manifest::{ENGRAVED_GRID, PROCESS_ID, manifest};
pub use overhang::{OVERHANG_NOTICE, OVERHANG_TOLERANCE, overhang_text};

use curvyo_extension_core::NoticeContext;

/// The text of the notice `id`, `None` while it does not apply or for an unknown id.
#[must_use]
pub fn notice(id: &str, context: &NoticeContext) -> Option<String> {
    match id {
        OVERHANG_NOTICE => overhang_text(context),
        _ => None,
    }
}

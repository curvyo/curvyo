//! The laser workbench extension (ADR 0034 §1): its manifest, the outlines of its fixtures, the
//! text of its notices, its machine profiles (ADR 0038, ADR 0039) and the rules of its operations
//! (ADR 0046 §5) with their settings and test grids (ADR 0054). It receives document data and
//! record texts as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod fixture_fit;
mod fixture_kind;
mod fixture_part;
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
mod material;
mod material_edits;
mod material_file;
mod material_preferences;
mod materials;
mod operation_cut_out;
mod operation_group;
mod operation_kind;
mod operation_kind_run;
mod operation_naming;
mod operation_nearest;
mod operation_options;
mod operation_sequence;
mod operation_settings;
mod operation_settings_rule;
mod operation_summary;
mod overhang;
mod record_envelope;
mod steps;
mod test_grid;
mod test_grid_layout;
mod test_grid_run;

/// The laser operations of `specs/0101-laser-operations` as pure rules over stored records
/// (ADR 0046 §5).
pub mod operations {
    pub use crate::operation_cut_out::{CutOutError, cut_out_contours};
    pub use crate::operation_group::{
        GroupTick, group_assign_keys, group_tick, group_unassign_keys,
    };
    pub use crate::operation_kind::{KindRead, OperationKind, Section};
    pub use crate::operation_kind_run::{KindRun, LeafShape, Lines, kind_run};
    pub use crate::operation_naming::{last_of_kind, next_operation_name, next_test_grid_name};
    pub use crate::operation_nearest::{Assignment, ChainScope, OperationList, OperationRule};
    pub use crate::operation_sequence::{
        FixtureJobRead, cuts_last, fixture_block, insert_index, may_move, run_sequence,
    };
    pub use crate::operation_summary::{Hidden, Leaf, LeafStatus, Summary, leaf_status, summarize};
}

/// The settings of operations and the in-memory material model of
/// `specs/0028-material-test-library` (ADR 0054 §1 to §3; ADR 0055 §2), with the operation-only
/// settings of `specs/0111-laser-operation-tools` (ADR 0056 §5, §6).
pub mod settings {
    pub use crate::material::{
        CellIndex, HistoryEntry, HistoryEvent, Interval, MAX_HISTORY_ENTRIES,
        MAX_MATERIAL_NAME_CHARS, MachineTie, Material, MaterialRead, MaterialSet, Passes,
        PowerPercent, SetKind, SetSource, Settings, Thickness, next_modified,
    };
    pub use crate::operation_options::{
        ENTER_NUMBER, ENTER_WHOLE_NUMBER, OperationOptions, OptionEdit, OptionsDamage, Overscan,
        ScanAngle, ZOffset, hatches, read_options,
    };
    pub use crate::operation_settings::{
        Field, MaterialLink, Overrides, Role, SetOverride, SettingsDamage, SettingsRead,
        StoredSettings, read_settings, set_value,
    };
    pub use crate::operation_settings_rule::{
        RunSettings, SettingsState, StoredReason, TypedNumber, edit_interval, edit_number,
        main_set, refresh, reset_to_material, resolve, use_material, use_own_numbers,
    };
}

/// The maker's material library of `specs/0028-material-test-library`: its files and the list
/// (ADR 0055).
pub mod material_library {
    pub use crate::material_edits::{
        LibraryRefusal, NewMaterial, RevertRefused, Reverted, SaveTarget, TestResult,
    };
    pub use crate::material_file::{
        MATERIAL_FORMAT_VERSION, MAX_MATERIAL_BYTES, MaterialFileError, read_material,
        write_material,
    };
    pub use crate::material_preferences::{
        MAX_GRID_CHOICE_MACHINES, RememberedGrid, read_grid_choice, read_order, remember_grid,
        write_grid_choice, write_order,
    };
    pub use crate::materials::{MAX_MATERIALS, Materials};
}

/// The test grid of `specs/0028-material-test-library`: recipe, layout and run rule (ADR 0054
/// §4).
pub mod test_grids {
    pub use crate::test_grid::{
        GridField, GridLabels, GridRange, GridRecipe, MAX_CELL_MM, MAX_CELLS, MAX_GAP_MM,
        MAX_LABEL_MM, MIN_CELL_MM, MIN_CELLS, MIN_LABEL_MM, mark_value, range_of, range_value,
        read_mark,
    };
    pub use crate::test_grid_layout::{
        BuiltCell, BuiltLabel, CellPlace, GRID_TOLERANCE_MM, LabelLanes, LabelPlace, LabelTag,
        TestGridLayout, UNIT_TEXT, fit_bed, free_spot, hand_edited, kept_mark, layout,
        no_free_spot_text, speed_texts,
    };
    pub use crate::test_grid_run::{CellTag, GridLeafRun, GridTag, grid_leaf_run, grid_run_order};
}

pub use fixture_fit::{FitMatch, fit_slot};
pub use fixture_kind::{
    FixtureKind, KIND_PREFIX, MAJOR_EVERY_RANGE, MmRange, ScrewPart, SlotShape, StopPart,
    board_side_range, grid_spacing_range, origin_offset_range, screw_diameters, screw_head_range,
    screw_thread_range, slot_size_range, stop_leg_range, stop_legs, stop_thickness_range,
};
pub use fixture_part::{FixturePart, job_parts};
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
    clamp_power, clamp_work_area, clamp_work_side, grouped, save_as_name,
};
pub use machine_profile_copy::{
    CopyRead, DAMAGED_NAME_NOTICE, ProfileCopy, differs, read_copy, write_copy,
};
pub use machine_profile_file::{
    MAX_RECORD_BYTES, RecordError, read_choice, read_record, write_choice, write_record,
};
pub use machine_profile_fixtures::{
    MAX_SET_FIXTURES, MAX_SET_JOBS, MAX_SET_SLOTS, MAX_SET_TARGETS,
};
pub use machine_profiles::{MAX_RECORDS, MachineProfiles};
pub use manifest::{ENGRAVED_GRID, PROCESS_ID, manifest};
pub use overhang::{OVERHANG_NOTICE, OVERHANG_TOLERANCE, overhang_text};
pub use record_envelope::{
    LibraryWrite, RECORD_SUFFIX, is_record_id, new_record_id, record_file_name, record_id,
};
/// The profile names of [`is_record_id`] and [`new_record_id`] (ADR 0038 §5), kept for their
/// callers.
pub use record_envelope::{is_record_id as is_profile_id, new_record_id as new_profile_id};

use curvyo_extension_core::NoticeContext;

/// The text of the notice `id`, `None` while it does not apply or for an unknown id.
#[must_use]
pub fn notice(id: &str, context: &NoticeContext) -> Option<String> {
    match id {
        OVERHANG_NOTICE => overhang_text(context),
        _ => None,
    }
}

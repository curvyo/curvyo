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
mod material_photo;
mod material_preferences;
mod material_record;
mod material_record_edits;
mod material_record_file;
mod material_retired;
mod material_store;
mod material_test_file;
mod operation_cut_out;
mod operation_group;
mod operation_kind;
mod operation_kind_run;
mod operation_naming;
mod operation_nearest;
mod operation_options;
mod operation_sequence;
mod operation_settings;
mod operation_settings_edit;
mod operation_settings_rule;
mod operation_summary;
mod overhang;
mod record_envelope;
mod steps;
mod test_grid_choice;
mod test_grid_fit;
mod test_grid_ops;
mod test_id;
mod test_param;
mod test_recipe;
mod test_recipe_json;
mod test_recipe_layout;

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

/// The settings of operations and the numbers they share with material records (ADR 0054 §1 to
/// §3; ADR 0055 §2; ADR 0058 §2), with the operation-only settings of
/// `specs/0111-laser-operation-tools` (ADR 0056 §5, §6).
pub mod settings {
    pub use crate::material::{
        CellIndex, Interval, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS, MachineTie, Passes,
        PowerPercent, SetKind, SetNumbers, Settings, Thickness, next_modified,
    };
    pub use crate::operation_options::{
        ENTER_NUMBER, ENTER_WHOLE_NUMBER, OperationOptions, OptionEdit, OptionsDamage, Overscan,
        ScanAngle, ZOffset, hatches, read_options,
    };
    pub use crate::operation_settings::{
        Field, MaterialLink, Overrides, Role, SetOverride, SettingsDamage, SettingsRead,
        StoredSettings, read_settings, set_value,
    };
    pub use crate::operation_settings_edit::{TypedNumber, edit_interval, edit_number};
    pub use crate::operation_settings_rule::{
        RunSettings, SettingsState, StoredReason, main_set, refresh, reset_to_material, resolve,
        use_material, use_own_numbers,
    };
}

/// The test grid object of `specs/0112-test-grid-and-materials`: test ids, the recipe, its layout
/// and the derived operations it resolves into (ADR 0057).
pub mod grids {
    pub use crate::test_grid_choice::{
        GRID_CHOICE_FORMAT_VERSION, read_test_grid_choice, remember_test_grid,
        write_test_grid_choice,
    };
    pub use crate::test_grid_fit::{
        GRID_TOLERANCE_MM, fit_bed, fit_bed_at, free_spot, no_free_spot_text,
    };
    pub use crate::test_grid_ops::{DerivedOperation, GridPart, LabelSet, derived_operations};
    pub use crate::test_id::{TEST_ID_CHARS, TestId};
    pub use crate::test_param::{Param, ParamValue};
    pub use crate::test_recipe::{
        Axis, MAX_CELL, MAX_COUNT, MIN_CELL, MIN_COUNT, MaterialRef, Recipe, RecipeDefaults,
        RecipeField, Side, TestRecipe,
    };
    pub use crate::test_recipe_layout::{
        GAP_MM, LabelPart, LaidCell, LaidLabel, TEXT_MM, TestLayout, layout,
    };
}

/// The maker's material library of `specs/0112-test-grid-and-materials`: record format 2 shared
/// by all lasers, its tests, retired ids, photos and the list with its order (ADR 0055, ADR 0058).
pub mod library {
    pub use crate::material_photo::{
        MAX_PHOTO_BYTES, MAX_PHOTO_SIDE, PhotoHeader, PhotoRefusal, PhotoWrite, check_photo,
        remove_photo, write_photo,
    };
    pub use crate::material_preferences::{read_order, write_order};
    pub use crate::material_record::{
        HistoryEntry, HistoryEvent, LaserSets, MAX_LASERS, MAX_NOTE_CHARS, MAX_TESTS, Material,
        MaterialRead, PhotoKind, Set, SetSource, TestRecord, live_test_ids, no_laser_set_text,
        photo_file_name,
    };
    pub use crate::material_record_edits::{RecordRefusal, Stamp};
    pub use crate::material_record_file::{
        MAX_RECORD_TEXT, RECORD_FORMAT_VERSION, RecordFileError, load_record, read_record,
        write_record,
    };
    pub use crate::material_retired::{
        MAX_RETIRED, RETIRED_DAMAGED_NOTICE, RETIRED_FORMAT_VERSION, RetireRefused, RetiredIds,
        read_retired, write_retired,
    };
    pub use crate::material_store::{
        Library, MAX_MATERIALS, RevertRefused, Reverted, StoreRefusal,
    };
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

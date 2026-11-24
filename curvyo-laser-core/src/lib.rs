//! The laser workbench extension (ADR 0034 §1): its manifest, the outlines of its fixtures, the
//! text of its notices, its machine profiles (ADR 0038, ADR 0039) and the rules of its operations
//! (ADR 0046 §5) with their settings and test grids (ADR 0054), and the Print and cut reading of
//! a placed sheet (ADR 0081 §5). It receives document data and record texts as arguments and
//! never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod fixture_fit;
mod fixture_grid;
mod fixture_kind;
mod fixture_marks;
mod fixture_part;
mod fixture_sets;
mod fixture_slot;
mod fixture_snap;
mod fixture_stored;
mod grid_layout;
mod image_adjust_ai;
mod image_adjust_cache;
mod image_adjust_field_specs;
mod image_adjust_fields;
mod image_adjust_kinds;
mod image_adjust_resolve;
mod image_adjust_stored;
mod image_dither;
mod image_preset;
mod job_anchor;
mod job_build;
mod job_effects;
mod job_fingerprint;
mod job_flatten;
mod job_frame;
mod job_gate;
mod job_gate_cause;
mod job_gate_raster;
mod job_gate_rows;
mod job_gate_text;
mod job_hatch;
mod job_input;
mod job_moves;
mod job_numbers;
mod job_order;
mod job_plan;
mod job_raster_dither;
mod job_raster_grid;
mod job_raster_items;
mod job_raster_plan;
mod job_raster_rows;
mod job_raster_sample;
mod job_raster_screen;
mod job_raster_tone;
mod job_raster_work;
mod job_timeline;
mod job_write;
mod job_write_frame;
mod job_write_grbl;
mod library_merge;
mod machine_profile;
mod machine_profile_copy;
mod machine_profile_file;
mod machine_profile_fixtures;
mod machine_profile_sets;
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
mod operation_image_options;
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
mod output_settings;
mod overhang;
#[path = "print_and_cut.rs"]
mod print_and_cut_reading;
#[path = "print_and_cut_text.rs"]
mod print_and_cut_words;
mod record_envelope;
mod steps;
mod test_cell_fill;
mod test_grid_choice;
mod test_grid_fit;
mod test_grid_ops;
mod test_id;
mod test_param;
mod test_recipe;
mod test_recipe_json;
mod test_recipe_layout;

/// The adjustments of a picture before its dots are made (`specs/0135-image-optimisation`, ADR
/// 0073): the stored steps, the presets with their Strength, and the adjusted image at an
/// Interval.
pub mod adjust {
    pub use crate::image_adjust_ai::{AiInput, ModelTag, ResultRef};
    pub use crate::image_adjust_cache::{
        ADJUST_WORK, ADJUSTED_BYTES, AdjustKey, AdjustedPicture, LOOK_BYTES, PyramidCache,
        RUN_BYTES, Resolved, reduced_level,
    };
    pub use crate::image_adjust_fields::{
        Band, CHANNEL_PRESETS, FieldSpec, LUMINANCE_MIX, MAX_CURVE_POINTS, ManualStep, StepDamage,
        StepKind,
    };
    pub use crate::image_adjust_resolve::{
        AdjustRegisters, DEFAULT_STRENGTH, StoredStep, adjust_picture, analysis_after, pixel_steps,
    };
    pub use crate::image_dither::DitherSettings;
    pub use crate::image_preset::{Preset, stretch_points};
}

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

/// Print and cut (`specs/0121-print-and-cut`, ADR 0081 §5): the reading of a placed sheet's
/// measures into the fit and its warnings, and their words.
pub mod print_and_cut {
    pub use crate::print_and_cut_reading::{
        Apply, FitState, FitSummary, MIN_SPREAD, MIRROR_MARGIN_MM, MarkRow, PrintAndCut, RowKind,
        SAME_PLACEMENT_MM, SCALE_USABLE, SCALE_WARN, SheetInput, Warning, head_position_um,
        machine_mm, read_print_and_cut,
    };
    /// The words of the Print and cut group.
    pub mod text {
        pub use crate::print_and_cut_words::*;
    }
}

/// The laser job of `specs/0029-laser-job-preview-and-output` (ADR 0064): the format-neutral
/// move list one page runs, its order, time and gate, the output registers of ADR 0065 §1 and the
/// one entry point to every output format.
pub mod job {
    pub use crate::job_anchor::{BurnBox, anchor, burn_box};
    pub use crate::job_build::{Built, build_frame_job, build_job, prepare_job};
    pub use crate::job_effects::{ExportEffects, export_effects};
    pub use crate::job_frame::{FrameShape, outline as frame_outline};
    pub use crate::job_gate::{
        BED_TOLERANCE_UM, Gate, GateFacts, Issue, MachineFacts, Targets, gate, outside_the_bed,
    };
    pub use crate::job_gate_cause::{Cause, Severity};
    pub use crate::job_gate_rows::{NumbersLine, RunFacts, numbers_lines};
    pub use crate::job_gate_text::{
        CheckText, Names, check_text, machine_line, numbers_text, start_text,
    };
    pub use crate::job_hatch::{HatchLines, Span, hatch};
    pub use crate::job_input::{
        BlockInput, BlockSource, CutOutItem, JobInput, LeafGeometry, LeafInput, LeafOutline,
        MachineInput, PointUm, Ring, mm_of_um, um_of_mm,
    };
    pub use crate::job_moves::{
        Block, BuiltMoves, Frame, Job, Move, MoveKind, RUN_OUT_ACCELERATION,
    };
    pub use crate::job_numbers::{
        BlockNumbers, NumbersContext, NumbersRead, NumbersSource, RunNumbers, block_numbers,
    };
    pub use crate::job_plan::{
        FLATTEN_MM, Plan, PlanCache, PlannedPath, PlannedRun, PlannedSweep, plan_block,
    };
    pub use crate::job_raster_items::ADJUST_DOTS;
    pub use crate::job_raster_plan::{
        MAX_DOTS, burns_grey, leaf_image_settings, raster_dot_map, raster_dot_map_adjusted,
    };
    pub use crate::job_raster_rows::{GAP_UM, Levels, Run, Sweep, run_power};
    pub use crate::job_timeline::{
        Head, Timeline, duration_text, estimate_text, length_text, lengths_text, readout_text,
    };
    pub use crate::job_write::{extension, file_type, name, write, write_frame};
    pub use crate::job_write_frame::FRAME_FEED;
    pub use crate::job_write_grbl::{coordinate, s_value};
    pub use crate::output_settings::{
        FIXTURE_MATERIAL_FIELD, OutputSettings, START_FIELD, Start, USER_ORIGIN_FIELD,
        USER_ORIGIN_LIMIT_UM, read_output_settings, register_key,
    };
}

/// The settings of operations and the numbers they share with material records (ADR 0054 §1 to
/// §3; ADR 0055 §2; ADR 0058 §2), with the operation-only settings of
/// `specs/0111-laser-operation-tools` (ADR 0056 §5, §6).
pub mod settings {
    pub use crate::material::{
        CellIndex, Interval, MAX_HISTORY_ENTRIES, MAX_MATERIAL_NAME_CHARS, MachineTie, Passes,
        PowerPercent, SetKind, SetNumbers, Settings, Thickness, next_modified,
    };
    pub use crate::operation_image_options::{Dither, ImageEdit, ImageField, ImageSettings};
    pub use crate::operation_options::{
        ENTER_NUMBER, ENTER_WHOLE_NUMBER, OperationOptions, OptionEdit, OptionsDamage, Overscan,
        ScanAngle, ZOffset, hatches, read_options, scans,
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
    pub use crate::test_cell_fill::{cell_gradient, cell_style};
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
        Axis, MAX_CELL, MAX_COUNT, MIN_CELL, MIN_COUNT, MaterialRef, PictureSource, Recipe,
        RecipeDefaults, RecipeField, Side, TestRecipe,
    };
    pub use crate::test_recipe_layout::{
        GAP_MM, LabelPart, LaidCell, LaidLabel, TEXT_MM, TestLayout, layout,
    };
}

/// The maker's material library of `specs/0112-test-grid-and-materials`: record format 3 shared
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
pub use fixture_grid::{
    GridCounts, MAX_GRID_CENTRES, MAX_GRID_COUNT, centres, clamp_count, mark_range, spacing_range,
};
pub use fixture_kind::{
    FixtureKind, KIND_PREFIX, MAJOR_EVERY_RANGE, MmRange, ScrewPart, SlotShape, StopPart,
    board_side_range, grid_spacing_range, origin_offset_range, pin_diameter_range, screw_diameters,
    screw_head_range, screw_thread_range, slot_size_range, stop_leg_range, stop_legs,
    stop_thickness_range,
};
pub use fixture_marks::{Circle, Line, MarkLines, centre_marks};
pub use fixture_part::{FixturePart, job_parts};
pub use fixture_sets::{FixtureHome, is_locked, on_bed};
pub use fixture_slot::Slot;
pub use fixture_snap::{Placement, board_points, stop_points};
pub use fixture_stored::{
    CentreGrid, SLOT_CIRCLE, SLOT_RECT, centre_grid, grid_spec, params_damage, pin_spec, read_slot,
    screw_spec, translation_part,
};
pub use grid_layout::{
    GridLayout, GridSpec, Lattice, Number, Tape, TapeEdge, TapeEdges, Tick, layout,
};
pub use library_merge::{
    CONFLICT_SUFFIX, MergeReason, RecordMerge, conflict_id, merge_machine_record,
    merge_material_record,
};
pub use machine_profile::{
    BaudRate, Connection, DefaultNumbers, FeedRate, GENERIC_LASER_ID, GENERIC_LASER_NAME, Limited,
    MAX_CONNECTION_CHARS, MAX_NAME_CHARS, MAX_WORK_MM, MIN_WORK_MM, MachineProfile, OriginCorner,
    OutputFormat, PowerValue, ProfileValues, clamp_address, clamp_baud, clamp_default_power,
    clamp_default_speed, clamp_feed, clamp_name, clamp_port, clamp_power, clamp_work_area,
    clamp_work_side, edit_max_feed, grouped, is_valid_work_side, machine_size_offer, save_as_name,
};
pub use machine_profile_copy::{
    CopyRead, DAMAGED_NAME_NOTICE, ProfileCopy, align_due, differs, read_copy, sets_differ,
    write_copy,
};
pub use machine_profile_file::{
    MAX_RECORD_BYTES, RecordError, read_choice, read_record, write_choice, write_record,
};
pub use machine_profile_sets::{
    MAX_RECORD_FIXTURES, MAX_RECORD_JOBS, MAX_RECORD_SETS, MAX_RECORD_SLOTS, MAX_RECORD_TARGETS,
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

use curvyo_extension_core::{Extension, Manifest, NoticeContext};

/// The laser workbench as a compiled-in extension (ADR 0080 §1): `manifest` and `notice` only.
#[derive(Debug, Clone, Copy)]
pub struct Laser;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static LASER: Laser = Laser;

impl Extension for Laser {
    fn manifest(&self) -> Manifest {
        manifest()
    }

    fn notice(&self, id: &str, context: &NoticeContext) -> Option<String> {
        notice(id, context)
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

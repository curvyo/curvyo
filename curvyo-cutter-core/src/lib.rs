//! The cutting plotter extension (ADR 0097 §2, `specs/0159-cutting-plotter`): its manifest,
//! profiles, mats, operations, job and writers. It receives document data as arguments and never
//! writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod builtins;
mod cut_area;
mod job;
mod manifest;
mod mat;
mod mat_print;
mod numbers;
mod operation;
mod play;
mod profile;
mod profile_copy;
mod profile_file;
mod write_hpgl;
mod write_svg;

pub use builtins::{
    BUILTINS, Builtin, CRICUT_MAKER_3, DEFAULT_BUILTIN, GENERIC_HPGL_CUTTER, builtin,
};
pub use cut_area::{CUT_AREA_TOLERANCE_MM, outside_cut_area};
pub use job::{
    CutterJob, FLATTEN_MM, ItemOutline, JobItem, JobMove, JobOperation, Lengths, MoveKind,
    OperationRun, build_job,
};
pub use manifest::{ADD_PAGE_COMMIT_LABEL, PROCESS_ID, WORD, manifest};
pub use mat::{
    CUT_INSET_MM, Grip, INCH_MM, MatRead, MatSize, PAGE_ORIGIN_ON_MAT, PARAM_GRID, PARAM_GRIP,
    Rect, SHEET_END_MM, SHEET_RADIUS_MM, SHEET_SIDE_MM, mat_params, mat_to_page, page_to_mat,
};
pub use mat_print::{
    CROSS_LINE_MM, GRID_LINE_MM, MatPrint, PrintLine, PrintText, TEXT_MM, mat_print,
};
pub use numbers::{Passes, PressurePct, SpeedMmS};
pub use operation::{
    CutterSettings, Field, KEY_PASSES, KEY_PRESSURE, KEY_SPEED, OpKind, edit_setting,
};
pub use play::PlayAxis;
pub use profile::{
    CutterProfile, CutterValues, ForceRange, MAX_NAME_CHARS, OriginCorner, OutputFormat, WorkArea,
};
pub use profile_copy::{CopyRead, CutterCopy, differs, read_copy, resize_anchor, write_copy};
pub use profile_file::{
    MAX_RECORD_BYTES, RECORD_FORMAT_VERSION, RecordError, merge_cutter_record, read_record,
    write_record,
};
pub use write_hpgl::{PlotterUnit, UNITS_PER_MM, plotter_units, write_hpgl};
pub use write_svg::{CUT_COLOURS, write_svg};

use curvyo_extension_core::{Extension, Manifest};

/// The cutting plotter workbench as a compiled-in extension (ADR 0080 §1): `manifest` only.
#[derive(Debug, Clone, Copy)]
pub struct Cutter;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static CUTTER: Cutter = Cutter;

impl Extension for Cutter {
    fn manifest(&self) -> Manifest {
        manifest()
    }
}

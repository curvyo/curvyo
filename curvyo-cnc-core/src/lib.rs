//! The CNC milling extension (ADR 0097 §2, `specs/0161-cnc-output`): its manifest, profiles,
//! stock, tools, the Profile operation's toolpath, the job's moves, its checks and the GRBL
//! writer. It receives document data as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod builtins;
mod checks;
mod job;
mod manifest;
mod operation;
mod profile;
mod profile_copy;
mod profile_file;
mod profile_toolpath;
mod stock;
mod tools;
mod units;
mod work;
mod write_grbl;

pub use builtins::{BUILTINS, Builtin, DEFAULT_BUILTIN, GENERIC_GRBL_ROUTER, builtin};
pub use checks::{EDGE_TOLERANCE_MM, other_tools, outside_stock, outside_work_area};
pub use job::{CncJob, JobOperation, Move, MoveKind, OperationReport, build_job};
pub use manifest::{ADD_PAGE_COMMIT_LABEL, PROCESS_ID, WORD, manifest};
pub use operation::{Direction, PROFILE_KIND, ProfileSettings, Side};
pub use profile::{CncProfile, CncValues, JUNCTION_DEVIATION_MM, MAX_NAME_CHARS, OutputFormat};
pub use profile_copy::{CncCopy, CopyRead, differs, read_copy, resize_anchor, write_copy};
pub use profile_file::{
    MAX_RECORD_BYTES, RECORD_FORMAT_VERSION, RecordError, merge_cnc_record, read_record,
    write_record,
};
pub use profile_toolpath::{
    FLATTEN_TOLERANCE_MM, ItemLoops, ItemOutline, OFFSET_TOLERANCE_MM, ToolLoop, profile_loops,
};
pub use stock::{MAX_MATERIAL_CHARS, STOCK_KIND, Stock};
pub use tools::{DEFAULT_TOOL_ID, TOOLS_FIELD, Tool, read_tools, write_tools};
pub use units::{AccelMmS2, Depth, FeedMmMin, Flutes, Rpm, StepDown, ToolDiameter};
pub use work::{WorkPoint, page_to_work, work_to_page};
pub use write_grbl::{GrblHeader, MAX_LINE, number, write_grbl, write_grbl_mapped};

use curvyo_extension_core::{Extension, Manifest};

/// The CNC milling workbench as a compiled-in extension (ADR 0080 §1): `manifest` only.
#[derive(Debug, Clone, Copy)]
pub struct Cnc;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static CNC: Cnc = Cnc;

impl Extension for Cnc {
    fn manifest(&self) -> Manifest {
        manifest()
    }
}

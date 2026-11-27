//! The embroidery extension (ADR 0097 §2, `specs/0160-embroidery`): its manifest, profiles, hoops,
//! operations, the stitch plan with its summary, and the DST writer and reader. It receives
//! document data as arguments and never writes.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod builtins;
mod dst_read;
mod dst_write;
mod emit;
mod hoop;
mod manifest;
mod operation;
mod plan;
mod profile;
mod profile_copy;
mod profile_file;
mod stitch;
mod summary;

pub use builtins::{BUILTINS, Builtin, DEFAULT_BUILTIN, W6_EU7, builtin};
pub use dst_read::{DstRead, read_dst};
pub use dst_write::{DstError, Extents, HEADER_BYTES, MAX_STEP, encode_step, extents, write_dst};
pub use emit::TRIM_ABOVE;
pub use hoop::{HoopSize, PAGE_MARGIN_MM, outside_field, page_mm};
pub use manifest::{ADD_PAGE_COMMIT_LABEL, PROCESS_ID, WORD, manifest};
pub use operation::{EmbroiderySettings, Field, KEY_THREAD, OpKind, ThreadColour, edit_setting};
pub use plan::{
    Block, FLATTEN_MM, ItemOutline, NARROW_SATIN_MM, PlanItem, PlanOperation, StitchPlan,
    build_plan,
};
pub use profile::{
    DST_MAX_STITCH, EmbroideryProfile, EmbroideryValues, MAX_NAME_CHARS, MIN_MAX_STITCH,
    OutputFormat, StitchesPerMin,
};
pub use profile_copy::{CopyRead, EmbroideryCopy, read_copy, write_copy};
pub use profile_file::{
    MAX_RECORD_BYTES, RECORD_FORMAT_VERSION, RecordError, merge_embroidery_record, read_record,
    write_record,
};
pub use stitch::{Stitch, StitchKind, StitchPoint, TenthMm};
pub use summary::{Summary, summarise};

pub use curvyo_inkstitch_core::Mm;

use curvyo_extension_core::{Extension, Manifest};

/// The embroidery workbench as a compiled-in extension (ADR 0080 §1): `manifest` only.
#[derive(Debug, Clone, Copy)]
pub struct Embroidery;

/// The one value the registry holds (`curvyo-ui-core/src/extensions.rs`).
pub static EMBROIDERY: Embroidery = Embroidery;

impl Extension for Embroidery {
    fn manifest(&self) -> Manifest {
        manifest()
    }
}

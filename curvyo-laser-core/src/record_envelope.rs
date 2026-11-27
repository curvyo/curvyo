//! The laser's view of the shared record envelope (ADR 0097 §4): the names of
//! `curvyo-library-core` its modules use, and the id rule of a material's machine.

pub use curvyo_library_core::{
    EnvelopeError, LibraryWrite, RECORD_SUFFIX, is_record_id, new_record_id, pretty,
    record_file_name, record_id,
};

/// The envelope of a laser record: [`curvyo_library_core::open_family`] for the laser.
pub(crate) fn open(
    text: &str,
    max_bytes: usize,
    oldest: u64,
    newest: u64,
) -> Result<(serde_json::Value, u64), EnvelopeError> {
    curvyo_library_core::open_family(
        text,
        max_bytes,
        curvyo_library_core::Family::Laser,
        oldest,
        newest,
    )
}

/// Whether `id` may name the machine of a material: a profile UUID or the built-in.
pub(crate) fn is_machine_id(id: &str) -> bool {
    id == crate::machine_profile::GENERIC_LASER_ID || is_record_id(id)
}

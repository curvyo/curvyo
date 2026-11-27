//! The Data Matrix parameter groups (UX notes of `0144` §11.14): the QR rows for content, size and
//! modules, no Finder, Error correction or Logo group, Quiet zone 1.

use curvyo_extension_core::ParamGroup;

use crate::qr::schema::{advanced, colours, content, modules, size};

/// The default Quiet zone in modules (ISO/IEC 16022 asks for one).
pub(crate) const QUIET_DEFAULT: u32 = 1;

pub(crate) fn groups() -> Vec<ParamGroup> {
    // The default content is a 22 x 22 symbol; with the quiet zone the outer width is 24 mm.
    vec![
        content(),
        size(24.0),
        modules(),
        colours(),
        advanced(QUIET_DEFAULT),
    ]
}

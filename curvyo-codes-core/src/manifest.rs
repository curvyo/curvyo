//! The codes' manifest: the kind `qr` and its parameter groups.

use curvyo_extension_core::{Contribution, GeneratorKind, Manifest};

use crate::qr;

/// The extension id; generated objects store `<id>/<kind>`.
pub const PROCESS_ID: &str = "curvyo.codes";
/// The kind id of the QR code.
pub const QR: &str = "qr";

/// The codes' manifest (one kind per symbology, ADR 0091 §2).
#[must_use]
pub fn manifest() -> Manifest {
    Manifest {
        id: PROCESS_ID,
        name: "Codes",
        version: env!("CARGO_PKG_VERSION"),
        licence: "AGPL-3.0-or-later",
        contributions: vec![Contribution::Generator(GeneratorKind {
            id: QR,
            label: "QR code",
            icon: "QrCode",
            groups: qr::groups(),
            source: None,
        })],
    }
}

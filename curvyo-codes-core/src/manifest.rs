//! The codes' manifest: one kind per symbology, all of the family `code` (ADR 0092 §2).

use curvyo_extension_core::{Contribution, Family, GeneratorKind, Manifest, Resize};

use crate::linear::{Symbology, schema};
use crate::{datamatrix, qr};

/// The extension id; generated objects store `<id>/<kind>`.
pub const PROCESS_ID: &str = "curvyo.codes";
/// The kind id of the QR code.
pub const QR: &str = "qr";
/// The kind id of the Data Matrix.
pub const DATA_MATRIX: &str = "data-matrix";

/// The family of every symbology: one segment in the Generators bar, one Symbology group.
pub const FAMILY: Family = Family {
    id: "code",
    segment: "Code",
    label: "Symbology",
};

/// The codes' manifest (one kind per symbology, ADR 0091 §2).
#[must_use]
pub fn manifest() -> Manifest {
    let mut contributions = vec![Contribution::Generator(GeneratorKind {
        id: QR,
        label: "QR code",
        icon: "QrCode",
        groups: qr::groups(),
        source: None,
        scanned: true,
        family: Some(FAMILY),
        resize: Resize::Proportional,
    })];
    contributions.push(Contribution::Generator(GeneratorKind {
        id: DATA_MATRIX,
        label: "Data Matrix",
        icon: "Grid3x3",
        groups: datamatrix::groups(),
        source: None,
        scanned: true,
        family: Some(FAMILY),
        resize: Resize::Proportional,
    }));
    contributions.extend(Symbology::ALL.into_iter().map(|symbology| {
        Contribution::Generator(GeneratorKind {
            id: symbology.id(),
            label: symbology.label(),
            icon: "Barcode",
            groups: schema::groups(symbology),
            source: None,
            scanned: true,
            family: Some(FAMILY),
            resize: Resize::Free,
        })
    }));
    Manifest {
        id: PROCESS_ID,
        name: "Codes",
        version: env!("CARGO_PKG_VERSION"),
        licence: "AGPL-3.0-or-later",
        contributions,
    }
}

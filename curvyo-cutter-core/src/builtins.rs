//! The built-in machine profiles of the cutting plotter, in code only and never written to a
//! file (ADR 0038 §4, ADR 0097 §11 step G2, `specs/0159-cutting-plotter` criterion 2).

use curvyo_document_core::DocumentSize;
use curvyo_library_core::Connection;

use crate::mat::MatSize;
use crate::numbers::{PressurePct, SpeedMmS};
use crate::profile::{
    CutterProfile, CutterValues, ForceRange, OriginCorner, OutputFormat, WorkArea,
};

/// One built-in profile: its id (not a UUID, as `generic-laser` is not) and its name; its values
/// come from [`Builtin::profile`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Builtin {
    /// The stable id.
    pub id: &'static str,
    /// The name the profile pick shows.
    pub name: &'static str,
}

impl Builtin {
    /// The built-in as a profile (Proposal values: 40 %, 200 mm/s, Max speed 800 mm/s).
    #[must_use]
    pub fn profile(&self) -> CutterProfile {
        let maker = self.id == CRICUT_MAKER_3.id;
        // invariant: the literals below are inside their ranges
        let speed = |n| SpeedMmS::new(n).unwrap_or_else(|| unreachable!());
        CutterProfile {
            id: self.id.to_owned(),
            modified: 0,
            values: CutterValues {
                name: self.name.to_owned(),
                work_area: if maker {
                    WorkArea::from_mm(304.8, 609.6)
                } else {
                    WorkArea::from_mm(600.0, 1000.0)
                }
                .unwrap_or_else(|| unreachable!()),
                origin: if maker {
                    OriginCorner::TopLeft
                } else {
                    OriginCorner::BottomLeft
                },
                output_format: if maker {
                    OutputFormat::DesignSpaceSvg
                } else {
                    OutputFormat::Hpgl
                },
                force: if maker {
                    ForceRange::new(1, 100)
                } else {
                    ForceRange::new(1, 80)
                }
                .unwrap_or_else(|| unreachable!()),
                max_speed: speed(800),
                default_pressure: PressurePct::new(40).unwrap_or_else(|| unreachable!()),
                default_speed: speed(200),
                mats: if maker {
                    MatSize::ALL.to_vec()
                } else {
                    Vec::new()
                },
                connection: Connection::default(),
            },
        }
    }

    /// The size of a new page that takes this profile: the first mat's field, else the work
    /// area (PL11).
    #[must_use]
    pub fn page_size(&self) -> DocumentSize {
        self.profile().values.page_size()
    }
}

/// The built-in Cricut Maker 3 (criterion 2): Design Space SVG, mats 12 x 12 and 12 x 24 in.
pub const CRICUT_MAKER_3: Builtin = Builtin {
    id: "cricut-maker-3",
    name: "Cricut Maker 3",
};

/// The built-in Generic HPGL cutter (criterion 2): HPGL, 600 x 1000 mm, origin bottom left.
pub const GENERIC_HPGL_CUTTER: Builtin = Builtin {
    id: "generic-hpgl-cutter",
    name: "Generic HPGL cutter",
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[CRICUT_MAKER_3, GENERIC_HPGL_CUTTER];

/// The profile a new page takes (criterion 1).
pub const DEFAULT_BUILTIN: Builtin = CRICUT_MAKER_3;

/// The built-in of id `id`.
#[must_use]
pub fn builtin(id: &str) -> Option<CutterProfile> {
    BUILTINS.iter().find(|b| b.id == id).map(Builtin::profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_have_their_ids_names_formats_and_page_sizes() {
        let maker = builtin("cricut-maker-3").unwrap();
        assert_eq!(maker.values.name, "Cricut Maker 3");
        assert_eq!(maker.values.output_format, OutputFormat::DesignSpaceSvg);
        assert_eq!(maker.values.mats, [MatSize::In12x12, MatSize::In12x24]);
        assert_eq!(
            CRICUT_MAKER_3.page_size(),
            DocumentSize::from_mm(304.8, 304.8)
        );
        let hpgl = builtin("generic-hpgl-cutter").unwrap();
        assert_eq!(hpgl.values.name, "Generic HPGL cutter");
        assert_eq!(hpgl.values.output_format, OutputFormat::Hpgl);
        assert_eq!(hpgl.values.origin, OriginCorner::BottomLeft);
        assert_eq!(hpgl.values.mats, []);
        assert_eq!(
            GENERIC_HPGL_CUTTER.page_size(),
            DocumentSize::from_mm(600.0, 1000.0)
        );
        assert_eq!(DEFAULT_BUILTIN.id, "cricut-maker-3");
        assert!(maker.values.is_valid() && hpgl.values.is_valid());
    }
}

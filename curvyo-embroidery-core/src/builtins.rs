//! The built-in machine profiles of embroidery, in code only and never written to a file (ADR 0038
//! §4, ADR 0097 §11 step G2, `specs/0160-embroidery` criterion 2).

use curvyo_document_core::DocumentSize;

use crate::hoop::HoopSize;
use crate::profile::{
    DST_MAX_STITCH, EmbroideryProfile, EmbroideryValues, OutputFormat, StitchesPerMin,
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

/// 600 stitches per minute (fact 2, Proposal).
const EU7_SPEED: u64 = 600;

impl Builtin {
    /// The built-in as a profile: DST, both hoops, 12.1 mm, 600 stitches per minute.
    #[must_use]
    pub fn profile(&self) -> EmbroideryProfile {
        EmbroideryProfile {
            id: self.id.to_owned(),
            modified: 0,
            values: EmbroideryValues {
                name: self.name.to_owned(),
                output_format: OutputFormat::Dst,
                hoops: HoopSize::ALL.to_vec(),
                max_stitch: DST_MAX_STITCH,
                // invariant: 600 lies within 100 to 1 500
                speed: StitchesPerMin::new(EU7_SPEED).unwrap_or_else(|| unreachable!()),
            },
        }
    }

    /// The size of a new page that takes this profile.
    #[must_use]
    pub fn page_size(&self) -> DocumentSize {
        self.profile().values.page_size()
    }
}

/// The built-in W6 EU7: its page is the 150 x 250 mm sewing field plus 20 mm on each side
/// (`adrs.md` EM10).
pub const W6_EU7: Builtin = Builtin {
    id: "w6-eu7",
    name: "W6 EU7",
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[W6_EU7];

/// The profile a new page takes (criterion 1).
pub const DEFAULT_BUILTIN: Builtin = W6_EU7;

/// The built-in of id `id`.
#[must_use]
pub fn builtin(id: &str) -> Option<EmbroideryProfile> {
    BUILTINS.iter().find(|b| b.id == id).map(Builtin::profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_w6_eu7_has_dst_both_hoops_12_1_mm_and_600_stitches_per_minute() {
        let eu7 = builtin("w6-eu7").unwrap();
        assert_eq!(eu7.values.name, "W6 EU7");
        assert_eq!(eu7.values.output_format, OutputFormat::Dst);
        assert_eq!(
            eu7.values.hoops,
            [HoopSize::Field150x250, HoopSize::Field100x100]
        );
        assert_eq!(eu7.values.max_stitch.mm(), 12.1);
        assert_eq!(eu7.values.speed.get(), 600);
        assert_eq!(W6_EU7.page_size(), DocumentSize::from_mm(190.0, 290.0));
        assert_eq!(DEFAULT_BUILTIN.id, "w6-eu7");
        assert!(eu7.values.is_valid());
    }
}

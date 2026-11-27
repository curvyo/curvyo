//! The built-in machine profiles of embroidery, in code only and never written to a file (ADR 0038
//! §4, ADR 0095 §11 step G2).

use curvyo_document_core::DocumentSize;

/// One built-in profile: its id (not a UUID, as `generic-laser` is not), its name and the size of
/// a new page that takes it. The profile body comes with the family's first slice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Builtin {
    /// The stable id.
    pub id: &'static str,
    /// The name the profile pick shows.
    pub name: &'static str,
    width_mm: f64,
    height_mm: f64,
}

impl Builtin {
    /// The size of a new page that takes this profile.
    #[must_use]
    pub const fn page_size(&self) -> DocumentSize {
        DocumentSize::from_mm(self.width_mm, self.height_mm)
    }
}

/// The built-in W6 EU7: its page is the 150 x 250 mm sewing field plus 20 mm on each side (`adrs.md` EM10).
const W6_EU7: Builtin = Builtin {
    id: "w6-eu7",
    name: "W6 EU7",
    width_mm: 190.0,
    height_mm: 290.0,
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[W6_EU7];

/// The profile a new page takes (criterion 1 of `specs/0160-embroidery`).
pub const DEFAULT_BUILTIN: Builtin = W6_EU7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_have_their_ids_names_and_page_sizes() {
        let w6_eu7 = BUILTINS.iter().find(|b| b.id == "w6-eu7").unwrap();
        assert_eq!(w6_eu7.name, "W6 EU7");
        assert_eq!(w6_eu7.page_size(), DocumentSize::from_mm(190.0, 290.0));
        assert_eq!(DEFAULT_BUILTIN.id, "w6-eu7");
    }
}

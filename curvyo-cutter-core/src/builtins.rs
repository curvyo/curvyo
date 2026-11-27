//! The built-in machine profiles of the cutting plotter, in code only and never written to a file (ADR 0038
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

/// The built-in Cricut Maker 3: its page is the 12 x 12 in mat's printed field (`adrs.md` PL11).
const CRICUT_MAKER_3: Builtin = Builtin {
    id: "cricut-maker-3",
    name: "Cricut Maker 3",
    width_mm: 304.8,
    height_mm: 304.8,
};

/// The built-in Generic HPGL cutter: its page is its work area (criterion 2).
const GENERIC_HPGL_CUTTER: Builtin = Builtin {
    id: "generic-hpgl-cutter",
    name: "Generic HPGL cutter",
    width_mm: 600.0,
    height_mm: 1000.0,
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[CRICUT_MAKER_3, GENERIC_HPGL_CUTTER];

/// The profile a new page takes (criterion 1 of `specs/0159-cutting-plotter`).
pub const DEFAULT_BUILTIN: Builtin = CRICUT_MAKER_3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_have_their_ids_names_and_page_sizes() {
        let cricut_maker_3 = BUILTINS.iter().find(|b| b.id == "cricut-maker-3").unwrap();
        assert_eq!(cricut_maker_3.name, "Cricut Maker 3");
        assert_eq!(
            cricut_maker_3.page_size(),
            DocumentSize::from_mm(304.8, 304.8)
        );
        let generic_hpgl_cutter = BUILTINS
            .iter()
            .find(|b| b.id == "generic-hpgl-cutter")
            .unwrap();
        assert_eq!(generic_hpgl_cutter.name, "Generic HPGL cutter");
        assert_eq!(
            generic_hpgl_cutter.page_size(),
            DocumentSize::from_mm(600.0, 1000.0)
        );
        assert_eq!(DEFAULT_BUILTIN.id, "cricut-maker-3");
    }
}

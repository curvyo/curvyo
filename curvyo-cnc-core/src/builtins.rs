//! The built-in machine profiles of CNC milling, in code only and never written to a file (ADR 0038
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

/// The built-in Generic GRBL router: its page is its work area (criterion 2).
const GENERIC_GRBL_ROUTER: Builtin = Builtin {
    id: "generic-grbl-router",
    name: "Generic GRBL router",
    width_mm: 300.0,
    height_mm: 300.0,
};

/// Every built-in, in list order.
pub const BUILTINS: &[Builtin] = &[GENERIC_GRBL_ROUTER];

/// The profile a new page takes (criterion 1 of `specs/0161-cnc-output`).
pub const DEFAULT_BUILTIN: Builtin = GENERIC_GRBL_ROUTER;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtins_have_their_ids_names_and_page_sizes() {
        let generic_grbl_router = BUILTINS
            .iter()
            .find(|b| b.id == "generic-grbl-router")
            .unwrap();
        assert_eq!(generic_grbl_router.name, "Generic GRBL router");
        assert_eq!(
            generic_grbl_router.page_size(),
            DocumentSize::from_mm(300.0, 300.0)
        );
        assert_eq!(DEFAULT_BUILTIN.id, "generic-grbl-router");
    }
}

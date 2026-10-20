//! The class a face declares for itself in its OS/2 table (`specs/0075-font-library` criterion 15,
//! `adrs.md` decision 4): Serif, Sans or Script from `sFamilyClass`, else from PANOSE.

use skrifa::{FontRef, Tag};

/// The class a face declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclaredClass {
    /// Serif faces (`sFamilyClass` 1 to 5 and 7, or PANOSE Latin Text with a serif style).
    Serif,
    /// Sans serif faces (`sFamilyClass` 8, or PANOSE Latin Text with a sans serif style).
    Sans,
    /// Script faces (`sFamilyClass` 10, or PANOSE Latin Hand Written).
    Script,
}

/// Where the 12 bytes start in the OS/2 table: `sFamilyClass` (2 bytes), then PANOSE (10).
const CLASS_OFFSET: usize = 30;
/// The number of class bytes: `sFamilyClass` (2) and PANOSE (10).
pub const OS2_CLASS_BYTES: usize = 12;

/// The class `os2` declares: the OS/2 bytes 30 to 41 (`sFamilyClass`, then the first ten bytes
/// of PANOSE). `sFamilyClass` decides when it names a class; else PANOSE family kind 3 (Latin
/// Hand Written) is Script, and family kind 2 (Latin Text) is Serif with serif style 2 to 10
/// and Sans with serif style 11 to 13.
/// Anything else, zeros included, declares nothing.
#[must_use]
pub const fn declared_class(os2: [u8; OS2_CLASS_BYTES]) -> Option<DeclaredClass> {
    let [class, _subclass, family_kind, serif_style, ..] = os2;
    match class {
        1..=5 | 7 => return Some(DeclaredClass::Serif),
        8 => return Some(DeclaredClass::Sans),
        10 => return Some(DeclaredClass::Script),
        _ => {}
    }
    match (family_kind, serif_style) {
        (3, _) => Some(DeclaredClass::Script),
        (2, 2..=10) => Some(DeclaredClass::Serif),
        (2, 11..=13) => Some(DeclaredClass::Sans),
        _ => None,
    }
}

/// The 12 class bytes of face `index` of `bytes`, or `None` when the face has no OS/2 table long
/// enough (the browser and Add font read them here; the desktop listing reads them by seek).
#[must_use]
pub fn os2_class_bytes(bytes: &[u8], index: u32) -> Option<[u8; OS2_CLASS_BYTES]> {
    let font = FontRef::from_index(bytes, index).ok()?;
    let table = font.table_data(Tag::new(b"OS/2"))?;
    table
        .as_bytes()
        .get(CLASS_OFFSET..CLASS_OFFSET + OS2_CLASS_BYTES)?
        .try_into()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(class: u8, kind: u8, serif: u8) -> [u8; 12] {
        let mut os2 = [0; 12];
        os2[0] = class;
        os2[2] = kind;
        os2[3] = serif;
        os2
    }

    /// Criterion 15: each rule's value, PANOSE only when the class says nothing, zeros nothing.
    #[test]
    fn ac15_the_class_rules() {
        for class in [1, 2, 3, 4, 5, 7] {
            assert_eq!(
                declared_class(with(class, 0, 0)),
                Some(DeclaredClass::Serif)
            );
        }
        assert_eq!(declared_class(with(8, 3, 0)), Some(DeclaredClass::Sans));
        assert_eq!(declared_class(with(10, 0, 0)), Some(DeclaredClass::Script));
        assert_eq!(declared_class(with(0, 3, 0)), Some(DeclaredClass::Script));
        for serif in [11, 12, 13] {
            assert_eq!(declared_class(with(0, 2, serif)), Some(DeclaredClass::Sans));
        }
        assert_eq!(declared_class(with(0, 2, 4)), Some(DeclaredClass::Serif));
        assert_eq!(declared_class(with(0, 2, 1)), None);
        assert_eq!(declared_class(with(6, 3, 0)), Some(DeclaredClass::Script));
        assert_eq!(declared_class([0; 12]), None);
    }
}

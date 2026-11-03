//! The naming rules of workspace documents and folders (`specs/0095-workspace` criteria 13, 14,
//! 18, 19; ADR 0037 decision 6): which names are refused at input, when two names are the same,
//! and the free name among siblings.

/// The longest name, in characters (`ux-notes.md` section 3.2).
pub const MAX_ENTRY_NAME_CHARS: usize = 120;

/// The characters refused in a name on every OS besides the separators and control characters.
const RESERVED_CHARS: [char; 7] = [':', '*', '?', '"', '<', '>', '|'];

/// The names Windows refuses for a file or folder, with or without an extension.
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Why a name is refused; the old name stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum NameRefusal {
    /// The name is empty or only white space.
    #[error("Enter a name.")]
    Empty,
    /// The name holds `/` or `\`.
    #[error("A name cannot contain / or \\.")]
    Separator,
    /// The name is refused by some OS: a reserved character, a control character, a leading dot,
    /// a trailing dot or space, or a Windows reserved name.
    #[error("This name is not allowed on this system.")]
    NotAllowed,
    /// The name is longer than [`MAX_ENTRY_NAME_CHARS`].
    #[error("A name can have at most 120 characters.")]
    TooLong,
}

/// Checks a name the maker typed for a document (without its extension) or a folder.
///
/// A leading dot is refused **(Proposal)**: such a file is hidden and the scan leaves it out.
///
/// # Errors
/// The first [`NameRefusal`] the name breaks.
pub fn check_entry_name(name: &str) -> Result<(), NameRefusal> {
    if name.trim().is_empty() {
        return Err(NameRefusal::Empty);
    }
    if name.contains(['/', '\\']) {
        return Err(NameRefusal::Separator);
    }
    if name.chars().count() > MAX_ENTRY_NAME_CHARS {
        return Err(NameRefusal::TooLong);
    }
    let refused_char = name
        .chars()
        .any(|c| c.is_control() || RESERVED_CHARS.contains(&c));
    let stem = name.split('.').next().unwrap_or_default().to_lowercase();
    if refused_char
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || WINDOWS_RESERVED.contains(&stem.trim_end())
    {
        return Err(NameRefusal::NotAllowed);
    }
    Ok(())
}

/// Whether two names are the same name: equal under `to_lowercase` (ADR 0037 decision 6), so
/// that a workspace moves unchanged between case-sensitive and case-insensitive file systems.
#[must_use]
pub fn same_entry_name(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

/// `wanted` if no sibling of the same kind has that name, else the first free of
/// "`wanted` 2", "`wanted` 3", ... (criterion 13). `wanted` is shortened where the number would
/// make the name longer than [`MAX_ENTRY_NAME_CHARS`].
#[must_use]
pub fn free_entry_name<'a>(wanted: &str, siblings: impl IntoIterator<Item = &'a str>) -> String {
    let taken: std::collections::HashSet<String> =
        siblings.into_iter().map(str::to_lowercase).collect();
    let first = with_suffix(wanted, "");
    if !taken.contains(&first.to_lowercase()) {
        return first;
    }
    // Of the `taken.len() + 1` numbered names at least one is free.
    (2..=taken.len() + 2)
        .map(|n| with_suffix(wanted, &format!(" {n}")))
        .find(|name| !taken.contains(&name.to_lowercase()))
        .unwrap_or_default()
}

/// The name of a duplicate, before [`free_entry_name`]: "`name` copy" (criterion 19), with
/// `name` shortened so the result keeps to [`MAX_ENTRY_NAME_CHARS`].
#[must_use]
pub fn copy_entry_name(name: &str) -> String {
    with_suffix(name, " copy")
}

/// `base` followed by `suffix`, with `base` cut so the whole has at most
/// [`MAX_ENTRY_NAME_CHARS`] characters; a cut never leaves a trailing dot or space.
fn with_suffix(base: &str, suffix: &str) -> String {
    let room = MAX_ENTRY_NAME_CHARS.saturating_sub(suffix.chars().count());
    if base.chars().count() <= room {
        return format!("{base}{suffix}");
    }
    let cut: String = base.chars().take(room).collect();
    format!("{}{suffix}", cut.trim_end_matches(['.', ' ']))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_pass() {
        for name in [
            "Lid",
            "Box job",
            "Lid v2.1",
            "Größe",
            "a",
            "über 100 mm",
            "con-panel",
        ] {
            assert_eq!(check_entry_name(name), Ok(()), "{name}");
        }
        assert_eq!(check_entry_name(&"x".repeat(MAX_ENTRY_NAME_CHARS)), Ok(()));
    }

    #[test]
    fn empty_and_blank_names_are_refused() {
        assert_eq!(check_entry_name(""), Err(NameRefusal::Empty));
        assert_eq!(check_entry_name("   "), Err(NameRefusal::Empty));
    }

    #[test]
    fn separators_are_refused_with_their_own_reason() {
        assert_eq!(check_entry_name("Sides/Lid"), Err(NameRefusal::Separator));
        assert_eq!(check_entry_name("Sides\\Lid"), Err(NameRefusal::Separator));
    }

    #[test]
    fn names_some_os_refuses_are_refused_everywhere() {
        for name in [
            "a:b",
            "a*",
            "why?",
            "\"q\"",
            "<x>",
            "a|b",
            "tab\there",
            "nul\u{0}",
            "Lid.",
            "Lid ",
            ".hidden",
            ".",
            "..",
            "CON",
            "con",
            "Nul.txt",
            "com1",
            "LPT9",
            "aux .x",
        ] {
            assert_eq!(
                check_entry_name(name),
                Err(NameRefusal::NotAllowed),
                "{name:?}"
            );
        }
    }

    #[test]
    fn long_names_are_refused() {
        assert_eq!(
            check_entry_name(&"x".repeat(MAX_ENTRY_NAME_CHARS + 1)),
            Err(NameRefusal::TooLong)
        );
    }

    #[test]
    fn names_are_the_same_regardless_of_case() {
        assert!(same_entry_name("Lid", "lid"));
        assert!(same_entry_name("GRÖSSE", "grösse"));
        assert!(!same_entry_name("Lid", "Lid 2"));
    }

    #[test]
    fn a_taken_name_gets_the_first_free_number() {
        assert_eq!(free_entry_name("Lid", ["Base"]), "Lid");
        assert_eq!(free_entry_name("Lid", ["lid"]), "Lid 2");
        assert_eq!(free_entry_name("Lid", ["Lid", "Lid 2", "LID 3"]), "Lid 4");
        assert_eq!(free_entry_name("Lid", ["Lid", "Lid 3"]), "Lid 2");
    }

    #[test]
    fn a_copy_is_named_after_its_original() {
        assert_eq!(copy_entry_name("Lid"), "Lid copy");
        assert_eq!(
            free_entry_name(&copy_entry_name("Lid"), ["Lid", "Lid copy"]),
            "Lid copy 2"
        );
    }

    #[test]
    fn a_long_name_is_cut_to_fit_its_suffix() {
        let long = "x".repeat(MAX_ENTRY_NAME_CHARS);
        let copy = copy_entry_name(&long);
        assert_eq!(copy.chars().count(), MAX_ENTRY_NAME_CHARS);
        assert!(copy.ends_with(" copy"));
        let numbered = free_entry_name(&long, [long.as_str()]);
        assert_eq!(numbered.chars().count(), MAX_ENTRY_NAME_CHARS);
        assert!(numbered.ends_with(" 2"));
        assert_eq!(check_entry_name(&numbered), Ok(()));
    }
}

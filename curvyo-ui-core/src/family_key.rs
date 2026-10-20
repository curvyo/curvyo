//! The family key (`specs/0075-font-library` "Terms", `adrs.md` decision 3): the one name under
//! which favourites, groups and font matching compare a family.

/// `name` trimmed, inner whitespace runs collapsed to one space and lower-cased (full Unicode,
/// no normalisation): "Source  Sans 3 " and "source sans 3" are one family.
#[must_use]
pub fn family_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Terms: case, outer and repeated whitespace do not make another family.
    #[test]
    fn keys_ignore_case_and_whitespace_runs() {
        assert_eq!(family_key("  Source\u{a0} Sans\t3 "), "source sans 3");
        assert_eq!(family_key("ÉLAN"), "élan");
        assert_eq!(family_key(""), "");
    }
}

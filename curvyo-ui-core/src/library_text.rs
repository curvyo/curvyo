//! The words of the synced per-user libraries (`specs/0116-library-sync` UX notes sections 1 to
//! 5, criteria 3, 5, 9 to 11): the toggle tooltip's state line and the arrival, conflict,
//! switch-off and delete notices, formatted with their names and numbers here so every build
//! says the same.

use crate::share_text::cut_label;

/// The toggle tooltip's second line while in sync (UX notes section 1): "In sync with 2
/// devices.", or the only-device wording while the group holds no other device.
#[must_use]
pub fn in_sync_line(others: usize) -> String {
    match others {
        0 => "In sync. Only this device.".to_owned(),
        1 => "In sync with 1 device.".to_owned(),
        n => format!("In sync with {n} devices."),
    }
}

/// The tooltip's second line while this item transfers (UX notes section 1).
pub const SYNCING_LINE: &str = "Syncing...";
/// The tooltip's second line while the group cannot be reached or was refused.
pub const OFFLINE_LINE: &str = "Offline.";
/// The first sync's confirm of criterion 3, the spec's sentence: the fonts arrive with slice 2,
/// the machines and materials are the libraries of slice 1.
pub const FIRST_SYNC_CONFIRM: &str =
    "Your fonts, machines and materials leave this computer encrypted, to your own computers only";

/// The arrival notice of a batch (criterion 5, P8): the kinds with none left out, at most 70
/// characters. Fonts join with slice 2.
#[must_use]
pub fn arrived_notice(machines: usize, materials: usize) -> String {
    let mut parts = Vec::new();
    for (count, one, many) in [
        (machines, "machine profile", "machine profiles"),
        (materials, "material", "materials"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {}", if count == 1 { one } else { many }));
        }
    }
    format!("{} arrived from your devices.", parts.join(" and "))
}

/// Criterion 10's notice on the other devices: the copy stays.
pub const SYNC_OFF_NOTICE: &str = "Sync off, this copy stays here.";

/// The conflict copy's name (criterion 9; P8: no device name travels with a record): "<name>
/// (from your devices)", the name cut so the whole stays within a record's 60 name characters.
#[must_use]
pub fn conflict_copy_name(name: &str) -> String {
    const SUFFIX: &str = " (from your devices)";
    const NAME_MAX: usize = 60;
    let room = NAME_MAX.saturating_sub(SUFFIX.chars().count());
    format!("{}{SUFFIX}", cut_label(name.trim(), room))
}

/// Criterion 9's notice: it names the conflict copy, at most 70 characters.
#[must_use]
pub fn conflict_notice(name: &str) -> String {
    const HEAD: &str = "Both copies are kept: ";
    let room = 70 - HEAD.chars().count() - 1;
    format!("{HEAD}{}.", cut_label(name, room))
}

/// Criterion 9's notice when the loser's text does not parse: its bytes are kept as they are.
pub const CONFLICT_UNNAMED_NOTICE: &str =
    "A record differed between your devices. Both copies are kept.";

/// Criterion 11's notice on the other devices.
#[must_use]
pub fn deleted_notice(name: &str) -> String {
    format!("{} was deleted on another device.", cut_label(name, 24))
}

/// The same without a name.
pub const DELETED_UNNAMED_NOTICE: &str = "A library item was deleted on another device.";

/// A damaged incoming record was skipped by the envelope gate (P6).
pub const DAMAGED_NOTICE: &str = "A library record could not be read. It was left as it is.";
/// An incoming record of a newer format was skipped (ADR 0004 §9).
pub const NEWER_NOTICE: &str = "A library record needs a newer Curvyo. It was left as it is.";
/// A material of the older format 1 was skipped until its conversion is built (P6).
pub const OLDER_NOTICE: &str = "A material has an older format. It was left as it is.";
/// A local record of a newer build is never rewritten (ADR 0004 §9).
pub const LOCAL_NEWER_NOTICE: &str = "A record here is from a newer Curvyo. Both copies are kept.";

/// A library update of a newer build was refused (ADR 0040 decision 11 for `library.loro`).
#[must_use]
pub fn newer_library_list_notice() -> String {
    "Another device uses a newer Curvyo. Update this one to see changes to your library.".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_lines_read_as_the_notes_say() {
        assert_eq!(in_sync_line(0), "In sync. Only this device.");
        assert_eq!(in_sync_line(1), "In sync with 1 device.");
        assert_eq!(in_sync_line(2), "In sync with 2 devices.");
        assert_eq!(SYNCING_LINE, "Syncing...");
        assert_eq!(OFFLINE_LINE, "Offline.");
        assert_eq!(
            FIRST_SYNC_CONFIRM,
            "Your fonts, machines and materials leave this computer encrypted, to your own \
             computers only"
        );
    }

    #[test]
    fn the_arrival_notice_counts_the_kinds_that_arrived() {
        assert_eq!(
            arrived_notice(1, 1),
            "1 machine profile and 1 material arrived from your devices."
        );
        assert_eq!(
            arrived_notice(2, 1),
            "2 machine profiles and 1 material arrived from your devices."
        );
        assert_eq!(
            arrived_notice(0, 3),
            "3 materials arrived from your devices."
        );
        assert!(arrived_notice(99, 99).chars().count() <= 70);
    }

    #[test]
    fn the_conflict_copy_is_named_and_its_notice_fits() {
        assert_eq!(conflict_copy_name("K40"), "K40 (from your devices)");
        let long = "K".repeat(60);
        let copy = conflict_copy_name(&long);
        assert_eq!(copy.chars().count(), 60);
        assert!(copy.ends_with("(from your devices)"));
        assert_eq!(
            conflict_notice("K40 (from your devices)"),
            "Both copies are kept: K40 (from your devices)."
        );
        assert!(conflict_notice(&conflict_copy_name(&long)).chars().count() <= 70);
        assert_eq!(
            conflict_copy_name("  K40  "),
            "K40 (from your devices)",
            "a padded name is trimmed"
        );
    }

    #[test]
    fn the_off_delete_and_skip_notices_read_as_the_spec_says() {
        assert_eq!(SYNC_OFF_NOTICE, "Sync off, this copy stays here.");
        assert_eq!(
            deleted_notice("ZQX-MAT-7731"),
            "ZQX-MAT-7731 was deleted on another device."
        );
        assert_eq!(
            deleted_notice(&"M".repeat(40)),
            format!(
                "{} was deleted on another device.",
                cut_label(&"M".repeat(40), 24)
            )
        );
        assert_eq!(
            DELETED_UNNAMED_NOTICE,
            "A library item was deleted on another device."
        );
        assert_eq!(
            CONFLICT_UNNAMED_NOTICE,
            "A record differed between your devices. Both copies are kept."
        );
        assert!(CONFLICT_UNNAMED_NOTICE.chars().count() <= 70);
        assert_eq!(
            newer_library_list_notice(),
            "Another device uses a newer Curvyo. Update this one to see changes to your library."
        );
    }
}

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
/// The first sync's confirm of criterion 3: the spec's sentence, with the closing period the
/// other sentences carry.
pub const FIRST_SYNC_CONFIRM: &str =
    "Your fonts, machines and materials leave this computer encrypted, to your own computers only.";

/// The arrival notice of a batch (criterion 5, P8): the kinds with none left out, at most 70
/// characters ("3 fonts, 2 machine profiles and 1 material arrived from your devices."); a
/// batch of one item of one kind is named by `font_arrived_notice`, `machine_arrived_notice` or
/// `material_arrived_notice` instead.
#[must_use]
pub fn arrived_notice(fonts: usize, machines: usize, materials: usize) -> String {
    let mut parts = Vec::new();
    for (count, one, many) in [
        (fonts, "font", "fonts"),
        (machines, "machine profile", "machine profiles"),
        (materials, "material", "materials"),
    ] {
        if count > 0 {
            parts.push(format!("{count} {}", if count == 1 { one } else { many }));
        }
    }
    let tail = parts.pop().unwrap_or_default();
    let list = if parts.is_empty() {
        tail
    } else {
        format!("{} and {tail}", parts.join(", "))
    };
    // The UX notes' limit of 70 characters: a long batch drops "from your devices".
    let full = format!("{list} arrived from your devices.");
    if full.chars().count() <= 70 {
        full
    } else {
        format!("{list} arrived.")
    }
}

/// The notice when exactly one font arrived (UX notes section 5): the family or file name cut
/// to 24 characters, at most 70 in all.
#[must_use]
pub fn font_arrived_notice(name: &str) -> String {
    format!("Font {} arrived from your devices.", cut_label(name, 24))
}

/// Criterion 5's notice when exactly one machine profile arrived (UX notes section 5): it
/// names the item, the name cut to 24 characters, at most 70 in all.
#[must_use]
pub fn machine_arrived_notice(name: &str) -> String {
    format!(
        "Machine profile {} arrived from your devices.",
        cut_label(name, 24)
    )
}

/// Criterion 5's notice when exactly one material arrived (UX notes section 5): it names the
/// item, the name cut to 24 characters, at most 70 in all.
#[must_use]
pub fn material_arrived_notice(name: &str) -> String {
    format!(
        "Material {} arrived from your devices.",
        cut_label(name, 24)
    )
}

/// Criterion 10's notice on the other devices: the copy stays.
pub const SYNC_OFF_NOTICE: &str = "Sync off, this copy stays here.";

/// The conflict copy's name (criterion 9; P8: no device name travels with a record):
/// "`<name>` (from your devices)", the name cut so the whole stays within a record's 60 name
/// characters.
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

const MIB: u64 = 1024 * 1024;

/// `bytes` as MiB with at most one decimal, rounded up so a size over a limit never reads as
/// the limit ("18.2 MiB", "16 MiB").
#[must_use]
pub fn mib_text(bytes: u64) -> String {
    let tenths = (bytes.saturating_mul(10)).div_ceil(MIB);
    if tenths.is_multiple_of(10) {
        format!("{} MiB", tenths / 10)
    } else {
        format!("{}.{} MiB", tenths / 10, tenths % 10)
    }
}

/// Criterion 19's notice when one file is over the per-file limit: it names the font, its size
/// and the limit; the toggle stays off and the font stays here. At most 70 characters.
#[must_use]
pub fn font_too_large_notice(name: &str, size: u64, limit: u64) -> String {
    format!(
        "Not synced: {} is {}, over {}.",
        cut_label(name, 24),
        mib_text(size),
        mib_text(limit)
    )
}

/// Criterion 19's notice when the synced fonts would pass the library limit.
#[must_use]
pub fn font_over_budget_notice(name: &str, total: u64, limit: u64) -> String {
    format!(
        "Not synced: {}; {} of {} would be used.",
        cut_label(name, 24),
        mib_text(total).trim_end_matches(" MiB"),
        mib_text(limit)
    )
}

/// The notice for a synced font the receiving device could not take (over the limit announced,
/// damaged in transfer).
pub const FONT_SKIPPED_NOTICE: &str = "A synced font could not be taken. It was skipped.";

/// Criterion 17's notice: another device's newer value of one metadata field replaced this
/// device's own change (`field` is "tags", "star" or "Cuttable"), at most 70 characters.
#[must_use]
pub fn font_field_notice(name: &str, field: &str) -> String {
    format!(
        "{}: newer {field} from your devices kept.",
        cut_label(name, 24)
    )
}

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
            "Your fonts, machines and materials leave this computer encrypted, to your own computers only."
        );
    }

    #[test]
    fn the_arrival_notice_counts_the_kinds_that_arrived() {
        assert_eq!(
            arrived_notice(0, 1, 1),
            "1 machine profile and 1 material arrived from your devices."
        );
        assert_eq!(
            arrived_notice(0, 2, 1),
            "2 machine profiles and 1 material arrived from your devices."
        );
        assert_eq!(
            arrived_notice(0, 0, 3),
            "3 materials arrived from your devices."
        );
        assert_eq!(
            arrived_notice(3, 2, 1),
            "3 fonts, 2 machine profiles and 1 material arrived from your devices."
        );
        assert_eq!(
            arrived_notice(2, 0, 0),
            "2 fonts arrived from your devices."
        );
        assert!(arrived_notice(99, 99, 99).chars().count() <= 70);
    }

    #[test]
    fn the_one_item_arrival_notice_names_the_item() {
        assert_eq!(
            machine_arrived_notice("K40"),
            "Machine profile K40 arrived from your devices."
        );
        assert_eq!(
            material_arrived_notice("Slate tile"),
            "Material Slate tile arrived from your devices."
        );
        let long = "K".repeat(40);
        assert_eq!(
            machine_arrived_notice(&long),
            format!(
                "Machine profile {} arrived from your devices.",
                cut_label(&long, 24)
            )
        );
        assert!(machine_arrived_notice(&long).chars().count() <= 70);
        assert!(material_arrived_notice(&long).chars().count() <= 70);
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

    #[test]
    fn the_font_notices_name_the_font_and_the_numbers_within_70_characters() {
        assert_eq!(
            font_arrived_notice("Source Serif 4"),
            "Font Source Serif 4 arrived from your devices."
        );
        assert_eq!(mib_text(16 * MIB), "16 MiB");
        assert_eq!(mib_text(16 * MIB + 1), "16.1 MiB");
        assert_eq!(mib_text(MIB * 18 + MIB / 5), "18.2 MiB");
        let long = "F".repeat(40);
        let too_large = font_too_large_notice(&long, 18 * MIB + MIB / 5, 16 * MIB);
        assert_eq!(
            font_too_large_notice("Big", 18 * MIB + MIB / 5, 16 * MIB),
            "Not synced: Big is 18.2 MiB, over 16 MiB."
        );
        assert!(too_large.chars().count() <= 70, "{too_large}");
        let budget = font_over_budget_notice(&long, 512 * MIB + MIB / 2, 512 * MIB);
        assert_eq!(
            font_over_budget_notice("Big", 512 * MIB + MIB / 2, 512 * MIB),
            "Not synced: Big; 512.5 of 512 MiB would be used."
        );
        assert!(budget.chars().count() <= 70, "{budget}");
        assert_eq!(
            font_field_notice("Lobster", "tags"),
            "Lobster: newer tags from your devices kept."
        );
        assert!(font_field_notice(&long, "Cuttable").chars().count() <= 70);
        assert!(FONT_SKIPPED_NOTICE.chars().count() <= 70);
    }
}

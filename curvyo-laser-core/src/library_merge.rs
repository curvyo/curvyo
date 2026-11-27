//! The laser's merge of one incoming synced machine or material record (ADR 0062 decision 5,
//! `specs/0116-library-sync`): its readers' outcomes handed to the shared
//! [`curvyo_library_core::merge_record`] (ADR 0097 §4).

use curvyo_library_core::{MergeReason, RecordMerge, merge_record, record_file_name};

use crate::machine_profile_file::{RecordError, read_record as read_machine};
use crate::material_record_file::{RecordFileError, read_record as read_material};

/// Merges the incoming machine record `incoming` against `local`, the record file `id` holds.
/// `modified` is read from the record texts themselves (ADR 0038 §2), never from a second
/// field: newest wins, equal `modified` is broken by the bytewise-greater text.
#[must_use]
pub fn merge_machine_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
    let name = record_file_name(id);
    let read = |text: &str| {
        read_machine(&name, text)
            .map(|profile| profile.modified)
            .map_err(|error| match error {
                RecordError::Newer => MergeReason::Newer,
                RecordError::OtherFamily => MergeReason::OtherFamily,
                // A record of a format before 5 is never read into sets (ADR 0060 §3): it counts
                // as unreadable here too.
                RecordError::Damaged | RecordError::Older => MergeReason::Damaged,
            })
    };
    merge_record(
        incoming,
        read(incoming),
        local.map(|local| (local, read(local))),
    )
}

/// Merges the incoming material record `incoming` against `local`, the record file `id` holds,
/// by the same rule as [`merge_machine_record`].
#[must_use]
pub fn merge_material_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
    let name = record_file_name(id);
    let read = |text: &str| {
        read_material(&name, text)
            .map(|material| material.modified)
            .map_err(|error| match error {
                RecordFileError::Newer => MergeReason::Newer,
                RecordFileError::OtherFamily => MergeReason::OtherFamily,
                RecordFileError::Older => MergeReason::Older,
                RecordFileError::Damaged(_) => MergeReason::Damaged,
            })
    };
    merge_record(
        incoming,
        read(incoming),
        local.map(|local| (local, read(local))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::MachineProfile;
    use crate::machine_profile_file::write_record as write_machine;
    use crate::material::Thickness;
    use crate::material_record::Material;
    use crate::material_record_file::write_record as write_material;
    use curvyo_library_core::{CONFLICT_SUFFIX, conflict_id, is_record_id, record_id};

    const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
    const OTHER: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

    fn machine_text(name: &str, modified: i64) -> String {
        let mut profile = MachineProfile::generic_laser();
        profile.id = ID.to_owned();
        profile.modified = modified;
        profile.values.name = name.to_owned();
        write_machine(&profile)
    }

    fn material_text(name: &str, modified: u64) -> String {
        write_material(&Material {
            id: ID.to_owned(),
            modified,
            name: name.to_owned(),
            thickness: Thickness::from_mm(3.0).unwrap(),
            lasers: Vec::new(),
            tests: Vec::new(),
            history: Vec::new(),
        })
    }

    #[test]
    fn a_record_without_a_local_copy_is_taken() {
        let machine = machine_text("ZQX-MACH-7731", 5);
        assert_eq!(
            merge_machine_record(ID, &machine, None),
            RecordMerge::IncomingWins { loser: None }
        );
        let material = material_text("ZQX-MAT-7731", 5);
        assert_eq!(
            merge_material_record(ID, &material, None),
            RecordMerge::IncomingWins { loser: None }
        );
    }

    #[test]
    fn the_same_text_does_nothing() {
        let machine = machine_text("ZQX-MACH-7731", 5);
        assert_eq!(
            merge_machine_record(ID, &machine, Some(&machine)),
            RecordMerge::Same
        );
        let material = material_text("ZQX-MAT-7731", 5);
        assert_eq!(
            merge_material_record(ID, &material, Some(&material)),
            RecordMerge::Same
        );
    }

    /// Criterion 9: the newer `modified` wins on both devices, whichever side is incoming, and
    /// the loser becomes the conflict copy.
    #[test]
    fn the_newer_modified_wins_on_both_sides() {
        let old = machine_text("ZQX-MACH-7731", 4);
        let new = machine_text("ZQX-MACH-7731", 5);
        assert_eq!(
            merge_machine_record(ID, &new, Some(&old)),
            RecordMerge::IncomingWins {
                loser: Some(old.clone())
            }
        );
        assert_eq!(
            merge_machine_record(ID, &old, Some(&new)),
            RecordMerge::LocalWins {
                conflict: old.clone()
            }
        );
        let gone = material_text("ZQX-MAT-7731", 4);
        let kept = material_text("ZQX-MAT-7731", 5);
        assert_eq!(
            merge_material_record(ID, &kept, Some(&gone)),
            RecordMerge::IncomingWins {
                loser: Some(gone.clone())
            }
        );
        assert_eq!(
            merge_material_record(ID, &gone, Some(&kept)),
            RecordMerge::LocalWins {
                conflict: gone.clone()
            }
        );
    }

    /// The feature-local rule: equal `modified` is broken by the bytewise-greater record text,
    /// so every device picks the same winner without a clock.
    #[test]
    fn equal_modified_is_broken_by_the_bytewise_greater_text() {
        let a = machine_text("ZQX-MACH-7731", 5);
        let b = machine_text("ZQX-MACH-7732", 5);
        assert_ne!(a, b);
        let (greater, smaller) = if a > b { (&a, &b) } else { (&b, &a) };
        assert_eq!(
            merge_machine_record(ID, greater, Some(smaller)),
            RecordMerge::IncomingWins {
                loser: Some(smaller.to_owned())
            }
        );
        assert_eq!(
            merge_machine_record(ID, smaller, Some(greater)),
            RecordMerge::LocalWins {
                conflict: smaller.to_owned()
            }
        );
        let x = material_text("ZQX-MAT-7731", 5);
        let y = material_text("ZQX-MAT-7732", 5);
        let (greater, smaller) = if x > y { (&x, &y) } else { (&y, &x) };
        assert_eq!(
            merge_material_record(ID, greater, Some(smaller)),
            RecordMerge::IncomingWins {
                loser: Some(smaller.to_owned())
            }
        );
        assert_eq!(
            merge_material_record(ID, smaller, Some(greater)),
            RecordMerge::LocalWins {
                conflict: smaller.to_owned()
            }
        );
    }

    #[test]
    fn a_damaged_incoming_text_is_skipped_with_its_local_untouched() {
        let local = machine_text("ZQX-MACH-7731", 4);
        assert_eq!(
            merge_machine_record(ID, "{ not json", Some(&local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Damaged
            }
        );
        let wrong_id = machine_text("ZQX-MACH-7731", 9).replacen(ID, OTHER, 1);
        assert_eq!(
            merge_machine_record(ID, &wrong_id, Some(&local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Damaged
            }
        );
        let material = material_text("ZQX-MAT-7731", 4);
        assert_eq!(
            merge_material_record(ID, "{ not json", Some(&material)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Damaged
            }
        );
    }

    #[test]
    fn an_incoming_text_from_a_newer_build_is_skipped() {
        let local = machine_text("ZQX-MACH-7731", 4);
        let newer = machine_text("ZQX-MACH-7731", 9).replacen(
            "\"format_version\": 8",
            "\"format_version\": 9",
            1,
        );
        assert_eq!(
            merge_machine_record(ID, &newer, Some(&local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Newer
            }
        );
        let material = material_text("ZQX-MAT-7731", 4);
        let newer_material = material_text("ZQX-MAT-7731", 9).replacen(
            "\"format_version\": 5",
            "\"format_version\": 6",
            1,
        );
        assert_eq!(
            merge_material_record(ID, &newer_material, Some(&material)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Newer
            }
        );
    }

    /// ADR 0097 §4: an incoming record of another machine family, at that family's first version
    /// 9, is that family's to merge: skipped here without a notice, the local text untouched.
    #[test]
    fn an_incoming_record_of_another_family_is_skipped_as_that_familys() {
        let local = machine_text("ZQX-MACH-7731", 4);
        let cutter = machine_text("ZQX-MACH-7731", 9)
            .replacen("\"kind\": \"laser\"", "\"kind\": \"cutter\"", 1)
            .replacen("\"format_version\": 8", "\"format_version\": 9", 1);
        assert_eq!(
            merge_machine_record(ID, &cutter, Some(&local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::OtherFamily
            }
        );
        let material = material_text("ZQX-MAT-7731", 4);
        let fabric = material_text("ZQX-MAT-7731", 9).replacen(
            "\"kind\": \"laser\"",
            "\"kind\": \"embroidery\"",
            1,
        );
        assert_eq!(
            merge_material_record(ID, &fabric, Some(&material)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::OtherFamily
            }
        );
    }

    /// ADR 0004 §9: a local text from a newer build is never rewritten; the incoming text is
    /// kept as the conflict copy.
    #[test]
    fn a_local_text_from_a_newer_build_stays_untouched() {
        let newer_local = machine_text("ZQX-MACH-7731", 4).replacen(
            "\"format_version\": 8",
            "\"format_version\": 9",
            1,
        );
        let incoming = machine_text("ZQX-MACH-7731", 9);
        assert_eq!(
            merge_machine_record(ID, &incoming, Some(&newer_local)),
            RecordMerge::LocalNewer {
                conflict: incoming.clone()
            }
        );
        let newer_material = material_text("ZQX-MAT-7731", 4).replacen(
            "\"format_version\": 5",
            "\"format_version\": 6",
            1,
        );
        let incoming_material = material_text("ZQX-MAT-7731", 9);
        assert_eq!(
            merge_material_record(ID, &incoming_material, Some(&newer_material)),
            RecordMerge::LocalNewer {
                conflict: incoming_material
            }
        );
    }

    /// A damaged local text cannot compete: the incoming one wins, and the damaged bytes are
    /// kept as the conflict copy, never silently dropped; the same for a material of the older
    /// format 1.
    #[test]
    fn a_damaged_local_text_loses_and_is_kept_as_the_conflict_copy() {
        let incoming = machine_text("ZQX-MACH-7731", 9);
        assert_eq!(
            merge_machine_record(ID, &incoming, Some("{ not json")),
            RecordMerge::IncomingWins {
                loser: Some("{ not json".to_owned())
            }
        );
        let material_incoming = material_text("ZQX-MAT-7731", 9);
        let older_local = material_text("ZQX-MAT-7731", 4).replacen(
            "\"format_version\": 5",
            "\"format_version\": 1",
            1,
        );
        assert_eq!(
            merge_material_record(ID, &material_incoming, Some(&older_local)),
            RecordMerge::IncomingWins {
                loser: Some(older_local)
            }
        );
        let older_incoming = material_text("ZQX-MAT-7731", 9).replacen(
            "\"format_version\": 5",
            "\"format_version\": 1",
            1,
        );
        let material_local = material_text("ZQX-MAT-7731", 4);
        assert_eq!(
            merge_material_record(ID, &older_incoming, Some(&material_local)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Older
            }
        );
    }

    /// The conflict copy's name is ignored by the record readers, like a folder-sync tool's
    /// copy (ADR 0038 §1, the feature-local rule).
    #[test]
    fn the_conflict_copy_is_ignored_by_the_readers() {
        let conflict = conflict_id(ID);
        assert_eq!(conflict, format!("{ID}{CONFLICT_SUFFIX}"));
        assert!(!is_record_id(&conflict));
        let file = record_file_name(&conflict);
        assert_eq!(record_id(&file), None);
        let text = machine_text("ZQX-MACH-7731", 5);
        assert_eq!(read_machine(&file, &text).err(), Some(RecordError::Damaged));
    }
}

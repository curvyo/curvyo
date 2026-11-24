//! What one incoming synced library record does to the local record of the same id (ADR 0062
//! decision 5, `specs/0116-library-sync`).

use crate::machine_profile_file::{RecordError, read_record as read_machine};
use crate::material_record_file::{RecordFileError, read_record as read_material};
use crate::record_envelope::record_file_name;

/// The suffix of a conflict copy's id (the feature-local rule of `specs/0116-library-sync`): the
/// file `<uuid>.conflict.json` beside the record, the browser key `curvyo.machines/<id>.conflict`
/// or `curvyo.materials/<id>.conflict`; readers ignore it as they ignore a folder-sync tool's
/// copies (ADR 0038 §1), and deleting it is the maker's.
pub const CONFLICT_SUFFIX: &str = ".conflict";

/// Why the envelope gate (ADR 0055 §7) skipped a record text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeReason {
    /// Not a valid record of this format: skipped with a notice, never rewritten.
    Damaged,
    /// Written by a newer build: skipped with a notice, never rewritten (ADR 0004 §9).
    Newer,
    /// A material of the older format 1: skipped until its conversion is built.
    Older,
}

/// The merge of one incoming library record against the local record of the same id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordMerge {
    /// The incoming text wins: it becomes the record; `loser`, when the local text existed,
    /// becomes the conflict copy.
    IncomingWins {
        /// The losing local text, kept as the conflict copy.
        loser: Option<String>,
    },
    /// The local text wins: it stays the record, the incoming text becomes the conflict copy,
    /// and the winner is written back to the stream (ADR 0062 decision 5).
    LocalWins {
        /// The losing incoming text, kept as the conflict copy.
        conflict: String,
    },
    /// The texts are byte-equal: nothing to do.
    Same,
    /// The incoming text was skipped by the envelope gate: the local record stays untouched,
    /// never rewritten, with a notice of the reason.
    SkipIncoming {
        /// Why the text did not open.
        reason: MergeReason,
    },
    /// The local text is from a newer build (ADR 0004 §9): it stays untouched and the incoming
    /// text becomes the conflict copy, with a notice.
    LocalNewer {
        /// The incoming text, kept as the conflict copy.
        conflict: String,
    },
}

/// The id of `id`'s conflict copy.
#[must_use]
pub fn conflict_id(id: &str) -> String {
    format!("{id}{CONFLICT_SUFFIX}")
}

/// Merges the incoming machine record `incoming` against `local`, the record file `id` holds.
/// `modified` is read from the record texts themselves (ADR 0038 §2), never from a second
/// field: newest wins, equal `modified` is broken by the bytewise-greater text, so every device
/// picks the same winner (the feature-local rule of `specs/0116-library-sync`).
#[must_use]
pub fn merge_machine_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
    let name = record_file_name(id);
    let incoming_modified = match read_machine(&name, incoming) {
        Ok(profile) => profile.modified,
        Err(RecordError::Newer) => {
            return RecordMerge::SkipIncoming {
                reason: MergeReason::Newer,
            };
        }
        // A record of a format before 5 is never read into sets (ADR 0060 §3): it counts as
        // unreadable here too.
        Err(RecordError::Damaged | RecordError::Older) => {
            return RecordMerge::SkipIncoming {
                reason: MergeReason::Damaged,
            };
        }
    };
    let Some(local) = local else {
        return RecordMerge::IncomingWins { loser: None };
    };
    if local == incoming {
        return RecordMerge::Same;
    }
    match read_machine(&name, local) {
        Ok(profile) => decide(incoming_modified, incoming, profile.modified, local),
        Err(RecordError::Newer) => RecordMerge::LocalNewer {
            conflict: incoming.to_owned(),
        },
        Err(RecordError::Damaged | RecordError::Older) => RecordMerge::IncomingWins {
            loser: Some(local.to_owned()),
        },
    }
}

/// Merges the incoming material record `incoming` against `local`, the record file `id` holds,
/// by the same rule as [`merge_machine_record`].
#[must_use]
pub fn merge_material_record(id: &str, incoming: &str, local: Option<&str>) -> RecordMerge {
    let name = record_file_name(id);
    let incoming_modified = match read_material(&name, incoming) {
        Ok(material) => material.modified,
        Err(RecordFileError::Newer) => {
            return RecordMerge::SkipIncoming {
                reason: MergeReason::Newer,
            };
        }
        Err(RecordFileError::Older) => {
            return RecordMerge::SkipIncoming {
                reason: MergeReason::Older,
            };
        }
        Err(RecordFileError::Damaged(_)) => {
            return RecordMerge::SkipIncoming {
                reason: MergeReason::Damaged,
            };
        }
    };
    let Some(local) = local else {
        return RecordMerge::IncomingWins { loser: None };
    };
    if local == incoming {
        return RecordMerge::Same;
    }
    match read_material(&name, local) {
        Ok(material) => decide(incoming_modified, incoming, material.modified, local),
        Err(RecordFileError::Newer) => RecordMerge::LocalNewer {
            conflict: incoming.to_owned(),
        },
        Err(RecordFileError::Older | RecordFileError::Damaged(_)) => RecordMerge::IncomingWins {
            loser: Some(local.to_owned()),
        },
    }
}

/// The winner of two open record texts: newest `modified`, equal `modified` by the
/// bytewise-greater text.
fn decide<T: Ord>(
    incoming_modified: T,
    incoming: &str,
    local_modified: T,
    local: &str,
) -> RecordMerge {
    if (incoming_modified, incoming) > (local_modified, local) {
        RecordMerge::IncomingWins {
            loser: Some(local.to_owned()),
        }
    } else {
        RecordMerge::LocalWins {
            conflict: incoming.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_profile::MachineProfile;
    use crate::machine_profile_file::write_record as write_machine;
    use crate::material::Thickness;
    use crate::material_record::Material;
    use crate::material_record_file::write_record as write_material;
    use crate::record_envelope::{is_record_id, record_id};

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
            "\"format_version\": 7",
            "\"format_version\": 8",
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
            "\"format_version\": 4",
            "\"format_version\": 5",
            1,
        );
        assert_eq!(
            merge_material_record(ID, &newer_material, Some(&material)),
            RecordMerge::SkipIncoming {
                reason: MergeReason::Newer
            }
        );
    }

    /// ADR 0004 §9: a local text from a newer build is never rewritten; the incoming text is
    /// kept as the conflict copy.
    #[test]
    fn a_local_text_from_a_newer_build_stays_untouched() {
        let newer_local = machine_text("ZQX-MACH-7731", 4).replacen(
            "\"format_version\": 7",
            "\"format_version\": 8",
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
            "\"format_version\": 4",
            "\"format_version\": 5",
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
            "\"format_version\": 4",
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
            "\"format_version\": 4",
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

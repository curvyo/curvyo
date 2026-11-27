//! What one incoming synced library record does to the local record of the same id, for every
//! machine family (ADR 0062 decision 5, ADR 0097 §4, `specs/0116-library-sync`).
//!
//! The merge takes what the family's own reader made of each text, so `modified` is only ever
//! read by that reader (ADR 0062, "never a second field").

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
    /// A record of another machine family this build knows: that family's merge takes it, and
    /// this one skips it without a notice (ADR 0097 §4).
    OtherFamily,
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

/// Merges the incoming record text `incoming` against `local`, the text the record file holds,
/// from what the family's reader made of each: `Ok(modified)` or why it was skipped. Newest
/// `modified` wins, equal `modified` is broken by the bytewise-greater text, so every device picks
/// the same winner (the feature-local rule of `specs/0116-library-sync`). A local text that is
/// newer or of another family is never rewritten; a damaged or older one loses.
#[must_use]
pub fn merge_record<T: Ord>(
    incoming: &str,
    incoming_read: Result<T, MergeReason>,
    local: Option<(&str, Result<T, MergeReason>)>,
) -> RecordMerge {
    let incoming_modified = match incoming_read {
        Ok(modified) => modified,
        Err(reason) => return RecordMerge::SkipIncoming { reason },
    };
    let Some((local, local_read)) = local else {
        return RecordMerge::IncomingWins { loser: None };
    };
    if local == incoming {
        return RecordMerge::Same;
    }
    match local_read {
        Ok(local_modified) => {
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
        Err(MergeReason::Newer | MergeReason::OtherFamily) => RecordMerge::LocalNewer {
            conflict: incoming.to_owned(),
        },
        Err(MergeReason::Damaged | MergeReason::Older) => RecordMerge::IncomingWins {
            loser: Some(local.to_owned()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_modified_wins_and_a_tie_goes_to_the_greater_text() {
        assert_eq!(
            merge_record("b", Ok(5_i64), Some(("a", Ok(4)))),
            RecordMerge::IncomingWins {
                loser: Some("a".to_owned())
            }
        );
        assert_eq!(
            merge_record("a", Ok(4_u64), Some(("b", Ok(5)))),
            RecordMerge::LocalWins {
                conflict: "a".to_owned()
            }
        );
        assert_eq!(
            merge_record("b", Ok(5_i64), Some(("a", Ok(5)))),
            RecordMerge::IncomingWins {
                loser: Some("a".to_owned())
            }
        );
        assert_eq!(
            merge_record("a", Ok(5_i64), Some(("a", Ok(5)))),
            RecordMerge::Same
        );
        assert_eq!(
            merge_record("a", Ok(5_i64), None),
            RecordMerge::IncomingWins { loser: None }
        );
    }

    #[test]
    fn a_skipped_incoming_leaves_the_local_and_a_skipped_local_loses_unless_newer() {
        for reason in [
            MergeReason::Damaged,
            MergeReason::Newer,
            MergeReason::Older,
            MergeReason::OtherFamily,
        ] {
            assert_eq!(
                merge_record::<i64>("a", Err(reason), Some(("b", Ok(1)))),
                RecordMerge::SkipIncoming { reason }
            );
        }
        for reason in [MergeReason::Newer, MergeReason::OtherFamily] {
            assert_eq!(
                merge_record("a", Ok(1_i64), Some(("b", Err(reason)))),
                RecordMerge::LocalNewer {
                    conflict: "a".to_owned()
                }
            );
        }
        for reason in [MergeReason::Damaged, MergeReason::Older] {
            assert_eq!(
                merge_record("a", Ok(1_i64), Some(("b", Err(reason)))),
                RecordMerge::IncomingWins {
                    loser: Some("b".to_owned())
                }
            );
        }
    }

    #[test]
    fn the_conflict_copy_is_no_record_id() {
        let id = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
        assert_eq!(conflict_id(id), format!("{id}.conflict"));
        assert!(!crate::is_record_id(&conflict_id(id)));
    }
}

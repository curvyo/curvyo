//! The words of branches and versions in the History list (`specs/0042-history-branches` criteria
//! 3, 4, 8, 9, 12, 15 and 22): the header row, the "N more branches" row, the row status word, the
//! lane part of a row's accessible name, and the notices of a go to, a restore and a clone.

/// The semibold word that starts line 2 of a row on a branch (criterion 4).
pub const BRANCH_STATUS_WORD: &str = "Branch";

/// The fork point as a header names it: the step id, or "the start" for a branch that forked
/// before the first listed step.
fn fork_name(fork: Option<&str>) -> String {
    fork.map_or_else(|| "the start".to_owned(), str::to_owned)
}

/// The text of a branch's header row: "Branch from a1b2c3d, 2 steps".
#[must_use]
pub fn branch_header_text(fork: Option<&str>, steps: usize) -> String {
    let noun = if steps == 1 { "step" } else { "steps" };
    format!("Branch from {}, {steps} {noun}", fork_name(fork))
}

/// The text of the row that holds the branches beyond the first three: "3 more branches".
#[must_use]
pub fn more_branches_text(count: usize) -> String {
    let noun = if count == 1 { "branch" } else { "branches" };
    format!("{count} more {noun}")
}

/// What a screen reader hears about the lane of a row on a branch: "on branch from a1b2c3d, step 2
/// of 2", counted from the branch's oldest step (criterion 9).
#[must_use]
pub fn branch_lane_name(fork: Option<&str>, step: usize, of: usize) -> String {
    format!("on branch from {}, step {step} of {of}", fork_name(fork))
}

/// The notice after a go to version (criteria 10 and 15): "Went to a1b2c3d.", plus the later
/// steps of other authors it went back over.
#[must_use]
pub fn went_to_notice(id: &str, others_reverted: usize) -> String {
    match others_reverted {
        0 => format!("Went to {id}."),
        1 => format!("Went to {id}. Earlier session's later step was reverted for this object."),
        n => format!(
            "Went to {id}. Earlier session's {n} later steps were reverted for this object."
        ),
    }
}

/// The notice after Restore previous state (criterion 12).
pub const RESTORED_NOTICE: &str = "Object restored to the state before the first click.";

/// The notice after a clone of a version (criterion 22).
#[must_use]
pub fn cloned_notice(id: &str) -> String {
    format!("Cloned version {id} as a new object.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_names_the_fork_point_and_counts_the_steps() {
        assert_eq!(
            branch_header_text(Some("a1b2c3d"), 2),
            "Branch from a1b2c3d, 2 steps"
        );
        assert_eq!(
            branch_header_text(Some("a1b2c3d"), 1),
            "Branch from a1b2c3d, 1 step"
        );
        assert_eq!(
            branch_header_text(None, 3),
            "Branch from the start, 3 steps"
        );
    }

    #[test]
    fn the_go_to_notice_names_the_later_steps_it_reverted() {
        assert_eq!(went_to_notice("a1b2c3d", 0), "Went to a1b2c3d.");
        assert_eq!(
            went_to_notice("a1b2c3d", 2),
            "Went to a1b2c3d. Earlier session's 2 later steps were reverted for this object."
        );
        assert_eq!(
            cloned_notice("a1b2c3d"),
            "Cloned version a1b2c3d as a new object."
        );
    }

    #[test]
    fn the_more_row_counts_the_held_branches() {
        assert_eq!(more_branches_text(3), "3 more branches");
        assert_eq!(more_branches_text(1), "1 more branch");
    }

    #[test]
    fn the_lane_name_counts_from_the_oldest_step() {
        assert_eq!(
            branch_lane_name(Some("a1b2c3d"), 2, 2),
            "on branch from a1b2c3d, step 2 of 2"
        );
    }
}

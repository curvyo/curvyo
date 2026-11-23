//! Which recovery copies to delete at start (`specs/0136-autosave-and-restore` criteria 11 and
//! 12, ADR 0079 decision 1): beyond the newest copy at most [`MAX_COPIES`] copies and
//! [`MAX_BYTES`] stay, the oldest go first. A copy that a running session holds is never
//! deleted and does not count.

/// How many copies stay beyond the newest.
pub const MAX_COPIES: usize = 20;
/// How many bytes (copy and assets, 200 MB of 1,000,000 bytes) stay beyond the newest.
pub const MAX_BYTES: u64 = 200_000_000;

/// One copy as the listing sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneCandidate {
    /// The session key.
    pub key: String,
    /// The copy's size with its assets.
    pub bytes: u64,
    /// The time of its last step (ms since the epoch).
    pub last_step_ms: u64,
    /// A running session holds it.
    pub locked: bool,
}

/// The keys to delete, oldest first. The newest unlocked copy is kept whatever its size, so a
/// crashed project with big pictures still reaches its row; locked copies are never listed.
#[must_use]
pub fn prune_plan(copies: &[PruneCandidate]) -> Vec<String> {
    let mut unlocked: Vec<&PruneCandidate> = copies.iter().filter(|c| !c.locked).collect();
    unlocked.sort_by(|a, b| {
        b.last_step_ms
            .cmp(&a.last_step_ms)
            .then_with(|| a.key.cmp(&b.key))
    });
    let mut kept = 0_usize;
    let mut kept_bytes = 0_u64;
    let mut cut = false;
    let mut delete = Vec::new();
    for copy in unlocked.into_iter().skip(1) {
        if !cut && kept < MAX_COPIES && kept_bytes.saturating_add(copy.bytes) <= MAX_BYTES {
            kept += 1;
            kept_bytes += copy.bytes;
        } else {
            cut = true;
            delete.push(copy.key.clone());
        }
    }
    delete.reverse();
    delete
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copy(n: u64, bytes: u64) -> PruneCandidate {
        PruneCandidate {
            key: format!("k{n:02}"),
            bytes,
            last_step_ms: n,
            locked: false,
        }
    }

    #[test]
    fn the_newest_stays_whatever_its_size() {
        assert_eq!(
            prune_plan(&[copy(2, 10 * MAX_BYTES), copy(1, 5)]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_newest_plus_twenty_stay_and_the_next_oldest_goes() {
        let twenty_one: Vec<_> = (1..=21).map(|n| copy(n, 1)).collect();
        assert_eq!(prune_plan(&twenty_one), Vec::<String>::new());
        let twenty_two: Vec<_> = (1..=22).map(|n| copy(n, 1)).collect();
        assert_eq!(prune_plan(&twenty_two), vec!["k01".to_owned()]);
        let twenty_four: Vec<_> = (1..=24).map(|n| copy(n, 1)).collect();
        assert_eq!(
            prune_plan(&twenty_four),
            vec!["k01".to_owned(), "k02".to_owned(), "k03".to_owned()],
            "oldest first"
        );
    }

    #[test]
    fn bytes_beyond_the_newest_are_limited_and_the_oldest_go_first() {
        let copies = [
            copy(4, 1),
            copy(3, MAX_BYTES / 2),
            copy(2, MAX_BYTES / 2),
            copy(1, 1),
        ];
        assert_eq!(prune_plan(&copies), vec!["k01".to_owned()]);
        let over = [copy(3, 1), copy(2, MAX_BYTES), copy(1, 1)];
        assert_eq!(
            prune_plan(&over),
            vec!["k01".to_owned()],
            "an older small copy does not slip past a copy that fits exactly"
        );
        let too_big = [copy(3, 1), copy(2, MAX_BYTES + 1), copy(1, 1)];
        assert_eq!(
            prune_plan(&too_big),
            vec!["k01".to_owned(), "k02".to_owned()]
        );
    }

    #[test]
    fn a_locked_copy_is_never_deleted_and_does_not_count() {
        let mut copies: Vec<_> = (2..=23).map(|n| copy(n, 1)).collect();
        let mut running = copy(1, 10 * MAX_BYTES);
        running.locked = true;
        copies.push(running);
        let plan = prune_plan(&copies);
        assert_eq!(plan, vec!["k02".to_owned()]);
        assert!(!plan.contains(&"k01".to_owned()));
    }
}

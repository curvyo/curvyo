//! Whether a fixture of a page lies on the bed or keeps its place
//! (`specs/0113-fixture-sets-and-pins` criteria 2, 8 and 32; ADR 0059 §2 amended 2026-11-16).

/// Where a fixture lives on a laser page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureHome {
    /// In a machine set, which is on this page or not.
    Set {
        /// The set's On this page check.
        on_page: bool,
    },
    /// A job fixture: a direct child of the manufacturing, in no set.
    Job,
}

/// Whether a fixture lies on the bed (Frage 211 B): its own On the bed check is on and, for a set
/// fixture, its set is on this page. Only such a fixture is drawn as bed, hit, a snap or fit
/// target and runs jobs.
#[must_use]
pub const fn on_bed(home: FixtureHome, active: bool) -> bool {
    active
        && match home {
            FixtureHome::Set { on_page } => on_page,
            FixtureHome::Job => true,
        }
}

/// Whether a fixture keeps its place (ADR 0059 §2 amended 2026-11-16): its own Lock is on or,
/// for a set fixture, its set's Lock (`set_locked`, always off for a job fixture).
#[must_use]
pub const fn is_locked(fixture_locked: bool, set_locked: bool) -> bool {
    fixture_locked || set_locked
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON: FixtureHome = FixtureHome::Set { on_page: true };
    const OFF: FixtureHome = FixtureHome::Set { on_page: false };

    #[test]
    fn a_set_fixture_lies_on_the_bed_only_when_its_set_and_its_own_check_are_on() {
        assert!(on_bed(ON, true));
        assert!(!on_bed(ON, false));
        assert!(!on_bed(OFF, true), "the set is off for this page");
        assert!(!on_bed(OFF, false));
        assert!(on_bed(FixtureHome::Job, true));
        assert!(!on_bed(FixtureHome::Job, false));
    }

    #[test]
    fn a_fixture_is_locked_by_its_own_lock_or_its_sets() {
        assert!(is_locked(true, false), "its own Lock");
        assert!(is_locked(false, true), "its set's Lock");
        assert!(is_locked(true, true));
        assert!(!is_locked(false, false));
    }
}

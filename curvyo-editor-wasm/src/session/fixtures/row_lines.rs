//! The words of line 2 of the Fixtures tab's rows (`specs/0113-fixture-sets-and-pins` UX notes
//! 4.1, 4.3, 5.2): a set's kinds and count, a fixture's kind, count, size, Lock and jobs.

use curvyo_document_core::JobRole;
use curvyo_laser_core::{FixtureHome, FixtureKind};

use super::params::number;
use super::placed::PlacedFixture;
use super::shape::Shape;
use super::view_json::RoleToggle;

/// The role job of `fixture` with `role`, as its toggle.
pub(super) fn role_toggle(fixture: &PlacedFixture, role: JobRole) -> Option<RoleToggle> {
    fixture
        .jobs
        .iter()
        .find(|job| job.role == Some(role))
        .map(|job| RoleToggle {
            job: job.id.full_id(),
            on: job.output,
        })
}

/// The words of the role jobs that are on: "Centre mark", "Centre mark, Diameters" (F12).
fn jobs_on(fixture: &PlacedFixture) -> Option<String> {
    let words: Vec<&str> = [
        (JobRole::CentreMark, "Centre mark"),
        (JobRole::Diameters, "Diameters"),
    ]
    .into_iter()
    .filter(|(role, _)| role_toggle(fixture, *role).is_some_and(|t| t.on))
    .map(|(_, word)| word)
    .collect();
    (!words.is_empty()).then(|| words.join(", "))
}

/// The size of a pin or a screw in line 2: "⌀ 6 mm"; a screw with a whole-millimetre thread
/// "M6" (UX notes F6).
fn size_words(shape: &Shape) -> Option<String> {
    match shape {
        Shape::Pin { diameter, .. } => Some(format!("⌀ {} mm", number(diameter.as_mm()))),
        Shape::Screw { thread, .. } => {
            let t = thread.as_mm();
            Some(if (t - t.round()).abs() < 1e-9 {
                format!("M{}", number(t))
            } else {
                format!("⌀ {} mm", number(t))
            })
        }
        _ => None,
    }
}

/// Line 2 of a fixture row (UX notes 4.3, 5.2): kind · slots or count · size · "locked" · the
/// jobs that are on; "not on this page" (its set is off) or "not on the bed" (its own check)
/// replaces the jobs part.
pub(super) fn line2(fixture: &PlacedFixture) -> String {
    let word = if fixture.kind == FixtureKind::Unknown {
        "Unknown fixture"
    } else {
        fixture.kind.word()
    };
    let mut parts = vec![word.to_owned()];
    if fixture.kind == FixtureKind::Holder {
        let n = fixture.slots.len();
        parts.push(format!("{n} {}", if n == 1 { "slot" } else { "slots" }));
    }
    if let Some(grid) = fixture.shape.centre_grid()
        && grid.counts.centres() > 1
    {
        parts.push(format!(
            "{} × {}",
            grid.counts.columns(),
            grid.counts.rows()
        ));
    }
    parts.extend(size_words(&fixture.shape));
    if fixture.locked() {
        parts.push("locked".to_owned());
    }
    if fixture.kind != FixtureKind::Unknown {
        if matches!(fixture.home, FixtureHome::Set { on_page: false }) {
            parts.push("not on this page".to_owned());
        } else if !fixture.active {
            parts.push("not on the bed".to_owned());
        } else {
            parts.extend(jobs_on(fixture));
        }
    }
    parts.join(" · ")
}

/// Line 2 of a set (UX notes 4.1, 4.5): its kinds in creation order and its count, " · locked"
/// after the count while its Lock is on, "not on this page" while it is off (that wins over the
/// count), "No fixtures" while it is empty (F3).
pub(super) fn set_line2(on_page: bool, locked: bool, fixtures: &[&PlacedFixture]) -> String {
    let mut kinds: Vec<&str> = Vec::new();
    for fixture in fixtures {
        let word = if fixture.kind == FixtureKind::Unknown {
            "Unknown fixture"
        } else {
            fixture.kind.word()
        };
        if !kinds.contains(&word) {
            kinds.push(word);
        }
    }
    let n = fixtures.len();
    let mut parts: Vec<String> = Vec::new();
    if fixtures.is_empty() {
        parts.push("No fixtures".to_owned());
    } else {
        parts.push(kinds.join(", "));
        if on_page {
            parts.push(format!(
                "{n} {}",
                if n == 1 { "fixture" } else { "fixtures" }
            ));
        }
    }
    if locked {
        parts.push("locked".to_owned());
    }
    if !on_page {
        parts.push("not on this page".to_owned());
    }
    parts.join(" · ")
}

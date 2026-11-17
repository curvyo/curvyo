use super::*;
use crate::job_gate::Issue;
use crate::job_gate_cause::Severity;
use crate::machine_profile::OutputFormat;
use crate::operation_nearest::tests::id;

fn line(cause: Cause, count: usize, targets: Targets) -> CheckText {
    let names = Names::of([(id(1), "Cut 1".to_owned()), (id(2), "Cut 2".to_owned())]);
    check_text(
        &Issue {
            cause,
            count,
            targets,
        },
        &names,
    )
}

fn op(n: u64) -> Targets {
    Targets::Operations(vec![id(n)])
}

#[test]
fn blocks_come_before_warns_and_start_damaged_follows_the_machine() {
    assert_eq!(Cause::NothingRuns.severity(), Severity::Block);
    assert_eq!(Cause::StartDamaged.severity(), Severity::Block);
    assert_eq!(Cause::GenericLaser.severity(), Severity::Warn);
    assert_eq!(Cause::StartDamaged.rank(), Cause::MachineDamaged.rank() + 1);
}

#[test]
fn the_button_is_placed_by_the_cause_not_found_in_the_sentence() {
    let text = line(Cause::NoSettings { op: id(2) }, 1, op(2));
    assert_eq!(text.before, "");
    assert_eq!(text.button, "Cut 2");
    assert_eq!(text.after, " has no power and speed.");
    let bed = Cause::OutsideBed {
        width_um: 400_000,
        height_um: 300_000,
    };
    let text = line(bed, 3, Targets::Items(vec![id(5), id(6), id(7)]));
    assert_eq!(text.button, "3 items");
    assert_eq!(text.whole(), "3 items lie outside the 400 × 300 mm bed.");
    let text = line(Cause::StartDamaged, 1, Targets::StartPick);
    assert_eq!(
        (text.button.as_str(), text.whole().as_str()),
        ("Start", "Start is damaged. Pick one above.")
    );
    let text = line(
        Cause::CutOutNoContour { op: id(1) },
        1,
        Targets::Items(vec![id(9)]),
    );
    assert_eq!(
        (text.before.as_str(), text.button.as_str()),
        ("Cut 1: ", "1 item")
    );
    assert_eq!(text.whole(), "Cut 1: 1 item has no closed contour.");
}

#[test]
fn a_line_without_targets_has_no_button() {
    let text = line(Cause::DamagedSettings, 2, Targets::None);
    assert_eq!(text.button, "");
    assert_eq!(
        text.before,
        "2 operations have damaged settings and do not run."
    );
    let text = line(Cause::MachineDamaged, 1, Targets::None);
    assert_eq!(text.whole(), "The machine profile of this page is damaged.");
}

#[test]
fn texts_read_as_the_ux_notes_and_stay_within_seventy_characters() {
    let typed = Cause::TypedOverride {
        op: id(1),
        material: "Birch 3 mm".into(),
        fields: vec![Field::Power, Field::Speed],
    };
    assert_eq!(
        line(typed, 1, op(1)).whole(),
        "Cut 1 runs a typed power and speed, not Birch 3 mm’s."
    );
    let over = Cause::OverFeed {
        op: id(1),
        feed: 6_000,
    };
    assert_eq!(
        line(over, 1, op(1)).whole(),
        "Cut 1 runs at 6,000 mm/min, the machine's maximum."
    );
    let items = Targets::Items(vec![id(5), id(6)]);
    assert_eq!(
        line(Cause::NotAssigned, 2, items.clone()).whole(),
        "2 items are not assigned."
    );
    assert_eq!(
        line(Cause::HiddenRuns, 1, items).whole(),
        "1 item runs while hidden."
    );
    let one = line(Cause::OutputOff { op: Some(id(2)) }, 1, op(2));
    assert_eq!(
        (one.button.as_str(), one.after.as_str()),
        ("Cut 2", " has Output off.")
    );
    let two = line(
        Cause::OutputOff { op: None },
        2,
        Targets::Operations(vec![id(1), id(2)]),
    );
    assert_eq!(two.whole(), "2 operations have Output off.");
    for (cause, targets) in [
        (Cause::FixtureNoMaterial, Targets::FixturePick),
        (Cause::MachineDamaged, Targets::None),
        (Cause::StartDamaged, Targets::StartPick),
        (Cause::NothingRuns, Targets::None),
        (Cause::GenericLaser, Targets::None),
        (Cause::DamagedSettings, Targets::None),
    ] {
        let text = line(cause, 2, targets).whole();
        assert!(text.chars().count() <= 70, "{text}");
    }
    assert_eq!(mm_text(297.5), "297.5");
    assert_eq!(
        interval_text(Interval::from_hundredths(10).unwrap()),
        "0.1 mm"
    );
}

#[test]
fn an_unknown_operation_reads_as_an_operation() {
    let text = line(Cause::ZWithoutAxis { op: id(42) }, 1, op(42));
    assert_eq!(text.button, "An operation");
}

#[test]
fn the_machine_line_names_the_profile_and_the_format() {
    let mut machine = MachineFacts {
        name: "Diode 10 W".into(),
        format: OutputFormat::Grbl,
        air_assist: false,
        damaged: false,
    };
    assert_eq!(machine_line(&machine), "Diode 10 W · GRBL");
    machine.air_assist = true;
    assert_eq!(machine_line(&machine), "Diode 10 W · GRBL · air assist");
    machine.damaged = true;
    assert_eq!(machine_line(&machine), "Machine profile damaged");
}

#[test]
fn the_start_options_read_as_the_ux_notes() {
    let words: Vec<_> = Start::ALL.into_iter().map(start_text).collect();
    assert_eq!(
        words,
        [
            "Machine origin",
            "Head position",
            "Home, then machine origin"
        ]
    );
}

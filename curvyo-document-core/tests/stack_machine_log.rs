//! Criterion 55 of `specs/0020-undo-redo`: the live stacks and the stacks derived from the log are
//! the same for the same sequence of events.

use curvyo_document_core::{Document, Event, STACK_LIMIT, StackMachine, Status, Point, RectBounds};

/// A small deterministic generator, so the sequence is the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound
    }
}

#[test]
fn a_thousand_generated_events_give_the_same_stacks_live_and_from_the_log() {
    let document = Document::new(1);
    let mut live = StackMachine::new(STACK_LIMIT);
    let mut rng = Lcg(7);
    let mut done = 0;
    let mut events = 0;
    while events < 1_000 {
        match rng.next(4) {
            0 | 1 => {
                let x = f64::from(done) * 12.0;
                let _ = document.create_rect(RectBounds::from_corners(
                    Point::new(x, 0.0),
                    Point::new(x + 10.0, 10.0),
                ));
                done += 1;
                let step = document.own_steps().last().unwrap().step;
                live.apply(Event::Do(step));
            }
            2 => {
                let Some(step) = live.top_undo() else { continue };
                assert!(document.undo_step(step).unwrap().wrote);
                live.apply(Event::Undo(step));
            }
            _ => {
                let Some(step) = live.top_redo() else { continue };
                assert!(document.redo_step(step).unwrap().wrote);
                live.apply(Event::Redo(step));
            }
        }
        events += 1;
        if events % 100 == 0 {
            compare(&document, &live);
        }
    }
    compare(&document, &live);
}

fn compare(document: &Document, live: &StackMachine) {
    let from_log = document.stacks_in_log(1);
    // The live stack keeps the newest 500 entries; the log replay has no limit.
    let tail = |stack: &[curvyo_document_core::StepRef]| {
        stack[stack.len().saturating_sub(STACK_LIMIT)..].to_vec()
    };
    assert_eq!(tail(from_log.undo_stack()), live.undo_stack());
    assert_eq!(from_log.redo_stack(), live.redo_stack());
    let mut a: Vec<_> = from_log.non_applied().collect();
    let mut b: Vec<_> = live.non_applied().collect();
    a.sort_by_key(|(step, _)| (step.peer(), step.counter()));
    b.sort_by_key(|(step, _)| (step.peer(), step.counter()));
    assert_eq!(a, b);
    assert!(a.iter().all(|(_, status)| *status != Status::Applied));
}

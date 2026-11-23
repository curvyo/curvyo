//! The fake controller's head: a position moved by jogs in the time the caller passes in
//! (`specs/0118` criterion 18, adrs J9). Pure: no clock, no thread.

use std::collections::VecDeque;
use std::time::Duration;

use curvyo_connector_core::{Event, JogAxis, JogDirection, Millimetres};

/// One move: `remaining` millimetres along `axis` at `feed` mm/min, `f64::INFINITY` for a hold.
#[derive(Debug, Clone, Copy)]
struct Move {
    axis: JogAxis,
    sign: f64,
    remaining: f64,
    feed: f64,
}

/// The head and the jogs queued behind each other.
#[derive(Debug, Clone, Default)]
pub struct FakeHead {
    pos: [f64; 2],
    moves: VecDeque<Move>,
    last: Duration,
}

impl FakeHead {
    /// Queues one step behind the ones running.
    pub fn step(
        &mut self,
        (axis, direction): (JogAxis, JogDirection),
        distance: f64,
        feed: f64,
        at: Duration,
    ) {
        self.start(at);
        self.moves.push_back(Move {
            axis,
            sign: direction.sign(),
            remaining: distance,
            feed,
        });
    }

    /// A move that goes on until [`FakeHead::cancel`].
    pub fn hold(&mut self, (axis, direction): (JogAxis, JogDirection), feed: f64, at: Duration) {
        self.moves.clear();
        self.start(at);
        self.moves.push_back(Move {
            axis,
            sign: direction.sign(),
            remaining: f64::INFINITY,
            feed,
        });
    }

    fn start(&mut self, at: Duration) {
        if self.moves.is_empty() {
            self.last = at;
        }
    }

    /// Ends every move where the head stands.
    pub fn cancel(&mut self, at: Duration) {
        self.catch_up(at);
        self.moves.clear();
    }

    /// The head is at the machine origin (a homing cycle ended); the position events if it moved.
    pub fn home(&mut self) -> Vec<Event> {
        self.moves.clear();
        let before = self.pos;
        self.pos = [0.0; 2];
        if Self::same(self.pos, before) {
            return Vec::new();
        }
        self.position_events()
    }

    fn same(a: [f64; 2], b: [f64; 2]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-12)
    }

    fn position_events(&self) -> Vec<Event> {
        let p = [self.pos[0], self.pos[1], 0.0].map(Millimetres);
        vec![
            Event::Position(p),
            Event::MachinePosition {
                machine: Some(p),
                offset: Some([Millimetres(0.0); 3]),
            },
        ]
    }

    /// Whether a move is under way.
    #[must_use]
    pub fn moving(&self) -> bool {
        !self.moves.is_empty()
    }

    fn catch_up(&mut self, at: Duration) {
        let mut seconds = at.saturating_sub(self.last).as_secs_f64();
        self.last = self.last.max(at);
        while seconds > 0.0 {
            let Some(front) = self.moves.front_mut() else {
                return;
            };
            let speed = front.feed / 60.0;
            let went = (speed * seconds).min(front.remaining);
            self.pos[front.axis.index()] += front.sign * went;
            front.remaining -= went;
            seconds -= if speed > 0.0 { went / speed } else { seconds };
            if front.remaining <= 0.0 {
                self.moves.pop_front();
            } else {
                return;
            }
        }
    }

    /// Lets time pass: the position events, and `JogEnded` when the last move finished.
    pub fn run_until(&mut self, at: Duration) -> Vec<Event> {
        let (before, was_moving) = (self.pos, self.moving());
        self.catch_up(at);
        let mut events = Vec::new();
        if !Self::same(self.pos, before) {
            events = self.position_events();
        }
        if was_moving && !self.moving() {
            events.push(Event::JogEnded);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: (JogAxis, JogDirection) = (JogAxis::X, JogDirection::Plus);

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_step_takes_its_distance_over_its_feed_and_ends_once() {
        let mut head = FakeHead::default();
        // 10 mm at 3,000 mm/min is 50 mm/s: 200 ms.
        head.step(X, 10.0, 3_000.0, ms(1_000));
        assert!(
            head.run_until(ms(1_100))
                .iter()
                .all(|e| *e != Event::JogEnded)
        );
        assert!((head.pos[0] - 5.0).abs() < 1e-9);
        let events = head.run_until(ms(1_250));
        assert_eq!(events.last(), Some(&Event::JogEnded));
        assert!((head.pos[0] - 10.0).abs() < 1e-9);
        assert_eq!(head.run_until(ms(9_000)), Vec::new());
    }

    #[test]
    fn a_hold_moves_until_it_is_cancelled_and_the_position_stays() {
        let mut head = FakeHead::default();
        head.hold((JogAxis::Y, JogDirection::Minus), 1_000.0, ms(0));
        head.run_until(ms(600));
        head.cancel(ms(600));
        assert!(!head.moving());
        assert!((head.pos[1] + 10.0).abs() < 1e-9);
        assert_eq!(head.run_until(ms(5_000)), Vec::new());
        assert!((head.pos[1] + 10.0).abs() < 1e-9);
    }

    #[test]
    fn a_queued_step_runs_after_the_first() {
        let mut head = FakeHead::default();
        head.step(X, 1.0, 1_000.0, ms(0));
        head.step(X, 1.0, 1_000.0, ms(0));
        head.run_until(ms(60));
        assert!(head.moving());
        let events = head.run_until(ms(200));
        assert_eq!(events.last(), Some(&Event::JogEnded));
        assert!((head.pos[0] - 2.0).abs() < 1e-9);
    }
}

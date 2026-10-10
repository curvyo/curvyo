//! Step identity: the [`StepRef`] that names a step on every replica, and the short id shown to the
//! maker (ADR 0014 §3).

/// How many characters of the hash a shown step id has at least.
pub const MIN_SHOWN_ID_LEN: usize = 7;

/// The width of the full hash in base 36: `36^13` is the first power above `2^64`.
const FULL_ID_LEN: usize = 13;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A step's identity: the id (peer, counter) of its first operation. The same on every replica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepRef {
    peer: u64,
    counter: i32,
}

impl StepRef {
    /// A step reference from the peer and the counter of the step's first operation.
    #[must_use]
    pub const fn new(peer: u64, counter: i32) -> Self {
        Self { peer, counter }
    }

    /// The peer that made the step.
    #[must_use]
    pub const fn peer(self) -> u64 {
        self.peer
    }

    /// The counter of the step's first operation.
    #[must_use]
    pub const fn counter(self) -> i32 {
        self.counter
    }

    pub(crate) fn id(self) -> loro::ID {
        loro::ID::new(self.peer, self.counter)
    }

    /// The text form used inside commit messages: `<peer>@<counter>`.
    pub(crate) fn encode(self) -> String {
        format!("{}@{}", self.peer, self.counter)
    }

    /// The inverse of [`StepRef::encode`].
    pub(crate) fn decode(text: &str) -> Option<Self> {
        let (peer, counter) = text.split_once('@')?;
        Some(Self::new(peer.parse().ok()?, counter.parse().ok()?))
    }

    /// The 13-character lowercase base-36 form of the FNV-1a 64 hash of the 12 bytes of the id
    /// (peer, then counter, little endian). Shown ids are prefixes of it.
    #[must_use]
    pub fn full_id(self) -> String {
        let mut hash = FNV_OFFSET;
        let bytes = self
            .peer
            .to_le_bytes()
            .into_iter()
            .chain(self.counter.to_le_bytes());
        for byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        let mut digits = [b'0'; FULL_ID_LEN];
        for slot in digits.iter_mut().rev() {
            *slot = b"0123456789abcdefghijklmnopqrstuvwxyz"[(hash % 36) as usize];
            hash /= 36;
        }
        digits.iter().map(|&d| char::from(d)).collect()
    }
}

/// The shown ids of `steps`, in the same order: each is the shortest prefix of the step's full id
/// that is at least [`MIN_SHOWN_ID_LEN`] characters long and shared with no other step of the list,
/// as git abbreviates object names. Two equal steps in the list get the full id.
#[must_use]
pub fn shown_ids(steps: &[StepRef]) -> Vec<String> {
    abbreviate(&steps.iter().map(|step| step.full_id()).collect::<Vec<_>>())
}

/// The shortest unique prefixes of `full`, each at least [`MIN_SHOWN_ID_LEN`] long.
fn abbreviate(full: &[String]) -> Vec<String> {
    let mut order: Vec<usize> = (0..full.len()).collect();
    order.sort_by(|&a, &b| full[a].cmp(&full[b]));
    let common = |a: &str, b: &str| a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    let mut lengths = vec![MIN_SHOWN_ID_LEN; full.len()];
    for pair in order.windows(2) {
        let shared = common(&full[pair[0]], &full[pair[1]]);
        let needed = (shared + 1).min(FULL_ID_LEN);
        for &index in pair {
            lengths[index] = lengths[index].max(needed);
        }
    }
    full.iter()
        .zip(lengths)
        .map(|(text, length)| text[..length].to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_step_has_a_known_id() {
        // Computed independently: FNV-1a 64 over `peer.to_le_bytes() ++ counter.to_le_bytes()`.
        assert_eq!(
            StepRef::new(0x1234_5678_9abc_def0, 42).full_id(),
            "2b9mxh6qsoyun"
        );
        assert_eq!(StepRef::new(7, 0).full_id(), "1ruaghukn7hc2");
    }

    #[test]
    fn the_text_form_round_trips() {
        let step = StepRef::new(u64::MAX, 123);
        assert_eq!(StepRef::decode(&step.encode()), Some(step));
        assert_eq!(StepRef::decode("nonsense"), None);
    }

    #[test]
    fn shown_ids_have_seven_characters_unless_two_collide() {
        let steps = [StepRef::new(7, 0), StepRef::new(7, 1), StepRef::new(8, 0)];
        let ids = shown_ids(&steps);
        assert!(ids.iter().all(|id| id.len() == 7));
        assert!(steps[0].full_id().starts_with(&ids[0]));
    }

    #[test]
    fn colliding_ids_are_lengthened_until_they_differ() {
        let full: Vec<String> = [
            "aaaaaaabcdefg",
            "aaaaaaabxdefg",
            "bbbbbbbbbbbbb",
            "aaaaaaazzzzzz",
        ]
        .map(String::from)
        .to_vec();
        let ids = abbreviate(&full);
        assert_eq!(
            ids,
            ["aaaaaaabc", "aaaaaaabx", "bbbbbbb", "aaaaaaaz"].map(String::from)
        );
    }

    #[test]
    fn twenty_thousand_generated_steps_have_no_duplicate_shown_id() {
        let steps: Vec<StepRef> = (0..20_000)
            .map(|i| {
                StepRef::new(
                    u64::from(i as u32 % 7).wrapping_mul(0x9e37_79b9_7f4a_7c15),
                    i,
                )
            })
            .collect();
        let ids = shown_ids(&steps);
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), steps.len());
    }
}

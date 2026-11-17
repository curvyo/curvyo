//! The pairing code: eight characters from a 32-symbol alphabet, valid 10 minutes, one use, three
//! wrong attempts. Pure: every call takes `now` from its caller.

use std::time::{Duration, Instant};

use ring::rand::{SecureRandom, SystemRandom};
use zeroize::Zeroizing;

/// 32 symbols without `I`, `L`, `O`, `U` (Crockford base32).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// The code's length in characters.
pub const CODE_LEN: usize = 8;
/// How long a code is valid.
pub const VALID_FOR: Duration = Duration::from_secs(10 * 60);
/// Wrong attempts a code allows.
pub const ATTEMPTS: u8 = 3;

/// The 8 normalised characters of a code, the SPAKE2 password.
pub type Password = Zeroizing<[u8; CODE_LEN]>;

/// Why no attempt can begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// There is no valid code, or one exchange is running already.
    NoCode,
    /// The code is older than 10 minutes.
    Expired,
}

/// Reads typed input: case and `-` and spaces ignored, `O` read as `0`, `I` and `L` as `1`.
/// `None` unless exactly 8 symbols of the alphabet remain.
#[must_use]
pub fn normalise(input: &str) -> Option<Password> {
    let mut out = Zeroizing::new([0; CODE_LEN]);
    let mut count = 0;
    for c in input.chars().filter(|c| *c != '-' && !c.is_whitespace()) {
        let upper = match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        };
        let byte = u8::try_from(upper).ok().filter(|b| ALPHABET.contains(b))?;
        *out.get_mut(count)? = byte;
        count += 1;
    }
    (count == CODE_LEN).then_some(out)
}

/// `XXXX-XXXX`.
#[must_use]
pub fn display(code: &Password) -> String {
    let text: String = code.iter().copied().map(char::from).collect();
    format!("{}-{}", &text[..4], &text[4..])
}

struct Active {
    code: Password,
    issued: Instant,
    attempts_left: u8,
}

/// The one code a connector holds, if any.
#[derive(Default)]
pub struct PairingCodes {
    active: Option<Active>,
    exchange_running: bool,
}

impl PairingCodes {
    /// Issues a code from the system's random source, replacing a valid one. `None` when the
    /// system has no random bytes.
    pub fn issue(&mut self, now: Instant) -> Option<String> {
        let mut bits = [0; 5];
        SystemRandom::new().fill(&mut bits).ok()?;
        Some(self.issue_from(bits, now))
    }

    /// Issues the code of these 40 bits.
    pub fn issue_from(&mut self, bits: [u8; 5], now: Instant) -> String {
        let value = bits.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
        let mut code = Zeroizing::new([0; CODE_LEN]);
        for (i, slot) in code.iter_mut().enumerate() {
            let shift = 5 * (CODE_LEN - 1 - i);
            *slot = ALPHABET[usize::try_from((value >> shift) & 31).unwrap_or(0)];
        }
        let shown = display(&code);
        self.active = Some(Active {
            code,
            issued: now,
            attempts_left: ATTEMPTS,
        });
        shown
    }

    /// The valid code as printed, for the terminal and tests.
    #[must_use]
    pub fn current(&self, now: Instant) -> Option<String> {
        self.active
            .as_ref()
            .filter(|a| now.saturating_duration_since(a.issued) < VALID_FOR)
            .map(|a| display(&a.code))
    }

    /// Begins one attempt: the password for SPAKE2. The attempt counts as wrong until
    /// [`PairingCodes::succeeded`] says otherwise.
    ///
    /// # Errors
    /// [`Refusal`] when there is no valid code or an exchange is running.
    pub fn begin(&mut self, now: Instant) -> Result<Password, Refusal> {
        if self.exchange_running {
            return Err(Refusal::NoCode);
        }
        let active = self.active.as_mut().ok_or(Refusal::NoCode)?;
        if now.saturating_duration_since(active.issued) >= VALID_FOR {
            self.active = None;
            return Err(Refusal::Expired);
        }
        active.attempts_left = active.attempts_left.saturating_sub(1);
        self.exchange_running = true;
        Ok(active.code.clone())
    }

    /// The running attempt failed or was abandoned; returns the attempts left. At 0 the code is
    /// spent.
    pub fn failed(&mut self) -> u8 {
        self.exchange_running = false;
        let left = self.active.as_ref().map_or(0, |a| a.attempts_left);
        if left == 0 {
            self.active = None;
        }
        left
    }

    /// The running attempt verified: the code is spent.
    pub fn succeeded(&mut self) {
        self.exchange_running = false;
        self.active = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: u64, base: Instant) -> Instant {
        base + Duration::from_secs(secs)
    }

    #[test]
    fn a_new_code_does_not_end_a_running_exchange() {
        let mut codes = PairingCodes::default();
        let base = Instant::now();
        codes.issue_from([1; 5], base);
        codes.begin(base).unwrap();
        codes.issue_from([2; 5], base);
        assert_eq!(codes.begin(base).unwrap_err(), Refusal::NoCode);
        codes.failed();
        assert!(codes.begin(base).is_ok());
    }

    #[test]
    fn a_code_has_eight_symbols_without_look_alikes() {
        let mut codes = PairingCodes::default();
        let base = Instant::now();
        let shown = codes.issue_from([0xFF; 5], base);
        assert_eq!(shown, "ZZZZ-ZZZZ");
        let zero = codes.issue_from([0; 5], base);
        assert_eq!(zero, "0000-0000");
        for b in 0..=255u8 {
            let code = codes.issue_from([b, b ^ 0x5A, b.wrapping_mul(3), 7, b], base);
            assert_eq!(code.len(), 9);
            assert!(!code.contains(['I', 'L', 'O', 'U']), "{code}");
        }
        assert_eq!(ALPHABET.len(), 32);
    }

    #[test]
    fn typed_input_is_normalised() {
        assert_eq!(&*normalise("k7qm-4xtr").unwrap(), b"K7QM4XTR");
        assert_eq!(&*normalise(" O1l1 abcd ").unwrap(), b"0111ABCD");
        assert!(normalise("K7QM4XT").is_none());
        assert!(normalise("K7QM4XTRR").is_none());
        assert!(normalise("K7QM4XTU").is_none());
    }

    #[test]
    fn a_code_is_valid_ten_minutes_by_the_injected_clock() {
        let base = Instant::now();
        let mut codes = PairingCodes::default();
        codes.issue_from([1; 5], base);
        assert!(codes.current(at(599, base)).is_some());
        assert!(codes.current(at(600, base)).is_none());
        assert_eq!(codes.begin(at(600, base)).err(), Some(Refusal::Expired));
        assert_eq!(codes.begin(at(601, base)).err(), Some(Refusal::NoCode));
    }

    #[test]
    fn a_code_works_once() {
        let base = Instant::now();
        let mut codes = PairingCodes::default();
        let shown = codes.issue_from([9; 5], base);
        let password = codes.begin(base).unwrap();
        assert_eq!(display(&password), shown);
        codes.succeeded();
        assert_eq!(codes.begin(base).err(), Some(Refusal::NoCode));
    }

    #[test]
    fn three_wrong_attempts_spend_the_code() {
        let base = Instant::now();
        let mut codes = PairingCodes::default();
        codes.issue_from([9; 5], base);
        for expect in [2, 1, 0] {
            codes.begin(base).unwrap();
            assert_eq!(codes.failed(), expect);
        }
        assert_eq!(codes.begin(base).err(), Some(Refusal::NoCode));
    }

    #[test]
    fn one_exchange_at_a_time_and_a_new_code_replaces_a_valid_one() {
        let base = Instant::now();
        let mut codes = PairingCodes::default();
        let first = codes.issue_from([1; 5], base);
        codes.begin(base).unwrap();
        assert_eq!(codes.begin(base).err(), Some(Refusal::NoCode));
        let second = codes.issue_from([2; 5], base);
        assert_ne!(first, second);
        // The running exchange outlives the new code (architect review).
        assert_eq!(codes.begin(base).err(), Some(Refusal::NoCode));
        codes.failed();
        assert!(codes.begin(base).is_ok());
        assert_eq!(codes.current(base), Some(second));
    }
}

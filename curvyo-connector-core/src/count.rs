//! A count with a thousands comma ("12,345"), as the app and `curvyoctl` print numbers.

/// `n` with a thousands comma.
#[must_use]
pub fn count(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn counts_have_a_thousands_comma() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(12_345), "12,345");
        assert_eq!(count(1_234_567), "1,234,567");
    }
}

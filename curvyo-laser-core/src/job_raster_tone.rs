//! The tone table of one Engrave operation (ADR 0068 §4, `specs/0123` criterion 9): Invert,
//! Brightness, Contrast and Gamma folded into one lookup from grey to grey.

use crate::operation_image_options::ImageSettings;

/// Grey in (0 black to 255 white) to grey out, after the Image settings' adjustments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneTable([u8; 256]);

impl ToneTable {
    /// The table of `settings`: each of the 256 greys through Invert, Brightness, Contrast and
    /// Gamma in that order, each step clamped to 0..=255.
    #[must_use]
    pub fn new(settings: &ImageSettings) -> Self {
        let gamma = f64::from(settings.gamma_hundredths) / 100.0;
        let brightness = div_round(255 * i32::from(settings.brightness_pct), 100);
        let contrast = i32::from(settings.contrast_pct);
        let mut table = [0_u8; 256];
        for (grey, slot) in (0_i32..).zip(table.iter_mut()) {
            let mut g = grey;
            if settings.invert {
                g = 255 - g;
            }
            g = clamp(g + brightness);
            g = clamp(contrasted(g, contrast));
            *slot = gamma_of(byte(g), gamma);
        }
        Self(table)
    }

    /// The adjusted grey of `grey`.
    #[must_use]
    pub const fn apply(&self, grey: u8) -> u8 {
        self.0[grey as usize]
    }
}

/// `128 + (g − 128)·(100 + c) / (100 − c)` rounded half away from zero; at `c = 100` black below
/// 128 and white from 128.
fn contrasted(g: i32, c: i32) -> i32 {
    if c >= 100 {
        return if g < 128 { 0 } else { 255 };
    }
    128 + div_round((g - 128) * (100 + c), 100 - c)
}

/// `round(255 · (g / 255)^(1 / γ))` with `libm::pow`, the same bits on every target.
fn gamma_of(g: u8, gamma: f64) -> u8 {
    if g == 0 || g == 255 {
        return g;
    }
    let value = 255.0 * libm::pow(f64::from(g) / 255.0, 1.0 / gamma);
    // invariant: `0 < g / 255 < 1` and `γ > 0`, so the power lies in (0, 1).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = value.round().clamp(0.0, 255.0) as u8;
    value
}

/// `n / d` rounded half away from zero, `d > 0`.
const fn div_round(n: i32, d: i32) -> i32 {
    if n >= 0 {
        (2 * n + d) / (2 * d)
    } else {
        -((-2 * n + d) / (2 * d))
    }
}

const fn clamp(g: i32) -> i32 {
    if g < 0 {
        0
    } else if g > 255 {
        255
    } else {
        g
    }
}

fn byte(g: i32) -> u8 {
    u8::try_from(clamp(g)).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(edit: impl FnOnce(&mut ImageSettings)) -> ToneTable {
        let mut settings = ImageSettings::DEFAULT;
        edit(&mut settings);
        ToneTable::new(&settings)
    }

    #[test]
    fn the_default_table_is_the_identity() {
        let t = table(|_| {});
        for g in 0..=255 {
            assert_eq!(t.apply(g), g);
        }
    }

    #[test]
    fn invert_turns_black_white() {
        let t = table(|s| s.invert = true);
        assert_eq!(t.apply(0), 255);
        assert_eq!(t.apply(255), 0);
        assert_eq!(t.apply(100), 155);
    }

    #[test]
    fn full_brightness_is_white_everywhere() {
        let t = table(|s| s.brightness_pct = 100);
        assert!((0..=255).all(|g| t.apply(g) == 255));
        let t = table(|s| s.brightness_pct = -100);
        assert!((0..=255).all(|g| t.apply(g) == 0));
        assert_eq!(
            table(|s| s.brightness_pct = 50).apply(0),
            128,
            "127.5 rounds up"
        );
    }

    #[test]
    fn contrast_spreads_or_flattens_around_128() {
        let full = table(|s| s.contrast_pct = 100);
        assert_eq!(full.apply(127), 0);
        assert_eq!(full.apply(128), 255);
        let none = table(|s| s.contrast_pct = -100);
        assert!((0..=255).all(|g| none.apply(g) == 128));
        let half = table(|s| s.contrast_pct = 50);
        assert_eq!(half.apply(138), 158, "10 x 150 / 50 = 30");
        assert_eq!(half.apply(0), 0);
    }

    #[test]
    fn gamma_above_one_lightens_mid_tones_and_keeps_the_ends() {
        let t = table(|s| s.gamma_hundredths = 200);
        assert_eq!(t.apply(0), 0);
        assert_eq!(t.apply(255), 255);
        assert_eq!(t.apply(128), 181, "255 x sqrt(128 / 255)");
        let dark = table(|s| s.gamma_hundredths = 50);
        assert_eq!(dark.apply(128), 64, "255 x (128 / 255)^2 = 64.25");
    }

    #[test]
    fn every_table_is_monotonic_without_invert() {
        for (b, c, g) in [(0, 0, 100), (-30, 40, 250), (60, -70, 10), (10, 99, 400)] {
            let t = table(|s| {
                s.brightness_pct = b;
                s.contrast_pct = c;
                s.gamma_hundredths = g;
            });
            assert!(
                (0..255).all(|x| t.apply(x) <= t.apply(x + 1)),
                "{b} {c} {g}"
            );
        }
    }
}

//! The dither a picture burns with (ADR 0072 amendment A1, A3, A5; `specs/0135` criteria 8, 22,
//! 23, 67; Frage 284 B: Lowest power stays the operation's).

use std::collections::BTreeMap;

use curvyo_document_core::StoredDither;

use crate::operation_image_options::{Dither, ImageEdit, ImageSettings};

const KEY_THRESHOLD: &str = "threshold_pct";
const KEY_SCREEN_SIZE: &str = "screen_size_um";
const KEY_SCREEN_ANGLE: &str = "screen_angle_deg";

/// The dither of one picture: the mode and the fields that belong to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DitherSettings {
    /// The dither mode.
    pub mode: Dither,
    /// Threshold mode's threshold, 0 to 100 %.
    pub threshold_pct: u8,
    /// The screens' Cell size or Line pitch in micrometres, 200 to 5000 on 10 µm steps.
    pub screen_size_um: u16,
    /// The screens' Angle in whole degrees, 0 to 89.
    pub screen_angle_deg: u8,
}

impl DitherSettings {
    /// The range of the screens' size in micrometres.
    const SCREEN_SIZE_RANGE_UM: std::ops::RangeInclusive<u16> = 200..=5000;
    /// The step of the screens' size in micrometres (two decimals of a mm).
    const SCREEN_SIZE_STEP_UM: u16 = 10;

    /// The dither of a picture that stores none: Floyd-Steinberg with the `0123` defaults.
    pub const DEFAULT: Self = Self {
        mode: Dither::FloydSteinberg,
        threshold_pct: 50,
        screen_size_um: 800,
        screen_angle_deg: 45,
    };

    /// The stored Dither as read: `None` (absent or a shape document-core refused) is the
    /// default; an unknown mode, a missing field, a field out of range or a size off the 10 µm
    /// step reads as the default and is damage (`true`), kept as stored (amendment A5).
    #[must_use]
    pub fn read(stored: Option<&StoredDither>) -> (Self, bool) {
        stored.map_or((Self::DEFAULT, false), |stored| {
            Self::parse(stored).map_or((Self::DEFAULT, true), |read| (read, false))
        })
    }

    fn parse(stored: &StoredDither) -> Option<Self> {
        let field = |key: &str| stored.fields.get(key).copied();
        let mode = Dither::from_key(&stored.mode)?;
        let threshold_pct = u8::try_from(field(KEY_THRESHOLD)?)
            .ok()
            .filter(|n| *n <= ImageSettings::MAX_THRESHOLD)?;
        let screen_size_um = u16::try_from(field(KEY_SCREEN_SIZE)?)
            .ok()
            .filter(|n| Self::SCREEN_SIZE_RANGE_UM.contains(n))
            .filter(|n| n % Self::SCREEN_SIZE_STEP_UM == 0)?;
        let screen_angle_deg = u8::try_from(field(KEY_SCREEN_ANGLE)?)
            .ok()
            .filter(|n| *n <= ImageSettings::MAX_SCREEN_ANGLE)?;
        Some(Self {
            mode,
            threshold_pct,
            screen_size_um,
            screen_angle_deg,
        })
    }

    /// The stored map, every field written (amendment A1).
    #[must_use]
    pub fn stored(&self) -> StoredDither {
        let fields: BTreeMap<String, i64> = [
            (KEY_THRESHOLD, i64::from(self.threshold_pct)),
            (KEY_SCREEN_SIZE, i64::from(self.screen_size_um)),
            (KEY_SCREEN_ANGLE, i64::from(self.screen_angle_deg)),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect();
        StoredDither {
            mode: self.mode.key().to_owned(),
            fields,
        }
    }

    /// The dither with `edit` applied; `None` for an edit that is not the dither's (a tone field
    /// or Lowest power, which stay the operation's).
    #[must_use]
    pub fn with(self, edit: ImageEdit) -> Option<Self> {
        let mut next = self;
        match edit {
            ImageEdit::Dither(mode) => next.mode = mode,
            ImageEdit::Threshold(pct) => next.threshold_pct = pct,
            ImageEdit::ScreenSize(hundredths) => next.screen_size_um = hundredths * 10,
            ImageEdit::ScreenAngle(degrees) => next.screen_angle_deg = degrees,
            ImageEdit::Brightness(_)
            | ImageEdit::Contrast(_)
            | ImageEdit::Gamma(_)
            | ImageEdit::Invert(_)
            | ImageEdit::LowestPower(_) => return None,
        }
        Some(next)
    }
}

impl Default for DitherSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl ImageSettings {
    /// The operation's settings with the picture's dither in place of its own (ADR 0073 note:
    /// the tone fields and Lowest power stay the operation's).
    #[must_use]
    pub const fn with_dither(self, dither: DitherSettings) -> Self {
        let mut next = self;
        next.dither = dither.mode;
        next.threshold_pct = dither.threshold_pct;
        next.screen_size_hundredths = dither.screen_size_um / 10;
        next.screen_angle_deg = dither.screen_angle_deg;
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(mode: &str, fields: &[(&str, i64)]) -> StoredDither {
        StoredDither {
            mode: mode.to_owned(),
            fields: fields.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
        }
    }

    fn dither_of(image: &ImageSettings) -> DitherSettings {
        DitherSettings {
            mode: image.dither,
            threshold_pct: image.threshold_pct,
            screen_size_um: image.screen_size_hundredths * 10,
            screen_angle_deg: image.screen_angle_deg,
        }
    }

    #[test]
    fn absent_reads_as_floyd_steinberg_with_the_0123_defaults() {
        assert_eq!(DitherSettings::read(None), (DitherSettings::DEFAULT, false));
        assert_eq!(dither_of(&ImageSettings::DEFAULT), DitherSettings::DEFAULT);
    }

    #[test]
    fn every_mode_round_trips_through_the_stored_map() {
        for mode in Dither::ALL {
            let dither = DitherSettings {
                mode,
                threshold_pct: 30,
                screen_size_um: 1_610,
                screen_angle_deg: 89,
            };
            let map = dither.stored();
            assert_eq!(map.fields.len(), 3, "every field written");
            assert_eq!(DitherSettings::read(Some(&map)), (dither, false));
        }
    }

    /// Amendment A5, one case per reason.
    #[test]
    fn each_damage_reads_as_the_default_and_counts() {
        let good = [
            ("threshold_pct", 50),
            ("screen_size_um", 800),
            ("screen_angle_deg", 45),
        ];
        let with = |key: &'static str, value: i64| {
            let mut fields = good.to_vec();
            fields.retain(|(k, _)| *k != key);
            fields.push((key, value));
            fields
        };
        let cases = [
            stored("halftone_x", &good),
            stored("halftone", &good[..2]),
            stored("halftone", &with("screen_size_um", 6_000)),
            stored("halftone", &with("screen_size_um", 805)),
            stored("halftone", &with("screen_size_um", 190)),
            stored("halftone", &with("threshold_pct", 101)),
            stored("halftone", &with("screen_angle_deg", 90)),
            stored("halftone", &with("threshold_pct", -1)),
        ];
        for case in cases {
            assert_eq!(
                DitherSettings::read(Some(&case)),
                (DitherSettings::DEFAULT, true),
                "{case:?}"
            );
        }
        assert_eq!(
            DitherSettings::read(Some(&stored("halftone", &good)))
                .0
                .mode,
            Dither::Halftone
        );
    }

    #[test]
    fn the_dither_takes_only_the_dither_edits() {
        let d = DitherSettings::DEFAULT;
        assert_eq!(
            d.with(ImageEdit::Dither(Dither::Halftone)).unwrap().mode,
            Dither::Halftone
        );
        assert_eq!(
            d.with(ImageEdit::ScreenSize(161)).unwrap().screen_size_um,
            1_610
        );
        assert_eq!(d.with(ImageEdit::Threshold(7)).unwrap().threshold_pct, 7);
        assert_eq!(
            d.with(ImageEdit::ScreenAngle(30)).unwrap().screen_angle_deg,
            30
        );
        for edit in [
            ImageEdit::Brightness(1),
            ImageEdit::Contrast(1),
            ImageEdit::Gamma(120),
            ImageEdit::Invert(true),
            ImageEdit::LowestPower(5),
        ] {
            assert_eq!(d.with(edit), None, "{edit:?} is the operation's");
        }
    }

    #[test]
    fn with_dither_keeps_the_operations_tone_fields_and_lowest_power() {
        let operation = ImageSettings {
            brightness_pct: 20,
            gamma_hundredths: 150,
            invert: true,
            lowest_power_tenths: 100,
            ..ImageSettings::DEFAULT
        };
        let picture = DitherSettings {
            mode: Dither::Grey,
            threshold_pct: 10,
            screen_size_um: 1_200,
            screen_angle_deg: 15,
        };
        let burned = operation.with_dither(picture);
        assert_eq!(dither_of(&burned), picture);
        assert_eq!(
            (
                burned.brightness_pct,
                burned.gamma_hundredths,
                burned.invert,
                burned.lowest_power_tenths
            ),
            (20, 150, true, 100)
        );
    }
}

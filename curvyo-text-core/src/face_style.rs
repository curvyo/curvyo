//! The weight class, width class and slant of a face and the style name they make
//! (`specs/0078-font-style-buttons` criterion 28, `adrs.md` decision 1): one band table for every
//! listing, and the name read back.

use skrifa::{FontRef, Tag};

/// The slant of a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slant {
    /// Upright.
    Upright,
    /// A drawn italic.
    Italic,
    /// A slanted roman.
    Oblique,
}

/// A weight band of the style names, in order from thin to black.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WeightBand {
    /// Weight class under 150.
    Thin,
    /// 150 to 249.
    ExtraLight,
    /// 250 to 349.
    Light,
    /// 350 to 449.
    Regular,
    /// 450 to 549.
    Medium,
    /// 550 to 649.
    Semibold,
    /// 650 to 749.
    Bold,
    /// 750 to 849.
    ExtraBold,
    /// 850 and over.
    Black,
}

/// Every band with its style word (empty for Regular) and its centre weight class.
const BANDS: [(WeightBand, &str, u16); 9] = [
    (WeightBand::Thin, "Thin", 100),
    (WeightBand::ExtraLight, "Extra Light", 200),
    (WeightBand::Light, "Light", 300),
    (WeightBand::Regular, "", 400),
    (WeightBand::Medium, "Medium", 500),
    (WeightBand::Semibold, "Semibold", 600),
    (WeightBand::Bold, "Bold", 700),
    (WeightBand::ExtraBold, "Extra Bold", 800),
    (WeightBand::Black, "Black", 900),
];

/// The style words of the OS/2 width classes 1 to 9 (5 is normal).
const WIDTHS: [&str; 9] = [
    "Ultra Condensed",
    "Extra Condensed",
    "Condensed",
    "Semi Condensed",
    "",
    "Semi Expanded",
    "Expanded",
    "Extra Expanded",
    "Ultra Expanded",
];

/// The normal width class.
pub const NORMAL_WIDTH: u16 = 5;

impl WeightBand {
    /// The band of an OS/2 weight class.
    #[must_use]
    pub const fn of(weight: u16) -> Self {
        match weight {
            0..=149 => Self::Thin,
            150..=249 => Self::ExtraLight,
            250..=349 => Self::Light,
            350..=449 => Self::Regular,
            450..=549 => Self::Medium,
            550..=649 => Self::Semibold,
            650..=749 => Self::Bold,
            750..=849 => Self::ExtraBold,
            _ => Self::Black,
        }
    }

    /// The band's centre weight class (Regular 400, Bold 700).
    #[must_use]
    pub fn centre(self) -> u16 {
        BANDS
            .iter()
            .find(|(band, ..)| *band == self)
            .map_or(400, |(.., centre)| *centre)
    }

    /// The band's name: "Extra Light", "Regular", "Bold".
    #[must_use]
    pub fn name(self) -> &'static str {
        match BANDS.iter().find(|(band, ..)| *band == self) {
            Some((_, "", _)) | None => "Regular",
            Some((_, word, _)) => word,
        }
    }
}

/// The weight class, width class and slant of a face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceStyle {
    /// The OS/2 weight class (400 regular, 700 bold).
    pub weight: u16,
    /// The OS/2 width class, 1 to 9 ([`NORMAL_WIDTH`] is normal).
    pub width: u16,
    /// Upright, italic or oblique.
    pub slant: Slant,
}

fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

impl FaceStyle {
    /// Regular: weight 400, normal width, upright.
    pub const REGULAR: Self = Self {
        weight: 400,
        width: NORMAL_WIDTH,
        slant: Slant::Upright,
    };

    /// The weight band.
    #[must_use]
    pub const fn band(&self) -> WeightBand {
        WeightBand::of(self.weight)
    }

    /// Whether the face is italic or oblique.
    #[must_use]
    pub fn is_slanted(&self) -> bool {
        self.slant != Slant::Upright
    }

    /// The style of OpenType face `index` from OS/2 (weight, width, the italic bit, and the
    /// oblique bit from version 4), else Regular with the `post` italic angle deciding the slant.
    /// `None` for bytes that are not an OpenType face.
    #[must_use]
    pub fn from_bytes(bytes: &[u8], index: u32) -> Option<Self> {
        let font = FontRef::from_index(bytes, index).ok()?;
        let os2 = font.table_data(Tag::new(b"OS/2"));
        let os2 = os2.as_ref().map(skrifa::raw::FontData::as_bytes);
        let weight = os2.and_then(|t| be_u16(t, 4)).unwrap_or(400);
        let width = os2.and_then(|t| be_u16(t, 6)).unwrap_or(NORMAL_WIDTH);
        let version = os2.and_then(|t| be_u16(t, 0)).unwrap_or(0);
        let selection = os2.and_then(|t| be_u16(t, 62)).unwrap_or(0);
        let post_italic = font
            .table_data(Tag::new(b"post"))
            .and_then(|post| post.as_bytes().get(4..8).map(|angle| angle != [0, 0, 0, 0]))
            .unwrap_or(false);
        let slant = if selection & 1 != 0 {
            Slant::Italic
        } else if version >= 4 && selection & (1 << 9) != 0 {
            Slant::Oblique
        } else if post_italic {
            Slant::Italic
        } else {
            Slant::Upright
        };
        Some(Self {
            weight,
            width,
            slant,
        })
    }

    /// The style name in the words every listing uses: width, weight, slant, "Regular" when all
    /// three are normal ("Bold Italic", "Condensed Light").
    #[must_use]
    pub fn name(&self) -> String {
        let width = usize::from(self.width)
            .checked_sub(1)
            .and_then(|at| WIDTHS.get(at))
            .copied()
            .unwrap_or("");
        let weight = match self.band() {
            WeightBand::Regular => "",
            band => band.name(),
        };
        let slant = match self.slant {
            Slant::Upright => "",
            Slant::Italic => "Italic",
            Slant::Oblique => "Oblique",
        };
        let words: Vec<&str> = [width, weight, slant]
            .into_iter()
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            "Regular".to_owned()
        } else {
            words.join(" ")
        }
    }

    /// The style a name made by [`FaceStyle::name`] stands for, ignoring case and whitespace
    /// runs; the weight is the band's centre. `None` for any other name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        let mut rest = lower.split_whitespace().collect::<Vec<_>>().join(" ");
        if rest == "regular" {
            return Some(Self::REGULAR);
        }
        if rest.is_empty() {
            return None;
        }
        // Takes `phrase` off the front of `rest` when it is there as whole words.
        let mut take = |phrase: &str| {
            let phrase = phrase.to_lowercase();
            if phrase.is_empty() {
                return false;
            }
            if rest == phrase {
                rest.clear();
                return true;
            }
            if rest.starts_with(&format!("{phrase} ")) {
                rest.drain(..=phrase.len());
                return true;
            }
            false
        };
        let width = (1..=9u16)
            .zip(WIDTHS)
            .find(|(_, word)| take(word))
            .map_or(NORMAL_WIDTH, |(class, _)| class);
        let weight = BANDS
            .iter()
            .find(|(_, word, _)| take(word))
            .map_or(400, |(.., centre)| *centre);
        let slant = if take("Italic") {
            Slant::Italic
        } else if take("Oblique") {
            Slant::Oblique
        } else {
            Slant::Upright
        };
        rest.is_empty().then_some(Self {
            weight,
            width,
            slant,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_in_the_listing_words() {
        let style = |weight, width, slant| FaceStyle {
            weight,
            width,
            slant,
        };
        assert_eq!(FaceStyle::REGULAR.name(), "Regular");
        assert_eq!(style(700, 5, Slant::Italic).name(), "Bold Italic");
        assert_eq!(style(300, 3, Slant::Upright).name(), "Condensed Light");
        assert_eq!(style(200, 5, Slant::Upright).name(), "Extra Light");
        assert_eq!(style(800, 5, Slant::Oblique).name(), "Extra Bold Oblique");
        assert_eq!(style(400, 7, Slant::Italic).name(), "Expanded Italic");
        assert_eq!(style(400, 0, Slant::Upright).name(), "Regular");
    }

    /// Criterion 28 (`adrs.md` decision 1): the name reads back to the style at every band
    /// centre, width and slant.
    #[test]
    fn ac28_parse_reads_every_name_back() {
        for (_, _, weight) in BANDS {
            for width in 1..=9 {
                for slant in [Slant::Upright, Slant::Italic, Slant::Oblique] {
                    let style = FaceStyle {
                        weight,
                        width,
                        slant,
                    };
                    assert_eq!(
                        FaceStyle::parse(&style.name()),
                        Some(style),
                        "{}",
                        style.name()
                    );
                }
            }
        }
    }

    #[test]
    fn parse_ignores_case_and_refuses_other_names() {
        assert_eq!(
            FaceStyle::parse("  bold   ITALIC "),
            Some(FaceStyle {
                weight: 700,
                width: 5,
                slant: Slant::Italic
            })
        );
        assert_eq!(FaceStyle::parse("ExtraLight"), None);
        assert_eq!(FaceStyle::parse("Regular Italic"), None);
        assert_eq!(FaceStyle::parse("Book"), None);
        assert_eq!(FaceStyle::parse(""), None);
    }

    #[test]
    fn bands_of_weight_classes() {
        assert_eq!(WeightBand::of(149), WeightBand::Thin);
        assert_eq!(WeightBand::of(150), WeightBand::ExtraLight);
        assert_eq!(WeightBand::of(449), WeightBand::Regular);
        assert_eq!(WeightBand::of(750), WeightBand::ExtraBold);
        assert_eq!(WeightBand::of(850), WeightBand::Black);
        assert_eq!(WeightBand::Bold.centre(), 700);
        assert_eq!(WeightBand::Regular.name(), "Regular");
        assert_eq!(WeightBand::ExtraBold.name(), "Extra Bold");
    }

    #[test]
    fn styles_of_the_included_faces() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts");
        let read = |file: &str| std::fs::read(dir.join(file)).unwrap_or_default();
        let light = FaceStyle::from_bytes(&read("source-sans-3/SourceSans3-LightIt.otf"), 0);
        assert_eq!(light.map(|s| s.name()).as_deref(), Some("Light Italic"));
        let bold = FaceStyle::from_bytes(&read("source-serif-4/SourceSerif4-Bold.ttf"), 0);
        assert_eq!(bold.map(|s| s.band()), Some(WeightBand::Bold));
        assert_eq!(FaceStyle::from_bytes(b"not a font", 0), None);
    }
}

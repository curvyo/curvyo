//! The 1D kinds as generators: stored parameters in, closed outlines out (ADR 0092 §2 to §4).

use std::borrow::Cow;

use curvyo_document_core::Length;
use curvyo_extension_core::{
    Bound, Edit, GenerateRequest, Generated, GlyphTable, Marks, Param, ParamValue, Part,
    ScaleFactor, Warning,
};

use super::style::{Drawn, LinearStyle, TextLayout, digit_advance, draw};
use super::{LinearCode, LinearError, Printed, encode, schema};
use crate::qr::PART;
use crate::values::Values;

/// A 1D symbology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbology {
    /// Code 128, sets B and C.
    Code128,
    /// Code 39.
    Code39,
    /// EAN-13.
    Ean13,
    /// EAN-8.
    Ean8,
    /// UPC-A.
    UpcA,
}

impl Symbology {
    /// Every 1D symbology, in the order of the manifest.
    pub const ALL: [Self; 5] = [
        Self::Code128,
        Self::Code39,
        Self::Ean13,
        Self::Ean8,
        Self::UpcA,
    ];

    /// The kind id inside the extension.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Code128 => "code-128",
            Self::Code39 => "code-39",
            Self::Ean13 => "ean-13",
            Self::Ean8 => "ean-8",
            Self::UpcA => "upc-a",
        }
    }

    /// The symbology of a kind id.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|symbology| symbology.id() == id)
    }

    /// The name of a new object ("EAN-13").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Code128 => "Code 128",
            Self::Code39 => "Code 39",
            Self::Ean13 => "EAN-13",
            Self::Ean8 => "EAN-8",
            Self::UpcA => "UPC-A",
        }
    }

    /// A visible placeholder; EAN and UPC from the GS1 restricted-circulation ranges, never a real
    /// product (UX notes §11.3).
    #[must_use]
    pub const fn default_content(self) -> &'static str {
        match self {
            Self::Code128 | Self::Code39 => "EXAMPLE-01",
            Self::Ean13 => "2001234567893",
            Self::Ean8 => "20123451",
            Self::UpcA => "212345678909",
        }
    }

    /// Whether the human-readable text is on in a new object.
    #[must_use]
    pub const fn text_default(self) -> bool {
        !matches!(self, Self::Code128 | Self::Code39)
    }

    /// Whether the standard fixes the quiet zones (EAN and UPC have no Quiet zone row).
    #[must_use]
    pub const fn fixed_quiet(self) -> bool {
        !matches!(self, Self::Code128 | Self::Code39)
    }

    /// The characters the content may hold, in words, for the sentence of a refusal.
    const fn accepted(self) -> &'static str {
        match self {
            Self::Code128 => "ASCII letters, digits and symbols",
            Self::Code39 => "A to Z, 0 to 9, space and - . $ / + %",
            _ => "digits",
        }
    }
}

fn warning(id: &'static str, value: Option<u32>, limit: Option<u32>) -> Warning {
    Warning {
        id,
        pieces: Vec::new(),
        value: value.map(ParamValue::Count),
        limit: limit.map(ParamValue::Count),
        param: Some("content"),
        subject: None,
    }
}

/// The result when the content cannot be drawn: no parts (the host keeps the last ones), the
/// parameters as typed and the reason; the content itself is never in it.
fn refused(values: Values<'_>, symbology: Symbology, error: LinearError) -> Generated {
    let (params, _) = values.finish();
    let counted = |n: usize| u32::try_from(n).ok();
    let mut found = match error {
        LinearError::Empty => warning("content_empty", None, None),
        LinearError::WrongLength { short, long } => {
            warning("wrong_length", Some(short), Some(long))
        }
        LinearError::DigitsOnly => warning("digits_only", None, None),
        LinearError::NotEncodable => warning("not_encodable", None, None),
        LinearError::TooLong { characters, limit } => {
            warning("too_many_characters", counted(characters), counted(limit))
        }
    };
    if error == LinearError::NotEncodable {
        found.subject = Some(symbology.accepted());
    }
    Generated {
        params,
        parts: Vec::new(),
        handles: Vec::new(),
        clamps: Vec::new(),
        automatic: Vec::new(),
        states: Vec::new(),
        limits: Vec::new(),
        warnings: vec![found],
        marks: Marks::default(),
    }
}

/// `value` held inside the range of `key`, reported as a clamp when it moved.
fn clamped(values: &mut Values<'_>, key: &'static str, value: f64) -> f64 {
    let (min, max) = values.length_range(key);
    let held = value.clamp(min, max);
    if (held - value).abs() > 1e-12 {
        let bound = if held > value {
            Bound::Minimum
        } else {
            Bound::Maximum
        };
        values.set_clamp(key, ParamValue::Length(Length::from_mm(held)), bound);
    }
    held
}

/// The sizes of one generation, millimetres.
#[derive(Debug, Clone, Copy)]
struct Sizes {
    module: f64,
    bar_height: f64,
    text: f64,
}

struct Frame<'a> {
    code: &'a LinearCode,
    quiet: (f64, f64),
    adjust: f64,
    inverted: bool,
    background: bool,
    text: bool,
    glyphs: Option<&'a GlyphTable>,
}

impl Frame<'_> {
    fn drawn(&self, sizes: Sizes) -> Drawn {
        draw(
            self.code,
            &LinearStyle {
                module: Length::from_mm(sizes.module),
                bar_height: Length::from_mm(sizes.bar_height),
                quiet: self.quiet,
                adjust: self.adjust,
                inverted: self.inverted,
                background: self.background,
                text: self.text.then_some(TextLayout {
                    size: Length::from_mm(sizes.text),
                    glyphs: self.glyphs,
                }),
            },
        )
    }

    fn total(&self) -> f64 {
        self.quiet.0 + self.code.width + self.quiet.1
    }
}

/// The sizes after a free resize by the factors `(x, y)` of the box: x writes the module width and
/// the text size, y the bar height from the box's new height less the band under the bars.
fn stretched(values: &mut Values<'_>, frame: &Frame<'_>, old: Sizes, (x, y): (f64, f64)) -> Sizes {
    let mut sizes = Sizes {
        module: clamped(values, "module_width", old.module * x),
        text: clamped(values, "text_size", old.text * x),
        ..old
    };
    if (y - 1.0).abs() > 1e-9 {
        let before = frame.drawn(old);
        let box_height = before.bottom.as_mm() - before.top.as_mm();
        let probe = frame.drawn(Sizes {
            bar_height: old.bar_height,
            ..sizes
        });
        let band = probe.bottom.as_mm() - probe.top.as_mm() - old.bar_height;
        sizes.bar_height = clamped(values, "bar_height", box_height * y - band);
    }
    sizes
}

/// The sizes after `edit`: Module width is the truth and Width derived from it, unless Width was
/// the field edited; a scale writes all three lengths, a stretch the two factors (ADR 0092 §4).
fn resolve(values: &mut Values<'_>, frame: &Frame<'_>, edit: Option<Edit<'_>>, ean: bool) -> Sizes {
    let old = Sizes {
        module: values.mm("module_width"),
        bar_height: values.mm("bar_height"),
        text: values.mm("text_size"),
    };
    let mut sizes = old;
    match edit {
        Some(Edit::Scale(ScaleFactor(f))) => {
            sizes = Sizes {
                module: clamped(values, "module_width", old.module * f),
                bar_height: clamped(values, "bar_height", old.bar_height * f),
                text: clamped(values, "text_size", old.text * f),
            };
        }
        Some(Edit::Stretch {
            x: ScaleFactor(x),
            y: ScaleFactor(y),
        }) => sizes = stretched(values, frame, old, (x, y)),
        Some(Edit::Param("width")) => {
            let asked = values.mm("width") / frame.total();
            sizes.module = clamped(values, "module_width", asked);
            values.drop_clamp("module_width");
            let width = sizes.module * frame.total();
            if (width - values.mm("width")).abs() > 1e-9 {
                let bound = if width > values.mm("width") {
                    Bound::DependentMinimum
                } else {
                    Bound::Dependent
                };
                values.set_clamp("width", ParamValue::Length(Length::from_mm(width)), bound);
            }
        }
        _ => {}
    }
    if ean && frame.text {
        // The digits stand under their 7-module groups: the size is limited by the module width.
        let most = 7.0 * sizes.module / digit_advance(frame.glyphs);
        if sizes.text > most + 1e-9 {
            sizes.text = most;
            values.set_clamp(
                "text_size",
                ParamValue::Length(Length::from_mm(most)),
                Bound::DependentFor("this module width"),
            );
        }
    }
    sizes
}

/// Only the bars that name the symbology, for the ghost under the pointer.
fn thinned(code: &LinearCode, ean: bool) -> LinearCode {
    let width = code.width;
    let mut thin = code.clone();
    thin.bars.retain(|bar| {
        if ean {
            bar.guard
        } else {
            bar.x < 11.0 || bar.x + bar.width > width - 13.0
        }
    });
    thin.printed = Printed::Line(String::new());
    thin
}

/// What a commit of the content does: the normalised text is written back (the field shows what is
/// encoded) and a replaced check digit is said, without the digit.
fn commit_content(
    values: &mut Values<'_>,
    code: &LinearCode,
    content: &str,
    edit: Option<Edit<'_>>,
) -> Vec<Warning> {
    let mut warnings = Vec::new();
    if matches!(edit, Some(Edit::Param("content"))) {
        if code.normalised != content {
            values.put("content", &ParamValue::Text(code.normalised.clone()));
        }
        if code.corrected == Some(true) {
            warnings.push(Warning {
                id: "check_digit_corrected",
                pieces: Vec::new(),
                value: None,
                limit: None,
                param: Some("content"),
                subject: None,
            });
        }
    }
    warnings
}

/// Generates the 1D code of `symbology` `request` names.
pub(crate) fn generate(request: &GenerateRequest<'_>, symbology: Symbology) -> Generated {
    let groups = schema::groups(symbology);
    let schema: Vec<Param> = groups.into_iter().flat_map(|group| group.params).collect();
    let mut values = Values::read(&schema, request.params);
    let ratio = if symbology == Symbology::Code39 {
        values.ratio("wide_narrow")
    } else {
        3.0
    };
    let content = values.text("content");
    let code = match encode(symbology, &content, ratio) {
        Ok(code) => code,
        Err(error) => return refused(values, symbology, error),
    };
    let mut warnings = commit_content(&mut values, &code, &content, request.edit);
    let quiet = code.fixed_quiet.unwrap_or_else(|| {
        let q = f64::from(values.count("bar_quiet_zone"));
        (q, q)
    });
    let text = values.switch("text");
    let ghost = matches!(request.edit, Some(Edit::Draft));
    let drawn_code = if ghost {
        thinned(&code, code.fixed_quiet.is_some())
    } else {
        code.clone()
    };
    let frame = Frame {
        code: &drawn_code,
        quiet,
        adjust: f64::from(values.percent("bar_adjust")) / 100.0,
        inverted: values.switch("inverted"),
        background: values.switch("background") && !ghost,
        text: text && !ghost,
        glyphs: request.glyphs,
    };
    let sizes = resolve(
        &mut values,
        &frame,
        request.edit,
        code.fixed_quiet.is_some(),
    );
    values.put_mm("module_width", sizes.module);
    values.put_mm("bar_height", sizes.bar_height);
    values.put_mm("text_size", sizes.text);
    values.put_mm("width", sizes.module * frame.total());
    let drawn = frame.drawn(sizes);
    if text && drawn.missing && !ghost {
        warnings.push(Warning {
            id: "glyph_missing",
            pieces: Vec::new(),
            value: None,
            limit: None,
            param: Some("font"),
            subject: None,
        });
    }
    let background = drawn
        .background
        .and_then(|ground| crate::paint::background(&values, ground));
    let (params, clamps) = values.finish();
    assembled(params, clamps, background, drawn.paths, warnings)
}

/// The result of a generation that drew the code.
fn assembled(
    params: curvyo_extension_core::StoredParams,
    clamps: Vec<curvyo_extension_core::Clamp>,
    background: Option<Part>,
    paths: Vec<curvyo_extension_core::ClosedPath>,
    warnings: Vec<Warning>,
) -> Generated {
    let mut parts: Vec<Part> = background.into_iter().collect();
    parts.push(Part {
        key: Cow::Borrowed(PART),
        paths,
        paint: None,
    });
    Generated {
        params,
        parts,
        handles: Vec::new(),
        clamps,
        automatic: Vec::new(),
        states: Vec::new(),
        limits: Vec::new(),
        warnings,
        marks: Marks::default(),
    }
}

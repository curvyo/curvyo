//! The trace job with OCR lines and the text section of its numbers
//! (`specs/0091-trace-text-recognition` `adrs.md` decisions 1 to 3): the text pass runs first, the
//! main trace walks the filled picture, and the accepted lines follow the outlines.

use crate::{PixelMap, Raster, Rgb8, TraceParams};
use kurbo::{BezPath, Vec2};

use crate::mask::Mask;
use crate::ocr::accept::OcrLine;
use crate::text_pass::TextPass;
use crate::text_pass::text_pass;
use crate::wire::{TraceStatus, count, mono_wire, read_outlines, status_only, write_outline};

/// The number that opens the text section (no count is negative).
pub(crate) const TEXT_MARK: f64 = -1.0;

/// The number that opens the lines' fills after the lines of a multicolour result (slice 2); a
/// black and white text section has none and is as in slice 1.
const FILL_MARK: f64 = -2.0;

/// The number that opens the excluded lines after the fills (their count and places); absent when
/// the maker excluded none of the lines' fills.
const EXCLUDED_MARK: f64 = -3.0;

/// One accepted line of a result.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceTextWire {
    /// The OCR's string.
    pub text: String,
    /// The line's direction in document millimetres.
    pub direction: Vec2,
    /// Pixels left out of the main trace for it.
    pub masked: usize,
    /// The OCR line passes the garbage filter (Frage 115 B / 117 B): it becomes text even
    /// without a sure letter.
    pub plausible: bool,
    /// The size of one picture pixel along the line, document millimetres (the fit's tolerance,
    /// criterion 26).
    pub pixel: f64,
    /// Where the reader saw each character, mm along the line's unit direction (round 3).
    pub positions: Vec<f64>,
    /// The text's fill in a multicolour result: the palette or background colour nearest the
    /// line's text pixels (slice 2, decision 6); `None` in black and white and in the text pass
    /// alone.
    pub fill: Option<Rgb8>,
    /// The maker excluded the line's fill: it is left out, as the shapes of that colour are
    /// (multicolour only).
    pub excluded: bool,
    /// The line's own outlines in document millimetres.
    pub outlines: Vec<BezPath>,
}

impl TraceTextWire {
    /// Whether `other` is the same line, its fill aside (the text pass alone has none).
    #[must_use]
    pub fn same_line(&self, other: &Self) -> bool {
        Self {
            fill: None,
            excluded: false,
            ..self.clone()
        } == Self {
            fill: None,
            excluded: false,
            ..other.clone()
        }
    }
}

/// The text section of a result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraceTexts {
    /// Lines the OCR found, accepted or not.
    pub found: usize,
    /// The accepted lines with a trace.
    pub lines: Vec<TraceTextWire>,
}

/// One trace with the OCR `lines` (in this picture's pixels): the text pass, then [`mono_wire`]
/// over the filled picture, then the text section. With a `mask` (`specs/0094` criterion 11)
/// every line is left out of the main trace, only those whose box centre the mask keeps stay as
/// text, and the outlines are clipped. Without lines the numbers are exactly those of
/// [`mono_wire`] (criteria 12 and 18).
pub(crate) fn text_mono_wire(
    raster: Raster<'_>,
    map: PixelMap,
    params: &TraceParams,
    lines: &[OcrLine],
    mask: Option<&dyn Mask>,
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    if lines.is_empty() {
        return mono_wire(raster, map, params, mask, on_percent);
    }
    let mut pass = text_pass(raster, map, params, lines);
    keep_lines_in(&mut pass, mask);
    let filled = Raster {
        rgba: &pass.rgba,
        ..raster
    };
    let mut wire = mono_wire(filled, map, params, mask, on_percent);
    if wire.first() != Some(&0.0) {
        return wire;
    }
    write_texts(&mut wire, &pass, &[]);
    wire
}

/// Drops the lines of `pass` whose box centre `region` does not keep (`specs/0094` criterion 11);
/// their pixels stay left out of the main trace, so they are neither text nor traced.
pub(crate) fn keep_lines_in(pass: &mut TextPass, region: Option<&dyn Mask>) {
    if let Some(region) = region {
        pass.regions.retain(|line| region.keeps(line.centre));
    }
}

/// The text section of `pass` after a result's numbers; with `fills` (one per line, multicolour:
/// the colour and whether it is excluded) [`FILL_MARK`] and the colours after the lines, then
/// [`EXCLUDED_MARK`] and the excluded lines when there are any.
pub(crate) fn write_texts(wire: &mut Vec<f64>, pass: &TextPass, fills: &[(Rgb8, bool)]) {
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.extend([TEXT_MARK, pass.found as f64, pass.regions.len() as f64]);
        for region in &pass.regions {
            let chars: Vec<char> = region.text.chars().collect();
            wire.push(chars.len() as f64);
            wire.extend(chars.iter().map(|&c| f64::from(u32::from(c))));
            wire.extend([
                region.direction.x,
                region.direction.y,
                region.masked as f64,
                f64::from(u8::from(region.plausible)),
                region.pixel,
                region.positions.len() as f64,
            ]);
            wire.extend(region.positions.iter().copied());
            wire.extend([region.outlines.len() as f64]);
            for outline in &region.outlines {
                write_outline(wire, outline);
            }
        }
        if !fills.is_empty() {
            wire.push(FILL_MARK);
            for (c, _) in fills {
                wire.extend([f64::from(c.r), f64::from(c.g), f64::from(c.b)]);
            }
            let excluded: Vec<f64> = (0..fills.len())
                .filter(|&k| fills[k].1)
                .map(|k| k as f64)
                .collect();
            if !excluded.is_empty() {
                wire.extend([EXCLUDED_MARK, excluded.len() as f64]);
                wire.extend(excluded);
            }
        }
    }
}

/// The text pass alone (`specs/0091` criterion 25: the lines are decided before the main trace):
/// the numbers of a result without outlines, then the text section of the accepted lines whose
/// box centre `mask` keeps (`specs/0094` criterion 11), as [`text_mono_wire`] would write it.
pub(crate) fn text_regions_wire(
    raster: Raster<'_>,
    map: PixelMap,
    params: &TraceParams,
    lines: &[OcrLine],
    mask: Option<&dyn Mask>,
) -> Vec<f64> {
    let mut pass = text_pass(raster, map, params, lines);
    keep_lines_in(&mut pass, mask);
    let mut wire = status_only(TraceStatus::Traced);
    write_texts(&mut wire, &pass, &[]);
    wire
}

/// The text section at the start of `tail` (it starts with [`TEXT_MARK`]) and the numbers it
/// took; `None` unless it is well formed.
pub(crate) fn read_texts(tail: &[f64]) -> Option<(TraceTexts, usize)> {
    let mut at = 1;
    let next = |at: &mut usize| -> Option<f64> {
        let value = *tail.get(*at)?;
        *at += 1;
        value.is_finite().then_some(value)
    };
    let found = count(next(&mut at)?)?;
    let n = count(next(&mut at)?)?;
    let mut lines = Vec::with_capacity(n.min(tail.len()));
    for _ in 0..n {
        let chars = count(next(&mut at)?)?;
        let mut text = String::new();
        for _ in 0..chars {
            let code = u32::try_from(count(next(&mut at)?)?).ok()?;
            text.push(char::from_u32(code)?);
        }
        let direction = Vec2::new(next(&mut at)?, next(&mut at)?);
        let masked = count(next(&mut at)?)?;
        let plausible = next(&mut at)? != 0.0;
        let pixel = next(&mut at)?;
        let positions_count = count(next(&mut at)?)?;
        let mut positions = Vec::with_capacity(positions_count.min(tail.len()));
        for _ in 0..positions_count {
            positions.push(next(&mut at)?);
        }
        let outlines = count(next(&mut at)?)?;
        let outlines = read_outlines(tail, &mut at, outlines)?;
        lines.push(TraceTextWire {
            text,
            direction,
            masked,
            plausible,
            pixel,
            positions,
            fill: None,
            excluded: false,
            outlines,
        });
    }
    if tail.get(at) == Some(&FILL_MARK) {
        at += 1;
        let byte = |v: f64| u8::try_from(count(v)?).ok();
        for line in &mut lines {
            line.fill = Some(Rgb8::new(
                byte(next(&mut at)?)?,
                byte(next(&mut at)?)?,
                byte(next(&mut at)?)?,
            ));
        }
        if tail.get(at) == Some(&EXCLUDED_MARK) {
            at += 1;
            for _ in 0..count(next(&mut at)?)? {
                lines.get_mut(count(next(&mut at)?)?)?.excluded = true;
            }
        }
    }
    Some((TraceTexts { found, lines }, at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{Job, Mode, run, text_regions};
    use crate::wire::read_wire;

    /// A 40 x 20 picture with a black bar (the "text") on white.
    fn picture() -> Vec<u8> {
        let mut rgba = vec![255_u8; 40 * 20 * 4];
        for y in 8..12 {
            for x in 8..32 {
                let i = (y * 40 + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
        rgba
    }

    fn line(text: &str) -> OcrLine {
        OcrLine {
            quad: [[6.0, 6.0], [34.0, 6.0], [34.0, 14.0], [6.0, 14.0]],
            text: text.to_owned(),
            confidence: vec![0.9; text.chars().count()],
            positions: Vec::new(),
        }
    }

    const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];

    #[test]
    fn the_text_section_round_trips_and_the_bar_leaves_the_main_trace() {
        let params = TraceParams::with_threshold(128);
        let wire = run(
            &Job {
                raster: Raster {
                    width: 40,
                    height: 20,
                    rgba: &picture(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Mono(&params),
                lines: &[line("Grüße")],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read_wire(&wire).unwrap();
        assert_eq!(read.outlines.len(), 0);
        assert_eq!(read.texts.found, 1);
        assert_eq!(read.texts.lines[0].text, "Grüße");
        assert_ne!(read.texts.lines[0].outlines.len(), 0);
        // Frage 115 B and criterion 26: the garbage filter and the pixel size travel along.
        assert!(read.texts.lines[0].plausible);
        assert!((read.texts.lines[0].pixel - 0.1).abs() < 1e-12);
    }

    /// Criterion 25: the text pass alone gives the text section the full job gives, no outline.
    #[test]
    fn the_regions_are_the_text_section_of_the_job() {
        let params = TraceParams::with_threshold(128);
        let lines = [line("Grüße")];
        let full = read_wire(&run(
            &Job {
                raster: Raster {
                    width: 40,
                    height: 20,
                    rgba: &picture(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Mono(&params),
                lines: &lines,
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        ))
        .unwrap();
        let regions = read_wire(&text_regions(&Job {
            raster: Raster {
                width: 40,
                height: 20,
                rgba: &picture(),
            },
            map: PixelMap::new(MAP),
            mode: Mode::Mono(&params),
            lines: &lines,
            mask: None,
            shapes: false,
        }))
        .unwrap();
        assert_eq!(regions.texts, full.texts);
        assert_eq!(regions.outlines.len(), 0);
    }

    /// `specs/0094` criterion 11: a line whose box centre the mask does not keep is left out of
    /// the main trace and is no text; with the mask keeping it, it is text.
    #[test]
    fn a_mask_keeps_the_lines_by_their_box_centre() {
        use crate::mask::test_double::BoxMask;
        let params = TraceParams::with_threshold(128);
        let lines = [line("Grüße")];
        // The line's box centre is at pixel (20, 10): 2 mm, 1 mm.
        let run = |mask: &BoxMask| {
            read_wire(&run(
                &Job {
                    raster: Raster {
                        width: 40,
                        height: 20,
                        rgba: &picture(),
                    },
                    map: PixelMap::new(MAP),
                    mode: Mode::Mono(&params),
                    lines: &lines,
                    mask: Some(mask),
                    shapes: false,
                },
                &mut |_| {},
            ))
            .unwrap()
        };
        let keeps = BoxMask {
            min: kurbo::Point::new(1.0, 0.5),
            max: kurbo::Point::new(3.0, 1.5),
        };
        let drops = BoxMask {
            min: kurbo::Point::new(3.0, 0.5),
            max: kurbo::Point::new(4.0, 1.5),
        };
        let kept = run(&keeps);
        assert_eq!((kept.texts.found, kept.texts.lines.len()), (1, 1));
        let left = run(&drops);
        assert_eq!((left.texts.found, left.texts.lines.len()), (1, 0));
        assert_eq!(
            left.outlines.len(),
            0,
            "its pixels stay out of the main trace"
        );
        let only_text = read_wire(&text_regions(&Job {
            raster: Raster {
                width: 40,
                height: 20,
                rgba: &picture(),
            },
            map: PixelMap::new(MAP),
            mode: Mode::Mono(&params),
            lines: &lines,
            mask: Some(&drops),
            shapes: false,
        }))
        .unwrap();
        assert_eq!(only_text.texts.lines.len(), 0);
    }

    /// Criteria 12 and 18: no lines, the same numbers as before.
    #[test]
    fn without_lines_the_job_is_the_plain_trace() {
        let params = TraceParams::with_threshold(128);
        let plain = run(
            &Job {
                raster: Raster {
                    width: 40,
                    height: 20,
                    rgba: &picture(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Mono(&params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        assert_eq!(
            run(
                &Job {
                    raster: Raster {
                        width: 40,
                        height: 20,
                        rgba: &picture()
                    },
                    map: PixelMap::new(MAP),
                    mode: Mode::Mono(&params),
                    lines: &[],
                    mask: None,
                    shapes: false
                },
                &mut |_| {}
            ),
            plain
        );
        let read = read_wire(&plain).unwrap();
        assert_eq!(read.texts, TraceTexts::default());
    }
}

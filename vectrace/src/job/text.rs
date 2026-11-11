//! The trace job with OCR lines and the text section of its numbers
//! (`specs/0091-trace-text-recognition` `adrs.md` decisions 1 to 3): the text pass runs first, the
//! main trace walks the filled picture, and the accepted lines follow the outlines.

use crate::{PixelMap, Raster, TraceParams};

use crate::mask::Mask;
use crate::ocr::OcrLine;
use crate::text_pass::TextPass;
use crate::text_pass::text_pass;
use crate::wire::{Status, mono_wire, status_only, write_texts};

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
    let mut wire = status_only(Status::Traced);
    write_texts(&mut wire, &pass, &[]);
    wire
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::{Job, Mode, run, text_regions};
    use crate::wire::read;

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
        let read = read(&wire).unwrap();
        assert_eq!(read.outlines.len(), 0);
        assert_eq!(read.texts.found, 1);
        assert_eq!(read.texts.lines[0].text, "Grüße");
        assert_ne!(read.texts.lines[0].outlines.len(), 0);
        // Frage 115 B and criterion 26: the garbage filter and the pixel size travel along.
        assert!(read.texts.lines[0].plausible);
        assert!((read.texts.lines[0].pixel.as_mm() - 0.1).abs() < 1e-12);
    }

    /// Criterion 25: the text pass alone gives the text section the full job gives, no outline.
    #[test]
    fn the_regions_are_the_text_section_of_the_job() {
        let params = TraceParams::with_threshold(128);
        let lines = [line("Grüße")];
        let full = read(&run(
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
        let regions = read(&text_regions(&Job {
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
            read(&run(
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
        let only_text = read(&text_regions(&Job {
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
        let read = read(&plain).unwrap();
        assert_eq!(read.texts, crate::wire::Texts::default());
    }
}

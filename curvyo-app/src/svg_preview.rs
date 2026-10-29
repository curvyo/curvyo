//! The small picture of an SVG and its page size for the import dialog's preview
//! (`specs/0087-image-clipboard-and-drop` criterion 26, lead decision 2026-10-29).
//!
//! Bytes in, picture out; only the faces of its texts are read, through [`PreviewFonts`]. The
//! file is read by `curvyo-svg-core`'s `read_svg`, the importer of Import SVG with its
//! hostile-input limits, so the preview shows the page Import SVG would place; [`crate::raster`]
//! draws it on a white page. Fills and solid strokes are drawn, hidden objects and guides are not.
//! Texts are drawn in the faces Import SVG would give them (a missing face in the substitute),
//! laid out by `curvyo-ui-core` as the text objects of the import; embedded images and dashes are
//! left out (a preview).

use curvyo_image_core::WorkingImage;
use curvyo_svg_core::{Cap, Join, LeafKind, LeafStyle, SvgDrawing, SvgError, SvgNode, read_svg};
use curvyo_ui_core::{ImportFaces, svg_text_glyphs};
use kurbo::{Affine, BezPath, Rect, Shape, Stroke, StrokeOpts};

use crate::preview_fonts::PreviewFonts;
use crate::raster::Canvas;

/// The page colour of the preview.
const PAPER: [u8; 3] = [255, 255, 255];
/// The thinnest stroke the preview draws, in pixels, so a hairline cut stays visible.
const MIN_STROKE_PX: f64 = 1.0;
/// The stroke outline's tolerance in pixels.
const STROKE_TOLERANCE: f64 = 0.1;
/// How far a stroke reaches past its path, in stroke widths: half the width times the miter
/// limit of 4 (kurbo's default), and a square cap's corner within it.
const MITER_REACH: f64 = 2.0;

/// The work units a letter is charged before its text is shaped and outlined, an estimate of
/// its glyph's outline and fill at preview sizes (a capital of Source Sans 3 has some 12 segments,
/// each a fill edge too); its segments are charged again before it is drawn. A text of
/// `MAX_TEXT_CHARS` letters is over the budget and never shaped.
const UNITS_PER_LETTER: usize = 24;

/// An SVG's preview: its page drawn with the longest side at most the asked size, and the page
/// size in millimetres as Import SVG places it.
#[derive(Debug, PartialEq)]
pub(crate) struct SvgThumbnail {
    /// The page, drawn.
    pub(crate) picture: WorkingImage,
    /// The page's width and height in millimetres.
    pub(crate) page_mm: (f64, f64),
}

/// The preview of the SVG `bytes`, the page's longest side `longest` pixels, aspect kept;
/// `Ok(None)` for a file that draws nothing and gives no size. Its texts are drawn in the faces
/// of `fonts`, which reads the ones it needs.
///
/// # Errors
/// Every refusal of [`read_svg`]; its limits apply before and while the XML is parsed.
pub(crate) fn svg_thumbnail(
    bytes: &[u8],
    longest: u32,
    fonts: &mut PreviewFonts,
) -> Result<Option<SvgThumbnail>, SvgError> {
    let drawing = read_svg(bytes)?;
    let Some(page_mm) = drawing.page else {
        return Ok(None);
    };
    let scale = f64::from(longest) / page_mm.0.max(page_mm.1);
    let side = |mm: f64| {
        // invariant: at most `longest`, at least 1.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = (mm * scale).round().clamp(1.0, f64::from(longest)) as u32;
        px
    };
    let (width, height) = (side(page_mm.0), side(page_mm.1));
    let mut canvas = Canvas::new(width, height, PAPER);
    let faces = fonts.faces_for(&drawing);
    draw(&drawing, Affine::scale(scale), &faces, &mut canvas);
    Ok(Some(SvgThumbnail {
        picture: WorkingImage {
            width,
            height,
            pixels: canvas.pixels,
            original_size: (width, height),
            truncated: false,
            animated: false,
        },
        page_mm,
    }))
}

/// The size line of a page of `width` x `height` mm, "210 × 297 mm", at most two decimals.
pub(crate) fn mm_size_line((width, height): (f64, f64)) -> String {
    format!("{} × {} mm", mm(width), mm(height))
}

fn mm(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// One leaf in document order of all leaves: its own outline, or the clone link.
struct Indexed<'a> {
    outline: Option<BezPath>,
    style: &'a LeafStyle,
    clone_of: Option<(usize, Affine)>,
}

/// What a walk over the tree draws with.
struct Painter<'a> {
    leaves: &'a [Indexed<'a>],
    faces: &'a ImportFaces<'a>,
    /// Millimetres to pixels.
    to_px: Affine,
}

/// Draws the visible leaves and texts of `drawing` in z-order through `to_px` (millimetres to
/// pixels), the texts in `faces`.
fn draw(drawing: &SvgDrawing, to_px: Affine, faces: &ImportFaces<'_>, canvas: &mut Canvas) {
    let mut leaves = Vec::new();
    index(&drawing.nodes, &mut leaves);
    let painter = Painter {
        leaves: &leaves,
        faces,
        to_px,
    };
    let mut at = 0;
    painter.walk(&drawing.nodes, 1.0, &mut at, canvas);
}

fn index<'a>(nodes: &'a [SvgNode], leaves: &mut Vec<Indexed<'a>>) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => index(children, leaves),
            SvgNode::Leaf { leaf, .. } => leaves.push(Indexed {
                outline: leaf.kind.outline_mm(),
                style: &leaf.style,
                clone_of: match leaf.kind {
                    LeafKind::Clone { original, matrix } => Some((original, matrix)),
                    _ => None,
                },
            }),
            SvgNode::Text { .. } | SvgNode::Image { .. } => {}
        }
    }
}

impl Painter<'_> {
    fn walk(&self, nodes: &[SvgNode], opacity: f64, at: &mut usize, canvas: &mut Canvas) {
        let scale = self.to_px.as_coeffs()[0];
        for node in nodes {
            let state = node.state();
            let visible = !state.hidden && !state.guide && !canvas.exhausted();
            let opacity = opacity * state.opacity;
            match node {
                SvgNode::Group { children, .. } => {
                    if visible {
                        self.walk(children, opacity, at, canvas);
                    } else {
                        *at += count_leaves(children);
                    }
                }
                SvgNode::Leaf { .. } => {
                    if visible && let Some((path, style)) = resolve(self.leaves, *at) {
                        paint(&(self.to_px * path), style, scale, opacity, canvas);
                    }
                    *at += 1;
                }
                SvgNode::Text { text, .. } if visible => {
                    // Charged before it is shaped: a text may hold 1,000,000 characters.
                    let letters = text.chars.iter().filter(|c| !c.ch.is_whitespace()).count();
                    if !canvas.charge(letters.saturating_mul(UNITS_PER_LETTER)) {
                        return;
                    }
                    let carrier = |leaf: usize| resolve(self.leaves, leaf).map(|(path, _)| path);
                    for glyphs in svg_text_glyphs(text, &carrier, self.faces) {
                        // The glyphs are charged their segments before they are drawn.
                        if canvas.exhausted() || !canvas.charge(glyphs.path.elements().len()) {
                            return;
                        }
                        paint(
                            &(self.to_px * glyphs.path),
                            &glyphs.paint,
                            scale,
                            opacity,
                            canvas,
                        );
                    }
                }
                SvgNode::Text { .. } | SvgNode::Image { .. } => {}
            }
        }
    }
}

fn count_leaves(nodes: &[SvgNode]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            SvgNode::Group { children, .. } => count_leaves(children),
            SvgNode::Leaf { .. } => 1,
            SvgNode::Text { .. } | SvgNode::Image { .. } => 0,
        })
        .sum()
}

/// The outline in millimetres and the style of leaf `index`, a clone through its chain (the
/// original's style); `None` for a broken or overlong chain.
fn resolve<'a>(leaves: &'a [Indexed<'a>], index: usize) -> Option<(BezPath, &'a LeafStyle)> {
    let mut matrix = Affine::IDENTITY;
    let mut at = index;
    for _ in 0..=curvyo_svg_core::limits::MAX_CLONE_CHAIN {
        let leaf = leaves.get(at)?;
        if let Some(outline) = &leaf.outline {
            return Some((matrix * outline.clone(), leaf.style));
        }
        let (original, step) = leaf.clone_of?;
        matrix *= step;
        at = original;
    }
    None
}

/// Fills and strokes `path` (pixels) with `style`; `scale` is pixels per millimetre. A path
/// whose box, widened by the stroke, misses the picture is skipped; a stroke is charged its
/// input segments before it is outlined.
fn paint(path: &BezPath, style: &LeafStyle, scale: f64, opacity: f64, canvas: &mut Canvas) {
    let stroke = style
        .stroke
        .as_ref()
        .zip(style.stroke_width_mm)
        .map(|(paint, width_mm)| (paint, (width_mm * scale).max(MIN_STROKE_PX)));
    let reach = stroke.map_or(0.0, |(_, width)| width * MITER_REACH);
    let picture = Rect::new(0.0, 0.0, f64::from(canvas.width), f64::from(canvas.height));
    let bounds = path.bounding_box().inflate(reach, reach);
    if !bounds.is_finite() || bounds.intersect(picture).is_zero_area() {
        return;
    }
    if let Some(fill) = &style.fill {
        canvas.fill(path, style.even_odd, fill.rgb, fill.opacity * opacity);
    }
    if let Some((stroke, width)) = stroke {
        if canvas.exhausted() || !canvas.charge(path.elements().len()) {
            return;
        }
        let outline = kurbo::stroke(
            path.iter(),
            &Stroke::new(width)
                .with_caps(match style.cap.unwrap_or(Cap::Butt) {
                    Cap::Butt => kurbo::Cap::Butt,
                    Cap::Round => kurbo::Cap::Round,
                    Cap::Square => kurbo::Cap::Square,
                })
                .with_join(match style.join.unwrap_or(Join::Miter) {
                    Join::Miter => kurbo::Join::Miter,
                    Join::Round => kurbo::Join::Round,
                    Join::Bevel => kurbo::Join::Bevel,
                }),
            &StrokeOpts::default(),
            STROKE_TOLERANCE,
        );
        canvas.fill(&outline, false, stroke.rgb, stroke.opacity * opacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(picture: &WorkingImage, x: u32, y: u32) -> [u8; 4] {
        picture.pixel(x, y).unwrap()
    }

    fn thumbnail(svg: &str) -> SvgThumbnail {
        svg_thumbnail(svg.as_bytes(), 256, &mut PreviewFonts::none())
            .unwrap()
            .unwrap()
    }

    #[test]
    fn an_a4_page_gives_a_256_px_tall_picture_and_its_mm_size() {
        let a4 = thumbnail(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 210 297">
                <rect x="0" y="0" width="105" height="297" fill="#000"/>
            </svg>"##,
        );
        assert_eq!((a4.picture.width, a4.picture.height), (181, 256));
        assert_eq!(mm_size_line(a4.page_mm), "210 × 297 mm");
        assert_eq!(
            pixel(&a4.picture, 40, 128),
            [0, 0, 0, 255],
            "the black half"
        );
        assert_eq!(
            pixel(&a4.picture, 140, 128),
            [255, 255, 255, 255],
            "the paper"
        );
    }

    #[test]
    fn the_page_follows_0024s_size_rules() {
        // viewBox only: the viewBox read as pixels (criterion 17).
        let only_viewbox = thumbnail(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50"><rect width="10" height="10"/></svg>"#,
        );
        assert_eq!(mm_size_line(only_viewbox.page_mm), "26.46 × 13.23 mm");
        assert_eq!(
            (only_viewbox.picture.width, only_viewbox.picture.height),
            (256, 128)
        );
        // Neither size nor viewBox: from (0, 0) to the drawing's right and bottom edge.
        let content = thumbnail(
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="378" height="189"/></svg>"#,
        );
        assert_eq!(mm_size_line(content.page_mm), "101 × 51 mm");
    }

    #[test]
    fn a_hairline_stroke_stays_visible_and_hidden_objects_are_not_drawn() {
        let t = thumbnail(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="1000mm" height="1000mm" viewBox="0 0 1000 1000">
                <path d="M 0 501.953125 L 1000 501.953125" stroke="#ff0000" stroke-width="0.01" fill="none"/>
                <rect x="0" y="0" width="1000" height="400" fill="#000" style="display:none"/>
            </svg>"##,
        );
        // The line runs through the centre of pixel row 128 (128.5 px).
        let line = pixel(&t.picture, 50, 128);
        assert_eq!(line, [255, 0, 0, 255], "the 0.01 mm line shows 1 px wide");
        assert_eq!(pixel(&t.picture, 50, 50), [255, 255, 255, 255]);
    }

    #[test]
    fn a_clone_draws_its_original_moved() {
        let t = thumbnail(
            r##"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100mm" height="100mm" viewBox="0 0 100 100">
                <rect id="r" x="0" y="0" width="20" height="20" fill="#000"/>
                <use xlink:href="#r" transform="translate(60 60)"/>
            </svg>"##,
        );
        assert_eq!(pixel(&t.picture, 10, 10)[0], 0);
        assert_eq!(pixel(&t.picture, 180, 180)[0], 0, "the clone");
        assert_eq!(pixel(&t.picture, 100, 100)[0], 255);
    }

    #[test]
    fn the_importers_refusals_and_an_empty_file_give_no_picture() {
        let over = vec![b' '; curvyo_svg_core::limits::MAX_FILE_BYTES + 1];
        assert_eq!(
            svg_thumbnail(&over, 256, &mut PreviewFonts::none()),
            Err(SvgError::TooLarge)
        );
        let entities = br#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "&b;&b;"><!ENTITY b "x">]><svg xmlns="http://www.w3.org/2000/svg">&a;</svg>"#;
        assert_eq!(
            svg_thumbnail(entities, 256, &mut PreviewFonts::none()),
            Err(SvgError::Entities)
        );
        assert_eq!(
            svg_thumbnail(&[0x1f, 0x8b, 8, 0], 256, &mut PreviewFonts::none()),
            Err(SvgError::Compressed)
        );
        assert_eq!(
            svg_thumbnail(
                br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#,
                256,
                &mut PreviewFonts::none()
            ),
            Ok(None)
        );
    }

    use std::fmt::Write as _;

    /// The architect's zigzag `M0 0 L0 297 L1 0 L1 297 …` with `teeth` teeth, inside 0024's
    /// limits; with a stroke so the stroker is charged too.
    fn zigzag(teeth: u32) -> String {
        let mut d = String::from("M0 0");
        for tooth in 0..teeth {
            write!(d, " L{tooth} 297 L{} 0", tooth + 1).unwrap();
        }
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="210mm" height="297mm" viewBox="0 0 210 297"><path d="{d}" fill="#000" stroke="#f00" stroke-width="0.1"/></svg>"##
        )
    }

    #[test]
    fn a_full_height_zigzag_is_cut_short_by_the_work_budget() {
        let t = thumbnail(&zigzag(200_000));
        assert_eq!((t.picture.width, t.picture.height), (181, 256));
        assert_eq!(mm_size_line(t.page_mm), "210 × 297 mm");
    }

    #[test]
    fn the_mm_line_keeps_at_most_two_decimals() {
        assert_eq!(mm_size_line((210.0, 297.0)), "210 × 297 mm");
        assert_eq!(mm_size_line((26.458_333, 13.229_166)), "26.46 × 13.23 mm");
        assert_eq!(mm_size_line((100.5, 1.0)), "100.5 × 1 mm");
    }

    use std::sync::{Arc, Mutex};

    use crate::preview_fonts::FontSources;

    /// Fonts with the included block read from the page's own folder; `read` records every file.
    fn included_fonts(read: &Arc<Mutex<Vec<String>>>) -> PreviewFonts {
        let read = Arc::clone(read);
        PreviewFonts::new(FontSources {
            curvyo_dir: None,
            included: Box::new(move |file| {
                read.lock().unwrap().push(file.to_string());
                let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
                std::fs::read(dir.join("../frontend/public/fonts").join(file)).ok()
            }),
            system: false,
        })
    }

    fn text_svg(family: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><text x="10" y="40" style="font-family:{family};font-size:30px;fill:#000">HH</text></svg>"#
        )
    }

    /// The dark pixels of `picture` inside and outside the box `x0..x1` x `y0..y1` (pixels).
    fn dark_in_and_out(
        picture: &WorkingImage,
        (x0, y0, x1, y1): (u32, u32, u32, u32),
    ) -> (usize, usize) {
        let (mut inside, mut outside) = (0, 0);
        for y in 0..picture.height {
            for x in 0..picture.width {
                if pixel(picture, x, y)[0] < 128 {
                    if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                        inside += 1;
                    } else {
                        outside += 1;
                    }
                }
            }
        }
        (inside, outside)
    }

    #[test]
    fn a_text_in_an_included_face_is_drawn_where_import_svg_puts_its_letters() {
        let read = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = included_fonts(&read);
        let svg = text_svg("'Source Sans 3'");
        let t = svg_thumbnail(svg.as_bytes(), 256, &mut fonts)
            .unwrap()
            .unwrap();
        assert_eq!((t.picture.width, t.picture.height), (256, 128));
        // 2.56 px/mm: the capitals stand on the baseline at y 40 mm (102 px) and are some 19.8
        // mm (0.66 em) tall; they start right of x 10 mm (26 px) and end before x 60 mm.
        let (inside, outside) = dark_in_and_out(&t.picture, (25, 50, 154, 104));
        assert!(inside > 1500, "the two H are drawn: {inside} dark pixels");
        assert_eq!(outside, 0, "nothing outside the letters' box");
        assert_eq!(
            *read.lock().unwrap(),
            ["index.json", "source-sans-3/SourceSans3-Regular.otf"],
            "the listing and the one face the text needs"
        );
        // The listing and the face are read once per dialog, not per file.
        let again = svg_thumbnail(svg.as_bytes(), 256, &mut fonts)
            .unwrap()
            .unwrap();
        assert_eq!(again.picture, t.picture);
        assert_eq!(read.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_missing_face_is_drawn_in_the_substitute_and_no_font_leaves_the_text_out() {
        let read = Arc::new(Mutex::new(Vec::new()));
        let substitute = svg_thumbnail(
            text_svg("'No Such Font'").as_bytes(),
            256,
            &mut included_fonts(&read),
        )
        .unwrap()
        .unwrap();
        let sans = svg_thumbnail(
            text_svg("'Source Sans 3'").as_bytes(),
            256,
            &mut included_fonts(&read),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            substitute.picture, sans.picture,
            "the included sans Regular"
        );
        let none = svg_thumbnail(
            text_svg("'No Such Font'").as_bytes(),
            256,
            &mut PreviewFonts::none(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(dark_in_and_out(&none.picture, (0, 0, 0, 0)), (0, 0));
    }

    #[test]
    fn a_text_of_a_million_letters_is_refused_by_the_budget_before_it_is_shaped() {
        let letters = "H".repeat(curvyo_svg_core::limits::MAX_TEXT_CHARS);
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50"><text x="10" y="40" style="font-family:'Source Sans 3';font-size:30px">{letters}</text></svg>"#
        );
        let read = Arc::new(Mutex::new(Vec::new()));
        let t = svg_thumbnail(svg.as_bytes(), 256, &mut included_fonts(&read))
            .unwrap()
            .unwrap();
        assert_eq!(dark_in_and_out(&t.picture, (0, 0, 0, 0)), (0, 0), "skipped");
    }

    #[test]
    fn a_file_loads_at_most_sixteen_faces_and_draws_the_rest_in_the_substitute() {
        let styles = [
            ("Source Sans 3", "font-weight:400"),
            ("Source Sans 3", "font-weight:200"),
            ("Source Sans 3", "font-weight:200;font-style:italic"),
            ("Source Sans 3", "font-weight:300"),
            ("Source Sans 3", "font-weight:300;font-style:italic"),
            ("Source Sans 3", "font-weight:700"),
            ("Source Sans 3", "font-weight:700;font-style:italic"),
            ("Source Sans 3", "font-style:italic"),
            ("Source Serif 4", "font-weight:400"),
            ("Source Serif 4", "font-style:italic"),
            ("Source Serif 4", "font-weight:700"),
            ("Source Serif 4", "font-weight:700;font-style:italic"),
            ("Great Vibes", "font-weight:400"),
            ("EMS Allure", "font-weight:400"),
            ("EMS Readability", "font-weight:400"),
            ("Hershey Sans 1-stroke", "font-weight:400"),
            ("Hershey Script 1-stroke", "font-weight:400"),
        ];
        let mut texts = String::new();
        for (row, (family, style)) in styles.iter().enumerate() {
            write!(
                texts,
                r#"<text x="5" y="{}" style="font-family:'{family}';{style};font-size:4px;fill:#000;stroke:#000;stroke-width:0.3">Hi</text>"#,
                5 + row * 5
            )
            .unwrap();
        }
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">{texts}</svg>"#
        );
        let read = Arc::new(Mutex::new(Vec::new()));
        let t = svg_thumbnail(svg.as_bytes(), 256, &mut included_fonts(&read))
            .unwrap()
            .unwrap();
        // 17 faces asked for: the first 16 are read, the last is drawn in the substitute (the
        // first face, Source Sans 3 Regular).
        let read = read.lock().unwrap();
        assert_eq!(read.len() - 1, crate::preview_fonts::MAX_FACES_PER_FILE);
        assert!(!read.iter().any(|file| file.contains("HersheyScript")));
        // Every text is drawn.
        for row in 0..styles.len() {
            let y = u32::try_from((5 + row * 5) * 256 / 100).unwrap();
            let (inside, _) = dark_in_and_out(&t.picture, (12, y - 8, 40, y + 1));
            assert!(inside > 0, "text {row} is drawn");
        }
    }
}

//! Test support: generate through the extension, a scanline filler for the styled outlines (the
//! non-zero rule, cubics flattened at 0.01 module), the independent decoder and the SVG goldens.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::format_push_string
)]

pub mod glyphs;

use curvyo_codes_core::CODES;
use curvyo_extension_core::{
    ClosedPath, Edit, Extension, GenerateRequest, Generated, Param, ParamValue, Seed, StoredParams,
};

/// The schema of the QR kind.
pub fn schema() -> Vec<Param> {
    let manifest = CODES.manifest();
    manifest.generators().next().expect("one kind").schema()
}

/// The schema of the kind `kind` (`qr`, `ean-13`, ...).
pub fn schema_of(kind: &str) -> Vec<Param> {
    let manifest = CODES.manifest();
    manifest
        .generators()
        .find(|k| k.id == kind)
        .expect("a kind of the codes")
        .schema()
}

/// The stored parameters of a new object of `kind` with `overrides` applied.
pub fn params_of(kind: &str, overrides: &[(&str, ParamValue)]) -> StoredParams {
    let mut stored = Param::defaults(&schema_of(kind));
    for (key, value) in overrides {
        stored.insert((*key).to_owned(), value.to_stored());
    }
    stored
}

/// Generates `kind` from `params` after `edit`, with the glyph table when given.
pub fn generate_kind(
    kind: &str,
    params: &StoredParams,
    edit: Option<Edit<'_>>,
    glyphs: Option<&curvyo_extension_core::GlyphTable>,
) -> Generated {
    CODES
        .generate(&GenerateRequest {
            kind,
            params,
            seed: Seed(0),
            edit,
            glyphs,
        })
        .expect("a kind of the codes")
}

/// The stored parameters of a new object with `overrides` applied.
pub fn params(overrides: &[(&str, ParamValue)]) -> StoredParams {
    let mut stored = Param::defaults(&schema());
    for (key, value) in overrides {
        stored.insert((*key).to_owned(), value.to_stored());
    }
    stored
}

/// Generates the QR kind from `params` after `edit`.
pub fn generate_from(params: &StoredParams, edit: Option<Edit<'_>>) -> Generated {
    CODES
        .generate(&GenerateRequest {
            kind: "qr",
            params,
            seed: Seed(0),
            edit,
            glyphs: None,
        })
        .expect("the qr kind")
}

/// Generates from the defaults with `overrides`.
pub fn generate(overrides: &[(&str, ParamValue)]) -> Generated {
    generate_from(&params(overrides), None)
}

pub fn text(value: &str) -> ParamValue {
    ParamValue::Text(value.to_owned())
}

pub fn choice(value: &str) -> ParamValue {
    ParamValue::Choice(value.to_owned())
}

/// The outlines of the one part.
pub fn paths(generated: &Generated) -> &[ClosedPath] {
    &generated.parts[0].paths
}

/// The outlines that reach a machine (every part but a `job: false` one), in part order: the ink the
/// decoder test rasterises on the Background's ground.
pub fn ink_paths(generated: &Generated) -> Vec<ClosedPath> {
    generated
        .parts
        .iter()
        .filter(|part| part.paint.is_none_or(|paint| paint.job))
        .flat_map(|part| part.paths.iter().cloned())
        .collect()
}

/// A subpath flattened to a polygon, the cubics cut into `steps` pieces.
fn flatten(path: &ClosedPath, steps: usize) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    let mut from = path.start;
    for cubic in &path.segments {
        let straight = cubic.c1 == from && cubic.c2 == cubic.to;
        let count = if straight { 1 } else { steps };
        for step in 1..=count {
            #[allow(clippy::cast_precision_loss)]
            let t = step as f64 / count as f64;
            let u = 1.0 - t;
            let at = |a: f64, b: f64, c: f64, d: f64| {
                u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
            };
            points.push((
                at(from.x, cubic.c1.x, cubic.c2.x, cubic.to.x),
                at(from.y, cubic.c1.y, cubic.c2.y, cubic.to.y),
            ));
        }
        from = cubic.to;
    }
    points
}

/// The area the outlines cover under the non-zero rule, in mm^2, by a scanline of `rows` rows.
pub fn covered_area(paths: &[ClosedPath], module: f64, rows: usize) -> f64 {
    let polygons: Vec<Vec<(f64, f64)>> = paths.iter().map(|p| flatten(p, 16)).collect();
    let (mut low, mut high) = (f64::MAX, f64::MIN);
    for polygon in &polygons {
        for point in polygon {
            low = low.min(point.1);
            high = high.max(point.1);
        }
    }
    let _ = module;
    let mut total = 0.0;
    #[allow(clippy::cast_precision_loss)]
    let height = (high - low) / rows as f64;
    for row in 0..rows {
        #[allow(clippy::cast_precision_loss)]
        let y = low + (row as f64 + 0.5) * height;
        total += span_length(&polygons, y) * height;
    }
    total
}

/// The length of the scanline at `y` that the polygons cover under the non-zero rule.
fn span_length(polygons: &[Vec<(f64, f64)>], y: f64) -> f64 {
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for polygon in polygons {
        for pair in polygon.windows(2) {
            let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
            if (y0 <= y) != (y1 <= y) {
                let x = x0 + (y - y0) / (y1 - y0) * (x1 - x0);
                crossings.push((x, if y1 > y0 { 1 } else { -1 }));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut winding, mut length) = (0, 0.0);
    for pair in crossings.windows(2) {
        winding += pair[0].1;
        if winding != 0 {
            length += pair[1].0 - pair[0].0;
        }
    }
    length
}

/// Renders the outlines as a greyscale bitmap, `per_module` pixels per module with `border`
/// modules around the symbol of `width` modules. Ink is black on white, or white on black when
/// `light_ink` (an inverted code engraved on a dark material).
pub fn raster(
    paths: &[ClosedPath],
    module: f64,
    width: usize,
    border: usize,
    per_module: usize,
    light_ink: bool,
) -> (usize, usize, Vec<u8>) {
    let size = (width + 2 * border) * per_module;
    let scale = per_module as f64 / module;
    let shift = (width as f64 / 2.0 + border as f64) * per_module as f64;
    raster_window(paths, scale, (shift, shift), (size, size), light_ink)
}

/// Renders the outlines into a `size` window: a point `(x, y)` in mm lands on the pixel
/// `(x * scale + shift.0, y * scale + shift.1)`.
pub fn raster_window(
    paths: &[ClosedPath],
    scale: f64,
    shift: (f64, f64),
    size: (usize, usize),
    light_ink: bool,
) -> (usize, usize, Vec<u8>) {
    let (width, height) = size;
    let polygons: Vec<Vec<(f64, f64)>> = paths
        .iter()
        .map(|path| {
            flatten(path, 12)
                .into_iter()
                .map(|(x, y)| (x * scale + shift.0, y * scale + shift.1))
                .collect()
        })
        .collect();
    let (ink, ground) = if light_ink {
        (255u8, 0u8)
    } else {
        (0u8, 255u8)
    };
    let mut pixels = vec![ground; width * height];
    for row in 0..height {
        let y = row as f64 + 0.5;
        let mut crossings: Vec<(f64, i32)> = Vec::new();
        for polygon in &polygons {
            for pair in polygon.windows(2) {
                let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
                if (y0 <= y) != (y1 <= y) {
                    crossings.push((
                        x0 + (y - y0) / (y1 - y0) * (x1 - x0),
                        if y1 > y0 { 1 } else { -1 },
                    ));
                }
            }
        }
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0;
        for pair in crossings.windows(2) {
            winding += pair[0].1;
            if winding != 0 {
                let (from, to) = (
                    (pair[0].0 - 0.5).ceil().max(0.0) as usize,
                    ((pair[1].0 - 0.5).floor() + 1.0).clamp(0.0, width as f64) as usize,
                );
                for x in from..to.min(width) {
                    pixels[row * width + x] = ink;
                }
            }
        }
    }
    (width, height, pixels)
}

/// What the independent decoder (`rqrr`) reads in the bitmap, `None` when it finds no code.
pub fn decode(size: (usize, usize, Vec<u8>)) -> Option<String> {
    let (w, h, pixels) = size;
    let mut image = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| pixels[y * w + x]);
    let grids = image.detect_grids();
    let grid = grids.first()?;
    grid.decode().ok().map(|(_, text)| text)
}

/// The SVG of the outlines: one `<path>` with `fill-rule="nonzero"` and three decimals (ADR 0091
/// §11), the view box the outlines' bounds.
pub fn svg(paths: &[ClosedPath]) -> String {
    let mut d = String::new();
    let (mut low, mut high) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    let mut note = |x: f64, y: f64| {
        low = (low.0.min(x), low.1.min(y));
        high = (high.0.max(x), high.1.max(y));
    };
    let n = |v: f64| format!("{:.3}", if v.abs() < 5e-4 { 0.0 } else { v });
    for path in paths {
        d.push_str(&format!("M{} {}", n(path.start.x), n(path.start.y)));
        note(path.start.x, path.start.y);
        let mut from = path.start;
        for c in &path.segments {
            if c.c1 == from && c.c2 == c.to {
                d.push_str(&format!("L{} {}", n(c.to.x), n(c.to.y)));
            } else {
                d.push_str(&format!(
                    "C{} {} {} {} {} {}",
                    n(c.c1.x),
                    n(c.c1.y),
                    n(c.c2.x),
                    n(c.c2.y),
                    n(c.to.x),
                    n(c.to.y)
                ));
            }
            note(c.to.x, c.to.y);
            from = c.to;
        }
        d.push('Z');
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\" width=\"{}mm\" height=\"{}mm\">\n<path fill-rule=\"nonzero\" d=\"{d}\"/>\n</svg>\n",
        n(low.0),
        n(low.1),
        n(high.0 - low.0),
        n(high.1 - low.1),
        n(high.0 - low.0),
        n(high.1 - low.1),
    )
}

/// The SVG of a painted generation: one `<path>` per part in draw order, each with its `fill` when
/// the part has a paint of its own (ADR 0091 P8); the view box is the union of the parts.
pub fn svg_parts(generated: &Generated) -> String {
    let all: Vec<ClosedPath> = generated
        .parts
        .iter()
        .flat_map(|part| part.paths.iter().cloned())
        .collect();
    let frame = svg(&all);
    let (head, _) = frame.split_once("<path").unwrap();
    let mut out = head.to_owned();
    for part in &generated.parts {
        let one = svg(&part.paths);
        let body = one.split_once("<path fill-rule=\"nonzero\"").unwrap().1;
        let fill = part.paint.map_or(String::new(), |paint| {
            format!(
                " fill=\"#{:02x}{:02x}{:02x}\"",
                paint.fill.r, paint.fill.g, paint.fill.b
            )
        });
        let body = body.split_once("</svg>").unwrap().0;
        out.push_str(&format!(
            "<path data-part=\"{}\"{fill} fill-rule=\"nonzero\"{body}",
            part.key
        ));
    }
    out.push_str("</svg>\n");
    out
}

/// The numbers in `text`, in order.
fn numbers(text: &str) -> Vec<f64> {
    let mut found = Vec::new();
    let mut current = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || c == '.' || (c == '-' && current.is_empty()) {
            current.push(c);
        } else {
            if let Ok(v) = current.parse() {
                found.push(v);
            }
            current.clear();
        }
    }
    found
}

/// Compares `paths` with the fixture `name`, number by number within 0.001 mm; with
/// `UPDATE_GOLDEN=1` the fixture is written instead.
pub fn assert_golden(name: &str, paths: &[ClosedPath]) {
    assert_golden_text(name, &svg(paths));
}

/// [`assert_golden`] for the SVG of every part with its fill ([`svg_parts`]).
pub fn assert_golden_parts(name: &str, generated: &Generated) {
    assert_golden_text(name, &svg_parts(generated));
}

fn assert_golden_text(name: &str, now: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let now = now.to_owned();
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &now).unwrap();
        return;
    }
    let stored =
        std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture {name} is missing"));
    let skeleton = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphabetic() || *c == '<')
            .collect::<String>()
    };
    assert_eq!(
        skeleton(&stored),
        skeleton(&now),
        "{name}: the path commands differ"
    );
    let (a, b) = (numbers(&stored), numbers(&now));
    assert_eq!(a.len(), b.len(), "{name}: number of coordinates");
    for (at, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!(
            (x - y).abs() <= 0.001 + 1e-9,
            "{name}: number {at}: {x} against {y}"
        );
    }
}

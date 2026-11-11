//! Reads the paths of an Inkscape "Trace Bitmap" baseline SVG for the quality harness: the root's
//! `width` and `viewBox`, nested `<g transform>` and `<path d>`. Anything else in the file (other
//! shapes, styles, `<defs>`) is ignored; the harness only compares outlines. Tests only.

#![allow(dead_code)]

use kurbo::{Affine, BezPath};

/// The page width in millimetres and the file's paths in millimetres of the page.
pub struct Baseline {
    pub page_width: f64,
    pub paths: Vec<BezPath>,
}

/// Millimetres per CSS pixel.
const MM_PER_PX: f64 = 25.4 / 96.0;

/// The paths of `svg`, or `None` when it is no SVG with a root `width`.
pub fn read_baseline(svg: &[u8]) -> Option<Baseline> {
    let text = std::str::from_utf8(svg).ok()?;
    let mut paths = Vec::new();
    let mut page_width = None;
    let mut stack: Vec<Affine> = Vec::new();
    let mut defs = 0_usize;
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let after = &rest[open + 1..];
        let close = after.find('>')?;
        let tag = &after[..close];
        rest = &after[close + 1..];
        if tag.starts_with(['?', '!']) {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            match name.trim() {
                "g" => {
                    stack.pop();
                }
                "defs" => defs = defs.saturating_sub(1),
                _ => {}
            }
            continue;
        }
        let tag = tag.trim_end_matches('/');
        let (name, attributes) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let self_closing = after[..close].ends_with('/');
        let own = attribute(attributes, "transform").map_or(Affine::IDENTITY, transform);
        let parent = stack.last().copied().unwrap_or(Affine::IDENTITY);
        match name {
            "svg" if page_width.is_none() => {
                let width = length_mm(&attribute(attributes, "width")?)?;
                page_width = Some(width);
                let map = match attribute(attributes, "viewBox").and_then(|v| view_box(&v)) {
                    Some([x, y, w, _]) if w > 0.0 => {
                        let k = width / w;
                        Affine::new([k, 0.0, 0.0, k, -x * k, -y * k])
                    }
                    _ => Affine::scale(MM_PER_PX),
                };
                stack.push(map);
            }
            "defs" if !self_closing => defs += 1,
            "g" => stack.push(parent * own),
            "path" if defs == 0 => {
                if let Some(path) =
                    attribute(attributes, "d").and_then(|d| BezPath::from_svg(&d).ok())
                {
                    paths.push(parent * own * path);
                }
            }
            _ => {}
        }
        if self_closing && name == "g" {
            stack.pop();
        }
    }
    Some(Baseline {
        page_width: page_width?,
        paths,
    })
}

/// The value of attribute `name` in `attributes`, in single or double quotes.
fn attribute(attributes: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(found) = attributes[from..].find(name) {
        let start = from + found;
        from = start + name.len();
        let before_ok = attributes[..start]
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
        let tail = attributes[from..].trim_start();
        if before_ok && let Some(value) = tail.strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let inner = &value[1..];
            return Some(inner[..inner.find(quote)?].to_string());
        }
    }
    None
}

/// A length with an optional unit in millimetres (a bare number is a CSS pixel).
fn length_mm(text: &str) -> Option<f64> {
    let text = text.trim();
    let split = text
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')))
        .unwrap_or(text.len());
    let number: f64 = text[..split].parse().ok()?;
    let per_unit = match text[split..].trim() {
        "mm" => 1.0,
        "cm" => 10.0,
        "in" => 25.4,
        "pt" => 25.4 / 72.0,
        "pc" => 25.4 / 6.0,
        "" | "px" => MM_PER_PX,
        _ => return None,
    };
    Some(number * per_unit)
}

fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn view_box(text: &str) -> Option<[f64; 4]> {
    <[f64; 4]>::try_from(numbers(text)).ok()
}

/// A `transform` list of `translate`, `scale`, `matrix` and `rotate` (degrees).
fn transform(text: String) -> Affine {
    let mut total = Affine::IDENTITY;
    let mut rest = text.as_str();
    while let Some(open) = rest.find('(') {
        let Some(close) = rest[open..].find(')') else {
            break;
        };
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let n = numbers(&rest[open + 1..open + close]);
        let step = match (name, n.as_slice()) {
            ("translate", [x]) => Affine::translate((*x, 0.0)),
            ("translate", [x, y]) => Affine::translate((*x, *y)),
            ("scale", [s]) => Affine::scale(*s),
            ("scale", [x, y]) => Affine::scale_non_uniform(*x, *y),
            ("matrix", &[a0, a1, a2, a3, a4, a5]) => Affine::new([a0, a1, a2, a3, a4, a5]),
            ("rotate", [a]) => Affine::rotate(a.to_radians()),
            _ => Affine::IDENTITY,
        };
        total *= step;
        rest = &rest[open + close + 1..];
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_inkscape_form() {
        let svg = br#"<?xml version="1.0"?>
<svg width="20mm" height="10mm" viewBox="0 0 40 20" xmlns="http://www.w3.org/2000/svg">
  <defs><path d="M0 0 L9 9"/></defs>
  <g transform="translate(2,4)"><path d="M0 0 L10 0 L10 10 Z" style="fill:#000"/></g>
</svg>"#;
        let b = read_baseline(svg).unwrap();
        assert!((b.page_width - 20.0).abs() < 1e-12);
        assert_eq!(b.paths.len(), 1);
        let bounds = kurbo::Shape::bounding_box(&b.paths[0]);
        assert!((bounds.x0 - 1.0).abs() < 1e-9 && (bounds.x1 - 6.0).abs() < 1e-9);
        assert!((bounds.y0 - 2.0).abs() < 1e-9 && (bounds.y1 - 7.0).abs() < 1e-9);
    }
}

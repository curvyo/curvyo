//! The Design Space SVG writer (`specs/0159-cutting-plotter` criteria 11, 16, `adrs.md` PL6): one
//! group of stroked paths per operation at mat scale, mm from the mat origin, no date and no
//! random id.

use std::fmt::Write as _;

use curvyo_document_core::DocumentSize;
use curvyo_geometry_core::{PointUm, mm_of_um};

use crate::job::CutterJob;
use crate::mat::MatSize;
use crate::operation::OpKind;

/// The colours of the Cut groups in operation order; Draw is black.
pub const CUT_COLOURS: [&str; 6] = [
    "#E5352B", "#1F6FB4", "#2CA02C", "#9467BD", "#FF7F0E", "#8C564B",
];

/// The stroke width of every path, in mm.
const STROKE_MM: &str = "0.250";

/// The SVG of `job` on a page of `page` size; `mat` writes the width and height in inches.
#[must_use]
pub fn write_svg(job: &CutterJob, page: DocumentSize, mat: Option<MatSize>) -> String {
    let (w, h) = (page.width.as_mm(), page.height.as_mm());
    let (width, height) = match mat {
        Some(mat) => {
            let (wi, hi) = mat.inches();
            (format!("{wi}in"), format!("{hi}in"))
        }
        None => (format!("{w:.3}mm"), format!("{h:.3}mm")),
    };
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:inkscape=\"http://www.inkscape.org/namespaces/inkscape\" \
         width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {w:.3} {h:.3}\">"
    );
    let mut ids: Vec<String> = Vec::new();
    let mut cut_index = 0;
    for run in &job.operations {
        if run.paths.is_empty() {
            continue;
        }
        let colour = match run.kind {
            OpKind::Draw => "#000000",
            OpKind::Cut => {
                let c = CUT_COLOURS[cut_index % CUT_COLOURS.len()];
                cut_index += 1;
                c
            }
        };
        let id = unique_id(&run.name, &mut ids);
        let label = escape(&run.name);
        let _ = writeln!(
            out,
            "  <g id=\"{id}\" inkscape:label=\"{label}\" inkscape:groupmode=\"layer\" \
             fill=\"none\" stroke=\"{colour}\" stroke-width=\"{STROKE_MM}\">"
        );
        for path in &run.paths {
            let _ = writeln!(out, "    <path d=\"{}\"/>", path_data(path));
        }
        out.push_str("  </g>\n");
    }
    out.push_str("</svg>\n");
    out
}

/// The `d` of one path: a closed path (ending on its start) ends with `Z`.
fn path_data(path: &[PointUm]) -> String {
    let closed = path.len() > 2 && path.first() == path.last();
    let points = if closed {
        &path[..path.len() - 1]
    } else {
        path
    };
    let mut d = String::new();
    for (index, p) in points.iter().enumerate() {
        let command = if index == 0 { "M" } else { " L" };
        let _ = write!(d, "{command} {:.3} {:.3}", mm_of_um(p.x), mm_of_um(p.y));
    }
    if closed {
        d.push_str(" Z");
    }
    d
}

/// An XML id from `name` (letters, digits, `-`, `_`; others become `_`), made unique in `taken`
/// by a counter.
fn unique_id(name: &str, taken: &mut Vec<String>) -> String {
    let mut base: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !base.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        base.insert(0, '_');
    }
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    taken.push(id.clone());
    id
}

/// `text` escaped for an XML attribute.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_unique_xml_ids() {
        let mut taken = Vec::new();
        assert_eq!(unique_id("Cut", &mut taken), "Cut");
        assert_eq!(unique_id("Cut", &mut taken), "Cut-2");
        assert_eq!(unique_id("1 große Form", &mut taken), "_1_gro_e_Form");
        assert_eq!(escape("A & \"B\""), "A &amp; &quot;B&quot;");
    }
}

//! Names the faces of an SVG font file for the Curvyo folder listing and Add font
//! (`specs/0076-single-line-fonts` criteria 1, 3 and 23, `adrs.md` decision 5): `fontdb` reads no
//! SVG fonts, so the text engine's own reader (`curvyo-text-core`) decides, and a face is listed
//! exactly when the engine can draw it.

use curvyo_text_core::{ListableFace, svg_listable_faces};

pub use curvyo_text_core::limits::MAX_SVG_FONT_BYTES;

/// The faces of an SVG font file: every `<font>` with a drawn glyph that the text engine reads.
/// Empty for a file that is not one, is not well-formed, or breaks a limit; nothing is fetched or
/// resolved.
#[must_use]
pub fn svg_font_faces(bytes: &[u8]) -> Vec<ListableFace> {
    svg_listable_faces(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_bytes_list_nothing() {
        let entity = br#"<!DOCTYPE svg [<!ENTITY a "x">]><svg><font><font-face font-family="&a;"/><glyph d="M0 0"/></font></svg>"#;
        assert_eq!(svg_font_faces(entity), []);
        let deep = format!("<svg>{}</svg>", "<g>".repeat(300));
        assert_eq!(svg_font_faces(deep.as_bytes()), []);
        let zero =
            br#"<svg><font id="F"><font-face units-per-em="0"/><glyph d="M0 0"/></font></svg>"#;
        assert_eq!(svg_font_faces(zero), []);
        let ok = br#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "x.dtd"><svg><font id="F"><glyph d="M0 0"/></font></svg>"#;
        let faces = svg_font_faces(ok);
        assert_eq!(faces.len(), 1);
        assert_eq!((faces[0].index, faces[0].family.as_str()), (0, "F"));
        assert_eq!(faces[0].style, "Regular");
    }
}

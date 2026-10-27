//! Phase one of an SVG import (`specs/0024-svg-import-export` criterion 106, `adrs.md` note
//! 2026-10-27 point 3): the listed faces the texts of a drawing ask for that are not loaded yet.

use curvyo_svg_core::{SvgDrawing, SvgNode, SvgText};

use crate::font_match::{FontBlock, ListedFace};
use crate::svg_font_match::{FaceMemo, GENERIC_SANS, SvgFace};

/// What the faces of a drawing's texts need before `svg_import` (phase one).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SvgFaceNeeds {
    /// The listed faces (indices into the listing) the texts ask for that are not loaded.
    pub load: Vec<usize>,
    /// Whether a text is drawn in the substitute face (a missing face).
    pub substitute: bool,
}

impl SvgFaceNeeds {
    /// Whether nothing has to be loaded besides the substitute, when that is loaded.
    #[must_use]
    pub fn ready(&self, substitute_loaded: bool) -> bool {
        self.load.is_empty() && (!self.substitute || substitute_loaded)
    }
}

/// Phase one: the faces the texts of `drawing` ask for among `faces` that are not loaded yet.
#[must_use]
pub fn svg_faces_needed(drawing: &SvgDrawing, faces: &[ListedFace]) -> SvgFaceNeeds {
    let mut needs = SvgFaceNeeds::default();
    let memo = FaceMemo::new(faces);
    visit_texts(&drawing.nodes, &mut |text| {
        let used: std::collections::BTreeSet<usize> = text.chars.iter().map(|c| c.style).collect();
        for style in used {
            match memo.face(&text.styles[style]) {
                SvgFace::Listed(at) if faces[at].key.is_none() => {
                    if !needs.load.contains(&at) {
                        needs.load.push(at);
                    }
                }
                SvgFace::Listed(_) => {}
                SvgFace::Missing { .. } => needs.substitute = true,
            }
        }
    });
    needs
}

/// The listed face a missing face is drawn in: the included sans, Regular.
#[must_use]
pub fn substitute_face(faces: &[ListedFace]) -> Option<usize> {
    faces.iter().position(|f| {
        f.block == FontBlock::Included && f.family == GENERIC_SANS && f.style == "Regular"
    })
}

fn visit_texts(nodes: &[SvgNode], f: &mut impl FnMut(&SvgText)) {
    for node in nodes {
        match node {
            SvgNode::Group { children, .. } => visit_texts(children, f),
            SvgNode::Text { text, .. } => f(text),
            SvgNode::Leaf { .. } | SvgNode::Image { .. } => {}
        }
    }
}

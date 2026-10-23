//! The faces of a font file Add font accepts in the browser, with their names and class bytes
//! (`specs/0075-font-library` criteria 28 and 32, `adrs.md` decisions 3 and 7).

use curvyo_text_core::{FaceStore, listable_faces, os2_class_bytes, preview_path};
use serde_json::{Value, json};

/// The faces Add font accepts in the browser (criteria 28 and 32), as JSON
/// `[{"index", "family", "style", "os2": hex|null, "kind"?}]` (`kind` is `"single_line"` for an
/// SVG font, `specs/0076-single-line-fonts` criterion 3); empty for a file that is not a font.
#[must_use]
pub fn font_file_faces(bytes: &[u8]) -> String {
    let faces: Vec<Value> = listable_faces(bytes)
        .into_iter()
        .map(|face| {
            let os2 = os2_class_bytes(bytes, face.index).map(|os2| {
                os2.iter().fold(String::new(), |mut hex, b| {
                    use std::fmt::Write as _;
                    let _ = write!(hex, "{b:02x}");
                    hex
                })
            });
            let mut entry =
                json!({ "index": face.index, "family": face.family, "style": face.style, "os2": os2 });
            if face.kind == curvyo_text_core::FaceKind::SingleLine {
                entry["kind"] = json!("single_line");
            }
            entry
        })
        .collect();
    Value::Array(faces).to_string()
}

/// Face `index` of `bytes` drawing `text` for its Fonts-tab row (`specs/0076-single-line-fonts`
/// criterion 6, `adrs.md` decision 6): JSON `{"d", "box": [x, y, width, height]}` in em units,
/// y down, origin on the baseline; empty when the bytes hold no such face.
#[must_use]
pub fn face_preview_path(bytes: &[u8], index: u32, text: &str) -> String {
    let mut store = FaceStore::new();
    let Some(face) = store
        .add(bytes.to_vec(), index)
        .ok()
        .and_then(|key| store.get(&key).cloned())
    else {
        return String::new();
    };
    preview_path(&face, text).map_or_else(String::new, |(d, area)| {
        json!({ "d": d, "box": [area.x0, area.y0, area.width(), area.height()] }).to_string()
    })
}

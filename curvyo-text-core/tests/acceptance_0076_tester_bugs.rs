//! Tester case for `specs/0076-single-line-fonts` criterion 23 that fails on the branch: an
//! internal DTD subset after a decoy `<!DOCTYPE` in a comment or a processing instruction
//! reaches the XML parser and its entity is resolved.
#![allow(clippy::unwrap_used, missing_docs)]
#![cfg(not(target_arch = "wasm32"))]

use curvyo_text_core::{FaceStore, listable_faces};

fn font_with_prolog(prolog: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0"?>{prolog}<svg xmlns="http://www.w3.org/2000/svg"><defs><font><font-face font-family="&a;" units-per-em="1000"/><glyph unicode="I" d="M0 0V700"/></font></defs></svg>"#
    )
    .into_bytes()
}

/// Criterion 23: a DOCTYPE with an internal subset is refused wherever the first `<!DOCTYPE`
/// text appears: also when a comment or a processing instruction before it holds a decoy
/// `<!DOCTYPE ...>` without one. Nothing is listed and the store refuses the bytes.
#[test]
fn ac23_an_internal_subset_behind_a_decoy_is_refused() {
    let subset = r#"<!DOCTYPE svg [<!ENTITY a "Evil">]>"#;
    for decoy in ["", "<!-- <!DOCTYPE svg> -->", "<?note <!DOCTYPE svg> ?>"] {
        let bytes = font_with_prolog(&format!("{decoy}{subset}"));
        let listed = listable_faces(&bytes);
        assert!(
            listed.is_empty(),
            "decoy {decoy:?}: the entity was resolved, listed as {:?}",
            listed.iter().map(|f| &f.family).collect::<Vec<_>>()
        );
        assert!(FaceStore::new().add(bytes, 0).is_err(), "decoy {decoy:?}");
    }
}

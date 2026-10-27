//! Independent tester test for `specs/0024-svg-import-export` criterion 103 at the import
//! conversion: images are taken in document order until the project's asset room (512 MiB in the
//! session) is used up; the rest is ignored silently. Observed through the public
//! `svg_import` with a small room, the only seam that sets it. Written from the specification.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::DrawingNode;
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{ImportFaces, SvgPlacement, read_svg_drawing, svg_import};

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255]))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            s.push(if i <= c.len() {
                char::from(T[((n >> (18 - 6 * i)) & 63) as usize])
            } else {
                '='
            });
        }
    }
    s
}

fn image_count(nodes: &[DrawingNode]) -> usize {
    nodes
        .iter()
        .map(|n| match n {
            DrawingNode::Group { children, .. } => image_count(children),
            DrawingNode::Image { .. } => 1,
            _ => 0,
        })
        .sum()
}

/// Criterion 103: a picture that would take the project over its asset room is ignored; images
/// before it (document order) come in, and the same bytes again cost no room.
#[test]
fn c103_images_until_the_asset_room_is_used_up() {
    let (a, b) = (png(30, 30), png(31, 31));
    let body = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 100 100">
        <image width="10" height="10" xlink:href="data:image/png;base64,{a64}"/>
        <image width="10" height="10" xlink:href="data:image/png;base64,{b64}"/>
        <image width="10" height="10" xlink:href="data:image/png;base64,{a64}"/>
        <rect width="5" height="5"/></svg>"#,
        a64 = b64(&a),
        b64 = b64(&b)
    );
    let drawing = read_svg_drawing(body.as_bytes()).unwrap();
    let store = FaceStore::new();
    let fonts = ImportFaces {
        faces: &[],
        store: &store,
        substitute: None,
    };
    let room = u64::try_from(a.len() + b.len() / 2).unwrap();
    let out = svg_import(&drawing, SvgPlacement::File, &fonts, room);
    assert_eq!(out.assets.len(), 1, "only the first picture fits");
    assert_eq!(image_count(&out.nodes), 2, "the first and its repeat");
    assert_eq!(out.leaves, 3, "two images and the rectangle");
    let all = svg_import(&drawing, SvgPlacement::File, &fonts, u64::MAX);
    assert_eq!((all.assets.len(), image_count(&all.nodes)), (2, 3));
}

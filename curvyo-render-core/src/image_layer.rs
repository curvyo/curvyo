//! Images in the draw list (`specs/0058-bitmap-import` criteria 27 and 30, ADR 0017 §5): each
//! image is one artwork layer of its own with no triangles, and an [`ImageQuad`] that names the
//! layer. The host draws the quad with its textured pipeline at that layer's depth, in list order,
//! so images and shapes keep their stacking. This crate stays pixel-free: the quad names the asset
//! and its corners, the host owns the texture.

use curvyo_document_core::{
    AnchorSnapshot, AssetHash, ImageSnapshot, NewAnchor, Opacity, Point, Vec2,
};

use crate::glyphs::DrawList;

/// One image to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageQuad {
    /// The picture.
    pub asset: AssetHash,
    /// The frame in document millimetres: the picture's top-left, top-right, bottom-right and
    /// bottom-left corners.
    pub corners: [Point; 4],
    /// The opacity it is drawn with (its own times its ancestors').
    pub opacity: Opacity,
    /// The index of its artwork layer in [`DrawList::layers`].
    pub layer: usize,
}

impl DrawList {
    /// Appends `image` as an artwork layer of its own.
    pub(crate) fn push_image(&mut self, image: &ImageSnapshot) {
        let layer = self.push_empty_layer();
        self.images.push(ImageQuad {
            asset: image.asset.clone(),
            corners: image.corners(),
            opacity: image.shown_opacity,
            layer,
        });
    }

    /// The vertex index at which artwork layer `layer` starts.
    #[must_use]
    pub fn layer_start(&self, layer: usize) -> usize {
        layer
            .checked_sub(1)
            .and_then(|previous| self.layers().get(previous).copied())
            .unwrap_or(0)
    }

    /// The depth of artwork layer `layer` as [`DrawList::vertex_depths`] gives it.
    #[must_use]
    pub fn layer_depth(&self, layer: usize) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        let depth = 1.0 - (layer + 1) as f32 / (self.layers().len() + 1) as f32;
        depth
    }
}

/// The frame of `image` as one closed outline of straight segments (for a stroke along it).
pub(crate) fn frame_anchors(image: &ImageSnapshot) -> Vec<AnchorSnapshot> {
    image
        .corners()
        .iter()
        .map(|&point| NewAnchor {
            id: crate::shape_preview::UNUSED_ANCHOR_ID,
            point,
            handle_in: Vec2::ZERO,
            handle_out: Vec2::ZERO,
            kind: curvyo_document_core::AnchorKind::Corner,
        })
        .collect()
}

/// The outline of an image whose picture cannot be read (`specs/0058-bitmap-import` criterion
/// 30): its frame and a diagonal cross, 1 px in the node outline colour at 60 %, over a casing.
/// Shape, not colour, says "unreadable".
#[must_use]
pub fn build_unreadable_frame(
    corners: [Point; 4],
    view: curvyo_document_core::ViewTransform,
) -> DrawList {
    let width_mm = crate::theme::NODE_OUTLINE_PX / view.scale();
    let tolerance_mm = crate::theme::DISPLAY_TOLERANCE_PX / view.scale();
    let anchor = |point: Point| NewAnchor {
        id: crate::shape_preview::UNUSED_ANCHOR_ID,
        point,
        handle_in: Vec2::ZERO,
        handle_out: Vec2::ZERO,
        kind: curvyo_document_core::AnchorKind::Corner,
    };
    let lines: [(Vec<AnchorSnapshot>, bool); 3] = [
        (corners.iter().map(|&p| anchor(p)).collect(), true),
        (vec![anchor(corners[0]), anchor(corners[2])], false),
        (vec![anchor(corners[1]), anchor(corners[3])], false),
    ];
    let colour = crate::theme::NODE_STROKE.with_alpha(153);
    crate::glyphs::cased(width_mm, colour, crate::theme::SELECTION_CASING, |w, c| {
        let mut list = DrawList::default();
        for (anchors, closed) in &lines {
            list.extend(crate::stroke::path_stroke(
                anchors,
                *closed,
                w,
                c,
                tolerance_mm,
            ));
        }
        list
    })
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Affine, ObjectSnapshot, Opacity, ViewTransform};

    use super::*;
    use crate::build_artwork;

    #[test]
    fn an_image_is_a_layer_of_its_own_between_its_neighbours() {
        let document = curvyo_document_core::Document::new(1);
        let id = document.create_rect(curvyo_document_core::RectBounds::from_corners(
            Point::new(0.0, 0.0),
            Point::new(4.0, 4.0),
        ));
        let image = document
            .add_images(&[curvyo_document_core::NewImage {
                bytes: std::sync::Arc::from(&b"x"[..]),
                placement: Affine::new(10.0, 0.0, 0.0, 5.0, 0.0, 0.0),
                name: String::new(),
            }])
            .unwrap()[0];
        let below = document.object(id).unwrap();
        let ObjectSnapshot::Image(mut image) = document.object(image).unwrap() else {
            unreachable!()
        };
        image.shown_opacity = Opacity::new(0.5).unwrap();
        let objects = [below.clone(), ObjectSnapshot::Image(image), below];
        let list = build_artwork(&objects, ViewTransform::identity());
        assert_eq!(list.images.len(), 1);
        let quad = &list.images[0];
        assert_eq!(quad.corners[2], Point::new(10.0, 5.0));
        assert!((quad.opacity.get() - 0.5).abs() < 1e-12);
        let layers = list.layers();
        assert!(quad.layer > 0 && quad.layer + 1 < layers.len());
        assert_eq!(
            list.layer_start(quad.layer),
            layers[quad.layer],
            "no triangles"
        );
        assert!(list.layer_depth(quad.layer) < list.layer_depth(quad.layer - 1));
    }
}

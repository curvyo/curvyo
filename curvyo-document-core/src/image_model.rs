//! The image object as read from the document (ADR 0017 §1): a leaf that shows one asset in the
//! unit square mapped by its own affine `placement`.

use serde::Serialize;

use crate::affine::Affine;
use crate::assets::AssetHash;
use crate::path_model::NodeId;
use crate::style_model::{Opacity, Stroke, Style};
use crate::units::Point;

/// The label of the import commit of one image; its history name is "Import image".
pub const IMPORT_IMAGE_COMMIT_LABEL: &str = "import_image";

/// The label of the import commit of several images; its history name is "Import images".
pub const IMPORT_IMAGES_COMMIT_LABEL: &str = "import_images";

/// The label of an import that brings SVG drawings and images in one step; its history name is
/// "Import" (`specs/0058-bitmap-import` criterion 5).
pub const IMPORT_COMMIT_LABEL: &str = "import";

/// One image object.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImageSnapshot {
    /// The node.
    pub id: NodeId,
    /// The picture's bytes in the asset store.
    pub asset: AssetHash,
    /// The map from the unit square (0,0)-(1,1), y down, the picture's top-left at (0,0), to
    /// document millimetres.
    pub placement: Affine,
    /// No paint: an image has no fill or stroke. Present so readers that take any object's style
    /// see one that paints nothing.
    #[serde(skip)]
    pub style: Style,
    /// The opacity it is drawn with: the node's effective opacity once the canvas faded it
    /// (`specs/0050-layers-and-objects-panel` criterion 27), opaque as read.
    #[serde(skip)]
    pub shown_opacity: Opacity,
}

impl ImageSnapshot {
    /// An image of `asset` at `placement`, as read.
    #[must_use]
    pub fn new(id: NodeId, asset: AssetHash, placement: Affine) -> Self {
        Self {
            id,
            asset,
            placement,
            style: no_paint(),
            shown_opacity: Opacity::OPAQUE,
        }
    }

    /// The four corners of the frame in document millimetres: the picture's top-left, top-right,
    /// bottom-right and bottom-left.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
            .map(|(x, y)| self.placement.apply_point(Point::new(x, y)))
    }

    /// The map from the pixel coordinates of a `width` x `height` working image (pixel (0, 0)'s
    /// top-left corner to (w, h)) to document millimetres: `placement` after a scale of
    /// `1/w, 1/h` (ADR 0017 §6, the trace hand-off of criterion 42).
    #[must_use]
    pub fn pixel_map(&self, width: u32, height: u32) -> Affine {
        self.placement.compose(Affine::scale(
            1.0 / f64::from(width.max(1)),
            1.0 / f64::from(height.max(1)),
        ))
    }

    /// This image with its frame mapped by `map` afterwards.
    #[must_use]
    pub fn mapped_by(&self, map: Affine) -> Self {
        Self {
            placement: map.compose(self.placement),
            ..self.clone()
        }
    }
}

/// The style of an image: neither fill nor stroke.
#[must_use]
pub fn no_paint() -> Style {
    Style {
        stroke: Stroke {
            enabled: false,
            ..Stroke::default()
        },
        ..Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pixel_map_puts_pixel_corners_on_the_frame() {
        let image = ImageSnapshot::new(
            NodeId::from_parts(1, 1),
            AssetHash::of(b"x"),
            Affine::new(40.0, 0.0, 0.0, 20.0, 10.0, 5.0),
        );
        let map = image.pixel_map(4, 2);
        assert_eq!(
            map.apply_point(Point::new(4.0, 2.0)),
            Point::new(50.0, 25.0)
        );
        assert_eq!(image.corners()[2], Point::new(50.0, 25.0));
    }
}

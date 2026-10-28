//! The Trace tool's tessellated previews kept across frames (`specs/0025-raster-trace`
//! criterion 56): a mesh is tessellated once per zoom and drawn again by the view transform, also
//! at another zoom while the view is still moving.

use std::cell::RefCell;

use curvyo_document_core::Point;
use curvyo_render_core::DrawList;

/// One tessellation: the zoom and strength it was made for and, for a culled one, the area
/// (document millimetres) whose content it holds.
#[derive(Debug, Clone, PartialEq)]
struct Mesh {
    scale: f64,
    stale: bool,
    area: Option<(Point, Point)>,
    list: DrawList,
}

/// The kept mesh of one preview; empty until the first frame draws it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct MeshCache(RefCell<Option<Mesh>>);

/// What a frame asks of a [`MeshCache`].
#[derive(Debug, Clone, Copy)]
pub(super) struct MeshFrame {
    /// The zoom of the frame.
    pub(super) scale: f64,
    /// The preview is drawn at half strength.
    pub(super) stale: bool,
    /// The visible part of the canvas, or `None` when nothing is culled.
    pub(super) visible: Option<(Point, Point)>,
    /// The view has not rested since it last moved.
    pub(super) moving: bool,
}

impl MeshCache {
    /// The kept mesh when this frame can draw it, else `build` (which returns the mesh and the
    /// area it covers) kept for the next frames. A moving view draws the kept mesh at any zoom
    /// and offset, so a pan or zoom never tessellates; once it rests, a mesh of another zoom, or
    /// one that does not cover the visible part, is made again.
    pub(super) fn draw(
        &self,
        frame: MeshFrame,
        build: impl FnOnce() -> (DrawList, Option<(Point, Point)>),
    ) -> DrawList {
        self.with(frame, build, DrawList::clone)
    }

    /// [`MeshCache::draw`] handing the mesh to `read` instead of a copy: an overlay mesh of a
    /// hundred thousand nodes is tens of megabytes, copied once into the frame, not twice.
    pub(super) fn with<R>(
        &self,
        frame: MeshFrame,
        build: impl FnOnce() -> (DrawList, Option<(Point, Point)>),
        read: impl FnOnce(&DrawList) -> R,
    ) -> R {
        let mut kept = self.0.borrow_mut();
        let reusable = kept.as_ref().is_some_and(|mesh| {
            mesh.stale == frame.stale
                && (frame.moving
                    || (mesh.scale.to_bits() == frame.scale.to_bits()
                        && covers(mesh.area, frame.visible)))
        });
        if !reusable {
            let (list, area) = build();
            *kept = Some(Mesh {
                scale: frame.scale,
                stale: frame.stale,
                area,
                list,
            });
        }
        // invariant: a mesh was kept above when none could be reused.
        #[allow(clippy::unwrap_used)]
        read(&kept.as_ref().unwrap().list)
    }
}

/// Whether a mesh holding `area` (`None`: everything) holds all of `visible`.
fn covers(area: Option<(Point, Point)>, visible: Option<(Point, Point)>) -> bool {
    match (area, visible) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some((lo, hi)), Some((min, max))) => {
            lo.x <= min.x && lo.y <= min.y && hi.x >= max.x && hi.y >= max.y
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn frame(scale: f64, moving: bool) -> MeshFrame {
        MeshFrame {
            scale,
            stale: false,
            visible: Some((Point::new(0.0, 0.0), Point::new(10.0, 10.0))),
            moving,
        }
    }

    /// A rested view rebuilds at a new zoom or outside the kept area; a moving one never does.
    #[test]
    fn a_moving_view_reuses_the_mesh_and_a_rested_one_remakes_it() {
        let cache = MeshCache::default();
        let builds = Cell::new(0);
        let build = || {
            builds.set(builds.get() + 1);
            (
                DrawList::default(),
                Some((Point::new(-10.0, -10.0), Point::new(20.0, 20.0))),
            )
        };
        cache.draw(frame(1.0, false), build);
        cache.draw(frame(1.0, false), build);
        assert_eq!(builds.get(), 1, "same zoom, inside the area");
        cache.draw(frame(2.0, true), build);
        assert_eq!(builds.get(), 1, "a zoom in motion draws the kept mesh");
        cache.draw(frame(2.0, false), build);
        assert_eq!(builds.get(), 2, "at rest the new zoom is tessellated");
        let mut far = frame(2.0, false);
        far.visible = Some((Point::new(50.0, 50.0), Point::new(60.0, 60.0)));
        cache.draw(far, build);
        assert_eq!(builds.get(), 3, "a rested pan past the area");
        let mut stale = frame(2.0, true);
        stale.stale = true;
        cache.draw(stale, build);
        assert_eq!(builds.get(), 4, "a change of strength always");
    }
}

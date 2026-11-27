//! A generated object's outline as a path snapshot, for a ghost or a live preview.

use curvyo_document_core::{
    Affine, Angle, CurveKind, GeneratedRecord, GeneratedSnapshot, NodeId, PathSnapshot, Point,
    Style,
};

/// The outline of `snapshot` as one compound path in the style `style`, its id the object's own;
/// `None` for a record without a subpath.
#[must_use]
pub fn outline_path(snapshot: &GeneratedSnapshot, style: Style) -> Option<PathSnapshot> {
    let mut subpaths = snapshot.outlines().into_iter().flatten();
    let first = subpaths.next()?;
    Some(PathSnapshot {
        id: snapshot.id,
        closed: true,
        style,
        anchors: first.anchors,
        extra_subpaths: subpaths.collect(),
        curve_kind: CurveKind::default(),
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: Angle::from_radians(snapshot.frame.b.atan2(snapshot.frame.a)),
        paint_runs: Vec::new(),
    })
}

/// The outline `record` would have placed with its origin at `at`: what the Generators tool's ghost
/// draws under the pointer.
#[must_use]
pub fn ghost_path(record: &GeneratedRecord, at: Point) -> Option<PathSnapshot> {
    let snapshot = GeneratedSnapshot {
        id: NodeId::NONE,
        record: record.clone(),
        frame: Affine::new(1.0, 0.0, 0.0, 1.0, at.x, at.y),
    };
    outline_path(&snapshot, Style::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::ExtensionSettings;
    use crate::instance::{find_kind, new_record};

    #[test]
    fn the_ghost_of_a_rounded_box_has_no_slits() {
        let found = find_kind(
            &ExtensionSettings::default(),
            "curvyo.generators/rounded-box",
        )
        .expect("rounded box");
        let ghost = crate::instance::new_ghost_record(&found).expect("ghost");
        let (real, _) = new_record(&found).expect("defaults");
        let ghost = ghost_path(&ghost, Point::new(0.0, 0.0)).expect("outline");
        let real = ghost_path(&real, Point::new(0.0, 0.0)).expect("outline");
        assert_eq!(ghost.extra_subpaths.len(), real.extra_subpaths.len());
        assert!(
            ghost.anchors.len() <= 4 + 1,
            "the strip is a plain rectangle"
        );
        assert!(real.anchors.len() > 100, "the real strip carries its slits");
    }

    #[test]
    fn the_ghost_of_a_box_is_its_plates_around_the_pointer() {
        let found = find_kind(&ExtensionSettings::default(), "curvyo.generators/box").expect("box");
        let (record, _) = new_record(&found).expect("defaults");
        let path = ghost_path(&record, Point::new(500.0, 300.0)).expect("outline");
        // Five plates: the first outline and four more, none of them the bounding box.
        assert_eq!(path.extra_subpaths.len(), 4);
        assert!(path.anchors.len() > 4, "the real outline has fingers");
        let xs: Vec<f64> = path.all_anchors().map(|anchor| anchor.point.x).collect();
        let (min, max) = (
            xs.iter().copied().fold(f64::MAX, f64::min),
            xs.iter().copied().fold(f64::MIN, f64::max),
        );
        assert!(
            (f64::midpoint(min, max) - 500.0).abs() < 1.0,
            "centred on the pointer"
        );
    }
}

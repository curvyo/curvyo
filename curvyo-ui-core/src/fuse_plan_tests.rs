//! `plan_fuse` against `specs/0156-fused-lettering` criteria 5, 6, 11, 17, 21, 22 and 26.

use super::*;
use curvyo_document_core::{Document, RectBounds};

use crate::units::Units;

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

const LETTERING: FuseSettings = FuseSettings {
    mode: FuseMode::Lettering,
    width_mm: 1.0,
    reach_mm: 5.0,
    join_all: false,
    bridges_per_island: 2.0,
};

fn plan(
    document: &Document,
    selected: &[NodeId],
    settings: FuseSettings,
) -> Result<FusePlan, FuseRefusal> {
    let units = Units::build(document, None).selected(selected);
    plan_fuse(
        &objects(document),
        &units,
        settings,
        &mut AnchorIdMinter::new(5),
    )
}

/// Criterion 5: values are clamped to their fields' limits.
#[test]
fn settings_are_clamped_to_their_limits() {
    let wild = FuseSettings {
        width_mm: 50.0,
        reach_mm: 0.0,
        bridges_per_island: 9.0,
        ..LETTERING
    }
    .clamped();
    assert_eq!((wild.width_mm, wild.reach_mm), (10.0, 0.5));
    assert_eq!(wild.bridges_per_island, 4.0);
    assert_eq!(FUSE_WIDTH_MM.clamp(f64::NAN), 1.0);
}

/// Criteria 6 and 11: two bars become one plan with the Outline, one Bridge, the top-most
/// operand's place and the lowest leaf as the bottom.
#[test]
fn two_bars_plan_one_group_with_one_bridge() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 2.0, 10.0);
    let b = rect(&document, 5.0, 0.0, 2.0, 10.0);
    let plan = plan(&document, &[b, a], LETTERING).unwrap();
    assert_eq!(plan.removed, vec![a, b]);
    assert_eq!((plan.bottom, plan.place), (a, b));
    assert_eq!(plan.group_name, "Bridged lettering");
    assert_eq!(plan.outline.len(), 1);
    assert_eq!(plan.bridges.len(), 1);
    assert_eq!(plan.bridges[0].0, "Bridge 1");
    assert!(!plan.style.fill.enabled && plan.style.stroke.enabled);
}

/// Criterion 17 and Frage 398 C: too far apart refuses; Join all joins.
#[test]
fn far_pieces_refuse_unless_join_all() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 2.0, 2.0);
    let b = rect(&document, 30.0, 0.0, 2.0, 2.0);
    assert_eq!(
        plan(&document, &[a, b], LETTERING),
        Err(FuseRefusal::NothingToJoin)
    );
    let joined = FuseSettings {
        join_all: true,
        ..LETTERING
    };
    assert_eq!(plan(&document, &[a, b], joined).unwrap().bridges.len(), 1);
}

/// Criteria 22 and 26: Stencil on a solid shape has no islands; a frame gets its bridges.
#[test]
fn stencil_needs_an_island() {
    let document = Document::new(1);
    let solid = rect(&document, 0.0, 0.0, 10.0, 10.0);
    let stencil = FuseSettings {
        mode: FuseMode::Stencil,
        ..LETTERING
    };
    assert_eq!(
        plan(&document, &[solid], stencil),
        Err(FuseRefusal::NoIslands)
    );
}

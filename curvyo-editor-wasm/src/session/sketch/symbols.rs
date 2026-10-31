//! Constraint symbols: which glyph each geometric constraint shows and where it is anchored (AC32).
use super::Session;
use curvyo_document_core::{Affine, Point, SketchSnapshot, Vec2};
use curvyo_sketch_core::{ConstraintId, Entity, EntityId, Relation, Sketch};
use curvyo_ui_core::{SymbolAnchor, SymbolItem, SymbolPlacement};
use std::collections::BTreeMap;

/// A badge is hit within 10px of its centre (16px padded to 20px).
const BADGE_HIT_HALF_PX: f64 = 10.0;

/// One constraint symbol, laid out for the current view.
#[allow(clippy::struct_excessive_bools)] // Independent presentation flags, not a state machine.
pub(super) struct SymbolRecord {
    pub id: ConstraintId,
    /// Glyph name, shared with the rail icons: "horizontal", "equal", "on-axis", ...
    pub glyph: &'static str,
    /// The equal group's number, shown as a small digit.
    pub index: Option<usize>,
    pub name: String,
    pub selected: bool,
    pub hovered: bool,
    pub conflicting: bool,
    /// Thinned to a 6px dot.
    pub dot: bool,
    pub screen: (f64, f64),
}

/// A "+n" badge for folded symbols of one entity.
pub(super) struct MoreRecord {
    pub count: usize,
    pub screen: (f64, f64),
}

impl SymbolRecord {
    pub fn hit(&self, x: f64, y: f64) -> bool {
        !self.dot
            && (x - self.screen.0).abs() <= BADGE_HIT_HALF_PX
            && (y - self.screen.1).abs() <= BADGE_HIT_HALF_PX
    }
}

impl Session {
    /// Every symbol of `snapshot` plus the "+n" badges, clear of the dimension texts, which
    /// count toward thinning.
    pub(super) fn sketch_symbol_records(
        &self,
        snapshot: &SketchSnapshot,
    ) -> (Vec<SymbolRecord>, Vec<MoreRecord>) {
        let texts = self
            .sketch_dimension_records(snapshot)
            .iter()
            .map(super::annotations::DimensionRecord::text_box)
            .collect::<Vec<_>>();
        let solution = self.sketch_preview().unwrap_or(&snapshot.solution);
        let placement = snapshot.placement;
        let keys: BTreeMap<&EntityId, usize> = solution
            .entities
            .keys()
            .enumerate()
            .map(|(i, id)| (id, i))
            .collect();
        let groups = equal_groups(&snapshot.graph);
        let mut expanded_entities = self.sketch.entities.clone();
        expanded_entities.extend(self.sketch.preselection.iter().cloned());
        let mut items = Vec::new();
        let mut pending = Vec::new();
        for (id, constraint) in &snapshot.graph.constraints {
            let Some(glyph) = glyph(&constraint.relation) else {
                continue;
            };
            let selected = self.sketch.constraints.contains(id);
            let hovered = self.sketch.annotation_hover.as_ref() == Some(id);
            let conflicting = snapshot.conflicting.contains(id) || snapshot.redundant.contains(id);
            let index = groups.get(id).copied();
            for (anchor, entity) in anchors(solution, &constraint.relation, placement, &keys) {
                items.push(SymbolItem {
                    anchor,
                    keep: selected || hovered || conflicting,
                    expanded: entity.is_some_and(|e| expanded_entities.contains(e)),
                });
                let name = format!(
                    "{} constraint{}{}",
                    title(glyph),
                    index.map(|i| format!(" {i}")).unwrap_or_default(),
                    if conflicting { ", conflicting" } else { "" }
                );
                pending.push((
                    id.clone(),
                    glyph,
                    index,
                    name,
                    selected,
                    hovered,
                    conflicting,
                ));
            }
        }
        let view = self.view();
        let layout = curvyo_ui_core::layout_symbols(&items, &texts, view);
        let records = layout
            .placements
            .into_iter()
            .zip(pending)
            .filter_map(
                |(placement, (id, glyph, index, name, selected, hovered, conflicting))| {
                    let (center, dot) = match placement {
                        SymbolPlacement::Badge(at) => (at, false),
                        SymbolPlacement::Dot(at) => (at, true),
                        SymbolPlacement::Folded => return None,
                    };
                    Some(SymbolRecord {
                        id,
                        glyph,
                        index,
                        name,
                        selected,
                        hovered,
                        conflicting,
                        dot,
                        screen: view.document_to_screen(center),
                    })
                },
            )
            .collect();
        let more = layout
            .more
            .into_iter()
            .map(|badge| MoreRecord {
                count: badge.count,
                screen: view.document_to_screen(badge.center),
            })
            .collect();
        (records, more)
    }
}

/// The glyph of a geometric constraint; dimensions and structural relations have none.
fn glyph(relation: &Relation) -> Option<&'static str> {
    Some(match relation {
        Relation::Coincident(..) => "coincident",
        Relation::Horizontal(..) => "horizontal",
        Relation::Vertical(..) => "vertical",
        Relation::Parallel(..) => "parallel",
        Relation::Perpendicular(..) | Relation::PerpendicularAt { .. } => "perpendicular",
        Relation::Tangent(..) => "tangent",
        Relation::Equal(..) => "equal",
        Relation::Fixed(..) => "fixed",
        Relation::Symmetric { .. } => "symmetric",
        Relation::Midpoint { .. } => "midpoint",
        Relation::OnAxis { .. } => "on-axis",
        _ => return None,
    })
}

fn title(glyph: &str) -> String {
    let name = if glyph == "on-axis" { "on axis" } else { glyph };
    let mut chars = name.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Equal constraints joined into groups (chains of equal entities share one), numbered from 1 in
/// the order of the sketch's constraints.
fn equal_groups(graph: &Sketch) -> BTreeMap<ConstraintId, usize> {
    let mut group_of: BTreeMap<EntityId, usize> = BTreeMap::new();
    let mut parent: Vec<usize> = Vec::new();
    let find = |parent: &[usize], mut i: usize| {
        while parent[i] != i {
            i = parent[i];
        }
        i
    };
    let mut members = Vec::new();
    for (id, constraint) in &graph.constraints {
        let Relation::Equal(a, b) = &constraint.relation else {
            continue;
        };
        let mut slot = |e: &EntityId, parent: &mut Vec<usize>| {
            *group_of.entry(e.clone()).or_insert_with(|| {
                parent.push(parent.len());
                parent.len() - 1
            })
        };
        let (ga, gb) = (slot(a, &mut parent), slot(b, &mut parent));
        let (ra, rb) = (find(&parent, ga), find(&parent, gb));
        parent[ra.max(rb)] = ra.min(rb);
        members.push((id.clone(), ga));
    }
    let mut numbers: BTreeMap<usize, usize> = BTreeMap::new();
    members
        .into_iter()
        .map(|(id, slot)| {
            let root = find(&parent, slot);
            let next = numbers.len() + 1;
            (id, *numbers.entry(root).or_insert(next))
        })
        .collect()
}

fn doc(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<Point> {
    let p = solution.point(id).ok()?;
    Some(placement.apply_point(Point::new(p.x.0, p.y.0)))
}

/// An entity's midpoint and direction there, in document coordinates.
fn middle(solution: &Sketch, id: &EntityId, placement: Affine) -> Option<(Point, Vec2)> {
    match solution.entities.get(id)? {
        Entity::Point { .. } => Some((doc(solution, id, placement)?, Vec2::new(1.0, 0.0))),
        Entity::Line { start, end } => {
            let (a, b) = (
                doc(solution, start, placement)?,
                doc(solution, end, placement)?,
            );
            Some((
                Point::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
                a.vector_to(b),
            ))
        }
        Entity::Circle { center, radius, .. } => {
            let c = solution.point(center).ok()?;
            let top = Point::new(c.x.0, c.y.0 - radius.0);
            Some((
                placement.apply_point(top),
                placement.apply_vector(Vec2::new(1.0, 0.0)),
            ))
        }
        Entity::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let c = solution.point(center).ok()?;
            let mut sweep = end_angle.0 - start_angle.0;
            if sweep < 0.0 {
                sweep += std::f64::consts::TAU;
            }
            let m = start_angle.0 + sweep / 2.0;
            let at = Point::new(c.x.0 + radius.0 * m.cos(), c.y.0 + radius.0 * m.sin());
            Some((
                placement.apply_point(at),
                placement.apply_vector(Vec2::new(-m.sin(), m.cos())),
            ))
        }
        Entity::Ellipse { center, focus, .. } => {
            let (c, f) = (
                doc(solution, center, placement)?,
                doc(solution, focus, placement)?,
            );
            Some((c, c.vector_to(f)))
        }
    }
}

/// The point two entities share, if they adjoin.
fn shared_point(solution: &Sketch, a: &EntityId, b: &EntityId) -> Option<EntityId> {
    let ends = |id: &EntityId| -> Vec<EntityId> {
        match solution.entities.get(id) {
            Some(Entity::Line { start, end } | Entity::Arc { start, end, .. }) => {
                vec![start.clone(), end.clone()]
            }
            _ => Vec::new(),
        }
    };
    let other = ends(b);
    ends(a).into_iter().find(|p| other.contains(p))
}

/// Where a two-entity symbol goes: the shared point, else between the two midpoints.
fn pair_spot(solution: &Sketch, a: &EntityId, b: &EntityId, placement: Affine) -> Option<Point> {
    if let Some(shared) = shared_point(solution, a, b) {
        return doc(solution, &shared, placement);
    }
    let (ma, _) = middle(solution, a, placement)?;
    let (mb, _) = middle(solution, b, placement)?;
    Some(Point::new(
        f64::midpoint(ma.x, mb.x),
        f64::midpoint(ma.y, mb.y),
    ))
}

/// The badges of one constraint: one anchor, or one per member for Equal; with the entity whose
/// hover or selection expands the row.
fn anchors<'a>(
    solution: &Sketch,
    relation: &'a Relation,
    placement: Affine,
    keys: &BTreeMap<&EntityId, usize>,
) -> Vec<(SymbolAnchor, Option<&'a EntityId>)> {
    let on_entity = |id: &'a EntityId| {
        let key = *keys.get(id)?;
        if matches!(solution.entities.get(id), Some(Entity::Point { .. })) {
            return Some((
                SymbolAnchor::Point {
                    key,
                    at: doc(solution, id, placement)?,
                },
                Some(id),
            ));
        }
        let (mid, direction) = middle(solution, id, placement)?;
        Some((
            SymbolAnchor::Entity {
                key,
                mid,
                direction,
            },
            Some(id),
        ))
    };
    let spot = |at: Option<Point>| at.map(|at| (SymbolAnchor::Spot { at }, None));
    let found = match relation {
        Relation::Horizontal(id) | Relation::Vertical(id) | Relation::Fixed(id) => {
            vec![on_entity(id)]
        }
        Relation::OnAxis { point, .. }
        | Relation::Coincident(point, _)
        | Relation::Midpoint { point, .. } => vec![on_entity(point)],
        Relation::Equal(a, b) => vec![on_entity(a), on_entity(b)],
        Relation::Parallel(a, b) | Relation::Perpendicular(a, b) | Relation::Tangent(a, b) => {
            vec![spot(pair_spot(solution, a, b, placement))]
        }
        Relation::PerpendicularAt { point, .. } => vec![spot(doc(solution, point, placement))],
        Relation::Symmetric { source, image, .. } => {
            let (s, i) = (
                doc(solution, source, placement),
                doc(solution, image, placement),
            );
            vec![spot(s.zip(i).map(|(s, i)| {
                Point::new(f64::midpoint(s.x, i.x), f64::midpoint(s.y, i.y))
            }))]
        }
        _ => Vec::new(),
    };
    found.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_sketch_core::{Constraint, Position};

    #[test]
    fn equal_chains_share_one_number_in_constraint_order() {
        let mut graph = curvyo_sketch_core::create_rectangle(
            &Sketch::new("origin".into()),
            "r",
            Position::mm(0.0, 0.0),
            Position::mm(30.0, 20.0),
        )
        .unwrap();
        for (id, a, b) in [
            ("e1", "r:line0", "r:line2"),
            ("e2", "r:line1", "r:line3"),
            ("e3", "r:line2", "r:line0"),
        ] {
            graph.constraints.insert(
                id.into(),
                Constraint::driving(Relation::Equal(a.into(), b.into())),
            );
        }
        let groups = equal_groups(&graph);
        assert_eq!(groups[&"e1".into()], 1);
        assert_eq!(groups[&"e2".into()], 2);
        assert_eq!(groups[&"e3".into()], 1);
    }
}

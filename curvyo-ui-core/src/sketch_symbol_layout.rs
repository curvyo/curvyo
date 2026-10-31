//! Places constraint symbol badges next to their geometry without overlap, thinning crowded sketches (AC32).
//!
//! Sizes are screen pixels turned into document millimetres with the view scale (UX notes
//! revision 2, section 7, "Constraint symbols").
use std::collections::BTreeMap;

use curvyo_document_core::{Point, Vec2, ViewTransform};

/// Distance of an entity's badge row from the entity, to the left of its direction.
const ENTITY_OFFSET_PX: f64 = 12.0;
/// Offset of a point's badges up and right of the point.
const POINT_OFFSET_PX: f64 = 10.0;
/// Badge size plus the 2px gap between badges in a row.
const PITCH_PX: f64 = 18.0;
/// At most this many badges per entity before a "+n" badge.
const ROW_LIMIT: usize = 3;
/// Above this many annotations (symbols plus dimensions) unkept symbols thin to dots.
pub const THINNING_LIMIT: usize = 150;
/// How often a badge moves on before it is left where it collides.
const MAX_SHIFTS: usize = 8;
/// Below this length in millimetres a direction is undefined.
const DIRECTION_TOLERANCE_MM: f64 = 1e-9;

/// Where a symbol belongs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolAnchor {
    /// Next to an entity: badges of one entity form one row along it.
    Entity {
        /// Groups the badges of one entity.
        key: usize,
        /// The entity's midpoint.
        mid: Point,
        /// The entity's direction at the midpoint.
        direction: Vec2,
    },
    /// Up and right of a point: badges of one point form one row.
    Point {
        /// Groups the badges of one point.
        key: usize,
        /// The point.
        at: Point,
    },
    /// Exactly at a spot: a shared point or the middle between two entities.
    Spot {
        /// The badge centre before overlap avoidance.
        at: Point,
    },
}

/// One constraint symbol to place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SymbolItem {
    /// Where it belongs.
    pub anchor: SymbolAnchor,
    /// Selected, hovered or conflicting symbols never thin and never fold into "+n".
    pub keep: bool,
    /// The entity is hovered or selected: its whole row shows.
    pub expanded: bool,
}

/// How one symbol is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolPlacement {
    /// A full 16px badge centred here.
    Badge(Point),
    /// A 6px dot here: the sketch is crowded (thinning).
    Dot(Point),
    /// Folded into its row's "+n" badge.
    Folded,
}

/// A "+n" badge standing for the folded symbols of one row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoreBadge {
    /// Badge centre.
    pub center: Point,
    /// How many symbols it stands for.
    pub count: usize,
}

/// Every symbol's placement, in input order, and the rows' "+n" badges.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolLayout {
    /// One placement per input item.
    pub placements: Vec<SymbolPlacement>,
    /// One badge per folded row.
    pub more: Vec<MoreBadge>,
}

/// A dimension text box symbols keep clear of; sizes in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBox {
    /// Text centre.
    pub center: Point,
    /// Half the text width, screen pixels.
    pub half_width_px: f64,
    /// Half the text height, screen pixels.
    pub half_height_px: f64,
}

/// Places `items` clear of each other and of the dimension `texts`, which also count toward the
/// thinning limit.
#[must_use]
pub fn layout_symbols(
    items: &[SymbolItem],
    texts: &[TextBox],
    view: ViewTransform,
) -> SymbolLayout {
    let px = 1.0 / view.scale();
    let thin = items.len() + texts.len() > THINNING_LIMIT;
    let mut placements = vec![SymbolPlacement::Folded; items.len()];
    let mut more = Vec::new();
    let half = PITCH_PX / 2.0 * px;
    let mut taken: Vec<(Point, f64, f64)> = texts
        .iter()
        .map(|t| (t.center, t.half_width_px * px, t.half_height_px * px))
        .collect();
    for (row, members) in rows(items) {
        let shown = members.iter().filter(|&&i| !thin || items[i].keep).count();
        let expanded = members.iter().any(|&i| items[i].expanded);
        let limit = if expanded { usize::MAX } else { ROW_LIMIT };
        let mut visible: Vec<usize> = members
            .iter()
            .copied()
            .filter(|&i| !thin || items[i].keep)
            .collect();
        // Kept symbols claim the row's slots first.
        visible.sort_by_key(|&i| !items[i].keep);
        let folded = shown.saturating_sub(limit);
        visible.truncate(shown - folded);
        visible.sort_unstable();
        let slots = visible.len() + usize::from(folded > 0);
        for (slot, &index) in visible.iter().enumerate() {
            let center = settle(slot_center(row, slot, slots, px), row, &taken, px);
            taken.push((center, half, half));
            placements[index] = SymbolPlacement::Badge(center);
        }
        if folded > 0 {
            let center = settle(slot_center(row, slots - 1, slots, px), row, &taken, px);
            taken.push((center, half, half));
            more.push(MoreBadge {
                center,
                count: folded,
            });
        }
        if thin {
            for &index in members.iter().filter(|&&i| !items[i].keep) {
                placements[index] = SymbolPlacement::Dot(slot_center(row, 0, 1, px));
            }
        }
    }
    SymbolLayout { placements, more }
}

/// Groups items into rows: one per entity, one per point, one per spot item; in input order.
fn rows(items: &[SymbolItem]) -> Vec<(SymbolAnchor, Vec<usize>)> {
    let mut rows: Vec<(SymbolAnchor, Vec<usize>)> = Vec::new();
    let mut by_key: BTreeMap<(u8, usize), usize> = BTreeMap::new();
    for (index, item) in items.iter().enumerate() {
        let key = match item.anchor {
            SymbolAnchor::Entity { key, .. } => Some((0, key)),
            SymbolAnchor::Point { key, .. } => Some((1, key)),
            SymbolAnchor::Spot { .. } => None,
        };
        if let Some(row) = key.and_then(|key| by_key.get(&key).copied()) {
            rows[row].1.push(index);
        } else {
            if let Some(key) = key {
                by_key.insert(key, rows.len());
            }
            rows.push((item.anchor, vec![index]));
        }
    }
    rows
}

fn unit(v: Vec2) -> Vec2 {
    if v.length() <= DIRECTION_TOLERANCE_MM {
        Vec2::new(1.0, 0.0)
    } else {
        v.normalized_to(1.0)
    }
}

/// The row direction and, for an entity, the side the row stands on.
fn frame(anchor: SymbolAnchor) -> (Vec2, Vec2) {
    match anchor {
        SymbolAnchor::Entity { direction, .. } => {
            let along = unit(direction);
            let mut left = Vec2::new(along.y, -along.x);
            if left.y > DIRECTION_TOLERANCE_MM
                || (left.y.abs() <= DIRECTION_TOLERANCE_MM && left.x > 0.0)
            {
                left = left.negated();
            }
            (along, left)
        }
        SymbolAnchor::Point { .. } | SymbolAnchor::Spot { .. } => {
            (Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0))
        }
    }
}

fn slot_center(anchor: SymbolAnchor, slot: usize, slots: usize, px: f64) -> Point {
    let (along, left) = frame(anchor);
    #[allow(clippy::cast_precision_loss)] // a row holds a handful of badges
    let (slot, slots) = (slot as f64, slots as f64);
    match anchor {
        SymbolAnchor::Entity { mid, .. } => mid
            .translated(left.scaled(ENTITY_OFFSET_PX * px))
            .translated(along.scaled((slot - (slots - 1.0) / 2.0) * PITCH_PX * px)),
        SymbolAnchor::Point { at, .. } => Point::new(
            at.x + (POINT_OFFSET_PX + slot * PITCH_PX) * px,
            at.y - POINT_OFFSET_PX * px,
        ),
        SymbolAnchor::Spot { at } => Point::new(at.x + slot * PITCH_PX * px, at.y),
    }
}

/// Moves `center` away from every text and earlier badge it would overlap: entity rows step
/// further out, point and spot badges step right.
fn settle(mut center: Point, anchor: SymbolAnchor, taken: &[(Point, f64, f64)], px: f64) -> Point {
    let (_, left) = frame(anchor);
    let step = match anchor {
        SymbolAnchor::Entity { .. } => left,
        SymbolAnchor::Point { .. } | SymbolAnchor::Spot { .. } => Vec2::new(1.0, 0.0),
    }
    .scaled(PITCH_PX * px);
    let half = PITCH_PX / 2.0 * px;
    // A touching edge is clear: a relative tolerance against rounding.
    let reach = |extent: f64| (extent + half) * (1.0 - 1e-9);
    for _ in 0..MAX_SHIFTS {
        let clear = taken.iter().all(|(other, w, h)| {
            (other.x - center.x).abs() >= reach(*w) || (other.y - center.y).abs() >= reach(*h)
        });
        if clear {
            break;
        }
        center = center.translated(step);
    }
    center
}

#[cfg(test)]
#[path = "sketch_symbol_layout_tests.rs"]
mod tests;

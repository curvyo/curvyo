//! The own paint of a generated part and the pure reads that split a painted path (ADR 0091
//! "Per-part paint"): `paint_groups` for the painters, `job_subpaths` for every machine job.

use serde::{Deserialize, Serialize};

use crate::path_model::{Color, PathSnapshot, SubpathRef};
use crate::pattern_model::FillPaint;
use crate::style_model::Style;

/// The own paint of one part: a solid colour and whether the part reaches machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPaint {
    /// The solid fill colour.
    pub fill: Color,
    /// Whether the part runs on machine jobs; `false` is printed and exported, never burnt
    /// (a registration mark's rule, ADR 0081 §1).
    pub job: bool,
}

impl PartPaint {
    /// `style` as this paint draws it: a solid fill of its colour (opacity kept, fill on) and, for a
    /// part that never reaches a job, no stroke.
    #[must_use]
    pub fn apply(&self, style: &Style) -> Style {
        let mut painted = style.clone();
        painted.fill.enabled = true;
        painted.fill.paint = FillPaint::Colour;
        painted.fill.color = self.fill;
        if !self.job {
            painted.stroke.enabled = false;
        }
        painted
    }
}

/// A stretch of consecutive subpaths of a path that one part painted (indices count the first
/// outline as 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaintRun {
    /// The first subpath of the run.
    pub first: u32,
    /// How many subpaths it holds.
    pub count: u32,
    /// The part's paint.
    pub paint: PartPaint,
}

/// One fill of a painted path: a stretch of subpaths and the style it is painted with.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintGroup {
    /// The first subpath.
    pub first: usize,
    /// How many subpaths.
    pub count: usize,
    /// The style to paint them with.
    pub style: Style,
    /// Whether this is a stretch of the object's own style (it keeps a gradient or a pattern).
    pub own_style: bool,
}

impl PathSnapshot {
    /// The fills this path is painted as, in part order. A path without paint runs is one group in
    /// its own style. An unpainted stretch keeps the node's style, a painted one the node's style
    /// with a solid fill of its colour (opacity kept), a `job: false` one its fill only, no stroke.
    #[must_use]
    pub fn paint_groups(&self) -> Vec<PaintGroup> {
        let total = 1 + self.extra_subpaths.len();
        let mut groups = Vec::new();
        let mut at = 0;
        let own = |from: usize, to: usize, groups: &mut Vec<PaintGroup>| {
            if to > from {
                groups.push(PaintGroup {
                    first: from,
                    count: to - from,
                    style: self.style.clone(),
                    own_style: true,
                });
            }
        };
        for run in &self.paint_runs {
            let first = (run.first as usize).min(total);
            let end = (first + run.count as usize).min(total);
            own(at, first, &mut groups);
            let style = run.paint.apply(&self.style);
            if end > first {
                groups.push(PaintGroup {
                    first,
                    count: end - first,
                    style,
                    own_style: false,
                });
            }
            at = end;
        }
        own(at, total, &mut groups);
        groups
    }

    /// The stretch `group` of this path as a path of its own, in the group's style, with no runs: what
    /// an exporter writes as one element.
    #[must_use]
    pub fn painted_part(&self, group: &PaintGroup) -> Self {
        let mut stretch = self.subpaths().skip(group.first).take(group.count);
        let first = stretch.next();
        let rest: Vec<crate::path_model::SubpathSnapshot> = stretch
            .map(|sub| crate::path_model::SubpathSnapshot {
                closed: sub.closed,
                anchors: sub.anchors.to_vec(),
            })
            .collect();
        Self {
            id: self.id,
            closed: first.as_ref().is_none_or(|sub| sub.closed),
            style: group.style.clone(),
            anchors: first.map(|sub| sub.anchors.to_vec()).unwrap_or_default(),
            extra_subpaths: rest,
            curve_kind: self.curve_kind,
            spiro_transform: self.spiro_transform.clone(),
            width_points: Vec::new(),
            paint_runs: Vec::new(),
            rotation: self.rotation,
        }
    }

    /// This path as a job reads it: without the outlines of a `job: false` part and without paint
    /// runs (a look drawn under an operation is one style over the whole job geometry).
    #[must_use]
    pub fn job_path(&self) -> Self {
        if self.paint_runs.is_empty() {
            return self.clone();
        }
        let mut kept = self.job_subpaths();
        let Some(first) = kept.next() else {
            return Self {
                paint_runs: Vec::new(),
                ..self.clone()
            };
        };
        let rest = kept
            .map(|sub| crate::path_model::SubpathSnapshot {
                closed: sub.closed,
                anchors: sub.anchors.to_vec(),
            })
            .collect();
        Self {
            closed: first.closed,
            anchors: first.anchors.to_vec(),
            extra_subpaths: rest,
            paint_runs: Vec::new(),
            ..self.clone()
        }
    }

    /// Every outline that reaches a machine job: all of them but those a `job: false` run holds.
    /// The one funnel of every job (ADR 0091 P6).
    pub fn job_subpaths(&self) -> impl Iterator<Item = SubpathRef<'_>> {
        self.subpaths().enumerate().filter_map(|(index, sub)| {
            let hidden = self.paint_runs.iter().any(|run| {
                !run.paint.job
                    && index >= run.first as usize
                    && index < (run.first + run.count) as usize
            });
            (!hidden).then_some(sub)
        })
    }
}

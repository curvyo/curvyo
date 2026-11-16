//! Apply fixtures from page (`specs/0113-fixture-sets-and-pins` criteria 33 to 37): the other
//! laser pages that have job fixtures, the preview of the picked page on this one, and the commit
//! with its "outside the work area" count. Never mirrored.

use curvyo_document_core::{Affine, NodeId, Page, Point, Vec2};
use curvyo_render_core::DrawList;
use serde::Serialize;

use super::super::Session;
use super::placed::PlacedFixture;

/// One page the Apply list offers (criterion 33).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApplyCandidate {
    /// The page's manufacturing, as its full id text: what a pick and Apply send back.
    pub id: String,
    /// The page's name.
    pub name: String,
    /// How many job fixtures it has ("4 job fixtures").
    pub count: usize,
}

/// `fixture` of another page placed at the same work-area position on `page`.
fn on_page(fixture: &PlacedFixture, page: &Page) -> PlacedFixture {
    let to_document = Affine::translation(Vec2::new(page.origin.x, page.origin.y));
    PlacedFixture {
        global: to_document.compose(fixture.matrix),
        ..fixture.clone()
    }
}

/// How far a copy may pass the work area's edge and still lie inside it, in mm.
const EPS: f64 = 1e-9;

/// Whether `fixture` lies partly outside the work area of `page` (criterion 36).
fn outside(fixture: &PlacedFixture, page: &Page) -> bool {
    let (w, h) = (page.size.width.as_mm(), page.size.height.as_mm());
    fixture.corners(page).iter().any(|c| {
        let p = Point::new(c.x - page.origin.x, c.y - page.origin.y);
        p.x < -EPS || p.y < -EPS || p.x > w + EPS || p.y > h + EPS
    })
}

/// The notice of `n` copies outside the work area, at most 70 characters.
fn outside_notice(n: usize) -> Option<String> {
    match n {
        0 => None,
        1 => Some("1 fixture lies outside the work area.".to_owned()),
        _ => Some(format!("{n} fixtures lie outside the work area.")),
    }
}

impl Session {
    /// The other laser pages with 1 to 64 job fixtures (ADR 0059 §8: Apply replaces this page's
    /// job fixtures, so only the source's count is limited), in page order, each with its count;
    /// empty hides the button (criteria 11, 17, 33).
    #[must_use]
    pub fn apply_fixture_candidates(&self) -> Vec<ApplyCandidate> {
        let Some(active) = self.active_fixtures() else {
            return Vec::new();
        };
        self.available_page_fixtures()
            .into_iter()
            .filter(|page| page.manufacturing != active.manufacturing)
            .filter_map(|page| {
                let count = page.job_fixtures().count();
                (1..=curvyo_document_core::MAX_JOB_FIXTURES)
                    .contains(&count)
                    .then(|| ApplyCandidate {
                        id: page.manufacturing.full_id(),
                        name: page.page.name.clone(),
                        count,
                    })
            })
            .collect()
    }

    /// The candidate named by its full id text.
    #[must_use]
    pub fn apply_candidate_named(&self, name: &str) -> Option<NodeId> {
        self.available_page_fixtures()
            .into_iter()
            .map(|page| page.manufacturing)
            .find(|id| id.full_id() == name)
    }

    /// The page whose job fixtures are previewed, while it is still a candidate.
    #[must_use]
    pub fn apply_preview_source(&self) -> Option<NodeId> {
        let source = self.fixtures.placement.apply?;
        self.apply_fixture_candidates()
            .iter()
            .any(|c| c.id == source.full_id())
            .then_some(source)
    }

    /// Picks the page `source` in the Apply list (criterion 34), or with `None` folds the list
    /// (Cancel, Escape): the preview follows. Not a step. `false` for a page that is no candidate.
    pub fn set_apply_preview(&mut self, source: Option<NodeId>) -> bool {
        if let Some(id) = source
            && !self
                .apply_fixture_candidates()
                .iter()
                .any(|c| c.id == id.full_id())
        {
            return false;
        }
        self.fixtures.placement.apply = source;
        true
    }

    /// Folds the Apply list (Cancel, Escape, a page switch): `true` when a page was picked.
    pub fn end_apply_preview(&mut self) -> bool {
        self.fixtures.placement.apply.take().is_some()
    }

    /// "Apply `<n>` fixtures" (criteria 35 to 37): replaces this page's job fixtures by copies of
    /// every job fixture of the page `source`, in one step "Apply fixtures from page", their jobs
    /// Output off, never mirrored; the preview ends and the copies are not selected (UX notes
    /// F7). A copy partly outside the work area is kept with the notice "`<n>` fixtures lie
    /// outside the work area.". Returns how many were copied; `None` when nothing was, also when
    /// the source holds more than 64 job fixtures (ADR 0059 §8: refused, no step).
    pub fn apply_fixtures_from_page(&mut self, source: NodeId) -> Option<usize> {
        let target = self.active_fixtures()?;
        if source == target.manufacturing {
            return None;
        }
        let copies = self.recording_unguarded(|this| {
            this.document
                .apply_page_fixtures(target.manufacturing, source)
                .ok()
        })?;
        self.fixtures.placement.apply = None;
        if copies.is_empty() {
            return None;
        }
        let page = self.active_fixtures()?;
        let n = copies
            .iter()
            .filter_map(|id| page.get(*id))
            .filter(|f| outside(f, &page.page))
            .count();
        self.fixtures.notice = outside_notice(n);
        Some(copies.len())
    }

    /// The job fixtures of the picked page drawn on this page in the dashed live preview, not
    /// hit (criterion 34, UX notes F2).
    pub(super) fn apply_preview_overlay(&self) -> DrawList {
        let mut list = DrawList::default();
        let (Some(source), Some(active)) = (self.apply_preview_source(), self.active_fixtures())
        else {
            return list;
        };
        let Some(page) = self
            .available_page_fixtures()
            .into_iter()
            .find(|p| p.manufacturing == source)
        else {
            return list;
        };
        for fixture in page.job_fixtures() {
            let copy = on_page(fixture, &active.page);
            list.extend(curvyo_render_core::build_fixture_preview(
                &copy.outline(&active.page),
                self.view(),
            ));
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::outside_notice;

    #[test]
    fn the_outside_notice_counts_its_fixtures_within_seventy_characters() {
        assert_eq!(outside_notice(0), None);
        assert_eq!(
            outside_notice(1).as_deref(),
            Some("1 fixture lies outside the work area.")
        );
        let many = outside_notice(64).unwrap();
        assert_eq!(many, "64 fixtures lie outside the work area.");
        assert!(many.chars().count() <= 70);
    }
}

//! The open Find centre lines bar in the session (`specs/0117-centre-lines` criteria 15, 15b and
//! 18, ADR 0063 amendment 2026-11-18, W and A): the subjects sent to the worker, the preview it
//! answers, and Apply of the worker's result in slices.

use curvyo_document_core::{DocumentVersion, NodeId, Point};
use curvyo_ui_core::{CentreLinesRefusal, CentreSubjects, centre_lines_drawing};
use std::collections::HashMap;

use super::Session;
use super::centre_lines::{CentreLinesOutcome, SubjectResult};
use crate::centre_lines_wire::{SubjectLines, decode_results, encode_subjects};

/// How many result anchors and width points one slice of Apply prepares (ADR 0063 amendment
/// 2026-11-18, A step 2): a fixed count, so the slices are the same on every run; about 5 ms
/// native per slice.
pub const CENTRE_APPLY_SLICE: usize = 20_000;

/// The open Find centre lines bar (`specs/0117-centre-lines` criteria 15 and 15b): the subjects
/// sent to the worker, the document and selection they were read from, and the shown preview.
#[derive(Debug)]
pub(super) struct CentreBar {
    version: DocumentVersion,
    selection: Vec<NodeId>,
    pub(super) found: CentreSubjects,
    /// The shown preview, from the worker.
    pub(super) preview: Option<Vec<SubjectLines>>,
    /// The preview is older than the values (a run is slow, UX notes §4): drawn at half strength.
    pub(super) stale: bool,
    /// Apply in progress: the worker's result and the drawings prepared so far.
    apply: Option<PendingApply>,
    /// The preview's tessellation, kept across frames until the preview changes.
    pub(super) mesh: super::trace_mesh::MeshCache,
}

/// The slices of an Apply: the result, and each subject's prepared drawing in order.
#[derive(Debug)]
struct PendingApply {
    results: Vec<SubjectLines>,
    /// Each subject's name, read in one pass at the start.
    names: Vec<Option<String>>,
    prepared: Vec<SubjectResult>,
}

impl std::fmt::Debug for SubjectResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubjectResult")
            .field("kept_regions", &self.kept_regions)
            .field("failed", &self.failed)
            .finish_non_exhaustive()
    }
}

/// The preview's counts for the Apply tooltip and the announcement (UX notes §3.3, §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CentrePreviewCounts {
    /// Centre paths.
    pub paths: usize,
    /// Their nodes.
    pub nodes: usize,
    /// Regions kept filled.
    pub kept: usize,
}

/// The counts of `results`.
fn counts_of(results: &[SubjectLines]) -> CentrePreviewCounts {
    let mut counts = CentrePreviewCounts::default();
    for result in results {
        match &result.lines {
            Some(lines) => {
                counts.paths += lines.paths.len();
                counts.nodes += lines.paths.iter().map(|p| p.anchors.len()).sum::<usize>();
                counts.kept += lines.filled.len();
            }
            None => counts.kept += 1,
        }
    }
    counts
}

/// The result anchors and width points of one subject, the measure of a slice.
fn weight_of(result: &SubjectLines) -> usize {
    result.lines.as_ref().map_or(1, |lines| {
        lines
            .paths
            .iter()
            .map(|p| p.anchors.len() + p.profile.positions().count())
            .sum::<usize>()
            + lines.filled.iter().flatten().map(Vec::len).sum::<usize>()
            + lines.inner_areas.iter().map(Vec::len).sum::<usize>()
            + 1
    })
}

impl Session {
    /// Opens the bar (criterion 15): the subjects' outlines as the worker's wire, or why the
    /// press is refused (nothing opens then). Opening again replaces the bar.
    ///
    /// # Errors
    /// The refusal of criteria 4 and 18.
    pub fn centre_lines_open(&mut self) -> Result<Vec<f64>, CentreLinesRefusal> {
        self.centre = None;
        if !self.centre_ready() || self.selection.is_empty() {
            return Err(CentreLinesRefusal::NotFilled);
        }
        self.flush_select_bar_preview();
        self.end_colour_pick();
        let found = self.centre_subjects()?;
        let wire = encode_subjects(
            &found
                .subjects
                .iter()
                .map(|s| s.outlines.clone())
                .collect::<Vec<_>>(),
        );
        self.centre = Some(CentreBar {
            version: self.document.version(),
            selection: self.selection.ids().to_vec(),
            found,
            preview: None,
            stale: false,
            apply: None,
            mesh: super::trace_mesh::MeshCache::default(),
        });
        Ok(wire)
    }

    /// Whether the bar is open.
    #[must_use]
    pub fn centre_lines_is_open(&self) -> bool {
        self.centre.is_some()
    }

    /// Whether the subjects sent are still those of the document and the selection; when not,
    /// the bar opens again to send them anew.
    #[must_use]
    pub fn centre_lines_current(&self) -> bool {
        self.centre.as_ref().is_some_and(|bar| {
            bar.version == self.document.version() && bar.selection == self.selection.ids()
        })
    }

    /// Shows the worker's preview `wire` (criterion 15b); its counts, or `None` when the bar is
    /// closed or the wire does not fit the subjects.
    pub fn centre_lines_show(&mut self, wire: &[f64]) -> Option<CentrePreviewCounts> {
        let bar = self.centre.as_mut()?;
        let results = decode_results(wire).ok()?;
        if results.len() != bar.found.subjects.len() {
            return None;
        }
        let counts = counts_of(&results);
        bar.preview = Some(results);
        bar.stale = false;
        bar.mesh = super::trace_mesh::MeshCache::default();
        Some(counts)
    }

    /// The shown preview is older than the values (`true`) or current again.
    pub fn centre_lines_set_stale(&mut self, stale: bool) {
        if let Some(bar) = self.centre.as_mut() {
            bar.stale = stale;
        }
    }

    /// Closes the bar without a write: Cancel, Escape, another tool, Undo or Redo; a pending
    /// Apply is dropped and nothing is written.
    pub fn centre_lines_close(&mut self) {
        self.centre = None;
    }

    /// A Node press while the bar is open: it does nothing on an object; on empty canvas it
    /// deselects, which closes the bar (UX notes §4). Whether the press was the bar's.
    pub(super) fn centre_lines_press(&mut self, point: Point) -> bool {
        if self.centre.is_none() {
            return false;
        }
        let pickable = self.pickable_objects();
        let hit = curvyo_ui_core::hit_test_objects(&pickable, point, self.object_tolerance());
        if hit.is_empty() {
            self.selection.clear();
            self.node.clear_selection();
            self.centre = None;
        }
        true
    }

    /// Starts Apply with the worker's result `wire` (criterion 2): the slices prepare the
    /// drawings off the document; the number of slices, or why nothing will be written.
    ///
    /// # Errors
    /// `CentreLinesOutcome::Stale` when the document or selection changed or the wire does
    /// not fit; `CentreLinesOutcome::Ignored` when the bar is closed.
    pub fn centre_lines_apply_begin(&mut self, wire: &[f64]) -> Result<usize, CentreLinesOutcome> {
        if !self.centre_lines_current() {
            return Err(if self.centre.is_some() {
                CentreLinesOutcome::Stale
            } else {
                CentreLinesOutcome::Ignored
            });
        }
        let Some(bar) = self.centre.as_mut() else {
            return Err(CentreLinesOutcome::Ignored);
        };
        let results = decode_results(wire).map_err(|_| CentreLinesOutcome::Stale)?;
        if results.len() != bar.found.subjects.len() {
            return Err(CentreLinesOutcome::Stale);
        }
        let total: usize = results.iter().map(weight_of).sum();
        let index: HashMap<NodeId, usize> = bar
            .found
            .subjects
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();
        let mut names = vec![None; results.len()];
        for node in self.document_read().nodes.iter() {
            if let Some(&i) = index.get(&node.id) {
                names[i].clone_from(&node.registers.name);
            }
        }
        let Some(bar) = self.centre.as_mut() else {
            return Err(CentreLinesOutcome::Ignored);
        };
        bar.apply = Some(PendingApply {
            results,
            names,
            prepared: Vec::new(),
        });
        Ok(total.div_ceil(CENTRE_APPLY_SLICE).max(1) + 1)
    }

    /// The share of Apply done, 0 to 1.
    #[must_use]
    pub fn centre_lines_apply_progress(&self) -> f64 {
        self.centre
            .as_ref()
            .and_then(|bar| bar.apply.as_ref())
            .map_or(0.0, |apply| {
                #[allow(clippy::cast_precision_loss)]
                let share = apply.prepared.len() as f64 / apply.results.len().max(1) as f64;
                share
            })
    }

    /// One slice of Apply: prepares the next subjects' drawings, about
    /// 20,000 result nodes; once all are prepared, writes them in one commit
    /// and closes the bar. `None` while slices remain; the outcome once done.
    ///
    /// An edit between two slices (a peer's, or a key) makes the result stale: the slices are
    /// dropped, nothing is written and the bar stays open to send the subjects anew.
    pub fn centre_lines_apply_step(&mut self) -> Option<CentreLinesOutcome> {
        let current = self.centre_lines_current();
        let Some(bar) = self.centre.as_mut() else {
            return Some(CentreLinesOutcome::Ignored);
        };
        if bar.apply.is_some() && !current {
            bar.apply = None;
            return Some(CentreLinesOutcome::Stale);
        }
        let Some(apply) = bar.apply.as_mut() else {
            return Some(CentreLinesOutcome::Ignored);
        };
        if apply.prepared.len() < apply.results.len() {
            let mut weight = 0;
            while apply.prepared.len() < apply.results.len() && weight < CENTRE_APPLY_SLICE {
                let index = apply.prepared.len();
                let result = &apply.results[index];
                weight += weight_of(result);
                let subject = &bar.found.subjects[index];
                let name = apply.names[index].clone();
                apply.prepared.push(match &result.lines {
                    None => SubjectResult {
                        drawing: None,
                        kept_regions: 1,
                        failed: 1,
                    },
                    Some(lines) => SubjectResult {
                        drawing: centre_lines_drawing(subject, lines, name.as_deref()),
                        kept_regions: lines.filled.len(),
                        failed: lines.failed,
                    },
                });
            }
            return None;
        }
        let Some(bar) = self.centre.take() else {
            return Some(CentreLinesOutcome::Ignored);
        };
        let prepared = bar.apply.map(|a| a.prepared).unwrap_or_default();
        Some(self.recording(|this| this.write_results(&bar.found, prepared)))
    }
}

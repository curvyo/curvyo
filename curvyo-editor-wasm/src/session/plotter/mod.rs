//! The session's plotter pages (`specs/0159-cutting-plotter` slice 1, ADR 0097): which output page
//! of process `curvyo.cutter` is active, its copy, mat and operations as read, and the maker's
//! cutter profiles. Reading only; the commands live in the submodules.

mod commands;
mod draw;
mod export;
mod job;
mod operations;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_cutter_core::{
    CutterCopy, CutterProfile, MatRead, MatSize, OpKind, PROCESS_ID, read_copy, read_record,
};
use curvyo_document_core::{ManufacturingSnapshot, NodeId, OperationSnapshot, Page};

use super::{Session, Workbench};

/// The maker's cutter profiles (records of kind `cutter` in `machines/`, `adrs.md` PL2).
#[derive(Debug, Default)]
pub(in crate::session) struct PlotterLibrary {
    /// The user profiles that read, in file order.
    pub(in crate::session) profiles: Vec<CutterProfile>,
}

impl PlotterLibrary {
    /// Reads every record of kind `cutter` of the host's `(file name, text)` pairs; another
    /// family's record and a damaged one are skipped (the laser reader counts its own).
    pub(in crate::session) fn load(entries: &[(String, String)]) -> Self {
        Self {
            profiles: entries
                .iter()
                .filter_map(|(name, text)| read_record(name, text).ok())
                .collect(),
        }
    }

    /// The user profile `id`.
    pub(in crate::session) fn find(&self, id: &str) -> Option<&CutterProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Adds or replaces the profile of its id.
    pub(in crate::session) fn put(&mut self, profile: CutterProfile) {
        match self.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(slot) => *slot = profile,
            None => self.profiles.push(profile),
        }
    }
}

/// The active plotter page as read.
pub(in crate::session) struct PlotterPage {
    /// The manufacturing node.
    pub(in crate::session) id: NodeId,
    /// Its page.
    pub(in crate::session) page: Page,
    /// The profile copy.
    pub(in crate::session) copy: CutterCopy,
    /// How many values of the copy were damaged.
    pub(in crate::session) copy_damaged: usize,
    /// The mat fixtures of the page by size, in fixture order.
    pub(in crate::session) mats: Vec<(NodeId, MatSize)>,
    /// The active mat, `None` on a matless profile.
    pub(in crate::session) mat: Option<(NodeId, MatRead)>,
    /// The operations in list order.
    pub(in crate::session) operations: Vec<OperationSnapshot>,
}

impl PlotterPage {
    /// The kind of operation `op`, `None` for a kind the plotter does not know.
    pub(in crate::session) fn kind_of(op: &OperationSnapshot) -> Option<OpKind> {
        op.kind.as_deref().and_then(OpKind::parse)
    }

    /// The machine's area on the page: the active mat's cut area, else the whole page.
    pub(in crate::session) fn cut_area(&self) -> curvyo_cutter_core::Rect {
        match (&self.mat, self.copy.values.mats.is_empty()) {
            (Some((_, mat)), false) => mat.size.cut_area(),
            _ => curvyo_cutter_core::Rect::new(
                0.0,
                0.0,
                self.page.size.width.as_mm(),
                self.page.size.height.as_mm(),
            ),
        }
    }

    /// The active mat when the profile offers mats.
    pub(in crate::session) fn active_mat(&self) -> Option<MatRead> {
        if self.copy.values.mats.is_empty() {
            return None;
        }
        self.mat.map(|(_, mat)| mat)
    }
}

/// The plotter page of `manufacturing`, `None` for another process or one without its page.
pub(in crate::session) fn plotter_page(
    manufacturing: &ManufacturingSnapshot,
) -> Option<PlotterPage> {
    if manufacturing.process != PROCESS_ID {
        return None;
    }
    let page = manufacturing.page.clone()?;
    let read = read_copy(manufacturing.profile.as_ref());
    let mats: Vec<(NodeId, MatSize)> = manufacturing
        .fixtures
        .iter()
        .filter(|f| MatSize::is_mat_kind(&f.kind))
        .map(|f| (f.id, MatRead::of(&f.kind, &f.params).size))
        .collect();
    let mat = manufacturing
        .fixtures
        .iter()
        .filter(|f| MatSize::is_mat_kind(&f.kind))
        .find(|f| f.active)
        .map(|f| (f.id, MatRead::of(&f.kind, &f.params)));
    Some(PlotterPage {
        id: manufacturing.id,
        page,
        copy: read.copy,
        copy_damaged: read.damaged,
        mats,
        mat,
        operations: Vec::new(),
    })
}

impl Session {
    /// The active page when it is a plotter page with its extension on.
    pub(in crate::session) fn active_plotter(&self) -> Option<PlotterPage> {
        let Workbench::Manufacturing(id) = self.workbench() else {
            return None;
        };
        let manufacturing = self.document.manufacturing(id)?;
        if !self.process_enabled(&manufacturing.process) {
            return None;
        }
        let mut page = plotter_page(&manufacturing)?;
        page.operations = self.document.operations(id).unwrap_or_default();
        Some(page)
    }

    /// The machine name of the plotter page `manufacturing`, for its page row and status.
    pub(in crate::session) fn plotter_machine_name(
        manufacturing: &ManufacturingSnapshot,
    ) -> Option<String> {
        plotter_page(manufacturing).map(|p| p.copy.values.name)
    }
}

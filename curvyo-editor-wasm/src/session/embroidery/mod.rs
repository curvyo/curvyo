//! The session's embroidery pages (`specs/0160-embroidery` slice 1, ADR 0097): which output page
//! of process `curvyo.embroidery` is active, its copy, hoop and operations as read, the maker's
//! embroidery profiles and the simulation's position. Reading only; the commands live in the
//! submodules.

mod commands;
mod draw;
mod export;
mod job;
mod operations;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_document_core::{ManufacturingSnapshot, NodeId, OperationSnapshot, Page, Point};
use curvyo_embroidery_core::{
    EmbroideryCopy, EmbroideryProfile, HoopSize, OpKind, PROCESS_ID, read_copy, read_record,
};

use super::{Session, Workbench};

/// The maker's embroidery profiles (records of kind `embroidery` in `machines/`) and the
/// simulation's position (session state, criterion 13).
#[derive(Debug, Default)]
pub(in crate::session) struct EmbroideryState {
    /// The user profiles that read, in file order.
    pub(in crate::session) profiles: Vec<EmbroideryProfile>,
    /// The stitches the Toolpath view shows sewn; `None` shows all.
    pub(in crate::session) play: Option<usize>,
}

impl EmbroideryState {
    /// Reads every record of kind `embroidery` of the host's `(file name, text)` pairs; another
    /// family's record and a damaged one are skipped (the laser reader counts its own).
    pub(in crate::session) fn load(entries: &[(String, String)]) -> Vec<EmbroideryProfile> {
        entries
            .iter()
            .filter_map(|(name, text)| read_record(name, text).ok())
            .collect()
    }

    /// The user profile `id`.
    pub(in crate::session) fn find(&self, id: &str) -> Option<&EmbroideryProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Adds or replaces the profile of its id.
    pub(in crate::session) fn put(&mut self, profile: EmbroideryProfile) {
        match self.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(slot) => *slot = profile,
            None => self.profiles.push(profile),
        }
    }
}

/// The active hoop of a page as read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct HoopRead {
    /// Its fixture.
    pub(in crate::session) id: Option<NodeId>,
    /// Its size; an unknown kind reads as the profile's first hoop.
    pub(in crate::session) size: HoopSize,
    /// Whether the stored kind was unknown.
    pub(in crate::session) damaged: bool,
}

/// The active embroidery page as read.
pub(in crate::session) struct EmbroideryPage {
    /// The manufacturing node.
    pub(in crate::session) id: NodeId,
    /// Its page.
    pub(in crate::session) page: Page,
    /// The profile copy.
    pub(in crate::session) copy: EmbroideryCopy,
    /// How many values of the copy were damaged.
    pub(in crate::session) copy_damaged: usize,
    /// The hoop fixtures of the page by size, in fixture order (unknown kinds left out).
    pub(in crate::session) hoops: Vec<(NodeId, HoopSize)>,
    /// The active hoop.
    pub(in crate::session) hoop: HoopRead,
    /// The operations in list order.
    pub(in crate::session) operations: Vec<OperationSnapshot>,
}

impl EmbroideryPage {
    /// The kind of operation `op`, `None` for a kind embroidery does not know.
    pub(in crate::session) fn kind_of(op: &OperationSnapshot) -> Option<OpKind> {
        op.kind.as_deref().and_then(OpKind::parse)
    }

    /// The hoop centre in document mm: the page centre (`adrs.md` EM10).
    pub(in crate::session) fn centre(&self) -> Point {
        Point::new(
            self.page.origin.x + self.page.size.width.as_mm() / 2.0,
            self.page.origin.y + self.page.size.height.as_mm() / 2.0,
        )
    }

    /// The hoop centre in page mm, the frame of the stitch plan's items.
    pub(in crate::session) fn page_centre(&self) -> Point {
        Point::new(
            self.page.size.width.as_mm() / 2.0,
            self.page.size.height.as_mm() / 2.0,
        )
    }
}

/// The embroidery page of `manufacturing`, `None` for another process or one without its page.
pub(in crate::session) fn embroidery_page(
    manufacturing: &ManufacturingSnapshot,
) -> Option<EmbroideryPage> {
    if manufacturing.process != PROCESS_ID {
        return None;
    }
    let page = manufacturing.page.clone()?;
    let read = read_copy(manufacturing.profile.as_ref());
    let fixtures: Vec<_> = manufacturing
        .fixtures
        .iter()
        .filter(|f| HoopSize::is_hoop_kind(&f.kind))
        .collect();
    let hoops = fixtures
        .iter()
        .filter_map(|f| HoopSize::of_fixture_kind(&f.kind).map(|size| (f.id, size)))
        .collect();
    let default = read.copy.values.default_hoop();
    let hoop = fixtures.iter().find(|f| f.active).map_or(
        HoopRead {
            id: None,
            size: default,
            damaged: false,
        },
        |f| match HoopSize::of_fixture_kind(&f.kind) {
            Some(size) => HoopRead {
                id: Some(f.id),
                size,
                damaged: false,
            },
            None => HoopRead {
                id: Some(f.id),
                size: default,
                damaged: true,
            },
        },
    );
    Some(EmbroideryPage {
        id: manufacturing.id,
        page,
        copy: read.copy,
        copy_damaged: read.damaged,
        hoops,
        hoop,
        operations: Vec::new(),
    })
}

impl Session {
    /// The active page when it is an embroidery page with its extension on.
    pub(in crate::session) fn active_embroidery(&self) -> Option<EmbroideryPage> {
        let Workbench::Manufacturing(id) = self.workbench() else {
            return None;
        };
        let manufacturing = self.document.manufacturing(id)?;
        if !self.process_enabled(&manufacturing.process) {
            return None;
        }
        let mut page = embroidery_page(&manufacturing)?;
        page.operations = self.document.operations(id).unwrap_or_default();
        Some(page)
    }

    /// The machine name of the embroidery page `manufacturing`, for its page row and status.
    pub(in crate::session) fn embroidery_machine_name(
        manufacturing: &ManufacturingSnapshot,
    ) -> Option<String> {
        embroidery_page(manufacturing).map(|p| p.copy.values.name)
    }
}

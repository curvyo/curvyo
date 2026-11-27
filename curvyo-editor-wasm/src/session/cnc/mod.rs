//! The session's CNC pages (`specs/0161-cnc-output` slice 1, ADR 0097): which output page of
//! process `curvyo.cnc` is active, its copy, stock, tools and operations as read, the maker's CNC
//! profiles, the play position and the send in flight. Reading only; the commands live in the
//! submodules.

mod commands;
mod draw;
mod export;
mod job;
mod operations;
#[cfg(test)]
mod tests;
mod view_json;

use curvyo_cnc_core::{
    CncCopy, CncProfile, PROCESS_ID, PROFILE_KIND, STOCK_KIND, Stock, TOOLS_FIELD, Tool, read_copy,
    read_record, read_tools,
};
use curvyo_document_core::{ManufacturingSnapshot, NodeId, OperationSnapshot, Page};

use super::{Session, Workbench};

/// The maker's CNC profiles (records of kind `cnc` in `machines/`), the play position and the
/// send in flight (session state).
#[derive(Debug, Default)]
pub(in crate::session) struct CncState {
    /// The user profiles that read, in file order.
    pub(in crate::session) profiles: Vec<CncProfile>,
    /// The play position in seconds of the planner's time; `None` shows the whole job.
    pub(in crate::session) play: Option<f64>,
    /// The token of the next send.
    pub(in crate::session) next_send: u32,
}

impl CncState {
    /// Reads every record of kind `cnc` of the host's `(file name, text)` pairs; another
    /// family's record and a damaged one are skipped.
    pub(in crate::session) fn load(entries: &[(String, String)]) -> Vec<CncProfile> {
        entries
            .iter()
            .filter_map(|(name, text)| read_record(name, text).ok())
            .collect()
    }

    /// The user profile `id`.
    pub(in crate::session) fn find(&self, id: &str) -> Option<&CncProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Adds or replaces the profile of its id.
    pub(in crate::session) fn put(&mut self, profile: CncProfile) {
        match self.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(slot) => *slot = profile,
            None => self.profiles.push(profile),
        }
    }
}

/// The active CNC page as read.
pub(in crate::session) struct CncPage {
    /// The manufacturing node.
    pub(in crate::session) id: NodeId,
    /// Its page.
    pub(in crate::session) page: Page,
    /// The profile copy.
    pub(in crate::session) copy: CncCopy,
    /// The stock fixture, `None` when it is missing (the default stock reads then).
    pub(in crate::session) stock_id: Option<NodeId>,
    /// The stock.
    pub(in crate::session) stock: Stock,
    /// The tools, never empty.
    pub(in crate::session) tools: Vec<Tool>,
    /// How many stored values of the copy, the stock and the tools were damaged.
    pub(in crate::session) damaged: usize,
    /// The operations in list order.
    pub(in crate::session) operations: Vec<OperationSnapshot>,
}

impl CncPage {
    /// Whether `op` is a Profile operation.
    pub(in crate::session) fn is_profile(op: &OperationSnapshot) -> bool {
        op.kind.as_deref() == Some(PROFILE_KIND)
    }

    /// The tool `id`, else the first.
    pub(in crate::session) fn tool(&self, id: &str) -> &Tool {
        self.tools
            .iter()
            .find(|t| t.id == id)
            .unwrap_or(&self.tools[0])
    }
}

/// The register key of the tool list.
pub(in crate::session) fn tools_key() -> String {
    format!("x.{PROCESS_ID}.{TOOLS_FIELD}")
}

/// The CNC page of `manufacturing`, `None` for another process or one without its page.
pub(in crate::session) fn cnc_page(manufacturing: &ManufacturingSnapshot) -> Option<CncPage> {
    if manufacturing.process != PROCESS_ID {
        return None;
    }
    let page = manufacturing.page.clone()?;
    let read = read_copy(manufacturing.profile.as_ref());
    let fixture = manufacturing.fixtures.iter().find(|f| f.kind == STOCK_KIND);
    let (stock, stock_damaged) = fixture.map_or((Stock::default(), 0), |f| {
        Stock::read(&f.params, &read.copy.values)
    });
    let (tools, tools_damaged) = read_tools(
        manufacturing.process_values.get(&tools_key()),
        read.copy.values.spindle_max,
    );
    Some(CncPage {
        id: manufacturing.id,
        page,
        damaged: read.damaged + stock_damaged + tools_damaged,
        copy: read.copy,
        stock_id: fixture.map(|f| f.id),
        stock,
        tools,
        operations: Vec::new(),
    })
}

impl Session {
    /// The active page when it is a CNC page with its extension on.
    pub(in crate::session) fn active_cnc(&self) -> Option<CncPage> {
        let Workbench::Manufacturing(id) = self.workbench() else {
            return None;
        };
        let manufacturing = self.document.manufacturing(id)?;
        if !self.process_enabled(&manufacturing.process) {
            return None;
        }
        let mut page = cnc_page(&manufacturing)?;
        page.operations = self.document.operations(id).unwrap_or_default();
        Some(page)
    }

    /// The machine name of the CNC page `manufacturing`, for its page row and status.
    pub(in crate::session) fn cnc_machine_name(
        manufacturing: &ManufacturingSnapshot,
    ) -> Option<String> {
        cnc_page(manufacturing).map(|p| p.copy.values.name)
    }
}

//! The commands of a CNC page (`specs/0161-cnc-output` criteria 1 to 4, UX 3 to 5): Add page,
//! Pick and Save as profile, the connector address, the stock and the tool list, each one history
//! step (the connector address is a record edit, not a document step).

use curvyo_cnc_core::{
    ADD_PAGE_COMMIT_LABEL, BUILTINS, CncCopy, CncProfile, DEFAULT_BUILTIN, PROCESS_ID, STOCK_KIND,
    Stock, TOOLS_FIELD, Tool, WORD, builtin, write_copy, write_record, write_tools,
};
use curvyo_document_core::{
    FixtureAdd, Length, NewFixture, NewOutputPage, NodeId, PageCorner, ProfileChange,
};
use curvyo_laser_core::{LibraryWrite, new_record_id};

use super::super::Session;
use super::CncPage;

/// The step of a stock field.
pub(in crate::session) const EDIT_STOCK_LABEL: &str = "edit_stock";
/// The step of + Tool.
pub(in crate::session) const ADD_TOOL_LABEL: &str = "add_tool";
/// The step of a tool field.
pub(in crate::session) const EDIT_TOOL_LABEL: &str = "edit_tool";
/// The step of a tool's delete.
pub(in crate::session) const REMOVE_TOOL_LABEL: &str = "remove_tool";
/// The step of Pick profile.
const PICK_PROFILE_LABEL: &str = "pick_profile";

/// The page keeps its top-left corner: the stock's corner at the page origin (UX N13).
const TOP_LEFT: PageCorner = PageCorner {
    right: false,
    bottom: false,
};

/// The default stock fitted into a machine's work area.
fn fitted_stock(profile: &CncProfile) -> Stock {
    let mut stock = Stock::default();
    let v = &profile.values;
    let fit = |l: Length, max: Length| Length::from_mm(l.as_mm().min(max.as_mm()));
    stock.width = fit(stock.width, v.work_area.width);
    stock.height = fit(stock.height, v.work_area.height);
    stock.thickness = fit(stock.thickness, v.z_travel);
    stock
}

/// Adds the stock fixture of `stock` to `manufacturing`.
fn add_stock(
    d: &curvyo_document_core::Document,
    manufacturing: NodeId,
    stock: &Stock,
) -> Option<NodeId> {
    d.add_fixture(
        manufacturing,
        &NewFixture {
            add: FixtureAdd::Holder,
            kind: STOCK_KIND,
            label: "Stock",
            params: &stock.params(),
            matrix: None,
        },
    )
    .ok()
}

impl Session {
    /// Every CNC profile the pick lists: the built-ins, then the maker's (criterion 2).
    pub(in crate::session) fn cnc_profiles(&self) -> Vec<CncProfile> {
        let mut out: Vec<CncProfile> = BUILTINS
            .iter()
            .map(curvyo_cnc_core::Builtin::profile)
            .collect();
        out.extend(self.manufacturing.cnc.profiles.iter().cloned());
        out
    }

    pub(in crate::session) fn cnc_profile(&self, id: &str) -> Option<CncProfile> {
        builtin(id).or_else(|| self.manufacturing.cnc.find(id).cloned())
    }

    /// Add page with CNC (criterion 1): "CNC N" with the copy of `profile` (else the Generic GRBL
    /// router), its page the work area, and the default stock, in one step.
    pub(in crate::session) fn add_cnc_page(&mut self, profile: Option<&str>) -> Option<String> {
        let profile = profile
            .and_then(|id| self.cnc_profile(id))
            .unwrap_or_else(|| DEFAULT_BUILTIN.profile());
        let copy = write_copy(&CncCopy::of(&profile));
        let stock = fitted_stock(&profile);
        let page = self.recording_unguarded(|this| {
            this.document.batch(ADD_PAGE_COMMIT_LABEL, |d| {
                let page = d
                    .add_output_page(NewOutputPage {
                        process: PROCESS_ID,
                        label: WORD,
                        profile: Some(&copy),
                        work_area: profile.values.work_area,
                    })
                    .ok()?;
                add_stock(d, page.kind.manufacturing()?, &stock)?;
                Some(page)
            })
        })?;
        self.activate_page(&page.id);
        Some(page.id)
    }

    fn write_cnc_copy(
        &mut self,
        active: &CncPage,
        copy: &CncCopy,
        label: &str,
        change: ProfileChange,
    ) -> bool {
        let map = write_copy(copy);
        let contents = self.page_contents().unwrap_or_default();
        let size = copy.values.work_area;
        self.recording_unguarded(|this| {
            this.document.batch(label, |d| {
                d.set_manufacturing_profile(active.id, &map, size, TOP_LEFT, change, &contents)
                    .unwrap_or(false)
            })
        })
    }

    /// Pick profile on the active CNC page (criterion 2): the copy of `id`, the page its work
    /// area, in one step. `false` for another page, an unknown id or no change.
    pub fn pick_cnc_profile(&mut self, id: &str) -> bool {
        let (Some(active), Some(profile)) = (self.active_cnc(), self.cnc_profile(id)) else {
            return false;
        };
        self.write_cnc_copy(
            &active,
            &CncCopy::of(&profile),
            PICK_PROFILE_LABEL,
            ProfileChange::Pick,
        )
    }

    /// Save as profile (criterion 2): the copy becomes a new user record of kind `cnc`, synced as
    /// a laser profile, and the copy points at it in one step. The new id.
    pub fn save_cnc_profile(&mut self, random: [u8; 16], now_ms: i64) -> Option<String> {
        let active = self.active_cnc()?;
        let profile = CncProfile {
            id: new_record_id(random),
            modified: now_ms,
            values: active.copy.values.clone(),
        };
        let text = write_record(&profile);
        let id = profile.id.clone();
        let contents = self.page_contents().unwrap_or_default();
        let map = write_copy(&CncCopy::of(&profile));
        self.recording_unguarded(|this| {
            this.document
                .set_manufacturing_profile(
                    active.id,
                    &map,
                    active.page.size,
                    TOP_LEFT,
                    ProfileChange::SaveAs,
                    &contents,
                )
                .unwrap_or(false)
        });
        self.manufacturing.cnc.put(profile);
        self.manufacturing.machine.queue(LibraryWrite::Write {
            id: id.clone(),
            text,
        });
        Some(id)
    }

    /// The user profile record the active CNC page's copy came from.
    pub(in crate::session) fn cnc_record(&self) -> Option<CncProfile> {
        let id = self.active_cnc()?.copy.id?;
        self.manufacturing.cnc.find(&id).cloned()
    }

    /// A stock field (criterion 3, UX 4): `width_mm`, `height_mm`, `thickness_mm` or `material`
    /// from the typed `text`, one step "Edit stock".
    ///
    /// # Errors
    /// The field message with the limit for a value out of its range.
    pub fn edit_cnc_stock(&mut self, field: &str, text: &str) -> Result<bool, String> {
        let active = self.active_cnc().ok_or_else(String::new)?;
        let stock = active.stock.edit(field, text, &active.copy.values)?;
        Ok(self.recording_unguarded(|this| {
            this.document
                .batch(EDIT_STOCK_LABEL, |d| match active.stock_id {
                    Some(id) => d.set_fixture_params(id, &stock.params()).unwrap_or(false),
                    None => add_stock(d, active.id, &stock).is_some(),
                })
        }))
    }

    fn write_cnc_tools(&mut self, active: &CncPage, tools: &[Tool], label: &str) -> bool {
        let value = write_tools(tools);
        self.recording_unguarded(|this| {
            this.document
                .set_process_register(active.id, TOOLS_FIELD, Some(&value), label)
                .unwrap_or(false)
        })
    }

    /// The + Tool button (criterion 4, UX 5): a flat 3.175 mm end mill with the defaults, its id from
    /// `random`, one step "Add tool". The new id.
    pub fn add_cnc_tool(&mut self, random: [u8; 16]) -> Option<String> {
        let active = self.active_cnc()?;
        let tool = Tool::default_with_id(&new_record_id(random));
        let id = tool.id.clone();
        let mut tools = active.tools.clone();
        tools.push(tool);
        self.write_cnc_tools(&active, &tools, ADD_TOOL_LABEL)
            .then_some(id)
    }

    /// A tool field (criterion 4): one step "Edit tool".
    ///
    /// # Errors
    /// The field message for a value out of its range.
    pub fn edit_cnc_tool(&mut self, id: &str, field: &str, text: &str) -> Result<bool, String> {
        let active = self.active_cnc().ok_or_else(String::new)?;
        let mut tools = active.tools.clone();
        let slot = tools
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(String::new)?;
        *slot = slot.edit(field, text, active.copy.values.spindle_max)?;
        Ok(self.write_cnc_tools(&active, &tools, EDIT_TOOL_LABEL))
    }

    /// Deletes tool `id` (UX 5, N3): the operations that used it move to the first remaining
    /// tool, one step "Remove tool"; the last tool stays.
    pub fn remove_cnc_tool(&mut self, id: &str) -> bool {
        let Some(active) = self.active_cnc() else {
            return false;
        };
        if active.tools.len() < 2 || !active.tools.iter().any(|t| t.id == id) {
            return false;
        }
        let tools: Vec<Tool> = active
            .tools
            .iter()
            .filter(|t| t.id != id)
            .cloned()
            .collect();
        let first = tools[0].id.clone();
        let moved: Vec<(NodeId, serde_json::Map<String, serde_json::Value>)> = active
            .operations
            .iter()
            .filter(|op| CncPage::is_profile(op))
            .filter_map(|op| {
                let (mut s, _) = active.settings_of(op);
                (s.tool == id).then(|| {
                    s.tool.clone_from(&first);
                    (op.id, s.options())
                })
            })
            .collect();
        let value = write_tools(&tools);
        self.recording_unguarded(|this| {
            this.document.batch(REMOVE_TOOL_LABEL, |d| {
                for (op, options) in &moved {
                    let _ = d.write_operation_options(*op, options);
                }
                d.set_process_register(active.id, TOOLS_FIELD, Some(&value), REMOVE_TOOL_LABEL)
                    .unwrap_or(false)
            })
        })
    }
}

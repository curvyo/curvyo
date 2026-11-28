//! The commands of a CNC page (`specs/0161-cnc-output` criteria 1 to 4, UX 3 to 5): Add page,
//! Pick and Save as profile, the connector address, the machine fields, the stock and the tool
//! list, each one history step (the connector address is a record edit, not a document step).

use curvyo_cnc_core::{
    ADD_PAGE_COMMIT_LABEL, BUILTINS, CncCopy, CncProfile, DEFAULT_BUILTIN, PROCESS_ID, Rpm,
    STOCK_KIND, Stock, TOOLS_FIELD, Tool, WORD, builtin, write_copy, write_record, write_tools,
};
use curvyo_document_core::{
    FixtureAdd, Length, NewFixture, NewOutputPage, NodeId, PageCorner, ProfileChange,
};
use curvyo_laser_core::{LibraryWrite, MAX_CONNECTION_CHARS, new_record_id};

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
/// The steps of the machine fields (UX 3).
pub(in crate::session) const EDIT_Z_TRAVEL_LABEL: &str = "edit_z_travel";
/// The step of Spindle max.
pub(in crate::session) const EDIT_SPINDLE_MAX_LABEL: &str = "edit_spindle_max";
/// The step of Safe height.
pub(in crate::session) const EDIT_SAFE_HEIGHT_LABEL: &str = "edit_safe_height";
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

    /// The connector address of the active page's user profile (criterion 19, `0064`): written
    /// to the record at `now_ms` and synced; no document step.
    ///
    /// # Errors
    /// "Save as profile first." on a built-in; the length message beyond 200 characters.
    pub fn set_cnc_connector(&mut self, address: &str, now_ms: i64) -> Result<bool, String> {
        let mut record = self.cnc_record().ok_or("Save as profile first.")?;
        let address = address.trim();
        if address.chars().count() > MAX_CONNECTION_CHARS {
            return Err(format!(
                "The address is at most {MAX_CONNECTION_CHARS} characters."
            ));
        }
        if record.values.connection.connector_address == address {
            return Ok(false);
        }
        address.clone_into(&mut record.values.connection.connector_address);
        record.modified = now_ms;
        let (id, text) = (record.id.clone(), write_record(&record));
        self.manufacturing.cnc.put(record);
        self.manufacturing
            .machine
            .queue(LibraryWrite::Write { id, text });
        Ok(true)
    }

    /// A machine field of the copy (UX 3): `z_travel_mm`, `spindle_max_rpm` or `safe_height_mm`
    /// from the typed `text`, one step "Edit Z travel", "Edit spindle max" or "Edit safe height".
    ///
    /// # Errors
    /// The field message for a value out of its range; empty for an unknown field.
    pub fn set_cnc_machine_value(&mut self, field: &str, text: &str) -> Result<bool, String> {
        let active = self.active_cnc().ok_or_else(String::new)?;
        let mut copy = active.copy.clone();
        let v = &mut copy.values;
        let number = text.trim().replace(',', "").parse::<f64>().ok();
        let label = match field {
            "z_travel_mm" => {
                let z = number
                    .filter(|z| (active.stock.thickness.as_mm().max(1.0)..=1_000.0).contains(z))
                    .ok_or_else(|| {
                        format!(
                            "Z travel is {} to 1,000 mm.",
                            active.stock.thickness.as_mm().max(1.0)
                        )
                    })?;
                v.z_travel = Length::from_mm(z);
                if v.safe_height.as_mm() > z {
                    v.safe_height = Length::from_mm(z);
                }
                EDIT_Z_TRAVEL_LABEL
            }
            "spindle_max_rpm" => {
                v.spindle_max = number
                    .filter(|n| n.fract() == 0.0 && *n >= 0.0)
                    .and_then(|n| {
                        // invariant: a whole, non-negative number checked above
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let n = n as u64;
                        Rpm::new(n)
                    })
                    .ok_or("Spindle max is 0 to 100,000 rpm.")?;
                EDIT_SPINDLE_MAX_LABEL
            }
            "safe_height_mm" => {
                let max = v.z_travel.as_mm();
                v.safe_height = number
                    .filter(|h| (0.5..=max).contains(h))
                    .map(Length::from_mm)
                    .ok_or_else(|| format!("Safe height is 0.5 to {max} mm."))?;
                EDIT_SAFE_HEIGHT_LABEL
            }
            _ => return Err(String::new()),
        };
        Ok(self.write_cnc_copy(&active, &copy, label, ProfileChange::EditName))
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

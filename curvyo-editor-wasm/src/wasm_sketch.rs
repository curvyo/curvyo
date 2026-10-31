//! Thin host bridge forwards sketch workbench actions to the Rust session.
use crate::{SketchTool, Workbench, wasm_api::WasmSession};
use curvyo_sketch_core::{Dimension, Length};
use wasm_bindgen::prelude::*;
fn error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
#[wasm_bindgen]
impl WasmSession {
    /// Cancels the current analytic gesture without creating geometry.
    pub fn sketch_secondary_press(&mut self) -> bool {
        self.session.sketch_secondary_press()
    }

    /// Complete read-only Properties and canvas annotation view.
    pub fn sketch_view_json(&self) -> String {
        self.session.sketch_view_json()
    }
    /// Switches the existing shell's workbench without a document write.
    /// # Errors
    /// Refuses unknown workbench names.
    pub fn set_workbench(&mut self, name: &str) -> Result<(), JsValue> {
        let workbench = match name {
            "vector" => Workbench::Vector,
            "sketch" => Workbench::Sketch,
            _ => return Err(error("Unknown workbench")),
        };
        self.session.set_workbench(workbench);
        Ok(())
    }
    /// Creates one empty analytic sketch with its origin.
    /// # Errors
    /// Reports invalid context or document errors.
    pub fn new_sketch(&mut self) -> Result<String, JsValue> {
        self.session
            .new_sketch()
            .map(curvyo_document_core::NodeId::full_id)
            .map_err(error)
    }
    /// Re-enters the sketch chosen in Properties.
    pub fn enter_sketch(&mut self, id: &str) -> bool {
        self.session.enter_sketch_named(id)
    }
    /// Selects a constraint tool or context-sensitive dimensioning.
    /// # Errors
    /// Rejects unknown action names.
    pub fn set_sketch_constraint_tool(&mut self, name: &str) -> Result<(), JsValue> {
        if self.session.set_sketch_constraint_tool(name) {
            Ok(())
        } else {
            Err(error("Unknown constraint tool"))
        }
    }
    /// Presses a Constraints rail tool: applies, arms or does nothing (0066 AC24).
    /// # Errors
    /// Reports a refused constraint.
    pub fn sketch_rail_press(&mut self, name: &str) -> Result<bool, JsValue> {
        self.session.sketch_rail_press(name).map_err(error)
    }
    /// Opens the on-canvas entry box on a dimension, from its text (0066 AC31).
    pub fn sketch_edit_dimension(&mut self, id: &str) -> bool {
        self.session.sketch_edit_dimension(&id.into())
    }
    /// Closes the on-canvas dimension edit without a change.
    pub fn sketch_close_dimension_edit(&mut self) {
        self.session.sketch_close_dimension_edit();
    }
    /// Chooses one concrete tool/variant.
    /// # Errors
    /// Refuses unknown tool names.
    pub fn set_sketch_tool(&mut self, name: &str) -> Result<(), JsValue> {
        let tool = SketchTool::from_name(name).ok_or_else(|| error("Unknown sketch tool"))?;
        self.session.set_sketch_tool(tool);
        Ok(())
    }
    /// Highlights validated free geometry without changing the document.
    pub fn sketch_show_free(&mut self) -> bool {
        self.session.sketch_show_free()
    }
    /// Highlights relationships fixing the selected geometry.
    pub fn sketch_inspect_selection(&mut self) {
        self.session.sketch_inspect_selection();
    }
    /// Highlights transitive parameter dependencies.
    pub fn sketch_inspect_parameter(&mut self, id: &str) -> bool {
        self.session.sketch_inspect_parameter(&id.into())
    }
    /// Selects an analytic entity from its inspection row.
    pub fn sketch_select_entity(&mut self, id: &str, extend: bool) -> bool {
        self.session.sketch_select_entity(&id.into(), extend)
    }
    /// Inspects or edits a relationship from its row/annotation.
    pub fn sketch_inspect_constraint(&mut self, id: &str, edit: bool) -> bool {
        self.session.sketch_inspect_constraint(&id.into(), edit)
    }
    /// Applies the named action to the analytic selection.
    /// # Errors
    /// Reports incompatible selection, units or expressions.
    pub fn sketch_apply_action(
        &mut self,
        name: &str,
        text: &str,
        reference: bool,
    ) -> Result<bool, JsValue> {
        self.session
            .sketch_apply_action(name, text, reference)
            .map_err(error)
    }
    /// Changes a dimension without changing its identity.
    /// # Errors
    /// Reports invalid expression or missing dimension.
    pub fn sketch_dimension_text(
        &mut self,
        id: &str,
        text: &str,
        reference: bool,
    ) -> Result<bool, JsValue> {
        self.session
            .sketch_dimension_text(&id.into(), text, reference)
            .map_err(error)
    }
    /// Removes an inspectable relationship.
    /// # Errors
    /// Reports missing editing context.
    pub fn sketch_remove_constraint(&mut self, id: &str) -> Result<bool, JsValue> {
        self.session
            .sketch_remove_constraint(&id.into())
            .map_err(error)
    }
    /// Adds or edits a named parameter using an explicit declared dimension.
    /// # Errors
    /// Reports invalid dimension, name or formula.
    pub fn sketch_parameter_text(
        &mut self,
        id: &str,
        name: &str,
        text: &str,
        dimension: &str,
    ) -> Result<bool, JsValue> {
        let dimension = match dimension {
            "Length" => Dimension::Length,
            "Angle" => Dimension::Angle,
            "Scalar" => Dimension::Scalar,
            _ => return Err(error("Unknown parameter dimension")),
        };
        self.session
            .sketch_parameter_text(id.into(), name, text, dimension)
            .map_err(error)
    }
    /// Creates a named parameter with its identity allocated by Rust.
    /// # Errors
    /// Reports invalid dimensions or parameter input.
    pub fn sketch_add_parameter(
        &mut self,
        name: &str,
        text: &str,
        dimension: &str,
    ) -> Result<String, JsValue> {
        let dimension = match dimension {
            "Length" => Dimension::Length,
            "Angle" => Dimension::Angle,
            "Scalar" => Dimension::Scalar,
            _ => return Err(error("Unknown parameter dimension")),
        };
        self.session
            .sketch_add_parameter(name, text, dimension)
            .map(|id| id.0)
            .map_err(error)
    }
    /// Deletes the current analytic selection.
    /// # Errors
    /// Reports invalid editing context.
    pub fn sketch_delete_selection(&mut self) -> Result<bool, JsValue> {
        self.session.sketch_delete_selection().map_err(error)
    }
    /// Changes selected geometry's construction membership.
    /// # Errors
    /// Reports invalid editing context.
    pub fn sketch_selection_construction(&mut self, construction: bool) -> Result<bool, JsValue> {
        self.session
            .sketch_selection_construction(construction)
            .map_err(error)
    }

    /// Removes an unreferenced parameter.
    /// # Errors
    /// Reports remaining formula references.
    pub fn sketch_remove_parameter(&mut self, id: &str) -> Result<bool, JsValue> {
        self.session
            .sketch_remove_parameter(&id.into())
            .map_err(error)
    }
    /// Releases or fixes the local origin.
    /// # Errors
    /// Reports missing origin/context.
    pub fn sketch_fix_origin(&mut self, fixed: bool) -> Result<bool, JsValue> {
        self.session.sketch_fix_origin(fixed).map_err(error)
    }
    /// Completes the entry box: `fields_json` is `[{"text": "...", "typed": bool}, ...]` in
    /// field order; each typed size becomes a driving dimension (AC39).
    /// # Errors
    /// Reports incomplete/invalid picks and values, or malformed JSON.
    pub fn sketch_complete_fields(&mut self, fields_json: &str) -> Result<bool, JsValue> {
        #[derive(serde::Deserialize)]
        struct Field {
            text: String,
            typed: bool,
        }
        let fields: Vec<Field> =
            serde_json::from_str(fields_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let texts: Vec<&str> = fields.iter().map(|f| f.text.as_str()).collect();
        let typed: Vec<bool> = fields.iter().map(|f| f.typed).collect();
        self.session
            .sketch_complete_fields(&texts, &typed)
            .map_err(error)
    }
    /// The entry box opened: a press-drag in progress continues as the follow state (AC39).
    pub fn sketch_open_entry(&mut self) {
        self.session.sketch_open_entry();
    }
    /// Previews a fillet from a typed length or parameter expression.
    /// # Errors
    /// Reports invalid expressions or incompatible relationships.
    pub fn sketch_preview_fillet_text(&mut self, text: &str) -> Result<(), JsValue> {
        self.session.sketch_preview_fillet_text(text).map_err(error)
    }

    /// Previews a radius-controlled fillet on the selected corner.
    /// # Errors
    /// Reports affected incompatible relationships.
    pub fn sketch_preview_fillet(&mut self, radius: f64) -> Result<(), JsValue> {
        self.session
            .sketch_preview_fillet(Length(radius))
            .map_err(error)
    }
    /// Commits a valid pending fillet preview.
    /// # Errors
    /// Reports a stale or missing preview.
    pub fn sketch_commit_preview(&mut self) -> Result<bool, JsValue> {
        self.session.sketch_commit_preview().map_err(error)
    }
    /// Explicitly removes parametric editing by conversion to ordinary paths.
    /// # Errors
    /// Reports invalid conversion context.
    pub fn sketch_convert_to_path(&mut self) -> Result<(), JsValue> {
        self.session.sketch_convert_to_path().map_err(error)
    }
}

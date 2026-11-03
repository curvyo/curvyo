//! The workbench registry and the active manufacturing as JSON for the front end (ADR 0001 §5:
//! one text across the wasm boundary): the switcher's entries, the Add manufacturing choices,
//! the extension list, the manifests, and the values the schema-rendered tab shows.

use curvyo_document_core::{ManufacturingSnapshot, NodeId};
use curvyo_extension_core::{Block, NoticeContext, Param, ParamValue};
use serde_json::{Value, json};

use super::super::{Session, Workbench};
use super::commands::{fixture_kind_of, workbench_of};

/// The id the front end names a built-in workbench by, or a manufacturing's full node id.
fn workbench_name(workbench: Workbench) -> String {
    match workbench {
        Workbench::Vector => "vector".to_owned(),
        Workbench::Sketch => "sketch".to_owned(),
        Workbench::Manufacturing(id) => id.full_id(),
    }
}

impl Session {
    /// The workbench named `name` (`vector`, `sketch` or a manufacturing's full id).
    #[must_use]
    pub fn workbench_named(&self, name: &str) -> Option<Workbench> {
        match name {
            "vector" => Some(Workbench::Vector),
            "sketch" => Some(Workbench::Sketch),
            _ => self
                .document
                .manufacturings()
                .into_iter()
                .find(|m| m.id.full_id() == name)
                .map(|m| Workbench::Manufacturing(m.id)),
        }
    }

    /// The manufacturing named by its full id.
    #[must_use]
    pub fn manufacturing_named(&self, name: &str) -> Option<NodeId> {
        self.document
            .manufacturings()
            .into_iter()
            .find(|m| m.id.full_id() == name)
            .map(|m| m.id)
    }

    /// The fixture of the active manufacturing named by its full id.
    #[must_use]
    pub fn fixture_named(&self, name: &str) -> Option<NodeId> {
        self.active_manufacturing()?
            .fixtures
            .into_iter()
            .find(|f| f.id.full_id() == name)
            .map(|f| f.id)
    }

    /// The registry as the switcher reads it (criteria 1, 11): the active workbench, the built-in
    /// entries, one entry per manufacturing in creation order (disabled when its process is not
    /// available), the Add manufacturing choices of the enabled extensions, the extension list
    /// and the enabled manifests.
    #[must_use]
    pub fn workbenches_json(&self) -> String {
        let mut entries = vec![
            json!({"id": "vector", "kind": "vector", "label": "Vector", "enabled": true}),
            json!({"id": "sketch", "kind": "sketch", "label": "Sketch", "enabled": true}),
        ];
        for manufacturing in self.document.manufacturings() {
            let enabled = self.process_enabled(&manufacturing.process);
            let icon = workbench_of(&manufacturing.process).map_or("Factory", |w| w.icon);
            entries.push(json!({
                "id": manufacturing.id.full_id(),
                "kind": "manufacturing",
                "process": manufacturing.process,
                "label": manufacturing.name,
                "icon": icon,
                "enabled": enabled,
            }));
        }
        let manifests = curvyo_ui_core::manifests();
        let add: Vec<Value> = manifests
            .iter()
            .filter(|m| self.manufacturing.extensions.is_enabled(m.id))
            .filter_map(|m| m.workbench())
            .map(|w| json!({"process": w.id, "label": w.label, "icon": w.icon}))
            .collect();
        let extensions: Vec<Value> = manifests
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "name": m.name,
                    "version": m.version,
                    "licence": m.licence,
                    "enabled": self.manufacturing.extensions.is_enabled(m.id),
                })
            })
            .collect();
        let enabled: Vec<&curvyo_extension_core::Manifest> = manifests
            .iter()
            .filter(|m| self.manufacturing.extensions.is_enabled(m.id))
            .collect();
        let disabled_manufacturings: Vec<Value> = self
            .document
            .manufacturings()
            .into_iter()
            .filter(|m| !self.process_enabled(&m.process))
            .map(|m| json!({"id": m.id.full_id(), "label": m.name}))
            .collect();
        json!({
            "active": workbench_name(self.workbench()),
            "entries": entries,
            "add": add,
            "extensions": extensions,
            "manifests": enabled,
            "disabled": disabled_manufacturings,
        })
        .to_string()
    }

    /// The active manufacturing's values for its schema-rendered tab and the status bar
    /// (UX notes §3 to §6; the Machine block of `0026`); `null` in a drawing workbench.
    #[must_use]
    pub fn manufacturing_view_json(&self) -> String {
        let Some(manufacturing) = self.active_manufacturing() else {
            return "null".to_owned();
        };
        let Some(contribution) = workbench_of(&manufacturing.process) else {
            return "null".to_owned();
        };
        let area = manufacturing.work_area();
        let context = NoticeContext {
            work_area: area,
            placed_bounds: self.placed_bounds_of(&manufacturing),
        };
        let mut notices = serde_json::Map::new();
        for block in contribution.tabs.iter().flat_map(|tab| &tab.blocks) {
            if let Block::Notice { id } = block
                && let Some(text) = curvyo_ui_core::notice(&manufacturing.process, id, &context)
            {
                notices.insert((*id).to_owned(), Value::String(text));
            }
        }
        let machine = self.machine_view();
        let machine_name = machine
            .as_ref()
            .and_then(|m| m["copy"]["name"].as_str().map(str::to_owned))
            .unwrap_or_default();
        let notice = self.manufacturing.notice.map(|n| {
            json!({
                "seq": n.seq,
                "text": super::READ_ONLY_NOTICE,
                "x": n.at.map(|a| a.0),
                "y": n.at.map(|a| a.1),
            })
        });
        json!({
            "id": manufacturing.id.full_id(),
            "name": manufacturing.name,
            "process": manufacturing.process,
            "status": format!(
                "{machine_name} · {:.1} × {:.1} mm",
                area.width.as_mm(),
                area.height.as_mm()
            ),
            "machine": machine,
            "notices": notices,
            "fixtures": fixtures_json(&manufacturing, &contribution),
            "read_only_notice": notice,
        })
        .to_string()
    }
}

/// Each fixture with its validated values in the manifest's units (mm for a length).
fn fixtures_json(
    manufacturing: &ManufacturingSnapshot,
    contribution: &curvyo_extension_core::WorkbenchContribution,
) -> Vec<Value> {
    manufacturing
        .fixtures
        .iter()
        .map(|fixture| {
            let kind = fixture_kind_of(contribution, &manufacturing.process, &fixture.kind);
            let values: serde_json::Map<String, Value> = kind
                .map(|kind| {
                    kind.params
                        .iter()
                        .map(|p| (p.key.to_owned(), value_json(p, &fixture.params)))
                        .collect()
                })
                .unwrap_or_default();
            json!({
                "id": fixture.id.full_id(),
                "name": fixture.name,
                "kind": kind.map(|k| k.id),
                "icon": kind.map(|k| k.icon),
                "visible": fixture.visible,
                "values": values,
            })
        })
        .collect()
}

fn value_json(param: &Param, stored: &curvyo_extension_core::StoredParams) -> Value {
    match param.read(stored) {
        ParamValue::Length(length) => json!(length.as_mm()),
    }
}

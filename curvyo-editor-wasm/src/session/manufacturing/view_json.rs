//! The page kinds and the active laser page as JSON for the front end (ADR 0001 §5: one text
//! across the wasm boundary): the kind list, each page's chip, the extension list, the
//! manifests, and the values the Machine and Fixtures blocks show.

use curvyo_document_core::{ManufacturingSnapshot, NodeId, PageKind};
use curvyo_extension_core::{Block, NoticeContext, Param, ParamValue};
use serde_json::{Value, json};

use super::super::{Session, Workbench};
use super::commands::{fixture_kind_of, workbench_of};

/// The id the front end names a workbench by.
const fn workbench_name(workbench: Workbench) -> &'static str {
    match workbench {
        Workbench::Vector => "vector",
        Workbench::Sketch => "sketch",
        Workbench::Manufacturing(_) => "manufacturing",
    }
}

/// How a page's kind is shown (`specs/0098` criterion 6): the class the front end keys its
/// registry by, the word and the Lucide glyph, and whether its extension is off.
pub struct KindChip {
    /// `drawing`, `sketch`, the process id of an output page, or `unknown`.
    pub kind: String,
    /// "Drawing", "Sketch", "Laser", "Laser (off)", "Unknown kind".
    pub word: String,
    /// The glyph's Lucide name.
    pub icon: &'static str,
    /// Whether the page is an output page of a process switched off.
    pub off: bool,
}

impl Session {
    /// The chip of a page of `kind`: Drawing and Sketch from the shell, an output kind's word and
    /// glyph from its manifest (`adrs.md` decision 3); no branch names a process.
    #[must_use]
    pub fn kind_chip(&self, kind: &PageKind) -> KindChip {
        let unknown = || KindChip {
            kind: "unknown".to_owned(),
            word: "Unknown kind".to_owned(),
            icon: "CircleHelp",
            off: false,
        };
        match kind {
            PageKind::Drawing => KindChip {
                kind: "drawing".to_owned(),
                word: "Drawing".to_owned(),
                icon: "PenTool",
                off: false,
            },
            PageKind::Sketch => KindChip {
                kind: "sketch".to_owned(),
                word: "Sketch".to_owned(),
                icon: "DraftingCompass",
                off: false,
            },
            PageKind::Output { process, .. } => workbench_of(process).map_or_else(unknown, |w| {
                let off = !self.process_enabled(process);
                KindChip {
                    kind: process.clone(),
                    word: if off {
                        format!("{} (off)", w.label)
                    } else {
                        w.label.to_owned()
                    },
                    icon: w.icon,
                    off,
                }
            }),
            PageKind::Unknown(_) => unknown(),
        }
    }

    /// The machine name of an output page's copy, for the second line of its row.
    #[must_use]
    pub fn machine_name_of(&self, kind: &PageKind) -> Option<String> {
        let manufacturing = self.document.manufacturing(kind.manufacturing()?)?;
        let read = curvyo_laser_core::read_copy(
            manufacturing.profile.as_ref(),
            manufacturing.legacy_machine_name.as_deref(),
        );
        Some(read.copy.values.name)
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

    /// The page kind list and the extensions (ADR 0042 §7): the active page's workbench, the
    /// kinds of the Kind switch (Drawing, Sketch, then one per enabled extension manifest), the
    /// extension list and the enabled manifests.
    #[must_use]
    pub fn workbenches_json(&self) -> String {
        let manifests = curvyo_ui_core::manifests();
        let mut kinds = vec![
            json!({"id": "drawing", "label": "Drawing", "icon": "PenTool", "workbench": "vector"}),
            json!({"id": "sketch", "label": "Sketch", "icon": "DraftingCompass", "workbench": "sketch"}),
        ];
        kinds.extend(
            manifests
                .iter()
                .filter(|m| self.manufacturing.extensions.is_enabled(m.id))
                .filter_map(|m| m.workbench())
                .map(|w| json!({"id": w.id, "label": w.label, "icon": w.icon, "workbench": "manufacturing"})),
        );
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
        let page = self.active_page();
        let chip = self.kind_chip(&page.kind);
        json!({
            "active": workbench_name(self.workbench()),
            "page": {"id": page.id, "name": page.name, "kind": chip.kind, "word": chip.word, "icon": chip.icon, "off": chip.off},
            "kinds": kinds,
            "extensions": extensions,
            "manifests": enabled,
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
        let name = manufacturing
            .page
            .as_ref()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        json!({
            "id": manufacturing.id.full_id(),
            "name": name,
            "process": manufacturing.process,
            "status": format!(
                "{machine_name} · {:.1} × {:.1} mm",
                area.width.as_mm(),
                area.height.as_mm()
            ),
            "machine": machine,
            "notices": notices,
            "fixtures": fixtures_json(&manufacturing, &contribution),
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

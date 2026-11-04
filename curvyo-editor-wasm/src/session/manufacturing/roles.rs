//! The laser roles in the session (`specs/0027-manufacturing-roles`, `adrs.md` notes 2 to 4): the
//! map of each leaf to the layer that holds it, read once per drawing read, the role press, the
//! Role view switch and the Layers block's view.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{ManufacturingSnapshot, NodeId, NodeKind, NodeRead, ObjectSnapshot};
use curvyo_laser_core::{Role, RoleRead, RoleTarget, read_role, role_notices};
use curvyo_render_core::Look;
use serde_json::{Value, json};

use super::super::Session;

/// One row of the Layers block, before its role is read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LayerRow {
    target: RoleTarget,
    name: String,
    hidden: bool,
    locked: bool,
}

/// What the role reads need of one drawing read, built in one pre-order pass.
#[derive(Debug, Default)]
pub(in crate::session) struct LayerMap {
    /// Each leaf's role target: its top-level layer, or `outside`.
    leaves: HashMap<NodeId, RoleTarget>,
    /// The layers front-most first, then the outside row while loose objects exist.
    rows: Vec<LayerRow>,
    /// Per target: how many output-capable leaves (visible, not a guide or dimension) and how
    /// many of them are images (criterion 15).
    capable: HashMap<RoleTarget, (usize, usize)>,
}

impl LayerMap {
    fn of(nodes: &[NodeRead]) -> Self {
        let mut map = Self::default();
        let mut top = RoleTarget::Outside;
        let mut outside = false;
        for node in nodes {
            if node.depth == 0 {
                if node.kind == NodeKind::Layer {
                    top = RoleTarget::Layer(node.id);
                    map.rows.push(LayerRow {
                        target: top,
                        name: node.registers.name.clone().unwrap_or_default(),
                        hidden: !node.visible,
                        locked: node.locked,
                    });
                } else {
                    top = RoleTarget::Outside;
                    outside = true;
                }
            }
            let Some(object) = node.object() else {
                continue;
            };
            map.leaves.insert(node.id, top);
            if node.is_output_node() && !matches!(object, ObjectSnapshot::Dimension(_)) {
                let entry = map.capable.entry(top).or_default();
                entry.0 += 1;
                entry.1 += usize::from(matches!(object, ObjectSnapshot::Image(_)));
            }
        }
        map.rows.reverse();
        if outside {
            map.rows.push(LayerRow {
                target: RoleTarget::Outside,
                name: "Objects outside layers".to_owned(),
                hidden: false,
                locked: false,
            });
        }
        map
    }

    /// The role target of the leaf `id`.
    fn target_of(&self, id: NodeId) -> RoleTarget {
        self.leaves.get(&id).copied().unwrap_or(RoleTarget::Outside)
    }
}

/// The layer map with the nodes of the read it was built from.
type HeldMap = (Arc<[NodeRead]>, Arc<LayerMap>);

/// The map of the last drawing read and the Role view switch.
#[derive(Debug, Default)]
pub(in crate::session) struct RoleState {
    /// Off when the maker switched Role view off; on by default and at every start (criterion 6).
    role_view_off: bool,
    map: RefCell<Option<HeldMap>>,
}

/// The look of a leaf under `role` on the output page (criterion 6).
fn look_of(role: RoleRead) -> Look {
    match role.role() {
        Some(Role::Cut) => Look::Outline(curvyo_render_core::ROLE_CUT),
        Some(Role::Engrave) => Look::Area(curvyo_render_core::ROLE_ENGRAVE),
        Some(Role::Ignore) | None => Look::Muted(curvyo_render_core::ROLE_IGNORE),
    }
}

impl Session {
    /// The layer map of the current drawing read, built once per read.
    pub(in crate::session) fn layer_map(&self) -> Arc<LayerMap> {
        let read = self.document_read();
        let cache = &self.manufacturing.roles.map;
        if let Some((nodes, map)) = cache.borrow().as_ref()
            && Arc::ptr_eq(nodes, &read.nodes)
        {
            return Arc::clone(map);
        }
        let map = Arc::new(LayerMap::of(&read.nodes));
        *cache.borrow_mut() = Some((Arc::clone(&read.nodes), Arc::clone(&map)));
        map
    }

    /// Whether the output page draws the role looks (criterion 6).
    #[must_use]
    pub const fn role_view(&self) -> bool {
        !self.manufacturing.roles.role_view_off
    }

    /// Switches Role view on or off: view state, not saved, not an undo step.
    pub fn set_role_view(&mut self, on: bool) {
        self.manufacturing.roles.role_view_off = !on;
    }

    /// The looks of `objects` on the output page of `manufacturing` while Role view is on,
    /// `None` while it is off.
    pub(in crate::session) fn role_looks(
        &self,
        manufacturing: &ManufacturingSnapshot,
        objects: &[ObjectSnapshot],
    ) -> Option<HashMap<NodeId, Look>> {
        if !self.role_view() {
            return None;
        }
        let map = self.layer_map();
        let mut cached: HashMap<RoleTarget, Look> = HashMap::new();
        Some(
            objects
                .iter()
                .map(|object| {
                    let target = map.target_of(object.id());
                    let look = *cached.entry(target).or_insert_with(|| {
                        look_of(read_role(&manufacturing.process_values, target))
                    });
                    (object.id(), look)
                })
                .collect(),
        )
    }

    /// Sets the role of the row `key` (a layer's `peer:counter` or `outside`) in the active
    /// manufacturing, or clears it with `None`, in one undo step "Set role" (criterion 4). `false`
    /// when nothing changed, the row does not exist or the process is switched off.
    pub fn set_layer_role(&mut self, key: &str, role: Option<Role>) -> bool {
        let Some(manufacturing) = self.active_manufacturing() else {
            return false;
        };
        let Some(target) = RoleTarget::from_key_text(key) else {
            return false;
        };
        if !self.process_enabled(&manufacturing.process)
            || !self.layer_map().rows.iter().any(|row| row.target == target)
        {
            return false;
        }
        let value = curvyo_laser_core::role_value(role);
        self.recording(|this| {
            this.document
                .set_process_register(
                    manufacturing.id,
                    &target.key(),
                    value.as_ref(),
                    curvyo_laser_core::SET_ROLE_COMMIT_LABEL,
                )
                .unwrap_or(false)
        })
    }

    /// The Layers block's view (criteria 1, 2, 11, 14, 15, 18): the Role view state, one row per
    /// layer with its role, and the notice lines.
    pub(in crate::session) fn layers_view(&self, manufacturing: &ManufacturingSnapshot) -> Value {
        let map = self.layer_map();
        let mut without_role = 0;
        let mut images_in_cut = 0;
        let rows: Vec<Value> = map
            .rows
            .iter()
            .map(|row| {
                let role = read_role(&manufacturing.process_values, row.target);
                let (capable, images) = map.capable.get(&row.target).copied().unwrap_or_default();
                match role.role() {
                    None if capable > 0 => without_role += 1,
                    Some(Role::Cut) => images_in_cut += images,
                    _ => {}
                }
                json!({
                    "key": row.target.key_text(),
                    "name": row.name,
                    "outside": row.target == RoleTarget::Outside,
                    "role": role.role().map(Role::as_str),
                    "unknown": role == RoleRead::Unknown,
                    "hidden": row.hidden,
                    "locked": row.locked,
                })
            })
            .collect();
        json!({
            "role_view": self.role_view(),
            "rows": rows,
            "notices": role_notices(without_role, images_in_cut),
        })
    }
}

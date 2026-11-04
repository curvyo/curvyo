//! The laser roles of ADR 0041 (`specs/0027-manufacturing-roles`): what the laser does with the
//! content of one drawing layer, as stored on the manufacturing under one register per layer.

use curvyo_document_core::NodeId;
use curvyo_extension_core::StoredParams;

use crate::manifest::PROCESS_ID;

/// The commit label of a role press ("Set role", criterion 4).
pub const SET_ROLE_COMMIT_LABEL: &str = "set_role";

/// The family of the role registers inside the laser's namespace (ADR 0041 §2).
const ROLE_FAMILY: &str = "role";

/// The reserved key word of the objects in no layer (criterion 11); no node id has this form.
const OUTSIDE: &str = "outside";

/// What the laser does with a layer's content (criterion 2; Frage 140 A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// Follows every outline.
    Cut,
    /// Burns the closed areas; all-open leaves as a line; the only role for images.
    Engrave,
    /// Left out of the job.
    Ignore,
}

impl Role {
    /// The stored value (published contract, ADR 0041 §2).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cut => "cut",
            Self::Engrave => "engrave",
            Self::Ignore => "ignore",
        }
    }

    /// The role a stored text names, `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "cut" => Some(Self::Cut),
            "engrave" => Some(Self::Engrave),
            "ignore" => Some(Self::Ignore),
            _ => None,
        }
    }
}

/// Whose role a register holds: a top-level layer of the drawing, or the objects in no layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleTarget {
    /// The layer with this node id.
    Layer(NodeId),
    /// The loose top-level objects and groups (criterion 11).
    Outside,
}

impl RoleTarget {
    /// The text after the family in the key: the layer's `peer:counter`, or `outside`.
    #[must_use]
    pub fn key_text(self) -> String {
        match self {
            Self::Layer(id) => id.to_register_text(),
            Self::Outside => OUTSIDE.to_owned(),
        }
    }

    /// The target a key text names (the inverse of [`RoleTarget::key_text`]).
    #[must_use]
    pub fn from_key_text(text: &str) -> Option<Self> {
        if text == OUTSIDE {
            return Some(Self::Outside);
        }
        NodeId::from_register_text(text).map(Self::Layer)
    }

    /// The register key, `x.curvyo.laser.role.<key text>` (criterion 5).
    #[must_use]
    pub fn key(self) -> String {
        format!("x.{PROCESS_ID}.{ROLE_FAMILY}.{}", self.key_text())
    }
}

/// A stored role as read (ADR 0041 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleRead {
    /// Nothing stored (criterion 3).
    Unassigned,
    /// One of the three roles.
    Known(Role),
    /// A value that is not one of the three strings: no role, never rewritten (criterion 18).
    Unknown,
}

impl RoleRead {
    /// The role, `None` for Unassigned and Unknown.
    #[must_use]
    pub const fn role(self) -> Option<Role> {
        match self {
            Self::Known(role) => Some(role),
            Self::Unassigned | Self::Unknown => None,
        }
    }
}

/// The role of `target` in the raw process registers of a manufacturing.
#[must_use]
pub fn read_role(process_values: &StoredParams, target: RoleTarget) -> RoleRead {
    match process_values.get(&target.key()) {
        None => RoleRead::Unassigned,
        Some(value) => value
            .as_str()
            .and_then(Role::parse)
            .map_or(RoleRead::Unknown, RoleRead::Known),
    }
}

/// The value a role press writes: the role, or `None` (delete) for no role.
#[must_use]
pub fn role_value(role: Option<Role>) -> Option<serde_json::Value> {
    role.map(|role| serde_json::Value::from(role.as_str()))
}

/// Whether an output-capable leaf (visible, not a guide, dimension or fixture) is output under
/// `role` (ADR 0041 §6): Cut or Engrave, an image only under Engrave. Unassigned and Unknown are
/// never output (fail closed).
#[must_use]
pub const fn is_output(role: RoleRead, is_image: bool) -> bool {
    match role {
        RoleRead::Known(Role::Engrave) => true,
        RoleRead::Known(Role::Cut) => !is_image,
        RoleRead::Known(Role::Ignore) | RoleRead::Unassigned | RoleRead::Unknown => false,
    }
}

/// The notice lines of the Layers block (criterion 15), each at most 70 characters and only for a
/// count above zero: rows with no role that hold output-capable objects, then images in Cut
/// layers.
#[must_use]
pub fn role_notices(layers_without_role: usize, images_in_cut: usize) -> Vec<String> {
    let mut lines = Vec::new();
    match layers_without_role {
        0 => {}
        1 => lines.push("1 layer has no role and is not output.".to_owned()),
        n => lines.push(format!("{n} layers have no role and are not output.")),
    }
    match images_in_cut {
        0 => {}
        1 => lines.push("1 image in a Cut layer is not output.".to_owned()),
        n => lines.push(format!("{n} images in Cut layers are not output.")),
    }
    lines
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn values(pairs: &[(&str, serde_json::Value)]) -> StoredParams {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    }

    fn layer(text: &str) -> RoleTarget {
        RoleTarget::from_key_text(text).unwrap()
    }

    #[test]
    fn the_key_names_the_layer_id_or_the_reserved_word() {
        assert_eq!(layer("3:17").key(), "x.curvyo.laser.role.3:17");
        assert_eq!(RoleTarget::Outside.key(), "x.curvyo.laser.role.outside");
        assert_eq!(
            RoleTarget::from_key_text("outside"),
            Some(RoleTarget::Outside)
        );
        assert_eq!(RoleTarget::from_key_text("Outside"), None);
        assert_eq!(layer("3:17").key_text(), "3:17");
    }

    #[test]
    fn the_three_values_read_as_roles_and_everything_else_as_unknown() {
        let stored = values(&[
            ("x.curvyo.laser.role.1:1", json!("cut")),
            ("x.curvyo.laser.role.1:2", json!("engrave")),
            ("x.curvyo.laser.role.1:3", json!("ignore")),
            ("x.curvyo.laser.role.1:4", json!("score")),
            ("x.curvyo.laser.role.1:5", json!(3)),
            ("x.curvyo.laser.role.1:6", json!("Cut")),
        ]);
        let read = |t: &str| read_role(&stored, layer(t));
        assert_eq!(read("1:1"), RoleRead::Known(Role::Cut));
        assert_eq!(read("1:2"), RoleRead::Known(Role::Engrave));
        assert_eq!(read("1:3"), RoleRead::Known(Role::Ignore));
        for unknown in ["1:4", "1:5", "1:6"] {
            assert_eq!(read(unknown), RoleRead::Unknown, "{unknown}");
            assert_eq!(read(unknown).role(), None);
        }
        assert_eq!(read("1:9"), RoleRead::Unassigned);
        assert_eq!(
            read_role(&stored, RoleTarget::Outside),
            RoleRead::Unassigned
        );
    }

    #[test]
    fn a_role_round_trips_through_its_stored_value_and_no_role_deletes() {
        for role in [Role::Cut, Role::Engrave, Role::Ignore] {
            let value = role_value(Some(role)).unwrap();
            assert_eq!(Role::parse(value.as_str().unwrap()), Some(role));
        }
        assert_eq!(role_value(None), None);
    }

    #[test]
    fn only_cut_and_engrave_are_output_and_an_image_only_under_engrave() {
        let cut = RoleRead::Known(Role::Cut);
        let engrave = RoleRead::Known(Role::Engrave);
        let ignore = RoleRead::Known(Role::Ignore);
        assert!(is_output(cut, false) && is_output(engrave, false));
        assert!(!is_output(cut, true) && is_output(engrave, true));
        for none in [ignore, RoleRead::Unassigned, RoleRead::Unknown] {
            assert!(!is_output(none, false) && !is_output(none, true));
        }
    }

    #[test]
    fn the_notices_count_singular_and_plural_within_70_characters() {
        assert_eq!(role_notices(0, 0), Vec::<String>::new());
        assert_eq!(
            role_notices(1, 1),
            vec![
                "1 layer has no role and is not output.",
                "1 image in a Cut layer is not output."
            ]
        );
        assert_eq!(
            role_notices(3, 0),
            vec!["3 layers have no role and are not output."]
        );
        assert_eq!(
            role_notices(0, 2),
            vec!["2 images in Cut layers are not output."]
        );
        for line in role_notices(usize::MAX, usize::MAX) {
            assert!(line.chars().count() <= 70, "{line}");
        }
    }
}

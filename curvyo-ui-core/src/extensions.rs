//! The registration module of the compiled-in extensions (ADR 0034 §3, §6): the list, the
//! dispatch by extension id and the per-user list of disabled extensions (`extensions.toml`).
//!
//! Adding an extension is a Cargo dependency plus one entry in [`manifests`] and one arm in each
//! dispatch below; nothing else in the shell names it. No `Extension` trait until a second
//! extension crate exists.

use std::collections::BTreeSet;

use curvyo_extension_core::{Manifest, NoticeContext};
use serde::{Deserialize, Serialize};

/// Every compiled-in extension's manifest, in list order.
#[must_use]
pub fn manifests() -> Vec<Manifest> {
    vec![curvyo_laser_core::manifest()]
}

/// The manifest of the extension `id`, `None` for an id no compiled-in extension has.
#[must_use]
pub fn manifest(id: &str) -> Option<Manifest> {
    manifests().into_iter().find(|manifest| manifest.id == id)
}

/// The text of the notice `id` of `process`, `None` while it does not apply.
#[must_use]
pub fn notice(process: &str, id: &str, context: &NoticeContext) -> Option<String> {
    match process {
        curvyo_laser_core::PROCESS_ID => curvyo_laser_core::notice(id, context),
        _ => None,
    }
}

/// The ids of the extensions the maker switched off: a per-user app setting kept in the small
/// text file `extensions.toml`, never in a project (ADR 0034 §6). Default: all enabled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionSettings {
    #[serde(default)]
    disabled: BTreeSet<String>,
}

impl ExtensionSettings {
    /// The settings `text` holds; a text that is not such a file reads as everything enabled
    /// (permit), and an unknown id is kept, so a build without that extension writes it back.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        toml::from_str(text).unwrap_or_default()
    }

    /// The file's text: `disabled = ["curvyo.laser"]`.
    #[must_use]
    pub fn to_text(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    /// Whether the extension `id` is switched on.
    #[must_use]
    pub fn is_enabled(&self, id: &str) -> bool {
        !self.disabled.contains(id)
    }

    /// Switches the extension `id` on or off; `false` when it already was.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if enabled {
            self.disabled.remove(id)
        } else {
            self.disabled.insert(id.to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_laser_is_the_compiled_in_extension() {
        let ids: Vec<&str> = manifests().iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["curvyo.laser"]);
        assert!(manifest("curvyo.laser").is_some());
        assert!(manifest("example.cnc").is_none());
    }

    #[test]
    fn the_settings_file_round_trips_and_a_broken_one_enables_everything() {
        let mut settings = ExtensionSettings::default();
        assert!(settings.is_enabled("curvyo.laser"));
        assert!(settings.set_enabled("curvyo.laser", false));
        assert!(!settings.set_enabled("curvyo.laser", false));
        let text = settings.to_text();
        assert_eq!(text.trim(), r#"disabled = ["curvyo.laser"]"#);
        assert_eq!(ExtensionSettings::parse(&text), settings);
        assert!(ExtensionSettings::parse("disabled = 3").is_enabled("curvyo.laser"));
        assert!(ExtensionSettings::parse("").is_enabled("curvyo.laser"));
    }
}

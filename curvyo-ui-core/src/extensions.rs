//! The registration module of the compiled-in extensions (ADR 0034 §3, §6, ADR 0080 §1): the
//! list of [`Extension`] values, the dispatch by extension id and the per-user list of disabled
//! extensions (`extensions.toml`).
//!
//! Adding an extension is a Cargo dependency plus one entry in [`EXTENSIONS`]; nothing else in the
//! shell names it.

use std::collections::BTreeSet;

use curvyo_extension_core::{Extension, GenerateRequest, Generated, Manifest, NoticeContext};
use serde::{Deserialize, Serialize};

/// Every compiled-in extension, in list order.
static EXTENSIONS: [&dyn Extension; 2] = [
    &curvyo_laser_core::LASER,
    &curvyo_generators_core::GENERATORS,
];

fn extension(id: &str) -> Option<&'static dyn Extension> {
    EXTENSIONS
        .iter()
        .copied()
        .find(|extension| extension.manifest().id == id)
}

/// Every compiled-in extension's manifest, in list order.
#[must_use]
pub fn manifests() -> Vec<Manifest> {
    EXTENSIONS
        .iter()
        .map(|extension| extension.manifest())
        .collect()
}

/// The manifest of the extension `id`, `None` for an id no compiled-in extension has.
#[must_use]
pub fn manifest(id: &str) -> Option<Manifest> {
    extension(id).map(Extension::manifest)
}

/// The text of the notice `id` of `process`, `None` while it does not apply.
#[must_use]
pub fn notice(process: &str, id: &str, context: &NoticeContext) -> Option<String> {
    extension(process)?.notice(id, context)
}

/// What the extension `id` generates for `request`; `None` for an unknown extension or kind.
#[must_use]
pub fn generate_object(id: &str, request: &GenerateRequest<'_>) -> Option<Generated> {
    extension(id)?.generate(request)
}

/// The ids of the extensions the maker switched off: a per-user app setting kept in the small
/// text file `extensions.toml`, never in a project (ADR 0034 §6). Default: all enabled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionSettings {
    #[serde(default)]
    disabled: BTreeSet<String>,
}

thread_local! {
    /// The settings the session that is writing right now runs with, for the commit paths that
    /// have no settings of their own (a resize of a generated object, ADR 0080 §5).
    static ACTIVE: std::cell::RefCell<ExtensionSettings> =
        std::cell::RefCell::new(ExtensionSettings::default());
}

/// Makes `settings` the ones the commit paths of this thread read (a session sets it before every
/// write it records).
pub fn set_active_extension_settings(settings: &ExtensionSettings) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if *active != *settings {
            active.clone_from(settings);
        }
    });
}

/// The settings [`set_active_extension_settings`] last set on this thread (all enabled before).
#[must_use]
pub fn active_extension_settings() -> ExtensionSettings {
    ACTIVE.with(|active| active.borrow().clone())
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
    fn the_laser_and_the_generators_are_the_compiled_in_extensions() {
        let ids: Vec<&str> = manifests().iter().map(|m| m.id).collect();
        assert_eq!(ids, vec!["curvyo.laser", "curvyo.generators"]);
        assert!(manifest("curvyo.laser").is_some());
        assert!(manifest("example.cnc").is_none());
    }

    #[test]
    fn every_declared_generator_kind_answers_at_its_defaults() {
        use curvyo_extension_core::{Param, Seed, StoredParams};
        let mut kinds = 0;
        for manifest in manifests() {
            for kind in manifest.generators() {
                let defaults: StoredParams = Param::defaults(&kind.schema());
                let request = GenerateRequest {
                    kind: kind.id,
                    params: &defaults,
                    seed: Seed(1),
                    edit: None,
                };
                let generated = generate_object(manifest.id, &request).expect(kind.id);
                assert_ne!(generated.parts.len(), 0);
                kinds += 1;
            }
        }
        assert_eq!(kinds, 4, "box, path box, gear, rounded box");
        let request = GenerateRequest {
            kind: "gear",
            params: &StoredParams::new(),
            seed: Seed(1),
            edit: None,
        };
        assert!(generate_object("curvyo.laser", &request).is_none());
        assert!(generate_object("example.cnc", &request).is_none());
    }

    #[test]
    fn every_manifest_serialises_to_json_for_the_front_end() {
        for manifest in manifests() {
            let json = serde_json::to_value(&manifest).unwrap();
            assert_eq!(json["id"], manifest.id);
        }
        let generators = serde_json::to_value(manifest("curvyo.generators").unwrap()).unwrap();
        assert_eq!(generators["contributions"][2]["kind"], "generator");
        assert_eq!(generators["contributions"][2]["id"], "gear");
        let web = &generators["contributions"][2]["groups"][2]["params"][0];
        assert_eq!(web["kind"]["kind"], "choice");
        assert_eq!(web["kind"]["options"][1]["id"], "spokes");
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

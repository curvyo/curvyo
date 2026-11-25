//! The host side of the library browser (`specs/0152-library-browser`, ADR 0086 decision 5):
//! the list of what the Library tab holds, once per change, and the icon tiles drawn in the UI's
//! colour. Nothing is fetched here that the Library tab would not fetch.

use std::hash::{Hash, Hasher};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use curvyo_library_core::{Rgb8, SourceEntry, Sources, has_fixed_colour, icon_tile_svg};
use curvyo_storage_io::Transport;
use curvyo_ui_core::library_browser::{BrowseList, LocalView, browse_list};
use curvyo_ui_core::library_online::OnlineView;

use crate::online_state::OnlineState;

impl OnlineState {
    /// The browser's list (criterion 9): the local part and what this session fetched.
    pub(crate) fn browse_list(
        &self,
        sources: &Sources,
        local: Option<LocalView<'_>>,
    ) -> BrowseList {
        let ready_added = sources
            .entries()
            .iter()
            .any(|e| curvyo_library_core::is_ready(&e.address));
        let expanded = std::collections::HashSet::new();
        self.with_views(sources, |views| {
            browse_list(
                local,
                &OnlineView {
                    sources: views,
                    ready_offered: !ready_added,
                    expanded: &expanded,
                    query: "",
                },
            )
        })
    }

    /// The tile SVGs of `names` of `prefix` drawn in `colour`, the ones not held fetched in one
    /// request: `(name, svg, has its own colours)` (criterion 6).
    pub(crate) fn icon_tiles(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        prefix: &str,
        names: &[String],
        colour: Rgb8,
    ) -> Vec<(String, String, bool)> {
        self.icons(transport, entry, prefix, names)
            .iter()
            .map(|icon| {
                (
                    icon.name.clone(),
                    icon_tile_svg(icon, colour),
                    has_fixed_colour(&icon.body),
                )
            })
            .collect()
    }
}

/// The list as JSON and its revision.
#[derive(Debug, Serialize)]
pub(crate) struct BrowseItems {
    revision: String,
    json: String,
}

/// The revision of a list's JSON: equal texts have equal revisions.
fn revision_of(json: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}-{}", hasher.finish(), json.len())
}

/// The browser's list; `None` while it is the one of `revision` (ADR 0086 decision 5, Proposal).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn library_browser_items(app: AppHandle, revision: String) -> Option<BrowseItems> {
    let (sources, index) = crate::library::local_parts(&app);
    let local = LocalView {
        sources: &sources,
        index: &index,
    };
    let list = app
        .state::<OnlineState>()
        .browse_list(&crate::online::listed_sources(&app), Some(local));
    let json = serde_json::to_string(&list).ok()?;
    let now = revision_of(&json);
    (now != revision).then_some(BrowseItems {
        revision: now,
        json,
    })
}

/// The tile SVGs of icons of one set, one request for the ones not held: `[name, svg, fixed]`.
#[tauri::command]
pub(crate) async fn online_icon_tiles(
    app: AppHandle,
    key: String,
    prefix: String,
    names: Vec<String>,
    colour: [u8; 3],
) -> Vec<(String, String, bool)> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OnlineState>();
        let Some(entry) = crate::online::listed_sources(&app).find(&key).cloned() else {
            return Vec::new();
        };
        let [r, g, b] = colour;
        state.icon_tiles(&state.transport, &entry, &prefix, &names, Rgb8(r, g, b))
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_lists_have_equal_revisions() {
        assert_eq!(revision_of("{}"), revision_of("{}"));
        assert_ne!(revision_of("{}"), revision_of("{\"a\":1}"));
    }
}

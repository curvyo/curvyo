//! The library browser's places and their history (spec `0152` criteria 30 and 31, UX notes 0.3
//! and 0.4): what a place holds, the session's stack of places left (at most 50, no Forward), the
//! Back button's name, and the selection rule of the multi-select facet lists.

use serde::{Deserialize, Serialize};

use crate::library_browser::BrowseList;
use crate::library_browser_query::{BrowseState, Sort};
use crate::library_browser_sets::SetsState;

/// The most places the history keeps.
pub const MAX_PLACES: usize = 50;

/// What the pane shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceView {
    /// The tiles.
    #[default]
    Items,
    /// The cards.
    Sets,
    /// The hosting guide.
    Guide,
    /// The Page preview of Pick elements….
    Pick,
}

/// A place: the view and everything that makes it (UX notes 0.3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Place {
    /// The view.
    pub view: PlaceView,
    /// The Items view's search, facets, sort and level.
    pub items: BrowseState,
    /// The Sets view's search and facets.
    pub sets: SetsState,
    /// The current item (Items) or card (Sets).
    pub current: Option<u32>,
    /// The first visible tile or card, the scroll position as an identity.
    pub anchor: Option<u32>,
    /// What was opened from this place (the focus goes back to it).
    pub opened: Option<u32>,
}

/// The session's stack of places left, newest last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaceHistory {
    entries: Vec<(Place, bool)>,
}

impl PlaceHistory {
    /// The maker left `place`; `by_opening` when something was opened from it (a set, card,
    /// multi-page item, Show pages, Pick elements…, the guide), not for a view switch.
    pub fn push(&mut self, place: Place, by_opening: bool) {
        if self.entries.len() == MAX_PLACES {
            self.entries.remove(0);
        }
        self.entries.push((place, by_opening));
    }

    /// Back: the newest place left, taken off the stack.
    pub fn back(&mut self) -> Option<Place> {
        self.entries.pop().map(|(place, _)| place)
    }

    /// The newest place left, without taking it.
    #[must_use]
    pub fn peek(&self) -> Option<&Place> {
        self.entries.last().map(|(place, _)| place)
    }

    /// Whether Escape goes Back: the newest entry was left by opening something (H6).
    #[must_use]
    pub fn escape_goes_back(&self) -> bool {
        self.entries.last().is_some_and(|(_, opened)| *opened)
    }

    /// The number of places held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there is no place to go back to.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The Back button's name for going back to `place` (UX U41): "Back to Sets", "Back to all
/// items", "Back to Lucide", 'Back to "arrow"'.
#[must_use]
pub fn back_name(list: &BrowseList, place: &Place) -> String {
    let short = match place.view {
        PlaceView::Sets => "Sets".to_owned(),
        PlaceView::Guide => "Host your own library".to_owned(),
        PlaceView::Pick => "the page".to_owned(),
        PlaceView::Items => items_name(list, &place.items),
    };
    format!("Back to {short}")
}

fn items_name(list: &BrowseList, state: &BrowseState) -> String {
    if let Some(item) = state.level.and_then(|l| list.items.get(l as usize)) {
        return item.name.clone();
    }
    if let [key] = state.sources.as_slice() {
        if key == "local" {
            return "My documents".to_owned();
        }
        if key == "search" {
            return "Search results".to_owned();
        }
        if let Some(source) = list.sources.iter().find(|s| &s.key == key) {
            return source.label.clone();
        }
    }
    if state.text.trim().is_empty() {
        "all items".to_owned()
    } else {
        format!("\"{}\"", state.text.trim())
    }
}

/// `place` with what names data no longer fetched dropped (UX notes 0.3): Sources and the second
/// level of the Items view, Sources of the Sets view.
#[must_use]
pub fn restored(list: &BrowseList, mut place: Place) -> Place {
    let known = |key: &String| {
        key == "local" || key == "search" || list.sources.iter().any(|s| &s.key == key)
    };
    place.items.sources.retain(known);
    place
        .sets
        .sources
        .retain(|key| list.groups.iter().any(|g| &g.key == key));
    if place
        .items
        .level
        .is_some_and(|l| l as usize >= list.items.len())
    {
        place.items.level = None;
    }
    place
}

/// The Items place opening a set, catalogue or folder shows (criterion 27): Sources = that one,
/// search empty, Category none, Colour All, Customizable off, sort Name.
#[must_use]
pub fn narrowed_to(source_key: &str) -> BrowseState {
    BrowseState {
        sources: vec![source_key.to_owned()],
        narrowed: true,
        sort: Sort::Name,
        own_sort: Sort::Name,
        ..BrowseState::default()
    }
}

/// The selection after a press on row `key` of a multi-select facet list (criterion 31): a press
/// selects only that row (with its `children`), a press on the only selected row clears the list,
/// Shift adds or removes the row (with its `children`).
#[must_use]
pub fn select_rows(
    selected: &[String],
    key: &str,
    children: &[String],
    shift: bool,
) -> Vec<String> {
    let mut group: Vec<String> = vec![key.to_owned()];
    group.extend(children.iter().cloned());
    if shift {
        if selected.iter().any(|k| k == key) {
            return selected
                .iter()
                .filter(|k| !group.contains(k))
                .cloned()
                .collect();
        }
        let mut out = selected.to_vec();
        out.extend(group.into_iter().filter(|k| !selected.contains(k)));
        return out;
    }
    let mut now: Vec<&String> = selected.iter().collect();
    now.sort();
    let mut wanted: Vec<&String> = group.iter().collect();
    wanted.sort();
    if now == wanted { Vec::new() } else { group }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|k| (*k).to_owned()).collect()
    }

    #[test]
    fn a_press_replaces_the_only_row_clears_and_shift_adds_or_removes() {
        assert_eq!(
            select_rows(&keys(&["a", "b"]), "c", &[], false),
            keys(&["c"])
        );
        assert_eq!(select_rows(&keys(&["c"]), "c", &[], false), keys(&[]));
        assert_eq!(
            select_rows(&keys(&["a", "c"]), "c", &[], false),
            keys(&["c"])
        );
        assert_eq!(
            select_rows(&keys(&["a"]), "c", &[], true),
            keys(&["a", "c"])
        );
        assert_eq!(
            select_rows(&keys(&["a", "c"]), "c", &[], true),
            keys(&["a"])
        );
        // My documents takes its folders with it; Shift on a child moves that child only.
        let kids = keys(&["local:w", "local:f"]);
        assert_eq!(
            select_rows(&[], "local", &kids, false),
            keys(&["local", "local:w", "local:f"])
        );
        assert_eq!(
            select_rows(
                &keys(&["local", "local:w", "local:f"]),
                "local",
                &kids,
                false
            ),
            keys(&[])
        );
        assert_eq!(
            select_rows(
                &keys(&["local", "local:w", "local:f"]),
                "local:f",
                &[],
                true
            ),
            keys(&["local", "local:w"])
        );
    }

    #[test]
    fn the_history_keeps_50_places_and_escape_goes_back_only_after_opening() {
        let mut history = PlaceHistory::default();
        assert!(!history.escape_goes_back());
        for n in 0..60 {
            history.push(
                Place {
                    current: Some(n),
                    ..Place::default()
                },
                false,
            );
        }
        assert_eq!(history.len(), MAX_PLACES);
        assert!(
            !history.escape_goes_back(),
            "a view switch is never undone by Escape"
        );
        history.push(
            Place {
                view: PlaceView::Sets,
                ..Place::default()
            },
            true,
        );
        assert!(history.escape_goes_back());
        assert_eq!(history.back().unwrap().view, PlaceView::Sets);
        assert_eq!(history.back().unwrap().current, Some(59));
        assert!(!history.escape_goes_back());
    }

    #[test]
    fn back_is_named_after_the_place() {
        let list = BrowseList::default();
        let sets = Place {
            view: PlaceView::Sets,
            ..Place::default()
        };
        assert_eq!(back_name(&list, &sets), "Back to Sets");
        assert_eq!(back_name(&list, &Place::default()), "Back to all items");
        let search = Place {
            items: BrowseState {
                text: "arrow".into(),
                ..BrowseState::default()
            },
            ..Place::default()
        };
        assert_eq!(back_name(&list, &search), "Back to \"arrow\"");
        let narrowed = Place {
            items: narrowed_to("local"),
            ..Place::default()
        };
        assert_eq!(back_name(&list, &narrowed), "Back to My documents");
    }

    #[test]
    fn a_restored_place_drops_what_is_no_longer_fetched() {
        let list = BrowseList::default();
        let place = Place {
            items: BrowseState {
                sources: keys(&["set:gone#x", "local"]),
                level: Some(4),
                ..BrowseState::default()
            },
            sets: SetsState {
                sources: keys(&["online:gone"]),
                ..SetsState::default()
            },
            ..Place::default()
        };
        let back = restored(&list, place);
        assert_eq!(back.items.sources, keys(&["local"]));
        assert_eq!(back.items.level, None);
        assert_eq!(back.sets.sources, keys(&[]));
    }
}

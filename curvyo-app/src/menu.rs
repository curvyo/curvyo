//! The menu bar of macOS, the only platform that has one
//! (`specs/0051-sidebar-and-no-menu/` criteria 38 and 39, `adrs.md` decision 6).
//!
//! macOS draws a menu bar that no application can remove, and without a menu of our own Tauri
//! installs a default one whose File, View and Window entries and Cmd+W would take chords away from
//! the page. So there is the application menu, with Quit as a custom item that asks the page about
//! unsaved changes (the predefined Quit would end the app without asking), and an Edit menu of
//! predefined items only: they act on a focused text field, which `WKWebView` otherwise cannot do
//! (it takes its editing key equivalents from menu items). No custom item carries an editor chord,
//! so each chord reaches the page once. Linux and Windows build no menu at all.
//!
//! The table is plain data and the builder compiles everywhere, so the unit tests below check on
//! every platform what the Mac gets.

#![cfg_attr(not(target_os = "macos"), allow(dead_code))]

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Wry};

/// The id of the one custom item.
const QUIT_ID: &str = "quit";

/// A predefined item: a native selector that acts on the focused field, or an application command
/// the system implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Predefined {
    About,
    Hide,
    HideOthers,
    ShowAll,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

/// One row of a menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entry {
    Predefined(Predefined),
    Separator,
    /// An item of ours: `id` comes back in the menu event.
    Custom {
        id: &'static str,
        text: &'static str,
        accelerator: &'static str,
    },
}

/// A menu of the bar.
pub(crate) struct Submenu {
    pub(crate) title: &'static str,
    pub(crate) entries: &'static [Entry],
}

/// The whole bar: the application menu and Edit. Nothing else: no File menu.
pub(crate) const MAC_MENU: &[Submenu] = &[
    Submenu {
        title: "Curvyo",
        entries: &[
            Entry::Predefined(Predefined::About),
            Entry::Separator,
            Entry::Predefined(Predefined::Hide),
            Entry::Predefined(Predefined::HideOthers),
            Entry::Predefined(Predefined::ShowAll),
            Entry::Separator,
            Entry::Custom {
                id: QUIT_ID,
                text: "Quit Curvyo",
                accelerator: "Cmd+Q",
            },
        ],
    },
    Submenu {
        title: "Edit",
        entries: &[
            Entry::Predefined(Predefined::Undo),
            Entry::Predefined(Predefined::Redo),
            Entry::Separator,
            Entry::Predefined(Predefined::Cut),
            Entry::Predefined(Predefined::Copy),
            Entry::Predefined(Predefined::Paste),
            Entry::Predefined(Predefined::SelectAll),
        ],
    },
];

/// Builds the bar from [`MAC_MENU`].
///
/// # Errors
/// Propagates any error Tauri's menu builders return.
pub(crate) fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let mut bar = MenuBuilder::new(app);
    for submenu in MAC_MENU {
        let mut menu = SubmenuBuilder::new(app, submenu.title);
        for entry in submenu.entries {
            menu = match *entry {
                Entry::Separator => menu.separator(),
                Entry::Predefined(item) => menu.item(&predefined(app, item)?),
                Entry::Custom {
                    id,
                    text,
                    accelerator,
                } => menu.item(
                    &MenuItemBuilder::with_id(id, text)
                        .accelerator(accelerator)
                        .build(app)?,
                ),
            };
        }
        bar = bar.item(&menu.build()?);
    }
    bar.build()
}

fn predefined(app: &AppHandle, item: Predefined) -> tauri::Result<PredefinedMenuItem<Wry>> {
    match item {
        Predefined::About => PredefinedMenuItem::about(app, Some("About Curvyo"), None),
        Predefined::Hide => PredefinedMenuItem::hide(app, None),
        Predefined::HideOthers => PredefinedMenuItem::hide_others(app, None),
        Predefined::ShowAll => PredefinedMenuItem::show_all(app, None),
        Predefined::Undo => PredefinedMenuItem::undo(app, None),
        Predefined::Redo => PredefinedMenuItem::redo(app, None),
        Predefined::Cut => PredefinedMenuItem::cut(app, None),
        Predefined::Copy => PredefinedMenuItem::copy(app, None),
        Predefined::Paste => PredefinedMenuItem::paste(app, None),
        Predefined::SelectAll => PredefinedMenuItem::select_all(app, None),
    }
}

/// A click on a custom item: only Quit exists, and it asks the page when there is anything to lose.
pub(crate) fn dispatch(app: &AppHandle, id: &str) {
    if id == QUIT_ID {
        crate::project::request_quit(app);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> impl Iterator<Item = &'static Entry> {
        MAC_MENU.iter().flat_map(|submenu| submenu.entries.iter())
    }

    /// Criterion 39: an application menu and Edit, and no File menu.
    #[test]
    fn the_bar_has_the_application_menu_and_edit_only() {
        let titles: Vec<_> = MAC_MENU.iter().map(|menu| menu.title).collect();
        assert_eq!(titles, ["Curvyo", "Edit"]);
    }

    /// Criterion 38: the only custom item is Quit, with Cmd+Q.
    #[test]
    fn the_only_custom_item_is_quit() {
        let custom: Vec<_> = entries()
            .filter_map(|entry| match entry {
                Entry::Custom {
                    id, accelerator, ..
                } => Some((*id, *accelerator)),
                _ => None,
            })
            .collect();
        assert_eq!(custom, [("quit", "Cmd+Q")]);
    }

    /// Criterion 38: no item carries a chord that the page handles, so each chord reaches the
    /// page once: the file chords and the editor's own keys.
    #[test]
    fn no_item_has_a_chord_of_the_page() {
        let page_chords = [
            "Cmd+N",
            "Cmd+O",
            "Cmd+S",
            "Cmd+Shift+S",
            "Cmd+W",
            "Cmd+Z",
            "Cmd+Shift+Z",
            "Cmd+A",
            "Cmd+U",
            "Cmd+Shift+U",
        ];
        for entry in entries() {
            if let Entry::Custom { accelerator, .. } = entry {
                assert!(!page_chords.contains(accelerator), "{accelerator}");
            }
        }
    }

    /// Criterion 39: the application menu has About, Hide, Hide Others and Show All, and Edit has
    /// Undo, Redo, Cut, Copy, Paste and Select All, all predefined.
    #[test]
    fn the_predefined_items_are_the_ones_the_specification_names() {
        let predefined = |menu: usize| -> Vec<Predefined> {
            MAC_MENU[menu]
                .entries
                .iter()
                .filter_map(|entry| match entry {
                    Entry::Predefined(item) => Some(*item),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            predefined(0),
            [
                Predefined::About,
                Predefined::Hide,
                Predefined::HideOthers,
                Predefined::ShowAll
            ]
        );
        assert_eq!(
            predefined(1),
            [
                Predefined::Undo,
                Predefined::Redo,
                Predefined::Cut,
                Predefined::Copy,
                Predefined::Paste,
                Predefined::SelectAll
            ]
        );
    }
}

//! The native File menu: New, Open…, Save, Save As…, a separator, Quit, and the Edit menu with Undo
//! and Redo (`specs/0020-undo-redo` criterion 8) and the two object steps (`specs/0041-object-history`
//! criterion 5), with OS-standard accelerators
//! bound directly on the menu items
//! (`specs/0001-project-file-foundation/specification.md`, "Keyboard shortcuts
//! (from day one)"). Native menus and accelerators are free accessibility
//! and keyboard traversal on every platform (same spec, "Window chrome and
//! menu").

use tauri::menu::{
    Menu, MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, State, Wry};

const NEW_ID: &str = "new";
const OPEN_ID: &str = "open";
const SAVE_ID: &str = "save";
const SAVE_AS_ID: &str = "save_as";
const UNDO_ID: &str = "edit_undo";
const REDO_ID: &str = "edit_redo";
const OBJECT_UNDO_ID: &str = "edit_object_undo";
const OBJECT_REDO_ID: &str = "edit_object_redo";

/// The event the Edit menu emits to the frontend; the payload is `"undo"`, `"redo"`,
/// `"object-undo"` or `"object-redo"`.
const HISTORY_MENU_EVENT: &str = "history-menu";

/// The Edit menu items whose text and enabled state follow the maker's stacks and selection.
pub struct HistoryItems {
    undo: MenuItem<Wry>,
    redo: MenuItem<Wry>,
    object_undo: MenuItem<Wry>,
    object_redo: MenuItem<Wry>,
}

/// Builds the application menu: File and Edit.
///
/// # Errors
/// Propagates any error Tauri's menu builders return.
pub fn build(app: &tauri::App) -> tauri::Result<Menu<Wry>> {
    let new_item = MenuItemBuilder::with_id(NEW_ID, "New")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_item = MenuItemBuilder::with_id(OPEN_ID, "Open…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let save_item = MenuItemBuilder::with_id(SAVE_ID, "Save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as_item = MenuItemBuilder::with_id(SAVE_AS_ID, "Save As…")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = PredefinedMenuItem::quit(app, Some("Quit"))?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&new_item)
        .item(&open_item)
        .item(&save_item)
        .item(&save_as_item)
        .item(&separator)
        .item(&quit_item)
        .build()?;

    let undo_item = MenuItemBuilder::with_id(UNDO_ID, "Undo")
        .accelerator("CmdOrCtrl+Z")
        .enabled(false)
        .build(app)?;
    let redo_item = MenuItemBuilder::with_id(REDO_ID, "Redo")
        .accelerator("CmdOrCtrl+Shift+Z")
        .enabled(false)
        .build(app)?;
    let object_undo_item = MenuItemBuilder::with_id(OBJECT_UNDO_ID, "Undo Step of Selected Object")
        .accelerator("CmdOrCtrl+U")
        .enabled(false)
        .build(app)?;
    let object_redo_item = MenuItemBuilder::with_id(OBJECT_REDO_ID, "Redo Step of Selected Object")
        .accelerator("CmdOrCtrl+Shift+U")
        .enabled(false)
        .build(app)?;
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(&undo_item)
        .item(&redo_item)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&object_undo_item)
        .item(&object_redo_item)
        .item(&PredefinedMenuItem::separator(app)?)
        .build()?;
    app.manage(HistoryItems {
        undo: undo_item,
        redo: redo_item,
        object_undo: object_undo_item,
        object_redo: object_redo_item,
    });

    MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .build()
}

/// The Edit menu's item text for a step name: "Undo Move", or plain "Undo" when the stack is empty
/// (the item is then greyed out).
fn history_item_text(verb: &str, name: Option<&str>) -> String {
    match name {
        Some(name) if !name.is_empty() => format!("{verb} {name}"),
        _ => verb.to_owned(),
    }
}

/// Sets the text and the enabled state of the Edit menu's Undo and Redo to follow the stacks:
/// `undo` and `redo` are the names of the steps a press would act on, `None` for an empty stack.
/// The two object items are enabled exactly when one object is selected (`object_step`).
///
/// # Errors
/// The menu error as text, when the native menu refuses the change.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn set_history_menu(
    items: State<HistoryItems>,
    undo: Option<String>,
    redo: Option<String>,
    object_step: bool,
) -> Result<(), String> {
    let apply = |item: &MenuItem<Wry>, verb: &str, name: Option<String>| {
        item.set_text(history_item_text(verb, name.as_deref()))
            .and_then(|()| item.set_enabled(name.is_some()))
            .map_err(|error| error.to_string())
    };
    apply(&items.undo, "Undo", undo)?;
    apply(&items.redo, "Redo", redo)?;
    items
        .object_undo
        .set_enabled(object_step)
        .and_then(|()| items.object_redo.set_enabled(object_step))
        .map_err(|error| error.to_string())
}

/// Routes a menu event id to its handler in `main.rs`. Quit is a
/// [`PredefinedMenuItem`] and needs no entry here — Tauri handles it
/// directly.
pub fn dispatch(app: &AppHandle, id: &str) {
    match id {
        NEW_ID => crate::handle_new(app),
        OPEN_ID => crate::handle_open(app),
        SAVE_ID => crate::handle_save(app),
        SAVE_AS_ID => crate::handle_save_as(app),
        UNDO_ID => {
            let _ = app.emit(HISTORY_MENU_EVENT, "undo");
        }
        REDO_ID => {
            let _ = app.emit(HISTORY_MENU_EVENT, "redo");
        }
        OBJECT_UNDO_ID => {
            let _ = app.emit(HISTORY_MENU_EVENT, "object-undo");
        }
        OBJECT_REDO_ID => {
            let _ = app.emit(HISTORY_MENU_EVENT, "object-redo");
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::history_item_text;

    #[test]
    fn the_item_names_the_step_or_stays_plain() {
        assert_eq!(history_item_text("Undo", Some("Move")), "Undo Move");
        assert_eq!(history_item_text("Redo", None), "Redo");
        assert_eq!(history_item_text("Redo", Some("")), "Redo");
    }
}

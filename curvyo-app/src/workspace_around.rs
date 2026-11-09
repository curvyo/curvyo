//! Make a workspace for this file (`specs/0105-sharing-concept` criteria 7 and 8, ADR 0044
//! decision 6): the folder the confirm names, then the folder made, the open file moved into it
//! and opened as a Local workspace with the file still the project. The page saves first.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::project::refresh_window_title;
use crate::state::AppState;
use crate::workspace::{WorkspaceAnswer, error_code, open_root, recents_file};

/// What Make a workspace for this file did: the workspace opened and the file's path in it.
#[derive(Debug, Serialize)]
pub(crate) struct MadeWorkspace {
    answer: WorkspaceAnswer,
    path: Vec<String>,
}

/// The folder the confirm names for the open project's file, as a path to show.
pub(crate) fn folder_for(state: &AppState) -> Result<String, String> {
    let file = state.lock_project().path.clone();
    let file = file.ok_or_else(|| "not-found".to_owned())?;
    let folder = curvyo_storage_io::workspace_folder_for(&file).map_err(|e| error_code(&e))?;
    Ok(format!("{}{}", folder.display(), std::path::MAIN_SEPARATOR))
}

/// Makes the folder, moves the project's saved file into it and opens it as the workspace.
pub(crate) fn make(state: &AppState, recents: Option<&Path>) -> Result<MadeWorkspace, String> {
    let file = state.lock_project().path.clone();
    let file = file.ok_or_else(|| "not-found".to_owned())?;
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| "not-found".to_owned())?;
    let root = curvyo_storage_io::make_workspace_around(&file).map_err(|e| error_code(&e))?;
    let moved = root.join(&name);
    state.follow_move(&file, &moved);
    let answer = open_root(state, recents, &root);
    if matches!(answer, WorkspaceAnswer::Opened { .. }) {
        state.confirm_workspace_opened(moved);
    }
    Ok(MadeWorkspace {
        answer,
        path: vec![name],
    })
}

/// The folder Make a workspace for this file would make (criterion 8).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn workspace_folder_for_file(app: AppHandle) -> Result<String, String> {
    folder_for(&app.state::<AppState>())
}

/// Make a workspace for this file, after the page saved it (criterion 8).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn make_workspace_for_file(app: AppHandle) -> Result<MadeWorkspace, String> {
    let made = make(&app.state::<AppState>(), recents_file(&app).as_deref());
    refresh_window_title(&app);
    made
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::state::ProjectState;

    #[test]
    fn the_file_moves_into_a_new_workspace_and_stays_the_project() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().canonicalize().unwrap();
        let file = parent.join("Lid.curvyo");
        std::fs::write(&file, b"lid").unwrap();
        let state = AppState::default();
        *state.lock_project() = ProjectState::opened(file.clone());
        assert_eq!(
            folder_for(&state).unwrap(),
            format!(
                "{}{}",
                parent.join("Lid").display(),
                std::path::MAIN_SEPARATOR
            )
        );
        let made = make(&state, None).unwrap();
        assert!(matches!(made.answer, WorkspaceAnswer::Opened { ref name, .. } if name == "Lid"));
        assert_eq!(made.path, vec!["Lid.curvyo".to_owned()]);
        let root = parent.join("Lid");
        assert_eq!(state.lock_project().path, Some(root.join("Lid.curvyo")));
        assert!(state.lock_project().in_workspace);
        assert_eq!(state.workspace(), Some(root.clone()));
        assert!(root.join(curvyo_storage_io::MARKER_DIR).exists());
        assert!(!file.exists());
    }

    #[test]
    fn without_a_saved_file_nothing_happens() {
        let state = AppState::default();
        assert!(make(&state, None).is_err());
        assert!(folder_for(&state).is_err());
    }
}

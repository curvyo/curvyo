//! Independent tester cases for `specs/0051-sidebar-and-no-menu/specification.md`, host side, as
//! far as they can run without a window: the capability file guard, the contract between the page
//! and the host (every command the page invokes is registered, every event it listens to is
//! emitted), the window configuration, and "no menu outside macOS" (criteria 24, 27, 37, 38, 41).
//!
//! `curvyo-app` is a binary crate, so these read the sources and configuration files; the pure
//! logic (`window_title`, `AppState::must_hold_close`, the macOS menu table) has unit tests inside
//! the crate, and the commands that need an `AppHandle` are NOT VERIFIED here.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name()
                .is_some_and(|n| n == "node_modules" || n == "dist")
            {
                continue;
            }
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "ts" || x == "tsx") {
            out.push(p);
        }
    }
}

fn frontend_sources() -> String {
    let mut files = Vec::new();
    walk(&root().join("../frontend/src"), &mut files);
    files.sort();
    files
        .iter()
        .map(|f| fs::read_to_string(f).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `"name"` that follows `needle(` or `needle<...>(` in `text`.
fn quoted_after(text: &str, needle: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(i) = rest.find(needle) {
        rest = &rest[i + needle.len()..];
        let mut after = rest;
        if after.starts_with('<') {
            // skip generics (one level of nesting is enough for this code base)
            let mut depth = 0;
            let mut end = None;
            for (k, c) in after.char_indices() {
                match c {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(k + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(end) = end else { continue };
            after = &after[end..];
        }
        let after = after.trim_start();
        if let Some(a) = after.strip_prefix('(') {
            let a = a.trim_start();
            if let Some(j) = a
                .strip_prefix('"')
                .and_then(|a| a.find('"').map(|j| (a, j)))
            {
                out.insert(j.0[..j.1].to_string());
            }
        }
    }
    out
}

#[test]
fn capability_file_lets_the_main_window_listen_and_nothing_broader() {
    let capability: serde_json::Value =
        serde_json::from_str(&read("capabilities/default.json")).unwrap();
    let strings = |key: &str| -> Vec<String> {
        capability[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(strings("windows"), ["main"], "only the main window");
    let permissions = strings("permissions");
    for needed in ["core:event:allow-listen", "core:event:allow-unlisten"] {
        assert!(
            permissions.iter().any(|p| p == needed),
            "{needed} missing: the page would never hear the host"
        );
    }
    for p in &permissions {
        assert!(
            !p.contains("fs:")
                && !p.contains("shell:")
                && !p.contains("http:")
                && !p.contains("process:"),
            "page must not get {p}"
        );
        assert!(
            !p.ends_with(":default") || p.starts_with("core:event"),
            "{p}: broad default permission set"
        );
    }
}

#[test]
fn capability_window_label_matches_the_configured_window() {
    let conf: serde_json::Value = serde_json::from_str(&read("tauri.conf.json")).unwrap();
    let windows = conf["app"]["windows"].as_array().unwrap();
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0]["label"], "main");
    assert_eq!(windows[0]["title"], "Curvyo");
}

#[test]
fn c37_window_config_declares_no_menu() {
    let conf = read("tauri.conf.json");
    assert!(!conf.contains("\"menu\""), "no menu key in tauri.conf.json");
    let main = read("src/main.rs");
    // The macOS menu is built only under cfg(target_os = "macos"); nothing builds a menu elsewhere.
    for (i, line) in main.lines().enumerate() {
        if line.contains("set_menu") || line.contains("menu::build") {
            let before: String = main.lines().take(i).collect::<Vec<_>>().join("\n");
            let gate = before
                .rfind("#[cfg(target_os = \"macos\")]")
                .expect("macOS gate before set_menu");
            // no closing of that gated block in between that would end the macOS scope early:
            assert!(
                i + 1 - before[..gate].lines().count() < 8,
                "set_menu is not within the macOS block (line {})",
                i + 1
            );
        }
    }
    assert!(
        main.contains("#[cfg(any(target_os = \"macos\", test))]\nmod menu;"),
        "menu module is macOS (or test) only"
    );
}

#[test]
fn c38_no_window_builder_or_default_menu_outside_macos() {
    // Tauri 2.12 installs a default menu on macOS only; make sure nothing adds one for the others.
    for rel in [
        "src/main.rs",
        "src/project.rs",
        "src/state.rs",
        "src/formats.rs",
    ] {
        let src = read(rel);
        assert!(!src.contains("Menu::default"), "{rel}");
        assert!(
            !src.contains("MenuBuilder") || rel.ends_with("menu.rs"),
            "{rel}"
        );
        assert!(
            !src.contains("history-menu") && !src.contains("set_history_menu"),
            "{rel}: old edit menu remains"
        );
    }
}

#[test]
fn c38_the_macos_menu_has_no_file_chord_accelerators() {
    let full = read("src/menu.rs");
    let menu = full.split("#[cfg(test)]").next().unwrap().to_string();
    let accelerators: Vec<&str> = menu
        .lines()
        .filter(|l| l.trim_start().starts_with("accelerator: \""))
        .collect();
    assert_eq!(accelerators.len(), 1, "exactly one custom item");
    assert!(accelerators[0].contains("Cmd+Q"));
    // a File submenu does not exist
    assert!(!menu.contains("title: \"File\""));
    for gone in [
        "history_item_text",
        "HistoryItems",
        "set_history_menu",
        "Cmd+Z",
        "CmdOrCtrl+",
    ] {
        assert!(
            !menu
                .replace("//", "")
                .lines()
                .any(|l| !l.trim_start().starts_with("//")
                    && l.contains(gone)
                    && !l.contains("assert")),
            "{gone} still in menu.rs"
        );
    }
}

#[test]
fn every_command_the_page_invokes_is_registered_in_the_host() {
    let page = frontend_sources();
    let used = quoted_after(&page, "invoke");
    assert!(used.len() >= 10, "found {used:?}");
    let main = read("src/main.rs");
    let handler_start = main.find("generate_handler![").unwrap();
    let handler = &main[handler_start..handler_start + main[handler_start..].find(']').unwrap()];
    for command in &used {
        assert!(
            handler.contains(&format!("::{command},"))
                || handler.contains(&format!("::{command}\n")),
            "page invokes {command}, host does not register it"
        );
    }
    // and each registered command is used by the page (no dead command)
    for line in handler.lines().skip(1) {
        let name = line
            .trim()
            .trim_end_matches(',')
            .rsplit("::")
            .next()
            .unwrap_or("");
        if !name.is_empty() {
            assert!(
                used.contains(name),
                "host registers {name}, the page never calls it"
            );
        }
    }
}

#[test]
fn every_browser_stand_in_exists_for_each_command_the_page_invokes() {
    let host_ts = read("../frontend/src/platform/host.ts");
    let page = frontend_sources();
    let used = quoted_after(&page, "invoke");
    for command in used {
        // quit_app is desktop only (the Quit button and Ctrl+Q do not exist in the browser build)
        if command == "quit_app" {
            continue;
        }
        assert!(
            host_ts.contains(&format!("{command}:")),
            "browser build lacks {command}"
        );
    }
}

#[test]
fn every_event_the_page_listens_to_is_emitted_by_the_host() {
    let page = frontend_sources();
    let listened = quoted_after(&page, "listen");
    for event in ["open-bytes", "open-error", "close-requested"] {
        assert!(
            listened.contains(event),
            "page no longer listens to {event}"
        );
    }
    let host = format!("{}{}", read("src/project.rs"), read("src/main.rs"));
    for event in &listened {
        assert!(
            host.contains(&format!("emit(\n                \"{event}\""))
                || host.contains(&format!("emit(\"{event}\""))
                || host.contains(&format!("\"{event}\"")),
            "{event} is never emitted"
        );
    }
    // the old menu events are gone from the page and the host
    for gone in ["history-menu", "menu-new", "menu-open", "menu-save"] {
        assert!(!page.contains(gone), "{gone} still in the page");
        assert!(!host.contains(gone), "{gone} still in the host");
    }
}

#[test]
fn c27_every_exit_route_asks_the_dirty_gate() {
    let main = read("src/main.rs");
    assert!(main.contains("CloseRequested"), "window close");
    assert!(main.contains("ExitRequested"), "OS-requested exit");
    assert!(main.contains("prevent_close") && main.contains("prevent_exit"));
    let project = read("src/project.rs");
    assert!(project.contains("must_hold_close"));
    // the exit the page asks for is not held (no endless loop) and clean projects never wait
    assert!(project.contains("begin_quit"));
}

#[test]
fn c20_c18_dialog_commands_return_results_and_do_not_emit_open_events() {
    let project = read("src/project.rs");
    let pick = project
        .split("pub(crate) async fn pick_project")
        .nth(1)
        .unwrap();
    let body = &pick[..pick.find("\n}\n").unwrap()];
    assert!(
        !body.contains(".emit("),
        "pick_project must return the file, not raise an event (criterion 18)"
    );
    let save = project
        .split("pub(crate) async fn save_project_bytes")
        .nth(1)
        .unwrap();
    let body = &save[..save.find("\n}\n").unwrap()];
    assert!(
        !body.contains(".emit("),
        "save returns its result directly (criterion 19)"
    );
    assert!(
        body.contains("SaveOutcome::Cancelled")
            && body.contains("SaveOutcome::Failed")
            && body.contains("SaveOutcome::Saved")
    );
}

#[test]
fn c42_one_version_source() {
    let ws = read("../Cargo.toml");
    assert!(ws.contains("version = \"0.1.0\""));
    let own = read("Cargo.toml");
    assert!(own.contains("version.workspace = true"));
    let wasm = read("../curvyo-editor-wasm/Cargo.toml");
    assert!(wasm.contains("version.workspace = true"));
    let conf = read("tauri.conf.json");
    assert!(
        !conf.contains("\"version\""),
        "tauri.conf.json must not carry a second version"
    );
    let page = frontend_sources();
    assert!(
        !page.contains("web-demo"),
        "the browser build must not record a different version"
    );
}

# Plan for Sidebar: a vertical tab strip, a File tab, and no menu bar

One branch `story/sidebar-and-no-menu`, one PR, three milestones. No `format_version` change, no new crate,
no new dependency. Acceptance criteria (AC) numbers are those of `specification.md`.

## Affected crates/modules

- `curvyo-ui-core`: `panel_tabs.rs` (File tab, first, sticky), `panel_content.rs` (File body), new
  `file_chords.rs` (chord table, gate-free matching, tooltip key text per platform).
- `curvyo-editor-wasm`: `session/mod.rs` (`saved`/`packed`, `is_dirty`, `mark_saved`, `pack_for_save`,
  `APP_VERSION`), `session/keys.rs` or a new `session/file_chord.rs` (the gate), `session/panel_tabs.rs`
  (File entry, `"file"` content), `wasm_api.rs`, `wasm_keys.rs` (new calls), `Cargo.toml` (`version.workspace`).
- `curvyo-app`: `main.rs` (commands, close and exit handshake, `window_title(file_name, dirty)`), `state.rs`
  (`dirty`, `quitting`), `menu.rs` (milestone 2: no accelerators, one `file-command` event; milestone 3: the
  macOS menu table only), `Cargo.toml` and `tauri.conf.json` (version from the crate).
- Root `Cargo.toml`: `[workspace.package] version`.
- `frontend/`: new `PanelStrip.tsx` (replaces `PanelTabStrip.tsx`), `PropertiesPanel.tsx`, new `FileTab.tsx`,
  new `useProject.ts`, `usePanelOpen.ts`, `lib/panelCollapsed.ts`, `lib/projectTitle.ts`, `lib/platform.ts`,
  `platform/host.ts`, `App.tsx`, `useEditorSession.ts`, `useHistory.ts` (Edit-menu code out), deleted
  `BrowserBar.tsx`.
- Docs: `docs/design-system.md`, amended specs and `adrs.md` of `0043`, `0020`, `0050`, `0041`, `0044`, `0001`.

## Tasks

### Milestone 1: the strip (AC 1 to 14)

- [ ] 1.1 `usePanelOpen` and `lib/panelCollapsed.ts`: collapsed state in `localStorage` key
  `curvyo.panel-collapsed`, every access in `try/catch`, unreadable means shown; unit test with a throwing
  storage (AC 10).
- [ ] 1.2 `PanelStrip.tsx`: 32 px strip (40 px coarse), collapse control, divider, vertical Radix tablist, 28 px
  buttons, 16 px glyphs with absolute 1.5 px stroke, tooltips on the left, scroll area, focus ring inside the
  box, no tab pressed while the body is hidden, Up/Down/Home/End, manual activation while hidden (AC 1 to 9, 11).
- [ ] 1.3 `PropertiesPanel.tsx`: strip + body (`aside`, `hidden` when collapsed), 24 px header row with the tab
  name and the subject line, `keep_view_origin_for_panel_toggle` on show/hide (AC 1, 7, 8, 13).
- [ ] 1.4 Shortcuts Shift+Ctrl+D/F/H show the body without hiding it; `0043`/`0020` tab rules unchanged (AC 14).
- [ ] 1.5 Measure the bars at 800 x 600 (464 px viewport) from the source numbers, report (AC 12).
- [ ] 1.6 Frontend tests: collapsed state, tab list helpers; tooltip-provider test still green.

### Milestone 2: File tab and unsaved changes (AC 15 to 35, 41 to 45)

- [ ] 2.1 Test first, then `file_chords.rs` in `curvyo-ui-core`: `FileCommand`, `Platform`, `file_chord`,
  `chord_label`; table per platform incl. QWERTZ typed character, Cmd+Q None, browser exceptions (AC 32, 34).
- [ ] 2.2 Test first, then `PanelTab::File` first and sticky, `PanelContent::File`; rewrite the five position-based
  tests to find tabs by name (AC 14, 45).
- [ ] 2.3 Test first, then the dirty flag in `Session`: `saved`, `packed`, `pack_for_save`, `mark_saved`,
  `is_dirty`; reference-model sequence test; "Open is clean" for every fixture; New is clean (AC 22, 43, 44).
- [ ] 2.4 Test first, then `Session::file_chord` with the gate (repeat, blocked, drag in flight, open entry chip;
  not the Pen path); `key_down` still ignores Ctrl+S (AC 33).
- [ ] 2.5 One version source: `workspace.package.version`, `APP_VERSION`, `WasmSession::app_version()`,
  `pack()` without argument, `tauri.conf.json` without `version`; test packed manifest equals the constant
  (AC 42).
- [ ] 2.6 wasm API: `is_dirty`, `mark_saved`, `pack`, `app_version`, `file_chord`, `file_command_tooltip`.
- [ ] 2.7 Host: `AppState.dirty/quitting`, `window_title(file_name, dirty)` with test, async commands
  `pick_project`, `save_project_bytes -> SaveOutcome`, `reset_project`, `set_dirty`, `quit_app`,
  `confirm_project_opened`, window close and exit handshake (`close-requested`); menu items lose their
  accelerators and emit one `file-command` event (AC 18, 19, 20, 27, 30, 41).
- [ ] 2.8 `useProject.ts`: mirror of dirty, `requestReplacingAction`, check, New/Open/Save/Save as/Close/Quit,
  chord listener, `beforeunload` while dirty, browser build commands in `host.ts`, title (AC 17 to 21, 25 to 31).
- [ ] 2.9 `FileTab.tsx`: project block, buttons, version line, inline three-button confirm, dirty dot, canvas
  notice "Saved <name>." (AC 15, 16, 23 to 26, 28).
- [ ] 2.10 Inventory checklist of the menu items (AC 36), written into the PR description draft.

### Milestone 3: no menu (AC 37 to 40)

- [ ] 3.1 `menu.rs` reduced to the macOS table (application menu with custom Quit, Edit menu with predefined
  items), built only on macOS; no menu on Linux and Windows; unit test of the table on every platform (AC 38, 39).
- [ ] 3.2 Delete `handle_new/open/save/save_as`, `request_pack`, `RequestPackPayload`, `set_history_menu`,
  `HistoryItems`, the events `new-project`, `request-pack`, `save-error`, `file-command`, the `history-menu`
  listener, `isDuplicateKey`, `pressedJustNow`, `markPressed`, `DUPLICATE_PRESS_MS`, `menuPressPlan`, `BrowserBar`
  (the unsupported-browser message stays) (AC 35, 37).
- [ ] 3.3 Docs: `docs/design-system.md`, "amended by `0051`" in the specs and `adrs.md` listed in the
  specification, `0051` status (AC 40).

## Validation

- Rust: unit tests (chords, tabs, `window_title`, macOS menu table), session tests (dirty reference model,
  fixtures open clean, chord gate), `cargo nextest` for the workspace.
- Frontend: `npm test` (collapsed storage, title rule, tooltips), `tsc -b`, `oxlint`.
- Gate of `CLAUDE.md` section 7 incl. `curvyo-app`, wasm32 build of each touched core crate, boolean budgets.
- By hand in `cargo tauri dev`: strip, collapse, File tab, check, chords, window close with changes. By hand on
  macOS and Windows (listed in the PR description).

# ADRs for Sidebar: a vertical tab strip, a File tab, and no menu bar

First list of points by the product owner (2026-10-11). Reviewed and decided by the architect on 2026-10-11 against ADR 0001, 0004, 0009, 0011, 0014, the `0020` and `0043` decisions and the code on `chore/spec-batch7` (`0043`, `0020`, `0045` built; `0049`, `0050` not built). The decisions below replace the PO's "to decide" points. The spec can go to `Ready` after the PO has handled the items under "Flagged to the PO".

**New ADR: none.** Each trigger the PO named was checked:

- *Dirty baseline:* `Document::version()` exists (Loro state frontiers, `PartialEq`). The flag is session state in `curvyo-editor-wasm`. No new function on `Document` or the history, and ADR 0014 is untouched (decision 2).
- *Host role:* removing the menu narrows the host. Intercepting the window close, setting the title and building the menu that macOS requires are shell work of the kind ADR 0001 §6 gives the host (the title and the menu were there already). The native-menu precedent is in **spec** `0001`, not in an ADR, so no ADR is superseded.
- *Preference store:* one boolean in webview storage, with no file and no host code (decision 8). ADR 0004 covers the files we write, and this is not one of them.
- *macOS menu:* a mechanism inside ADR 0001 §6 (decision 6). It needs a hand check on a Mac, not a platform decision.

## Depends on

- [ADR 0001](../../docs/adr/0001-ui-framework-and-canvas-rendering.md) §1: the tab rule and the chord table stay in Rust (`curvyo-ui-core`); the frontend renders and forwards. §2: Radix `Tabs` with `orientation="vertical"`, Lucide glyphs, no new dependency. §6: the host keeps byte I/O, the file dialogs, the window title and close interception, and the macOS menu. It loses the File and Edit menus.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §1: the `.curvyo` container is unchanged, with no `format_version` bump and no golden-file change.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §2: the active tab, the collapsed state and the dirty flag are view state. They are never written to the document.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §3: no new crate. Tab rule and chord table go in `curvyo-ui-core`, dirty flag and gate in `curvyo-editor-wasm`, and window and dialogs in `curvyo-app`.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): every step is one commit, so the version changes exactly when the document does. An undo is a forward commit, which is why "clean again after undo" is rejected (decision 2). A wipe is a commit (§9), so it makes the project dirty, as criterion 22 says.
- [`0043` adrs.md](../0043-properties-tabs/adrs.md) decisions 4 to 6, and [`0020` adrs.md](../0020-undo-redo/adrs.md) Placement ("`curvyo-app`: the native Edit menu"): both are amended in milestone 3 (see "Amendments to other adrs.md").

## Decisions (architect, 2026-10-11)

1. **Tab rule.** `PanelTab::File` comes first in `TABS` (`curvyo-ui-core/src/panel_tabs.rs`), with name `"file"` and tooltip `"File"`. `observe` needs no new arm: it only switches when Document or Style is active, so File is sticky by construction. New and Open build a fresh `Session` with `PanelTabs::new(false)`, which gives Document (criterion 14). When a check is raised from outside the File tab, the frontend calls `press(File)` and remembers the previous tab. Cancel calls `press(previous)`. If that is refused (Style became dimmed meanwhile), Cancel falls back to `press(Document)`. The collapsed state and "no tab pressed while hidden" (criterion 5) are frontend state, like the panel's `open` today. Tests find tabs by name, never by index (see "Tests at risk").

2. **Dirty flag: session state over `Document::version()`, no core API.** `Session` gets `saved: DocumentVersion` and `packed: Option<DocumentVersion>`:
   - `pack()` (now `&mut self`) records `packed = Some(version)`.
   - `mark_saved()` moves `packed` into `saved`. The frontend calls it only after the host reports `Saved`.
   - `is_dirty()` is `document.version() != saved`.
   - `Session::new` and `Session::open` set `saved` as their last step, so anything open commits counts as saved.

   A change during a slow write stays dirty, and a failed write changes nothing (criterion 22). Read `is_dirty()` only outside a batch.

   **The Proposal "clean again after undo" is rejected; the project stays dirty.** The undo is a forward commit, so the version differs. Three ways to check content equality instead were considered, and none qualifies:
   - A snapshot hash costs time proportional to the document on every undo.
   - A Loro diff between two frontiers needs a new `Document` function, and nobody has checked that a map key set and reset, or a tree move and its move back, gives an empty diff (that needs a spike). It also fails across a wipe.
   - A clean index on the stack (as in `QUndoStack`) is cheap, but wrong once a merge or an object undo (`0041`) changes content off the stack.

   This is the safe direction the spec allows. If the customer asks for it, it becomes a small story with its own spike.

3. **Host commands replace the menu dispatch** (`curvyo-app/src/main.rs`). Every command that opens a dialog is `async`. Today `save_project_bytes` is a sync command and the menu handlers run in the menu callback, so both call `blocking_*` dialogs on the main thread, which `tauri-plugin-dialog` does not allow. This is a latent hang on macOS and Windows that this slice removes. The commands:
   - `pick_project() -> Option<OpenPick>`, where `OpenPick` is `Bytes { path, bytes }` or `IoError { message }`. The dialog and the read return as a result, not as an event, so a file chosen after Discard never re-enters the check (criterion 18).
   - `confirm_project_opened(path)`: unchanged.
   - `save_project_bytes(bytes, save_as) -> SaveOutcome`, where `SaveOutcome` is `Saved { file_name, path }`, `Cancelled` or `Failed { message }`. The `save-error` event goes. A cancelled dialog inside the check keeps the block and the pending action (criterion 26).
   - `reset_project()` (New, Close): path `None`, title updated.
   - `set_dirty(dirty)`: stored in `AppState`, title updated.
   - `quit_app()`: sets `quitting`, then calls `app.exit(0)`.

   `open-bytes`, `open-error` and `take_pending_open` stay, but only for a file the OS hands over. The frontend routes them through the check (a clean project at launch runs none).

   Deleted: `handle_new`, `handle_open`, `handle_save`, `handle_save_as`, `request_pack`, `RequestPackPayload`, the events `new-project`, `request-pack` and `save-error`, `menu::dispatch`, `HistoryItems` and `set_history_menu`.

   The host stays the source of truth for the path (`ProjectState`), and the frontend shows the name and path the last command returned. The title rule is `window_title(file_name, dirty)` in `main.rs`, a pure function with a unit test. The browser build applies the same rule to `document.title` in TypeScript.

   The version comes from one place: `WasmSession::app_version()` (`CARGO_PKG_VERSION`) feeds `pack` and the version line in both builds. `curvyo-app` and `curvyo-editor-wasm` take `version.workspace = true`, and the `version` key leaves `tauri.conf.json` (Tauri then reads Cargo's). A browser-saved file then records the real version instead of `web-demo`.

4. **Close and quit handshake.** `AppState` gets `dirty` and `quitting` (`AtomicBool`):
   - `on_window_event`, `WindowEvent::CloseRequested { api }` (window button, Alt+F4): if dirty and not quitting, call `api.prevent_close()` and emit `close-requested`. Otherwise the window closes.
   - `RunEvent::ExitRequested { api, .. }` (last window closed, OS quit): the same test, with `api.prevent_exit()`.
   - The macOS Quit item calls the same `request_quit` (decision 6).
   - The frontend answers `close-requested` with `requestReplacingAction("quit")` and finishes with `quit_app`.

   A clean project closes without a round trip to the webview, so a hung webview never blocks closing a clean project. Ctrl+Q (Linux) and the Quit button run in the frontend and end in `quit_app`.

5. **One path for replacing actions.** A new frontend hook `useProject.ts` owns the mirror of the dirty flag, the one pending action, the check, and New, Open, Save, Save as, Close and Quit. `requestReplacingAction(kind, payload?)` is the only place that reads dirty and raises the block. Buttons, chords, `close-requested` and `open-bytes` all call it. A second request replaces the pending action (criterion 28). `App.tsx` keeps only the wiring.

6. **The macOS menu (Question 3): the mechanism.**
   - **Linux and Windows:** `set_menu` and `on_menu_event` are gone. Tauri 2 sets no default menu on these platforms, so there is no bar and Alt opens nothing.
   - **macOS:** Tauri 2 installs its default menu when none is set. That menu has File, View, Window and Help, and Cmd+W (Close Window) among its keys, which would take chords away from the page. So `menu.rs` becomes one `#[cfg(target_os = "macos")]` builder over a const table:
     - Application menu: About, Hide, Hide Others and Show All (predefined), a separator, and **Quit** as a custom item with Cmd+Q. It is custom because `PredefinedMenuItem::quit` terminates without asking the frontend, and the custom item calls `request_quit` instead.
     - Edit menu: **predefined** Undo, Redo, Cut, Copy, Paste and Select All.
     - Nothing else.

   **Why the Edit menu has predefined Undo, Redo and Select All.** This contradicts criterion 39 and the default A of Question 3. On macOS, WKWebView gives a key equivalent to the page first and passes it to the menu only when the page did not call `preventDefault`. Predefined items are native selectors sent to the focused field, not Tauri events.
   - On the canvas, the editor takes Cmd+Z and Cmd+A and prevents the default, so the menu never acts.
   - In a text field, the editor ignores them, and the menu performs the field's own undo or select all.

   So there is no event, no listener and no dedupe. Without these items (option A), Cmd+Z and Cmd+A do nothing in text fields on macOS, because WKWebView's editing key equivalents come only from menu items. That fails criterion 39's own hand check, and it is also why Cut, Copy and Paste must be in the menu. Undo chosen with the mouse in the Edit menu acts on a focused field only; document undo stays with the keys and the History tab.

   **By hand on a Mac** (no Mac in CI): the list in criterion 39, plus "Cmd+Z on the canvas undoes exactly one step". If a double fire shows, the fallback is to drop the three items and let the frontend call `document.execCommand("undo" | "redo" | "selectAll")` for a focused text field on macOS. A unit test that runs on Linux checks the table: the only custom item is Quit, and no item has a file chord (decision 7).

7. **Chords: the table and the gate live in Rust.** In `curvyo-ui-core/src/file_chords.rs`:
   - `FileCommand` has the variants New, Open, Save, SaveAs, Close and Quit.
   - `Platform` has Linux, Windows, MacOs and Browser. All four differ today, so the enum is not speculative.
   - `file_chord(key, shift, primary, alt, platform) -> Option<FileCommand>` matches the typed character case-insensitively. Cmd+Q on macOS gives `None` (the menu owns it), and N, W and Q in the browser give `None`.
   - `chord_label(command, platform)` returns the key text for the tooltips (criterion 34), so a tooltip and its chord cannot drift apart.

   `Session::file_chord(input, platform) -> &'static str` adds the gate. It ignores a key repeat, `dom_blocked` (the frontend's busy flag and an IME composition), `operation_in_flight()` and an open entry chip. It does **not** block on an unfinished Pen path (criterion 33). It is a separate method from `key_down`, so `key_down` still returns `Ignored` for Ctrl+S, as two existing tests pin.

   The frontend has one `keydown` listener on `window` in the capture phase (in `useProject.ts`) for every target, text fields included. When the session names a command, the listener calls `preventDefault`, blurs a focused text field (whose blur path commits the value, criterion 32), and then runs the command. The PO's `fileChords.ts` with a TypeScript table is not built.

8. **The collapsed state (criterion 10): webview `localStorage` in both builds.** The key is `curvyo.panel-collapsed`. Every access is in `try/catch`, and a failure means the body is shown. There is no host command, no file in the data directory and no settings store. One boolean for one viewer fits webview storage: all three desktop webviews keep it per OS user, and both builds share one code path. The dev server (`devUrl`) and a release build keep separate values, which is accepted. A file next to `document-formats.toml` waits until a preference must be read by the host or exported.

9. **Browser build.**
   - **Open:** a promise over `<input type=file>`; the `cancel` event resolves to `null`, as `pickTextFile` already does.
   - **Save and Save as:** download `<file name>` or `Untitled.curvyo`. The download counts as `Saved` (the page cannot see a browser save prompt being cancelled).
   - **No File System Access API:** it exists only in Chromium and would add a second save path. That is a later story.
   - **Reload and close:** `beforeunload` is registered only while dirty (registering it always disables the back-forward cache in some browsers). It is registered in **both** builds: on the desktop it catches a webview reload (WebView2's F5 and Ctrl+R), which no host event sees. `quit_app` removes it first.

10. **Layout.** The PO's point 8 is confirmed. The strip is a sibling of the body and outside the element that is hidden. `PANEL_WIDTH_PX` stays 280 for `keep_view_origin_for_panel_toggle`, and the 32 px strip never toggles. DOM order, Tab order and screen order are the same.

## Accepted risks

- **Dirty in the host trails the commit by one IPC call.** A window close in that gap (milliseconds, and only on the first change after a save) closes without the check. To keep the gap small, the frontend sends `set_dirty(true)` in the same task as the re-read after the commit, before the paint.
- **A hung webview with a dirty project:** the window does not close. There is no timer, because a timer cannot tell a hang from a maker reading the sentence. The exit is the OS's force quit. The document is unreachable in a hung webview anyway.
- **macOS** is checked by hand only (decision 6).
- **Milestone 2 on macOS:** the menu without accelerators breaks Cmd+Z in text fields until milestone 3. It is never shipped on its own.

## Flagged to the PO (spec text; defaults taken, the build is not blocked)

1. **Criterion 39 and Question 3:** make B the default with the mechanism of decision 6 (predefined items, no dedupe). Option B's "needs the same dedupe" is wrong for predefined items. Reword "no item with an accelerator that an editor action also uses" to "no custom item with an editor chord; the predefined Undo, Redo and Select All act only when the page did not handle the key".
2. **Criterion 22:** strike the Proposal (decision 2). Criterion 30's test "edit, save, edit, undo the edit" expects `*` after the undo.
3. **Criterion 27:** Linux and Windows have no handover to a running app. A double-click while Curvyo runs starts a second process, because there is no single-instance handling today. Say so, and put single-instance handling under Out of scope.
4. **Criteria 19 and 29 (browser):** a download counts as saved, and a cancelled browser prompt cannot be seen. The download name is the file name or `Untitled.curvyo`; today it is always `untitled.curvyo`.
5. **Criterion 36:** a test over a menu that milestone 3 deletes becomes an empty test. Make the inventory a checklist in the PR. The lasting test is the macOS table of decision 6.
6. **Criterion 38:** "each chord on each OS runs once" can be automated only on Linux CI. Split it: unit tests (decisions 6 and 7), the frontend's single listener, and a hand list for Windows (nightly) and macOS.
7. **Hang case:** add "a hung webview with unsaved changes cannot close its window; the OS force quit can" to Out of scope.
8. **Amendments table:**
   - The `0020` row misses the `0020` adrs.md Placement line ("`curvyo-app`: the native Edit menu") and the dedupe it caused (`isDuplicateKey`, `pressedJustNow`, `DUPLICATE_PRESS_MS`), all deleted here.
   - The `0044` row should say that macOS keeps a native Select All, which acts on text fields only.

## Amendments to other adrs.md (made in milestone 3, each with "amended by `0051`")

- `0043` adrs.md decision 5: vertical Radix `Tabs`; the tablist is outside the collapsible body; `activationMode` is `manual` while the body is hidden (criterion 9). Decision 6: Up and Down replace Left and Right.
- `0020` adrs.md Placement: "`curvyo-app`: the native Edit menu (Q5)" is withdrawn. The frontend loses the `history-menu` listener, the `set_history_menu` call in `refresh`, and `isDuplicateKey`, `pressedJustNow`, `markPressed` and `DUPLICATE_PRESS_MS`. `menuNames` stays, because it feeds the History tab's buttons.

## Tests at risk

Expected to change:

- `curvyo-ui-core/tests/acceptance_0043_tester.rs` (c1: three tabs, `entries[2]`).
- The unit tests in `curvyo-ui-core/src/panel_tabs.rs` (`empty[2]`).
- `curvyo-editor-wasm/tests/acceptance_0043_tester.rs` (`assert_consistent`: `tabs.len() == 3`, `tabs[0]`, `tabs[1]`).
- The tests in `curvyo-editor-wasm/src/session/panel_tabs.rs` (`tabs[1]`, `len() == 3`).
- `curvyo-editor-wasm/tests/history_tab_session.rs` (`tabs.len() == 3`).

Rewrite all of these to find tabs by name.

- `curvyo-app/src/menu.rs` tests: deleted together with `history_item_text`.
- `curvyo-editor-wasm/tests/undo_redo_keys.rs` (`the_buttons_and_the_menu_share_the_gate_of_the_keys`): rename only.
- `frontend/tests/historyText.test.ts` (`menuNames`): stays.

Must pass **unchanged**, because they pin that `key_down` ignores Ctrl+S (decision 7): `curvyo-editor-wasm/tests/edit_polish_escape_keys.rs` and `curvyo-editor-wasm/tests/acceptance_edit_polish_pr1_escape_keys.rs`.

New tests:

- The dirty transitions over a generated sequence (edit, save, edit during a pending save, undo, wipe, open, new) against a reference model.
- "Open, then idle, is clean" for every fixture in `tests/fixtures/`. This catches an open path that commits lazily.
- The `file_chord` table per platform, and the gate.
- `window_title`.
- The macOS menu table.

## Collisions and order

- **Footprint:** `PropertiesPanel.tsx`, `PanelTabStrip.tsx` (replaced), `App.tsx`, `host.ts`, `BrowserBar.tsx` (deleted), `useHistory.ts`, `useEditorSession.ts`, a new `useProject.ts`, `panel_tabs.rs` (ui-core and session), a new `file_chords.rs`, `session/mod.rs`, `session/keys.rs`, `wasm_api.rs`, `curvyo-app/src/main.rs`, `menu.rs`, `state.rs`, `tauri.conf.json` (the `version` key), the two crates' `Cargo.toml` (`version.workspace`), and `docs/design-system.md`.
- **Runs alone among the panel slices.** It conflicts with `0050`, `0044` (the key gate), `0047`, `0049` and `0041`. Build it **before `0050`**. If `0050` merges first, this slice rebuilds the Layers entry and its numbers.
- No `format_version` bump, no new crate, no new dependency. Check at build time that `FolderOpen`, `PanelRightClose` and `PanelRightOpen` exist in the pinned `lucide-react`.

## Delivery: one branch `story/sidebar-and-no-menu`, one PR

1. **Milestone 1, the strip** (criteria 1 to 14): the frontend, the tab list without File yet, the collapsed state in `localStorage`, and the UX measurement of the bars at 464 px.
2. **Milestone 2, the File tab and unsaved changes** (criteria 15 to 36): decisions 2 to 5, 7 and 9; accelerators removed from the menu items. The architect reviews this milestone's branch diff (public session API, host commands).
3. **Milestone 3, no menu** (criteria 37 to 40): decision 6, the deletions of decision 3, the browser bar removed, the documentation, and the amendments.

**Review guide** (the customer reads `.rs`, `Cargo.toml` and docs):

- `curvyo-app/src/main.rs`: the commands, the close and exit handshake, no `set_menu` off macOS, `window_title`.
- `curvyo-app/src/menu.rs`: the macOS table.
- `curvyo-app/src/state.rs`: `dirty` and `quitting`.
- `curvyo-ui-core/src/panel_tabs.rs`: File is sticky.
- `curvyo-ui-core/src/file_chords.rs`: the table.
- `curvyo-editor-wasm/src/session/`: the dirty flag and the chord gate.
- The `version.workspace` lines.
- Docs: `docs/design-system.md` and the amended specs.

## Feature-local decisions

- 2026-10-11 (PO, confirmed): the strip is on the inner edge, next to the canvas. DOM order is natural. The cost is that the strip travels 280 px when the body is hidden. The outer edge stays as option B, reversible at the demo.
- 2026-10-11 (PO, confirmed): unsaved changes are asked inline in the File tab, not in a dialog. The only layers are the OS file dialogs and the two existing error dialogs.
- 2026-10-11 (PO, confirmed): the dirty marker in the title is a leading asterisk.
- 2026-10-11 (PO, confirmed with a different mechanism): the collapsed state is saved per OS user, not per project and not in the file. It is stored in webview storage, not in a data-directory file (decision 8).
- 2026-10-11 (PO, confirmed): no Export, Recent files or start screen.
- 2026-10-11 (architect): decisions 1 to 10 above, with no new ADR.
- 2026-10-11 (architect, review): `curvyo-app/capabilities/default.json` is new: `core:event:allow-listen` and `allow-unlisten`, window `main`. Without it Tauri 2.12 drops every host event. No ADR: Tauri's ACL is host mechanism under ADR 0001 §6. The test in `main.rs` guards it.
- 2026-10-11 (architect, review): the page reads the dirty flag once per animation frame and once synchronously in `request`, instead of after each commit. This is accepted.
- 2026-10-11 (architect, review): the commands live in `curvyo-app/src/project.rs`, and `main.rs` keeps the builder and the handshake. `Session::pack(&self, v)` stays for tests, and `pack_for_save(&mut self)` records the version.
- 2026-10-11 (architect, review): file dialogs are modal to the main window, and replacing actions wait while a dialog or a write runs (review B1). A page that starts, a reload included, tells the host `reset_project` before it takes a file handed over at launch (review B2).

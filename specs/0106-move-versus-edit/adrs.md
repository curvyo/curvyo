# ADRs for move versus edit

Architect, 2026-11-09. Slice 3, part 2 of `0063` (concept A). Frage 164 A, 165 A and 166 A
assumed. No new ADR: the slice changes no document model, no file format and no published
interface; every decision below is session or read-model code, reversible in one PR.

- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) §1: the one membership
  read (`curvyo-ui-core::PageMembership`) is the only input of the context rule (note 2);
  nothing keeps its own overlap test. §2 (a sketch is selected only on its sketch page) holds.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) and
  [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md): the active page's workbench
  (`Session::workbench_of_kind`, Laser (off) and Unknown kind read as Vector) decides the context
  kind; unchanged.
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §7: a
  placed page is an object in the one selection, so it is classified like any object; §5 and §6
  (what a clone offers on its laser page) are the in-context commands of criterion 12.
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): a refused edit writes no step;
  activation by press or double-click is not a step; move commands keep their labels.
- No `format_version` change (stays 38, criterion 21): the last active drawing page, the scope of
  a selection and the laser page tree are session state or derived reads; nothing is written.

## Feature-local decisions

1. 2026-11-09: **What a press on another page's object does today** (code check on `98e10c8e`,
   asked by the lead). It activates that page, by two paths:
   - other workbench (drawing to laser and back, or any page while a sketch page is active):
     `Session::activate_page_of_other_kind_at` (`curvyo-editor-wasm/src/session/pages.rs`) runs
     first in `pointer_down`, activates the page under the press and returns: the press selects
     nothing and starts nothing. The only exception is a drawing object hit over a sketch page
     while a drawing page is active (`0099` criterion 2);
   - same workbench (drawing to drawing, laser to laser): `press_page_view` calls
     `PageViewState::press` (`curvyo-ui-core/src/pages.rs`), which sets `active` to the topmost
     non-sketch frame under the point unless the active frame contains it; the press then selects
     and moves the object, and `pointer_up` repeats the activation. So with A active, a press on
     a rectangle of B makes B active and the rectangle editable.
   A pasteboard press leaves the active page as it is. Criterion 2 changes both paths: each asks
   `press_hits_object` (over every selectable object of every context, note 4) first and
   activates only on a press that hits no object; `PageViewState::press` gets a `hit: bool`
   argument and only records `context` when it is true.
2. 2026-11-09: **The context rule is one pure module, `curvyo-ui-core/src/edit_context.rs`**,
   beside `page_membership.rs` (no I/O, wasm32 as every core crate):
   - `ContextKind { Drawing, Laser, Sketch }`; the session maps each page's kind to it through
     `workbench_of_kind`, so the core function never reads process enablement;
   - `home_of(membership, pages, id, bounds) -> Home`, with `Home::Pasteboard`,
     `Home::Drawing(pages)` (every drawing page it reaches, list order), `Home::Laser(page)` and
     `Home::Sketch(page)` (a sketch's owner page). A laser page wins over a drawing page
     (`0100` criterion 5); placed pages and clones on a laser page are its members by the same
     read. No other rule exists;
   - `in_context(active: (&str, ContextKind), home: &Home) -> bool` is criterion 1's table;
     an object reaching drawing pages A and B is in context on either (Proposal);
   - `scope(active, homes) -> Scope { Edit, MoveOnly }`: `MoveOnly` when any object is out of
     context (Frage 165 A); an empty selection is `Edit`;
   - `double_click_target(active, home, last_drawing, pages) -> DoubleClickTarget { AsBuilt,
     Activate(page), NoDrawingPage }` is criteria 13 to 16 (the first page in list order for a
     `Home::Drawing` of several, Proposal);
   - one table test of criterion 1's test project (seven rectangles, four active pages) and one
     of the double-click targets.
3. 2026-11-09: **Every commit label is classified, unknown labels are edits.**
   `curvyo-ui-core/src/step_scope.rs` holds `MOVE_STEPS` (criterion 5: `translate_objects`,
   `duplicate_objects`, the resize, rotate, skew, transform, flip, align and distribute labels,
   `delete_objects`, `linked_clone`, `place_on_page`, `place_linked_clones`, `place_page`),
   `FREE_STEPS` (not about the selection: page, manufacturing, profile and fixture labels,
   creation and import labels, `set_node_visible`, `set_node_locked`, layer add and delete,
   document settings, history versions, sketch labels, which only run on the active sketch page)
   and `EDIT_STEPS` (the rest), and `step_scope(label) -> StepScope { Move, Free, Edit }` that
   returns `Edit` for a label in none. The guard test asserts every label of
   `history_names::OPERATION_NAMES` is in exactly one list (criterion 9's "every command id is
   classified"); the existing names guard already fails a label missing from that table.
   `arrange_grid` is Move (criterion 5 names it; it writes one translation per selected unit);
   `move_nodes` (row drag, z-order) and Z-order are edits because criterion 5 does not list them.
4. 2026-11-09: **The gate is one feature-owned method: `Session::recording` itself**
   (architecture review finding 1). New module `curvyo-editor-wasm/src/session/edit_context.rs`:
   - one `Session` field `edit_context: EditContext`, fields private to the module: the cached
     scope keyed by (document version, selection, active page id) and `last_drawing_page`. The
     session calls only `EditContext::scope(&self, read) -> Scope`, `note_activated(page,
     kind)`, `double_click_target(..)` and `out_of_context_line(..)`;
   - `recording` becomes the guarded write: it asks `scope()` once before the body and returns
     `R::default()` without running the body when it is `MoveOnly`. Commands of `MOVE_STEPS` and
     `FREE_STEPS`, and the input dispatchers (`pointer_down`, `pointer_move`, `pointer_up`,
     `double_click_hint`, the key handler, the Layers eye and lock), call
     `recording_unguarded`, the old body. A new command is therefore guarded unless someone opts
     it out (criterion 9's default);
   - the second net is in `record_new_steps`: a step whose `step_scope` is `Edit`, recorded while
     the scope before the body was `MoveOnly`, is a `debug_assert!` failure, so an edit reached
     through an unguarded dispatcher fails every debug test that hits it;
   - the edit tools never see out-of-context objects: the Node tool, Text tool hit and Pen join
     read `edit_targets()`, the in-context part of `pickable_objects()`; Select keeps the whole
     set. The Style tab and Select bar views read `scope()` once per view and hide the edit
     sections (criterion 11); the muted line's words come from a ui-core text function with the
     70 and 24 character cuts.
   Rejected: (a) a check at each of the 121 `recording` call sites or the 94 `selection.ids()`
   reads, which is the scatter the review warns of and silently misses a new command; (b)
   refusing at the document commit, which cannot work because the Loro operations are applied
   before `commit_with_label` and a revert writes new operations (not byte-identical, criterion 9).
5. 2026-11-09: **Last active drawing page is session state in `EditContext`.** Set by
   `activate_page` for a page whose context kind is Drawing; read through `pages` with the
   fallback of criterion 14 (first drawing page in list order, else `NoDrawingPage`). It is not
   `last_work_page` (which also takes laser pages, for `sketch_use.rs`); both stay. Never saved.
6. 2026-11-09: **Selection rules of criterion 3 live in the session's Select dispatch**, not in
   `select_tool.rs` (2,707 lines): Ctrl+A filters `page_selection_ids` by `in_context` (it replaces
   today's `page_view.context` scoping); marquee and Shift+press stay context-blind; the
   sketch/drawing exclusivity is one check in `sketch_pointer_down`'s caller.
7. 2026-11-09: **The laser page tree is a second builder of the existing Layers read model**
   (Frage 166 A). `curvyo-ui-core/src/page_tree_rows.rs` builds `LayerRows` for a laser page:
   page root row, placed pages and own items in draw order topmost first, then one Fixtures row.
   `RowKind` gains `PlacedPage`, `Fixtures` and `PageRoot`; `LayerRow` gains `via:
   Option<NodeId>` (the placed page an inner row is shown through, so one source placed twice has
   distinct rows) and `read_only: bool`; `index_of`/`row` key on `(via, id)`. Inner rows are built
   only for expanded placed pages, so the flattening stays O(visible + own items) and the window,
   `aria-*` and roving focus of `0050` are reused unchanged (criterion 20; `0050` decision 7).
   `layers_panel.rs` picks the builder by the active page's context kind; drawing and sketch
   pages keep `LayerRows::build`. Rejected: a separate tree type and frontend list, which
   duplicates the virtualisation and re-proves the `0050` budgets for no gain; option B of
   Frage 166 (document layers with placed pages added), which mixes pages in one list.
8. 2026-11-09: **Budgets of criterion 22 confirmed**, thread CPU in
   `curvyo-editor-wasm/tests/cpu_time/`: (a) `scope()` over 1,000 selected objects on 10 pages,
   cold cache, 16 ms (one `PageMembership::of` and one `home_of` per object); (b) press, marquee
   step and double-click 16 ms each; (c) drag of 1,000 out-of-context objects within 10 % of the
   in-context drag per pointer move (the scope is cached for the drag, so the move path is the
   same). Criterion 20's 100 ms first rows uses the `0050` measurement.
9. 2026-11-09: **Session size.** New code goes to `session/edit_context.rs` and
   `session/page_tree_panel.rs` (the laser tree's JSON and presses); `mod.rs` gets only the field
   and the `recording`/`recording_unguarded` split; `pages.rs` only the `hit` argument of note 1.

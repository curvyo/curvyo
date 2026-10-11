# Plan for Groups (0023)

One branch `story/groups`, one PR, opened when the maker can group, nest, enter, leave and ungroup in
the app and the full local gate is green (`CLAUDE.md` §7, the frontend checks, the budget tests).
Milestones are commits on the branch; none is a PR. `format_version` goes from 10 to **11**.

## Design notes

- **A group is a tree node with `group: true`** and nothing else (`adrs.md`). `Document::object_ids()`
  keeps meaning "every object that has geometry, in paint order": it now walks the tree depth-first
  and lists the **leaves** only. Every reader that wants the flat paint list (render, bounds, Fit to
  content, export) therefore stays as it is. `Document::object(id)` is `None` for a group, as
  `Document::path(id)` is `None` for a primitive.
- **The selection stays a set of leaf ids.** A group is selected when all its leaves are. Nothing that
  already works on a multi-selection (group box and handles of 0019, move, nudge, typed values,
  style, Node tool, delete) changes. What is new is the *unit*: the object of the current context
  that a leaf belongs to. A click, Shift-click, marquee, lasso, Alt-click cycle and hover resolve a
  hit leaf to its unit and select or toggle all of the unit's leaves.
- **The context is session state** (`Session::context`, a stack of group ids). `Session::objects()`
  returns only the leaves inside the entered group, so everything the Select and Node tools do is
  inert outside it by construction. Rendering takes the whole list and draws the outside at 35 %.
- **Commands that copy or place objects are unit-aware** in `document-core`: `duplicate_objects`
  deep-copies a group, `delete_objects` and every replace command remove a group that lost its last
  child in the same commit, new objects are created in the group the Session has entered.
- **Undo needs the parent and z-position restore** that `specs/0020-undo-redo/adrs.md` (review notes)
  assigns to this slice: `history/restore_plan.rs` learns moves, and revival places a node under its
  old parent.

## Affected crates/modules

- `curvyo-document-core`: new `groups.rs` (tree reads, group, ungroup, depth, emptied groups);
  `objects.rs` (deep duplicate and delete), `replace.rs`, `path_topology.rs` (empty-group cleanup),
  `paths.rs`/`shapes.rs` (creation parent), `path_codec.rs` (validation), `document.rs` (format 11,
  `document.json`), `history/` (`restore_plan.rs`, `restore.rs`, `revive.rs`, `checkout.rs`,
  `report.rs`: `ObjectShape::Group`). Golden fixtures in `tests/fixtures/`.
- `curvyo-ui-core`: new `context.rs` (the unit map and context stack), `grouping.rs`
  (availability and refusals of Group, Ungroup and the other commands), changes in `select_tool*`
  (units for click, toggle, marquee, lasso, cycle), `object_selection.rs`, `style_panel.rs` (subject
  line), `history_names.rs`, Escape cascade.
- `curvyo-render-core`: dimming of the artwork outside the context, unit boxes (one box per selected
  unit, hover box of a unit), context readout data.
- `curvyo-editor-wasm`: `session/grouping.rs` (Group, Ungroup, context, enter, leave, notices),
  `session/mod.rs`, `select*.rs`, `keys.rs` (Ctrl+G, Ctrl+Shift+G, Enter, the Escape step),
  `wasm_group.rs`.
- `frontend`: the rail (Boolean card moves to column B, Group card in column A), Group and Ungroup
  glyphs, tooltips, the "Open groups" indicator in the canvas notice slot, notices, the hint chip,
  key handling.
- Docs: `specs/README.md` (format plan, status), `specs/0023-groups/specification.md` (status line),
  `docs/design-system.md` only where the build differs.

## Tasks

### Milestone 1: model, commands, file format (AC 1 to 5, 13, 14, 25, 27)

- [x] 1.1 Tree reads: `children`, `parent`, `is_group`, `leaves_of`, `object_ids` as leaves in paint order,
  `object`/`path`/`primitive` are `None` for a group (AC 25)
- [x] 1.2 `group_objects` and `ungroup` with the 32-level limit, order rule, one commit each (AC 1, 3, 4, 5)
- [x] 1.3 Empty groups removed in the same commit by delete, replace, break apart, split, last-node
  delete (AC 14); a guard test lists the commands
- [x] 1.4 Deep duplicate with new ids for group, descendants and anchors; deep delete (AC 13)
- [x] 1.5 Creation inside the entered group (`Document::set_draw_parent`) (AC 19)
- [x] 1.6 Open-file validation (group shape, depth > 32, child of a leaf), `format_version` 11, golden
  files, older files byte-identical after Open, `document.json` nests groups (AC 27)

### Milestone 2: undo and redo of group steps (AC 6 history, `0020` review note)

- [x] 2.1 Restore moves (parent and z-position) in `restore_plan.rs`, revival under a parent, `ObjectShape::Group`
- [x] 2.2 Tests: Group, Ungroup, nested delete with emptied group, duplicate of a group; undo and redo
  restore `export_json` exactly

### Milestone 3: selection, context, tools in `ui-core` (AC 8 to 12, 15 to 22, 28 to 30)

- [x] 3.1 `context.rs`: unit map, context stack, fallback (AC 8, 15, 20)
- [x] 3.2 Click, Shift-click, hover, marquee, lasso, Alt-click cycle by unit (AC 8, 9, 10)
- [x] 3.3 Group and Ungroup availability and commands; refusals of Boolean, Combine, Split, Break apart,
  Object to path for groups (AC 2, 28, 29)
- [x] 3.4 Enter, double-click leave, Escape step (AC 15, 17, 18)
- [x] 3.5 Subject line, Node tool, Pen targets, style on groups (AC 11, 12, 26, 30)
- [x] 3.6 Operation names `group` and `ungroup` (history names guard)

### Milestone 4: drawing (AC 15, 16, 22)

- [x] 4.1 Dimming outside the context; group box only, no member boxes; unit hover box (the hover hint chip "Double-click to open" is not built)

### Milestone 5: session and wasm API (AC 6, 7, 15 to 21)

- [x] 5.1 `Session` context, `group`/`ungroup`/`enter`/`leave`, notices, keys (Ctrl+G, Ctrl+Shift+G, Enter,
  Escape step), gate, wasm facade

### Milestone 6: frontend (AC 2, 6, 7, 15 to 18)

- [x] 6.1 Rail: Boolean card to column B, Group card in column A, glyphs, tooltips
- [x] 6.2 Notices, the "Open groups" indicator, hint chip, keys, announcements; frontend tests

### Milestone 7: gate and docs

- [x] 7.1 Full local gate, wasm builds, boolean budgets, spec status and format plan, PR draft

## Validation

Unit tests first in each core crate; two-replica and undo tests for the history engine; golden
fixtures `group_v11.curvyo`, `group_too_deep_v11.curvyo`, `group_leaf_child_v11.curvyo`; the
older-file test (a v10 file opened and saved byte-identical as in 0040); frontend `npm test`,
`tsc`, `oxlint`; the Boolean budget tests; wasm32 builds of every touched `-core` crate.

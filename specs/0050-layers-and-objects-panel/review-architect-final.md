# Architect review: 0050 final (M1 fixes, M2 to M6)

Date: 2026-10-12. Branch `story/layers-and-objects-panel`, diff `024150e..30b617d` (a237bb3 fix,
c266292 M2, 3521f23 M3, cd4926a M4, a7cbea2 M5, 30b617d M6). Rust only; the TypeScript is not
reviewed. Read against `specification.md`, `adrs.md`, `plan.md`, ADR 0015, `CLAUDE.md` §5/§6 and
`review-architect-m1.md`.

Checked locally: `cargo clippy -p curvyo-ui-core -p curvyo-render-core -p curvyo-editor-wasm
--all-targets -D warnings` clean; `cargo nextest run` on document-core, ui-core, render-core and
editor-wasm 5705 passed, 62 skipped; `cargo build --target wasm32-unknown-unknown` of
`curvyo-document-core`, `curvyo-ui-core` and `curvyo-render-core` clean.

## Verdict

**APPROVE WITH CHANGES.** The M1 items are all resolved. Dependency direction, core purity and
the file format are untouched by M2 to M6: no new crate, no new dependency, no `Cargo.toml` or
`Cargo.lock` change, no I/O, clock or thread in a core crate, no trait, no generic. Two must-fix
items, both in `curvyo-editor-wasm`, both small; neither touches the format or a core API.

## M1 items 1 to 9

| # | Item | State |
|---|---|---|
| 1 | Group and Ungroup accept layers | Resolved: `ObjectEditError::IsALayer`, checked before the first write in `group_objects` and `ungroup`; tested |
| 2 | Record the M1 decisions in `adrs.md` | Resolved: note of 2026-10-12 under decision 8 (but see should-fix 6) |
| 3 | Undecodable leaf read as a group | Resolved: `read_node` returns `None`, the node and its subtree are left out, doc comment says so |
| 4 | `move_nodes` keeps descendants of moved nodes | Resolved: `checked` filters with `is_or_under`; tested (`moving_a_group_with_one_of_its_members_keeps_the_member_inside`) |
| 5 | Capped ancestor walks before `unwrap` | Resolved: caps removed in `is_or_under` and `locked_through`, with the reason (a Loro tree has no cycle) |
| 6 | `parent_node` in `node_move.rs` | Resolved: moved to `group_tree.rs` next to `parent_group` |
| 7 | `delete_objects` accepts a layer | Resolved: refuses with `IsALayer`; tested |
| 8 | `next_layer_name` is `pub` | Resolved: `pub(crate)`, re-export removed |
| 9 | Deep-tree test missing | Resolved: `the_read_of_a_deep_chain_carries_flags_and_opacity_to_the_bottom` (32 levels, flags and opacity at the bottom) |

## Must fix

1. **The Opacity preview recomputes the effective opacity in the session, at O(n x t)**
   (`curvyo-editor-wasm/src/session/layers_edit.rs`, `previewed_drawn`).
   Rules: ADR 0015 §6 and `adrs.md` decisions 3 and 4 (the effective values come from the one
   read pass; consumers do not compute them again). Also §5, one responsibility: the binding crate
   should not own a render rule.
   - The function multiplies opacity down the tree a second time, next to `Document::read_nodes`.
     When `0049` adds its clone rule (criterion 28: the root original's opacity), `read_nodes`
     changes and the preview does not, so a drag shows a value the commit does not draw.
   - `targets.contains(&node.id)` is a `Vec` lookup per node. With 10,000 selected objects the
     preview costs about 10^8 comparisons per pointer move during an Opacity drag (criterion 26,
     "live while dragging").
   - Fix: move the function to `curvyo-render-core/src/node_opacity.rs` as a pure function
     next to `drawn_leaves`, e.g. `drawn_leaves_with(reads: &[NodeRead], targets:
     &HashSet<NodeId>, opacity: Opacity) -> Vec<ObjectSnapshot>`. Give its doc comment a link
     to `read_nodes`, so that whoever changes the rule changes both. Add one test that it equals
     `drawn_leaves` when `opacity` is each target's own value. The session keeps only the call.
2. **The hover box of a Layers row is O(n x k) per frame** (`curvyo-editor-wasm/src/session/
   select_view.rs`, the `self.panel.hover` block). Rule: criteria 48 to 52 (frame budgets) and
   `adrs.md` decision 7 (panel work is O(window), not O(n²)).
   - `leaves.contains(&object.id())` is a `Vec` lookup for every object of the frame.
   - Hovering a group row of 5,000 leaves in a 10,000-object document costs about 5 x 10^7
     comparisons on every frame while the pointer rests there.
   - `layer_hover_leaves` also walks the tree (`Document::leaves_of`) on every frame.
   - The M6 budget test does not cover hovering.
   - Fix: build a `HashSet<NodeId>` once per frame, or cache the leaves with `panel.hover` in
     `set_layer_hover`. Add the 5,000-leaf group hover to `layers_budget.rs`.

## Should fix (in this branch, not blocking the tester)

3. **`run_reorder` discards a refusal inside the batch** (`session/layers.rs`,
   `let _ = document.move_nodes(plan)`).
   - Every plan is checked against the state before the first move, and then the plans run one
     after another. The comment "a refusal here would change nothing" is wrong: a refusal of plan
     2 would leave plan 1 written in the step.
   - With the selection held in one context the plans are independent today (one per parent,
     targets among that parent's siblings), so nothing breaks now.
   - Fix: replace the `let _` with an `// invariant:` comment that states the independence, and
     add a `debug_assert!(result.is_ok())`, or check each plan right before it runs and stop
     there.
4. **Words of the panel outside ui-core** (`session/layers_panel.rs`, `row_json`'s `inherited`
   closure builds "Hidden by layer Cut"). Rule: ui-core owns the words (`adrs.md` decision 14,
   `layer_text.rs` doc comment). `layer_text::by` already builds the same phrase. Fix: add a
   `pub fn inherited_line(row) -> (String, String)` (or two functions) to `layer_text.rs` and
   call it.
5. **The Pen gate has three entry points instead of one** (`adrs.md` decision 9: "the one place
   where panel actions enter the session").
   - `delete_layer_row` checks `pen_path_unfinished` by hand on its object branch and calls
     `delete_selected` outside `panel_action`.
   - `preview_layer_opacity` checks it inline.
   - Fix: run the object branch of `delete_layer_row` through `panel_action`. The preview writes
     nothing, so its inline check is acceptable; say so in its doc comment.
6. **`adrs.md` and `plan.md` name functions that do not exist.**
   - The note of 2026-10-12 and decision 14 say ui-core `plan_move`. The code has
     `reorder_plans` (gesture to `MovePlan`s) and `move_refusal_text`.
   - Decision 10 says `artwork.rs`. The code is `node_opacity.rs` (`faded`, `drawn_leaves`).
   - `plan.md` tasks 15 and 17 and the Validation section repeat `plan_move`.
   - Fix: the architect corrects `adrs.md` (one line each) on the lead's go. The implementer
     corrects `plan.md`.
7. **`LayerRows::build` is about 100 lines** (`curvyo-ui-core/src/layer_rows.rs`). Rule: §5,
   functions over ~60 lines need a reason in the PR or get split. Fix: move the index/children/
   `below_selected` preparation into a private `Tree::of(nodes, selected)` and the row
   construction into `fn row(...)`. The other option is a reason in the PR's Review guide.
8. **The Select tool filters and clones the pickable objects again on each press, marquee and
   lasso** (`select_tool.rs`, `gesture.rs`: `self.units.pickable(objects)`).
   - The session already caches the same list in `Held::pickable`.
   - `Cow::Owned` clones every `ObjectSnapshot` of the context as soon as one node is inert:
     10,000 clones per press.
   - The result is right; there are just two producers of one list.
   - Fix: pass the cached `pickable` slice from the session into the Select tool calls, as
     `classify_press` already takes it, and drop `Units::pickable`'s callers in the tool. If no
     caller is left, remove the method.
9. **`Units` now has two jobs** (`curvyo-ui-core/src/units.rs`). Grouping the leaves of a context
   and knowing the inert set are related, but the module doc comment still describes only the
   first. Rule: §5, one sentence per module. Fix: reword the module doc to one sentence that
   covers both ("which objects of the context a gesture reaches, and as which unit"), or move
   the inert set into a small `Inert` newtype that `Units` holds. Rewording is enough.
10. **Unknown strings at the wasm boundary are handled two ways** (`wasm_layers.rs`).
    - `press_from` and the `flag` match fall back to `Plain` and `Visible`. `reorder_from` and
      `DropPlace::from_name` refuse an unknown string.
    - `reorder_layer_rows` hand-writes `{"kind":"","text":""}`.
    - Fix: give `RowPress` and `RowFlag` a `from_name -> Option<Self>` next to `DropPlace`'s.
      An unknown string then returns `PanelReply::Quiet.to_json()`.

## Public API

- **ui-core.**
  - `classify_press(objects, pickable, ...)`: confirmed. The handles and boxes of the selection
    need every object, and hits need the pickable ones; two slices are clearer than a filter
    closure. See item 8 for the other caller.
  - `Units::with_inert`, `is_inert`, `without_inert`: confirmed, each has a session caller.
  - `pickable`: see item 8.
  - `PanelTab::Layers` and `PanelContent::Layers`: confirmed, they follow the History pattern.
  - `layer_move`: confirmed. It holds no rule; refusals come from `Document::check_move`, as the
    M1 note requires.
  - `layer_rows`: confirmed. `LayerRows::build` + `window` is decision 7. `percent` is exported
    for the session's Opacity header; fine.
  - `layer_text`: confirmed. Plain functions returning `String`, and every name is plain text
    (criterion 17).
- **render-core.** `faded` and `drawn_leaves`: confirmed. They are pure, use `Opacity` and no
  bare `f64` in signatures, and the multiplication carries an `// invariant:` note. Paints are
  multiplied one by one (decision 10, Question 5 A).
- **editor-wasm.**
  - `PanelReply`, `RowPress`, `RowFlag`, `DropPlace`, `KeyOutcome::Reordered` and the Layers
    session methods: confirmed as the minimum the tab needs.
  - `wasm_layers.rs`: confirmed. It passes straight through, with strings and scalars only
    (ADR 0001 §5); only the window crosses (criterion 48).
  - `DocumentRead` (session cache) holding nodes, leaves, visible, drawn and inert, built once
    per version: confirmed. It is decision 3 and 4 as written, and the plain document shares one
    `Arc` for all three leaf lists.

## Engineering rules

- **Newtypes:** opacity is `Opacity` in every new public signature. Bare `f64` remains only at the
  wasm edge (percent from the host) and in the private preview accumulator (item 1 moves it).
- **unwrap:** no new `unwrap`/`expect` outside tests. `unwrap_or` fallbacks on `Opacity::new`
  have invariant notes, except in `previewed_drawn` (item 1).
- **Module size:** new production modules are 150 to 425 lines; tests are split into
  `layers_panel_tests.rs` and `layers_tests.rs`. Only `LayerRows::build` is over the function
  limit (item 7).
- **Dependency direction:** ui-core and render-core depend only on document-core, as before.
  editor-wasm calls render-core `drawn_leaves` and `faded`, which is the existing direction.
- **Budgets:** `layers_budget.rs` is `#[ignore]`d and run by hand in release, as `0019` and
  `0047` do; the numbers are recorded in `plan.md`. Accepted, with the hover case added (item 2).

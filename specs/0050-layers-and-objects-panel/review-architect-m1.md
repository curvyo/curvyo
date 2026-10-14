# Architect review: 0050 milestone M1 (document model and file format)

Date: 2026-10-12. Branch `story/layers-and-objects-panel`, diff `origin/main...HEAD` (8f71417 plan,
25676a6 M1). Read against `specification.md`, `adrs.md`, `plan.md`, ADR 0015, `CLAUDE.md` §5/§6.

Checked locally: `cargo clippy -p curvyo-document-core -p curvyo-ui-core --all-targets -D warnings`
clean, `cargo nextest run -p curvyo-document-core` 1372 passed, `cargo build --target
wasm32-unknown-unknown -p curvyo-document-core` clean.

## Verdict

**APPROVE WITH CHANGES.** The stored shape, format 12, the damage rules and the read pass are
right and are what `adrs.md` decision 1 and ADR 0015 §2, §4, §6 ask for. Three must-fix items
before M2 builds on this; none changes the file format.

## Must fix

1. **`group_objects` and `ungroup` accept layers** (`curvyo-document-core/src/groups.rs`).
   Rule: `adrs.md` decision 1, "a layer is at the top level" (format, protect this), and the
   reason `check_move` lives in core (rules once, in the crate that writes).
   - `group_objects(&[layer_a, layer_b])` passes every check (both are top-level siblings), puts
     both layers into a new group, and the `layer` markers become residue: two layers silently
     turn into plain groups.
   - `ungroup(&[layer])` passes `is_group_node` and dissolves the layer.
   - Fix: refuse before the first write when any id is a layer (`is_layer_node`). Add one
     variant, e.g. `ObjectEditError::IsALayer` ("a layer cannot be grouped or ungrouped"), or
     reuse `NotAGroup` for `ungroup`; one test per function.
2. **Record the M1 decisions in `adrs.md`** (rule: `CLAUDE.md` §4, `adrs.md` holds the decisions;
   `plan.md` "Decisions taken in this plan" is not the record). `adrs.md` decisions 8 and 14 still
   say the move rules live in ui-core `plan_move`. Add one dated note (architect writes it on the
   lead's go, or the implementer does):
   > **2026-10-12 (amends decisions 8 and 14).** The rules of criterion 30 live once in
   > `curvyo-document-core` `Document::check_move`; `move_nodes` runs it before the first write.
   > ui-core `plan_move` only turns a gesture into a `MovePlan` and asks `check_move`, and maps
   > `MoveRefusal` to the texts of criterion 32. `parent_group`, `ancestor_groups` and
   > `child_groups(None)` treat layers as the top level (criterion 42); `parent_node` and
   > `children` give the raw tree. Layers count toward the 32-level limit of `0023` criterion 5,
   > in the reader and in every writer alike, so a layer holds at most 31 nested groups.
3. **`read_nodes` reports an undecodable leaf as a group** (`curvyo-document-core/src/node_read.rs`,
   `read_node`, the `None => NodeKind::Group` arm). Rule: ADR 0015 §6 (one read that every
   consumer trusts); `NodeKind` is new public API. A non-group node whose object does not decode
   (only merge residue can produce it, open refuses it) becomes a phantom "Group" row in the
   panel and a container in the read. Fix: skip the node, as `json_children` already does with
   `self.object(id)?`, and say so in the doc comment.

## Should fix (in this branch, not blocking M2)

4. **`move_nodes` does not drop descendants of moved nodes** (`node_move.rs`, `checked`).
   Moving a group together with one of its members pulls the member out of the group.
   `delete_objects` already filters with `has_ancestor_in`; do the same before
   `in_stacking_order`.
5. **Capped ancestor walks before `unwrap`** (`node_move.rs` `is_or_under`, `node_read.rs`
   `locked_through`). Both stop after `MAX_GROUP_DEPTH * 4` steps and then answer "no"; on a
   merged tree deeper than 128, `move_nodes` would reach `mov_after(...).unwrap()` with a cyclic
   move and panic. Not reachable before sharing (open refuses depth > 32). Fix: drop the cap
   (a Loro tree has no cycle, so the walk ends) or map the tree error to `MoveRefusal`.
   Otherwise add one line to `docs/technical-debt.md` next to the deep-merge item of ADR 0015.
6. **`parent_node` sits in `node_move.rs`** (rule §5, one responsibility per module). It is a tree
   read; move it to `group_tree.rs` next to `parent_group`, so the two parent functions and their
   different layer rules are read side by side.
7. **`delete_objects` accepts a layer id** and so bypasses `delete_layer`'s lock check. The
   selection never holds a layer (criterion 3), so it is not reachable from the UI today; refuse
   layers there too when doing item 1, it is the same three lines.
8. **`next_layer_name` is `pub` with no caller outside the crate** (rule §5, no speculative API).
   Make it `pub(crate)` unless M3 calls it.
9. **Plan task 2 is checked but its test is half there.** `layers_model.rs` has the wide case
   (10,000 in a layer); the deep case (a 32-level chain) of "O(n) test over a deep and a wide
   tree" is missing. Add it or reword the task.

## The six questions

1. **Note of 2026-10-12 superseding decision 11: confirmed.** `document.json` is the
   non-authoritative view (ADR 0004 §1); no format version depends on it. Keeping `0023`'s
   `{"shape": "group", "objects": [...]}` avoids renaming a merged view, and one `shape` key for
   leaves, groups and layers is the simpler form. `"shape": "layer"` plus the optional `name`,
   `visible: false`, `locked: true`, `opacity < 1` keys, each left out at its default, is checked
   by `document_json_writes_layers_and_registers_away_from_their_defaults`. No key clashes with
   leaf fields (`style` holds `fill_opacity`/`stroke_opacity`; no leaf has `name` or `visible`).
2. **Format v12: confirmed.**
   - Stored meta keys `name` (string), `visible`, `locked` (bool), `opacity` (number in 0..=1),
     and `layer: true` on a top-level group: as decision 1 and ADR 0015 §2. Absent reads as the
     default, so v11 files open unchanged and are not rewritten (oplog version vector compared).
   - Damage rules: wrong type, non-finite or out-of-range opacity, `layer` on a leaf are
     damaged; `layer` below the top level reads as a plain group. Matches ADR 0015 §4. Accepting
     an integer `0`/`1` for `opacity` is a harmless leniency for hand-edited files.
   - Names are cleaned on read for display and on write; the stored string is left as it is
     (tested). Correct per decision 1 and criterion 17.
   - Golden files: `layers_v12.curvyo` (layer with group, hidden node, locked layer, opacities,
     markup name, loose object), `layers_opacity_damaged_v12.curvyo`,
     `layers_marker_in_group_v12.curvyo`: exactly the three of decision 15. Regeneration only
     with `CURVYO_WRITE_FIXTURES`; the golden test also compares against the code-built document,
     so a drift in either fails.
3. **Public API: confirmed with items 3, 6 and 8.** `read_nodes`, `node_registers`, `is_layer`,
   `parent_node`, `rename_node`, `set_node_visible/locked/opacity`, `new_layer`, `delete_layer`,
   `check_move`, `move_nodes`, and the types `MovePlan`, `MoveTarget`, `MoveRefusal`, `NodeRead`,
   `NodeKind`, `NodeRegisters` are the minimum M2 to M4 need: no trait, no generic, each
   function has a named caller in the plan. `ObjectEditError::Locked` and `NotALayer` are
   specific and documented on the functions that return them. `MoveRefusal` as its own enum is
   right: the UI needs the reason, and its `thiserror` messages are not UI strings (ui-core
   owns those).
4. **Move rules in document-core `check_move` instead of ui-core `plan_move`: confirmed.**
   Decision 8 already required `move_nodes` to re-check the same rules before writing; keeping a
   second copy in ui-core would be two lists that drift. The rules are domain invariants (layer
   at top level, depth, lock), so they belong next to the write. Recorded by must-fix 2.
5. **Layers as the top level for `parent_group`/`ancestor_groups`/`child_groups`: confirmed.**
   Without it, `Units::build` (ui-core `units.rs`) would treat a whole layer as one selectable
   group; with it, criterion 42 holds and no ui-core walker is written (decision 4). Internal
   `group_parent` keeps the raw parent, so `group_objects`, `ungroup`, `set_draw_parent` and the
   move targets work inside layers. **`delete_layer` sweeps empty groups: confirmed.** It calls
   `remove_emptied_groups(&[None])`, which only removes empty non-layer groups at the top level
   (merge residue), in the same commit, as every `0023` delete does; undo restores them with the
   layer. **Layers count toward the 32-level limit: confirmed.** Reader (`validate_tree`) and
   writers (`group_objects`, `check_move`) count the same way, which is what matters; no v11
   file is affected (it has no layers). Relaxing it is the sharing story's (ADR 0015
   Consequences).
6. **Edits to the tester's tests (0017, 0023, 0047 part A): acceptable.** All three are
   mechanical consequences of the bump and keep each test's intent: 0017 lists the new damaged
   fixture among the files that must not open; 0023 AC27 now checks "writer declares the current
   version, current + 1 is refused" and skips the `layers_*` fixtures, which are not files of
   earlier builds and are covered by `layers_format.rs`; 0047 still pins "no bump of its own" at
   the new number. The tester confirms them in Verify, as for any test touched outside the
   tester's own pass.

## Engineering rules

- **Units:** opacity is `Opacity` (the existing newtype of `style_model`) in every public
  signature and field (`NodeRegisters`, `NodeRead`, `set_node_opacity`). Bare `f64` appears only
  in the private `Inherited` accumulator and the `document.json` serialization struct; both are
  acceptable at those edges.
- **Module size and responsibility:** the five new modules are 125 to 242 lines, each with a
  one-sentence doc comment; functions are under 60 lines. Only `parent_node` is misplaced
  (item 6).
- **No unwrap without invariant:** every `unwrap` in new code is under `#[allow]` with an
  `// invariant:` comment and a `# Panics` section. Item 5 names the one invariant that rests on
  a capped walk.
- **Dependency direction:** no new crate, no new dependency; document-core gains no I/O and
  still builds for wasm32; ui-core only gains seven operation names.
- **Golden files for the format:** present (§5).

# Architect review: 0049 linked clone, milestones 2 to 6

Date: 2026-10-14. Branch `story/linked-clone`, diff `ea415a7..HEAD` (fbc7964 M1 fixes, e718657 M2,
ed7dd06 M3, 6024903 M4, b25f586 M5, 38906bc M6). TypeScript not reviewed. Read: `adrs.md`,
`plan.md` (measured numbers of milestone 6), the new modules `clone_copy.rs`, `clone_repair.rs`,
`clone_transform.rs`, `clone_write.rs`, the changes in `clone_read.rs`, `object_read.rs`,
`objects.rs`, `transform_objects.rs`, `history/restore.rs`; in ui-core `clone_gesture.rs`,
`clone_panel.rs`, `clone_marks.rs`, `linked_clone.rs`, `group_transform.rs`,
`transform_drag.rs`; in render-core `artwork.rs`, `clone_marks.rs`; in editor-wasm
`session/clones.rs`, `session/clone_view.rs`, `session/draw.rs`, `tests/clone_budget.rs`; the
three changed tester tests; `ci.yml`; and `node_read.rs` and `render-core/src/node_opacity.rs` on
`origin/story/layers-and-objects-panel` (0050). The wasm32 build of `curvyo-document-core`,
`curvyo-ui-core` and `curvyo-render-core` passes (checked).

## Verdict

**CHANGES REQUESTED.** The model, the writers, the repair, undo and the merges follow ADR 0015 and
`adrs.md`; the M1 fixes are in. Five must-fix items (F1 to F5): one is the missed criterion 44,
three are reads of the whole document or quadratic loops where the design names a cheaper source,
one is a test for the repair ids. No new crate, no new dependency, no trait or generic in
product code, no `unwrap` without `// invariant:`. Core purity holds.

## M1 fixes: verified

1. F1: the golden test compares every stored origin and matrix bit of `clone_v12.curvyo` with
   `GOLDEN_LINKS`, plus literal resolved points. Done.
2. F2: `assemble` orphans a clone with a non-finite mapped outline and, in depth order, every
   clone behind it; unit test with 43 links of 1e7. Done.
3. F3: `read_one_clone` collects its chain and calls `resolve`; `object(id)` equals the read entry
   for every leaf of the golden and the orphan file (`clone_format.rs` line 429). Done.
4. F4: `outline_transforms` is gone; a test-only thread-local counter in `CloneSnapshot::new`.
   Done.
5. F5: two residue tests (`ac38_a_link_on_a_path_is_ignored_residue`,
   `ac38_outline_and_style_keys_on_a_clone_are_ignored_residue`). Done.

Should-fix items of M1: `&roots[&root]` is gone, `IdParts` is two free functions, the plan names
`CURVYO_WRITE_FIXTURES`, `resolve` returns one result per leaf, `leaf_kind` exists. Still open:
the doc comment on `Document::object_ids` (`paths.rs`) does not say that it lists orphan clones,
for which `object()` returns `None` (S6).

## Decisions on the five questions

### 1. Criterion 44: build the shared root tessellation now

Decided: **build it in this slice**, as a milestone 6b commit. Measured: 181 to 234 ms, of which
about 85 ms tessellate 10,000 copies of the same 100-node outline. Decision 3 named this step.

How (`adrs.md` decision 3, amendment of today):

- `curvyo-document-core`, `CloneChain` gets `pub root_outline: Arc<PathSnapshot>`, the root's own
  snapshot as read (unmapped), `#[serde(skip)]`. `assemble` makes one `Arc` per root and every
  clone of that root shares it. `CloneChain` loses `Copy`; `mapped_by` clones the chain and keeps
  the `Arc`. Invariant, documented on the field: `resolved`'s anchors are `root_outline`'s under
  `composed`. `mapped_by` and `clone_previews` keep it. The box stays, so no `ObjectSnapshot` grows.
- `curvyo-render-core`, new module `shared_outline.rs`: the tessellation of a clone root, made
  once per `build_artwork` call and mapped onto each clone that may share it. `build_artwork` keeps
  its signature. It keeps a per-call map from `Arc::as_ptr(root_outline)` to the root's fill and
  stroke triangles in root coordinates. A clone uses the map when all three hold, and is otherwise
  tessellated from `resolved` as today:
  (a) the linear part of `composed` is rigid (orthonormal within 1e-9; reflections allowed);
  (b) the root's stroke has no dashes and no markers, so the frame budgets for dashes and markers
  are charged exactly as today;
  (c) `resolved.style` equals `root_outline.style` apart from the two paint opacities.
  The mapped vertices get `composed` applied to their positions and the clone's own paint colours
  (`paint(fill.color, fill.opacity)`, same for the stroke). The layers are the same as today: fill
  then stroke, per object, in stacking order. A rigid map keeps distances, so the flattening
  tolerance in mm holds. The non-zero and even-odd rules ignore orientation, so a mirrored fill is
  correct.
- Tests in render-core: for translation, rotation 0.6 rad, a reflection, a compound root with a
  hole, an open filled path and a faded clone, the shared and the unshared build give the same
  layer count, colours, bounding box and triangle area per layer within 1e-6. A test-only counter
  shows one root tessellation for 1,000 rigid clones. A scaled clone, a dashed root and a clone
  with a different style use the old path.
- `tests/clone_budget.rs` (editor-wasm): criterion 44's 150 ms is asserted in release. `plan.md`
  gets the breakdown of the frame (read, artwork, marks, decorations). If the frame is still over
  150 ms, the implementer removes per-frame full copies of the object list in `draw_list` first
  (`paths`, `editable`). If it is then still over, stop and report the breakdown; criterion 44
  goes to the PO before the PR. No instancing in this slice.
- Compatibility with 0050: nothing changes in `read_nodes` or `drawn_leaves`. `read_nodes` gets its
  clones from `resolve`, so the `Arc` is in each `NodeRead`'s `ObjectSnapshot`. `faded` clones the
  snapshot, which copies the `Arc` cheaply, and changes only the opacities of `resolved.style`. Rule
  (c) lets that clone share, and the clone's own paint colours carry its effective opacity. A
  hidden root is missing from `drawn_leaves` but is still inside each clone's `Arc`, so its clones
  share too. `drawn_leaves_with` is the same case.

Rejected: (1) GPU instancing (one root triangle list plus per-instance matrices in the
`DrawList`). It is the cheapest per frame, but it changes the draw-list contract with the host
(ADR 0001 §5), the TypeScript renderer and the per-layer depth rule. It stays the next step if
CPU sharing is not enough. (2) A tessellation cache across frames keyed by object version. The
commit changes the root, so all 10,000 clones miss on exactly the frame criterion 44 measures. (3)
Reusing the root's draw from the object list instead of an `Arc`. A hidden root is not in the list,
and the Node tool's live drag substitutes the root's path, which would move the clones with an
uncommitted drag.

### 2. Repair anchor ids in a reserved half: accepted

`(peer, 2^63 | n)` with `n = max(op count of the peer, last used)` is unique. The peer is fresh
per session, which Loro needs anyway. The minter counts from 0 and never reaches 2^63. Anchor ids
are stored as 128-bit hex text and never cross the wasm boundary as a JS number, so the top bit
loses nothing. The alternative, minted ids passed into Delete, every Boolean, Break apart, Split
and the history restore (and into 0037, 0038 and 0054 later), changes about ten public signatures
for a case the caller cannot size before the command runs. Reusing the root's anchor ids breaks
the global uniqueness of ADR 0002. Conditions: F5, and the dated note in `adrs.md` decision 5,
which records the reserved half as the one exception to "the caller mints".

### 3. The gesture's map read back, `transform_objects` signature kept: half accepted

The `transform_objects` signature stays: the document core needs only the settled results.
Reading G back as `M_result · M_stored⁻¹` is not accepted (F3). The caller has G exactly:
`map_affine` of the `GroupMap`. Reading G back loses precision for an ill-conditioned stored M
(entries up to ±1e7, |det| just above 1e-9). It also hides the rule "one G per commit", and it
reads the whole document at every commit.

### 4. Three tester tests skip the orphan fixture: accepted

Criterion 35 says the Open of an orphan file removes the orphans in one step and marks the
project changed, so "opens clean with an empty history" cannot hold for that file. The positive
side is tested (`ac35_an_open_removes_orphans_in_one_step_with_a_notice`: notice, dirty, no
orphan left). Should-fix S3: skip by the exact fixture name, from one constant, not by the
substring `"orphan"`, so a later fixture with that word is not silently skipped.

### 5. New public API

- `curvyo-document-core`: `Document::read_objects`, `linked_clone`, `unlink_clones(&[CopySource])`,
  `remove_orphan_clones`, `take_clone_repair` with `CloneRepair`; `ObjectRead`, `clone_previews`;
  `CloneLinks`, `CloneMatrices`, `clone_matrices_after`; `CloneError` (thiserror);
  `UNLINK_CLONE_COMMIT_LABEL`, `REMOVE_ORPHAN_CLONES_COMMIT_LABEL`; `ObjectShape::Clone`;
  `PathSnapshot::all_anchors_mut`. Accepted. Two additions: `Document::clone_links()` becomes
  public (F3), and `CloneChain::root_outline` (F1). `clone_previews` stays in the document core
  because it uses the one resolver. `take_clone_repair` is a side channel: accepted with S2.
- `curvyo-ui-core`: `clone_marks` with `CloneMarks`, `MarkFrame`, `MarkRect`, `MarkTrace`,
  `MarkTag`, `EdgeChip`, `MarkRole`, `MarkForm` and the eight caps; `clone_block`, `CloneBlock`,
  `followers_of`, `object_name`, `select_clones`, `select_originals`, `select_root_originals`;
  `plan_linked_clone`, `LinkedClonePlan`, `linked_clone_shown`, `MAX_REFUSAL_OUTLINES`. The
  gesture helpers are `pub(crate)`. `MarkRect` in screen px as `f64` follows the viewport
  convention of the crate. Accepted, but the panel functions take the wrong input (F4).
- `curvyo-render-core`: `build_clone_marks`, `CloneTrace`, `theme::CLONE_LINK`. Accepted.

## Must fix (F1 to F5)

**F1. Criterion 44 missed.** Build the shared root tessellation as decided above (render-core
`shared_outline.rs`, `CloneChain::root_outline`) and assert 150 ms in release.

**F2. The limit check reads the whole document at every transform commit.**
`clone_write.rs` `clone_limits_hold`. It is called by `transform_objects`, `translate_objects`,
`rotate_object` and the resize as soon as one clone exists. Rule: decision 3 (one read per version)
and criterion 43. It reads every leaf through `raw_leaf_of`, all anchors included, resolves the
whole document and maps every clone's outline. In a document with one clone and 10,000 ordinary
paths, every nudge then pays a second full read: `plan.md` measures that read at seconds for
10,000 paths of 100 nodes. Fix: build the reverse map from `clone_links()` (metas only), walk the
clones affected by `changed`, and collect the leaves of their chains (the clones and the roots).
Read only those, then call `resolve` on that list, as `read_one_clone` and `clone_previews` do.
That costs O(links + affected), with no anchors read for unaffected paths. Add a test with a
test-only counter: a nudge of one path in a document with one clone and 1,000 paths reads one root
and no other path.

**F3. The gesture reads G back and the whole document.** `clone_gesture.rs`
`settle_clone_matrices`. Rule: decision 4 ("one pure function ... serves ui-core"), and the same
read cost as F2. Fix: `settle_clone_matrices(links: &CloneLinks, results, g: Affine)`.
`commit_group` takes `g: Option<Affine>`: `group_drag.rs` and `group_entry.rs` pass
`map_affine(map)`, and `transform_commit.rs` passes `None`, because a single result never moves
its own chain. The links come from a public `Document::clone_links()`, not from `read_objects()`.
Test: a stored M = diag(1e6, 1e-6) with O and the clone moved by (100, 0) keeps the drawn outline
within 1e-6 mm.

**F4. The panel data is quadratic and ignores `clones_of`.** `session/clones.rs`
`clone_panel` runs `followers_of(&objects, &[id])` once per selected id, each a scan of all
objects. `followers_of`, `select_clones` and `selected_clones` use `selected.contains` on a slice.
`object_name` builds a map of all objects on every call. Rule: decision 3 ("Select clones, the
origin line, the Layers rows ... read `clones_of`") and criterion 61 ("the Style tab's counts stay
exact" for 10,000 clones). Ctrl+A over 10,000 clones gives about 10^8 comparisons per panel query;
at milestone 7 the Layers rows name 10,000 clones, which is O(n²) again. Fix: the session's object
cache keeps the whole `ObjectRead` (`objects` and `clones_of`, at M7 the node reads and
`clones_of`). The ui-core panel functions take `&ObjectRead`, or `clones_of` and an id index, and
a `HashSet` of the selection. `object_name` takes the index. Test: `clone_panel` over a Ctrl+A of
10,000 clones within 5 ms in the release budget test.

**F5. The repair ids have no uniqueness test.** `clone_repair.rs` `repair_anchor_ids`. Rule:
ADR 0002 §5 (globally unique anchor ids), plus my condition for accepting question 2. Fix: one test
in one session. A Delete repair, an undo whose restore repairs, and a second Delete repair: all
ids distinct, all with the top bit, and none equal to an id from the session's minter. Add
`debug_assert!(self.next < 1 << 63)` to `AnchorIdMinter::mint` with a comment naming the reserved
half.

## Should fix

- S1. `history/restore.rs`: the repair gets only deletes without children
  (`children_num == 0`). `repair_clones_of_removed` expands subtrees itself, so pass every deleted
  id. Then a restore that removes a group whose leaves are not separate actions still repairs.
- S2. `take_clone_repair` keeps its count until the host reads it. A restore or Boolean that
  repairs, followed by a Delete that repairs nothing, shows the old count. Reset it at the start of
  each command in `Session::recording`, and test that case.
- S3. The three tester skips: one constant with the exact fixture name (question 4).
- S4. `clone_read.rs` is 320 production lines with two jobs, the resolver and the preview of the
  clones downstream of an edit. Move `clone_previews` to `clone_preview.rs` (§5, one
  responsibility per module).
- S5. `ci.yml`: the job `boolean-budgets` ("boolean budgets (release)") now runs the clone
  budgets. Rename it to `performance-budgets` ("performance budgets (release)") and add 0049 to
  its comment. The four new lines themselves are right. They are the release-only route of my M1
  answer 7, and `clone_history ac53` runs 1,000 sequences there and 60 in debug.
- S6. `Document::object_ids` doc: it lists orphan clones, and `object()` returns `None` for them
  (M1 question 5 b).
- S7. `objects.rs` duplicate: `sources.iter().position(..)` inside the loop over the waiting
  copies is O(s²) for a Duplicate of 10,000 clones with their root. Use an index map.
- S8. `render-core/src/clone_marks.rs`: `objects.iter().find` per mark, twice (casing, line), is
  up to 600 scans of the whole list per frame. Pass an index or the marked snapshots.
- S9. `plan.md` milestone 6: criterion 43's "first frame after opening within 1 s" is not measured,
  and its "one node dragged, previews off" is measured as a nudge of the whole original. Add the
  first one, or say why the nudge stands for the drag.

## Section 5 and 6 checks

- Traits, generics, crates, dependencies: none added. Pass.
- `unwrap`: every production `unwrap` in the diff has an `// invariant:` comment with
  `#[allow(clippy::unwrap_used)]`. Pass.
- Module sizes: new modules are under 500 production lines. `clone_marks.rs` (ui-core) is 383
  production lines and `clone_read.rs` 320 (S4). `objects.rs`, `paths.rs`, `path_topology.rs`,
  `select_tool.rs` and `node_tool.rs` were already over and grew little. No new split is required
  by this slice.
- Units: matrices follow the `Point`/`Vec2` convention (M1); screen px are `f64` as elsewhere in
  ui-core. Tolerances explicit (`SINGULAR_DETERMINANT`, ±1e7 limits). Pass.
- Core purity: no filesystem, clock, thread or UI in the three core crates. The repair reads the
  Loro op count, which is document state, not a clock. The wasm32 build passes. Pass.
- Removal guard: `clone_repair_guard.rs` reads the source tree from a test, which is fine for a
  test. The two exemptions (`groups.rs`, `clones.rs`) carry their reason. Pass.
- Session import: decision 6 names "after `import_updates`". The session has no import path yet, so
  there is nothing to call. The sharing story adds it.

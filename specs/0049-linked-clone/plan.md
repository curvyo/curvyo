# Plan for Linked clone (0049)

One branch `story/linked-clone`, one PR, opened when the maker can make, transform, follow, unlink
and delete clones and chains in the app, the Layers parts of `0050` included, and the full local
gate is green (`CLAUDE.md` §7, the frontend checks, the budget tests). Milestones are commits on
the branch; none is a PR. `format_version` goes to **13** (see "Format version").

Started 2026-10-14 from `main` at `b564ac5` (format 11). Not merged at that point: `0050`
(PR #118, format 12), `0054`, `0055`, `0038` (Offset), `0037` (Fracture, Flatten). Merged and
built on: `0019`, `0020`, `0023`, `0041`, `0044`, `0047`, `0048`.

## Design notes

- **A clone is its own read variant**, `ObjectSnapshot::Clone(CloneSnapshot)` (`adrs.md` decision
  2), so the compiler lists every match site (about 50 today, most in `curvyo-ui-core`). Readers
  that draw, hit, measure or feed the kernel go through `resolved: PathSnapshot`; writers get
  `Planned::Clone`, which writes only `clone_link`.
- **The chain resolver is a pure, iterative function** in `curvyo-document-core`
  (`clone_read.rs`): input the raw leaves in stacking order (paths, primitives, raw links), output
  the resolved list, `clones_of` and the orphan set. On `main` it is called from a new
  `Document::read_objects()`, which the session's object cache calls instead of its own
  `object_ids().filter_map(object)` loop. `0050` replaces that loop with `read_nodes()`; after
  `0050` merges, `read_nodes()` calls the same resolver for its leaves and adds the clone's
  effective opacity (milestone 7). Keeping the resolver independent of the tree walk is what makes
  that rebase a call-site change, not a rewrite.
- **One function for transforming a set**, `clone_matrices_after(read, selected, g)` (decision 4),
  in `clone_transform.rs`, used by the ui-core preview and by every document writer that moves a
  set.
- **The clone read is two parts** (2026-10-14, M1): `CloneSnapshot { resolved: PathSnapshot, chain:
  Box<CloneChain> }`, where `CloneChain` holds `origin`, `root`, `depth`, `matrix` and `composed`
  (the fields of decision 2; the id is `resolved.id`). Kept flat, the variant made every
  `ObjectSnapshot` 304 bytes instead of 168 and tripped `large_enum_variant` in two ui-core
  enums; behind the box a clone is 160 bytes and no other type changes. Readers match
  `ObjectSnapshot::Path(path) | ObjectSnapshot::Clone(CloneSnapshot { resolved: path, .. })`.
- **After the M1 review** (2026-10-14, `review-architect-m1.md`): `resolve` is the only place that
  follows links and returns one result per input leaf in input order; `Document::object` on a clone
  collects its chain's leaves and calls it. A clone whose mapped outline is not finite is an
  orphan, with every clone behind it. The criterion 63 count is a test-only counter in
  `CloneSnapshot::new`. Criterion 39 at full size runs in the release job
  (`tests/clone_hostile_sizes.rs`).
- **Milestone 2** (2026-10-14): the gesture layer maps a clone's drawn outline by the gesture's
  map G (read back from the path transform through a three-point probe, `clone_gesture.rs`) and
  gives it `G·M`; `commit_group` settles the matrices with `clone_matrices_after` before
  `transform_objects` writes only `clone_link`. `translate_objects`, `rotate_object` and the
  document resize apply the same rule themselves. A clone whose chain moves too gets its unlinked
  path at the press (minted like a converted primitive), used when G is singular. The repair on
  removal gives unlinked clones anchor ids in a reserved half of the counter space,
  `(peer, 2^63 | n)`, because Delete, the Booleans and undo take no minted ids (deviation from
  "the caller mints", for the architect). The counts for the notice are read with
  `Document::take_clone_repair`. Criterion 17 is checked by `transform_objects`,
  `translate_objects` and `rotate_object`; node edits of a root are not checked yet.
- **`Affine` gets its own module `affine.rs`**, not `units.rs` as decision 4 says: `units.rs` is
  already 558 lines (`CLAUDE.md` §5, ~500). Same crate, same public name; noted for the architect.
- **Removal repair is one helper**, `repair_clones_of_removed`, called next to every
  `remove_emptied_groups` call (today: `objects.rs`, `replace.rs`, `path_topology.rs`, `paths.rs`,
  `path_extend.rs`, `groups.rs`) and from the history restore. A guard test lists the callers of
  both helpers and fails when they differ, so `0037`, `0038`, `0054` and `0050`'s Delete layer
  cannot add a removal without the repair.
- **Commands not on `main` yet** (Offset `0038`, Fracture and Flatten `0037`): this branch covers
  the commands on `main` at its last rebase. A slice that merges later reads clones through the
  `resolved` arm (the compiler forces the match) and its removals through the guard test. The
  Linked clone button goes last in the Select bar; after Offset once Offset exists.
- `0020` is merged, so the notices of criterion 5 never carry "No undo yet.".

## Affected crates/modules

- `curvyo-document-core`: new `affine.rs`, `clone_read.rs` (resolver, `clones_of`, orphans),
  `clone_transform.rs` (`clone_matrices_after`, limit check), `clones.rs` (Linked clone, unlink,
  repair on removal, orphan repair, relink of copies); changes in `shape_codec.rs` (`clone`),
  `path_codec.rs` (open-file validation), `primitive_model.rs` (variant), `transform_objects.rs`
  (`Planned::Clone`), `objects.rs` (translate, rotate, duplicate, delete), `replace.rs`,
  `path_topology.rs`, `paths.rs`, `path_extend.rs`, `groups.rs` (group transform, deep duplicate),
  `document_size.rs` (resize, Fit to content), `shapes.rs` (conversion on clone), `document.rs`
  (format, `document.json`, `read_objects`), `history/restore.rs`, `history/restore_plan.rs`,
  `history/report.rs` (`ObjectShape::Clone`). Golden fixtures in `tests/fixtures/`.
- `curvyo-ui-core`: new `linked_clone.rs` (availability, depth check, refusal data), `clone_marks.rs`
  (decision 12); changes in every `ObjectSnapshot` match site (`group_transform.rs`,
  `transform_commit.rs`, `transform_drag.rs`, `param_edit.rs`, `param_handles.rs`, `conversion.rs`,
  `hit_test_object.rs`, `style_scope.rs`, `style_panel.rs`, `select_bar.rs`, `select_tool/*`,
  `object_bounds.rs`, `boolean.rs`, `combine.rs`, `split.rs`, `cut.rs`, `break_apart.rs`,
  `node_tool.rs`, `pen_target.rs`, `segment_bend.rs`, `history_names.rs`).
- `curvyo-render-core`: the clone arm in `artwork.rs` and the previews, new `clone_marks.rs` (traces
  and rings; tags and chips are DOM).
- `curvyo-editor-wasm`: new `session/clones.rs` and `wasm_clone.rs` (commands, notices, Select
  original, root original and clones, panel block data, marks); `session/object_cache.rs`,
  `session/document.rs` / `project.rs` (orphan repair after Open and import), `select_bar.rs`,
  `node.rs`, `style.rs`, `draw.rs`, `select_readout.rs` (announcer text).
- `frontend`: Linked clone button in `SelectToolbar.tsx`, a `CloneBlock.tsx` and origin line in the
  Style tab, notices, canvas tags and edge chips, the `--clone-link` token in `index.css`.
- Docs: `specs/README.md` (format plan, status), `specification.md` (status line, PR link),
  `docs/design-system.md` only where the build differs.

## Tasks

### Milestone 1: model, read, file format (AC 1 to 3 core, 7 to 9, 37 to 41, 50, 63)

- [x] 1.1 `Affine`: compose, apply to point and vector, `inverse() -> Option` (None below |det|
  1e-9), finiteness and ±1e7 checks; unit tests (AC 12, 16, 50)
- [x] 1.2 Codec and validation: `shape = "clone"`, one `clone_link` map value; damaged cases (no
  link, bad id, not six numbers, non-finite, beyond ±1e7, clone with children); residue ignored
  (AC 38)
- [x] 1.3 `CloneSnapshot` and the resolver: explicit stack, `Pending/OnStack/Resolved/Orphan`,
  composed matrix once per clone, one outline transform per clone with a test-build counter,
  `clones_of` in stacking order; ring and 10,000-chain without recursion (AC 7 to 9, 35 read
  side, 39, 50, 63)
- [x] 1.4 `Document::read_objects()` and the object cache on it; `Document::object(id)` on a clone
  walks its own chain (AC 7, 63)
- [x] 1.5 Linked clone command: for each selected object a clone above it, identity matrix,
  primitives converted first in the same commit (`0047` conversion), clone of a clone, depth > 8
  refused all or nothing, label `linked_clone`, ids minted by the caller (AC 1, 2, 3, 52)
- [x] 1.6 `document.json` entry, `format_version` bump, golden `clone_v<N>.curvyo` (root, two
  clones, a chain of three, a compound root, a converted rectangle), damaged and orphan/ring
  fixtures, older fixtures open unchanged (AC 37, 38, 40)
- [x] 1.7 Path read of a clone equals the path read after unlink (test lands with 2.3) (AC 41)

### Milestone 2: writers, removal, copies, refusals (AC 10, 12, 15 to 17, 19, 26 to 35, 47, 48, 51, 54, 55)

- [x] 2.1 `clone_matrices_after` and the limit check downstream, naming the first clone over the
  limit; spec tests 16 (a), (b), (c) and 17 (AC 12, 15, 16, 17, 51)
- [x] 2.2 `Planned::Clone` in `transform_objects`, `translate_objects`, `rotate_object`, resize,
  Fit to content, the group transform of `0023`; singular case unlinks first (AC 10, 15, 16, 34)
- [x] 2.3 `unlink_clones` (`unlink_clone`): same id and place, fresh anchor ids, root style and
  `opacity`, rotation 0, clones behind it keep following (AC 28, 29, 55)
- [x] 2.4 `repair_clones_of_removed` at every removal site, re-point or unlink, counts for the
  notices; the caller guard test (AC 30, 31, 32, 54)
- [x] 2.5 `remove_orphan_clones` (`remove_orphan_clones`), order-independent (AC 35)
- [x] 2.6 Copies: Duplicate and Ctrl-copy of a clone is a sibling, of a clone original a clone of
  its original; `relink_copied_clones` for sets, roots first (AC 19, 47, 48)
- [x] 2.7 Geometry commands read the resolved outline and consume clones as operands: Boolean,
  Combine, Break apart, Split, Cut, Split compound path; base style from the root (AC 33)
- [x] 2.8 Refusals: node edits, Join, Split at a node, segment bending, Pen continue and connect,
  Object to path (AC 26 core, 27)

### Milestone 3: undo, merges, no cycles (AC 13, 36, 53)

- [x] 3.1 Restore rules: a `clone_link` only when its origin is live (skipped and reported
  otherwise); a restore that removes nodes repairs (decision 9 a, b); `ObjectShape::Clone`
- [x] 3.2 Operation names "Linked clone", "Unlink clone", "Remove orphan clones" in the
  `history_names` guard (AC 36)
- [x] 3.3 Undo tests: Unlink, Delete of a root with two clones, Delete of a middle clone, object
  undo of A's delete after B moved (AC 36, 54)
- [x] 3.4 Merge tests of decision 7 in the `acceptance_0005_peers.rs` style (AC 13)
- [x] 3.5 Property test: 1,000 random sequences on 30 objects, two replicas with merges (AC 53;
  the `0050` commands join in 7.6)

### Milestone 4: ui-core and render-core (AC 4, 11, 14, 18, 20 to 26, 52, 56 to 62)

- [x] 4.1 All `ObjectSnapshot` match sites: hit test by resolved outline and fill, bounds, group
  box, handles, typed entries, skew and stretch never convert (AC 14, 20)
- [x] 4.2 `linked_clone.rs`: button shown rule (selection without a group), depth refusal data
  and refusal outline items (at most 200) (AC 4, 52)
- [x] 4.3 Gesture preview of clones downstream of an edited original, none above 500 (AC 11)
- [x] 4.4 Select bar: switches leave the tree with only clones (AC 18)
- [x] 4.5 Panel data: subject line, clone block, chain line, origin line with live count; style
  edits skip clones and Mixed ignores them (AC 21, 22, 23, 24, 56)
- [x] 4.6 Select original, Select root original, Select clones as view state, group context entry
  (AC 25; the hidden-or-locked rule in 7.3)
- [x] 4.7 Node tool on a clone: no overlay, pill text, double-click hint (AC 26)
- [x] 4.8 `clone_marks`: roles, strongest role, caps 200/50/12, rings under 6 px, tag placement
  against glyphs and chips, edge chips, hidden while dragging or previewing; the 200-selection
  placement test; render-core traces with casing (AC 57 to 62; the Proposal half of 59 included,
  vetoable)

### Milestone 5: session, wasm API, frontend (AC 1, 5, 6, 22, 23, 25, 28, 31, 35, 45, 52, 57 to 62)

- [x] 5.1 `session/clones.rs`, `wasm_clone.rs`: commands with the busy state, notices of AC 5, 28,
  31, 35 and 52, orphan repair after Open and after `import_updates`
- [x] 5.2 Select bar button, last in its own group, tooltip; no key binds any clone command (key
  table test) (AC 1, 6)
- [x] 5.3 Style tab clone block and origin line, focus rules, notices anchored as in the UX notes
  (AC 22, 23, 25, 56)
- [x] 5.4 Canvas tags and edge chips (DOM, `aria-hidden`, `pointer-events: none`), announcer text
  (AC 57 to 62)
- [x] 5.5 Frontend tests for the block, the button and the chips

### Milestone 6: budgets (AC 43 to 46, 61, 63)

- [x] 6.1 Budget tests in release: 10,000 clones and 1,250 chains of depth 8 read in 300 ms, a
  second read in 5 ms, origin-edit frame within 1.2x, next frame 150 ms after a node commit, Linked
  clone and Unlink of 10,000 within 5 s, Ctrl+A over 10,000 clones within 1.2x (AC 43, 44, 45,
  61, 63)
- [x] 6.2 File size per clone measured for the PR description (AC 46)
- Measured 2026-10-14, release, development machine: read of 10,000 clones 44 ms, of 1,250
  chains of 8 34 ms (budget 300); Linked clone of 10,000 0.1 s, Unlink of 10,000 1.8 s (5 s);
  Ctrl+A over 10,000 clones 222 ms against 198 ms over 10,000 paths (1.2x); the frame after
  the original of 10,000 clones moved 181 to 234 ms against 5.9 s for 10,000 ordinary copies,
  so criterion 43's ratio holds but criterion 44's 150 ms does not: about 85 ms of it is the
  tessellation of the 10,000 outlines. Open point for the architect (decision 3's next step).
  File size: 29 bytes per clone in the packed file.
- Milestone 6b (2026-10-14, architect review of M2 to M6, F1): the shared root tessellation
  (`CloneChain::root_outline`, render-core `shared_outline.rs`) and a cheap on-screen test of the
  marks (the root's hull under the composed map, not the curve bounds of each clone). The frame
  after the original of 10,000 clones moved is now 128 ms, asserted at 150 ms in release. Its
  breakdown: the read 30 ms, the artwork 63 ms (6,000,600 vertices mapped from one tessellation,
  memory bound), the marks 1.4 ms, decorations and the rest of the commit frame about 34 ms. The
  per-frame copies of the object list in `draw_list` were not needed.
- Review S9: criterion 43's first frame after opening is measured by
  `ac43_the_first_frame_after_opening` (the Open and the first draw list, budget 1 s). Its "frame
  in which the origin is changed (one node dragged, previews off)" is measured as the frame after
  a nudge of the whole original: that commit changes every anchor of the root, so the read, the
  resolution of 10,000 clones and the artwork cost at least what a one-node drag costs; a node
  drag in the session needs pointer plumbing the budget test does not have.

### Milestone 7: after `0050` merges (its criteria 7, 24, 25, 26, 28, 36, 38, 45; this spec's 8, 21, 24, 25, 49, 53, 58)

- [ ] 7.1 Rebase onto `main` with `0050`; `format_version` 13, golden renamed and regenerated
- [ ] 7.2 `read_nodes()` calls the resolver; a clone's effective opacity is the root's `opacity`
  times its own ancestors'; its flags and name are its own; a hidden origin still feeds its clones
  (0050 c24, c25, c28; AC 8, 49)
- [ ] 7.3 Hidden or locked targets left out of Select original, root original and clones, with
  the refusal and partial notices; the hidden-original tag "Clone (original hidden)" (AC 25, 58)
- [ ] 7.4 Opacity field and `set_node_opacity` skip clones, caption for only clones (0050 c26; AC
  24)
- [ ] 7.5 Layers rows: `Link2` glyph, "Clone of ..." names, root opacity readout, Select original
  row action, tooltip with the chain line, subject slot at 80 px (0050 c7, c38; AC 21)
- [ ] 7.6 Delete layer repairs clones; undo of Delete layer restores them; the property test adds
  New layer, Delete layer and moves (0050 c36, c45; AC 53)

### Milestone 8: gate, docs, PR

- [ ] 8.1 Full gate (`CLAUDE.md` §7), frontend checks, budgets
- [ ] 8.2 `specs/README.md` format plan row and status, spec status line, design system if the build
  differs
- [ ] 8.3 PR: "What changes in the demo", Review guide, file size per clone (AC 46)

## Format version

- `main` is 11; `0050` takes 12 (`specs/README.md`, "`format_version` plan": 0049 is "13 once 0050
  has taken 12").
- Until the rebase in 7.1 the branch sets `CURRENT_FORMAT_VERSION = main + 1` (12 today) in one
  line, and every clone test reads one constant `CLONE_FORMAT_VERSION`; the golden and the other clone
  fixtures are written when `CURVYO_WRITE_FIXTURES` is set (the repo pattern). The rebase onto `0050` conflicts on that line by design, and
  setting it to 13, renaming the golden to `clone_v13.curvyo` and regenerating it is one commit.
- The version is never skipped: no branch state has 13 while `main` is at 11.
- The PR is opened only after `0050` is on `main`, so 0049 merges with 13. If `0050` were still
  open when milestones 1 to 6 are done, I stop and report (Question 2).

## Validation

- Golden files for the format (1.6); damaged, orphan and ring fixtures.
- Property test of no cycles with two replicas (3.5), merge tests (3.4).
- Exactness: drawn outline before and after transform, unlink and re-point within 1e-9 mm
  (1e-6 mm for transforms, AC 15, 16).
- Test-build counter: one outline transformation per clone (AC 63).
- Budget tests in release (milestone 6), with `CARGO_BUILD_JOBS=4`, `nextest -j 4`,
  `ulimit -v 8000000` for cargo tests.
- Manual check in the browser build: twelve hinge tabs from one path, a chain of three, delete of
  the middle clone, marks at several zooms.

## Review of M2 to M6 (2026-10-14, `review-architect-m2-m6.md`)

- F1 shared root tessellation (commit b24ad14): criterion 44 asserted, 128 ms.
- F2 the limit check reads only the chains of affected clones; F3 the gesture settles matrices
  with its own map over `Document::clone_links`; F4 the panel reads `clones_of` and an index
  from the cached `ObjectRead`; F5 the repair ids are pinned unique in one session.
- S1 to S9 done (38e6efc).
- Gaps closed: AC 17 node edits of a root (`move_anchors`, `resize_path`, `set_handle`; only
  anchor points are limited, so handle edits never refuse), AC 25 hover box, AC 45 busy state,
  AC 60 reference marks and DOM chips as tag obstacles, AC 61 dashed ring, AC 62 screen-reader
  sentence, the notice anchoring (Linked clone under its button, the repair sentence joined to
  a rail command's notice, a Delete key's in the canvas slot).
- Milestone 7 was prepared on `scratch/linked-clone-on-0050` and came onto this branch with the
  merge of `main` after `0050` (#118) and `0038` (#115): format 13, the `0050` clone parts, the
  Linked clone button in the Layers toolbar, Offset reading a clone as a path (criterion 33).
- Tester review F1 (2026-10-14): undo rule (a) leaves a clone with a dead link out of the
  restore whole; a clone without a link is an orphan the repair turns into its outline
  (architect decision B, criteria 35 and 38 amended).

# ADRs for "Manufacturing roles"

Architect, 2026-11-04, against `story/0027-manufacturing-roles` on top of `main` at `d32a78bc`
(format 36). One new ADR, **no new crate, no new external dependency, no `format_version` bump**.
Written for Frage 140 A, 141 A and 142 A; note 8 says what changes on B or C.

## Depends on or extends

- [ADR 0040](../../docs/adr/0040-laser-roles-keyed-by-layer.md) (new, extends ADR 0033 §3): one
  register per layer `x.curvyo.laser.role.<peer:counter>`, reserved `...role.outside`, values
  `cut`/`engrave`/`ignore`, absent = Unassigned; Unknown and Unassigned never output; what is
  output for `0029`; no format bump, golden `laser_roles_v36.curvyo`.
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §3 to §5: opaque
  `x.*` values, never rewritten on read, unknown processes untouched, merge per register.
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §3 to §5: role logic in
  `curvyo-laser-core`, one more arm in `ui-core/src/extensions.rs`, no trait; a new block kind
  (note 1); key form is published contract.
- [ADR 0032](../../docs/adr/0032-workbench-model-and-registry.md) §1, §6, §9: Role view is session
  state; placement membership unchanged; placed content stays read-only.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md) §2 and `0050`
  adrs: effective hidden from the one read; hidden is not output is decided here (Frage 142).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): a deleted layer is revived with
  its node id, so its role returns (criterion 17).
- [`0026` adrs.md](../0026-machine-profile/adrs.md) note 4: host-drawn block pattern.

## Feature-local notes (2026-11-04)

1. **The Layers block is a host-drawn block kind: `Block::Layers { label }`** in
   `curvyo-extension-core/src/manifest.rs`, entry in its `CHANGELOG.md`; the laser manifest puts
   it below `MachineProfile`. ADR 0034 is not amended, for the reasons of `0026` note 4 (§4
   "only the kinds a story uses", Consequences "a new block kind in the application"). Its view
   comes from the session's manufacturing view JSON: rows from `DocumentRead.nodes` (top-level
   layers, front-most first; name; effective hidden), the `outside` row while loose top-level
   objects exist, each row's role read by `curvyo-laser-core::roles`, the notice lines, the Role
   view state. Rejected: one `Param` of kind `Choice` per layer (a dynamic schema, and the same
   objections as `0026` note 4).
2. **Code places.** `curvyo-laser-core/src/roles.rs`: `Role`, key text, parse to `Known | Unknown`,
   the output rule of ADR 0040 §6, notice texts from counts the caller passes. Document-core: the
   one `x.*` write command (ADR 0040 §4), `NodeId::to_register_text`/parse, the container check.
   `curvyo-editor-wasm/src/session/manufacturing/roles.rs`: Layers block view, `set_layer_role`,
   the leaf-to-layer map. `curvyo-render-core`: the looks (note 3).
3. **The role look is a per-leaf override at artwork build, not in the resolution.** Placement
   membership (ADR 0032 §6, `frame.rs`) stays as it is; the snapshots are never restyled or
   copied with a new style, so the geometry `0029` reads is the resolution's. The session maps
   each leaf to its top-level layer in one O(n) pre-order pass over `DocumentRead.nodes`, cached
   per read like `PlacedCache`, and the role to a look. `build_artwork_with_guides(objects,
   &HashSet<NodeId>, view)` becomes `build_artwork_with_looks(objects, &HashMap<NodeId, Look>,
   view)` with `Look::{Marking(color), Outline(color), Area(color)}`: guides use
   `Marking(GUIDE)` (behaviour unchanged), Cut `Outline`, Engrave `Area`, Ignore and Unassigned
   `Marking(muted)`. Two users justify the enum (`CLAUDE.md` §5); no trait. Widths and dashes are
   screen pixels at the **tessellation** view (it includes the placement's scale), so the 1 px
   line holds under a scaled placement. `Area` fills by the leaf's fill rule; a leaf whose
   outlines are all open (open path, single-line text) draws as a 1 px line in the engrave colour.
   Images: `Area` pushes the image with a new `grayscale` flag on the image draw item (the front
   end applies the filter); other looks draw the image's frame as a closed outline. The clone
   fast path (`SharedOutlines`) is skipped for a leaf with a look. Rejected: rewriting each
   member's `Style` in the session (does not cover images or text kinds, duplicates the guide
   path); a look in document-core's read (view state in the document, ADR 0032 §1).
4. **Role view** is one `bool` in the session (default on, not saved, no undo, for every
   manufacturing of the session). Off: `build_artwork` as `0093` shipped it.
5. **Colours:** `--role-cut`, `--role-engrave`, `--role-ignore` (ux-engineer) are mirrored in
   `curvyo-render-core/src/theme.rs` like `--guide`, covered by the existing theme test.
6. **Criterion 21 budgets confirmed** (release build, CI `performance-budgets` job, two tests in
   `curvyo-editor-wasm/tests/manufacturing_budget.rs` using `tests/cpu_time/`): (a) with 1,000
   rectangles in 10 layers and the Laser tab open, `set_layer_role` plus the next
   `frame_draw_list` together at most 50 ms of thread CPU. Basis: a role write re-reads the
   drawing (`0050` adrs decision 3 measured 10,000 objects within 150 ms, so 1,000 are about a
   tenth) plus one tessellation of 1,000 rectangles; the first-open budget of `0093` (1.0 s)
   includes setup this does not. (b) With 200 layers, `manufacturing_view_json` including the
   Layers block at most 100 ms (200 rows and one O(n) count, the size of `0026`'s 200-record
   list within 100 ms). If (a) fails, the fix is a read cache keyed on the drawing only, a
   separate decision.
7. **No format bump** (ADR 0040 §7). Criterion 19's golden is `laser_roles_v36.curvyo`; the
   tests that `laser_workbench_v36.curvyo` and `machine_profile_v36.curvyo` open with no role and
   write nothing stay as written.
8. **Other answers.**
   - Frage 140 B: an all-open leaf under Engrave draws in the Ignore look, is not output (ADR 0040
     §6 gains "closed outlines only") and is counted in criterion 15. Storage unchanged.
   - Frage 140 C: values `line`, `fill`, `fill_line`, and a fourth look (`Area` plus `Outline`).
     The strings are written into files, so C must be chosen before merge; afterwards it is a
     format change (ADR 0040 §2).
   - Frage 141 B: absent reads as Cut; there is no Unassigned and pressing the pressed button
     does nothing. Changing the meaning of absent after merge is a bump, so this too is final at
     merge.
   - Frage 142 B: placements draw from `DocumentRead.leaves` (hidden included) and ADR 0040 §6
     drops the visibility condition. Frage 142 C: hidden leaves are drawn with a faded `Marking`
     look and not output. Neither touches the file.

## Spec issues (for the product owner before Ready)

1. **Criterion 8 (a) and (b) disagree for open outlines under Engrave.** Proposal: a leaf with at
   least one closed outline is filled (open outlines closed by a straight line); a leaf whose
   outlines are all open is a line. Say it in one sentence in criterion 8.
2. **Criterion 8 (f) points at "criterion 16" for the image notice;** it is criterion 15.
3. **Criterion 19:** the golden is `laser_roles_v36.curvyo` (no bump); drop "next free at merge"
   and the Format line's conditional.
4. **Criterion 18:** "refused as damaged only as ADR 0033 §7 says" should read ADR 0040 §3 (a
   container value under `x.*`); §7 lists no such case.
5. **Criterion 5:** name the key form (`x.curvyo.laser.role.<layer id>`, `...role.outside`) so the
   tester can assert it; criterion 11's "reserved key" is `outside`.
6. **Criterion 3 or 17:** add that a duplicated or pasted layer is a new layer with no role.
7. **Criterion 15:** define "output-capable object" as a visible leaf that is not a guide or
   dimension (an image counts, it can be engraved).
8. **Criterion 21:** say "the role set plus the next frame" for the 50 ms and "the Laser tab's
   view including the Layers block" for the 100 ms (note 6).

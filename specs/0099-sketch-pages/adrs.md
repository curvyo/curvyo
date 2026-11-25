# ADRs for sketch pages

Architect, 2026-11-06. Slice 2 of `0063` (concept A). Frage 157 A and 158 A assumed.

- [ADR 0043](../../docs/adr/0043-sketch-pages-content-by-ownership.md) (new, Proposed,
  `needs-customer` for the document model; design by the customer): one kind-aware membership read
  (criteria 1, 2, 15, 17, 18), the sketch not pickable and moved only by its frame (3, 4, 5, 6),
  Convert to paths writing `kind` Sketch to Drawing (15), Delete with the `0049` clone repair (16,
  17), `place_linked_clone` (8, 11), output through the membership read (18), no format bump (19).
  With 157 B or C, or 158 B or C, its decisions 4 and 5 shrink as its Consequences say; the format
  is unchanged in every case.
- [ADR 0042](../../docs/adr/0042-page-kinds.md) §1, §3, §4: kind register and read rule
  unchanged; amended by 0043 for the one second write of `kind` and for a sketch page's content.
  §8 (only sketch pages own a sketch) and §9 (migration) unchanged.
- [ADR 0024](../../docs/adr/0024-spatial-pages.md) §1, §5: position membership for drawing and
  output pages, the frame move as one commit with the clone-aware translation; amended by 0043 for
  sketch pages.
- [ADR 0026](../../docs/adr/0026-parametric-sketch-nodes-and-workbench.md): claim registers, lazy
  creation, conflict objects and the conversion body (same id, claim cleared) unchanged; page
  deletion and original transforms amended by 0043.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md),
  [ADR 0016](../../docs/adr/0016-clone-overrides.md) §2: clones read the root original's outline
  and style (7); `own_placement` on placed clones (8); `Hold::Whole` holds every clone back in the
  frame move (5).
- [ADR 0014](../../docs/adr/0014-history-undo-and-branches.md): Convert, Delete and Place are one
  step each, restored field by field, including the `kind` register (15, 17); activation and the
  row drag before the drop write nothing (8, 9).

## Feature-local decisions

1. 2026-11-06: **History labels.** New: `place_sketch_clone` ("Place sketch clone"),
   `convert_sketch_page` ("Convert sketch to paths", a document-level page operation),
   `delete_sketch_page` ("Delete sketch page"). Kept: `move_page`, `resize_page`, `unlink_clones`.
   The `0020` criterion 41 label guard gets the three new entries.
2. 2026-11-06: **CPU budget** of criterion 20 confirmed at 16 ms thread CPU per pointer move, in
   `curvyo-editor-wasm/tests/cpu_time/`, measuring the session's move call and the canvas view
   built for that frame. Neither drag re-solves the sketch: the frame drag changes only the
   placement (the graph fingerprint matches, the checkpoint is reused), and the ghost outline is
   resolved once when the row drag begins and only translated per move.
3. 2026-11-06: **Press order** for criterion 2: hit-test pickable drawing objects first (the owned
   sketch is not pickable, ADR 0043 §2); only a press that hits nothing activates the page under it
   by the `0098` criterion 8 rule. The same page-under-pointer rule picks the drop target of the
   row drag where frames overlap.
4. 2026-11-06: **Page-row drag plumbing, shared with `0100`.** Built once by whichever slice merges
   first; the other adds one enum variant and its drop command, nothing else:
   - front end: the drag-out of a row lives in `PagesBlock.tsx` next to the reorder drag (a release
     inside the list still reorders); one hook `frontend/src/hooks/usePageRowDrag.ts` forwards
     pointer positions in canvas coordinates to the session and draws the badge from its answer;
   - session: `curvyo-editor-wasm/src/session/row_drag.rs` holds the drag as view state
     (`RowDrag { source: RowDragSource, ghost, at }`), with `row_drag_begin(page)`,
     `row_drag_move(point)` returning the badge kind, the target page and the snapped ghost
     offset, `row_drag_drop()` and `row_drag_cancel()`; `RowDragSource` is an enum (`Sketch` here,
     `DrawingPage` in `0100`), no trait; snapping is the Select-move snapping of `0060` with the
     ghost box as the moving box;
   - document-core: no shared code; each source has its own one-commit drop command
     (`place_linked_clone` here).
   Badge words are front-end strings keyed by the badge kind (UX notes). Nothing is written before
   the drop; Escape or a drop on a sketch page calls `row_drag_cancel`.
5. 2026-11-06: **"Has geometry"** (criteria 8, 10, 11, 15) means the solved sketch has at least one
   non-construction entity, so a clone, a ghost and a conversion draw something. A sketch with only
   parameters or only construction geometry counts as empty for these buttons and badges
   (Proposal; see spec issue 6).
6. 2026-11-06: **Usage count** (criteria 12, 16) counts every clone whose chain's root original is
   the sketch (direct and chained), by `clone_link` read, with its pages from the ADR 0043
   membership read.

## Spec issues (for the product owner)

1. Criterion 18 (a) asks for "closed loops"; `0024` criterion 138 writes a sketch as one element
   per entity, no merging, and its golden `export_sketch` pins that. Proposal: 18 (a) says "as
   `0024` criterion 138"; no export change in this slice.
2. Criterion 18 (c) holds only for **Active page only** (`0024` criterion 130): a whole-plane
   export writes every object at its coordinates (criterion 128), so the circle is in the file
   and Inkscape shows it inside the sketch page's frame. Proposal: (c) and the "page holds
   exactly" of (a) are stated for Active page only with the sketch page active.
3. Criterion 18 (d): a whole-plane export writes a clone as a `<use>` where `0024` criterion 59
   allows it, a path otherwise; only with Active page only on the drawing page is the clone a path.
   Proposal: "as `0024` criteria 59 and 130 (e) write it".
4. "or prints" in criterion 18: there is no print command (`0059` lists printing as out of scope).
   Proposal: drop print from this slice.
5. Criterion 4 forbids selecting the sketch "in no context", but Out of scope keeps "the Layers row
   of a sketch on drawing pages as built", which selects it. Proposal: a click on that Layers row
   activates the sketch page like Select original (criterion 13); nothing is selected.
6. Criteria 8, 10, 11, 15 "with geometry" / "empty": a sketch with parameters but no entity, or only
   construction lines, is neither empty (`0098` criterion 3) nor drawable. Proposal: feature note 5;
   the badge of criterion 10 then also covers it.
7. Criterion 12 with clones on the pasteboard: "on 2 pages" does not count them, and Select clones
   has no page to activate if all lie on the pasteboard. Proposal: the line reads "Used by 3 linked
   clones on 2 pages" counting pasteboard clones only in the first number; Select clones then keeps
   a drawing page active (the last active one) and selects the pasteboard clones.
8. Criterion 4 makes ADR 0026's pending original-scaling question unreachable (no command
   transforms an original). The spec lists it as out of scope; it should say it is closed as "not
   offered" unless the customer reopens it.
9. Criterion 17: a clone a peer creates while the page is deleted elsewhere becomes an orphan and is
   removed (`0049` criterion 35), not turned into a path. Proposal: add one sentence; it cannot
   occur without a shared room.
10. Criterion 1 test wording: "belongs to its sketch page only" also needs the sketch overlapping a
    drawing page to be no member of that drawing page (ADR 0043 §1); suggest the test place the
    sketch across a neighbouring drawing page's edge.

## Revision 2026-11-19 (sketch instances, criteria 23 to 31)

- 2026-11-23: **Instance mechanism decided in [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md)
  §6 and §9** (proposed, `needs-customer`), shared with the generated objects of `0141`: the sketch
  holds `exposed_params` (one atomic list of `{ id, min, max }`, criterion 23); a clone holds its
  own values in one atomic map `instance_params` keyed by `ParameterId` (absent = the sketch's
  value, the nearest own value on a chain wins; criteria 24, 27, 28); document-core solves a clone
  with its values in the read pass, once per distinct value set, keeping the last valid geometry
  (26, 31); one `curvyo-ui-core/src/instance.rs` for the schema, the edit (`change_parameter`), the
  preview memo, the handles (25) and Convert to paths / Unlink with own values (29). Its own
  `format_version` bump and goldens when the revision lands (30). The 16 ms budget of criterion 31
  stands. Still missing before Ready: UX notes for Expose and the handles. (Keys, entry shape and
  the move of `ParamValue` amended by the addendum of 2026-11-25, A2, A3.)

## Addendum 2026-11-25: making the instance revision buildable (criteria 23 to 31)

Architect, 2026-11-25. Read against main at `CURRENT_FORMAT_VERSION` 60, ADR 0080 §2, §6, §7, §9
and its notes, the generator arm on main (`curvyo-ui-core/src/instance*.rs`,
`curvyo-editor-wasm/src/session/generators*.rs`), the sketch read (`sketch_read.rs`,
`sketch_commands.rs`, `sketch_codec.rs`), the clone read (`clone_read.rs`, `clone_codec.rs`,
`clone_repair.rs`), `svg_export.rs` `as_use` and UX notes §12 (2026-11-25). The code is the record
where it differs from ADR 0080; the differences are listed in A1 and noted in ADR 0080 (note
2026-11-25). Everything not fixed by a criterion or an accepted ADR is **(Proposal)**.

### A1. What main has and what ADR 0080 assumed

- `enum InstanceSource` does **not** exist. Every function of `curvyo-ui-core/src/instance.rs`
  takes a generated `NodeId`; `instance_source.rs` is the ADR 0083 box-from-outline code (a
  different "source"). `instance.rs` is 569 lines (over the §5 limit).
- `ParamValue` and its codec are still in `curvyo-extension-core`, which depends on
  document-core; the read pass cannot use them. `ParamValue::Path(ClosedPath)` (ADR 0083) would
  drag the contract's path types into document-core if the type moved as ADR 0080 §2 said.
- `Param` holds `&'static str` key, label and tooltip; a sketch's names are runtime strings.
- A driving dimension is a `Constraint` with `value: Expression`, keyed by `ConstraintId`; ADR
  0080 §6 keys values by `ParameterId` only. Both id types are free strings and can collide.
- Count parameters (`0066` AC53, revision 6) are a Draft; sketch-core has `Scalar`, `Length`,
  `Angle` only.

### A2. Keys: tagged keys, one namespace (Proposal)

An exposed value is named by an `ExposedKey`, stored as one string: `p:<ParameterId>` for a named
parameter, `d:<ConstraintId>` for a driving dimension. Rejected: (B) promoting a dimension to a
parameter on Expose: it writes a parameter the maker did not make, its name must be an identifier
(UX 12.1 prefills "Distance 1"), and Stop exposing leaves it behind; (C) two maps, one per id
type: two codecs, two orders, and the shared value block needs one key anyway.

- A named parameter may be exposed whatever its expression (literal or formula); an own value
  replaces its expression by a literal for that clone, so formulas that use it follow (a
  `hole_spacing = width - 2 * margin` follows an own `width`). Its default is the evaluated value.
- A driving dimension may be exposed whatever its expression; an own value replaces that
  constraint's expression by a literal. A reference dimension cannot be exposed.
- Precedence: parameter literals first, then the parameters are evaluated, then dimension
  literals. An own `d:` value therefore wins over an own `p:` value the dimension's formula uses.
- Kinds this revision: Length (distance, length, radius, diameter, horizontal and vertical
  distance, ellipse radius) and Angle (angle, axis angle, sweep). Count arrives with `0066`
  revision 6 (its own codec arm `{"count": n}` and `ParamKind::Count`, already in extension-core).
  **Scalar is not exposable** in this revision (spec lists length, angle, count; it would need a
  new value encoding and `ParamKind`): see PO finding 2 and UX mismatch U3.

### A3. Data model and codec (`format_version` 62)

One value type in document-core, `InstanceValue { Length(Length), Angle(Angle) }` (unit
newtypes; `Angle` in radians in memory). Its stored encoding is ADR 0080 §2's one-key map,
`{"mm": n}` or `{"deg": n}`, owned by document-core's new `instance_codec.rs`. Extension-core's
`ParamValue::to_stored` / `from_stored` call that codec for their `mm` and `deg` arms, so there is
one encoding; `ParamValue` itself stays in extension-core (amends ADR 0080 §2). Equality of two
value sets compares the **stored** numbers (f64 bits of mm or deg), never radians, so a
deg-to-rad round trip cannot split one set into two.

Sketch node (`shape = "sketch"`), one new register, an LWW **list value** written whole:

```text
exposed_params : [ { "key": "p:width",  "min": {"mm": 20}, "max": {"mm": 200} },
                   { "key": "d:c7", "name": "Radius 1" } ]
  key   "p:<id>" or "d:<id>", the id non-empty; unique in the list
  name  only on "d:" entries: 1 to 64 characters, no control character (the clone's field label)
  min, max  optional one-key values of the key's unit; absent = unlimited
```

Clone node (`shape = "clone"`), two new registers, LWW **map values** written whole (never
containers, like `clone_link`):

```text
instance_params   : { "p:width": {"mm": 82}, "d:c7": {"mm": 6.5} }   own values; absent or {} = none
instance_fallback : { "p:width": {"mm": 150} }  the last value set that solved; present only
                                                 while instance_params has no solution (A5)
```

Read and damage rules (ADR 0033 §3, ADR 0072 §5, ADR 0080 §4 precedent: refuse only a wrong
register shape, never a value):

- **Refuses the file (Damaged):** `exposed_params` present on a sketch and not a list value;
  `instance_params` or `instance_fallback` present on a clone node and not a map value (a
  container, a string, a number). The shape is checked on every sketch node in the `sketch` arm
  of `path_codec::validate_object_node` (beside `sketch_codec::read`) and on every clone node in
  `clone_codec::validate_clone_node`, whatever the clone's root.
- **Ignored at read, kept, never rewritten on read:** an entry or bound that does not decode (not
  a map, key not `p:`/`d:`, a name of the wrong shape on a `d:` entry, a bound that is not one
  finite one-key value within ±1e7, `min > max`); a duplicate key (the first wins); an entry
  whose parameter or dimension is gone, is a reference dimension, or whose unit does not match
  the bound (that bound is unlimited); a clone value whose key is not exposed now, whose unit
  does not match the key's kind, or that does not decode (the key reads as absent: the value up
  the chain or the sketch's). A stored own value outside the current limits is **clamped at
  read** and not rewritten (permit over refuse).
- **Residue (ADR 0015 §4):** `exposed_params` on any node that is not a sketch and the two clone
  registers on any node that is not a clone: not validated, never read. On a clone whose root is
  not a sketch the two registers are shape-checked (above) and ignored.
- Names: uniqueness among exposed names is the writer's rule (UX 12.1 "Name in use"); a merge
  that duplicates one is not damage.

`format_version` **62** (next free after 61, renumbered from 61 at rebase; renumberable at rebase: ADR 0082 §10 `part_keys`
also claims "the next"). A build that stops at 60 would draw every clone with the sketch's values
and cut the wrong part, so it refuses 62 by version; a file of 61 opens unchanged. The
`CURRENT_FORMAT_VERSION` doc comment gets the paragraph. Goldens in
`curvyo-document-core/tests/fixtures/`: `sketch_instances_v62.curvyo` (a sketch exposing one
parameter with limits and one dimension with a name; clones: none own, one own, a chain whose
second clone has its own second value, one with no solution and a fallback);
`sketch_instances_damaged_v62.curvyo` (an `instance_params` container); a residue case as a unit
test (stale keys, wrong unit, out of limits: opens, reads as stated, saves back equal).

### A4. Where solving happens, and the cache (criteria 26, 31)

- **Substitution** is pure sketch-core: `Sketch::with_values(&self, &InstanceValues) -> Result<Sketch,
  SketchError>` in a new `curvyo-sketch-core/src/instance_values.rs` ("Replaces exposed parameter
  and dimension values of a graph by literals"), A2 precedence.
- **Solve** in document-core's read pass (ADR 0080 §6), new `curvyo-document-core/src/sketch_instance.rs`
  ("Solves a sketch with one value set and memoises the result"). Seed: always the sketch's own
  solved candidate (`SketchSnapshot::solution`), never an earlier drag step, so a preview, the
  committed read, a reopen and every peer give the same geometry. Mode `SolveMode::Edit` unless
  the first plan task measures one solve of the `tests/sketch_pages_budget.rs` 100-entity
  workload above 4 ms thread CPU; then a `SolveMode::Values` (no rank analysis, residuals still
  validated) is added in the same slice with the number in the PR.
- **Cache** `Document::instance_solves: RefCell<InstanceSolves>` keyed by **content**, not by
  document version, in two levels: per sketch `NodeId` one entry holding the sketch's `graph`
  **and** `checkpoint` (the seed depends on both, as `Document::sketch()` compares them),
  compared once per read, and inside it per canonical value set `Result<Arc<Sketch>,
  NoSolution>` plus `Arc<PathSnapshot>` per refinement stretch. A changed graph or checkpoint
  clears that sketch's sets (never hash the graph into each set's key). After each full read only the keys that read used, plus the one preview key,
  are kept. A commit of one clone's value therefore re-solves at most that one set (the release
  reuses the last drag step's entry: zero solves).
- **Read pass.** `RawOverrides` gains the decoded own values. `walk` carries the effective set
  (origin's set overlaid by the clone's own, nearest own wins per key: criterion 28) as an index
  into a per-read interner. `refine_sketch_roots` refines per `(root, set)` and
  `outline_clones` hands every clone of one `(root, set)` the **same** `Arc<PathSnapshot>` as
  `root_outline`, so `SharedOutlines` (render-core), the visibility hulls (`clone_marks.rs`) and
  `gradient_box` keep sharing per set by `Arc::as_ptr`; the invariant "resolved = root_outline
  under composed" holds per set. An empty effective set is the sketch's own outline (no solve).
  `resolve_in` and `read_one_clone` are free functions: they take `&InstanceSolves` as a
  parameter (tests pass a fresh one).
- `CloneChain` gains `own_params: bool` (the clone itself holds own values) and
  `no_solution: bool`.
- **Drag step (31).** A field drag (slice 1) or knob drag (slice 2) writes nothing: the session
  asks `Document::instance_outline(clone, &values)` (same cache), keeps it in its one preview
  slot and draws it as the blue overlay over the memoised black artwork (the `overlay` route of
  `generators.rs`); the 199 other clones are not touched. CPU test
  `curvyo-editor-wasm/tests/sketch_instances_cpu_time.rs` (`mod cpu_time;`): 200 clones of the
  100-entity sketch, each with a distinct own value; per drag step the session's preview call and
  the canvas view of that frame at most 16 ms thread CPU; a test-build solve counter asserts one
  solve per step and none on release; the other clones' `root_outline` pointers are unchanged.
  The file open (200 solves) and the release commit's full read are not budgeted; the PR states
  the measured open time.

### A5. No solution (criterion 26)

The sketch keeps its last valid state in its checkpoint, so a clone's fallback must also come from
the file, not from session memory (otherwise what is cut depends on which device opens the file).
**(Proposal, Frage-ready, default A.)** A: the writer stores `instance_fallback`, the last value
set that solved (the last solvable drag step, or the stored set before a typed edit, or the
existing fallback), whenever it commits a set with no solution, and removes it when it commits one
that solves. A clone whose effective set has no solution draws, with `no_solution` set: its own
`instance_fallback` solved, if present and solvable; else, when it holds no own values, exactly
what its origin draws (recursively, so a plain clone of a failed clone matches it, and its `<use>`
export stays true); else the sketch's own geometry. B: nothing stored; the same rule without the
first step (simpler, but the part jumps back to the original's shape, which is not "its last
valid geometry"). The golden holds a chain: a failed clone with a fallback and a plain clone of it. Separate registers are safe under
LWW: a fallback paired with a solvable set is never read, a set without its fallback falls to the
upstream rule, deterministic on every peer.

### A6. Sketch edits, chains, Convert, Unlink, export (criteria 27 to 29)

- **Sketch edits (27).** Clones follow through the cache (the graph changed, its entries are
  cleared). Rename follows: keys are ids. A deleted parameter or dimension makes its key residue;
  Undo brings it back with the values.
- **Expose commands** (document-core `instance_commands.rs`, one commit each): `expose_parameter`
  (writes the list), `unexpose_parameter` (removes the entry **and** the key from every clone's
  `instance_params` and `instance_fallback` in the same commit, so a later Expose starts clean),
  `parameter_limits` (limits and a dimension's name). Clone side: `change_parameter` (exists, ADR
  0080 §7; subject field `gp` = the key), `reset_parameter`, `reset_parameters`, `own_parameters`.
  Row texts per UX 12.1, 12.2; the `0020` criterion 41 label guard gets the six new keys.
- **Chains (28):** A4 `walk`. "Back" removes only this clone's key.
- **Unlink (29):** writes `clone.resolved` (already the per-set geometry, including the A5
  fallback) and removes `instance_params` and `instance_fallback` with `own_placement` and
  `own_style`.
- **One follower rule** for Unlink (29), Delete sketch page (16, 17) and Convert to paths on the
  sketch page (15) **(Proposal)**: after the command a clone's chain may end in a path, where own
  values are residue, so a clone that **holds own values** below the changed node would jump. In
  the same commit each such clone is unlinked into a path from its `resolved`, deepest first;
  every clone without own values keeps following its nearest original, which now draws what it
  drew before, so nothing jumps and no more links are lost than needed. Checked against
  `clone_repair.rs` `repair_clones_of_removed`: it fixes only clones whose direct origin is
  removed (unlinked from `read.resolved`, correct), and leaves their followers linked, so Delete
  needs the rule too. Convert also removes `exposed_params` with the sketch keys.
- **Export, output, hit test, membership** read `resolved`: no change. `svg_export.rs` `as_use`
  adds `!chain.own_params`: a clone with own values is a path, never a `<use>`; a clone with no
  own values of such a clone may stay a `<use>` of it (same values). Golden
  `curvyo-ui-core/tests/fixtures/export_sketch_instance.svg`.

### A7. ui-core and session: what the Sketch arm adds, what stays shared

- `instance.rs` splits first (behaviour-preserving, in the same branch): the block view
  (`RowView`, `InstanceView`, `view`, `value_from_number`, `value_from_json`) moves to
  `instance_view.rs`. `instance.rs` then holds `pub enum InstanceSource { Sketch { clone: NodeId },
  Generated { node: NodeId } }` and the dispatching `change_parameter(source, key, value)`;
  generator-only functions (place, resize, convert, records) keep their `NodeId` signatures.
- New `instance_sketch.rs` (the sketch-clone arm): rows from the exposed list and the clone's
  effective values (own / inherited from depth n / sketch's), clamp to limits with the shared
  `clamp_notice`, plan through `Document::instance_outline`, the commits of A6. Its row type
  carries owned strings (`key`, `name`) and the shared `ParamKind` and `ParamValue`; `Param` is
  not used for sketch rows (its `&'static str` stays).
- **Shared:** `ParamKind` and `ParamValue` (field kind, bounds, value), the one encoding, the clamp
  notice, the `ValueField` (both blocks, UX 12.2), the knob drag math (`dragged_mm`, Shift whole
  mm), the session's one preview slot (`generators.rs` `Preview` gains the `InstanceSource` and
  holds the outline to draw, not a generator record), the overlay drawing, the label
  `change_parameter`. **Not shared:** the generator's record, seed, resize and Convert; the
  sketch's exposed list, fallback and chain.
- **Slice 2 handles** (`instance_handles.rs` sketch arm, anchor geometry in a new
  `instance_sketch_handles.rs`): a knob for an exposed `d:` length, and for a `p:` key only when
  exactly one driving dimension's expression is that bare reference (UX 12.3); anchor and
  direction from the solved instance under `composed`, rate 1 (2 for a diameter), divided by the
  composed scale so the value is in sketch units. Angle needs a turning knob: `Knob` gets
  `motion: KnobMotion { Along { direction, rate }, Around { centre } }` (the generator's knobs
  become `Along`).

### A8. Slices (Proposal)

1. **Expose, values, Params row** (criteria 23, 24, 26, 27, 28, 29, 30, 31 for the field drag):
   format 62 with goldens, codec, cache and read pass, Expose UI on the sketch page, the Parameters
   override row with `ValueField`s, no-solution look and fallback, Convert / Unlink / export
   rules, the CPU test with the field drag. The customer can expose a value and type or drag it
   per clone: one PR.
2. **Canvas handles** (criteria 25 and 31 re-run with a knob drag): knobs, `KnobMotion`, readout,
   Shift snap, Escape, double-click entry chip. Commits on the same branch and its own PR when
   slice 1 is accepted, or the same PR if both land before the demo.

### A9. Risks

- One solve per distinct set: 200 distinct sets cost 200 solves at open and after any sketch edit
  (every entry is cleared); measured and stated in the PR, a background solve is a later change.
- Seeding from the sketch's solution, a large value change can land on another branch (a tangent
  side, a flipped triangle) than an incremental drag would; preview and commit agree, so the
  maker sees it. Pinned by one test.
- The raw read already re-solves each sketch on every full read (`object_read.rs`, no cache);
  unchanged here, but the instance cache must not depend on it being cheap.
- `instance.rs` split and the `Preview` generalisation touch merged generator code; the 0141
  tests are the safety net.
- `format_version` 62 races with ADR 0082 §10; renumber at rebase, goldens renamed with it.

### A10. Findings for the product owner (spec text)

1. Criterion 23: "stores the parameter's name, its unit kind and its limits" → stores which value
   (by id) and its limits; the name and kind are read from the parameter (a rename follows, 27); an
   exposed dimension stores its own name (UX 12.1).
2. Criteria 23, 25: Count depends on `0066` revision 6 (Draft); until then length and angle only.
   Scalar is not in the criteria (UX 12.1 proposes it): add it or leave it out (default out).
3. Criterion 26: "keeps its last valid geometry" needs the file rule of A5 (Frage-ready, default
   A: the last solvable values are saved with the clone).
4. Criteria 15, 16, 17 with 26 to 29: Convert to paths and Delete on the sketch page turn every
   clone that holds own values into its own path in the same step (A6); add one sentence each.
5. Criterion 29: Unlink of a clone also turns the clones below it that hold own values into paths
   (A6); the others follow the new path.
6. Criterion 29 / `0024` criterion 59: a clone with own values is exported as a path, never as a
   `<use>`.
7. Criterion 27: Stop exposing drops every clone's own value for it in the same step (Undo
   restores); a deleted parameter or dimension does the same by read.
8. Criterion 30: the change is three registers (`exposed_params`, `instance_params`,
   `instance_fallback`); "its clone matrix and override flags as of 0057" are unchanged. The
   Links "Format" line needs a revision entry: 62.
9. Criterion 24 "and one for all": UX 12.2 folds it into the Original's segment; align the text.
10. Criterion 31 budgets the drag step only; say that file open and the release commit are
    measured, not budgeted.

### A11. Against UX notes §12 (as of 2026-11-25)

- U1, 12.1 "Off: the fields leave the tree, their values kept and back when on again": Stop
  exposing removes the entry, so limits and a dimension's name are not in the file while off;
  they come back by Undo, and the shell may keep them as view memory until the selection changes.
  Storing an "off" entry would be a further format element for a convenience (rejected).
- U2, 12.1 a dimension's Name edit has no step name; it shares the key `parameter_limits`
  (wording for the UX notes, e.g. "Change exposed parameter").
- U3, 12.1 Scalar exposure is not in this model (A2); the UX notes keep it only if the PO adds it.
- U4, 12.4: the "last valid geometry" of the canvas and of the cut is A5's stored fallback, so it
  is the same after a reopen and on every device; with A5 option B the dashed clone shows the
  original's shape instead.
- Everything else of §12 (Own writes all current values, Original's drops all, per-key Back,
  muted inherited values, bare-name knob rule, value in sketch units) fits A2 to A7 as written.

## Addendum A12ff, 2026-11-25: slice 2 (handles and links)

Architect, 2026-11-25. Read against the branch after PR #482 (`CURRENT_FORMAT_VERSION` 62 on main
and here), criteria 24, 25 and 31 to 37 as the product owner drafted them on 2026-11-25, UX notes
12.3, the generator knobs (`curvyo-ui-core/src/instance_handles.rs`,
`curvyo-editor-wasm/src/session/generators/handles.rs`), the sketch arm
(`curvyo-ui-core/src/instance_sketch.rs`, `curvyo-editor-wasm/src/session/sketch_instance.rs`) and
document-core's `instance_codec.rs`, `instance_commands.rs`, `instance_value.rs`,
`sketch_instance.rs`, `clone_read.rs`, `clone_preview.rs`. Frage 373 B and 374 B are the customer's;
everything else is **(Proposal)**.

### A12. Canvas handles (criterion 25, 31 re-run)

- **One knob type, two arms (ADR 0080 §6).** `instance_handles.rs` keeps `Knob` and `dragged_mm`;
  `Knob` changes `key` and `label` from `&'static str` to `String` (sketch keys are runtime
  strings) and gains `motion: KnobMotion { Along { direction: Vec2, rate: f64 }, Around { centre:
  Point, sign: f64 } }` with `value: f64` (mm or degrees) in place of `value_mm`; the generator's
  knobs become `Along`. `dragged_mm` becomes `dragged(knob, from, to, snap: bool) -> f64`: `Along`
  projects the pointer's move on `direction` times `rate`; `Around` adds the signed angle swept
  around `centre` since the press, times `sign`. The snap **modifier is chosen by the session per
  arm** (generators keep Shift, `0141` §9; sketch clones Ctrl/Cmd, criterion 25), the math only
  takes the bool.
- **Anchors** in a new `curvyo-ui-core/src/instance_sketch_handles.rs` ("Where each exposed value
  of a sketch clone puts its knob"): `knobs(sketch: &SketchSnapshot,
  solved: &Sketch, clone: &CloneSnapshot, view) -> Vec<Knob>`. It needs the sketch page's
  dimension geometry, today `pub(super)` in `curvyo-editor-wasm/src/session/sketch/dimension_geometry.rs`:
  that file **moves to ui-core** unchanged (`curvyo-ui-core/src/sketch_dimension_geometry.rs`, a
  `refactor` commit first; it only uses sketch-core, document-core and ui-core types; the sketch
  page calls it from there). The clone arm calls it with `placement = composed · sketch.placement`
  and lays it out with `layout_dimensions` and the dimension's stored label (`SketchSnapshot::labels`,
  mapped the same way), so the knob and the hover guides (UX 12.3) are the sketch page's own marks:
  - `Linear` / `Projected` (length of a line, distance, horizontal, vertical): the second
    arrowhead's tip, `Along` the measured direction.
  - `Radius` (radius, a circle's length, ellipse radius): where the leader meets the curve,
    `Along` `toward`; `Diameter`: the same point, rate doubled.
  - `Angle` (angle, axis angle): the second arrowhead's tip on the dimension arc, `Around` the
    vertex, `sign = -1` when `det(composed) < 0` (a mirrored clone).
  - Sweep has no dimension geometry (structural): field only; a count: field only.
  - **Rate in sketch units:** `rate = base / |composed · placement · u|` for the local unit
    direction `u` of the measurement (`base` 1, 2 for a diameter), so a 50 % clone gives 20 mm per
    10 mm and a non-uniform scale is right along the knob's own direction.
  - A `p:` key gets the knob of the one driving dimension whose expression is exactly
    `Expression::Reference(id)`, none when zero or several (criterion 25); a `d:` key whose own
    expression is not a literal still gets its knob (the own value replaces it, A2).
- **Solved geometry, one solve per step.** `Document::instance_solution(clone: NodeId, own:
  Option<&ValueSet>) -> Option<Arc<Sketch>>`: the solved graph for the clone's committed set
  (`None` own) or a preview set, through the same `InstanceSolves::solved` entry that
  `instance_preview` fills, so anchors at rest and the moving partner's knob during a drag (33)
  cost no second solve. An empty set is the sketch's own `solution`. The knob memo (as
  `KnobMemo`, keyed by clone id, `Arc::as_ptr` of `root_outline`, `composed`, own values) lives in
  a new `curvyo-editor-wasm/src/session/sketch_instance_handles.rs`.
- **Hit test, 72 px, collisions, readout, draw list, Escape**: session, shared with the generator
  arm. `generators/handles.rs`' `visible_knobs` rule (72 px on the shorter box side, 16 px
  collisions with earlier knobs and transform glyphs, none during a transform drag), `knob_at` and
  the draw list become functions over `&[Knob]` used by both arms; each arm only supplies its raw
  knobs and box. Knob press order in the pointer router: generator knob, then sketch knob, then
  Select. A double-click on a knob opens the entry chip of `0052` with one field (the knob wins
  over Select original, criterion 13).
- **Drag step.** Each move calls `instance_sketch::plan(document, clone, key, number, only_this)`
  (A14), which asks `Document::instance_preview` for the one clone: one solve per step for the
  whole link group (one value set), no write. The knob drag uses the slice-1 slot
  `InstanceState { previews, last_solvable }` of `session/sketch_instance.rs` (not the generator's
  `Preview`), with a new `overlay: bool` so a knob drag draws blue-new over black-old (UX 12.3) and
  the field drag keeps the in-place look it has. Release = `instance_sketch::change` with the
  drag's `last_solvable`, one step "Change parameter"; Escape clears the slot. CPU test: the slice-1
  `sketch_instances_cpu_time.rs` gains the knob drag and a linked drag (criterion 37: one solve per
  step, the 199 other `root_outline` pointers unchanged).

### A13. Links: data model and codec (`format_version` 64)

**Link groups on the sketch.** Each `exposed_params` entry may carry `"link": "<group>"`, a
string of 1 to 64 characters; entries with the same string form a group. A new group takes a
string no stored entry uses (raw items included, since an invalid `link` is kept): the starting
entry's key when free, else `<key>#2`, `#3`, ... (no id allocation; the list is one LWW value, so
checking the current list is enough). Rejected: (B) a separate
register `exposed_links: [[key, key], ...]`: it can name keys that are no longer exposed and races
with `exposed_params` under LWW, while a field on the entry leaves with its entry in the one
written list; (C) a link stored on each clone: the customer wants it set once in the sketch
(Frage 374 B).

```text
exposed_params : [ { "key": "d:c7", "name": "Radius 1", "link": "d:c7" },
                   { "key": "d:c8", "name": "Radius 2", "link": "d:c7" } ]
```

- Read: `ExposedEntry` and `ExposedParam` gain `link: Option<String>` (the decoded string) and
  `ExposedParam` gains `group: Option<usize>` (resolved per read). A group's kind is the kind of
  its first member in list order; a member of another kind is not in the group. A group of one
  reads as no group. A `link` that is not a valid string is ignored and kept.
- **Hazard fixed in this slice:** `set_exposed_limits` rebuilds the entry through `decode_entry`
  and `ExposedEntry::to_json`, which drops every field it does not know. `to_json` writes `link`,
  and the rebuild **merges into the raw map** of the stored entry, so later fields survive.
- Commands (`instance_commands.rs`, one commit each): `link_exposed(sketch, a, b)` "Link
  parameters" (both of one kind, else `InstanceError::NotLinkable`; neither grouped: both start a
  new group; exactly one grouped: the other joins that group; both in different groups: `b`'s
  whole group joins `a`'s);
  `unlink_exposed(sketch, key)` "Unlink parameters" (removes its `link`; a group left with one
  member loses that link too). In both cases the same commit drops the `instance_unlinked` marks
  of keys that leave a group (35).

**Unlinked state on a clone.** A new clone register, an LWW **list value** of keys written whole
in the same commit as `instance_params`:

```text
instance_unlinked : ["d:c8"]   keys this clone changed alone (Shift); absent or [] = none
```

Rejected: a flag inside the value (`{"mm": 82, "unlinked": true}`): it breaks the one encoding
`ParamValue` and `instance_fallback` share (A3), and `decode_value` refuses a two-key map, so
every reader of that encoding would need a second shape.

- Invariants: unlinked keys are a subset of the clone's own keys and are members of a group of two
  or more; **per clone, never inherited** (36): a clone below reads only its own list. Read
  ignores and keeps a key that does not parse, has no own value on this clone, or is in no group.
  `CloneChain` gains `unlinked: KeySet` (the filtered own list; `KeySet` is a `BTreeSet<ExposedKey>`
  newtype in `instance_value.rs`); `InstanceWrite` gains `unlinked: KeySet`.
- **Damage:** `instance_unlinked` present on a clone and not a list value refuses the file:
  `instance_codec::clone_shape_ok` checks the third register (`clone_codec::validate_clone_node`
  already calls it). Residue on a non-clone node: never read, as A3.
- **Stop exposing (35):** `unexpose_parameter` also drops the key from every clone's
  `instance_unlinked`; if its group is left with one member, it removes that member's `link` and
  every clone's mark for it, in the same commit. **A deleted parameter or dimension** applies 35
  **by read** (its entry is residue, the group shrinks, a group of one is none, marks of a key in
  no group are ignored); the sketch edit commands write nothing more (A6 precedent; Undo restores).
- **Chains, Convert, Unlink, export:** values are already per clone in the read (A4), links change
  no geometry: Unlink and Convert also remove `instance_unlinked`; the follower rule (A6),
  `as_use` and output are unchanged.
- **`format_version` 64** (next free after 63, which is 0118 slice 2d; renumberable at rebase). Links change no geometry at
  read: a coupled edit writes the own value of every member, so a 63 build opening a 64 file
  draws the same shapes. It must still refuse: its edits would couple nothing (members drift
  apart while shown linked) and its limit edit (`to_json`) strips the `link` fields. The
  `CURRENT_FORMAT_VERSION` doc comment gets the paragraph. Goldens in
  `curvyo-document-core/tests/fixtures/`: `sketch_instance_links_v64.curvyo` (a sketch with a
  group of two lengths and a group of two angles, one entry with an invalid `link` that is kept;
  clones: none own, coupled own values, one with a key in `instance_unlinked`, a chain whose second
  clone has its own coupled values) and `sketch_instance_links_damaged_v64.curvyo`
  (`instance_unlinked` as a container). A unit test covers the residue cases (mixed-kind member,
  group of one, mark without own value: opens, reads as stated, saves back equal).

### A14. The edit in core: one function

New pure module `curvyo-document-core/src/instance_links.rs` ("Applies one value change to a
clone's own values through the sketch's link groups"):

```rust
pub struct LinkedChange { pub own: ValueSet, pub unlinked: KeySet, pub held: Option<Held> }
pub fn linked_change(exposed: &[ExposedParam], own: &ValueSet, unlinked: &KeySet,
                     key: &ExposedKey, number: f64, only_this: bool) -> LinkedChange
pub fn linked_reset(own: &ValueSet, unlinked: &KeySet, key: Option<&ExposedKey>)
                     -> (ValueSet, KeySet)
```

- Targets: `key` alone when `only_this`, when `key` is in no group, or when `key` is unlinked on
  this clone; else `key` and every member of its group that is not unlinked on this clone.
- Clamp: `number` into the intersection of the targets' limits (one value for all, so a partner
  never leaves its limits); `Held` names the bound that held for the notice (the edited value's
  label, A7 wording). An empty intersection (a member's minimum above another's maximum): each
  target is clamped into its own limits and they differ (permit over refuse); finding F2.
- `only_this` adds `key` to `unlinked` only when the value actually changes and `key` is in a
  group of two or more (34), so the writer never stores a mark the read ignores.
- `linked_reset(Some(key))` (Back): drops `key`'s own value and its mark; partners untouched (34).
  `linked_reset(None)` (Original's): empty set, empty marks. `own_all` (Own) keeps the marks.
- `instance_sketch::plan`, `change`, `reset`, `own_all` call these instead of their own
  `own.insert` / `remove`; `plan` and `change` gain `only_this: bool`; several clones are planned
  one by one, each coupled inside itself (33). `InstanceSource::Sketch` in `instance.rs`'
  `change_parameter` passes `only_this = false`.

### A15. Slice-1 leftovers folded in

- **(a) Move preview of a no-solution clone** (`docs/technical-debt.md`, entry of 2026-11-25).
  `RawLeaf::of_clone` sets `fallback: ValueSet::default()` because `CloneChain` holds no fallback.
  Fix: `CloneChain` gains `fallback_values: ValueSet` (the decoded `instance_fallback`, filtered
  like `own_values`), `of_clone` copies it and `unlinked`. Found with it: `clone_previews`
  (`clone_preview.rs`, called from `session/clone_view.rs` and `session/sketch/draw.rs`) and the
  limit check in `clone_write.rs` run `resolve(leaves)`, i.e. a **fresh** `InstanceSolves`, so each
  move step re-solves every downstream clone with own values. They switch to `resolve_with` and
  the document's cache (`Document::clone_previews` wraps the free function; the session calls the
  method). The debt entry is removed in the same PR.
- **(b) The pill reads Own during a drag.** `instance_sketch::view` counts `own_params` over
  `clones_of(document, ids, stepped)`, which already holds the preview. Fix: `state`, each row's
  `origin`, `chained` and the new `unlinked` flag come from the committed clones
  (`clones_of(document, ids, &[])`); only `value` and `mixed` follow `stepped` (criterion 24).

### A16. Findings for the product owner and UX

1. Criterion 37's reason for refusing is not right: a 63 build would draw the same shapes (the
   own values are written per member). It refuses because its edits would not couple and a limit
   edit strips the links (A13).
2. Criterion 33: say what happens when the members' limits do not overlap (A14: each clamped into
   its own, they differ, no refusal).
3. Criterion 34 covers Back on an unlinked value; say the same for Back on a linked value (only it
   goes back, the partners keep theirs).
4. Criterion 32: linking two values that are both already in different groups merges the groups
   (A13), or the control is off; pick one (default merge).
5. Criterion 25: the generator knobs of `0141` keep Shift for whole millimetres; sketch knobs use
   Ctrl/Cmd. Two modifiers for one knob look; UX may want `0141` to follow (a separate change).
6. `ValueField` steps without modifiers today, so Shift on arrows (34) collides with nothing; UX
   notes should name Shift+arrow and Shift+field-drag as "only this".
7. UX 12.3's knob for a `d:` with a formula: the knob stays (A12); criterion 25 says "field only"
   only for a `p:` used by a formula or several dimensions. Consistent; mention it in the UX notes.

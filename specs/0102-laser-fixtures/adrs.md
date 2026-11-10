# 0102 laser fixtures: architecture decisions

Status: Draft, revision 2 (architect, 2026-11-10; aligned with `ux-notes.md` of the same day and the lead's decisions D24 to D30). Revision 1 was written on Frage 171 to 175. Revision 2 takes the customer's feedback of 2026-11-10 on PR #313: a fifth kind `screw` (head and thread diameters, round, concentric); holder, clamp, stop and screw sized and placed on the canvas with the rectangle's handles and entry chips; no size or position field of these kinds in the Fixtures block; the Select bar shows nothing for them. The two new ADRs are proposed and need the customer's acceptance at the demo, as document-model and persistence decisions do. Numbers 0049 and 0050 are requested as reservations on `main` (see §11).

## 1. ADRs this feature depends on, extends or amends

- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md) (new, proposed): five kinds, slots as child nodes, `active`, `fixture_matrix` (rigid), the snap switch register, `format_version` 40, the goldens.
- [ADR 0050](../../docs/adr/0050-laser-fixture-set-in-machine-profile.md) (new, proposed): profile record format 2 with the fixture set (five kinds), pick and reset copy, the size limits.
- ADR 0032 §8: the hit rule of criteria 8 to 11 is an amendment (on acceptance, a note on its Status line).
- ADR 0033 §2 and §7: new fixture keys and the bump; §3 unchanged (the kinds are opaque to document-core).
- ADR 0034 §1 to §3: laser-core owns fixture meaning; ui-core depends on it already; no new edge. ui-core learns no fixture kind.
- ADR 0038 (record format 2) and ADR 0039 §4, §5: notes on acceptance of ADR 0050.
- ADR 0042 §4 and ADR 0046 §6: a fixture is never an output leaf; the child-node precedent and the bump precedent.
- ADR 0014: undo of a removed node restores its children (slots) in one step; nothing new.
- ADR 0060 settings pattern: `snapping_fixtures` is one more root register.
- ADR 0008 (22.5 degree rotation snap) and ADR 0009 (the handle model, the parameter knob): reused as they are; no amendment. Handle sizes and hit radii are those of `curvyo-ui-core` (`docs/design-system.md`).

## 2. Storage (ADR 0049)

- Slots are child nodes, not an atomic list (A rejected: two peers editing two slots lose one edit). Slot values are one atomic map per slot.
- The position is a separate `fixture_matrix` register, not part of `fixture_params` (a move and a size edit must not race each other).
- A screw is a `fixture` node with no children, like a grid: `fixture_params` carry `thread_diameter_mm` and `head_diameter_mm`; its matrix places its **centre** (frame origin at the centre, §6a.3). No new node shape for it.
- `format_version` **40 is needed.** A new node shape (`fixture_slot`) under the manufacturing; build 39 would keep or drop it by accident. No migration; a 39 file opens unchanged. The screw adds a kind value only and does not change the bump. `active` alone would not have needed a bump.
- Goldens (revision 2), in `curvyo-document-core/tests/fixtures/`:
  - `fixtures_v40.curvyo` holds **the screw**: one of each of the five kinds, so one screw node with its params and matrix. The round trip of criterion 31 covers it.
  - `fixtures_damaged_v40.curvyo` holds a screw with text in `thread_diameter_mm` (criterion 33).
  - `fixtures_unknown_kind_v40.curvyo`, `fixtures_slot_orphan_v40.curvyo`, `fixtures_grid_v39.curvyo` as before.
  - The existing goldens `laser_workbench_v35`, `laser_workbench_v36`, `machine_profile_v36`, `manufacturing_unknown_process_v35` and `operations_*_v39` stay and must open unchanged.

## 3. Profile and fixture set (ADR 0050)

- With Frage 175 A the profile record gains `fixtures` (format 2). A format 1 record reads as the empty set and is never rewritten on read.
- The set holds the five kinds. A screw entry carries `thread_diameter_mm`, `head_diameter_mm` and its matrix, no slots (ADR 0050 §1).
- Pick profile and Reset from profile replace the page's fixture nodes in one commit with fresh node ids; one undo step restores the old nodes with their ids. Fresh ids are safe: no membership key and no other record refers to a fixture id.
- A damaged set makes the whole profile record damaged (skipped with its notice). This is stricter than the per-field defaults of the page copy, deliberately (ADR 0050 §2).
- Generic laser holds an empty set. Picking it on a page with fixtures removes them, undoable; the customer should see that at the demo.
- The set includes the matrix (position and angle); the eye and selection stay out (ADR 0050 §1).

## 4. Command labels, step scope and the edit context (`0106`)

Fixtures are not edits. Three facts decide it: a fixture is never in a selection with other objects (criterion 8), so the move-only guard of `history.rs` cannot see a fixture step; fixtures are reachable only on the active laser page, which is in context by definition; and the fixture labels are already in `FREE_STEPS`. EDIT would wrongly refuse a fixture step from an out-of-context move-only selection, which cannot occur. The only move that belongs to `MOVE_STEPS` is the placed page's, because it moves an object of the selection.

The undo names are those of `ux-notes.md` §12, which is the authority for names; `history_names.rs` holds one name per label.

| Commit label | Undo name | `step_scope` list | Note |
|---|---|---|---|
| `add_grid`, `add_holder`, `add_clamp`, `add_stop`, `add_screw` | Add grid / holder / clamp / stop / screw | FREE | replace `add_fixture`; one name per label |
| `delete_fixture` | Remove fixture | FREE | existing; removes the slot children in the same commit |
| `rename_fixture` | Rename fixture | FREE | new |
| `set_fixture_visible` | Show or hide fixture | FREE | the eye is stored (`visible`, §9) and is one undo step per fixture, as criterion 6 says and `0093` shipped (lead, 2026-11-10: the spec wins over the earlier "no step") |
| `set_fixture_active`, `set_fixture_inactive` | Set fixture active / inactive | FREE | new, two labels for two names |
| `set_grid_spacing`, `set_grid_major`, `set_tape_edge`, `set_grid_origin` | Set grid spacing / Set grid major / Set tape edge / Set grid origin | FREE | **replace `set_fixture_params` for the grid**; one label per field, as the names of `ux-notes.md` §12 |
| `resize_fixture` | Resize fixture | FREE | **new.** Holder, clamp, stop (legs and thickness) and screw (head and thread diameters), from any route: handle drag, double-click chip, S chip. Writes `fixture_params` and `fixture_matrix` in one commit, and the slot centres of a holder whose origin moves (§6a.5) |
| `move_fixture`, `rotate_fixture` | Move fixture / Rotate fixture | FREE | matrix only; from a handle drag, the centre chip (M) or the Angle chip (R) |
| `add_slot`, `remove_slot`, `rename_slot`, `set_slot_shape`, `set_slot_position`, `set_slot_size` | Add slot / Remove slot / Rename slot / Set slot shape / Set slot position / Set slot size | FREE | new; the block's slot fields |
| `move_to_slot` | Move to slot | MOVE | new; moves a placed page or clone, an object of the selection |
| `pick_profile`, `save_as_profile`, `edit_profile_name` | existing names | FREE | no change |
| `reset_profile` | Reset fixtures from profile | FREE | the name of `ux-notes.md` §12 |
| `set_fixtures_snap_on`, `set_fixtures_snap_off` | Snap to fixtures on / off | FREE | replace `set_document_snapping` for the Fixtures switch (the other switches keep theirs) |

Each label is in exactly one list (the test `every_command_label_is_classified_once` enforces it) and names a known label in `history_names.rs`. Every fixture label, the eye's included, is one step.

**Edit context.** `session/edit_context.rs` is not changed. Its pickable targets never include a fixture. A fixture press is resolved first by the selected fixture's handles (`session/fixtures/handles.rs`), then by the outline (`hit.rs`), and only while the active page is a laser page (criterion 11); otherwise the press goes to the page as before.

## 5. Crate placement: fit and fixture snap points (no new crate)

Decision: **`curvyo-laser-core`** owns the fixture model, the snap points of the fixtures and the fit rule. `curvyo-ui-core` owns only the generic snapping mechanics and the generic box handles, which learn no fixture kind.

Why not ui-core: the fixture meaning (kinds, board frame, slots, the 0.1 mm fit rule, the 90 degree turn) is the laser extension's, as ADR 0034 §1 already puts the fixture outlines there; ui-core must not learn fixture kinds (ADR 0034 §2: ui-core dispatches to extensions). Why not a new crate: no ADR asks for it, and the edge already exists (`ui-core → laser-core`).

Real file names in `curvyo-laser-core/src/` (revision 1 said `fixture_spec`; the code has these):

- `fixture_kind.rs`: `FixtureKind` (adds `Screw`), `from_stored`, `stored`, `word`, the ranges (adds `screw_thread_range()` and `screw_head_range()`, both 0.5 to 1,000 mm, defaults 6 and 10 mm; §6a.6). This file is also the "spec" of revision 1.
- `fixture_slot.rs`: `SlotShape`, slot values. No change.
- `fixture_stored.rs`: node values to the typed spec, with damage counts (adds the screw's two params, the thread-not-above-head relation, and the rigid-matrix check). Used by the page and the profile.
- `fixture_snap.rs`: snap targets. **Screw: none** (criterion 40; not even its centre). Slot, board and stop targets unchanged; clamps none.
- `fixture_fit.rs`: `fit_slot(item_size, slots, tolerance) -> Option<FitMatch { slot, turned }>`. Unchanged: a screw is not a fit target.
- `machine_profile_fixtures.rs`: the profile's fixture entries (adds `screw` to the kind list and its two params).
- `grid_layout.rs`: the engraved grid, extended with major lines and tapes. Unchanged by revision 2.

`curvyo-ui-core` (no new module, no fixture kind):
- Reused unchanged: `transform_handle_layout.rs` (`transform_handles`, `hit_transform_handle`, `resize_handle_local_position`, `rotate_handle_local_position`, `EditHandle`), `oriented_box.rs` (`OrientedBox`), `transform_math.rs` (`resize_local_box_with_prior`, `local_delta_for_size`, `ResizedBox`, `rotate_delta_angle`), `transform_entry.rs` (`parse_entry_number`, `EntryOutcome`, `InvalidReason`). The nine-point Reference of the Move chip: `reference_point.rs` (`ReferencePoint`), if its functions take a box; if not, the nine points are one small function in `fixtures/entry.rs`.
- Visibility only (these become `pub` so `curvyo-editor-wasm` can use them): `transform_entry::EntryField::for_param` (today `pub(crate)`), `transform_entry::format_mm` (today `pub(crate)`), `param_handles::CENTRE_YIELD_PX` (today `pub(crate)`). The knob constants `KNOB_DIAMETER_PX`, `PARAM_HIT_PX`, `PARAM_MIN_SIDE_PX` are already `pub`.
- Snapping: one new target kind `Fixture` in `snapping/targets.rs`; a **lattice** input in `snapping/grid.rs` for the engraved grid (nearest intersection by rounding per axis). Capture, release, hysteresis and Alt stay as `0060`.
- Fit tie rule (Proposal, spec change 6): a square item in a square slot is never turned; a turn happens only when the other orientation is the fit.

## 6. Session shape (feature-owned, one field)

- `Session` gets **one** field, `fixtures: fixtures::FixtureState` (review finding 1). Module `curvyo-editor-wasm/src/session/fixtures/`. Revision 2 changes:
  - **new** `handles.rs`: `FixtureHandle` (an `EditHandle`, or the knobs `Head`, `Thread` of a screw and `Thickness` of a stop), the handle set per kind (§6a.1), the fixture-to-`OrientedBox` adapter (§6a.2), the knob geometry, the hit test of the selected fixture's handles.
  - **new** `edit.rs`: the one resolver for drag and typed entry: `resize`, `rotate`, `translate`, `knob`. Each returns a `FixtureEdit` (params, matrix, optional slot centres). Pure, no document access.
  - **new** `entry.rs`: `FixtureEntry` (the open typed entry of one handle or chip: fields, start box, kind), its `EntryView` (§6a.4), its commit through `edit.rs` and `commands.rs`.
  - **changed** `drag.rs`: `DragMode` is replaced by the `FixtureHandle` of the press (Move, Rotate, Resize, the knobs; Hold for the grid). `FixtureDrag` keeps `start`, `start_matrix`, `live`.
  - **changed** `hit.rs`: keeps the outline hit (criterion 8, 4 and 12 px; a screw on either circle). Its own rotate glyph and its `ROTATE_OFFSET_PX` and `ROTATE_HIT_PX` are deleted; handles come from `handles.rs`.
  - **changed** `commands.rs`: the grid commands keep their own functions; one commit function `write_fixture_edit(id, edit, label)` for all geometry; the label is chosen by the handle (§6a.5).
  - **changed** `view_json.rs`: see §6a.7 (fields removed and added).
  - **changed** `draw.rs`: the selected fixture's handles and knobs are drawn with `transform_handles` (skew filtered) and the knob constants; the screw's two rings and the stop's bar are drawn as the look of `ux-notes.md` §5.
  - **changed** `keys.rs`: M, R and S open the chips of the selected fixture (§6a.4; no R on a screw).
  - **changed** `params.rs`: the screw's two keys, the limits and the relation of §6a.6; `LengthField` reused for the knobs and the chips.
  - `snap.rs` and `fit.rs` are unchanged (a screw is in neither). `placed.rs` gets the screw `Shape` variant.
- Cross-feature transitions are named, not spread: object selection clears a fixture selection in `select.rs`; leaving a laser page clears it in `pages.rs`. Nothing else reads the fixture state. `FixtureState` gains one field, `entry: Option<FixtureEntry>`, cleared with the selection.
- The fixture command glue now in `session/manufacturing/commands.rs` moves into `session/fixtures/commands.rs` in the first commit of the story, as a behaviour-preserving move (unchanged in revision 2).
- document-core: fixture node reads and writes go to a new module `curvyo-document-core/src/manufacturing_fixtures.rs`. `manufacturing.rs` is already 626 lines, over the §5 limit, and does not grow. The document-core commands clamp the screw's thread to its head (§6a.6), so the rule holds for every writer, not only the UI.

### 6a. Handles, entries and commits: the rectangle's model (revision 2)

**Decision: reuse the box layer of ui-core and the entry chips; do not reuse the Select tool or `TransformEntry`.** The Select tool and its typed entries are bound to `ObjectSnapshot` and to the object selection: `hover_handle_at` and `double_click` take object lists, `TransformEntry` stores an `ObjectSnapshot`, commits go through `commit_gesture` and `still_current` on document primitives. A fixture is not an object and must never enter the object selection (criterion 8). Wrapping fixtures as `ObjectSnapshot`s would couple every one of those paths to fixture nodes. The reuse is one level lower: the box, the handle layout, the hit test, the resize arithmetic, the entry fields and the chip.

Why this is not a copy: the fixture code of revision 1 had its own rotate glyph (24 px offset, 10 px hit), its own drag modes and its own commits. Those are deleted; the rectangle's constants and rules come instead.

**6a.1 Handle sets per kind** (`ux-notes.md` §6.2 and §4.3). `handles.rs` builds the set from `transform_handles(box, spec, tolerances)` and removes every `EditHandle::Skew` (no fixture is skewed; `HandleSpec` and its 18 constructions stay unchanged).

| Kind | Resize | Rotate | Knobs | Centre (Move) |
|---|---|---|---|---|
| Holder | 8 squircles: corners and edge midpoints (W and H) | 4 corners; Shift adds the 4 sides | none (D22) | yes, from a side of 48 px |
| Clamp | as holder | as holder | none | yes |
| Stop | **Leg 1** squircle at the free end of leg 1, on its centreline; **Leg 2** squircle at the free end of leg 2 | 4 corners | **Thickness** knob, on the top edge of leg 1 at its midpoint (D23) | yes |
| Screw | none (D25) | none (a round part looks the same turned) | **Head** knob at 0 degrees (east) on the head circle; **Thread** knob at 180 degrees (west) on the thread circle | yes, from a head of 48 px |
| Grid | none | none | none | none (criterion 13) |

Squircles are hidden, not disabled, when a side is too short (24 px rule of the rectangle). Knobs are drawn when the box's shorter side is at least `PARAM_MIN_SIDE_PX` (72 px on screen; for a screw, the head); with the knob diameter `KNOB_DIAMETER_PX` and hit radius `PARAM_HIT_PX` (12 px). The centre handle yields to a knob within `CENTRE_YIELD_PX` (20 px). The two screw knobs sit at 0 and 180 degrees, so they never meet. The thread circle is drawn at 4 device px across or more; the head always.

**6a.2 Box adapter.** A fixture's frame has its origin `o = (e, f)` of its matrix; its box is given in frame coordinates relative to `o`. `OrientedBox` gets `pivot = o`, `angle = atan2(b, a)`, and `min`, `max` as `o + (x0, y0)` and `o + (x1, y1)`:
- holder, clamp, stop: the origin is the top-left corner, so `min = o`, `max = o + (w, h)`;
- screw: the origin is the centre, so `min = o − (d/2, d/2)`, `max = o + (d/2, d/2)` with `d` the head diameter.

For a rigid matrix (§6a.6) this is exact: `to_document(q) = o + R(q − o)` is the fixture's own map. The inverse after an edit: the new matrix is `[cos, sin, −sin, cos, o'x, o'y]` with `o' = box.to_document(new_origin)` where `new_origin` is the new frame origin in box coordinates. The adapter never writes a scale or a skew.

**6a.3 Stop and screw geometry (local frame).**
- Stop: the origin is the outer corner. Leg 1 is the band `y ∈ [0, t]` from the origin to `x = leg_a`; leg 2 is the band `x ∈ [0, t]` from the origin to `y = leg_b` (as `placed.rs` already draws the L). Leg 1's squircle is at `(leg_a, t/2)`, leg 2's at `(t/2, leg_b)`, its thickness knob at `(leg_a/2, 0)`. Dragging leg 1 or leg 2 changes its length; the origin does not move.
- Thickness knob: the outer (top) edge of leg 1 and the outer edge of leg 2 move with the thickness and the inner edges (the inner corner) stay, so the frame origin moves by the change on both axes; the knob sits on the top edge of leg 1 at `(leg_a/2, 0)` and moves perpendicular to it; the legs keep their lengths (criterion 23, D23; lead, 2026-11-11).
- Screw: the origin is the centre. The head circle has radius `head/2`, the thread circle `thread/2`, both about the origin. Head knob at `(head/2, 0)`, thread knob at `(−thread/2, 0)` in the frame. Because the origin is the centre, changing a diameter never moves the centre; no slot or origin rewrite is needed.

**6a.4 Entries and keys** (`ux-notes.md` §4.3, §10; D27). Double-click on a handle opens its chip with the rectangle's rules for the handle's first press (`last_press_handle`). The chips:

| Route | Holder, clamp | Stop | Screw |
|---|---|---|---|
| Double-click a squircle | Size chip: a corner "W" and "H", an edge its one axis ("H" for n and s, "W" for e and w), the opposite side or corner kept | Size chip, one field: "Leg 1" or "Leg 2" | — |
| Double-click the thickness or head or thread knob | — | "Thickness", one field | "Head diameter" or "Thread diameter", one field |
| Double-click a rotate glyph, or R | Angle chip, one 80 px field, "°", absolute, about the box centre | Angle chip | none (no R) |
| Double-click the centre, or M | Move chip: "X", "Y", Relative or Absolute, the nine-point Reference; no Copy (a fixture is not cloned) | same | same |
| S | Size chip, "W" and "H", about the centre (the rectangle's S, criterion 57a) | one chip with "Leg 1", "Leg 2" (row 1) and "Thickness" (row 2, 6 px below) | one chip with "Head" and "Thread" (Tab between) |

Fields come from `EntryField` and `parse_entry_number` (ui-core); numbers are written as `format_mm`; a value out of range takes the limit and notice of §6a.6. The chip is the frontend's `TransformEntryChip` fed an `EntryView`, with kinds `size` (holder, clamp, corner squircle), `angle`, `fixture-length` (one field: a leg, the thickness, a knob) and `fixture-size` (the S chips of stop and screw: several fields in one chip). The Move chip is `MoveEntryChip`. Undo of every chip commit is one step "Resize fixture", "Rotate fixture" or "Move fixture" as the route says. Delete on a grid shows its notice (criterion 13).

**6a.5 Hit, drag and commit.**
- Press on the canvas on the active laser page: objects first, as today (criterion 11). Then, if a fixture is selected, its handles: `hit_transform_handle(handles, box, p, tolerances, include_move = true)` on the list of §6a.1 (the rectangle's outer band and nearest-centre rule). Then the outline of a shown fixture (`hit_outline`, criterion 8: 4 px, 12 px coarse; a screw on either circle). A press inside a selected fixture's box, on no handle and no object, moves it (D13); inside an unselected fixture it passes through (criterion 9).
- Drag: `resize`, `rotate` (`rotate_delta_angle`, Ctrl snaps to 22.5 degrees), `translate` (with the fit and snap of criteria 25 to 27 for moves only, as now) and the knobs, computed by `edit.rs` for the live frame, which shows the ghost of the committed geometry (`ux-notes.md` §7). Nothing is written before the release. A drag at a limit stops there without a notice (D24).
- Release or Enter: one commit through `write_fixture_edit(id, edit, label)`: Move → `move_fixture`; Rotate → `rotate_fixture`; a squircle, a leg, a size chip, the S chip or a screw knob → `resize_fixture`; the stop's thickness knob → `resize_fixture` too (its outer edges and frame origin move, the inner corner stays). One undo step per commit.
- **A move of the frame origin writes the slots.** Whenever a holder's or clamp's frame origin moves (a resize from its N or W edge, or the S chip about the centre), the commit also writes each slot's centre: `cx' = cx − Δx`, `cy' = cy − Δy`, with `(Δx, Δy) = new_origin − old_origin` in the board frame. Each slot keeps its place on the sheet, not on the board's corner. The slots are in the same commit, so one undo restores them. A slot that leaves the board is allowed and noted (criterion 20).

**6a.6 Limits and relations** (`ux-notes.md` §4.4, D24, D26, D30). The typed value gives way; the other value never moves.
- Holder and clamp: width and height 1 to 1,000 mm (criteria 2 and 18).
- Stop: each leg 1 to 1,000 mm and at least the thickness; thickness 0.5 to 50 mm and at most the shorter leg. A typed leg below the thickness is set to the thickness; a typed thickness above the limit is set to the limit.
- Screw (**D30, chosen over the revision 1 values 1 to 100 and 1 to 150, which are dropped**): each diameter 0.5 to 1,000 mm; defaults thread 6 mm and head 10 mm (an M6 screw, criterion 37). The thread is never larger than the head (D26, lead's decision): a typed thread above the head is set to the head, a typed head below the thread is set to the thread, each with the Action notice of `ux-notes.md` §4.1 ("Thread diameter is 0.5 to 10 mm. Set to 10 mm."; the limit shown is the current one). A head drag stops at the thread and a thread drag stops at the head, without notice. **The document-core command clamps the same rule**, so a writer that is not the UI cannot store a thread above its head.
- Round only: no hex, no countersink (D30).
- On read (page), a head below the thread reads as the thread's value and counts as one damaged value (ADR 0049 §6). In a profile it is damage (ADR 0050 §2).
- Matrix: rigid, a rotation with translation (`a = d`, `c = −b`, `a² + b² = 1` within 1e-6); anything else reads as identity and counts as damage (ADR 0049 §6).

**6a.7 View contract (`view_json.rs`).**
- **Removed:** `FixturesView.selected_pose`, `Pose`, `pose_of`; `FixtureRow.size` and `FixtureRow.stop` with their types `SizeValues` and `StopValues`. The Select bar shows nothing for these kinds (`ux-notes.md` §8: hidden, not disabled), so there is no pose and no field to feed it.
- **`FixtureRow` keeps:** `id`, `name`, `kind` (`FixtureKindView`), `active`, `visible` (the eye), `grid: Option<GridValues>` (the grid's four typed fields and the tapes; the grid keeps its typed fields in the block, `ux-notes.md` §4.2), `slots: Vec<SlotRow>` (the slot list of a holder, each with its own typed fields), `can_add_slot`. A holder's word "· 2 slots" comes from `slots.len()`.
- **`FixtureRow` adds nothing for the screw.** Its row has no field (`ux-notes.md` §4.2); its sizes reach the user through the knobs, the chips and the announcer.
- **`FixturesView` adds one field:** `selection_announcement: Option<String>`, the `0044` announcer text of the selected fixture, built in Rust from its values: "Holder 1 selected, 100.0 by 60.0 mm.", "Stop 1 selected, legs 60.0 and 60.0 mm, thickness 5.0 mm.", "Screw 1 selected, thread 6.0 mm, head 10.0 mm." The frontend holds no sizes of its own.
- The live readouts ("⌀ 6.0 mm", "⌀ 10.0 mm max", "⌀ 6.0 mm min", D29) are the pointer readout of the drag, built in Rust with the rectangle's readout formats; they are not view fields.
- Named accessible labels of the chips are the frontend's, from `ux-notes.md` §11.

## 7. The JSON view contract (review finding 3)

- The Fixtures block is built by `session/fixtures/view_json.rs`. One decoder on the frontend reads it; a field the decoder cannot read decodes to its default, and an unknown kind decodes to "Unknown fixture", as `0101` does.
- One shared file, `curvyo-editor-wasm/tests/fixtures/fixtures_view_contract.json`, with one row of each kind (the screw has no size fields: its row is the name, the kind, Active and the eye), a circle slot, an unknown fixture, one damage notice and a `selection_announcement`. The Rust test `fixtures_view_contract` checks that the view builds exactly that JSON. A frontend test decodes the same file. No generated types (finding 3: simpler than maintaining them).

## 8. Budgets (thread CPU time, `curvyo-editor-wasm/tests/cpu_time/`)

The figures of spec criterion 35 hold; each has a mechanism:

- (a) Drag move, 16 ms: the drag's fixture targets (up to 60 slots, about 540 points) are built once at drag start. Each move scans at most 60 slots for the fit and rounds the lattice; the rest is the existing indexed resolver. **Revision 2:** the test measures a move after the start and a resize drag from the N edge of a holder with 15 slots (at most 60 slot centres recomputed per move, in memory; no document write during the drag).
- (b) Hit test with 50 fixtures, 2 ms: a bounding-box reject first, then the outline distance of the few that remain. **Revision 2:** handles are built and tested only for the selected fixture: at most 13 (holder: 8 squircles, 4 rotate glyphs, centre), 8 for a stop, 3 for a screw (two knobs, centre). The handle test adds no per-fixture cost.
- (c) Fixtures block, 50 fixtures, first rows 100 ms: one small JSON row per fixture, no geometry; the row is smaller than in revision 1 (no size fields).
- (d) Grid draw list at 1 mm over 1,000 x 1,000 mm, 16 ms: generated in laser-core and cached in the session by (parameters, origin corner, work area). The test measures a cache miss; a frame with no change does no work.
- (e) **New, revision 2.** A typed size entry or a release of an N-edge resize on a holder with 50 slots: the commit (one step, params, matrix and 50 slot centres) takes within 16 ms of thread CPU time. It is a write, not a drag frame; the test measures the commit function.

## 9. Decisions made here that the spec does not state (Proposal, vetoable at the demo unless marked decided)

- **The eye is stored and is one undo step per fixture** ("Show or hide fixture", criterion 6, as `0093` shipped). Revision 2 of this file said "no step"; the spec wins (lead, 2026-11-10), so no history-free commit path is needed. The eye is a stored `visible` on the node, so it travels with the file and merges between peers.
- Fixture commands are FREE (§4); the fixture selection is one field and never mixes with object selection.
- The grid's origin corner is the configurable origin corner of the 0090 grid (its `grid_origin` register) plus `origin_*_mm`; it is not read from the profile copy and not stored on the grid (Frage 174 B). All four tapes are on by default (Frage 174 C).
- `snapping_fixtures` absent reads on (criterion 24).
- **Decided (lead, 2026-11-10):** holder, clamp, stop and screw have **no size or position field in the Fixtures block**. Their sizes are set only by the handles, the double-click chips and the M, R and S chips (`ux-notes.md` D27). Only the grid keeps typed fields in the block, and the slots keep theirs. This revision supersedes the revision 1 choice to keep the block fields.
- **Revision 2:** the handles of holder, clamp and stop are the rectangle's, minus skew and corner knobs (§6a.1, D22). Stop: two leg squircles and a thickness knob, no corner or edge resize (D23). Screw: two knobs, the head at 0 degrees and the thread at 180 degrees, no rotate, no resize, no skew (D25).
- **Revision 2:** a screw has **no snap point** (not even its centre, criterion 40) and is **not a fit target** (no slot). Its eye has no snap to keep.
- **Revision 2:** a holder's or clamp's frame-origin move rewrites the slot centres in the same commit so that the slots keep their place on the sheet (§6a.5). Rejected: the slots moving with the board's corner, which pushes every slot when the left edge is dragged.
- **Revision 2:** the Select bar shows nothing for a fixture; no pose field remains (§6a.7).
- **Revision 2:** a matrix with scale or shear reads as identity and counts as damage (ADR 0049 §6); the handles can then never write one.
- **Revision 2, built:** the outer (top) edge of leg 1 and the outer edge of leg 2 move with the thickness and the inner edges (the inner corner) stay, so the frame origin moves by the change on both axes; the knob sits on the top edge of leg 1 at `(leg_a/2, 0)` and moves perpendicular to it; the legs keep their lengths (criterion 23, D23; lead, 2026-11-11).

## 10. Spec changes the lead must take into `specification.md` (the product owner edits it)

1. Criterion 2: five kind buttons, Grid, Holder, Clamp, Stop and Screw, at 44 px each (D28); a screw starts at a 6 mm thread and a 10 mm head, centred on the work area (D30). The Grid button stays hidden while a grid exists.
2. Criterion 6: the eye stays one undo step per fixture, as the spec says (§9; superseded the earlier "not undoable").
3. Criterion 8: a fixture is selected by its outline (a screw on either circle); with it selected its handles are hit before the outline; a press inside it and on no object moves it (D13).
4. Criterion 14: the typed X and Y of the Select bar do not apply to fixtures and the bar is hidden for them. Holder, clamp, stop and screw are moved, turned and sized by their handles, the double-click chips and the M, R, S chips (D27; "M opens the offset entry" reads "the Move chip"; "rotate handle" reads "the four corner rotate glyphs"; no R on a screw). Labels: "Move fixture", "Rotate fixture", "Resize fixture".
5. Criterion 18 and 23: the block shows no size or leg field for holder, clamp, stop, screw (D27, lead's decision). Criterion 23: the stop's handles (two leg squircles, the thickness knob on the top edge of leg 1, D23), and its limits (D24: a leg at least the thickness; thickness at most the shorter leg, or 50 mm).
6. Criterion 25: add the tie rule (a square item in a square slot is not turned). A screw is not a fit target.
7. Criterion 27 and 40: a screw is not a snap target at all; the spec's "centre" for a screw is dropped.
8. Criterion 28: the set holds the five kinds, the screw with its two diameters and its matrix.
9. Criterion 31 and the "Format" section: format 40 (decided); golden `fixtures_v40.curvyo` holds the screw.
10. Criterion 33: a screw's thread or head damaged reads as the default; a head below its thread reads as the thread and counts as damaged (§6a.6).
11. Criterion 37 (screw): **conflict with D26.** Criterion 37 allows a head smaller than the thread; the lead's decision is that the thread is never larger than the head. The spec must change criterion 37 to the lead's rule, or the lead records the conflict for the customer. Defaults and limits: 6 and 10 mm; 0.5 to 1,000 mm each (D30; the spec already says 6 and 10 mm).
12. Every place that lists the fixture kinds (the Fixtures block, the Layers rows, the profile's Machine block, and the "Changes to accepted specs" rows of `0093`, `0098` and `0026`): add the screw. The "Changes" table gains no new spec.
13. Out of scope (add): skew of any fixture; turning a screw; a screw as a fit target or a snap target; slots on a screw.

## 11. Numbers and reservations

- **ADR 0049 and ADR 0050 are free on every branch I checked** (`git ls-tree` over all remote and local branches).
- **ADR 0048 is taken**: `origin/story/pen-modes-variable-stroke` holds `docs/adr/0048-editable-curves-and-width-profiles.md`, accepted there on 2026-11-07. Main has 0047 last. The lead reserves 0049 and 0050 on main and checks again at merge.
- No `format_version` 40 reservation exists on any branch.

## 12. Technical debt to record (the architect does not edit `docs/technical-debt.md` in this draft)

- `curvyo-editor-wasm/src/session/mod.rs` is 1,948 lines and `curvyo-document-core/src/manufacturing.rs` 626 lines; both are over the 500-line limit of `CLAUDE.md` §5. This slice adds one field and a new module, and does not split them.
- The fixture code of revision 1 duplicated the rectangle's rotate glyph and drag (`hit.rs`, `drag.rs`). Removing it is part of this story, not debt.
- The stop's thickness move (§9) follows the spec: the outer edges move, the inner corner stays (lead, 2026-11-11).

# Plan for Rounded box (slice 1)

Implementer, 2026-11-24, branch `story/generators-flex-box`. Built under `specification.md` (revision 5),
`adrs.md`, ADR 0082 (revision 3) and `ux-notes.md` (revision 3). Order agreed with the customer: first the
`0141` revision 2 fixes (no Convert and no Joint work), then Rounded box slice 1 (no Detach part, no format
change). Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-generators-core`:** new crate-level `fingers.rs`, `contour.rs`, `footprint.rs`, `slit_bend.rs`,
  `wall.rs`, `wall_plates.rs` (ADR 0082 §4); `boxes/` (corner rule, uses `fingers.rs`); `gear/mod.rs` (resize);
  new `rounded_box/` (`mod.rs`, `schema.rs`); `manifest.rs`, `lib.rs`; goldens in `tests/fixtures/`.
- **`curvyo-extension-core`:** `param.rs` (`automatic` on `Length` and `Count`), `extension.rs`
  (`Generated.automatic`), `CHANGELOG.md` (source-breaking 0.x, ADR 0082 §7).
- **`curvyo-ui-core`, `curvyo-editor-wasm`, `frontend`:** registry already generic; the third segment, glyph,
  muted automatic values, Value-field sliders, shared operation (one leaf, nothing new).
- No new crate, no new dependency, no `format_version` change.

## Tasks

Part 0, `0141` revision 2 (criteria of `specs/0141-parametric-generators`):

- [x] 0.1 Corner rule: `fingers.rs` holds `finger_count` and the owner rule; `horizontal()` edges
  `even_solid: true`, the walls' horizontal edges `false`; tests: every plate outline simple at kerf 0 and 0.2
  for every lid and piece counts 1 to 9, every piece and corner cube owned once, mating test unchanged;
  re-bless `box_default.txt` and dependent goldens (0141-3).
- [x] 0.2 Gear resize writes Teeth: `Edit::Scale` sets `round(factor (z + 2) - 2)` clamped 6 to 200, keeps
  Module, Bore, Keyway, Rim, Hub; Bore knock-on clamp (0141-5).
- [x] 0.3 Generator block numbers are Value fields (slider rows) with tracks from the schema (0141-9).

Part 1, Rounded box slice 1 (criteria of `specs/0147-rounded-box`):

- [x] 1. Contract: `automatic` on Length and Count (a stored 0 passes `accept`), `Generated.automatic`,
  CHANGELOG (2, ADR 0082 §7).
- [x] 2. `contour.rs`: closed contour of lines and arcs, kerf offset of ADR 0082 §2, conversion to
  `ClosedPath`; tests for exact offsets, tangent joins, miters (4, 5, 10).
- [x] 3. `footprint.rs`: rounded rectangle, spans, neutral lengths, tab places with flat windows, seam,
  degenerate straights and the exact circle (3, 5).
- [x] 4. `slit_bend.rs`: Straight, Offset and Wave slots (`wave_slot.rs`), no uncut band, windows, bridges (2, 4).
- [x] 5. `wall.rs`: the developed strip with slots, edge notches, tab slots, seam; strip length test (3, 4, 5, 10).
- [x] 6. `wall_plates.rs`: inner base, base cover, seat ring, lid with finger hollow, tabbed lid (5, 10, 12, 13).
- [x] 7. `rounded_box/`: schema, values, automatic values, clamps, five thicknesses, layout in thickness runs,
  handles, resize keeps the radius (2, 6, 7, 9).
- [x] 8. Manifest kind `rounded-box`, goldens `rounded_box_default.txt` and `rounded_box_circle.txt`, mating,
  circle and pattern tests (1, 5, 8, 9).
- [x] 9. Shell wiring: third segment and glyph, muted automatic values, corner radius handle (1, 7).
- [x] 10. CPU-time test of a 400-slot box (6).
- [ ] 11. Full gate once into a file.

## Validation

Property tests with explicit tolerances (strip length 0.1 mm, tab/slot centres, window sag plus kerf, no
uncut band), goldens for the default box and the circle, thread-CPU test, full gate.

## Not built yet (slice 1)

Nothing of slice 1 is open; Wave, the hollow on a footprint without a front straight and the two-switch conditions are built. Tester, UX and architect reviews and a browser try are still to do.

## Own choices (Proposal)

- A bend gets at least two slit columns (ADR 0082 §6 says `max(1, ...)`): one column leaves an uncut band at
  the bridge height, which criterion 4 forbids.
- `ShownWhen::AllOn { keys }` (contract, CHANGELOG) hides Ring thickness and the hollow rows without Lid or Lid seat.
- `Param.track` (slider track and curve) is in the contract for `0141` revision 2 criterion 9; ADR 0082 §7 lists
  only `automatic`, so the architect should confirm it (CHANGELOG of `curvyo-extension-core`).
- The gear's Pitch diameter knob is dropped (UX notes of `0141` §15.6 default).
- The seam sits at the middle of the longest straight when it holds 8 wall thicknesses, else at the back's middle.

## Function length (CLAUDE.md §5)

`wall_plates::straight` (about 70 lines) is one walk along a straight with its tabs and the finger hollow, kept
whole because its pieces share the cursor and the outward direction; `rounded_box::generate` (about 60 lines)
is the sequence clamp, footprints, strip, plates, layout, handles. `Footprint::split_bend` was shortened into
`tab_layout.rs` (`Layout::split_bend`, `fitting_count`). A small circle takes fewer windows so that every curved
stretch keeps two slit columns and its bridges; Wall thickness is clamped to a sixth of the smaller inner side
so that one tab window fits (**Proposal**, dependent clamp).

---

# Plan for Rounded box slice 2, the curved top

Implementer, 2026-11-25, branch `story/rounded-box-curved-top`. Built under `specification.md` (revision 8, criteria 11 and
16 to 24), ADR 0082 §11 (revision 6, amended), `adrs.md` and `ux-notes.md` (revision 6, section 10). Slice 1's code is reused;
no new crate, no dependency, no `format_version` change. Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-extension-core`:** `param.rs` (`ShownWhen::All` and `Any`), `CHANGELOG.md` (source-breaking 0.x).
- **`curvyo-generators-core`:** `boxes/plate.rs` moves to crate-level `plate.rs` (a notch depth per edge, `Corner::Round`, a
  tab edge, the finger hollow in the top edge); new `rounded_box/curved_top.rs` (strip, plates, clamps, layout, handles,
  marks); `rounded_box/schema.rs` (`top_shape`, `end_plate_thickness`, `front_thickness`, conditions only in the rounded
  box's own groups, the path box's manifest stays); `rounded_box/mod.rs` branches on `top_shape` first; goldens.
- **`curvyo-ui-core`, `curvyo-editor-wasm`:** the overlay of a generated object draws marks for the rounded box in Curved
  (ux-notes §12 item 17, **Proposal**: `Mark::Bend` per end-plate arc and the piece badges, only if it stays small).

## Tasks

- [x] 1. `ShownWhen::All` / `Any` evaluated by `holds`, CHANGELOG (criterion 21, ADR 0082 §7 revision 6).
- [x] 2. `plate.rs` to crate level, behaviour-preserving: per-edge notch depth, round corner, tab edge, hollow in the top edge;
  `box_default.txt` and every golden byte-identical (criterion 16, 18; ADR 0082 §4, §11).
- [x] 3. Schema: `top_shape`, `end_plate_thickness`, `front_thickness`, rows hidden in Curved keep their values, the path
  box's manifest unchanged (criteria 11, 21, 24).
- [x] 4. `curved_top.rs` strip: footprint of three shapes, the length test, whole-arc slits, no slit on a flat, edge notches
  for the end plates' tabs, finger end with the base (criteria 16 to 19).
- [x] 5. `curved_top.rs` plates: end plates with profile and back-straight tabs, front with the hollow, base owning every bottom
  corner cube; mating and occupancy tests at five thicknesses, simple polygons at kerf 0 and 0.2 (criteria 16, 18, 21, 22).
- [x] 6. Clamps (radius range, two columns, height, hollow, kerf, thickness), switching Top shape keeps values, automatic
  values, layout in thickness runs, handles, `Edit::Draft`, resize (criteria 20, 21, 23, 24).
- [x] 7. Golden `rounded_box_curved.txt`, 400-slot CPU time, save and reopen without `top_shape` reads Flat (criteria 24).
- [x] 8. Marks: `Mark::Bend` on the end plates' arcs and part badges (ux-notes §10.8, §10.9, §12 item 17).
- [x] 9. Full gate once into files (fmt, clippy, nextest 17986 passed, doc, wasm build, tsc, npm test 1716, oxlint: green, 2026-11-25).

## Own choices (Proposal)

- The Wall thickness bound in Curved is the sixth of the smaller inside side only; the 0.25 times Inner height part of Flat is
  replaced by the Inner height minimum 2 t_e + 6 t_w of criterion 20. No bound is invented for End plate, Front or Inner base
  thickness (ADR 0082 §11 lists none; the radius range stays non-empty by the height rule).
- The two-column radius of criterion 19 is `4 s / pi - t_w / 2` for the spacing `s` in use (a typed spacing, else the least
  `width + 1 mm`); a typed spacing that asks for more raises Corner radius up to its upper bound, there `slots` keeps two columns.
- The hollow's "2 t margin to the nearest finger joint" (criterion 21) is read from the front's side edge, the joint line: the
  widest hollow (Inner width minus 2 t_e, UX notes §10.10: 114 mm) leaves 2 t_e to that edge and t_e to the finger root.
- The overlay of marks is drawn for a rounded box whose `top_shape` is Curved, not for Flat (ux-notes §10.12); all marks are in the
  object's frame (`Marks.footprint` zero): `Mark::Bend` on both end plates' arcs, `Mark::Piece` badges 1 to 5, the strip's badge in
  the middle of the lid run.
- The two end plates are one outline cut twice (the Box's side walls), not a mirror pair.
- `tests/acceptance_0147.rs` criterion 2 listed `top_shape` among the keys slice 1 must not have; the assertion now lists only Finger
  width and Clearance (the key arrived with this slice, criterion 11).

---

# Plan for Rounded box slice 3, Detach part

Implementer, 2026-11-26, branch `story/rounded-box-detach-part`. Built under `specification.md` (criterion 15; the customer accepted
slice 3 with the format change, Frage 370 A, 2026-11-25), ADR 0082 §10 (Option C: part keys, `member.<node>#<part>`), `adrs.md` item 7
and `ux-notes.md` §9. No new crate, no dependency. `format_version` 64 to 65. Own choices are marked **(Proposal)**.

## Affected crates/modules

- **`curvyo-extension-core`:** `Part.key` (contract, 0.x, CHANGELOG). **`curvyo-generators-core`:** every generator names its parts
  (rounded box by role: `strip`, `base`, `cover`, `ring`, `lid`, and in Curved `left`, `right`, `front`; Box `bottom`, `front`, `back`,
  `left`, `right`, `lid`; gear `gear`; path box `wall-<n>` then the rounded box's keys).
- **`curvyo-document-core`:** `NodeId` gains a part tag (none = the node itself, else a `u16` into an interned table of key texts; never stored, never in
  `document.json`); `GeneratedRecord.part_keys` (written whole with `parts`, damage: wrong length or duplicate); membership key
  `member.<node>#<part>` read, kept when the part is missing, swept with its node; `detach_part` / `attach_part` as one commit each;
  `convert_generated` rewrites `N#P` to the new path; `CURRENT_FORMAT_VERSION` 65 with goldens `generated_part_ops_v65.curvyo` and
  `generated_part_keys_damaged_v65.curvyo`.
- **`curvyo-ui-core`:** record keys from the generator result, step names "Detach part" / "Attach part", the Parts rows of the block.
- **`curvyo-editor-wasm`, `frontend`:** the leaf index splits a box with a detached part into the rest and one leaf per detached part
  (geometry from the record), Detach / Attach commands, the Parts group in the Generator block (muted operation name, text button).
- `curvyo-laser-core`: no change; the nearest rule already looks up a chain of `Vec<NodeId>` keys and `N#P` is one more key.

## Tasks

- [x] 1. `Part.key` and keys in every generator; unique within one result, stable across regenerations (criterion 15; ADR 0082 §10).
- [x] 2. `NodeId` part tag; membership key `#` form: read, write, liveness, sweep, a missing part keeps its key (15, ADR 0082 §10 Lifecycle).
- [x] 3. Record `part_keys` + damage rules; `detach_part` (copy of the box's operations, "<operation> · <part>", right after it, one
  step) and `attach_part` (one step); `convert_generated` rewrite; tests first, golden files (15).
- [x] 4. `format_version` 65: constant, doc comment, version numbers in tests, goldens, test that a 65 file is refused by a reader
  limited to 64 and that a file of 64 opens unchanged (15, ADR 0033 §7).
- [x] 5. ui-core: record keys, step labels and names, Parts rows (15).
- [x] 6. Leaf index and job input: rest leaf plus one leaf per detached part; geometry from the record; chain `[N#P]`, `[N]`, groups
  above; test that the lid runs with its own operation, a thickness change regenerates it, a move moves it, save and reopen keeps both (15).
- [x] 7. Session commands, wasm bindings, frontend Parts group (text button Detach / Attach).
- [x] 8. Docs: spec status, ADR 0082 status of §10, technical debt, ux-notes §9.
- [x] 9. Full gate once into files (see the PR; 2026-11-26).

## Own choices (Proposal)

- Detach copies **every** operation of the box's nearest assignment (usually one), each named "<operation> · <part>" and placed right
  after its original; the copies carry the part key and nothing else changes in the job.
- A part tag is a `u16` into an interned table of key texts (a 32-bit hash was the first idea; the table keeps the key text for writing
  and compares exactly; ids stay 16 bytes), recorded in `docs/technical-debt.md`.
- The rest of a box with detached parts is one leaf (the box's key `[N]`) with the geometry of its remaining parts; with every part
  detached it has no leaf. Without a detached part nothing changes: one leaf per box, counts and goldens stay.
- Detach is offered for a box whose record already carries part keys; an older file gets them at its next parameter edit (ADR 0082 §10).
  The Parts rows are shown from the generated result, so they appear at once; Detach on a record without keys writes them in the same step.
- Hover of a Parts row on the canvas and the part's context-menu entry are left out (ux-notes §9: "unless the customer asks").

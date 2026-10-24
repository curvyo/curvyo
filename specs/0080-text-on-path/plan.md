# Plan for text on path, slice 1

Implementer, 2026-10-24, on `story/text-on-path` (stacked on `story/text-kerning`, PR #189).
Slice 1: criteria 1 to 36 and 43 to 48; slice 2 (37 to 42) is not built. Built on the default A
of ADR 0022 (customer answer pending through the PR) and on decisions 1 to 11 of
[`adrs.md`](adrs.md); UX per [`ux-notes.md`](ux-notes.md) with its ten §0 proposals applied to
the criteria.

## Affected crates/modules

- `curvyo-geometry-core`: new `path_measure.rs` ("arc length, point, tangent and nearest arc
  length along one outline"): `PathMeasure` over exact cubics, the topmost point.
- `curvyo-document-core`: new `text_path_model.rs` (`TextOnPath`, `CarrierRef`, `PathSide`,
  `LineOnPath`, `CarrierCurve`, `OnPathValues`, `OnPathLabel`), `text_path_codec.rs` (the seven
  keys, lenient read), `text_carrier.rs` (carrier resolution in the one read pass, `texts_on`,
  the kept copy on removal, `text_path_previews`), `text_path_commands.rs`
  (`put_text_on_path`, `set_text_path`, `release_from_path`); `TextSnapshot::on_path`;
  `guides.rs` (a clone carries its own flag); Duplicate copies the keys; `CURRENT_FORMAT_VERSION`
  22, golden `text_on_path_v22.curvyo`.
- `curvyo-ui-core`: new `text_on_path.rs` (rigid per-letter map, letter boxes), new
  `text_path_edit.rs` (default position, release placement, handle drag and snap, nudge, degree
  readout); `text_layout::lay_out` branches once; `text_box` hit and bounds by letter boxes;
  caret and selection mapped along the carrier; history names.
- `curvyo-editor-wasm`: session `text_on_path.rs` (pick mode, hover preview, put, handle drag
  with preview, options, release, carrier line, the bar view), carrier drag previews through
  `text_path_previews`, bindings.
- `frontend`: On path button (custom glyph) in the Text bar and the Select bar Text group, pick
  cursor, `TextPathGroups` on row 2 (Placement, On path values, Release), greyed writing mode,
  the row-2 dot, `lib/textPathText.ts` with node tests.
- Docs: `docs/design-system.md` rows (UX notes §8), ADR index unchanged (0022 Proposed),
  `specs/README.md` row 0080.

## Tasks

Milestone 1, core (test first)
- [x] 1. geometry-core `PathMeasure`: length, point and unit tangent at an arc length on exact
  cubics, incoming tangent at a corner, chord at a cusp, straight extensions beyond an open
  carrier's ends, modulo on a closed one, length 0, nearest arc length, topmost point (first in
  carrier direction; the middle of a flat top edge); 100,000 segments do not hang (AC 8, 11, 15,
  26, 45, 46)
- [x] 2. document-core model and codec: the seven keys, defaults never written, read rules
  (unknown code, non-finite, outline index past the last, malformed kept copy); `on_path` on
  `TextSnapshot` (AC 17, 34, 35)
- [x] 3. document-core carrier resolution: path, compound outline, primitive by
  `drawing_outline_of_rotated`, resolved clone; kept copy; else straight; `texts_on`; start and
  direction pinned per kind (AC 4, 24, 25, 32, 35, 36)
- [x] 4. document-core commands: `put_text_on_path` (carrier made a guide in the same commit, not
  on a locked layer), `set_text_path` with labels, `release_from_path`; a clone carries its own
  guide; kept copy written by the removal funnel; Undo; Duplicate (+5 mm) and copy with the
  carrier; format 22 and golden file (AC 4, 4a, 6, 7, 17, 18, 23, 25, 31, 34)
- [x] 5. ui-core placement: criterion 9 (straight carrier = straight text), 10 (four cases on
  the circle), 11 (corners), 12 (lines), 13 (single-line), 18/19 (Mirror, Mirror letters), 26
  (overflow), 47 (kerns), 48 (vertical laid out horizontally); letter boxes, hit, marquee box,
  caret along the carrier; Object to path equals drawing; non-finite letters left out (AC 8 to
  13, 18, 19, 26 to 29, 45, 47, 48)
- [x] 6. ui-core edit math: default position (start, or the text's middle at the topmost
  point), release placement keeping the first letter, nearest-point drag with the side switch
  past 8 px, Ctrl snap points, nudge in reading direction and Distance on Up/Down, degrees on an
  ellipse (AC 5, 15, 16, 20, 21, 23)
- [x] 7. Timing tests, release build: 200 characters on 1,000 cubics, a position drag frame and
  a carrier node drag frame within 33 ms; 100,000 segments and letters past 1e7 mm (AC 43, 45)

Milestone 2, session
- [x] 8. Pick mode: start, hover outline and preview, click puts (one step), Escape, right
  click, second press, tool switch; Select tool route with a selected carrier; locked text
  refused; from edit mode back to edit mode; re-pick (AC 1 to 7)
- [x] 9. Position handle: drawn, dragged with preview, side switch, Escape, Ctrl snap, readout;
  body drag slides; arrow nudge; carrier line while selected (AC 15, 16, 21)
- [x] 10. Options: Side, Line, Mirror, Position (mm or degrees), Distance, Release; Flip toggles
  Mirror letters; carrier edits followed in the same frame during drags (`text_path_previews`);
  the edit session lays out along the carrier (AC 17 to 19, 23, 24, 27)

Milestone 3, frontend
- [x] 11. `lib/textPathText.ts` with node tests; On path button, pick cursor, `TextPathGroups`,
  greyed Writing mode and Glyph orientation, row-2 dot (AC 1, 2, 17, 48)
- [x] 12. Design-system rows (UX notes §8)

Milestone 4, gate
- [x] 13. Full gate (`CLAUDE.md` §7), frontend tsc, npm test, oxlint; wasm size against the
  budget; headless Chromium screenshots (circle in four cases, star, open path, handle drag,
  carrier edit followed).

## Validation

- geometry-core unit tests on lines, a four-cubic circle (exact at its anchors), a rectangle, a
  cusp; a 100,000-segment timing test.
- document-core codec unit tests, command tests with Undo, a removal writing the kept copy, the
  golden-file round trip.
- ui-core tests with the included Source Sans 3 face for each criterion's numbers (9, 10, 12,
  47, 48), release-build timing tests for 43 and 45.
- session tests for pick mode, the handle drag, the options and their history steps.
- node tests for the frontend text rules; a headless Chromium look at the bars and the canvas.

## Dated notes

- 2026-10-24, criterion 35 against `adrs.md` decision 1: a missing or non-finite Position or
  Distance reads the text **straight** (the criterion), not as 0 (the decision's read rule).
- 2026-10-24, ADR 0022 §2 and the `0046` tester case "a clone itself cannot be marked": Guide on
  still skips clones; a clone that carries its own flag (written only by Put text on path) can be
  turned off by Guide off. So a clone carrier behaves as criterion 4 asks without changing the
  accepted Guide button on clones.
- 2026-10-24, criterion 5 on a flat top edge (rectangle, rotated polygon): the text's middle goes to
  the middle of the topmost flat stretch, not to its first point (a rectangle would put the text at
  its top-left corner otherwise). Proposal for the demo.
- 2026-10-24, single reads resolve too: `Document::text` and `Document::object` read the carrier of
  a text on path, so every reader of one text (bar, edit mode, Object to path) sees the curve.
  `carrier_curve` is public in `document-core` (the session previews a pick with it).
- 2026-10-24, notices: "1 text kept its path." is the first sentence of the existing removal
  notice (`take_clone_repair_notice`). The locked text uses the layer lock notice.
- 2026-10-24, budgets (release, navi): 200 characters on 1,000 cubics, worst frame of a position
  drag 0.63 ms and of a carrier node drag 0.60 ms (budget 33 ms); 100,000 segments read and laid
  out in 86 ms; letters past 1e7 mm are left out.
- 2026-10-24, wasm size (release editor wasm after `wasm-bindgen`, no `wasm-opt`): base
  `story/text-kerning` at `f3f6aeb` 12,763,446 bytes, this slice 12,866,733 bytes: **+103,287
  bytes (100.9 KiB; +40.0 KiB gzip -9)**. The 0079 note left about 156 KiB after slice 3; the base
  here is already 124.5 KiB above that build, so the 2 MiB text-engine budget needs a recount by
  the lead or architect before merge.
- 2026-10-24, measured bars with a text on a circle: Text bar 800 x 600 closed 2 lines, open 5;
  1280 x 800 closed 1, open 3. Select bar 800 x 600 closed 5, open 8; 1280 x 800 closed 2, open 3
  (UX notes predicted 4).
- Not built in this slice (open points for the lead): the typed rotate entry `R` on an ellipse
  carrier (criterion 20; the rotate handles work); the Ctrl-copy move of a text on path alone shifts
  by 5 mm, not by the move projected on the carrier (criterion 31; `duplicate_objects_shifting`
  takes the shift, the Select tool does not pass it yet); with text and carrier selected together a
  transform also rewrites the text's stored straight placement (harmless, not drawn).

## Customer changes 2026-10-24 (folded into the slice-1 commits)

- [x] Position is the arc length of the text's middle (`PathPlacement` places `position +
  σ(x_c − x_m)`, `x_m` the middle of all lines' extent); default on an open carrier half the
  text's width, on a closed one the topmost point; handle, snaps, `R`, Size drag about the middle
  follow (criteria 5, 9, 15, 18 to 21, 47). Tests asserting the start anchor updated; golden
  `text_on_path_v22.curvyo` regenerated (no migration, format 22 not on main).
- [x] Row 2 keeps only Release from path; Position, Distance, Side, Line and Mirror moved into the
  position handle's entry box (`text_on_path_handle_entry.rs`, `TextPathEntryChip.tsx`), opened by
  a double-click on the handle, previewed, one step on Enter (new label "Change text on path" for
  several values), nothing on Escape (criterion 17).
- [x] Bug "text slides after double-click, Escape twice, V": regression test
  `bug_no_slide_after_edit_mode_escape_and_v`; not reproducible on this branch at session or
  browser level after the change.

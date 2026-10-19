# Plan for Pen angle snap and typed values

Branch `story/pen-angle-snap-and-typed-values`, one PR for the whole slice, opened by the lead
after the reviews (`CLAUDE.md` §4). Inputs: `specification.md` (criteria 1 to 36), `adrs.md`.
No new crate, dependency, ADR or `format_version` change; no public API change in
`curvyo-document-core`.

## Affected crates/modules

- `curvyo-ui-core`
  - new `pen_constraint.rs`: `constrain_segment(last, raw)` on top of `snap_angle` (decision 1).
  - new `pen_entry.rs`: the entry state (`PenEntry`: L, following or frozen, the two field
    texts), the field texts, and `resolve` with the permit rules (notices) and the polar step.
  - new `pen_entry_text.rs`: labels, accessible names, the live-region sentences and the
    notice texts.
  - `shape_placement/text.rs`: `number` and `unit_words` become `pub(crate)` for the sentence.
  - `lib.rs`: exports.
- `curvyo-editor-wasm`
  - `session/pen.rs`: the Ctrl constraint at the press (no target), the handle snap of the
    release and the preview, the rubber band end, the drag readout.
  - new `wasm_pen_entry.rs`: the chip view and the entry calls on the wasm surface, and
    `pen_secondary_press`.
  - new `session/pen_entry.rs`: open (Tab), follow, edit, place (Enter or a frozen press),
    cancel, the chip view, announcement and notice.
  - `session/mod.rs`: the `pen_entry` field, the press routing, `set_tool` cancels.
  - `session/key_table.rs`, `session/keys.rs`: `KeyAction::OpenPenEntry`, Tab consumed with a
    button held, the entry counts as an open entry and an operation in flight, Escape step 1.
  - `session/draw.rs`: the pending anchor with the handle snap.
  - `session/shapes.rs`: `live_readout` asks the Pen first.
- `frontend`
  - new `hooks/usePenEntry.ts`, new `components/PenEntryChip.tsx` (two `EntryField`s,
    `placePlacementChip`), mounted in `Canvas.tsx`; `useEditorSession.ts` routes the key outcome,
    pointer moves and notices.
- Docs: `docs/design-system.md` (shortcut rows, Pen entry chip row), corrections of `0010`
  criterion 39 and `0034` "Out of scope"; `specification.md` status at the end.

## Tasks

1. [x] `plan.md`.
2. [x] Tests first, then `constrain_segment` (criteria 1, 2, 3).
3. [x] Session: Ctrl at the press, handle snap at release and preview, rubber band end, target
   wins, first node untouched, drag readout (criteria 1 to 8, 11). Benchmark `#[ignore]`
   (criterion 10). Criterion 9 needs no code (the modifier comes from the key events).
4. [x] Tests first, then `PenEntry` and `resolve`: live texts, freeze, polar point, unit
   suffixes, not a number, negative, zero, modulo 360, coordinate limit (criteria 15 to 17,
   21, 24 to 28).
5. [x] Session entry: Tab gate, open, Enter places like a click, frozen press places, unfrozen
   press closes, Escape, tool switch, finish closes, nothing written, continuation order
   (criteria 12, 14, 19, 20, 22, 23, 30 to 32, 34). Right-click finishes the path with a path
   in progress (criterion 19a, customer 2026-10-19): `Session::pen_secondary_press`.
6. [x] Wasm surface and frontend chip (criteria 13, 18, 23, 29, 33, 35).
7. [x] Docs (criterion 36, "Changes to accepted behaviour").
8. [x] Full gate (`CLAUDE.md` §7), frontend lint and type check; history tidied to at most
   three commits.

## Validation

- Unit tests in `curvyo-ui-core` for the constraint and the entry (one focused test per
  criterion, the spec's examples as data).
- Session tests in `curvyo-editor-wasm/src/session/pen_entry.rs` (and `pen.rs`) driving
  `Session` with Ctrl, Tab, Enter, Escape and presses, reading the in-progress nodes and the
  document.
- Criterion 10: the tester's release benchmark `the_snap_adds_at_most_a_tenth_of_a_millisecond_per_move`
  (`acceptance_0072_tester.rs`); the implementer's own was removed after the review (lean tests).
- Manual: `cd frontend && npm run dev`, Pen tool, hold Ctrl, Tab.

## Decisions (2026-10-19, implementer)

- The handle snap needs the press point N; the session keeps it next to the press target
  (`PenCueState`), so `pen_tool.rs` keeps its signatures (adrs decision 3).
- The drag readout (criterion 8) is the existing create-drag readout route
  (the wasm `live_readout`, the DOM `ReadoutChip`), fed by `Session::pen_drag_readout`; the
  same formatter and placement as the create-drag readout, which the criterion names; no new
  draw-list text item. `Session::live_readout` itself stays the marquee's and the creation
  tools' (a tester test of `0014` reads it as "the legend" and expects none in the Pen).
- An untouched field keeps the exact live value at the freeze (as `0052` keeps the live shape);
  the field shows it rounded.
- Shift+Tab on the canvas acts like Tab (the spec excludes only Ctrl, Alt and Cmd).
- A Tab repeat with a path open is consumed and opens nothing, so focus never leaves the canvas.
- With several notices at once (for example "not a number" and "negative") the first in the
  order of criteria 24, 25, 28 is shown.
- The wasm calls live in a new `wasm_pen_entry.rs` (one responsibility per module) rather than
  in `wasm_pen.rs`.
- Untouched over a join target, Enter joins as the click there would (review 2026-10-19).
- No existing Pen test used Ctrl as a "no effect" case (checked with grep).

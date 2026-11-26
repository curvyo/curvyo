# Plan for Bridged lettering (0156), slices 1 and 2

Implementer, 2026-11-26. Branch `story/fused-lettering`. Both slices (Lettering and Stencil) are
built on this branch and go into one PR (lead's instruction of 2026-11-26).

## Customer answers of 2026-11-26 that change spec and UX notes (relayed by the lead)

The lead updates `specification.md` and `ux-notes.md`; this plan already follows the answers:

- **Frage 395 (changed): no entry chip, no canvas preview.** Pressing Bridge applies at once with
  the current values. Width, Reach, Join all (slice 1) and Mode, Bridges per island (slice 2) are
  fields in the Select tool's context bar, shown when the selection allows Bridge on a laser page.
  The Bridge button is on the Boolean card (laser pages only) **and** in the Select bar.
  Refusals show as the usual rail notice. `adrs.md` decision 8 (preview cache) is dropped;
  criterion 3's chip and preview are not built.
- **Frage 398 C (changed):** Reach as specified plus a **Join all** switch in the Select bar: on,
  every piece is joined regardless of Reach (the kernel reads Reach as unbounded).
- Fragen 396 A and 397 A unchanged.
- **Frage 404 A (changed, follow-up of 2026-11-26):** no refusal under a group or layer that holds
  an operation; the Bridges inherit it by the nearest rule like any leaf (`0101` criterion 12).
- **Frage 403 (changed):** column B does not scroll; the rail wraps by itself on small windows.
  The rail is as before apart from the Bridge button.
- **Criterion 15 (lead decision, Proposal):** meant with a non-dot middle piece. A dot is attached
  to its nearest non-dot (rule 7a) and never forms a path between two letters, so a dot between
  two letters joins one of them and the other stays apart (`tests/acceptance_0156.rs`,
  `c15_as_written_a_dot_in_the_middle_joins_one_letter_only`).

## Affected crates/modules

- `curvyo-geometry-core/src/fuse/` (new module family, `adrs.md` decision 1): `mod.rs` (entry
  points `fuse_cut_out`, `fuse_stencil`, `Fused`, `FuseError`), `pieces.rs`, `gap.rs`,
  `forest.rs`, `axis.rs`, `blend.rs`, `islands.rs`, plus `measure.rs` (the f64 point/segment
  helpers on the grid they share). Goldens in `curvyo-geometry-core/tests/fixtures/fuse/`.
- `curvyo-ui-core`: `kernel_operands.rs` (Weld's operand collection moved out of `weld_plan.rs`,
  no behaviour change), `fuse_plan.rs` (clamps, defaults, `FUSE_MAX_BRIDGES`, refusal codes,
  plan), `history_names.rs` (label `fuse` reads "Bridge").
- `curvyo-document-core/src/replace_group.rs`: `Document::replace_with_group`, one commit.
- `curvyo-editor-wasm`: `session/fuse.rs` (`fuse_available`, `fuse_availability`, `apply_fuse`),
  `wasm_fuse.rs`; budget test `tests/fuse_budget.rs`.
- `frontend`: `lib/bridgeText.ts` (words), Bridge button on the Boolean card (laser pages only),
  a Bridge group in the Select bar (fields, switch, mode, button), app preferences for the values,
  `useEditorSession` / `useRailCommands` wiring.

## Tasks

- [x] 1. Refactor: move Weld's operand collection to `curvyo-ui-core/src/kernel_operands.rs`; Weld
  tests unchanged and green (prepares AC 6).
- [x] 2. Kernel slice 1, test first: pieces and dots (AC 7a, Words "Dot piece"); narrowest gap
  with the tie rule (AC 9, Words "Gap"); forest with Reach, Join all and obstruction (AC 7, 15,
  17); axis, band and blend (AC 8, 16); outline as Union and connectors as added material
  (AC 11, 12, 13); determinism (AC 18); refusals `NothingToJoin`, `TooManyBridges` and the
  `BooleanError`s (AC 17). Property test of AC 12/13 on 200 random sets, fixed seed. Goldens for
  the i, a geometric ü, the bars and the AC 16 cases.
- [x] 3. Kernel slice 2, test first: islands, crossings, the per-island choice rule and chains
  (AC 22, 23, 24, 25 geometric B/A/e/8 stand-ins), outline without inner contours (AC 24, 28),
  bridge width clamp (AC 27), `NoIslands` (AC 22).
- [x] 4. `Document::replace_with_group`: group "Bridged lettering"/"Bridged stencil" at the
  top-most operand's place, Outline then Bridges with "Bridge n", style given, `member.` keys of
  the given operations on the Outline, one commit (AC 11, 14, 20, 26).
- [x] 5. `fuse_plan.rs` and session: laser-page gate (AC 1, 2), one-page and placement refusal
  (AC 4), operands as Weld reads them with texts converted in the step (AC 6), effective
  assignment of the bottom-most operand on the Outline, Bridges none of their own and inheriting
  an enclosing group's or layer's operation (AC 14, Frage 404 A), result selected (AC 11), one
  undo step "Bridge", run-twice
  determinism (AC 18), save and reopen (AC 20), CPU budget 60 pieces / 3000 nodes / 300 ms
  (AC 19).
- [x] 6. Frontend: Bridge on the Boolean card after Weld, laser pages only, absent elsewhere
  (AC 1, 2); Bridge group in the Select bar with Width, Reach, Join all, Mode, Bridges per island
  and the Bridge button (Frage 395 changed, AC 5, 21); values as app preferences, clamped on
  commit with the limit notice (AC 5); refusal notices (AC 4, 14, 17, 22); Layers expansion of
  the result (AC 11).
- [x] 7. Full gate (`CLAUDE.md` §7) once into a file, green.

## Validation

- Kernel unit tests per module; geometry goldens in `curvyo-geometry-core/tests/fixtures/fuse/`
  compared at 1e-6 mm.
- Property test (AC 12, 13): 200 random shape sets, fixed seed.
- Session tests in `curvyo-editor-wasm` for the gate, refusals, result tree, undo, determinism,
  save and reopen; CPU-time budget with the `cpu_time` helper.
- Frontend unit tests for the words and the clamps (`frontend/tests/bridgeText.test.ts`).

## Deviations (Proposal, for the lead)

1. **Tie rule (Words, "Gap").** Read literally (the middle of the tied stretch on the lower
   piece), criterion 9's bars give y = 6.38, because the 0.01 mm tie lets the stretch run 0.245 mm
   round the far bar's corner on one end only. Built: the bridge's centre is half way between the
   middles of both pieces' tied stretches, its ends the points of each piece nearest that centre.
   The bars give 6.5, the i (1, 10) to (1, 12).
2. **Blend radius.** A closing of radius Width fills every gap narrower than twice the Width
   along the whole neighbourhood of the bridge (a 2 mm gap at Width 1 became a 5 mm plug). Built:
   radius = min(Width, 0.45 x the gap), so only the junctions are filled.
3. **Stencil crossings** are cast from the island's boundary along its normal into the stroke,
   sampled every 0.05 mm and at every vertex (the nearest point of the outer contour was
   ambiguous where a vertex faces a chord and put the O's bridge 0.024 mm off). Stencil bridges
   have a straight axis.
4. **Member order.** Outline is the top member, Bridges below it (UX notes §6 and criterion 11's
   order); `adrs.md` decision 5 had the Connectors above.
5. **Criterion 4.** A leaf off the laser page is refused before the edit guard, so the maker sees
   "Bridge works on one laser page" instead of the generic move-only refusal.
6. **Criteria 10 and 25** are covered with geometric stand-ins in the kernel and with Source Sans 3
   Regular (OFL, already shipped in `frontend/public/fonts/source-sans-3/` with its licence) in
   `curvyo-editor-wasm/tests/fuse_fonts.rs`; the font is read from there, not copied into a
   `tests/fixtures/` folder.
7. **Limit notice** shows under the Bridge group of the Select bar for 3 s in the status look (the
   chip it was written for is gone).
8. The design-system rows of `ux-notes.md` §10 are not added (the chip row no longer applies);
   left to the UX engineer with the updated notes.

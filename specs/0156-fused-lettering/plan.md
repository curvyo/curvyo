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
- Fragen 396 A, 397 A, 403 A and 404 B unchanged (404 B: refused when the result would stand
  under a group or layer that holds an assignment).

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

- [ ] 1. Refactor: move Weld's operand collection to `curvyo-ui-core/src/kernel_operands.rs`; Weld
  tests unchanged and green (prepares AC 6).
- [ ] 2. Kernel slice 1, test first: pieces and dots (AC 7a, Words "Dot piece"); narrowest gap
  with the tie rule (AC 9, Words "Gap"); forest with Reach, Join all and obstruction (AC 7, 15,
  17); axis, band and blend (AC 8, 16); outline as Union and connectors as added material
  (AC 11, 12, 13); determinism (AC 18); refusals `NothingToJoin`, `TooManyBridges` and the
  `BooleanError`s (AC 17). Property test of AC 12/13 on 200 random sets, fixed seed. Goldens for
  the i, a geometric ü, the bars and the AC 16 cases.
- [ ] 3. Kernel slice 2, test first: islands, crossings, the per-island choice rule and chains
  (AC 22, 23, 24, 25 geometric B/A/e/8 stand-ins), outline without inner contours (AC 24, 28),
  bridge width clamp (AC 27), `NoIslands` (AC 22).
- [ ] 4. `Document::replace_with_group`: group "Bridged lettering"/"Bridged stencil" at the
  top-most operand's place, Outline then Bridges with "Bridge n", style given, `member.` keys of
  the given operations on the Outline, one commit (AC 11, 14, 20, 26).
- [ ] 5. `fuse_plan.rs` and session: laser-page gate (AC 1, 2), one-page and placement refusal
  (AC 4), operands as Weld reads them with texts converted in the step (AC 6), effective
  assignment of the bottom-most operand on the Outline, Bridges none, refusal under an assigned
  group or layer (AC 14, Frage 404 B), result selected (AC 11), one undo step "Bridge", run-twice
  determinism (AC 18), save and reopen (AC 20), CPU budget 60 pieces / 3000 nodes / 300 ms
  (AC 19).
- [ ] 6. Frontend: Bridge on the Boolean card after Weld, laser pages only, absent elsewhere
  (AC 1, 2); Bridge group in the Select bar with Width, Reach, Join all, Mode, Bridges per island
  and the Bridge button (Frage 395 changed, AC 5, 21); values as app preferences, clamped on
  commit with the limit notice (AC 5); refusal notices (AC 4, 14, 17, 22); Layers expansion of
  the result (AC 11).
- [ ] 7. Full gate (`CLAUDE.md` §7) once into a file, green.

## Validation

- Kernel unit tests per module; geometry goldens in `curvyo-geometry-core/tests/fixtures/fuse/`
  compared at 1e-6 mm.
- Property test (AC 12, 13): 200 random shape sets, fixed seed.
- Session tests in `curvyo-editor-wasm` for the gate, refusals, result tree, undo, determinism,
  save and reopen; CPU-time budget with the `cpu_time` helper.
- Frontend unit tests for the words and the clamps (`frontend/tests/bridgeText.test.ts`).

## Deviations (Proposal, for the lead)

(Filled in as built.)

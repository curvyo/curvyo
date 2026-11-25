# ADRs for library into slots

Architect, 2026-11-25. One new ADR, not `needs-customer` (no crate, no dependency, no format
change). Fragen 377 (C, slice 1 copy only), 378 (B), 379 (A) and 385 (C) are assumed at their
defaults; ADR 0088 "If a Frage is answered otherwise" says what the other answers change.

- [ADR 0088](../../docs/adr/0088-library-into-slots-fit-filled-and-one-step-put.md) (Proposed):
  - the fit (criterion 6): `fit_into_slot` and `slot_fit_matrix` in `curvyo-laser-core::fixture_fit`
    beside `fit_slot`; `FIT_TOLERANCE` (0.1 mm) public there; the 0.1 mm match first, then the turn
    only when it gains more than 0.1 mm on the longer side; a holder turned by any angle turns the
    unit with it (decision 1);
  - the unit, its box and the wrap into one group named after the source page (criterion 2;
    decision 2); the write as `FragmentTarget::Group` plus a mapped private fragment, one fragment
    per distinct fit (decision 3);
  - "filled" computed from top-level members by outline box centre in the slot's closed outline
    grown by 0.1 mm (Terms, criteria 7, 8, 10; decision 4);
  - the session commands `put_into_slot`, `fill_empty_slots`, the drag cue and the list view, each
    one step (`put_into_slot`, `replace_slot_content`, `fill_slots`) (criteria 1, 4, 7 to 10;
    decision 5);
  - Operations and jobs unchanged, the unit is a work item (criteria 11, 12; decision 6);
  - the slice-2 seam: `slot_fit_matrix` into `place_page` (criterion 3; decision 7);
  - no `format_version` change (criterion 14; decision 8).
- [ADR 0049](../../docs/adr/0049-laser-fixtures-in-the-project-file.md): slots as child nodes, the
  rigid `fixture_matrix`; nothing stored about a slot's content.
- `0102` adrs.md §5: laser-core owns the fit rule; the drag fit of moves (`session/fixtures/fit.rs`,
  quarter-turned holders only) is unchanged.
- [ADR 0084](../../docs/adr/0084-document-library-index-read-only-sources-and-copy-import.md) §6,
  §7: `Fragment`, copy import and Came from; the copies keep their own Came from.
- [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §7:
  `place_page` with a matrix, the slice-2 target.
- [ADR 0051](../../docs/adr/0051-page-auto-layout-and-ownership-by-centre.md) §4: ownership by the
  box centre, the model for "filled".
- ADR 0034 §1, §2: no fixture kind in ui-core; ui-core gets only a kind-free uniform map.

## Feature-local notes (2026-11-25)

- Proposal: the fit scales stroke widths and corner radii always, whatever the maker's scale
  toggles, so the piece looks like the source page at that size.
- Proposal: the slot drag honours the Fixtures snap switch, as the `0102` move fit does; the
  Into slot list works with it off.
- Proposal: hidden and locked members fill a slot, and Replace (armed, one undo step) removes them.
- The fragment's private copy is written to (decision 3). It is never saved or committed into the
  source, which is what ADR 0084 §6 protects.
- `docs/adr/index.md`: ADR 0087 is on `story/library-keywords`; this branch adds the 0088 row only,
  so the index needs a one-line merge after whichever story lands second.

## Fill budget (architect, 2026-11-26; ADR 0088 decision 9)

- Measured at `eb63f973`, release: a fill of 50 slots with a 200-object page takes about 0.40 s of
  thread CPU time, linear at 8 ms per slot. 59 % is the shared copy kernel (`copy_subtree`, about
  23 µs per new object), 18 % the history record, 7 % the commit. One mapped fragment per fit and one
  batch are already built. 100 ms would need 10 µs per object in all.
- Decision: keep the insert path. Assert 50 x 200 within 500 ms and 15 x 200 within 150 ms (Proposal,
  spec change 9 below). A copy kernel that writes each register once is logged in
  `docs/technical-debt.md`.
- Fix in this story: the filled read is quadratic (`top_level` calls `Document::is_placement`, which
  rebuilds `placed_pages()` on every step). `slot_targets_view` after a fill of 50 takes 0.62 s.
  Read the placed pages once per call; assert `slot_targets_view` with 50 filled slots
  (10,000 objects) within 50 ms.

## Spec changes the product owner should make (2026-11-25)

1. **Criterion 6, test case 2 is wrong.** A 40 x 60 mm page turned is 60 x 40 and matches a
   60 x 40 slot within 0.1 mm, so it is turned and **not scaled**. Proposal for a turned and scaled
   case: a 40 x 60 mm page into a 30 x 20 mm slot is turned and scaled to 50 %.
2. **Criterion 6 (b), circle slot.** The architect fits a circle slot by the square of its
   diameter (`Slot::extent`, as `0102`'s fit does), not by its inscribed square: a round 30 mm
   coaster into a 25 mm circle becomes 25 mm, not 17.7 mm, and a 25.5 mm one is not cut to 72 %.
   A box fit never contains a shape anyway (out of scope). If the PO keeps the inscribed square,
   it is one line in `fit_into_slot`.
3. **Criterion 6, add (e):** a holder turned on the bed turns the unit with it.
4. **Terms, "Filled":** say top-level members (a whole group counts once), the outline box centre
   (a placed page: its frame), the closed slot outline grown by 0.1 mm; guides and dimensions do not
   count; hidden and locked members count and Replace removes them.
5. **Criterion 9:** a damaged slot is not read by the fixtures read (`0102` criterion 33), so there
   is no slot under the pointer and no reason badge; drop it from the list. Add the reason
   "Fixture snapping is off." (the drag honours the switch, the Into slot list does not).
6. **Criterion 2:** a whole page with one member is not wrapped (the same rule as one picked
   element); a wrap that would pass the group depth limit is refused with a notice.
7. **Criterion 3:** Came from is on the wrapping group (the source page as the item) and each copy
   keeps its own, as `0137` writes it.
8. **Criterion 16:** the fit function is in `curvyo-laser-core` (`fixture_fit.rs`), not
   `curvyo-editor-core` or `curvyo-ui-core`.
9. **Criterion 8, budget (2026-11-26):** replace "a fill of up to 50 slots with a page of 200
   objects takes within 100 ms" with "a fill of 50 slots with a page of 200 objects (10,000 new
   objects) takes within 500 ms, and a fill of 15 slots with a page of 200 objects within 150 ms, of
   thread CPU time" (ADR 0088 decision 9). Add: "the Into slot list of a holder with 50 filled
   slots (10,000 objects) is built within 50 ms".

## Review of the build (2026-11-26)

Architect, re-checked at `4f62c253`. The two must-fix findings (the freed reader behind the armed
Replace, guides filling slots) are fixed. Still open, none of them blocking:

1. **A failed fill can leave a partial step** (`CLAUDE.md` §5 errors). `Document::batch` commits
   whatever was written even when the closure returns Err, so a failed copy k > 1 of a fill would
   leave copies 1 to k-1 committed while an error is reported. The comment at
   `curvyo-editor-wasm/src/session/fixtures/into_slot_put.rs:288` says the opposite. No error path
   fails after the first copy today, so either fix the comment or check every copy before the
   batch. `let _ = document.delete_objects(remove)` at `into_slot_put.rs:327` drops a Replace
   delete error without a word.
2. **Units are newtypes** (§5). `fit_angle` (`into_slot_put.rs:116`) and `Mapped.angle`
   (`into_slot_put.rs:135`) hold the angle as a bare `f64`; use `Angle`.
3. **Function size** (§5). `fill_empty_slots` (`into_slot_put.rs:209`, 73 lines) and
   `Document::insert_fragment` (`curvyo-document-core/src/fragment_insert.rs:125`, 69 lines):
   split out the fill's planning loop (fits, shared fragments, offsets), or justify both in the PR.
4. **One responsibility per module doc** (§5). `into_slot.rs:1`,
   `curvyo-ui-core/src/slot_drop_text.rs:1` and `curvyo-editor-wasm/src/wasm_into_slot.rs:1` each
   join several jobs with "and"; reword each to one job.
5. **One undo step for a fill is not asserted.**
   `fill_puts_one_copy_into_every_empty_slot_and_counts_the_filled_ones`
   (`curvyo-editor-wasm/src/session/fixtures/tests_into_slot.rs:175`) checks only the last label;
   also assert `own_steps().len() == before + 1`, as the Replace test does.

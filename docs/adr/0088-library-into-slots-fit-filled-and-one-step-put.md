# ADR 0088: Library into slots: the fit in `curvyo-laser-core`, "filled" computed from the members, and a copy put into a slot as one step

**Status:** Proposed 2026-11-25 by the architect. Not `needs-customer`: no new crate, no dependency, no format change, no new register, no request that leaves the machine. Spec: [`0154`](../../specs/0154-library-into-slots/specification.md), Fragen 377 (C, slice 1 copy only), 378 (B), 379 (A) and 385 (C) at their defaults. Builds on [ADR 0049](0049-laser-fixtures-in-the-project-file.md) (slots as child nodes, the rigid `fixture_matrix`), the fit rule of `0102` adrs.md §5 (`curvyo-laser-core::fixture_fit`, 0.1 mm, the 90 degree turn), [ADR 0084](0084-document-library-index-read-only-sources-and-copy-import.md) §6 and §7 (`Fragment`, copy import, Came from), [ADR 0045](0045-clones-of-groups-texts-images-and-movable-placed-pages.md) §7 (`place_page` with a matrix, for slice 2) and [ADR 0051](0051-page-auto-layout-and-ownership-by-centre.md) §4 (membership by the box centre). Feature notes: [`0154` adrs.md](../../specs/0154-library-into-slots/adrs.md). **Amended 2026-11-26** (decision 9: the fill budget, measured after the build).

**In plain words (for the customer).** A slot keeps no record of what lies in it. The app looks at the drawing each time: a slot is filled when some piece's middle lies in it. Putting a page into a slot is the existing library import plus one size and turn, done once for all slots of the same size, so filling 15 slots is one undo step and nothing new is saved in the file.

## Context

`0102` already fits a moved item to a slot: `curvyo-laser-core::fit_slot` matches sizes within 0.1 mm and turns by 90 degrees only when the other orientation is the match; the drag in `curvyo-editor-wasm/src/session/fixtures/fit.rs` reads the Active holders once at the drag's start and only offers holders turned by a multiple of 90 degrees. `0137` copies a library page or picked elements with `Document::extract_fragment_placing` and `Document::insert_fragment` (`FragmentTarget::Elements { page, offset }` translates only), one step, Came from on every copied root. A group in our model has no transform of its own: scaling a group writes every leaf, kind by kind, through `curvyo-ui-core::group_transform` (`map_all`, `commit_group`: clone matrices settled, generated objects resized through their generator, gradients carried). `0154` adds scaling, a turn by the holder's angle, wrapping into one group, "filled" and a fill of up to 50 slots in 100 ms.

## Options

**Where the fit math lives**

| Option | Why it won or lost |
|---|---|
| **A. `curvyo-laser-core::fixture_fit`, beside `fit_slot` (chosen)** | The slot, its extent and the 0.1 mm rule are the laser extension's (`0102` adrs.md §5, ADR 0034 §1). Pure, sizes in `Length`, no document access; the table test sits next to the `fit_slot` tests. ui-core already depends on laser-core. |
| B. `curvyo-geometry-core` | Geometry-core knows no slot; the function would take two bare rectangles and the laser rule (tolerance, circle extent, the turn preference) would live in the caller. |
| C. `curvyo-ui-core` (the spec's suggestion) | ui-core must not learn fixture kinds (ADR 0034 §2); `Slot` is a laser-core type. |

**How the fit is written**

| Option | Why it won or lost |
|---|---|
| **A. Map the fragment's private copy once per distinct fit, then insert it with a translation (chosen)** | A fill of 15 equal slots maps 200 objects once and inserts 15 times; each insert is the unchanged `0137` path. The private copy is never saved or committed into the source (ADR 0084 §6), so writing to it is safe. |
| B. Insert, then transform the copies in the same batch | Every object is written twice per slot (10,000 inserts and 10,000 transforms for 50 x 200), clone settlement runs on the live document; the 100 ms budget is out of reach. |

**What "filled" is**

| Option | Why it won or lost |
|---|---|
| **A. Computed on read from the page's top-level members by box centre (chosen)** | The spec's "computed, never stored"; the same rule as ADR 0051 §4 (a page owns what its frame holds by centre). No format change, merges trivially. |
| B. A stored slot-to-content link | A new register, a format bump and a merge rule; out of scope (`0154` criterion 14). |
| C. Any overlap of the box with the slot | One large piece would fill every slot it touches, and a neighbour's spill would block a slot. |

## Decision

1. **The fit function** (`curvyo-laser-core/src/fixture_fit.rs`, criterion 6). `FIT_TOLERANCE` (0.1 mm, `Length`) moves here as a public constant; the session's `FIT_TOLERANCE_MM` uses it. New, pure:
   - `enum FitMode { Fit, RealSize }` (RealSize is the held Alt).
   - `struct SlotFit { scale: f64, turned: bool, matched: bool, overflows: bool }` (`scale` is a dimensionless ratio, not a length).
   - `fn fit_into_slot(unit: (Length, Length), slot: &Slot, mode: FitMode) -> SlotFit`, sizes in the board's frame, the slot's box is `Slot::extent` (a circle: the square of its diameter, as for `fit_slot`):
     1. `fit_slot(unit, [slot], FIT_TOLERANCE)` matches (either orientation): scale 1, its `turned`, `matched` true. A square in a square is never turned (`fit_slot` prefers unturned).
     2. `RealSize` without a match: scale 1, not turned; `overflows` when a side passes the slot's side by more than the tolerance.
     3. `Fit` without a match: `s0 = min(W/w, H/h)`, `s90 = min(W/h, H/w)`; a side of the unit at or below 1e-6 mm is left out of the minimum, and a unit with both sides there is not scaled. Turned only when the turn gains more than the tolerance on the unit's longer side `L`: `(s90 − s0) · L > FIT_TOLERANCE`. Scale up or down, aspect kept, margin 0.
   - `fn slot_fit_matrix(unit_centre: Point, fit: SlotFit, slot_centre: Point, board: Affine) -> Affine`: source document space to page document space, `board ∘ T(slot centre) ∘ R(turn) ∘ S(scale) ∘ T(−unit centre)`. `board` is the holder's rigid `fixture_matrix` (ADR 0049 rule 6: rotation and translation, no mirror), so a holder turned by any angle turns the unit with it; the 90 degree-only restriction of the drag fit (`fit.rs`) stays for moves and does not apply here.
   - Table-driven unit test: the cases of criterion 6 (as corrected in the notes), a circle slot, the tolerance boundary, the turn gain just above and below 0.1 mm, a zero-width unit, Alt with and without overflow.
2. **The unit and its box** (criterion 2). A whole page: its top-level members by the membership read, in paint order; its box is the page frame. Picked elements: the picked roots; their box is their joint outline box (`library_import::joint_box`). Two or more roots are wrapped into one new group; one root is not wrapped (for a page with one member too).
3. **The write.** `curvyo-document-core::FragmentTarget` gains one variant `Group { page, offset, name }`: the roots are copied as in `Elements` (`copy_fragment`, their own Came from unchanged) into one new group created in the draw parent, named `name` (the source page's name), carrying Came from with the source page as the item. A wrap that would pass `MAX_GROUP_DEPTH` is refused with a new `ImportError::TooDeep`; nothing is written. The linear part of the fit (scale, turn about the unit's centre) is applied to the fragment's private copy by a new public `curvyo-ui-core` function beside `flip` (`map_uniform(document, ids, pivot, scale, angle) -> bool`, built on `map_all` and `commit_group`, strokes and corner radii scaled, so every kind maps as in a group scale of `0010`); the translation is the insert's `offset`. A fill extracts and maps one fragment per distinct `(scale, angle)` and reuses it for every slot of that fit.
4. **"Filled"** (Terms, criteria 7, 8, 10). A slot is filled by every **top-level member** of the active laser page (a leaf or a whole group, layers transparent, ADR 0051 §4; own items by the membership read and the page's placed pages) whose **outline box centre** (`object_outline_bounds`; for a placed page its placed frame), mapped by the inverse of the holder's rigid matrix into the board frame, lies in the slot's **closed outline grown by `FIT_TOLERANCE`** (rectangle: `|dx| ≤ W/2 + 0.1 mm` and `|dy| ≤ H/2 + 0.1 mm`; circle: distance `≤ D/2 + 0.1 mm`). Guides and dimensions never fill; hidden and locked members do, and Replace removes them. Only the centre counts: a large piece fills at most the slots its centre lies in, and overlapping slots can both be filled by one piece. Pure test in laser-core: `Slot::holds(point, tolerance) -> bool`; the session gathers the members and the centres.
5. **Session commands** (`curvyo-editor-wasm/src/session/fixtures/`), each one history step through `recording_unguarded` and `Document::batch`, the nested `insert_fragment` and delete commits joining it:
   - `begin_slot_drop(unit)` / `set_slot_drop_source(source)` / `slot_drop_cue(at, alt, shift) -> SlotDropCue` / `end_slot_drop()`: targets and their filled state read once at the drag's start (the Active holders of the active laser page, and the slots of the other laser pages for criterion 9's activation); per move a point-in-outline test (the outline grown by the `0102` hit tolerance) over at most 60 slots, the slot drawn last wins where slots overlap (UX notes 3.1). The cue holds the slot id, its name, filled, and the reason when there is no target. Like the `0102` fit, the drag honours the Fixtures snap switch; the Into slot list does not.
   - `put_into_slot(from: &SlotSource, slot, mode, replace) -> Result<SlotDrop, String>` (`SlotSource`: the read-only source session, the picked roots or the page, the Came from record), label `put_into_slot` ("Put into slot"), or `replace_slot_content` ("Replace slot content") when `replace` is set: the filling members are deleted (`delete_objects`; a placed page loses only its reference) and the unit is put, in one batch. A filled slot without `replace` writes nothing and returns `SlotDrop { refused: Some(name), .. }`.
   - `fill_empty_slots(from: &SlotSource, holder, mode) -> Result<SlotDrop, String>`, label `fill_slots` ("Fill slots"): the empty slots are computed once before the first write, in slot creation order; each copy lies above the previous one; none empty writes nothing.
   - `slot_targets_view() -> JSON` for the Into slot list (criterion 10): holders, slots with name, shape, size in mm and filled, or the reason the button is disabled.
   - After a put or a fill the new units are selected and the tool is Select. The three labels go into `curvyo-ui-core::history_names` and `step_scope::FREE_STEPS` (creation and import).
6. **Operations and jobs** (criteria 11, 12). The unit is an ordinary own item or group of the laser page: Not assigned (Frage 155 A), a work item for `0103` criterion 10, never in a fixture job, ignored by Save as profile and Update profile. It is not a child of the holder. No code in the job build, the operations or `0113`.
7. **The slice-2 seam** (Frage 377 C, a live placed page). `slot_fit_matrix` already gives the whole map. Slice 2 converts it into the placement's page-local matrix (`T(−laser page origin) ∘ M ∘ T(source page origin)`) and calls `Document::place_page(output, source, matrix)` inside the same batch; for a page of another document, after the `0137` page import. `put_into_slot` and `fill_empty_slots` gain the copy-or-place choice then, not now (`CLAUDE.md` §5, no speculative options). A placed page already counts as filling by decision 4.
8. **Format.** None: `format_version` unchanged. Came from on a group is an existing key on an existing node shape; the group, the copies and a later placement are existing kinds.

## Consequences

- One new pure function pair and one predicate in laser-core, one new `FragmentTarget` variant and one `ImportError` variant in document-core, one public map function in ui-core, three session commands and a cue. No new crate, no new edge (`ui-core → laser-core` exists).
- The fill budget of criterion 8 is set by decision 9 (amendment 2026-11-26): 100 ms for 50 x 200 was missed by four times, and the write cost per new object, not the fill's structure, is the reason.
- The circle slot is fitted by its diameter square, so a round piece keeps its size; a rectangular piece in a circle slot can reach past the circle by its corners (a box fit is never a shape fit, `0154` out of scope). Changing that to the inscribed square is one line in `fit_into_slot`.
- A unit whose content is off-centre on its page and larger than the slot (Alt) can leave its own slot reading empty, because its group's box centre is not the frame's centre. Accepted; the notice already says it does not fit.

## Amendment 2026-11-26: the fill budget (decision 9)

**Measured** on `story/library-into-slots` at `eb63f973`, release build, thread CPU time, the scenario of `curvyo-editor-wasm/tests/into_slot_budget.rs` (50 slots of 30 x 30 mm, a 60 x 40 mm page of 200 triangles, so 10,000 new objects in 50 wrapping groups). `perf` is not installed; the split comes from temporary timers around the phases (removed again).

| Fill of n slots | 1 | 5 | 10 | 25 | 50 |
|---|---|---|---|---|---|
| CPU time | 14 ms | 45 ms | 84 ms | 206 ms | 395 to 403 ms |

It is linear, about 8 ms per slot. For n = 50 (about 0.40 s):

| Phase | Time | Share |
|---|---|---|
| `copy_roots` (one Loro node per object, every register, the anchor list entry by entry, then `renumber_anchors` and `translate_meta` writing the anchors again) | 236 ms | 59 % |
| The history record after the commit (`recording_unguarded`) | 74 ms | 18 % |
| The commit of the one batch | 28 ms | 7 % |
| Relink, checks, the private source's clone links per insert, Came from, the group | about 50 ms | 12 % |
| Extract and map the fragment once (decision 3) | 7 ms | 2 % |

The levers the lead listed are already in place or worth little: one mapped fragment per distinct fit (decision 3, built), one document write (already one batch; the 50 nested batches cost nothing measurable), and the per-insert checks and relink together cost about 30 ms. The cost is about 23 µs per new object in the shared copy kernel (`objects::copy_subtree`, also used by paste, duplicate and every import), plus about 10 µs per object for the commit and the history record. A budget of 100 ms would need 10 µs per object in all. Even a copy kernel that writes every register exactly once (an estimated 30 to 40 % of `copy_roots`) would land near 0.30 s.

9. **Decision: keep the insert path and change the budget (Proposal, the PO changes criterion 8).**
   - A fill is a command with a notice, not a frame. The assert becomes: **a fill of 50 slots with a page of 200 objects (10,000 new objects) within 500 ms of thread CPU time**, and **a fill of 15 slots with a page of 200 objects (the jig of the user value, 3,000 objects) within 150 ms**. Both go in `into_slot_budget.rs`, with `CURVYO_BUDGET_FACTOR` as for every budget test. Today: about 0.40 s and 0.12 s.
   - No new insert API (`insert_fragment_many` was rejected: about 4 % for a second public path). Rewriting `copy_subtree` so it writes each register once is shared work on the copy kernel and goes into `docs/technical-debt.md`, not into this story.
   - **Fixed in this story (found by the same measurement):** the filled read is quadratic. `slot_targets_view` after the fill of 50 takes 0.62 s (10 slots 35 ms, 25 slots 178 ms). `member_centres` calls `page_top_level`, and its `top_level` asks `Document::is_placement` on every step of every leaf, which builds `placed_pages()` again each time. Every put checks "filled" first, so a put into a full jig and the Into slot list each cost about 0.6 s. Fix: read the placed pages once per call and walk the parents with that set (session side, behaviour-preserving). Budget: **`slot_targets_view` on a laser page with 50 filled slots (10,000 objects) within 50 ms of thread CPU time**, asserted in the same file.

## If a Frage is answered otherwise

- 377 A: decision 7 is never built. 377 B: decision 3 is replaced by decision 7 for pages of the open document; elements are refused.
- 378 A: `fill_empty_slots` moves to slice 2; nothing else changes.
- 379 B: `fit_into_slot` is called with `RealSize` only. 379 C: the frontend asks and passes the mode.
- 385 A: `put_into_slot` always replaces. 385 B: the filled check is skipped and nothing is deleted.

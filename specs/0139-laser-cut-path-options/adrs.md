# ADRs for laser cut path options (tabs, start and end, lead-in, lead-out, overcut)

Architect, 2026-11-26. Covers criteria 1 to 25 of `specification.md` (revision 2026-11-26), both
slices. **No new ADR, no new crate, no new dependency.** One `format_version` bump for the whole
story (L6). The decisions below are amendments by note to the ADRs listed first, in the same way
`0111` (ADR 0056 §6) and `0118` slice 2d (adrs P7, P8) changed the operation and profile
registers. Everything below is a **(Proposal)** unless it quotes an ADR, a criterion or a
customer answer (Fragen 279 A, 280 A, 281 A; 407 A as the default).

## ADRs this feature depends on or amends

- [ADR 0046](../../docs/adr/0046-laser-operations-record.md) §2 to §4, §7, §8: membership keys
  `member.<key>` on the operation node (node id, clone path, and since format 65
  `<node>#<part key>`); the nearest rule decides the target; deleted targets drop their keys in
  the same step; copies get the keys, linked clones start with none. **Amended by L3 to L5:** three
  more key families on the operation node, keyed by the same `<key>` text, swept, copied and
  removed with the membership.
- [ADR 0056](../../docs/adr/0056-laser-operation-tools-in-the-project-file.md) §6: the atomic
  `op_options` register (operation-only settings, never touched by materials). **Amended by L2:**
  eight keys on `cut` and `cut_out` only; every other kind refuses them as `KeyOnKind`.
- [ADR 0064](../../docs/adr/0064-laser-job-builder-and-output-writers.md) §5 (order), §7 (moves),
  §9 (plan cache), §10 (Toolpath view from the moves). **Amended by L7 to L12:** the cut path step
  inside the plan; the order gets single-entry candidates; the moves, the writers, the Timeline,
  the Frame and the bed check are unchanged and read the new paths as they read any burn path.
- ADR 0069 (raster) and `0123`: untouched; no block of this story applies to Score, Engrave,
  Engrave + Outline, Image, raster sweeps, hatch spans, fixture jobs, test grids or registration
  marks.
- ADR 0016 / spec `0057`: the all-or-nothing rule per register (own placement, own style) is the
  model for "own tabs" and "own path options": the register's presence means "own".

## Feature-local decisions

**L1. Crate placement.** All rules in `curvyo-laser-core` (wasm32, `#![forbid(unsafe_code)]`, no
I/O); `curvyo-document-core` stores the new registers as opaque JSON and sweeps them (ADR 0046
§8: document-core decides no rule); `curvyo-editor-wasm` reads, resolves per leaf with a core
function, commits, and draws what core returns; the frontend only draws and forwards pointer
events. `curvyo-generators-core` is not touched. New laser-core modules, each one responsibility
and under 500 lines: `operation_cut_options.rs` (the operation's cut settings: newtypes, keys,
read and write, the pattern of `operation_image_options.rs`), `cut_item_records.rs` (the three
item registers: codec and damage), `cut_tabs.rs` (automatic placement, criterion 2 and 3, and the
edit rules of criteria 5 and 6: slide, add, delete, the 1 mm limit, node snapping),
`job_cut_path.rs` (the path rewrite of L8), `job_cut_lead.rs` (lead geometry and waste side,
L9), `job_cut_checks.rs` (the two warn counts, L11). `job_order.rs` is at 497 lines: a
behaviour-preserving commit first moves `parents`, `bounds`, `inside_box` and `winding` to
`job_nesting.rs`, so the order change of L7 fits.

**L2. Operation values: eight more `op_options` keys, on `cut` and `cut_out` only.**
`tab_count` (whole number 0 to 20), `tab_width_mm` (0.10 to 20.00, two decimals),
`tab_skip_inner` (boolean), `lead_in` and `lead_out` (`"none"`, `"line"`, `"arc"`),
`lead_in_mm` and `lead_out_mm` (0.10 to 20.00), `overcut_mm` (0.00 to 10.00). Absent reads as
the default (0, 0.50, false, none, 1.00, 0). `OperationOptions` gains one field `cut:
CutSettings` (stays `Copy` and `Hash`); `to_value` writes the eight keys for Cut and Cut-out
only. Damage is the existing rule of ADR 0056 §6 (`0111` criterion 25): a wrong type, a value out
of range or with more decimals, or a key on another kind reads the whole map as damaged
settings, the operation runs nothing, the map is saved back unchanged. Newtypes, integer inside:
`TabCount(u8)`, `CutLength(u16)` in hundredths of a mm (tab width, lead length; one type, two
ranges as associated constants), `Overcut(u16)` in hundredths, `LeadKind` enum. Typing uses
`typed_steps` with the ranges' chip texts (criteria 1, 13). None of these is a material number
or a profile field (criterion 22); materials never write `op_options`.

**L3. Item records: three registers on the operation node, keyed by the membership key.**
`<key>` is the exact text that follows `member.` for the target the nearest rule chose (ADR 0046
§2, §7; criterion 21), so clones, clone members, placed-page inner rows (keyed by the source
object, shared by every placement of it, as ADR 0046 §2) and detached generator parts work
without a new key form.
- `tabs.<key>`: **own tabs** (criterion 5). Present = own, absent = the operation's. Value: a
  map from an outline reference to a list of `{ "at": <ppb>, "width_mm"?: <n> }`, ordered by
  `at`. An outline missing from the map follows the operation's placement (a leaf added to a
  group later gets the operation's tabs); an empty list means no tabs.
- `path_own.<key>`: **own path options** (criterion 21, Frage 280 A): one map with all five
  values `lead_in`, `lead_in_mm`, `lead_out`, `lead_out_mm`, `overcut_mm`, ranges as L2.
- `entry.<key>`: **start points and directions** (criteria 19, 20): a map from an outline
  reference to `{ "start": <ppb> }` (closed) or `{ "direction": "as_drawn" | "reversed" }`
  (open). Missing = Automatic (Frage 407 A); Automatic deletes the entry, the last one deletes
  the register.
- **Outline reference:** `"<n>"` when the target is the leaf itself (n = the outline's index in
  the leaf, a compound path's subpath), `"<path>:<n>"` for a leaf below the target (`<path>` the
  node ids from below the target down to the leaf, `encode_node` joined by `/`, clone members by
  their member ids as in ADR 0046 §2), `"<n>"` = the contour index for a Cut-out item, and `"0"`
  for an image's frame. A reference that names no current outline is ignored and kept.
- **Fractions** are integers in parts per billion of the flattened outline length,
  `PathFraction(u32)`, 0 to 999,999,999 (a closed path wraps, so 1.0 is 0): exact in the file and
  in Loro, equality and damage checks without float noise, 0.004 mm resolution on a 4 m outline.
  Measured from the outline's first stored point along its drawn direction (criteria 2, 5, 19).
  A path edit keeps the fraction (criteria 11, 22).
- **Per operation, not per page.** An item in two Cut operations has a record on each; the
  marker and handles on the canvas edit the operation named by the item block row (the UX notes
  name it). This keeps one record shape, one sweep and the nearest rule's target per operation.
- **Damage:** a register that is not a map, an unknown field, a fraction outside the range, a
  width or length outside its range, or two tabs that overlap makes that item **run nothing in
  that operation** (safer than cutting a held part loose) and counts in `DamagedSettings` (spec
  issue 4); the register is kept and saved back unchanged.

**L4. Lifetime of the records (amends ADR 0046 §3, §4).** The sweep that drops dead `member.`
keys (`sweep_dead_member_keys`, `member_entries`) reads the three new prefixes too, under the same
`member_keys_may_exist` flag (a record write sets it). Removing an item from an operation (its
`member.` key goes) removes its three records of that operation in the same commit; deleting an
item, a clone that stops being a clone (§2 c), an item moved off the page and an operation delete
take them as they take the keys today (criteria 11, 22; `0101` criterion 27). Ctrl copy,
Duplicate and Paste copy the records with the key (§4); a new linked clone starts with none.
Undo brings them back with the step (ADR 0014).

**L5. Steps.** Every change is one commit with the criterion's step name: "Add tab", "Move tab"
(one step per drag, written on release; the drag itself is session state), "Delete tab", "Tab
width", "Own path options", "Back to the operation's", "Move start point", "Automatic start",
"Direction", "Edit settings" (criteria 12, 23). The first tab change writes the whole own record:
every outline of the item with the operation's current placement as fractions, then the change
(all or nothing, criterion 5). Back to the operation's deletes the register; because the
automatic placement is a pure function of path and settings, the output is byte for byte the
operation's again (criterion 5 test).

**L6. Format: `format_version` 66 (the next free number at merge; main wrote 65 on
2026-11-26; if the generators session takes 66 first, this story takes the next).** Why a bump:
a build of 65 meets an unknown `op_options` key and reads the whole map as damaged (the Cut runs
nothing), and it ignores `tabs.`, `entry.` and `path_own.`, which would cut held parts loose:
wrong laser output, the case ADR 0033 §7 and ADR 0065 §4 refuse by version. Older builds refuse
the file with the newer-version message (criterion 11). No migration: a file of 65 or lower
opens unchanged with the defaults and nothing is written on open. The `CURRENT_FORMAT_VERSION`
doc comment gets the version paragraph. Goldens:
`curvyo-document-core/tests/fixtures/laser_path_options_v66.curvyo` and
`laser_path_options_damaged_v66.curvyo` as listed in the spec; the damaged cases are read by the
laser-core codec in a laser-core test that opens the fixture through document-core (as
`operation_options_damaged_v44.curvyo` is read today), each reads damaged and saves back byte
for byte.

**L7. Where the rewrite sits in the job (amends ADR 0064 §5).** The session resolves, per leaf
and per Cut-out item, a `LeafCut` (the operation's `CutSettings`, the item's own tabs per
outline, own path options, entries) with one core function, `cut_of(settings, records,
outline_ref)`, and passes it in `LeafInput.cut` / `CutOutItem.cut` (types only, as ADR 0064 §1).
Fixture blocks and test-grid derived operations (`grid: Some`) always get the default, so their
output cannot change. In `plan_block` each path unit carries an `Option<Arc<CutUnit>>`, `None`
exactly when the resolved values are the defaults (Count 0, no own tabs, leads None, overcut 0,
Automatic): that unit takes today's code path, which is the guard of criterion 24 (every
existing GRBL golden passes unchanged). `order()` computes the nesting once (`job_nesting.rs`);
with it each `CutUnit` gets (a) its tabs as arc-length spans on the flattened outline (automatic
ones skipped under Skip inner shapes when the unit has a parent, criterion 3), (b) its depth for
the waste side. Candidates (the nearest rule of `0029` criterion 22):
- closed, Automatic, no tabs: every vertex (today);
- closed, Automatic, with tabs: every burnt vertex and every piece start (spec issue 2);
- closed with a set start: one candidate, the start point (inserted as a vertex);
- open, Automatic: the first burnt point forward and the last burnt point reversed;
- open, As drawn or Reversed: one candidate.
The distance is measured to the entry on the path, not to the lead-in's start, so adding a lead
never reorders a job; the head after the unit is its real exit (lead-out end, overcut end or
last piece end), so the next choice is still nearest-from-the-head. Inner before outer comes from
the whole shape and is unchanged (criterion 9).

**L8. The rewrite (`job_cut_path.rs`), one pure function per entered unit.** In: the flattened
µm outline rotated to the entry, its tab spans, leads, overcut, depth. Out: the **pieces** in run
order, each one `PlannedPath` of the unit's node: lead-in points + first piece, the other pieces,
the last piece + overcut + lead-out. Rules, in this order: tabs are cut out of the outline first
(tabs win, criterion 15); the closed path runs once around from the entry, wrapping once; the
overcut exists only when the last piece ends at the entry (no tab covers the closing point),
runs on along the path by `min(d, L)` and stops at the first tab start; the lead-in ends at the
first burnt point, the lead-out starts at the last burnt point (the overcut's end when there is
one). Tab boundaries are interpolated on the polyline and rounded to µm (each end within 0.0007
mm, inside the 0.005 mm of criteria 8 and 15); a boundary that rounds onto a vertex is merged. A
gap is then simply the Travel `build_moves` already writes between two `PlannedPath`s (laser
off, `G0`, GRBL `M5` / `M3 S<n>` with M3, no dwell; criterion 8): **`job_moves.rs`,
`job_write_grbl.rs`, the Timeline, the Frame and the bed check need no change**; Passes repeat
the run's paths, so every pass writes the same gaps, leads and overcut (criteria 9, 16). A unit
whose tabs cover everything yields no piece and counts for L11. Arc length is the sum of the
flattened segments (criterion 8 measures on the written path). Leads and overcut never apply to
open paths (criterion 13). In `job_moves.rs` the overscan locals `lead_in` / `lead_out` are
renamed `run_in` / `run_out` in the refactor commit, so "lead" means only this story.

**L9. Lead geometry and waste side (`job_cut_lead.rs`).** Waste side: depth from the nesting of
L7 (even = outside, odd = inside the hole; no parent = depth 0, so a hole cut alone gets its lead
outside, as decided) and the outline's orientation from the exact integer shoelace area (`i128`);
a zero-area outline gets no lead. Direction at the junction: the leaving segment (lead-in) or the
arriving segment (lead-out), criterion 14. Line: one segment of Length along the waste-side
normal. Arc: a quarter circle of radius Length, tangent at the junction, centre on the waste side,
written as `2^m` chords with the smallest `m` whose sagitta is at most `FLATTEN_MM` (0.0095 mm),
points found by repeated bisection (midpoint of two circle points pushed out to the radius), so
only `+ - * /` and `sqrt` are used: no `sin`, `cos` or `hypot`, whose results depend on the
platform's libm, so native and wasm32 write the same bytes (the rule already in
`job_moves::extend`). Leads are **not** shortened to the bed (unlike the overscan run-out): a lead
outside the work area blocks through 4 b (criterion 17).

**L10. Determinism and tolerances.** Every stored and typed value is an integer newtype (L2,
L3). Geometry stays in integer µm (`PointUm`); the only floats are interpolation and the arc,
each rounded once to µm with the existing `round`. Ties in the order keep ADR 0064 §5 (distance,
stacking, outline index, candidate index, unit index); candidates are listed vertices first,
then piece starts by fraction. No `HashMap` iteration reaches output. Named constants, as
`Length` / `Tolerance` values, not bare literals: `MIN_PIECE` 1 mm (criterion 5), `NODE_SNAP_PX`
8 (converted to mm by the caller's view scale, which core receives), `LEAD_TOUCH` = `Tolerance`
0.01 mm (criterion 18), `BED_TOLERANCE_UM` (existing, criterion 17). `plan_fingerprint` (ADR 0064
§9) hashes `options.cut` and every leaf's and item's `LeafCut`, so the plan cache never serves a
plan with old tabs.

**L11. Checks (`job_cut_checks.rs`, words in `job_gate_text.rs`).** Two new warn `Cause`s,
counted in the plan and read by `gate`: `CoveredByTabs` (criterion 10: "<n> paths are covered by
tabs and cut nothing.") and `LeadsTouchCut` (criterion 18: "<n> leads cross or touch a cut.").
The touch test checks each lead and overcut segment against the other burn paths of the same
block within 0.01 mm, its own path excepted on the stretch from the junction until the lead
first leaves the 0.01 mm band (an arc lead runs tangent for a while) and the overcut, which lies
on its own path, against other paths only; boxes first, then segments, so the cost stays linear
in practice (L13).

**L12. Views: core builds, the UI draws (amends ADR 0064 §10).**
- **Toolpath view:** nothing new; it already draws the moves. Gaps are Travel (dashed), leads
  and overcut are Burn in the kind look, and every piece starts after a Travel, so it gets its
  arrowhead. The implementer confirms in `session/output/toolpath.rs` that an arrowhead is drawn
  per burn run after a Travel, not per node.
- **Operations view:** `Plan` gains `marks: Vec<CutMark>`, built by the same function as the
  pieces (L8), so the marks cannot differ from the file: per entered outline its node, the tab
  gaps (two µm points each, plus the bar's normal), the entry point with its direction, the
  lead-in and lead-out polylines and the overcut polyline. `Built` exposes them per block; the
  Operations view look (`session/operations/look.rs`) reads them from the page job (`PageJob`,
  built lazily and cached; while it is stale the previous job's marks show, the stale rule of
  `0029`).
- **Handles (selected item, normal and Operations view, never in the Toolpath view):**
  `cut_tabs.rs` exposes, for one outline, the handle points and tangents of its tabs and start
  marker from the item's own flattened outline (not the job), and the edit functions (slide with
  node snapping, the 1 mm limit, add at a pointer, delete), so a drag needs no job rebuild.

**L13. Budgets (criterion 25), confirmed.** (a) 1,000 Cut items of 20 nodes with Count 4,
Lead-in Arc 1 mm, Lead-out Line and Overcut 2 mm build within **150 ms** thread CPU time,
checks of L11 included; the case is added to `curvyo-editor-wasm/tests/job_builder_budget.rs`
next to `0029` 36 (a) (that is where the thread clock and `CURVYO_BUDGET_FACTOR` live, per
`0029` adrs "Dependencies"; asserted in release builds); 36 (a) itself stays 100 ms with the
defaults. (b) The handles and markers of a selected item of 20 nodes with 20 tabs redraw within
**16 ms** per pointer move (a new case in `operation_tools_budget.rs`). No new budget for the
plan cache: an edit that does not touch cut settings keeps the cached plans.

**L14. Amendment notes for the story PR** (the implementer adds them; text proposed):
- `0029` criterion 22: "*Amended by `0139` (2026-11-26): a closed path with a set start point and
  an open path with a fixed direction have one entry; a closed path with tabs is entered at a
  burnt vertex or a piece start.*"
- `0101` criterion 19: "*Amended by `0139` (2026-11-26): the direction of an open path may be
  fixed per item; tabs leave gaps in the output, the outline itself is unchanged.*"
- ADR 0046, 0056 and 0064 Status lines: "Note (`specs/0139` adrs.md, 2026-11-26): ..." naming
  L3 to L5, L2, and L7 to L12.

## Spec issues (for the product owner; each with a Proposal)

1. Criterion 19 says an Automatic path's marker rests at the first stored point, but its
   lead-in and the Operations view marks sit at the job's real entry (nearest vertex). Proposal:
   the Automatic marker shows at the job's entry in the automatic look; a drag makes it a set
   start.
2. Criterion 9 "entered at the nearest burnt vertex or piece end": entering at a piece's end
   and running forward goes straight into the tab. Proposal: "piece start".
3. A set start point that lies inside a tab (criterion 19 "+/- 0.005 mm" cannot hold).
   Proposal: the start marker cannot be dropped inside a tab (it stops at the tab edge, as the 1
   mm limit), and if a later tab covers it, the burn begins at that tab's end.
4. Damaged item records are not covered by criteria 11, 22 or the damaged fixture's
   behaviour. Proposal (L3): the item runs nothing in that operation and is counted in the
   existing "damaged settings" warn; saved back unchanged.
5. Criterion 2 allows automatic pieces shorter than the 1 mm of criterion 5 (small path, small
   width). Proposal: k = min(n, floor(L / (4 x Width)), floor(L / (Width + 1 mm))); the spec's
   test values do not change.
6. Unassigning an item drops its own records; copies keep them (L4). Proposal: state both in
   criteria 11 and 22.
7. Criterion 25 names `curvyo-laser-core` for the budget; the test lives in
   `curvyo-editor-wasm/tests/` (L13). Proposal: name that path.
8. Criterion 10 with every path covered: the job burns nothing and `0029` 4 (d) "nothing runs"
   blocks, while criterion 10 says it still runs. Proposal: 4 (d) stays (nothing to send);
   criterion 10 says "the job still runs when anything else burns".

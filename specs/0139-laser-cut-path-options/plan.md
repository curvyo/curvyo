# Plan for laser cut path options, slice 1: tabs

Implementer, 2026-11-26. Slice 1 only (criteria 1 to 12, 24, 25 a and b for tabs, the tab goldens,
the format bump, the amendments of `0029` 22 and `0101` 19, the ADR Status notes). Slice 2 (leads,
overcut, start points, directions) keeps its own plan section when it starts. Where `ux-notes.md`
and `adrs.md` disagree the ADR wins (no start marker in this slice at all; item records per item
and operation).

## Affected crates/modules

- `curvyo-laser-core` (rules, L1):
  - `operation_cut_options.rs` (new): `TabCount`, `CutLength`, `CutSettings`, the three `op_options`
    keys `tab_count`, `tab_width_mm`, `tab_skip_inner` on `cut` and `cut_out` only (L2; slice 2 adds
    the lead keys).
  - `cut_item_records.rs` (new): `PathFraction`, the `tabs.<key>` register codec and its damage (L3).
  - `cut_tabs.rs` (new): automatic placement (criteria 2, 3), tab spans on a flattened outline, the
    edit rules of criteria 5 and 6 (slide, add, delete, width, the 1 mm limit, node snap), handle
    points and tangents (L12).
  - `job_nesting.rs` (new, refactor first): `parents`, `bounds`, `inside_box`, `winding` moved out of
    `job_order.rs` without a behaviour change; `job_moves.rs` locals `lead_in`/`lead_out` renamed
    `run_in`/`run_out` (L8).
  - `job_cut_path.rs` (new): the rewrite of one entered unit into pieces and its marks (L8, L12).
  - `job_order.rs`, `job_plan.rs`, `job_input.rs`, `job_fingerprint.rs`, `job_gate*.rs`: the
    `LeafCut` input, the candidates of L7, `Plan.marks`, `Plan.covered`, the warn `CoveredByTabs`
    (criterion 10) and damaged item records in `DamagedSettings`.
- `curvyo-document-core`: `tabs.<key>` on the operation node (opaque plain value), read into
  `OperationSnapshot.tabs`, written by one command with the step label, removed with the membership
  key and by the dead-key sweep (L4); `format_version` 66; goldens
  `laser_path_options_v67.curvyo`, `laser_path_options_damaged_v67.curvyo`.
- `curvyo-ui-core`: history names of the new steps.
- `curvyo-editor-wasm`: the resolved `LeafCut` per leaf and Cut-out item into the job input; the
  options edits (Count, Width, Skip inner shapes); the item part's Tabs pill; the Tabs tool and
  tab handles (add, drag, delete, width chip); the Operations view tab marks from `Plan.marks`; the
  budget cases.
- `frontend`: the Tabs group of the lower part, the item part (Tabs row only), the Tabs toggle in
  the Operation group, the capsule handles, the width chip, the Operations view tab bars.

## Tasks

- [x] 1. Refactor commit: `job_nesting.rs` out of `job_order.rs`, `run_in`/`run_out` in
  `job_moves.rs` (no behaviour change; all goldens unchanged) (L1, L8; AC 24).
- [x] 2. `operation_cut_options.rs`: newtypes, keys, defaults, typed values with the chip texts,
  `KeyOnKind` off Cut and Cut-out, `OperationOptions.cut` (AC 1).
- [x] 3. `cut_item_records.rs`: `tabs.<key>` read and write, damage (not a map, unknown field,
  fraction out of range, width out of range, not ascending) (AC 5, 6, 11).
- [x] 4. `cut_tabs.rs`: automatic k and centres (AC 2), Skip inner shapes (AC 3), own tabs and
  own widths per outline reference (AC 4, 5, 6), overlap at a length as damage (AC 11), slide with
  the 1 mm stop and node snap, add at a pointer, delete, width with "No room for this width."
  (AC 5, 6), the placed-line text (AC 2).
- [x] 5. `job_cut_path.rs` and the order: pieces, gaps as Travel, entry at burnt vertices and
  piece starts, passes repeat, covered paths (AC 8, 9, 10); `Plan.marks` (AC 7); fingerprint
  (L10); defaults take today's code path byte for byte (AC 24).
- [x] 6. Gate: `CoveredByTabs` warn with its texts (AC 10); damaged item records run nothing and
  count in `DamagedSettings` (AC 11).
- [x] 7. G-code goldens `tabs_*.gcode` (AC 2, 3, 5, 6, 8, 9, 10) with an independent read-back of
  gap and burn lengths; every existing golden unchanged (AC 24).
- [x] 8. Document: `tabs.` register read, written, swept with the membership, removed on
  unassign and delete, undo; format 66 with the refusal by older builds; the two document
  goldens (AC 11, 12).
- [x] 9. Session: `LeafCut` per leaf through the nearest rule's target key and outline reference
  (AC 4); options edits (AC 1); item part view and pill (AC 5); tab add, move, delete, width, own,
  back (AC 5, 6, 12); handles and their limits; Operations view marks (AC 7); Checks line (AC 10).
- [x] 10. Budgets: job of 1,000 Cut items of 20 nodes with Count 4 within 150 ms; handle redraw of
  a 20-node item with 20 tabs within 16 ms (AC 25; slice 2 adds leads and overcut to case a).
- [x] 11. Frontend: Tabs group (Count, Width, placed line, Skip inner shapes), item part Tabs
  row, Tabs toggle (Frage 415 C needs no start marker in this slice), capsule handles, width chip,
  Operations view bars; no hint text (AC 1, 2, 5, 6, 7).
- [x] 12. Docs: amendments `0029` 22 and `0101` 19, Status notes of ADR 0046, 0056, 0064, the
  `CURRENT_FORMAT_VERSION` paragraph, design-system rows (ux-notes 13, tab rows only).

## Deviations (Proposal, vetoable at the demo)

- The cut path keys are written to `op_options` only when one differs from its default, so a
  project without tabs stores the same map as before (criterion 24).
- Tab handles use the round "Parameter handle" glyph; the capsule of `ux-notes.md` 5.1 needs a new
  render-core glyph. The width field of the current tab sits in the item part, not in a canvas
  entry chip; double-click and Enter on a handle do not open a chip yet.
- With the Tabs tool on, a press near a selected cut path adds the tab at the press (no 3 px dead
  zone and no drag-to-move from there); Tab, the arrows and Insert do not act on handles yet
  (`ux-notes.md` 7); no sr-only announcements of the current handle.
- The handles show in the item's first Cut or Cut-out of the run order, not the unfolded row's.
- Copies (Ctrl copy, Duplicate, Paste) keep no tab records: the copy of membership keys of ADR
  0046 §4 is not built in this codebase (the `0101` tester notes the missing seam), so the
  records have nothing to follow.
- The document golden holds an own item, a group's member outline and a Cut-out item; no clone
  and no placed-page row (their key forms are the membership's, covered by `0101`).
- `acceptance_0029_tester.rs` c31 and `acceptance_0023_tester.rs` ac27 exclude the new goldens
  by prefix, as earlier stories did.

## Tester round 1 (2026-11-27, lead decisions, Proposal)

- F1: a closed outline with tabs is entered at a burnt vertex that is no piece start (else the
  middle of a piece), never at a tab's end, so every tab is one G0 between two pieces (adrs L7
  "piece start" candidates dropped).
- F2: criterion 8's "no S above 0 in the gap (GRBL M4)" means no beam in the gap: under M4 a G0
  never fires the laser, so the modal S of the burn before it may stay in force; no S0 word is
  written. The product owner updates the spec text.
- F3: on an open path the automatic count is also at most floor(L / (W + 2 mm)), so both end
  pieces keep 1 mm; closed paths keep floor(L / (W + 1 mm)).

## Validation

- Unit tests first in each new laser-core module; the spec's numbers as tests (40 mm square,
  100 mm line, 10 mm circle, 1 mm stop, 94/6 mm burn and gap).
- GRBL goldens read back by a parser in the test (burn and gap lengths, `M5`/`M3` around gaps,
  no `S` above 0 in a gap), every existing golden re-run unchanged.
- Document goldens: open, read, save back byte for byte; a 65 build's refusal through the
  newer-version check.
- Thread CPU time budgets in `curvyo-editor-wasm/tests/`.
- Full gate of `CLAUDE.md` §7 once at the end.

# Plan addendum, slice 2: path options (start and end, lead-in, lead-out, overcut)

Implementer, 2026-11-27. Criteria 13 to 23, 24, 25 (a and b with leads and markers), the path
goldens, `format_version` 68 (the next free number after slice 1's 67). Where `ux-notes.md` and
`adrs.md` disagree the ADR wins: start and direction records are per item and operation
(`entry.<key>` on the operation node, the item part shows the values of its row's operation), and
a start marker never rests inside a tab (it stops at the tab's edge; a later tab over a set start
moves the burn start to that tab's end). Frage 414 A (Automatic until set) and 415 C (an Automatic
marker only while the Tabs tool is on) are applied.

## Affected crates/modules

- `curvyo-laser-core`:
  - `operation_path_options.rs` (new): `LeadKind`, `Overcut`, `PathOptions`, `PathEdit`, the five
    `op_options` keys `lead_in`, `lead_in_mm`, `lead_out`, `lead_out_mm`, `overcut_mm` on `cut`
    and `cut_out` only (L2), written only when one differs from its default; the
    `path_own.<key>` register codec (all five values, damage as L3). `CutLength` gains the lead
    range (one type, two ranges, L2). `OperationOptions.path` (a field beside `cut`, so the
    slice 1 literals of `CutSettings` stay as they are).
  - `cut_entry_records.rs` (new): `Direction`, `OutlineEntry`, the `entry.<key>` register codec
    and its damage, `LeafPath` (an item's own path options and entries resolved per outline).
  - `job_cut_lead.rs` (new): waste side from the nesting depth and the exact `i128` shoelace
    sign, Line and Arc leads (quarter circle by bisection, `+ - * / sqrt` only), L9.
  - `job_cut_path.rs`, `job_plan_cuts.rs`, `job_order.rs`, `job_plan.rs`: a unit takes the cut
    path step when it has tabs, a lead or overcut on a closed outline, or an entry; candidates of
    L7 (one for a set start or a fixed direction; every vertex for an Automatic closed outline
    without tabs); overcut and leads around the pieces (L8); `CutMark` gains the lead, overcut
    and start polylines; `Plan.starts` (where each cut outline is really entered, for the
    Automatic marker, C7) and `Plan.leads_touch`.
  - `job_cut_checks.rs` (new): the lead-touches-a-cut count of L11; `Cause::LeadsTouchCut` and
    its texts in `job_gate_text.rs`; damaged `path_own.`/`entry.` records in `DamagedSettings`.
  - `job_input.rs`, `job_fingerprint.rs`: `LeafInput.path`, `CutOutItem.path`, hashed (L10).
- `curvyo-document-core`: the `path_own.` and `entry.` registers beside `tabs.` (read, written
  in one step, swept and removed with the membership, L4); `format_version` 68 with its
  paragraph; goldens `laser_path_options_v68.curvyo`, `laser_path_options_damaged_v68.curvyo`.
- `curvyo-ui-core`: history names "Own path options", "Back to the operation's", "Move start
  point", "Automatic start", "Direction".
- `curvyo-editor-wasm`: the records into the job input; the Path options edits of the lower part;
  the item part's Path options pill, own fields, Start readout with Automatic, Direction pill;
  start markers (drag, Delete, Automatic only while the Tabs tool is on); Operations view lead,
  overcut and chevron marks from `Plan.marks`; the Checks line; budget cases.
- `frontend`: the Path options group (Lead-in and Lead-out pills with length, Overcut), the item
  part rows, the glyphs; no hint text.

## Tasks

- [ ] 13. `operation_path_options.rs`: ranges, defaults, typed values with the chip texts,
  `KeyOnKind` off Cut and Cut-out, write only when not default, the `path_own.` codec (AC 13,
  21, 22).
- [ ] 14. `cut_entry_records.rs`: `entry.<key>` read and write, damage (not a map, unknown
  field, fraction out of range, unknown direction), `LeafPath` per outline reference (AC 19 to
  22).
- [ ] 15. `job_cut_lead.rs`: waste side by depth and orientation, Line and Arc lead-in and
  lead-out, direction at a corner from the leaving and arriving segment (AC 14).
- [ ] 16. Cut path step: overcut `min(d, L)` stopping at the first tab, none when a tab covers
  the closing point, lead-out after the overcut; tabs win; set start inside a tab moves to the
  tab's end; open Direction one entry; nearest order measured to the entry on the path; passes
  repeat (AC 15, 16, 19, 20, 9).
- [ ] 17. Marks and starts for the Operations view and the markers; fingerprint (AC 19, L10,
  L12).
- [ ] 18. Checks: `LeadsTouchCut` with its texts, damaged records run nothing (AC 18, 22).
- [ ] 19. GRBL goldens `lead_in_line_outer`, `lead_in_arc_outer`, `lead_in_hole`,
  `lead_out_arc`, `overcut_closed`, `overcut_stops_at_tab`, `path_options_passes`,
  `path_options_own_item`, `start_point_set`, `open_direction_reversed`, `tabs_leads_overcut`
  with a read-back of burn lengths and lead geometry; work area 4 b and Frame count leads; every
  existing golden unchanged (AC 14 to 17, 19, 20, 24).
- [ ] 20. Document: the two registers, sweep and unassign, undo, format 68 with the refusal by
  older builds, the old tests' format numbers and the svg-core export goldens' `format_version`
  line, the two goldens (AC 22, 23).
- [ ] 21. Session: records into the job input; Path options edits; item part (pill, own fields,
  Start and Automatic, Direction); start markers with drag, snap, tab stop and Delete; marks;
  Checks line (AC 13, 19 to 21, 23).
- [ ] 22. Budgets: case a with Lead-in Arc, Lead-out Line, Overcut 2 mm within 150 ms; case b
  with start markers within 16 ms (AC 25).
- [ ] 23. Frontend: Path options group, item part rows, no hint text (AC 13, 19 to 21).
- [ ] 24. Docs: design-system rows for the start marker and the lead look; ADR Status notes
  extended where slice 2 adds to them.

## Validation

- Unit tests first in each new laser-core module with the spec's numbers (2 mm below the side,
  1 mm inside the hole, 65 mm and 59 mm burn).
- GRBL goldens read back by the reader of `tabs_goldens.rs` (extended for lead lengths), every
  existing golden re-run unchanged (AC 24).
- Document goldens of format 68: read, saved back byte for byte; a 67 build's refusal.
- Full gate of `CLAUDE.md` §7 once at the end, into a file.

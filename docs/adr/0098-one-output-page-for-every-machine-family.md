# ADR 0098: One output page for every machine family: page size anchored at the export origin, panel tabs per process, shared output views, the Machines tab, machine follow on local edits only, and capability-gated operation settings

**Status:** Proposed 2026-11-28 (architect). The customer answered the same day: Frage 434
MF-MACHINE-FOLLOW **C** (decision 5) and Frage 435 MACHINE-SWITCHES (decision 6). The rest is
architect scope. Accepted with the `story/machine-families` PR. Asked by the UX revision
`docs/design-drafts/2026-11-machine-families/ux-unify.md` (its seams A1 to A5, section 10), written
after the customer tried the three MVPs and asked for all four families to work like the laser.
Amends, by notes on their Status lines: [ADR 0042](0042-page-kinds.md)
§6 (a centre anchor; who writes the size); [ADR 0097](0097-cutting-plotter-embroidery-and-cnc-families.md)
decision 3 (shared *view* types, no shared domain type). Supersedes `specs/0159-cutting-plotter/adrs.md`
PL11 and the page and Set hoop sentences of `specs/0160-embroidery/adrs.md` EM10.

## Context

The MVPs draw each family in its own Document-tab panel (`PlotterPanel.tsx`, `EmbroideryPanel.tsx`,
`CncPanel.tsx`) fed by its own JSON (`plotter_view_json`, `embroidery_view_json`, `cnc_view_json`).
The UX revision deletes those panels and makes the laser's components serve all four families.
Facts checked in the code on 2026-11-28:

- The tab strip has four gate sites in `curvyo-editor-wasm/src/session/panel_tabs.rs` (`tab_allowed`,
  the insertion in `panel_view`, `press_panel_tab`, `follow_tab`). All go through
  `operations_available()`, which is laser-only via `active_manufacturing()`.
- The laser views (`OperationsView`, `OutputGroupView`, `PlayView`) are typed `Serialize` structs,
  pinned by `curvyo-editor-wasm/tests/fixtures/operations_view/`. The family views are ad-hoc
  `json!` values.
- `write_profile` in `curvyo-document-core/src/manufacturing.rs` is the only page-size writer. Its
  `PageCorner { right, bottom }` cannot express a centre.
- The plotter page is the mat's printed field today, with the mat origin at the page origin (PL11).
  The embroidery page is the largest field plus 20 mm (`hoop.rs` `PAGE_MARGIN_MM`, `page_mm`), and
  Set hoop does not resize it.
- Material edits and Fixtures-tab set edits are undo steps with the library journal (ADR 0055 §9,
  Frage 189 B; ADR 0060 §5a; `session/library_journal.rs`, `RecordKind::Machine`). Only the
  Connection group writes without a step. UX 4.4's "no History step, as the Materials tab today"
  therefore does not match the Materials tab.
- `align_fixture_pages` (ADR 0059 §10, ADR 0060 §5a) already makes pages follow a record, in a step
  (`AlignCommit::Step`) or in a reserved commit (`AlignCommit::Reserved`).

## Decision

1. **The page is the machine's work area, and a resize keeps the export origin in place** (A1).
   - **Rule.** Every output page is its machine's work area. When the page size changes, the
     point the family's export counts from keeps its place in the plane. Placements keep their
     global place (ADR 0042 §6 mechanics), and fixtures are recomputed.
   - **Anchor type.** `curvyo-document-core` gains `pub enum PageAnchor { Corner(PageCorner), Centre }`
     with `From<PageCorner>`. It replaces the `anchor: PageCorner` parameter of
     `set_manufacturing_profile`, `_and_sets`, `_filled`, `write_profile` and `anchored_frame`.
     `Centre` shifts the frame origin by half the size change on each axis.
   - **Who chooses the anchor.** One function per family, `resize_anchor(&copy) -> PageAnchor`, in
     the family core's `profile_copy.rs`:

     | Family | Anchor |
     |---|---|
     | Laser, CNC and the matless plotter | the origin corner (as now) |
     | Plotter on a mat profile | top-left: the cut area's corner, which is fixed to the mat origin |
     | Embroidery | `Centre`, for Pick, Reset and Set hoop (the hoop centre is the DST origin) |

   - **Writers.** The writer stays `set_manufacturing_profile*` (ADR 0042 §6). Its callers are
     Pick, Reset, Set mat and Set hoop, plus the follow of decision 5 under A.
   - **Plotter** (`curvyo-cutter-core/src/mat.rs`).
     - The page is the active mat's cut area: 292.1 x 292.1 or 292.1 x 596.9 mm. `MatSize::page_size`
       returns that size.
     - All mat geometry (sheet, field, notch, grid, centre cross, cut area) stays in mat
       coordinates.
     - One constant, `PAGE_ORIGIN_ON_MAT` = (6.35, 6.35) mm, with `page_to_mat` and `mat_to_page`.
       It is the only place that knows the offset. Drawing, snapping and export all use it.
     - `PlotterPage::cut_area()` becomes the page rectangle on every profile, so "outside the cut
       area" means outside the page.
   - **Plotter export.** The plotter job builder maps page to mat coordinates for a mat profile
     before any writer runs. `write_svg` takes the mat's field (`MatSize::field`) as its size, not
     the page; otherwise the header would say 11.5 in and Design Space would shift the design. The
     matless HPGL output is unchanged.
   - **Embroidery** (`curvyo-embroidery-core/src/hoop.rs`). The page is the active hoop's sewing
     field. `PAGE_MARGIN_MM` and `page_mm` are deleted in favour of `HoopSize::page_size`. The
     plan still counts from the page centre, so the DST translation (EM10) is unchanged. Set hoop
     becomes one step "Set hoop" through `set_manufacturing_profile` with `Centre`.
   - **CNC.** No change. The stock label is drawing only.
   - **Goldens.** The writer goldens in the family crates keep their bytes, because the writers
     are untouched. In `curvyo-editor-wasm` there is one new test per family: the same item at the
     same mat or hoop place exports bytes equal to the existing golden. Test literals in page
     coordinates move: plotter −6.35 mm; embroidery by the old page centre minus the new one (−20 mm for 150 x 250, −45 / −95 mm for 100 x 100).
   - **Format.** No new number. 69 and 70 stay reserved and are re-cut (ADR 0097 §6, Frage 431 A).
     There is no migration: a branch file keeps its old page size until Set mat, Set hoop or Reset.

2. **Panel tabs per process** (A2).
   - `curvyo-ui-core`: `PanelTab` and `PanelContent` gain `Tools` and `Machines`.
     `PanelTabsRecord::new` maps them to `"tools"` and `"machines"`.
   - **One list.** `curvyo_ui_core::panel_tabs::output_tabs(process: &str) -> &'static [PanelTab]`,
     in strip order:

     | Process | Tabs |
     |---|---|
     | `curvyo.laser` | Operations, Materials, Fixtures, Machines |
     | `curvyo.cutter`, `curvyo.embroidery` | Operations, Fixtures, Machines |
     | `curvyo.cnc` | Operations, Fixtures, Tools, Machines |
     | any other | none |

   - **One session read.** `Session::output_tabs()` is the active output page's list while its
     process is enabled, else empty. All four gate sites use it: `tab_allowed` takes the list
     instead of `laser: bool`, `panel_view` inserts the list after Document, and `press_panel_tab`
     and `follow_tab` refuse a tab that is not in it. `operations_available()` stops being a tab
     gate. `active_manufacturing()` stays laser-only, so laser settings, materials and tabs code
     never runs on a family page.
   - **Options.**
     - **A. A match in ui-core beside the tab rules (chosen).** ui-core already registers the
       manifests (`extensions.rs`) and holds the tab rules and their tests.
     - **B. Each manifest declares its tab ids.** Rejected: the family cores cannot see `PanelTab`
       (ui-core depends on them), so the ids would be a second string spelling. Manifest `tabs`
       already means extension-drawn tabs (ADR 0032 §2).
     - **C. A match in editor-wasm.** Rejected: it works, but it is tested only through a
       `Session`.

3. **One view shape for all families** (A3; amends ADR 0097 §3 for views only).
   - **Shared types.** The types the laser components read become every family's output-page
     view: `OperationsView`, `RowView`, `KindButtonView`, `OutputGroupView` and `PlayView`. They
     move into a types-only module `curvyo-editor-wasm/src/session/output_view.rs`. The laser
     builders keep building them. Each family's `view_json.rs` builds the same types, and its own
     JSON shrinks to its Fixtures section (mat, hoop or stock) and, for CNC, the Tools tab. Domain
     operation, job and timeline types stay per family.
   - **New fields and variants.**
     - `KindView` gains `Draw`, `RunningStitch`, `TatamiFill`, `Satin`, `ProfileOutside` and
       `ProfileInside`. The plotter's Cut is `Cut`. `KindView` is glyph and word, not the stored
       kind, so a CNC Profile row takes its view from its Side.
     - Select-bar button ids are `KindView` names. `press_operation_kind` dispatches by process,
       and each family parses its own ids (`profile_outside` becomes Profile with Side Outside).
       Stored kinds do not change.
     - Laser-only fields are `None` or empty on family pages: `pick`, `tabs_tool`, `tab_chip`,
       `settings`, `item_tabs`, `item_path`, `source`, `start` and `frame`. `cuts_last` becomes
       `Option<bool>`, with `None` meaning no switch (embroidery, CNC).
     - New on `RowView`: `thread: Option<String>` (hex) and `body: Option<FamilyBodyView>`, a
       tagged enum `Plotter | Embroidery | Cnc`. It holds the unfolded row's fields, which the
       MVP views already compute. It is data, not a form schema.
     - `OutputGroupView`: `total.time` becomes `Option` (Design Space has none). `export` carries
       the file type and extension per format.
     - `PlayView` gains `step` (the fraction of one arrow press; one stitch on embroidery) and
       `speed: Option<SpeedView>` (embroidery 1×, 4×, 16×). The readout is the family's text. The
       play commands dispatch by process, and `set_cnc_play` and its siblings are deleted.
   - **Dispatch** in `session/output_page.rs` (new): `enum ActiveOutput { Laser(ActiveOperations),
     Plotter(PlotterPage), Embroidery(EmbroideryPage), Cnc(CncPage) }` and `Session::active_output()`.
     Every shared entry point is one four-arm `match`: the three views, kind press, row commands,
     unfold, export, send, Pick, Reset, Save as, and the Machine block view.
   - **Options.**
     - **A. Enum and exhaustive match (chosen).** A fifth family makes the compiler list every
       missing arm.
     - **B. A trait `OutputFamily` with four implementations.** `CLAUDE.md` §5 now allows it but
       does not require it. Every method would need `&Session` or `&mut Session`, its only caller
       would be the match it replaces, and it is the `Family` abstraction ADR 0097 §3 refused.
       Revisit only for a family from outside the workspace, with its own ADR.
     - **C. TypeScript adapters from the family JSON.** Rejected: the shape would exist twice, the
       customer reviews the `.rs`, and the shared fixtures would not pin it.
   - **Fixtures and front end.** `tests/fixtures/operations_view/` gains one fixture per family.
     The TypeScript decoders and their tests read them. The front end adds the kinds to its union,
     `KIND_WORDS` and `OperationGlyph`. No family copy of a component is written.

4. **Machines tab and Machine block** (A4, A5).
   - **The view.** `Session::machines_view()` and `machines_view_json` give one section per enabled
     family (Laser, Plotter, Embroidery, CNC, with the active page's family first). The rows come
     from the four libraries (`MachineProfiles`, `PlotterLibrary`, `EmbroideryState`, `CncState`),
     and each body is a tagged enum per family.
   - **Commands.** They are keyed by record id (ids are unique across families), and the session
     finds the family by which library holds the id:
     - `set_machine_value(id, field, text, now_ms) -> Result<Option<String>, String>`;
     - `duplicate_machine`, `rename_machine`, `delete_machine`;
     - `set_machine_connection` (unchanged).

     The five built-in ids are distinct constants (`generic-laser`, `cricut-maker-3`,
     `generic-hpgl-cutter`, `w6-eu7`, `generic-grbl-router`), and a test asserts that they are.
     The laser's `duplicate_`, `rename_`, `delete_machine_profile` and `set_profile_work_area`
     become these.
   - **Undo** (Proposal; corrects UX 4.4). Every field edit, rename, duplicate and delete in the tab
     is one undo step with the library journal (`RecordKind::Machine`, the ADR 0055 §9 limits),
     as Materials and Fixture sets already are. A step that changes no page writes a root
     marker, as `commit_material_edit` does in `session/materials/step.rs` (a machine variant
     of `MaterialStep`, or that marker generalised). For the field values this keeps the accepted
     rule that a machine edit can be undone. For the profile list (rename, delete, work area),
     which writes no step today, it is a change. The Connection group stays a plain write.
   - **Removed from the page in the same PR** (as dead code): the copy field setters on the page
     (`set_machine_name`, `_origin`, `_max_feed`, `_max_power`, `_default_power`, `_default_speed`,
     `_laser_mode`, `set_machine_motorised_z`, `set_machine_air_assist`, `update_machine_profile`,
     `set_plotter_default`, `set_embroidery_speed`, `set_cnc_machine_value`) and the matching
     `ProfileChange::Edit*`, `MotorisedZ` and `AirAssist` variants and labels.
   - **Air assist and Motorised Z** leave the page as page switches (`0111` criterion 22, `0029`
     criterion 25). They become machine capabilities with per-operation settings (decision 6,
     Frage 435).

5. **Machine follow: the page keeps its copy, and only a local edit offers to update it**
   (Frage 434 C; amends ADR 0039's Differs rules by these).
   - **Rules.**
     - **R1. The copy is the page's machine.** Output, checks and time always come from the
       page's own copy (`mfg_profile`). The library never writes a page by itself: not at open,
       not at a library reload, not at a library or workspace sync.
     - **R2. Not in your machines.** Say a copy's id names no record in the local library: a
       workspace from another maker, a deleted machine, or a record skipped as damaged or newer.
       The page works from its copy and shows "Not in your machines" with Save as machine. Saving
       is the only way a copy enters the library, never by itself.
     - **R3. Differs.** Say a copy's id names a local record whose values differ (the record
       changed by sync, or the file was saved elsewhere). The page shows "Differs from profile"
       with Reset from profile: one step, on that page only. There is no question.
     - **R4. Ask on a local edit only.** After a Machines-tab edit of record X (one journal step,
       decision 4), the session counts the pages of the open project whose copy id is X and whose
       copy equalled X before the edit. If there are any, the machine's unfolded body in the
       Machines tab shows an inline confirm, never a popup: "Update <n> pages" or "Keep".
       - Update is its own undo step, "Update pages from <machine>", after the edit's step and
         separate from it. It writes through the family writers. An undo of it returns the pages;
         a second undo returns the record.
       - Keep leaves the pages with "Differs".
       - The offer is session state with no step. It ends with the next edit, a tab or page
         switch, or an undo of the edit.
       - Pages that already differed before the edit are not offered (Proposal): they keep their
         R3 state and their own Reset.
     - **R5. Sync stays clean.** Sync carries record texts and project documents unchanged.
       Neither writes the other: no record is made from a copy, and no copy is made from a record.
       Another participant's page edit is a document change like any other.
     - **R6. Other projects are not touched.** Only the open project's pages are offered. Another
       project of the workspace meets R3 when it is opened.
   - **Built pieces.**
     - Each family core gets `copy_of(&record)` and `differs(&copy, &record)` (laser: `ProfileCopy::of`
       and `differs`). On mat and hoop families, `differs` compares the values but not the page
       size, which follows the active fixture.
     - One session writer per family, `write_<family>_copy(page, copy, change)`, resizes with the
       anchor of decision 1 and writes the `0150` fills, as laser's `write_active_copy` does.
     - Reset is that writer on one page. Update is `follow_machine(record_id, pages)` calling it
       for every offered page in one batch.
     - A page whose resize `check_layout` refuses (`OutOfRange`) is left out and keeps "Differs".
       The step goes on.
   - **Fixture sets: Frage 436 answered A (customer, 2026-11-28).** The laser's fixture-set alignment
     (`align_fixture_pages` at open, reload and page activation; ADR 0059 §10, ADR 0060 §5a,
     accepted) silently makes the page's sets follow the record's. R1 covers the machine values
     and the page size, not the sets. Under default A the sets keep following silently. If the
     answer is otherwise, the sets get the R3 and R4 treatment in a note here.

6. **Machine capabilities and per-operation settings** (Frage 435).
   - **Record.** The laser record's `motorised_z` and `air_assist` keep their keys and are now
     capabilities: "the machine has a motorised Z axis" and "has air assist". They are edited in
     the Machines tab and copied to the page as before. The record format is unchanged.
   - **Operation.** Every laser operation has `z_offset` (exists, ADR 0056 §6) and a new `air`
     option in `op_options` (`curvyo-laser-core/src/operation_options.rs`).
     - `air` is on by default, and an absent key reads as on. An old project's job therefore
       writes the same M8 and M9 as before.
     - **GRBL, with the capability** (`job_write_grbl.rs`, from per-run `air` flags in the move
       list, `job_moves.rs`):
       - `M8` is written before the first burn move of an operation whose `air` is on while the
         air is off.
       - `M9` is written before the first move of an operation whose `air` is off while the air
         is on.
       - At the end of the job, `M9` is written only while the air is on.
       - Consecutive operations with air on write no extra codes. A travel between them keeps
         the air on.
       - Fixture jobs run with air on (Proposal; they have no operation setting).
     - **Without the capability:** no `M8` and no `M9` are written, whatever the stored values.
     - **Goldens:** `air_assist.gcode` keeps its bytes (every operation on, so one `M8` and one
       `M9`). The others have no air and stay as they are. New goldens:
       - `air_per_operation.gcode`: on, off, on, which writes M8, M9, M8, M9;
       - `air_stored_without_capability.gcode`: no M8 or M9.

       The tester's GRBL parser checks that air is on exactly over the runs marked on.
   - **Gating.** A setting whose capability the page's copy lacks is not in the view: there is no
     field, no chip and no line-3 text. The shared `RowView` and the family `body` list only
     settings the copy supports. Other families declare capabilities the same way in their record
     body when they need one; none needs one today (CNC Z is inherent).
   - **Stored values on a machine without the capability** (Proposal). They are kept and
     inactive. A pick, Reset or Update to such a machine never deletes or rewrites them; the job
     ignores them; the page writes nothing for them; and no check warns, because the setting is
     hidden. This replaces the "Z offset with no motorised Z axis" warn of `0029` criteria 5 and
     19. Back on a capable machine, the values work again.
   - **Format.** `air` is a stored `op_options` shape that an older build would ignore, switching
     the air on for the whole job: wrong output. By ADR 0097 §6 that is one project
     `format_version` bump, `CURRENT_FORMAT_VERSION` 71 to 72, renumbered to the next free at
     merge. An absent key reads as on, so there is no migration, and files of format 71 and
     earlier export the same bytes.

## Central files

- **Rust core:**
  - `curvyo-document-core/src/manufacturing.rs` (`PageAnchor`, `anchored_frame`);
  - `curvyo-cutter-core/src/mat.rs` and `write_svg.rs`;
  - `curvyo-embroidery-core/src/hoop.rs`;
  - `profile_copy.rs` in the cutter, embroidery and CNC cores (`resize_anchor`, `copy_of`,
    `differs`);
  - `curvyo-ui-core/src/panel_tabs.rs` and `panel_content.rs`.
  - `curvyo-laser-core/src/operation_options.rs`, `job_build.rs`, `job_gate.rs`,
    `job_write_grbl.rs` (per-operation `air`, capability gating);
  - `curvyo-document-core/src/document.rs` (`CURRENT_FORMAT_VERSION`).
- **Session** (`curvyo-editor-wasm/src/session/`):
  - `panel_tabs.rs`;
  - `output_view.rs` and `output_page.rs` (new);
  - `operations/view_json.rs`, `operations/kind_press.rs`, `output/view_json.rs`, `output/play.rs`;
  - `{plotter,embroidery,cnc}/{commands,view_json,job}.rs`;
  - `manufacturing/machine.rs` and `machine_view.rs` (R1 to R6, the Update offer);
  - `operations/options.rs`, `operations/settings_view.rs` (gating);
  - `library_journal.rs`.
- **Fixtures:** `curvyo-editor-wasm/tests/fixtures/operations_view/` and `output_view/` (one per family); `plotter_page_v69.curvyo` and `embroidery_page_v70.curvyo` re-cut for the new page sizes.
- **Front end:**
  - `frontend/src/lib/operationsView.ts`, `lib/kindButtons.ts`, `lib/operationRows.ts`;
  - `components/panel/OperationGlyphs.tsx`;
  - `components/workbench/ManifestTab.tsx` (three early returns deleted), together with
    `PlotterPanel.tsx`, `EmbroideryPanel.tsx` and `CncPanel.tsx`.

## Consequences

- The view types and their fixtures are the one contract for four families. A laser-only field
  added later must be optional.
- `PageAnchor` is a public API change in document-core, with no stored-shape change.
- The plotter's rulers count from the cut area (UX U1). The 6.35 mm mat offset lives in one
  constant.
- Machine edits move from page steps to record steps. Undo still reaches them within the session,
  but not in a later session (ADR 0055 §9 (a)).
- One project format bump for the per-operation `air` (next free at merge). No record format
  change.
- No page ever changes without a step the maker took on this computer, so a synced workspace
  stays as its author left it.
- New crates: none. New dependencies: none.

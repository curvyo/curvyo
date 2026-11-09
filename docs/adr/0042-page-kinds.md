# ADR 0042: Page kinds, output pages in the plane, and the workbench of the active page

**Status:** Accepted 2026-11-05 (customer accepted PR #279; document model and file format). The design is the
customer's (concept A of `specs/0063-manufacturing-workbenches/concept-output-pages.md`,
accepted 2026-11-05, Frage 151 A, 155 A); the storage choices below are the architect's
**Proposal**. Amends, by notes on their Status lines: [ADR 0024](0024-spatial-pages.md) §1, §3
(a page kind); [ADR 0032](0032-workbench-model-and-registry.md) §1 to §5, §9 (output page listed
and in the plane, workbench derived, no switcher, no Add manufacturing);
[ADR 0033](0033-manufacturing-data-in-the-project-file.md) §1, §5 (lifecycle bound to the page);
[ADR 0039](0039-machine-profile-copy-in-the-project-file.md) §4 (resize about the origin corner);
[ADR 0026](0026-parametric-sketch-nodes-and-workbench.md) "Page ownership revision" (only sketch
pages own a sketch). Spec: `specs/0098-page-kinds`.
Note 2026-11-06: §1, §3 and §4 amended by [ADR 0043](0043-sketch-pages-content-by-ownership.md) (proposed): Convert to paths on a sketch page writes `kind` from `"sketch"` to `"drawing"` (the only second write); a sketch page's content is its sketch by ownership.

Note 2026-11-09: Consequences ("moving a placement off its laser page has to be decided there", "group clones (`0100`)") answered by [ADR 0045](0045-clones-of-groups-texts-images-and-movable-placed-pages.md) (proposed, `needs-customer`, spec `0100`): a placement stays on its own laser page by ownership; clones of groups are decided there.

## Context

Today a page is a drawing frame (ADR 0024); a drawing page may own one sketch (ADR 0026); a
laser output page is owned by a manufacturing node, hidden from the page list and the plane, and
shown only in a workbench chosen in a switcher (ADR 0032 §5, WB-PAGE-VIS A). The customer
reversed that on 2026-11-05: every page has a fixed kind (Drawing, Sketch, Laser) shown next to its
name; all kinds sit in one list and one plane; the tools follow the active page; the switcher
goes; a laser page is a full editing context whose size is the machine's work area. Three
questions: where the kind is stored, whether the page or the manufacturing is primary, and how a
format-36 file opens.

### Options: where the kind lives

- **A. One register `page.<id>.kind` for pages without an owner; a page with `page.<id>.owner`
  is an output page whose process is its manufacturing's `mfg_process` (chosen).** No value is
  stored twice; format-36 output pages need no write to become output pages.
- **B. `kind` on every page, the process id on output pages.** A second copy of `mfg_process`
  and a disagreement state with no meaning. Rejected.
- **C. Derived from content (a page with a sketch claim is a sketch page).** A new sketch page
  has no claim until its first edit (ADR 0026), and deleting the sketch would change the kind.
  The kind must be fixed (criterion 5). Rejected.

### Options: page or manufacturing first

- **A. Keep the manufacturing node and both links unchanged; the page is primary to the maker
  and the node is its machine data, created and deleted with it (chosen).** No change to the
  stored shape of ADR 0033, and placements, fixtures and later operations stay ordered, undoable
  tree nodes.
- **B. Machine data as page registers (`page.<id>.mfg_*`), no node.** Placements and fixtures
  are tree objects, which root registers cannot hold (ADR 0032 option C lost for the same
  reason), and every format-36 laser file would need rewriting. Rejected.
- **C. Own items (made on a laser page with the vector tools) as children of the manufacturing.**
  Every vector tool, Layers, snapping and style path would have to learn the subtree, and a move
  off the page would reparent. Frage 155 A says they belong by position. Rejected.

### Options: opening a format-36 file

- **A. One migration commit at open, outside the undo history (chosen).**
- **B. Refuse format 36.** Allowed (`0059` criterion 1) but loses the customer's sketch files.
- **C. Derive on read, write nothing.** A moved sketch on a page that does not exist yet would
  need a virtual page every consumer knows about. Rejected.

## Decision

1. **Kind register.** `page.<id>.kind`, a string, `"drawing"` or `"sketch"`, written once in the
   commit that creates the page (Add page, new-document setup, SVG import pages) and never again.
   Output pages get no `kind`: `page.<id>.owner` (ADR 0032 §5) makes them output pages.
2. **Reading** (`curvyo-document-core`, one pure function over the page's registers):
   `owner` names a live manufacturing → `Output { manufacturing, process }`; else `kind` absent or
   `"drawing"` → Drawing; `"sketch"` → Sketch; another string → `Unknown(text)`, kept, listed as
   "Unknown kind", saved back unchanged (R-MFG-010); not a string → Drawing, shown, never
   rewritten (ADR 0033 §3 rule). A `kind` register on an owned page is ignored. `Page` gains
   `kind: PageKind`. The editor shows Output of a disabled process as "<word> (off)" and of an
   unknown process as "Unknown kind".
3. **Fixed kind.** No command writes `kind` after creation, and no command adds or removes
   `owner` on an existing page. `set_page_size` is refused for Output and Unknown pages
   (`PageError::FixedSize`); moving their frame in the plane is allowed (ADR 0024 §5).
4. **The output page is listed and in the plane** (replaces ADR 0032 §5's "not in the drawing
   page list, not on the Vector canvas, no spatial membership"). `pages()` returns every live
   page of every kind in list order. An output page's content is (a) its manufacturing's
   placements and fixtures, page-local as before (global = frame origin + local), and (b) the
   objects of the drawing tree that are its members by ADR 0024 §1, its own items. A frame move
   carries both: (a) by construction, (b) by the ADR 0024 §5 move. The ADR 0032 skip predicate
   stays for Layers, Select all, repairs and membership; it no longer hides the subtree from
   drawing: placements and fixtures are drawn on their page in every workbench, placements are
   hit (select, delete), fixtures are not hit. Placement targets stay non-output pages. Fixtures draw with
   their page's grid, below the drawing; the origin marker and the placement box draw above it
   (lead, 2026-11-05, Proposal: no render-core layer API needed, the marker stays visible).
5. **The page owns the manufacturing's lifetime** (amends ADR 0032 §4, ADR 0033 §1, §5).
   `add_output_page(process, profile map, work area)` writes the page registers (name "Laser N",
   order after the last page, frame by the Add page rule of `0059`), the node, `mfg_*` and
   `owner` in one commit "Add laser page", with no placement. `delete_page` on an output page
   marks the page deleted and deletes the node in one commit; undo restores both.
   `add_manufacturing`, `delete_manufacturing` and `set_work_area` are removed. The page name is
   the only name shown; the node's `name` is no longer written and is left as is in old files. A
   manufacturing whose page is missing is listed nowhere and kept (ADR 0033 §4).
6. **Size from the machine only** (amends ADR 0032 §5 "editing the work area resizes the page"
   and ADR 0039 §4). Only `set_manufacturing_profile` (pick and Reset from profile) writes an
   output page's size. The copy's origin corner keeps its place in the plane: the frame origin
   moves by the size change on the axes the corner sits on (bottom-left: `y -= Δh`), each
   placement's local translation compensates in the same commit so it keeps its global place,
   fixtures are recomputed from the new work area, and own items are not written. The work area
   is typed only in a user profile record (ADR 0038, no record format change).
7. **The workbench is the active page's** (amends ADR 0032 §1 to §3). The session's `Workbench`
   is derived on read from the active page's kind: Drawing and Unknown → Vector; Sketch →
   Sketch; Output of an enabled process → `Manufacturing(node)`; Output of a disabled process →
   Vector with the notice block. No command sets it; the switcher and the Add manufacturing
   choices go. The registry list of ADR 0032 §2 becomes the **page kind list** for the Kind
   switch: Drawing, Sketch, then one entry per enabled extension manifest (glyph, word). The
   front-end `WorkbenchEntry` registry is keyed by kind; its dummy-manifest test moves to the Kind
   switch (`0098` criterion 18 a). Activating a page cancels pending gestures as ADR 0026's page
   switch does and is not a history step. In a Vector workbench on an Unknown or disabled output
   page, nothing writes the manufacturing (ADR 0033 §4).
8. **Only sketch pages own a sketch** (amends ADR 0026). `create_page_sketch` refuses a page that
   is not a Sketch page. The claim registers (`page.<id>.sketch_owner`, the sketch's
   `sketch_page`) are unchanged; a claim on a page of another kind reads as no owner and is never
   repaired on read. A new sketch page holds no node until its first durable edit (ADR 0026's
   lazy creation), which the maker sees as the empty sketch of `0098` criterion 3.
9. **Migration from format 36**, one commit with the new reserved label `migrate_format`
   (ADR 0014: not an undo step, not listed in History), written when the file is opened, before
   anything reads membership:
   - each output page (all sit at (0, 0) today) gets `order` after the drawing pages and a frame
     origin by the Add page rule, in tree order of their manufacturings; nothing else moves;
   - each drawing page whose `page_sketch` is live gets a sketch page with the id
     `<page id>-sketch` (deterministic, so two peers write the same page registers), named
     "<page name> sketch", of the same size, at the Add page position; the sketch moves by the
     origin difference through the clone-aware transform with every downstream clone held back
     (ADR 0016 §2, `Hold::Whole`), so no existing clone moves; the claim moves to the new page;
     a linked clone of the sketch with `own_placement` on is created in the sketch's old parent
     and z-position with the old placement. Every resolved outline in the drawing is unchanged.
   A file that needs none of this gets no commit. The session treats the result as unmodified
   (written at the next save, criterion 19).
10. **`format_version` 37** (next free at merge; main is at 36). A version-36 build would hide
    output pages again and let a drawing page own a sketch, recreating what decision 9 removed,
    so it must refuse the file. Damage: decision 2's not-a-string case reads as Drawing and is not
    refused. Goldens in `curvyo-document-core/tests/fixtures/`: `page_kinds_v37.curvyo`
    (criterion 20), `page_kind_damaged_v37.curvyo`, `page_kind_unknown_v37.curvyo` (a page of kind
    `"template"`, saved back unchanged), `page_kinds_migration_v36.curvyo` (an output page with a
    placement and a hidden fixture, a drawing page with other content, its sketch and an external
    clone of it; opens to decision 9's result with equal resolved outlines). `laser_workbench_v36`
    and `machine_profile_v36` stay and open through the migration.
11. **Crates.** No new crate, no new dependency. `document-core`: `PageKind`, the kind read,
    `add_page(kind)`, `add_output_page`, `delete_page` for output pages, the size refusal, the
    corner-anchored resize, the migration module (`page_kind_migration.rs`). `editor-wasm`: the
    derived workbench, the kind list, `Ctrl+Page Up/Down`; removed `set_workbench` and the
    manufacturing add and delete exports. `laser-core`: unchanged except the work-area edit in the
    profile row (ADR 0038 fields). Front end: the registry keyed by kind.

## Consequences

- The kind is fixed and cheap to read (one register, or the owner link), and adding CNC later is
  one more process id, with no new kind value.
- Output pages now mix two coordinate frames: placements and fixtures page-local under the node,
  own items global by position. Moving a placement off its laser page (`0100`) has to be decided
  there; a placement belongs to its page by ownership, not by position.
- Every consumer of `pages()` that assumed drawing pages only (size presets, fit, export, print,
  page membership, templates) must be audited; owned pages drew nothing in them before.
- An object over a drawing page and a laser page that overlap belongs to both (ADR 0024 overlap
  rule); a placement of that drawing page then also shows it. Visible and accepted.
- The migration commit syncs at once when the document is in a shared room; two devices opening
  the same format-36 workspace before they sync each create a clone (the sketch page itself
  converges by its id). Recorded in `docs/technical-debt.md`; this development build has no
  compatibility promise.
- Not decided here: own Layers tree of an output page, the edit-context rule and group clones
  (`0100`), operations (`0101`), fixtures as objects (`0102`).

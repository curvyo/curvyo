# Feature specs

The list of all features comes first, then the convention for writing them.
State of this list: 2026-10-12, synced with the merged pull requests up to #112
(the last story merged is #105, Split, Break apart and Cut). The customer
renumbered every folder on 2026-10-09, see "Numbering" below.

Spec-driven development, adapted from
[spec-driven-dev-kit](https://github.com/trojava/spec-driven-dev-kit) for
this project's roles and file layout (see `CLAUDE.md` §2, §4).

## Feature list

One numbered list. The number is the folder's position in this list:
`specs/<NNNN-slug>/`. The list is grouped by state, and the numbers run
without gaps from top to bottom: first what is built (in the order it was
merged), then what is specified and waiting (in the order the lead starts it),
then drafts, then MVP slices nobody has specified yet. Rows without a link
have no folder yet; their number is reserved. **From 0030 on, numbers are
assigned in the order the folders are created** and a row sits in the group of
its state, so inside a group the numbers can be out of order (the order the
lead starts the waiting ones is in "Proposed build order" below).

Columns: **No.** = folder number. **MVP slice** = the label the same feature
had in the old MVP sequence (slices 1 to 16), `-` for features that came from
customer feedback and were never an MVP slice. Spec numbers and slice labels
agree for 0001 to 0007 only; see "Spec number versus slice label" below.

### Done

| No. | Slug | What it delivers | Status | Priority | Requirements | MVP slice |
|---|---|---|---|---|---|---|
| [0001](0001-project-file-foundation/) | `project-file-foundation` | Create, open and save a local `.curvyo` project with an empty canvas; same result after close and reopen. | Done (#3) | Must | R-SYS-001, R-SYS-002, R-SYS-007 | 1 |
| [0002](0002-path-node-editing/) | `path-node-editing` | Draw and edit a Bézier path with a pen tool: add, move and delete nodes, drag handles; same node, handle and segment model as Inkscape. | Done (#7) | Must | R-EDIT-001 | 2 |
| [0003](0003-primitive-shapes/) | `primitive-shapes` | Rectangle (with corner radius), circle/ellipse and polygon/star tools; "object to path" turns any of them into an editable path. | Done (#10) | Must | R-EDIT-002, R-EDIT-004 | 3 |
| [0004](0004-canvas-navigation-and-selection/) | `canvas-navigation-and-selection` | Pan and zoom the canvas (zoom toward the cursor), stable on window resize; a general Select tool to click, move and delete any object. | Done (#25) | Must | R-EDIT-010, R-EDIT-011 | 4 |
| [0005](0005-object-transform/) | `object-transform` | Move, scale and rotate any path or primitive with on-canvas handles on the Select tool's bounding box; "Scale stroke width" and "Scale corner radius" switches (both off by default). | Done (#29) | Must | R-EDIT-012 | 5 |
| [0006](0006-path-merge-split-and-node-types/) | `path-merge-split-and-node-types` | Join two path endpoints into one node, split a path at a node; a third node type (Asymmetric) next to Corner and Symmetric. | Done (#26) | Must | R-EDIT-014, R-EDIT-001 | 6 |
| [0007](0007-stroke-and-fill-styling/) | `stroke-and-fill-styling` | Stroke width, dash, join, cap and colour, and fill, on any path or primitive. The gradient fill and the popover panel built here are replaced by 0017 (customer decision after trying it, 2026-10-08). | Done (#54, #55, #58, #59) | Must | R-EDIT-005, R-EDIT-006 | 7 |
| [0008](0008-object-transform-refinements/) | `object-transform-refinements` | Centre move, rotate corners, pivots, typed entry, 22.5° snap, skew. | Done (#35) | Must | R-EDIT-012 | - |
| [0009](0009-unified-object-editing/) | `unified-object-editing` | One Select tool with parameter handles for every object; blue/black preview; shape tools only create. | Done (#38, fix #39, docs #40) | Must | R-EDIT-002, R-EDIT-011, R-EDIT-012 | - |
| [0010](0010-edit-interaction-polish/) | `edit-interaction-polish` | Polygon/star angle, Escape cascade, keyboard shortcuts, dashed selection box, typed skew and move, Shift axis lock, Ctrl copy of a move. | Done (#42, #45, #46) | Must | R-EDIT-012, R-EDIT-014 | - |
| [0011](0011-shape-creation-from-center/) | `shape-creation-from-center` | Shift while drawing a shape creates it from the centre. | Done (#47) | Must | R-EDIT-002 | - |
| [0012](0012-polygon-star-box-refit/) | `polygon-star-box-refit` | The selection box of a polygon or star follows the shape's shown orientation. | Done (#49) | Should | R-EDIT-012 | - |
| [0013](0013-rectangle-corner-radii/) | `rectangle-corner-radii` | A separate radius for each corner of a rectangle. | Done (#53) | Should | R-EDIT-002 | - |
| [0014](0014-advanced-selection/) | `advanced-selection` | Bigger hit area, candidate disambiguation, marquee and lasso select. | Done (#61) | Must | R-EDIT-010, R-EDIT-011 | - |
| [0015](0015-document-size-and-rulers/) | `document-size-and-rulers` | Rulers in mm along the top and left, document resize with the content staying centred, fit the document to the drawing, drawing outside the document edge. | Done (#67, #72) | Must | none yet | - |
| [0016](0016-boolean-operations/) | `boolean-operations` | Union, difference, intersection, exclusion and reverse difference on closed paths, compound-path results; their own toolbox card (amendment of 2026-10-10, shipped in #79). The hover preview is not part of it (see "Specified but not built"). | Done (#69, #73; amendment #79) | Must | R-EDIT-003 | 9 |
| [0017](0017-style-panel-rework/) | `style-panel-rework` | Empty panel when nothing is selected, controls hidden instead of disabled, 8-digit RGBA hex, inline colour picker, eyedropper, custom dash text line, GIMP-style value fields, gradients removed. Replaces parts of 0007. | Done (#78) | Must | R-EDIT-005, R-EDIT-006 | - |
| [0018](0018-stroke-markers/) | `stroke-markers` | Arrow or dot at the start, the end, N places along and on every node of a path. | Done (#78) | Should | R-EDIT-005, R-EDIT-016 | - |
| [0030](0030-document-size-presets/) | `document-size-presets` | Paper sizes A0 to A6 and slide formats 16:9, 16:10, 4:3 as inline buttons in the Document section, a portrait/landscape switch, the list from one validated data file. | Done (#78) | Should | R-EDIT-019 | - |
| [0031](0031-segment-drag-bending/) | `segment-drag-bending` | In the Node tool, drag a line or curve segment to bend it; node types honoured, blue/black preview, one commit, Escape cancels. | Done (#79) | Should | R-EDIT-020, R-EDIT-001 | - |
| [0034](0034-pen-path-extension/) | `pen-path-extension` | With the Pen: continue an open path from either end, connect two paths, close with a sharp or smooth closing node; a Close path command in the Node bar. | Done (#79) | Should | R-EDIT-022 | - |
| [0035](0035-combine-and-break-apart/) | `combine-and-break-apart` | Combine closed shapes into one compound path (inner shapes become holes); the command built here as Break apart is called Split since `0048`. Brought the second rail column. Criterion 15 (group box after the command) has no test, see the spec. | Done (#79) | Should | R-EDIT-023 | - |
| [0019](0019-multi-object-transform/) | `multi-object-transform` | One group box with the same handles as a single object for a selection of several objects: move, scale, rotate, skew; stretch in one direction for any selection, converting shapes that cannot represent it. | Done (#80) | Should | R-EDIT-012 | - |
| [0043](0043-properties-tabs/) | `properties-tabs` | Tabs on the Properties panel (Document, Style, History) with auto-switch rules and shortcuts. Its horizontal strip was replaced by `0051`. | Done (#84; History tab #95) | Should | R-EDIT-025 | - |
| [0045](0045-document-formats-library/) | `document-formats-library` | User-extendable format list: own formats and groups, favourites, groups that can be switched off, an inline full list, import and export of a formats file, a per-user file over the built-in TOML. | Done (#84) | Should | R-EDIT-027 | - |
| [0044](0044-editing-quick-wins/) | `editing-quick-wins` | Ctrl+A selects all objects of the context; arrow keys nudge by 1 mm, Shift+Arrow by 10 mm. | Done (#87) | Should | R-EDIT-026 | - |
| [0040](0040-document-background/) | `document-background` | A background fill for the document (colour with alpha or none as a checkerboard), a Background block in the Document section, the eyedropper picks the background. `format_version` 10. | Done (#90) | Should | R-EDIT-024 | - |
| [0020](0020-undo-redo/) | `undo-redo` | Per-user undo and redo, one interaction = one step, a document history with step ids, authors and operation names in a History tab, wipe of the history. | Done (#95, fix #101) | Must | R-EDIT-008, R-HIST-001, R-HIST-004 | 8 |
| [0047](0047-transform-polish/) | `transform-polish` | Silent conversion to paths on skew and stretch; click and double-click on every slider; the typed move measures from a visible reference box with nine clickable reference points. | Done (#97) | Should | R-EDIT-029, R-EDIT-012, R-EDIT-015 | - |
| [0023](0023-groups/) | `groups` | Group and ungroup, nested groups, enter a group to edit inside it, a group transforms as one. The grouping half of the old slice `layers-and-grouping`. `format_version` 11. | Done (#102) | Must | R-EDIT-009 | 10 |
| [0041](0041-object-history/) | `object-history` | The history of one selected object, object undo and redo (Ctrl+U, Ctrl+Shift+U), copies inherit their original's history, wipe of one object's history. **Open:** criterion 18 (timeline of a group) and criterion 25 for path copies; see the spec. | Done except criteria 18 and 25 (paths) (#103) | Should | R-HIST-002, R-HIST-004 | - |
| [0051](0051-sidebar-and-no-menu/) | `sidebar-and-no-menu` | A vertical strip of icon tabs on the panel's inner edge with the collapse control, a File tab (New, Open, Save, Save as, Close, Quit), an inline unsaved-changes check, no native menu bar on Linux and Windows. | Done (#104) | Should | R-EDIT-032, R-SYS-010 | - |
| [0048](0048-split-compound-path/) | `split-compound-path` | Split separates the unconnected parts of a compound path, holes stay with their part; Break apart gives every outline its own object. | Done (#105) | Should | R-EDIT-023 | - |
| [0036](0036-cut-at-crossings/) | `cut-at-crossings` | Cut the selected paths into open pieces wherever they cross or touch, on the exact curves. | Done (#105) | Should | R-EDIT-023 | - |

### In progress, Ready (next, in the order the lead starts them)

Every spec here has complete criteria, `adrs.md` and UX notes; no customer
question blocks any of them (open questions have recorded defaults, Proposal
items are vetoable at the demo). 0032 and 0038 are being built (branches
pushed 2026-10-12, no PR yet). 0050 is the layers half of the Must requirement
R-EDIT-009; what a hidden object does in a machine output is decided with the
manufacturing view (R-MFG-005), before 0027 and 0029. The open criteria of the
built 0041 (18 and 25 for path copies) are follow-ups without a folder yet.
Their order, dependencies and conflicts are in "Proposed build order" below.

| No. | Slug | What it delivers | Status | Priority | Requirements | MVP slice |
|---|---|---|---|---|---|---|
| [0032](0032-pen-tablet-input/) | `pen-tablet-input` | A minimal Pencil (freehand) tool, and pen pressure from a drawing tablet, read through browser Pointer Events in the webview, that sets the width of Pencil strokes; the result is an ordinary outline; a mouse or a pen without pressure draws a plain path. Promoted by the customer on 2026-10-10 (was a Could draft). Device and OS questions are confirmed at the demo. | In progress (`story/pen-tablet-input`, no PR yet) | Should | R-INP-001 | - |
| [0037](0037-fracture-and-flatten/) | `fracture-and-flatten` | Fracture: cut overlapping shapes into the pieces the overlaps make. Flatten: trim every shape to its visible part and remove hidden shapes (decided by the customer, 2026-10-10). | Ready (since 2026-10-10; 0036 is built, so it waits only for 0038, same crate) | Should | R-EDIT-023 | - |
| [0038](0038-path-offset/) | `path-offset` | Outset and Inset by a typed distance in mm with Round, Miter or Bevel corners, open paths grow into a closed outline, live blue/black preview, the original kept. Uses the `i_overlay` offset already in the project (decided 2026-10-10, no new dependency); results are polylines within the kernel tolerance. | In progress (`story/path-offset`, no PR yet) | Should | R-EDIT-007 | - |
| [0042](0042-history-branches/) | `history-branches` | Steps taken back and then not redone become a branch (no linear redo until the conflict step is taken back); lanes in the history list; a Preview / Live switch to look at or go to an earlier version of an object, with a Restore button; a clone of the state at any row, also of deleted objects; step ids everywhere. Preview by default, Live by a switch (customer accepted the default, 2026-10-10). Builds on `0020` and `0041`. | Ready (since 2026-10-10; 0020 and 0041 are merged, nothing to wait for) | Should | R-HIST-003 | - |
| [0049](0049-linked-clone/) | `linked-clone` | A linked clone of a path or compound path: a second object that follows its original live (nodes, move, rotate, scale, skew, style) and can still be moved, scaled, rotated and skewed on its own through a matrix on top of the original's geometry. A primitive is converted to a path first, silently (`0047`). Select original, Unlink clone and Select clones, an inline block in the Style tab; deleting the original turns its clones into paths; a clone of a clone is allowed (chains up to 8 levels, a removed clone in the middle re-points the clones behind it); a selected clone and its original are marked on the canvas together; no style overrides; a clone is expanded to a path in every output. A new node kind, so a `format_version` bump. Customer request of 2026-10-10, decisions of 2026-10-11. | Ready (2026-10-11: customer accepted the ADR 0015 exceptions, chains and the marking; builds after 0050 and 0054) | Should | R-EDIT-030, R-EDIT-004, R-EDIT-012 | - |
| [0050](0050-layers-and-objects-panel/) | `layers-and-objects-panel` | A Layers tab in the Properties panel: a virtualised tree of layers, groups and objects (compound paths, paths, clones) with an eye, a lock and an opacity for every layer and every object, select by row in sync with the canvas, rename, drag to reorder and to move into or out of layers and groups, New layer and Delete layer, an active layer for new objects, keyboard and screen-reader access, and 10,000 objects without a hang. Visible, locked and opacity are view-side flags; what a hidden object does in a machine output is not decided (customer, 2026-10-11, R-MFG-005). Replaces the placeholder `0039-layers` (the layers half of R-EDIT-009). Needs `0023` built first and a `format_version` bump. Customer request of 2026-10-10, decisions of 2026-10-11. | Ready (2026-10-11: customer accepted the ADR 0015 exceptions and the Layers tab; 0023 and 0051 are merged, nothing to wait for) | Must | R-EDIT-031, R-EDIT-009 | 10 |
| [0052](0052-click-to-create-shapes/) | `click-to-create-shapes` | With the Rectangle, Ellipse and Polygon/Star tools a click (under 3 px of movement) starts a placement: the shape grows from the click point to the pointer without a held button, with today's modifiers; a chip on the shape takes the size and the angle (rectangle W, H, Angle; circle and polygon/star r or ⌀ by Space, Angle), the first typed character freezes the mouse, the typed values preview, a second click or Enter places, Escape or a right click cancels; Enter starts a placement from the keyboard (Proposal); one step "Draw rectangle" etc.; the drag is unchanged. Customer request of 2026-10-12. | Ready (2026-10-12, #112: architect decisions in `adrs.md`, no new ADR; UX notes confirmed; not beside 0032) | Should | R-EDIT-033, R-EDIT-002 | - |
| [0053](0053-node-marquee-selection/) | `node-marquee-selection` | In the Node tool a drag on empty canvas draws a red box that selects every node whose anchor lies inside, across all paths the Node tool shows; Shift adds, Ctrl removes, Ctrl wins, Alt nothing; nodes inside are ringed with a live count while dragging; Escape restores the previous selection; Join reads a box selection in document order, Delete removes all selected nodes in one step; Ctrl+A selects all nodes and the Node bar shows "5 of 120 nodes" (both Proposal). Customer request of 2026-10-12; no lasso. | Ready (2026-10-12, #112: architect decisions in `adrs.md`, no new ADR; UX notes confirmed) | Should | R-EDIT-034, R-EDIT-001, R-EDIT-014 | - |
| [0054](0054-combine-open-paths/) | `combine-open-paths` | Compound paths may hold open outlines and outlines that touch or cross (each other and themselves); fill is nonzero with open outlines closed by a straight line, the stroke stays open; Combine accepts every path and refuses no longer for open, touching, crossing or no-area outlines, winds holes by containment and counts touching outlines in its notice; Split attaches an open outline to the part it touches or lies on, Break apart gives every outline back unchanged; the Boolean operations, Fracture, Flatten and Inset close open outlines with a straight line and say so; Outset and Cut refuse; requirements for the SVG import and export of `0024`; the Node tool stays as today until `0055`. No `format_version` bump (customer default, 2026-10-12). Customer approval and decisions of 2026-10-12 (options B of `0035` Questions 1 and 3). | Ready (2026-10-12: customer decisions applied; Q1 and Q2 have defaults pending the customer; `adrs.md` and UX notes re-checked; builds after 0050, before 0049) | Should | R-EDIT-023 | - |
| [0055](0055-compound-path-node-editing/) | `compound-path-node-editing` | The Node tool edits every compound path: nodes selected across outlines and objects, move, kind conversion, segment bending, insert and delete (an outline left with one node is removed, a compound path left with one outline becomes an ordinary path); Join of open outline ends within one object or across objects (the other object's outlines move in, with a notice); Split at a node opens or splits an outline in the same object; Close path closes selected open outlines; the Pen continues and connects open outline ends. No format change. Customer decision 7 of 2026-10-12 on `0054`, split out by the lead the same day. | Ready (2026-10-12: `adrs.md` and UX notes done; the product owner's rules are vetoable; builds right after 0054) | Should | R-EDIT-023, R-EDIT-001 | - |
| [0056](0056-mirror-and-negative-scale/) | `mirror-and-negative-scale` | Two icon buttons in the Select bar, Flip horizontally and Flip vertically, about the middle of the selection's box; a resize drag, a typed W or H and the key-S size entry may pass through zero to the other side (negative scale) in every mode, in millimetres only (no percent); a size that would land on exactly 0 becomes 0.01 mm with a notice, so a shape can always be pulled apart again; a mirrored primitive stays a primitive. The H and V keys are a Proposal. Customer request and decisions of 2026-10-14 (PERCENT A, ZERO-RECOVER C). | Ready (2026-10-14, customer decisions applied; `adrs.md` of the architect still names the superseded ZERO-TYPED collapse) | Should | R-EDIT-012, R-EDIT-035 | - |
| [0057](0057-clone-overrides/) | `clone-overrides` | Two separate, visible and reversible overrides of a linked clone, switched in two rows of its Style tab block: own placement (the clone stays where it is when its original is transformed; node edits of the original still reach it) and own style (a complete style of its own including opacity, fill and stroke colour per clone, e.g. one fish in twelve thread colours; back to the original's style in one click); a Proposal button "Spread colours" gives selected clones evenly spaced hues. Nothing jumps on a switch. The own rotation pivot of the first draft was dropped (customer, 2026-10-14: miscommunication). Extends 0049 and amends accepted text (see "Changes to accepted specs" in the spec). Customer decisions of 2026-10-14 applied (UI A, colours A, opacity B); one question open with a default (style of a clone of a styled clone). | In progress (`story/clone-overrides`, no PR yet; ADR 0016 accepted 2026-10-14; format 14) | Should | R-EDIT-036, R-EDIT-030, R-EDIT-012 | - |
| [0021](0021-ellipse-arcs-and-shaping/) | `ellipse-arcs-and-shaping` | Arc handles on an ellipse (pie, arc, chord; Start and End angles in the Select bar) and a Curve handle that turns a circle, polygon or star from straight sides to bulging or concave ones. Arc and curve are parameters of the primitive, so it stays a primitive. One `format_version` bump at merge; no new crate, no new ADR. | Ready (2026-10-17) | Should | R-EDIT-002 | - |

### Specified but not built

Written as criteria in a feature's spec, then taken out of that feature's
slice (lead decision 2026-10-09). No folder and no number yet; each becomes its
own entry when the customer schedules it.

| Part of | What it would deliver | Status | Priority |
|---|---|---|---|
| 0016 `boolean-operations` | Hover preview of the result: the outline of the result drawn over the canvas while the pointer rests on a Boolean button (criteria P1 to P3 in the 0016 spec). A Proposal, never accepted; no numbered criterion of 0016 depends on it. | Not started | Should |

### Draft (waiting for the customer to schedule)

| No. | Slug | What it delivers | Status | Priority | Requirements | MVP slice |
|---|---|---|---|---|---|---|
| [0024](0024-svg-import-export/) | `svg-import-export` | First slice: import a plain SVG from Inkscape into the open project as objects (File tab **Import SVG…**, placed on the centre of the view, one layer, one history step) and Open of `.svg` as a new project. Transforms, arcs, CSS classes, even-odd fills (re-wound by depth), `<use>` (a linked clone where it is only a move, rotation or flip, else a copy) and page sizes (viewBox, no viewBox, Illustrator and old Inkscape pixels) have defined results; what is not supported (text, images, clips, masks, filters, foreign data) is ignored silently: there is no loss report. Untrusted-file limits, golden files from Inkscape-made fixtures, budgets. SVG export is kept as a section "Later: export" (second slice; R-SYS-006 is met in two steps). No open questions (customer decisions of 2026-10-16). | Draft (2026-10-16: customer decisions applied) | Must | R-SYS-006 | 11 |
| [0022](0022-color-management/) | `color-management` | Palettes, thread palettes for embroidery, colour history, other colour models. A sketch for the customer, after the MVP. | Draft | Could | R-EDIT-017 | - |
| [0046](0046-document-shapes/) | `document-shapes` | Non-rectangular document areas: ellipse and circle, rounded rectangle, and an outline loaded from SVG or DXF (key rings, labels, hoops); the size is the bounding box, the area is a guide and not a mask. A sketch with open questions. Needs a document-model decision, the SVG import of `0024`, and a DXF importer nobody has specified (`0040` and `0045` are built). | Draft (sketch; the document-model ADR is `needs-customer` and not yet written) | Should | R-EDIT-028 | - |
| [0033](0033-stroke-brushes/) | `stroke-brushes` | Brushes for the stroke (width profile, stamp, pattern along path), a picker in the Stroke section, brushes as data files the maker can add, plugin brushes later; expanded to paths for jobs. After the MVP, 0017, 0018 and the plugin host. | Draft | Could | R-EDIT-021 | - |
| [0025](0025-raster-trace/research.md) | `raster-trace` | Trace a raster image to vector paths (threshold, colour count, presets) with smoother, lighter results than Inkscape's "Trace Bitmap"; a Trace tool with settings in the top-middle bar. Research and 8 customer questions are in `research.md`; no `specification.md` yet. Needs bitmap import (0058). | Draft (research; waiting for the customer) | Must | R-VEC-001 | 12 |
| [0058](0058-bitmap-import/) | `bitmap-import` | Import a PNG, JPEG or WebP (customer 2026-10-16: no BMP or GIF; TIFF only if easy) as an image object: an "Import image…" button in the File tab (Ctrl+I, drop and several files are Proposals), size from the file's DPI else 96 dpi and fitted to the document with a notice, placed at the view centre on top of the active layer, always embedded in the project (content-addressed, original bytes kept, a working image of at most 16 Mpx), drawn by the renderer with opacity, moved, scaled, rotated, skewed and flipped like any object, hit by its whole frame, an "Original size" button and an Image block in the Style tab (Proposals), safe against damaged and hostile files, the data hand-off for `0025`. A new node kind, so a `format_version` bump. Customer request of 2026-10-14 ("upstream of tracing"); Questions decided 2026-10-16. Builds before `0025`. | Ready (customer decisions of 2026-10-16 applied; ADR 0017 accepted, its spike confirms the numbers before the build) | Must | R-EDIT-037, R-VEC-001 | - |

### MVP slices not yet specified (no folder, number reserved)

| No. | Slug | What it delivers | Status | Priority | Requirements | MVP slice |
|---|---|---|---|---|---|---|
| 0026 | `machine-profile` | Define and reuse a machine profile (work area, connection, limits) for a laser cutter; select it for a project. | Not started | Must | R-MFG-001 | 13 |
| 0027 | `manufacturing-roles` | Assign a cut or engrave role to geometry (by layer) within one file. | Not started | Must | R-MFG-002 | 14 |
| 0028 | `material-test-library` | Generate a power/speed test-cut grid for the selected machine, record which cell worked, store it as a reusable material record. | Not started | Must | R-MAT-001, R-MAT-002, R-MAT-003 | 15 |
| 0029 | `laser-job-preview-and-output` | Toolpath and time preview, an explicit machine and material gate before export, GRBL G-code export for cut and engrave geometry. The MVP's capstone: a maker's own design goes from drawing to a file their laser runs. | Not started | Must | R-MFG-003, R-MFG-LASER-001, R-SYS-008 | 16 |
| 0039 | `layers` | Layers with per-layer visibility and lock, to separate cut, engrave and reference geometry; the second half of the old slice `layers-and-grouping` (the first half is `0023-groups`). Number assigned 2026-10-10 when 0023 was given to groups. **Superseded the same day by [0050](0050-layers-and-objects-panel/)**, which delivers layers together with the customer's layers and objects panel; the number stays as a placeholder and is not reused. | Superseded by 0050 | Must | R-EDIT-009 | 10 |

Not in this list: Should and Could requirements, and every Must the customer
deferred past the laser MVP on 2026-10-02 (multi-OS parity and sync, fonts,
asset management, collaboration, the asset connector, plugins, other machine
families). They are tracked in `docs/requirements.md` ("MVP (confirmed)" and
"Explicitly deferred past MVP") and get a number here once the customer
prioritises them. The next free number is 0059 (0058 is `bitmap-import`; 0057 is `clone-overrides`; 0056 is `mirror-and-negative-scale`; 0055 is `compound-path-node-editing`; 0054 is `combine-open-paths`; 0047 is `transform-polish`; 0048 is `split-compound-path`; 0049 is `linked-clone`; 0050 is `layers-and-objects-panel`; 0051 is `sidebar-and-no-menu`; 0052 is `click-to-create-shapes`; 0053 is `node-marquee-selection`).

## Proposed build order (product owner, updated 2026-10-12)

For the specs that are not built yet. The lead decides; this is my reading of dependencies and of which slices can run side by side (at most two implementers, `CLAUDE.md` §4). Almost every slice touches `curvyo-editor-wasm` and `frontend`, so "same crate" cannot be the test; the table names the files and modules that actually collide. Everything in "Needs first" that is already merged is left out.

| Spec | Main footprint | Needs first | Collides with |
|---|---|---|---|
| 0038 `path-offset` (in progress) | `geometry-core` (offset on `i_overlay`, no new dependency), an entry widget, `render-core` preview | nothing | 0037 (same crate) |
| 0037 `fracture-and-flatten` | `geometry-core` (kernel use), rail buttons (the Boolean card moves to column B) | nothing | 0038 (same crate): after it |
| 0042 `history-branches` | `document-core` (branch derivation, state at a version, preview fork, clone), `render-core` preview, `editor-wasm`, History tab lanes | nothing (0020 and 0041 are merged) | the History tab files of 0041's follow-ups |
| 0024 `svg-import-export` (Draft) | an SVG reader in a `*-core` crate (`curvyo-svg-core`, ADR 0018 proposed; XML parser dependency), `document-core` apply of a drawing (one commit, existing kinds, linked clones of `0049`), `editor-wasm` commands (Open of `.svg`, Import), `frontend` (one File tab button), `storage-io`/`app` (file dialogs), fixtures in `tests/fixtures/svg/`; the export (writer, Export button) is a later slice | 0050, 0054 (layers, compound paths), 0051 (File tab); 0049 and 0056 for the `<use>` clone and flip criteria | 0046 stage 2 (uses its importer), the File tab files of 0051; nothing else of the Ready slices |
| 0046 `document-shapes` (Draft) | `document-core` (shape register, format bump), `render-core` (area, edge), importers in `0024` (SVG) and an unspecified DXF reader, the Document tab | the customer and architect on the model; stage 2 needs 0024, stage 3 a DXF importer | 0050 and 0049 (format version) |
| 0050 `layers-and-objects-panel` | `document-core` (layer marker, registers `name`, `visible`, `locked`, `opacity`, effective flags in the read, tree-move command, format), `ui-core` (selection filter against hidden and locked, drag-drop rules, Ctrl+A across layers, panel row window), `render-core` (per-paint alpha product, hidden skipped), `editor-wasm` (active layer, row window, panel target), `frontend` (Layers tab in the vertical strip, treegrid, virtualised list, Opacity field), `docs/design-system.md` | nothing (0023, 0043, 0051 are merged; ADR 0015 accepted) | 0049, 0054, 0053 (hidden and locked filter): runs alone among them |
| 0054 `combine-open-paths` | compound-path model and the path commands of 0035, 0048, 0036 to 0038 (see its `adrs.md`) | 0050 (lead order) | 0055, 0037, 0038 (the same commands): not beside them |
| 0055 `compound-path-node-editing` | `ui-core` `node_tool.rs` and `pen_tool`, `document-core` node commands (see its `adrs.md`) | 0054 | 0053 (`node_tool.rs`), 0032 (Pen) |
| 0049 `linked-clone` | `document-core` (clone node kind, one `clone_link` register `{origin, matrix}`, resolved read, bake on removal, orphan repair, format), `ui-core` (matrix gesture function, selection and Node-tool rules, availability), `editor-wasm` (commands, notices), `frontend` (Select-bar button, clone block in the Style tab) | 0050 and 0054 (recommended, so its rows show the link glyph from day one) | 0050 (document read model, selection, panel files); every command that removes objects (the bake hook) |
| 0052 `click-to-create-shapes` | `ui-core` creation tools (`shape_tool_common.rs`, the rectangle, ellipse and polygon/star tools: placement state, freeze, one resolving function with the drag), a length parser with unit suffixes, `render-core` (anchor marker; the preview is the create-drag's), `editor-wasm` (`session/shapes.rs`, the key gate in `session/keys.rs`), `frontend` (a placement chip built from the move chip's `EntryField` and surface, rail tooltips) | nothing (0047 and 0020 are merged) | 0032 (tool rail, bars, key gate): not beside it. 0053 only in `keys.rs` (rebase-level): may run beside it |
| 0053 `node-marquee-selection` | `ui-core` `node_tool.rs` (a marquee drag, the clear moved to the release) and `selection.rs` (ordered add and remove), `render-core` (the Select marquee box with a colour, node rings), `editor-wasm` `session/node.rs` and `keys.rs` (Ctrl+A in the Node tool), `frontend` (Node bar count slot, legend, live region) | nothing | 0055 (`node_tool.rs`, Node bar); 0050 (hidden and locked filter); 0052 only in `keys.rs` |
| 0057 `clone-overrides` (Ready) | `document-core` (override fields on the clone node, held-back rule in the transform writers, effective style in the read, format), `ui-core` (panel rows), `render-core` (effective style and opacity), `editor-wasm`, `frontend` (two rows and Spread colours in the clone block of the Style tab, Opacity field in the Layers tab) | 0049 merged; ADR 0016 accepted; the customer's answer to Frage 5 (default A) | 0049 (same clone read, `clone_link`, clone block), 0050 (row tooltip), 0056 (Flip writer) |
| 0058 `bitmap-import` (Draft) | `document-core` (image node kind, asset reference, format), a pure decoder function in a `-core` crate (crate choice and ADR: architect), the asset entry in the container (`0001`/ADR 0004 §1 persistence), `render-core` (texture, mipmaps, opacity), `ui-core` (hit-test and transforms of a leaf with a frame), `editor-wasm` (import command, notices), `frontend` (File tab button, Select bar "Image" group, Style tab Image block) | the architect's check; 0050 merged (layers, active layer, opacity); 0051 merged | 0049 and 0057 (the clone rule for images, one word in `0049` criterion 4), 0056 (flip of a leaf), 0050 (new node kind in the same read model). Goes before 0025 |
| 0032 `pen-tablet-input` (in progress) | pointer input in `frontend` and `editor-wasm` (input record, coalesced events), a Pencil tool in `ui-core`, `geometry-core` (fit, variable-width outline), the tool rail and a Pencil bar | nothing | 0052 (key gate, tool rail and bars); 0038 and 0037 only in `geometry-core` (separate modules); 0033 (width model) only if the stored result changes |
| 0033 `stroke-brushes` | style model, `render-core`, `geometry-core` generators, panel row, plugin host | 0017, 0018, the plugin host | 0018, 0032 (style and width model) |

**Waves, two slices at a time** (a slice may start as soon as its own "needs first" is merged; waves 1 to 7 of the 2026-10-10 plan are built, see "Done"):

1. Now: 0032 `pen-tablet-input` and 0038 `path-offset` (both in progress).
2. 0053 `node-marquee-selection` next to either of them (disjoint); 0052 `click-to-create-shapes` after 0032 (same tool rail and key gate); 0037 `fracture-and-flatten` after 0038 (same crate).
3. 0050 `layers-and-objects-panel`, alone among the panel and document-model slices. It is the layers half of the Must requirement R-EDIT-009 and comes before the laser output slices 0027 and 0029, which read layers; what the hidden flag means for their output is decided with the manufacturing view (R-MFG-005) before those slices are built.
4. 0054 `combine-open-paths`, then 0055 `compound-path-node-editing`, then 0049 `linked-clone` (lead order of 2026-10-12). 0042 `history-branches` can run beside any of them that does not touch the History tab or the renderer preview.
5. 0058 `bitmap-import` before 0025 `raster-trace` (customer, 2026-10-14: a bitmap must be importable first); it needs 0050 and 0051 merged and not 0049 or 0057, so it can run beside the clone slices once the architect has checked it. MVP slices 0024 to 0029 are not specified yet; 0046 `document-shapes` stage 2 needs 0024.
5b. MVP slice 0024 `svg-import-export` is specified (Draft; customer decisions of 2026-10-16 applied: import first, export later, no loss report); 0025 to 0029 are not specified yet; 0046 `document-shapes` stage 2 needs 0024. 0024 builds after 0050 and 0054 (layers, compound paths) and shares nothing else with the Ready slices beside the File tab.
6. After the MVP: 0033 (after the plugin host).

**The left rail has two columns (decided 2026-10-10; layout in `docs/design-system.md`, row "Tool rail architecture").** Column A: the Tools card (seven tools once the Pencil of 0032 ships) and the Group card (Group, Ungroup, 0023). Column B, drawn since 0035: the Path card (Combine, Split, Break apart, Cut; 0035, 0048, 0036) and the Boolean card (the five Boolean operations, then Fracture and Flatten, 0037). The Boolean card moves from column A to column B in the first release that needs the room in column A (0023, 0032 or 0037); it is the only built card that ever moves. Offset (0038) is not a rail button: it is a text button in the Select bar that opens a section of the Properties panel. Both columns fit the 800 x 600 window; a further family of commands goes to the Select bar or the panel. The specs define behaviour; placement is the design system's.

## The MVP and its sequence

The laser MVP (`docs/requirements.md`, "MVP (confirmed)") is the 16 slices
that carry an entry in the "MVP slice" column. Scope: **laser-only,
local-only, single-user, one controller dialect** (GRBL-family G-code),
confirmed by the customer on 2026-10-02. Cutting plotter, embroidery, CNC,
real-time collaboration, the asset connector and the plugin interface are each
later slices once the MVP ships.

Every MVP slice is **Must** priority; the MVP has no Should or Could. A slice
may fold in a `Proposal` requirement that only makes sense next to the
Customer one beside it.

Two requirements are not their own slice because every slice must already
satisfy them: **R-SYS-002** (offline-capable) and **R-SYS-007** (no
telemetry). A slice that needs the network, or phones home for anything the
maker did not trigger, has a bug, not a variant.

### Spec number versus slice label

The old MVP list numbered the slices 1 to 16 and gave folders the same
number. Since the customer asked for one consecutive numbering of all specs,
the folder number now follows the list above, and the slice label lives in the
"MVP slice" column. The two agree for 0001 to 0007. After that they differ:
slice 8 `undo-redo` is 0020, slice 9 `boolean-operations` is 0016, slice 10
`layers-and-grouping` is split into 0023 `groups` and 0039 `layers` (the layers
half is delivered by 0050 `layers-and-objects-panel`, which replaced the
placeholder 0039), and slices 11 to 16 are 0024 to 0029. Existing text that says "slice 5" or "slice 8"
means the old label. Text that says `0005` or `specs/0005-object-transform/`
means the folder.

### Order and dependencies between MVP slices

Slice labels, as in the column above.

- **1 before everything**: nothing to draw on or persist without a project
  file and a canvas. It uses the `.curvyo` container and `document.loro`
  backing decided in ADR 0004 §1.
- **2 before 3**: primitives are "editable as paths after creation"
  (R-EDIT-002) and "object to path" (R-EDIT-004) needs a path-editing surface
  to convert into. The shape tools are a thin layer over it.
- **4 to 6 were inserted after the customer tried slices 1 to 3 by hand**
  (2026-10-05): pan/zoom and a general Select tool (4), scale and rotate by
  handles (5), join, split and a third node type (6). They run before styling
  at the customer's request, because styling is hard to judge on objects you
  cannot select, transform or view at a useful zoom.
- **8 (`undo-redo`) after 2 to 3 and 5 to 7, before 9 and 10**: undo is
  cross-cutting (ADR 0002 §9, ADR 0004 §2, peer-scoped and CRDT-backed), but
  "covers every editing operation" (R-EDIT-008) can only be checked once
  several operations exist. Pan, zoom and selection are view state and have no
  undo surface. Slices 9 and 10 each add a criterion that their new operation
  is undoable through the mechanism slice 8 sets up, rather than repeating its
  work. The list above does not follow this order: 0016 (slice 9) was specified
  ahead of it. The customer's undo text arrived on 2026-10-10 and is specified in
  0020 (slice 8), with the object history and branches in 0041 and 0042. How
  0016 handled its undo criterion until 0020 exists was 0016's own decision.
- **11 (SVG round trip) after 7 and 10**: round-tripping a real Inkscape file
  is only a meaningful test once paths, primitives, styling, groups and
  layers exist.
- **12 (`raster-trace`, 0025) after 0058 `bitmap-import`**: tracing needs an
  image object to trace; the customer asked for the import first (2026-10-14).
  0058 has no slice label because it came from customer feedback.
- **13 to 16 last and in that order**: a job needs a machine profile (13)
  before roles can target one (14); roles and a machine before a material test
  is worth running (15); all three before the capstone export (16), where
  R-SYS-008's "no job without a machine profile" gate applies.

### `format_version` plan

The project file's `format_version` is **11** on `main` (9 set by 0017 and 0018, PR #78; 10 by
0040, PR #90; 11 by 0023, PR #102). The next bump is 12.
Each change that alters the on-disk shape takes the next number at merge, and
a reader refuses a file with a higher number ("saved by a newer version").

| Version | Taken by | State |
|---|---|---|
| 7 | 0007 `stroke-and-fill-styling` | Done, merged (#54) |
| 8 | 0016 `boolean-operations` (compound path, PR #73) | Done, merged |
| 9 | 0017 `style-panel-rework` (odd dash lists) and 0018 `stroke-markers`, one PR | Done, merged (#78) |
| none | 0015 `document-size-and-rulers` | No bump (its `adrs.md`, decision 1) |
| 10 | 0040 `document-background` (background paint and colour registers in the document root) | Done, merged (#90) |
| 11 | 0023 `groups` (a group node with children: the register `group: true`, tree children as members) | Done, merged (#102). Golden: `groups_v11.curvyo` |
| next free at merge | 0050 `layers-and-objects-panel` (the `layer` marker and the registers `name`, `visible`, `locked`, `opacity` on any node; an older build would draw a hidden node and could output it) | Ready; 12 if it is the next bump to merge |
| none (architect, 2026-10-12) | 0054 `combine-open-paths` (Combine starts writing `closed: false` in the per-outline register that exists since 8, and touching or crossing outlines; older builds already read both) | Ready (2026-10-12); builds after 0050 and before 0049; no bump (customer default, 2026-10-12) |
| 13 | 0049 `linked-clone` (a clone node: `clone_link` = `{origin, matrix}`; an older build would read a node without an outline) | Built on `story/linked-clone`, takes 13 after 0050's 12. Golden: `clone_v13.curvyo` (a chain of three, a compound root, a converted rectangle), damaged `clone_matrix_five_v13.curvyo`, orphans `clone_orphan_and_ring_v13.curvyo` |
| 14 | 0057 `clone-overrides` (a placement flag, an own style and an own opacity on the clone node; an older build would draw a clone with own style in its original's colours, which matters for embroidery) | Built on `story/clone-overrides` (2026-10-15); takes 14 if nothing else bumps first. Golden: `clone_overrides_v14.curvyo`, damaged `clone_overrides_flag_damaged_v14.curvyo` |
| next free at merge | 0058 `bitmap-import` (an image node kind: asset hash, frame map, name; the bytes sit in `assets/` of the container; an older build would show nothing where the picture is) | Draft (2026-10-14); 15 if it merges after 0057, otherwise the next free one |
| next free at merge | 0021 `ellipse-arcs-and-shaping` (a list register `ellipse_arc` on the ellipse and a `curve` register on ellipse, polygon and star; an older build would cut an arc as a whole ellipse) | Ready (2026-10-17); one bump for the arc and the curve, 15 if it merges after 0058 and 0058 takes 15 |
| next free at merge | 0033 `stroke-brushes` (brush reference and embedded definition) | Draft, after the MVP |
| none (ADR 0014 Q2 A, accepted 2026-10-10) | 0041 `object-history` (the optional keys `history_floor` and `clone_of`, ADR 0014 §7) | Done (#103); the keys are additive and older builds ignore them (as 0015's `display_unit`) |
| next free at merge | 0046 `document-shapes` (the `page_shape` register; an older build would draw a rectangle where a key ring is) | Draft; `needs-customer` |
| none | 0020, 0042, 0043, 0044, 0045, 0047 | 0020 saves the history that `document.loro` already holds (step headers in commit messages) and adds one optional root key, `history_wiped` (ADR 0014 §7); 0042 derives branches from the log; 0043 and 0044 write nothing; 0045 keeps formats in a user file outside the project; 0047 writes paths, an existing kind. 0042 keeps whatever version is current when it merges. A bump for history would come only with soft delete (ADR 0014 Q7 B), which the customer gets back only if a Loro upgrade breaks the canary |
| none | 0051 | No bump: the saved file is untouched; the collapsed state of the panel is a per-user preference outside the project, and the dirty flag is view state |
| none | 0030, 0031, 0034, 0035, 0036, 0037, 0038, 0048 | No bump: they write existing object kinds (0035 to 0037 and 0048 use the compound path of 0016) |
| none | 0056 | No bump: a mirrored object is written with the fields of its own kind (rotation, radii, frame angle, anchors); no negative length and no matrix is stored (its criterion 26) |
| none | 0024 | No bump: an import writes existing kinds and registers (a linked clone uses the register of 0049); no passthrough bag (BAG A, decided 2026-10-16); a bag would take the next free number if SVG ever becomes a native format |
| none | 0052, 0053 | No bump: a placement writes the same kinds and fields as a create-drag; the node marquee changes view state only, and Delete and Join use their existing commits |
| none | 0032 | No bump: a pressure stroke is stored as an ordinary closed path (its Question 2, A); a stored width profile (B) would take the next free number together with 0033 |

A spec does not hard-code a version number it does not own; it says "next free
at merge". A slice names a number only when it is the next bump to merge: 0016
named 8, 0040 names 10. If another bump merges first, that slice's tests
compare against its own named constant and the rebase renumbers it; this table
is then corrected in the same PR.

## Numbering

`NNNN` is a 4-digit, zero-padded number, the same style `docs/adr/NNNN-slug.md`
uses for ADRs. The number is the folder's position in the feature list above,
assigned when the folder is created: take the next free number (or the reserved
number of a placeholder row) and add the row to the list in the same PR.

On 2026-10-09 the customer had every folder renumbered into one consecutive
sequence (built features in merge order, then the planned ones). Before that,
only the MVP slices had numbers. That renumbering was a one-time decision by
the customer; no one renumbers on their own. Folders keep their number after
it is assigned, even if the list is later regrouped. A slug may change while its spec has no branch and no PR: `0036-split-at-crossings` became `0036-cut-at-crossings` on 2026-10-10 when the customer renamed the command; the number stayed.

Branches, PR titles and prose refer to a feature by its slug alone
(`story/<slug>`), with no number. Paths and links use the full
`NNNN-slug`.

## Keeping the list current

The product owner owns this list. Change it in the same PR that changes the
fact:

- a new folder adds its row (status, priority, requirements);
- a status change in `specification.md` changes the row in the same PR;
- when a story is accepted: `Status: Done` in the spec, the PR numbers in the
  row and in the spec's Links.

A status in a row that disagrees with the spec's `Status:` line is a bug in
the row.

## Layout of a feature folder

One folder per feature: `specs/<NNNN-feature-slug>/`, three files.

### `specification.md` — what and why (product-owner)

```text
# <feature title>

Status: Draft | Ready | In progress | Done
Priority: Must | Should | Could
Origin: Customer | Proposal

## User value
As a <maker role> I want <capability> so that <outcome>.

## Acceptance criteria
1. Given ..., when ..., then ...

## Out of scope
- ...

## UX notes
(filled in by ux-engineer before Ready)

## Links
Requirements: ...  PR: ...
```

### `adrs.md` — which decisions apply (architect)

```text
# ADRs for <feature title>

- [ADR 000X](../../docs/adr/000X-slug.md): <one line on what it decides
  for this feature>

## Feature-local decisions
Anything too small for a full ADR, dated:

- YYYY-MM-DD: <decision> — <why>
```

### `plan.md` — how it gets built (implementer)

```text
# Plan for <feature title>

## Affected crates/modules
- ...

## Tasks
- [ ] 1. <task> (fulfils AC 1, 2)
- [ ] 2. <task> (fulfils AC 3)

## Validation
How this gets verified beyond the tester's acceptance tests (e.g. golden
files, property tests, manual check with hardware).
```

## Lifecycle

Same stages as `CLAUDE.md` §4:

1. **Ready** — `specification.md` + `adrs.md` exist, UX notes attached if
   the feature has UI.
2. **Build** — implementer writes `plan.md`, then works on branch
   `story/<feature-slug>` in a worktree, checking off tasks as it goes.
3. **Verify** — tester works from `specification.md` only; `ux-engineer`
   and `architect` review per `CLAUDE.md` §4.
4. **Demo** / **Done** — as in `CLAUDE.md` §4; `specification.md` gets
   `Status: Done` and the PR link.

**One PR per slice (customer rule, 2026-10-09).** A slice is one story and
one PR on one branch. A large slice is built as milestones (commits on that
branch, each green), not as several PRs; the customer accepts the slice once,
against its review guide. A part the customer does not need for the slice moves
out as its own entry (see "Specified but not built").

A feature folder's `specification.md` is the single source of truth for
what "done" means. `plan.md` tracks progress; it is not a second backlog.

# Concept: page kinds, output pages, operations and fixtures (base for every machine workbench)

Status: Accepted (concept A, customer 2026-11-05; Fragen 147 to 156 answered, §10). It replaces the
direction of `0093`, `0027` and the Sketch workbench switch of `0066`. Backlog: the epic
`specification.md` in this folder; slices `0098` to `0103`, then `0028` and `0029` (§9).
Sketch h is drawn for 154 B (the Cuts last switch, §2.6).
Sketches: `concept-sketches.html` in this folder (a to i; open it in a browser, about 1440 px wide). New in round 2: f (page kinds), g (fixtures), h (run order), i (move versus edit); a to e updated, no switcher in any sketch.
Everything marked **(Proposal)** is a reversible UX choice, vetoable at the demo.

## 0. The customer's feedback of 2026-11-05, read as requirements

| # | Feedback (translated) | What it requires |
|---|---|---|
| 1 | "Overall we're on the wrong track." LightBurn uses layers, PewPew Studio uses nodes; neither is quite right. | A model that is neither "drawing layer = laser layer" nor a graph editor. |
| 2 | A vector layer is not necessarily one laser vector; single paths, objects or groups too. | An operation attaches to *any* node: object, group, clone, placed page, a layer inside a placed page. |
| 3 | More than Cut/Engrave: fill and outline together, cut out, etc. | Operation *kinds* with a geometry rule each; Fill+Outline and Cut-out among them. |
| 4 | Position things; whole documents/pages that snap into a fixture; fixtures are holders, clamps, grids. | Fixtures are objects with snap targets; a placed page snaps into a holder slot by fitting size. |
| 5 | No workbench switching. The laser output is a completely normal page; the workbench follows the active page; Add page has a machine switch. | Page *kind* instead of a manufacturing switcher entry. The workbench is derived, never chosen. |
| 6 | Content arrives as linked clones by drag, or is made directly there. Outputs are compositions with connecting elements. Easy, no unnecessary steps. | Drag = linked clone. All drawing tools work on the output page. No mandatory dialog or step. |
| 7 | Layers, Snapping, Grid, Share tabs and the Text tool on the laser page. | The output page is a full editing context, not a read-only viewer. |
| 8 | Frage 140 A, 141 A, 142 B. | Engrave fills closed areas and draws open paths as a line; nothing runs until assigned; Show and Output are separate, a hidden thing with an output setting still runs. |

### Round 2 (customer, 2026-11-05, after choosing concept A)

| # | Feedback (translated) | What it requires |
|---|---|---|
| 9 | Operation optimisations, like PewPew Studio: run all cuts last, engraving on parts that have already fallen out is bad. | A run-order rule that cannot be broken by accident, and its relation to the list order (§2.6, sketch h, Frage 154). |
| 10 | The Sketch workbench moves to the same model: a Sketch is a page kind. The workbench switcher (a thorn from FreeCAD) goes. Pages show their type. Sketches are positioned only via linked clone, they are fixed on their page. | Page kinds Drawing, Sketch, Laser; **no workbench switcher anywhere**; the kind shown as glyph and word next to every page name (§2.1, sketch f). |
| 11 | Fixtures: the engraved grid is good but needs measuring tapes and thicker major lines; fixtures may need cutting or engraving themselves (a slide-in board with engraved coaster outlines); fixtures are selectable; several active at once. | Grid with tapes and major lines, fixtures as selectable objects with their own operations (fixture jobs), an Active switch per fixture (§6, sketch g). |
| 12 | A laser page's size is not editable: the laser is as big as it is. | Work area from the machine profile only, shown as text (§2.1, sketch a3). |
| 13 | Objects can be moved anywhere: pasteboard, over a laser page, over a sketch page. Moving always works; editing only in the context of an active vector page or a matching manufacturing page. | A precise split of move and edit, and a rule for what an object over a laser or sketch page is (§2.7, sketch i, Frage 155, 156). |
| 14 | (addition) The switcher at the top right disappears in every sketch; the page type is shown with symbol and text next to the page title, wherever the active page's title appears. | Kind chip in the page list row, the panel header, the canvas page label and the status bar (all sketches). |

What this rules out in what is built today: the read-only output page (`0093` criteria 4, 7), the
Add manufacturing entry in the switcher (`0093` criterion 2, `0063` criteria 1 to 3), the output page
hidden from the page list (WB-PAGE-VIS A), the role keyed by the drawing's layer (`0027`), and now
the **workbench switcher itself** (`0093` UX notes §1, `0066` criterion 18: Vector / Sketch selector).

## 1. Research: what each tool gets right and wrong for this customer

| Tool | How it works | Right for us | Wrong for us |
|---|---|---|---|
| **LightBurn** | One canvas = the bed. Up to 30 colour-indexed layers in the Cuts/Layers list; an object's stroke colour picks its layer. Each layer has a mode (Line, Fill, Offset Fill, Image), sub-layers for a second pass, and separate **Show** and **Output** switches. The list order is the run order. A colour palette at the bottom assigns the selection in one click. | The ordered list as the one overview of "what runs"; Show vs Output; one-click assignment; Fill+Line as one mode. | The drawing's colour is the machine instruction, so the drawing is abused and one file serves one setup. Everything is copied in, no live link. "Layer" here is a set of objects by colour, which is exactly what the customer calls not quite right. No fixtures (the camera and Print-and-Cut are the workaround). |
| **PewPew Studio** (as the customer describes it; I could not verify its UI from public sources, so this row is about node-based laser and CAM tools in general, Grasshopper and Blender geometry nodes included) | Sources (files, shapes) are nodes, operations are nodes, wires feed one into the next and into the machine. | Explicit data flow: one source can feed several operations; nothing is decided by a colour. | The graph is a second editor away from the canvas; even a simple "cut this out" takes several nodes and wires. Fails "very easy, no unnecessary steps". |
| **xTool Creative Space** | Canvas = bed. Select an object, choose its processing type (Score, Engrave, Cut) and power/speed in the right panel; material presets per machine. | Direct: one selection, one choice. The canvas is the bed. Kinds named in plain words (Score, Engrave, Cut). | Settings live on objects, so there is no single list to reorder, compare or switch off; changing ten objects means ten selections. No live link to a design, no fixtures beyond the bed. |
| **Glowforge app** | Steps grouped by colour, each step Cut, Score, Engrave or Ignore; steps reorder. | The kinds Cut / Score / Engrave / Ignore and a reorderable step list. | Grouping by colour again; no compositions. |
| **LaserGRBL** | Load one file, convert it, send it. | Zero ceremony. | No composition, no placement, no mixed operations. |
| **Inkscape / Affinity pages** | Several pages in one plane, a page list, drag between pages. Affinity's personas (Designer, Photo, Publisher) switch the tool set over one document by hand. | Pages as the unit a maker thinks in; the persona idea (same document, different tools). | The persona is switched by hand; the customer wants it to follow the page. |
| **Fusion 360 Manufacture** | A Setup holds stock, origin and fixtures (bodies); an ordered operation list per setup in the browser tree; each operation *selects* its geometry; the toolpaths follow the design live. | Operations select geometry explicitly (not by colour or layer); ordered list per setup; fixtures as real bodies; live link to the design. | Every operation is a modal dialog with many tabs; the workspace switch is manual; far too many steps for a laser. |

**What we take:** the ordered operation list with Show and Output (LightBurn), the explicit
selection of what an operation applies to (Fusion), one-click assignment from a selection (LightBurn
palette, xTool), plain kind names (Glowforge, xTool), pages in one plane (Inkscape), the persona that
follows the page instead of a switch (Affinity, changed per customer).
**What we refuse:** colour as an instruction, a graph editor, per-object settings with no overview,
dialogs for operations, a manual workbench switch.

## 2. The proposed mental model

The model in seven points; only these words appear in the UI: page kind, operation, assignment, fixture, fixture job.

1. **Page kinds.** Every page has exactly one kind: **Drawing**, **Sketch**, or an output kind for a
   machine (**Laser** first; CNC, Plotter, Embroidery later). The kind is chosen once, in Add page,
   and never changes. All kinds sit in one page list and on one canvas plane (151 A). Every place
   that shows a page name shows its kind next to it as a glyph and a word (Drawing `PenTool`,
   Sketch `DraftingCompass`, Laser `Zap`): the page list row, the panel header, the canvas page
   label and the status bar. Shape and text, never colour alone.
   - A **laser page's size is the machine's work area** and is not editable. The Machine block shows
     it as text ("Work area 400 × 300 mm"); picking another profile changes it, content keeps its
     place relative to the origin corner (Proposal).
   - A **sketch page** holds exactly one sketch (the "page sketch" of `0066`, one owning Sketch
     object per page) and nothing else. The sketch is fixed to its page: its coordinates are the
     page's. To use it elsewhere (a drawing page, a laser page) you place a **linked clone** of it,
     which has its own position, rotation and scale; dragging the sketch off its page makes that
     clone (sketch i, drop badges). Moving the sketch page in the plane moves the sketch with it.
     Its size is editable like a drawing page's (Proposal).
2. **The tools follow the active page; there is no workbench switcher.** Activating a page (a press
   inside it, its row in the page list, `Ctrl+Page Up` / `Ctrl+Page Down` through the list,
   Proposal) shows that kind's tool rail and tabs: Drawing the vector tools, Sketch the sketch rail
   and the Sketches and Parameters tabs, Laser the Operations tab. Nothing else switches it, and
   there is nothing to switch by hand. The switcher row above the tab strip is removed (`0093`
   UX notes §1, `0066` criterion 18); the panel header line names the tab, the page and its kind.
3. **Content of an output page.** Three things, all visible in its Layers tab:
   - **Placed items**, live references to drawing content: a *linked clone* of an object or group
     (with own placement on by default, so moving the original on its drawing page never pushes the
     clone out of its holder; 150 A), or a *placed page* (a whole drawing page, live, the
     placement of ADR 0032 §6). Their shape follows the drawing; their position, rotation and scale
     belong to the output page.
   - **Own items**, made on the output page with the normal tools: connecting bars, tabs, text,
     frames. They are ordinary objects of that page.
   - **Fixtures**, the machine setup: grid, holder (board), clamp, stop. They lie in the bottom
     "Fixtures" layer, are selectable by their outline, and run only through their own fixture jobs
     (§6).
4. **Operations.** Each output page has an **ordered list of operations**: a kind (§3), a name, a
   colour, its settings (power and speed come with `0028`), a **Show** switch and an **Output**
   switch. The list order is the run order.
5. **Assignment.** An item belongs to zero, one or several operations (153 A). Assigning is done
   on the output page, never in the drawing, and can reach any depth: the placed page as a whole,
   one of its layers, one group or path inside it, a clone, a group of clone plus connector.
   **Rule: a leaf runs with the operations of the nearest thing above it (itself included) that has
   any.** A leaf with none is *Not assigned*: drawn dimmed, never output (Frage 141 A). The drawing
   is never written.

6. **Run order.** The list order is the run order, always: what you read top to bottom is what the
   machine does. A switch **Cuts last** at the top of the Operations tab keeps the PewPew rule
   (154 B, customer 2026-11-05). It is **on by default** for every new output page and stored per
   output page (Proposal).
   - **Cuts last on:** the list has two sections, **Marking** (Engrave, Engrave + Outline, Score)
     and **Cutting** (Cut, Cut-out), and Cutting always runs after Marking. A new operation lands in
     its section by kind; drag and `Alt+Up/Down` reorder only inside a section, a Cut dropped above
     the section line is refused (no insertion line there, the row snaps back).
   - **Cuts last off:** the section line goes; any operation can be dragged to any place and the
     list order alone is the run order. A new operation lands at the end of the list.
   - **Switched on again:** the cutting operations move below the marking ones, each group keeping
     its relative order, in one undo step "Cuts last on" (Proposal). Switching off moves nothing.
   - Always, whatever the switch: inside a cutting operation the machine runs every part's inner
     shapes before its outer outline (holes before the part falls out), parts in short-travel order
     (computed with the G-code of `0029`, no setting); and the **Fixture jobs** section (§6) stays
     below the work, its Output off by default.
   So with the default nothing is ever engraved on a part that has already dropped out, and the
   maker who wants another order turns one visible switch off instead of fighting an optimiser.

   **Sketch h notes as they read for 154 B** (for the redraw; the sketch itself still shows 154 A):
   (1) the **Cuts last** switch at the top of the list, on; (2) Marking section: every operation
   that does not separate material, new operations land in their section by kind; (3) a Cut dragged
   above the section line while Cuts last is on: no insertion line, the row snaps back, `Alt+Up`
   stops at the line; (4) Cutting section, header "runs last"; (5) inside a Cut or Cut-out, inner
   shapes before the outer outline, then the next part by short travel; (6) Fixture jobs, collapsed;
   (7) a small inset with Cuts last off: one list without a section line, a Cut placed above an
   Engrave, the insertion line shown between them.
7. **Moving versus editing (sketch i).**
   - **Moving** changes where a whole object is: drag, arrow keys, the X, Y, W, H and angle fields,
     scale and rotate handles, align and distribute, delete, copy, paste. It works on every
     selectable object in every context, across pages and onto the pasteboard. The selection box and
     handles are the same everywhere.
   - **Editing** changes what an object is: nodes and segments, path operations (Boolean, Offset,
     Object to path), Style, text content and font, entering a group, drawing into it, sketch
     geometry, constraints and dimensions.
   - **Context** is the active page's kind. Drawing page active: drawing objects on it and on the
     pasteboard are editable (156 A). Laser page active: its own items and fixtures are editable;
     linked clones and placed pages only in their placement and assignments (their shape belongs to
     their source). Sketch page active: its sketch with the sketch tools. Everything else selects and
     moves, and an edit tool shows nothing on it (the Node tool shows no nodes, a Style change does
     not apply to it). A **double-click** on such an object activates its context (its page; for a
     pasteboard object the last drawing page), as a press into a page already does, so editing is
     one gesture away and there is no message to read.
   - **Where an object belongs** stays the position rule of `0059` (PAGE-MODEL A, no stored
     membership). Dropped over a laser page, an object is that page's own item: Not assigned, so it
     never runs by accident, editable while the laser page is active; dragged off again it is a
     drawing object again (155 A). Over a sketch page it only lies there: a sketch holds sketch
     geometry only, so a drawing object over it is never part of it and never output.
   - **What a drag makes** (cursor badge, sketch i): from a drawing page or the pasteboard onto a laser
     page, a linked clone (150 A, the original snaps back); with `Alt` at release, Move; with `Ctrl`,
     Copy. A drag that stays on one page or ends on the pasteboard moves. A sketch dragged anywhere
     leaves a linked clone (it never leaves its page).

**Show vs Output (Frage 142 B).** Show is view only: the operation's eye, and the eye of the output
page's own Layers rows. Output decides what runs: the operation's Output switch. A hidden item in an
operation with Output on still runs; the Operations tab counts "runs while hidden" so it is never a
surprise. What the drawing hides is a drawing matter (149 A: an assignment of a
whole placed page does not reach hidden drawing content; hidden content runs only with an
assignment of its own).

**Look on the output page.** With "View: Operations" (default) every leaf is drawn by its
operation's kind (shape) and the operation's colour (identity): Cut a 1-device-pixel solid line,
Score a dashed line, Engrave a flat fill, Engrave + Outline fill plus line, Cut-out a 2 px line on the
outer contour. Not assigned: muted dashed outline. Several operations on one leaf are drawn in list
order. "View: Drawn" shows the drawing's own style. Tokens `--op-1` to `--op-8` (Proposal values in
the sketches), each at least 3:1 on the page ground; kinds differ by shape, never by colour alone.

## 3. Operation kinds (laser)

| Kind | Geometry it runs | Open paths | Images | Typical use |
|---|---|---|---|---|
| **Cut** | every outline of the leaves, through the material | traced | frame only, not output | parts, holes |
| **Score** | every outline, on the surface (a Cut with surface power, `0028`) | traced | not output | fold lines, outlines on wood |
| **Engrave** | closed areas filled (fill rule of the object); open paths as a line (140 A) | as a line | raster engraved | text, logos, photos |
| **Engrave + Outline** | Engrave, then the outlines traced, in one operation | as a line | raster engraved, frame traced | crisp edges on filled art |
| **Cut-out** | the **outer contour of the assigned item as a whole** (union of all its leaves; holes and inner detail ignored), optional margin (offset, `0038`) | closed by the contour | contour of the frame | stickers, keychains, a composition cut free in one line |

Cut-out is the kind that makes compositions work: two name clones plus a connecting bar, grouped,
one assignment, and the laser cuts one clean line around all three. Its contour is computed at the
node it is assigned to, so it is the one kind where *what* you assign (a group versus its members)
changes the result. Style never matters (as `0027` criterion 7). Guides, dimensions and fixtures
of the drawing never run.
Not in the first set (148 A): Offset fill, Dot mode, Image as a separate kind, Ignore (a kind that
does nothing is the Output switch). **Noted for later (customer, 2026-11-05, option C):** image
optimisation (dithering, resolution, contrast for photos) is needed too and Offset fill is sensible;
both join the Marking section when their story comes, no change to the list or the rule.

Other machines get their own kinds through the same list: CNC (Profile, Pocket, V-carve, Drill),
plotter (Cut, Draw, Kiss-cut), embroidery (Fill, Satin, Run). The list, assignment rule, Show/Output
and fixtures are shared; only the kinds and their settings are per machine.

## 4. Alternatives

**A. Operation list with explicit assignment (recommended).** As §2. One list per output page,
assignment by selection at any depth, nearest assignment wins. Strengths: the overview of LightBurn
without colour abuse, the directness of xTool (select, press a key), works for objects, groups,
clones, placed pages and layers alike, one item may run in several operations, Cut-out has a clear
anchor. Cost: a new idea ("assignment") the maker must see; mitigated by the dimmed Not assigned
look and the per-selection membership in the Operations tab.

**B. Output-page layers are operations (main alternative, sketch e).** The output page has its own
layers; each layer carries a kind, Show and Output. Dropping an item into a layer assigns it.
Strengths: one concept fewer (just layers), familiar from LightBurn but freed from colour, and the
drawing's layers are not involved. Weaknesses: one item lives in exactly one layer, so "engrave and
cut out" needs either a Fill+Outline-like combined kind for every pair or a second clone; a placed
page is one item, so engraving one of its layers and cutting another means placing it twice with a
layer filter; Cut-out of a composition forces the grouping into a layer structure; the Layers tab
has to mean two things on two page kinds.

**C. Per-object processing (xTool style).** No list; the Style-like "Process" block of the selection
sets kind and settings. Strengths: fastest first job. Weaknesses: no run order without a second UI,
no way to switch a whole step off, settings duplicated per object, no overview of what runs. It
fails "see what will run".

**Rejected outright: a node graph** (§1). It meets feedback 2 and 3 but contradicts feedback 6.

**Decided: A** (Frage 147 A, customer 2026-11-05: "deutlich besser"). B and C stay here for the
record only.

## 5. Main flows (recommended concept)

Each flow names its presses. No flow needs a dialog.

1. **Add a laser page.** Document tab, Pages, `Add page` (+): the inline Add group opens under the
   list with a **Kind** switch `Drawing | Sketch | Laser` (machine kinds only when their extension is
   on) and, for Laser,
   the machine profile list (`0026`, last picked preselected). `Add` (Enter). Result: a page "Laser 1"
   of the machine's work area, placed at the next free spot in the plane (0059 rule), active, so the
   Laser workbench shows; one undo step. 3 presses; 2 when the kind was chosen last time (the
   group remembers the last kind, Proposal).
2. **Bring in an object as a linked clone by drag.** On the canvas, drag an object (or a selection,
   or a group) from a drawing page across onto the output page. The original snaps back; a linked
   clone with own placement lands where it was dropped; snapping (0060) and fixture targets apply
   during the drag; the cursor badge says "Linked clone". Ctrl at release: an independent copy
   (Proposal). Alternative without a canvas drag: Copy on the drawing page, then on the output page
   "Paste as linked clone" (Ctrl+Shift+V, Proposal). With `Alt` at release the object itself moves
   onto the laser page and becomes its own item (§2.7).
3. **Place a whole page into a fixture.** Drag a page row from the page list onto the output page (or
   drag the page's frame edge on the canvas with Alt, Proposal). While dragging, the page's frame is
   the ghost; when it nears a holder slot of fitting size (within the snap tolerance, also rotated
   90 degrees), it snaps to the slot's centre and the slot lights in `--accent`. Release: a placed
   page, live. Drop outside any slot: placed where dropped. Rotate and scale on the output page only.
4. **Add a connecting element or text directly.** Use Rectangle, Pen or Text on the output page as
   on any page; the new object is an own item of the output page, Not assigned until assigned.
   Group it with clones (Ctrl+G) to make a composition. Fonts tab and Text bar work as on drawing
   pages.
5. **Assign operations.** Select one or more items (on the canvas or in Layers; inside a placed page:
   double-click into it as into a group, or pick its layer row in Layers). Then either:
   press the kind button in the Operations toolbar (creates "Cut 1" if no Cut exists and assigns,
   one step), press a digit `1` to `9` on the canvas (toggles membership in operation n, Proposal),
   tick the operation in the Selection block, or drag the selection onto an operation row. Every
   assignment is one undo step "Assign to Cut 1".
6. **Reorder operations.** Drag a row by its grip, or `Alt+Up` / `Alt+Down` on the focused row (the
   Layers keys), inside its section while Cuts last is on, anywhere when it is off (§2.6). New
   operations land in their section by kind, at its end, so the common order needs no reorder.
7. **See what will run.** The Operations list shows per row: kind look, name, item count, Show,
   Output. Below the list one summary line: "Runs 3 operations, 18 items." and, only when above
   zero, "2 items not assigned" and "1 item runs while hidden", each a button that selects those
   items. "View: Operations" draws exactly what runs (Not assigned dimmed). Preview and time come
   with `0029` as a Preview control in the same tab.
8. **Back to drawing.** Press inside a drawing page (or its row, or double-click a drawing object,
   §2.7). Edit the original; the clone on the bed follows in the same frame.
9. **Use a sketch on the bed.** Drag the sketch from its sketch page onto the laser page (or its page
   row onto the bed): a linked clone with its own placement lands there, snapping to fixtures. Turn
   it with the rotate handle; assign Cut. Change a dimension on the sketch page; the clone follows.

## 6. Fixtures as objects (sketch g)

- **Kinds (laser):**
  - **Grid**, the engraved bed grid of `0093`, now with **minor lines** (default every 10 mm, 1 px)
    and **major lines** (every 5th line, 50 mm, 2 px, stronger ink), and **measuring tapes** along
    two edges (default bottom and left, the origin corner's edges): ticks every 5 mm, longer every
    10 mm, longest with a number every 50 mm, counted from the origin. Parameters: spacing, major
    every n lines, tape edges, numbers from (origin corner). The tapes are part of what the grid's
    job engraves onto the bed board.
  - **Holder**, a board or jig with one or more **slots** (rectangles, circles or any closed path,
    each with a name, e.g. "Coaster 1"). Covers the customer's slide-in board with engraved coaster
    outlines: the board is the holder, each engraved outline a slot.
  - **Clamp**, a keep-out area, drawn hatched; operations that cross it are counted in the summary
    ("1 operation crosses Clamp 1"), never refused.
  - **Stop**, a fence edge or corner that a board or part is pushed against.
- **Where they live:** in the machine setup (WB-FIXTURE-HOME A): the profile of `0026` gains a
  fixture set; a laser page holds a copy (as ADR 0039 does for the profile) and "Differs from
  profile" works the same way. On the laser page they form the bottom layer "Fixtures".
- **Active, several at once:** each fixture has an **Active** check box in the Document tab's
  Fixtures block ("on the bed now"). Any number may be active (grid, board, two clamps, a stop).
  An inactive fixture is not drawn, not a snap target, and its jobs do not run.
- **Selectable:** a fixture is an ordinary object of the laser page. A press on its outline selects
  it (same box, handles and a "Fixture" tag); a press inside its area does not, so marquees and
  presses on the parts lying on a board never catch the board (the rule of closed guides, `0046`).
  It is also selected from its row in the Fixtures block or in Layers. Moving it moves its slots;
  node-editing a holder outline or drawing a slot are edits in the laser page's context.
- **Snap points** (Snapping target "Fixtures", on by default on laser pages): grid intersections;
  slot centre, corners and edge midpoints; **fit**: an item whose bounds match a slot's size within
  0.5 mm (Proposal), also turned by 90 degrees, snaps centre to centre and is rotated to fit; stop
  edges and corners.
- **Fixture jobs:** a fixture can carry its own operations (the customer's board: "Board outlines"
  engraved the four coaster outlines once; the grid: "Bed grid"). They are listed in the **Fixture
  jobs** section of the Operations tab, below Cutting, with Output **off** by default; turning one
  on runs it with the next job and the summary counts it ("1 fixture job runs"). They are stored
  with the fixture, so they travel with the machine setup. A fixture can be in fixture jobs only,
  and a fixture job takes fixtures only (Proposal), so a bed grid never runs with the coasters by a
  stray assignment.
- **Never output by accident:** no work operation takes a fixture, fixture jobs start with Output
  off, and an inactive fixture never runs.

## 7. Panels, tabs and tools on an output page

| Element | On a drawing page | On an output page |
|---|---|---|
| Workbench switcher | **removed** (no row above the tab strip on any page kind) | **removed** |
| Kind shown | panel header "Document · Keychain · Drawing", page label, page row, status bar | the same with "Laser"; on a sketch page "Sketch" |
| Tool rail | all vector tools | all vector tools, Text included; no extra tool. A sketch page shows the sketch rail of `0066` (geometry card, constraint card) |
| File, History | yes | yes (project-wide) |
| Document | page list, page size, background, grid of the page | **page list (the same list)**, then Machine block (profile picker, work area as read-only text, origin corner, `0026`), then Fixtures block (Active per fixture, parameters in place) |
| **Operations** (new, machine glyph) | absent | the operation list, toolbar of kinds, Selection block, summary; first tab after Document |
| Style | yes | yes for own items (their look with View: Drawn); clones show the clone block (`0049`) |
| Layers | the drawing's layers | **the output page's own tree**: placed items (expandable into the source, read-only rows with operation chips), own items, Fixtures layer |
| Fonts | yes | yes (Text) |
| Snapping | yes | yes, plus the target kind Fixtures |
| Grid | yes | yes (view grid of `0090`; not the engraved grid fixture) |
| Share | yes | yes (the output page is part of the document) |
| Sketches, Parameters | on sketch pages only | absent |
| Status bar right block | "Drawing · 125.0 x 75.0 mm" | "Laser · K40 in the garage · 400.0 x 300.0 mm" |
| Canvas | pages in the plane | the same plane; the output page frame carries a page label with the machine glyph and the origin marker (`0026` criterion 11) |

Tab order with an output page active: File, Document, Operations, Style, Layers, History, Fonts,
Snapping, Grid, Share (Proposal; the order never changes, absent tabs are not built). Leaving to a
drawing page while Operations is open shows the last drawing-page tab (the `0093` rule).

## 8. What of 0093, 0026 and 0027 survives

| Built | Survives | Changes |
|---|---|---|
| `0093` manufacturing node + owned page (ADR 0032 §4, §5; ADR 0033) | the node, its owned page, `mfg_process`, unknown processes kept, the extension registry and `ManifestWorkbench` | the owned page **is listed** in the page list and drawn in the plane (WB-PAGE-VIS A reversed by feedback 5); "Add manufacturing" becomes Add page with Kind Laser; delete manufacturing = delete the page |
| `0093` placement (`placement_link`) | the live page reference, unchanged in the file | becomes user-made by drag, movable, snappable; it can carry assignments into its source |
| `0093` read-only output page | nothing | the output page is fully editable; only the *inside* of a placed item is read-only (double-click opens the original) |
| `0093` engraved grid fixture | the kind and its spacing | gains major lines, measuring tapes, an Active switch and a fixture job; one fixture kind of four, with snap points |
| `0093` workbench switcher (and `0066` criterion 18, the Vector / Sketch selector) | the workbench registry and manifests (they still name each kind's tools and tabs) | the switcher row is **removed**; the active page's kind selects the workbench; the kind chip replaces the pressed button as the mode cue |
| `0066` Sketch workbench, page sketch (one owning Sketch object per page) | the sketch model, solver, tools, rail, Sketches and Parameters tabs, all looks | a page *is* a Sketch page instead of a drawing page with a sketch mode; a drawing page has no sketch. Files with a sketch on a drawing page open with the sketch moved to a new Sketch page beside it and a linked clone in its old place, so the drawing looks the same (Proposal; old files need not load, `0059`) |
| `0026` profiles, library, copy in the file (ADR 0038, 0039), origin marker | all of it | the Machine block moves from the Laser tab to the Document tab of the laser page; the work area is read-only text there; the profile gains a fixture set with fixture jobs (later slice) |
| `0027` (PR #269, not merged) roles keyed by drawing layer (the withdrawn laser-roles ADR of the closed PR #269) | the geometry rules (style ignored, 140 A open/closed rule, images only engrave), the role-look idea and its tokens, the notice counts, the CPU budgets | roles become **operations** with assignment on output-page nodes; storage is not keyed by drawing layer; criterion 14 (hidden not output) is replaced by 142 B. #269 is closed unmerged (152 A) |
| 0049 linked clone | the mechanism, chains, own placement (0057) | clones of **groups** are needed (0049 criterion 4 forbids them today; ADR needed); on an output page own placement defaults to on |

Architect work implied (not decided here): an ADR for operations and assignments in the file (keyed
by output-page node and, inside a placed item, by source node id; per-register merge as the withdrawn laser-roles ADR of the closed PR #269
required in its §1), an ADR amendment to 0032 §5 (owned page listed and in the plane), group clones
(amends ADR 0015), the fixture set with fixture jobs in the profile record (ADR 0038 bump), the page
kind in the pages register (amends the `0059` ADR: `kind` plus, for Sketch, the owned sketch; a
`format_version` bump), and the run-order sections (derived from the kind, not stored).

## 9. Slices this concept becomes (numbers reserved 2026-11-05)

1. **`0098` Page kinds, no switcher:** the page kind in the file, Add page with Kind (Drawing,
   Sketch, Laser), kind chips everywhere a page name appears, the tools and tabs follow the active
   page, the switcher row removed; laser page listed, editable with all tools, Machine block in
   Document with the work area read-only. Replaces Add manufacturing. A file with a sketch on a
   drawing page converts here (moved from slice 2, so no sketch becomes unreachable when the
   switcher goes).
2. **`0099` Sketch pages:** a sketch page holds its sketch only, the sketch is fixed to its page,
   dragging it off leaves a linked clone, moving the page moves the sketch. Can run in parallel
   with 3.
3. **`0100` Bring content in, move versus edit:** drag onto the laser page makes linked clones (own
   placement), Alt moves, Ctrl copies; page rows dragged become placed pages; group clones; the edit
   context rule and the double-click to activate; the output page's own Layers tree.
4. **`0101` Operations:** list, five kinds, assignment at any depth, Show/Output, looks, summary,
   the Cuts last switch with the Marking and Cutting sections. Replaces `0027`.
5. Fixtures, split in two so each part is tried on its own:
   - **`0102` Fixtures as objects:** grid with tapes and major lines, Holder (boards with slots),
     Clamp, Stop; Active per fixture; fixtures selectable; fixture snap targets and fit snap.
   - **`0103` Fixture jobs:** operations of fixtures in the Fixture jobs section, Output off by
     default, stored with the machine setup.
6. Then `0028` (settings per operation from the material library) and `0029` (preview, gate, G-code
   reading the operation list in order, inner-first and travel order inside cutting operations).
   They keep their reserved numbers.

## 10. Questions for the customer

### Answered (customer, 2026-11-05)

| Frage | Answer | Recorded as |
|---|---|---|
| 147 (OP-ATTACH) | **A** | an ordered operation list per output page with explicit assignment at any depth; nearest assignment wins (§2.4, §2.5) |
| 148 (OP-KINDS) | **A now**, C noted for later | Cut, Score, Engrave, Engrave + Outline, Cut-out. Later: image optimisation is needed too, and Offset fill is sensible (§3) |
| 149 (HIDDEN-INHERIT) | **A** | hidden drawing content runs only with an assignment of its own |
| 150 (DROP-LINK) | **A, default, not answered** | a drag onto the laser page makes a linked clone with its own position |
| 151 (SAME-PLANE) | **A** | all page kinds in one plane |
| 152 (PR-269) | **A** | PR #269 is closed unmerged; its geometry rules, look tokens and tests move into the operations story |
| 153 (MULTI-OP) | **A** | one item may be in any number of operations; they run in list order |
| 154 (RUN-ORDER) | **B** (not the recommended A) | the Marking/Cutting rule as a switch **Cuts last** in the Operations tab, on by default; off allows any order (§2.6) |
| 155 (DROP-ON-LASER) | **A** | a drawing object moved onto a laser page is that page's own item by position, Not assigned, editable while the laser page is active; dragged off, a drawing object again (§2.7) |
| 156 (PASTEBOARD-EDIT) | **A** | pasteboard objects are editable while a drawing page is active; with a laser or sketch page active they only move; a double-click switches to the last drawing page (§2.7) |

No question of this concept is open.

## Links

Epic: `specification.md` (this folder). First slice: `specs/0098-page-kinds/`. Superseded in direction: `specs/0093-laser-workbench-shell/`
(criteria 2, 4, 7 and WB-PAGE-VIS), `specs/0027-manufacturing-roles/` (branch, PR #269).
Kept: `specs/0026-machine-profile/`, ADR 0033, 0034, 0038, 0039. Amended in direction: ADR 0032 §4 to
§6, the withdrawn laser-roles ADR of the closed PR #269 (replaced). Related: `0049`, `0057`, `0059`, `0060`, `0090`, `0092`, `0095`, `0074`.

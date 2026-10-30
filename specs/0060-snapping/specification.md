# Snapping: document settings, object edges, guides, nodes and page fractions

Status: In review (PR #225; 2026-10-30: temporary inversion, per-axis settings and creation hover/click feedback implemented and independently verified; customer acceptance pending)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "A fairly complex feature that you switch on and off in the Select tool and configure in properties. Snap to the bounding box of other objects, to guide lines, to nodes. A small zone close by where it is magnetically pulled, and a somewhat larger one to let go again (hysteresis)." Guide lines are paths marked as guides (the rework of `0046`).
Amendment: Customer (2026-10-29): "snapping ist eine property vom document. eine page hat snapping punkte bei 25%, 33.3% 50% 66.6% 75% 100% sowohl horizental als auch vertikal". Snapping is therefore stored in the document, not per user, and the page contributes six fractions on each axis.
Amendment: Customer (2026-10-30): snapping has its own entry in the right Properties rail. A normal click opens its settings; Shift+click toggles snapping, with a subtle blue enabled indication. The customer also confirmed that Curvyo is unreleased (SID): old-file compatibility and migration are not required.
Amendment: Customer feedback on PR #225 (2026-10-30): holding Alt temporarily inverts snapping during an already-running Select object move (off becomes on, on becomes off); horizontal and vertical snapping can be disabled independently in the panel; drawing tools snap during hover before the first press and when creating with a simple click.

## User value

As a maker I want a moved, scaled or drawn thing to jump onto the edge, centre or corner of another object, onto a guide, onto a node, or onto one of the page's fractions, and to stay there until I deliberately pull away, so that parts line up exactly without typing numbers and without the jitter of a snap that switches on and off at one pixel. The setting belongs to the drawing: a file prepared for precise work opens with the same snapping state on every machine.

**What other tools do.** Inkscape introduced a simple/advanced snap popover and global preferences in 1.2; it also offers alignment and distribution snapping. VectorCraft combines snapping options in its View menu and Properties panel and separates guide/label display from snapping. Curvyo deliberately keeps document-owned settings, fixed page fractions, explicit capture/release zones and markers without labels. These are Curvyo requirements, not claims of matching another editor's complete target set. See `research.md` for official sources and the comparison's limits.

## Acceptance criteria

1. Given a document, then **snapping is a property of the document**, not of the user or the session: one switch for on/off, one switch per target kind, independent horizontal/X and vertical/Y switches, and the two zone sizes are stored in the document's root registers and travel with the file. A new document has snapping off, all target kinds on, both axes on, capture 8 px and release 14 px. Opening a file restores its stored values; no `localStorage` or other per-user store is involved.
2. Given any active tool and selection state, then the right Properties rail has a dedicated **Snapping** entry (icon, tooltip and accessible name). A normal click opens the Snapping panel without changing the document's enabled switch. Shift+click toggles the switch without changing the open panel, as one history step labelled like the other document settings. The entry is subtly blue while snapping is enabled; panel selection and snapping enabled are independent states. The panel also has an explicit, keyboard-accessible enabled switch. When snapping is effectively off under criterion 9, no target pulls and no marker shows.
3. Given snapping effectively on under criterion 9 and pointer movement (during a drag or the creation hover of criterion 17), then a moving feature is pulled onto a **target** when it comes within the **capture zone**; while it stays inside the **release zone** around the same target it stays on it; beyond the release zone it moves freely again. The release zone is never smaller than the capture zone (a stored smaller value is read as the capture size). Both are measured in **screen pixels**, so they feel the same at every zoom.
4. Given the release zone, then it is measured from the **raw** pointer-derived position to the target, not from the snapped position, so a slow drag across a target neither sticks forever nor flickers.
5. Given targets, then they are:
   - **object boxes**: the four edges, the centre lines and the nine reference points of the bounding box of every other visible object; for a group, its box; for a linked clone, its own box.
   - **guides**: the lines and anchor points of every visible path marked as a guide (`0046`).
   - **nodes**: the anchor points of every other visible path, and of the path the Node tool edits.
   - **page fractions** (amendment of 2026-10-29): six vertical lines at the page width times 25 %, one third, 50 %, two thirds, 75 % and 100 %, and six horizontal lines at the page height times the same fractions. The thirds are exact fractions, not rounded 0.333/0.666. Each page origin offsets its lines into document coordinates. Pending SNAP-PAGES, all visible pages contribute targets by default, not just the active page. There is no 0 % line, no grid and no page centre line of its own (the 50 % line is the centre).
   Each kind has its own switch in the settings; a kind switched off contributes no target. Hidden objects are no targets; locked objects are (they are often the reference). The object being dragged contributes no target.
6. Given several targets within the capture zone, then the nearest wins; on an exact tie, nodes before guides before object boxes before page fractions. Along one axis an edge may snap and along the other a guide may snap at the same time. With both axes enabled and unconstrained, a node point within the capture zone snaps both axes at once; it wins both axes when its displacement is not larger than the nearest line-only displacement. Per-axis settings and tool constraints take precedence as specified in criterion 16.
7. Given where it applies, then: move and resize with the Select tool (a move snaps the selection's box by its edges, centre lines and reference points; a resize snaps the moving edges and the dragged reference point), node and handle drag in the Node tool (anchors snap to every target; handles snap to nodes only), creation hover, creation drags and simple-click creation of the Rectangle, Ellipse and Polygon/Star tools, and the placed nodes of the Pen including hover before its first click (criteria 17 and 18). Rotation keeps its 22.5° steps (`0008`). A move with Shift axis lock (`0010`) snaps along the free axis only.
8. Given a snap, then the canvas shows what it snapped to during a drag or an active creation-hover snap: a line along the matched edge, guide or page fraction and a small cross at the matched moving point, in one colour from the design system, thin and at constant screen width. No text label and no tooltip. The drag marker disappears on release, on Escape and when the snap releases. A subsequent creation-hover marker appears only for a currently resolved hover snap (criterion 17).
9. Given an **already-running Select object move**, then holding **Alt** temporarily **inverts** the document's snapping-enabled state: document off means effectively on while held, and document on means effectively off while held. Releasing Alt immediately restores the document state. Modifier changes apply to the current raw pointer-derived move position without needing another press or committing geometry. This override changes neither stored settings nor history and keeps the document's target-kind and axis settings. This customer decision supersedes the earlier Ctrl+Alt-off Proposal as the requested move interaction. During that move the effective state is stored-enabled XOR Alt, including when Ctrl is also held; Ctrl+Alt must not create a copy. Other contexts retain their existing modifier behaviour. Alt at the initial press retains its existing interaction; this inversion does not apply to resize, Node-tool interactions, other tools or creation hover, which use the stored enabled state with their existing modifier behaviour.
10. Given the settings, then they live in the dedicated **Snapping** Properties panel: the enabled switch, target switches (Objects, Guides, Nodes, Page), independent horizontal/X and vertical/Y switches, and capture and release fields in px. These controls are available with every tool and selection state and are not disabled with snapping off. A value below the capture size is raised to it with the field showing the stored result; a non-finite or negative value is refused.
11. Given a drawing of 10,000 objects, then dragging with snapping on retains the responsiveness of dragging without it: targets come from a spatial index built from the visible session read before the drag, with bounded queries around moving features rather than scanning every target each frame. No document read is repeated per frame. The target search must not silently discard nearby targets because of document order. The frame budget is a drag without snapping plus at most 2 ms of thread CPU on the reference machine.
12. Given the result of a snapped move or resize, then it is exactly the target coordinate (to the kernel tolerance) and one history step as before; the snap state and the marker are ephemeral and never stored.
13. Given a linked clone (`0049`), then its box and nodes are targets as any object; a clone being dragged snaps by its own box.
14. Given an open file with no snapping registers, then it opens with the defaults (off, all kinds on, 8/14 px) and is not rewritten by opening. A file with a malformed register is refused as `OpenError::Damaged`, as for the background.
15. Given a page is resized or moved, then its page fraction lines follow its new size and origin in the same frame; nothing about the fractions is stored. A hidden page contributes no target. The choice of active page does not limit targets under the default for SNAP-PAGES.

16. Given the document's independent axis switches, then a new document enables both, and either can be disabled, saved, reopened and undone like the other document snapping settings. **(Proposal: labels)** Horizontal (X) controls corrections to the moving point's X coordinate; Vertical (Y) controls corrections to its Y coordinate, not the orientation of the displayed target line. When an axis is off, snapping leaves that raw coordinate unchanged, including for node and guide-point targets: no point snap, hysteresis or marker may introduce a correction on a disabled axis. With one axis enabled, snapping resolves only that permitted coordinate. With both off, neither coordinate snaps and no marker shows, including during a Select move with Alt inversion. Existing tool constraints still take precedence: an axis-locked move snaps only on an axis both free and enabled; constrained resize and shape creation retain their existing geometric constraints.
17. Given Rectangle, Ellipse, Polygon/Star or Pen is active and no creation drag has started, then moving the pointer already resolves the next placement against enabled targets using the document's stored snapping-enabled state and the creation tool's existing modifiers; Select-move Alt inversion does not apply. A valid hover snap shows the same marker as a drag, before any press and including before the Pen's first click; leaving capture/release range, moving off the canvas, Escape or changing tools clears the hover state and marker. Hover never edits the document or adds history. Targets and enabled settings reflect the current document, and hover queries use the same indexed candidate search as dragging.
18. Given a valid creation-hover snap, then the next press/placement click uses that snapped document coordinate, including the first Pen node and simple-click creation with Rectangle, Ellipse or Polygon/Star. The existing simple-click creation workflow, dimensions and placement reference remain unchanged except for the corrected placement coordinate. A press without a preceding hover event still resolves its own raw coordinate. A creation drag starts from the resolved press position and continues snapping under criterion 7; a later release or placement uses the current effective state and axis settings. The resulting creation remains one normal creation history step; preview snapping adds none.

## Out of scope

- Snapping to a grid (there is no grid; own entry if wanted), to path intersections, to tangent and perpendicular points (Inkscape's), to text baselines, to stroke-centre lines and to object centres other than the box centre.
- Live distance and spacing guides between objects (smart-guide measuring).
- Creating guides (`0046` rework); this spec only snaps to them.
- Angle snapping for rotation (exists, `0008`).
- A per-page or per-document editable list of fractions: the six values above are fixed by the customer's amendment. A later spec may make them editable.
- Backwards compatibility or migration for previously saved unreleased files. Missing snapping registers still resolve to the defaults in criterion 14; opening must not mutate a document.

## Architect notes (2026-10-17, superseded 2026-10-29)

The earlier note "No document or format change" is **superseded**: the customer's amendment makes snapping a document property, so `format_version` goes up and the settings are root registers (`adrs.md`, decisions 1 to 3). The pure-function shape, the tool-session snap state and the use of the existing read pass and true-box function stay.

## Open questions (customer; each has a default)

Frage 1 (SNAP-DEFAULT): Is snapping on or off when a new document is created?
- A (default): off; nothing pulls the pointer until asked. The state is stored in the document from the first press of the toggle.
- B: on.
- Default if unanswered: A.

Frage 2 (SNAP-ZONES): Are the zone sizes fixed or settable?
- A (default): both settable in the Snapping Properties panel, defaults 8 and 14 px; the release zone cannot be smaller than the capture zone.
- B: fixed values, no fields.
- Default if unanswered: A.

SNAP-HOME — decided by the customer (2026-10-30): dedicated Snapping entry in the right Properties rail; normal click opens its panel, Shift+click toggles enabled, subtle blue indicates enabled. No separate Select-toolbar snapping control is required.

SNAP-OFF-KEY — superseded by the customer (2026-10-30): Alt temporarily inverts the stored enabled state during an already-running Select object move. It neither changes document state nor adds a history step; releasing restores the stored state. During that move inversion also applies with Ctrl held, without creating a copy. Initial-press behaviour and other tools retain their existing modifiers; creation hover uses the stored toggle.

SNAP-AXIS-LABELS — **(Proposal)**, pending customer feedback: use Horizontal (X) and Vertical (Y), referring to the corrected movement coordinate. Default if unanswered: these labels. The independent switches themselves are customer-required, not a Proposal.

Frage 5 (SNAP-PAGES): Which pages contribute snapping fractions?
- A (recommended/default): all visible pages, with each page's position and size applied.
- B: only the active visible page.
- Default while awaiting the customer's answer: A. This changes target scope only, not document-owned settings or the fixed fractions.

## Links

Requirements: R-EDIT-039, R-EDIT-041 (guides)
Depends on: `specs/0004-canvas-navigation-and-selection/`, `specs/0019-multi-object-transform/`
Related: `specs/0046-document-shapes/` (guides), `specs/0008-object-transform-refinements/`, `specs/0010-edit-interaction-polish/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0059-multiple-pages/`, `specs/0063-manufacturing-workbenches/` (placement snaps by fitting size)
ADRs: none in `docs/adr/`; feature-local decisions in `adrs.md` (document registers, `format_version` 27, pure engine, marker)
PR: #225

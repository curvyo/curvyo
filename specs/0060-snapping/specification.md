# Snapping: document settings, object edges, guides, nodes and page fractions

Status: Ready (2026-10-29: criteria, `adrs.md`, `ux-notes.md` and `plan.md` exist; open questions have defaults)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "A fairly complex feature that you switch on and off in the Select tool and configure in properties. Snap to the bounding box of other objects, to guide lines, to nodes. A small zone close by where it is magnetically pulled, and a somewhat larger one to let go again (hysteresis)." Guide lines are paths marked as guides (the rework of `0046`).
Amendment: Customer (2026-10-29): "snapping ist eine property vom document. eine page hat snapping punkte bei 25%, 33.3% 50% 66.6% 75% 100% sowohl horizental als auch vertikal". Snapping is therefore stored in the document, not per user, and the page contributes six fractions on each axis.

## User value

As a maker I want a moved, scaled or drawn thing to jump onto the edge, centre or corner of another object, onto a guide, onto a node, or onto one of the page's fractions, and to stay there until I deliberately pull away, so that parts line up exactly without typing numbers and without the jitter of a snap that switches on and off at one pixel. The setting belongs to the drawing: a file prepared for precise work opens with the same snapping state on every machine.

**What other tools do.** Inkscape: a snapping toolbar with many toggles (bounding box corners, edges, midpoints, nodes, guides, page, grid), a dialog with a tolerance in pixels; it snaps while dragging and shows the target as a tooltip-like label. Illustrator and Affinity: "smart guides", always on, magenta lines and a hint label. LightBurn: snap to grid and to shape points. Our version: the same targets as Inkscape, one switch, one place for the settings, a frozen set of page fractions, and a marker without text.

## Acceptance criteria

1. Given a document, then **snapping is a property of the document**, not of the user or the session: one switch for on/off, one switch per target kind, and the two zone sizes are stored in the document's root registers and travel with the file. A new document has snapping off, all target kinds on, capture 8 px and release 14 px. Opening a file restores its stored values; no `localStorage` or other per-user store is involved.
2. Given the Select tool, then its bar has one **Snapping** toggle (icon, tooltip, accessible name, pressed state) in every selection state. It reads and writes the document's switch as one history step labelled like the other document settings. With snapping off no target pulls and no marker shows.
3. Given snapping on and a drag, then a moving feature is pulled onto a **target** when it comes within the **capture zone**; while it stays inside the **release zone** around the same target it stays on it; beyond the release zone it moves freely again. The release zone is never smaller than the capture zone (a stored smaller value is read as the capture size). Both are measured in **screen pixels**, so they feel the same at every zoom.
4. Given the release zone, then it is measured from the **raw** pointer-derived position to the target, not from the snapped position, so a slow drag across a target neither sticks forever nor flickers.
5. Given targets, then they are:
   - **object boxes**: the four edges, the centre lines and the nine reference points of the bounding box of every other visible object; for a group, its box; for a linked clone, its own box.
   - **guides**: the lines and anchor points of every visible path marked as a guide (`0046`).
   - **nodes**: the anchor points of every other visible path, and of the path the Node tool edits.
   - **page fractions** (amendment of 2026-10-29): six vertical lines at the page width times 25 %, 33.3 %, 50 %, 66.6 %, 75 % and 100 %, and six horizontal lines at the page height times the same fractions. There is no 0 % line, no grid and no page centre line of its own (the 50 % line is the centre).
   Each kind has its own switch in the settings; a kind switched off contributes no target. Hidden objects are no targets; locked objects are (they are often the reference). The object being dragged contributes no target.
6. Given several targets within the capture zone, then the nearest wins; on an exact tie, nodes before guides before object boxes before page fractions. Along one axis an edge may snap and along the other a guide may snap at the same time. A node point within the capture zone snaps both axes at once; it wins both axes when its displacement is not larger than the nearest line-only displacement.
7. Given where it applies, then: move and resize with the Select tool (a move snaps the selection's box by its edges, centre lines and reference points; a resize snaps the moving edges and the dragged reference point), node and handle drag in the Node tool (anchors snap to every target; handles snap to nodes only), creation drags of the Rectangle, Ellipse and Polygon/Star tools, and the placed nodes of the Pen including its first click. Rotation keeps its 22.5° steps (`0008`). A move with Shift axis lock (`0010`) snaps along the free axis only.
8. Given a snap, then the canvas shows what it snapped to while the drag runs: a line along the matched edge, guide or page fraction and a small cross at the matched moving point, in one colour from the design system, thin and at constant screen width. No text label and no tooltip. The marker disappears on release, on Escape and when the snap releases.
9. Given a modifier held (Ctrl+Alt, checked against the table of R-SYS-009), then snapping is off for as long as it is held, whatever the toggle says. The key is a Proposal and may be vetoed at the demo.
10. Given the settings, then the Document tab has a **Snapping** block below the size controls and Fit to content, above the Background block: the target switches (Objects, Guides, Nodes, Page), the capture and release fields in px, and nothing else. The controls are always there and are not disabled with snapping off. A value below the capture size is raised to it with the field showing the stored result; a non-finite or negative value is refused.
11. Given a drawing of 10,000 objects, then a drag with snapping on does not read the document again per frame: the target list is built once when the drag starts from the one visible read the session already holds, and each frame scans that list. The frame budget is the same as a drag without snapping plus at most 2 ms of thread CPU on the reference machine.
12. Given the result of a snapped move or resize, then it is exactly the target coordinate (to the kernel tolerance) and one history step as before; the snap state and the marker are ephemeral and never stored.
13. Given a linked clone (`0049`), then its box and nodes are targets as any object; a clone being dragged snaps by its own box.
14. Given an open file with no snapping registers, then it opens with the defaults (off, all kinds on, 8/14 px) and is not rewritten by opening. A file with a malformed register is refused as `OpenError::Damaged`, as for the background.
15. Given the page size changes, then the page fraction lines follow the new size in the same frame; nothing about the fractions is stored.

## Out of scope

- Snapping to a grid (there is no grid; own entry if wanted), to path intersections, to tangent and perpendicular points (Inkscape's), to text baselines, to stroke-centre lines and to object centres other than the box centre.
- Live distance and spacing guides between objects (smart-guide measuring).
- Creating guides (`0046` rework); this spec only snaps to them.
- Angle snapping for rotation (exists, `0008`).
- A per-page or per-document editable list of fractions: the six values above are fixed by the customer's amendment. A later spec may make them editable.
- A snapping section in a future Settings tab.

## Architect notes (2026-10-17, superseded 2026-10-29)

The earlier note "No document or format change" is **superseded**: the customer's amendment makes snapping a document property, so `format_version` goes up and the settings are root registers (`adrs.md`, decisions 1 to 3). The pure-function shape, the tool-session snap state and the use of the existing read pass and true-box function stay.

## Open questions (customer; each has a default)

Frage 1 (SNAP-DEFAULT): Is snapping on or off when a new document is created?
- A (default): off; nothing pulls the pointer until asked. The state is stored in the document from the first press of the toggle.
- B: on.
- Default if unanswered: A.

Frage 2 (SNAP-ZONES): Are the zone sizes fixed or settable?
- A (default): both settable in the Document tab, defaults 8 and 14 px; the release zone cannot be smaller than the capture zone.
- B: fixed values, no fields.
- Default if unanswered: A.

Frage 3 (SNAP-HOME): Where do the settings live?
- A (default): a block in the Document tab (the existing home of view-like document settings such as the background).
- B: a new Settings tab in the sidebar strip (`0051`).
- Default if unanswered: A.

Frage 4 (SNAP-OFF-KEY): Which modifier turns snapping off while held?
- A (default): Ctrl+Alt. Neither key alone is free in every tool (Shift locks, Ctrl copies or snaps an angle), and the combination is easy to reach.
- B: Alt alone.
- C: no modifier.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-039, R-EDIT-041 (guides)
Depends on: `specs/0004-canvas-navigation-and-selection/`, `specs/0019-multi-object-transform/`
Related: `specs/0046-document-shapes/` (guides), `specs/0008-object-transform-refinements/`, `specs/0010-edit-interaction-polish/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0059-multiple-pages/`, `specs/0063-manufacturing-workbenches/` (placement snaps by fitting size)
ADRs: none in `docs/adr/`; feature-local decisions in `adrs.md` (document registers, `format_version` 26, pure engine, marker)
PR: -

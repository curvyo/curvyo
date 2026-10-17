# Snapping: bounding boxes, guides and nodes pull the pointer with a small zone in and a larger zone out

Status: Draft (a sketch for the customer; not Ready: needs UX notes and the customer's answers)
Priority: Should
Origin: Customer (request of 2026-10-17, translated): "A fairly complex feature that you switch on and off in the Select tool and configure in properties. Snap to the bounding box of other objects, to guide lines, to nodes. A small zone close by where it is magnetically pulled, and a somewhat larger one to let go again (hysteresis)." Guide lines are paths marked as guides (the rework of `0046`).

## User value

As a maker I want a moved, scaled or drawn thing to jump onto the edge, centre or corner of another object, onto a guide, or onto a node, and to stay there until I deliberately pull away, so that parts line up exactly without typing numbers, and without the jitter of a snap that switches on and off at one pixel.

**What other tools do.** Inkscape: a snapping toolbar with many toggles (bounding box corners, edges, midpoints, nodes, guides, page, grid), a dialog with a tolerance in pixels; it snaps while dragging and shows the target as a tooltip-like label. Illustrator and Affinity: "smart guides", always on, magenta lines and a hint label. LightBurn: snap to grid and to shape points. Our version: the same targets as Inkscape, one switch, one place for the settings, and a marker without text.

## Acceptance criteria (as far as clear)

1. Given the Select tool, then its bar has one **Snapping** toggle button (symbol, tooltip, accessible name, pressed state). The setting is per user, kept between sessions, and applies to the tools of criterion 6. The default is off (Question 1).
2. Given snapping on and a drag, then the pointer-derived position is pulled onto a **target** when it comes within the **capture zone**; while it stays inside the **release zone** around the same target it stays on it; beyond the release zone it moves freely again. The release zone is larger than the capture zone (hysteresis). Both are measured in screen pixels, so they feel the same at every zoom. Defaults: capture 8 px, release 14 px (Question 2).
3. Given the release zone, then it is measured from the **raw** pointer position to the target, not from the snapped position, so a slow drag across a target neither sticks forever nor flickers.
4. Given targets, then they are: the **bounding box** of every other visible object (four edges, four corners, the centre; for a group, the group's box), **guides** (paths marked as guide, `0046`; their lines and nodes), and **nodes** (anchors of the other visible paths and of the Node tool's path itself). Each kind can be switched off in the settings. Hidden objects are no targets; locked objects are (they are often the reference).
5. Given several targets within the capture zone, then the nearest wins; on a tie, nodes before guides before boxes (Proposal). Along one axis an edge may snap and along the other a guide may snap at the same time; a node snaps both axes at once.
6. Given where it applies, then in the first slice: move and resize with the Select tool (the moving box snaps by its edges, corners and centre, not by the pointer), node and handle drag in the Node tool (anchors snap to targets; handles to nodes only), creation drags of the shape tools and the first clicks of the Pen. Rotation keeps its 22.5° steps (`0008`); a move with Shift axis lock (`0010`) snaps along the free axis only.
7. Given a snap, then the canvas shows what it snapped to: a line along the matched edge or guide and a small square or cross at the matched point, in a colour from the design system, thin and constant screen width. No text label, no tooltip (customer rule: no hint labels by default).
8. Given a modifier held (a key to be checked against the table of R-SYS-009), then snapping is off for as long as it is held, whatever the toggle says (Proposal; Inkscape uses the same idea).
9. Given the settings, then they live in the Properties panel where the maker finds the rest of the view and tool settings, as controls that are always there (no section appears for an operation): the three target switches, capture and release zone as value fields (`0017`), and nothing else (Question 3).
10. Given a drawing of 10,000 objects, then dragging with snapping on keeps the frame rate of dragging without it: targets come from a spatial index of the visible region, not a scan of all nodes (budget in the plan).
11. Given the result of a snapped move, then it is exactly the target coordinate (to the tolerance of the kernel), and one history step as before; the snapping itself is not stored in the document.
12. Given a linked clone (`0049`), then its box and nodes are targets as any object; a clone being dragged snaps by its own box.

## Out of scope

- Snapping to a grid (there is no grid; own entry if wanted), to the page edge and page centre (Proposal as a fourth target switch once `0059` pages exist), to intersections of paths, to tangent and perpendicular points (Inkscape's), to text baselines.
- Live distance and spacing guides between objects (smart-guide measuring).
- Creating guides (`0046` rework); this spec only snaps to them.
- Angle snapping for rotation (exists, `0008`).

## Architect notes (2026-10-17)

- The snapping function is pure: `(raw position, previous snap state, targets, zones) -> (position, new state, marker)`, in `ui-core`, testable without a canvas. The state is the one thing that makes hysteresis; keep it in the tool session, not in the document.
- No document or format change. Targets come from the read pass (`NodeRead`), bounding boxes from the existing true-box function.
- If `0046`'s guide rework is not built, the guide target is simply absent.

## Open questions (customer; each has a default)

Frage 1 (SNAP-DEFAULT): Is snapping on or off when the maker first starts?
- A (default): off; nothing pulls the pointer until asked.
- B: on, with the toggle always visible in the Select bar.
- Default if unanswered: A.

Frage 2 (SNAP-ZONES): Are the zone sizes fixed or settable?
- A (default, recommended): both settable in the settings (criterion 9), defaults 8 and 14 px; the release zone cannot be smaller than the capture zone.
- B: fixed values, no settings (less to build, nothing to explain).
- Default if unanswered: A.

Frage 3 (SNAP-HOME): Where do the settings live?
- A (default): a block in the Document tab (the existing home of view-like settings such as the rulers).
- B: a new Settings tab in the sidebar strip (`0051`), which other future settings (units, shortcuts) could use.
- Default if unanswered: A.

## Links

Requirements: R-EDIT-039 (new), R-EDIT-041 (guides, new)
Depends on: `specs/0004-canvas-navigation-and-selection/`, `specs/0019-multi-object-transform/`
Related: `specs/0046-document-shapes/` (guides, being reworked), `specs/0008-object-transform-refinements/`, `specs/0010-edit-interaction-polish/`, `specs/0049-linked-clone/`, `specs/0050-layers-and-objects-panel/`, `specs/0059-multiple-pages/`, `specs/0063-manufacturing-workbenches/` (placement snaps by fitting size)
ADRs: none expected (no format change, no new crate)
PR: -

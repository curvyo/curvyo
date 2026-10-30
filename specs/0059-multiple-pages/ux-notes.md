# Multiple pages — UX notes

Status: UX implementation review APPROVE (2026-10-29); quality gate and customer acceptance remain separate.

References: `specification.md`, `docs/design-system.md` (Document section,
Layers tab, ruler tokens and keyboard conventions). New interaction choices below
are **(Proposal)**: reversible in this story and vetoable at the demo. Existing
controls, validation, units, background and object editing retain their conventions.
ADR 0019 supersedes the draft specification's shaped-page wording: pages stay
rectangles; template outlines are ordinary guide objects and are not copied by Add.
No new dialog, operating hints, decorative effects, theme or shortcut category.

## Defaults and placement (criteria 1, 3, 10, 11)

- Use the specification's unanswered defaults: shared layers and placement to the
  right. Pages are frames in the same drawing; no per-page layer tree or object
  ownership badge. Cross-page clones keep their existing appearance and actions.
- New documents show `Page 1` at (0, 0) with the default size, as a rectangle. Old-file compatibility is out of scope (customer, 2026-10-29).
- **(Proposal)** Add copies the last listed page's dimensions; it does not copy objects or guides. Gap in millimetres is
  `max(20, 0.1 × max(last.width, last.height))`. Start at the last page's right edge
  plus gap with the same top coordinate.
- **(Proposal)** Make the spec's approximate wrap deterministic: inspect the trailing
  run of listed pages with the same top coordinate (using the domain's geometric
  tolerance). Wrap when that run already has eight pages, or the proposed page's
  right edge would exceed 4000 mm from that run's leftmost edge. Wrapped position
  is the run's leftmost x and its greatest bottom edge plus gap. A page wider than
  4000 mm is permitted; this is layout, never a size limit. Existing frames do not
  move. Reorder affects which page is last for the next Add.
- New names use `Page N`, choosing the next unused default number. Add activates
  the new page and reveals it through the visibility rule below, without opening
  any transient UI. One document step: `Add page`.

## Document tab and page list (criteria 2–6, 13)

- Keep the current size and format controls; their target is the active
  page. Put `Pages` and the `Add page` icon button below the existing size fields,
  followed by the list. Background and display unit remain document-wide.
- **(Proposal)** Reuse the 28px icon buttons, 16px Lucide glyphs, existing text sizes,
  focus rings and panel surfaces. Rows have two compact lines: editable name,
  then `width × height unit` in the muted tabular style. Reserve action columns so
  hover and activation do not shift text. Long names ellipsize with a full-name
  tooltip. No fixed maximum page count.
- The active page has the Layers active-row convention: a 3px accent bar and
  semibold name, `aria-current="true"`; focus is a separate visible inset ring.
  Each row contains a reorder grip, name/activation target, `Move frame only`
  switch (off initially), and `Delete page` button. Tooltip and accessible name
  identify both action and page where needed. Hide Delete when only one remains.
- A row press changes active page without changing object selection. **(Proposal)**
  a page is visible when its rectangle intersects the usable canvas viewport;
  otherwise pan to its centre at the current zoom. Never zoom on row activation.
  Keep keyboard focus in the list; pointer actions follow the panel's existing
  return-to-canvas convention. Scrolling the list does not activate rows.
- F2 or double-click the name edits inline. Enter or blur commits one `Rename page`
  step; Escape restores the old name. **(Proposal)** empty/whitespace-only input
  restores the old name; a no-op creates no history. Text editing consumes text
  shortcuts and never deletes canvas objects.
- Delete removes only the frame, immediately, with no confirmation. Objects and
  object selection remain. **(Proposal)** deleting the active page activates the
  next surviving row, or the preceding row at the end. Keyboard focus follows
  that row. Undo restores the frame; it does not replay a view-navigation step.
- **(Proposal)** Width, Height, format and orientation change only the active
  frame, anchored at its top-left. Preserve the existing single-page resize
  behaviour. In a multi-page document, translate only wholly contained objects
  by half the size delta, keeping them centred as the frame grows or shrinks;
  partially overlapping objects stay. Never scale objects or move adjacent pages.
  Keep existing number entry, preview, validation and commit rules.

## Reorder and keyboard access (criteria 5, 13, 14)

- **(Proposal)** Use a labelled `Pages` grid with roving row/cell focus; nested
  buttons and switches remain operable controls. Up/Down and Home/End navigate
  rows without activation; Enter/Space activates the focused name cell. Left/Right
  navigate cells, F2 edits, Delete removes a page when allowed. Escape leaves the
  list for the canvas. Preserve native text-field keys while renaming.
- Grip drag begins after 4 CSS px, captures the pointer, and shows the existing
  insertion line/ghost style. Release commits one `Reorder pages` step only when
  order changes. Escape or pointer cancellation restores the original order.
  List order never changes canvas positions, object order or active-page identity.
- **(Proposal)** Alt+Up/Down moves the focused page one position, matching Layers;
  provide `Move page up` and `Move page down` icon buttons beside Add as the
  single-pointer alternative. Buttons act on the active page and use the Layers
  toolbar's dimmed, focusable `aria-disabled` boundary treatment. Keep focus on the
  moved page and reveal its list row. Announce `Page 3, position 2 of 8` through the
  existing polite status region, without making the whole list live.
- Reuse coarse-pointer row/button sizing and drag hold from Layers. Keep all
  actions reachable without hover. The list scrolls inside the panel; with 200
  pages, preserve the focused row if virtualized and expose total row count/index.

## Canvas frames and movement (criteria 7, 9, 13)

- Draw every page with the shared document background and existing rectangular outline.
  Objects render above all page backgrounds, including objects crossing edges;
  do not clip objects to any page. Outside all page rectangles is the pasteboard.
  Names are small, screen-sized text above each frame's top-left, in the existing
  readable chrome text colour. **(Proposal)** inactive outlines are 1 CSS px and
  active outlines 2 CSS px, so activation is visible without colour alone.
- **(Proposal)** Select-tool page-edge hit testing uses a 6 CSS px outside-only edge band (the exact boundary remains content so existing marquees still work), after
  existing object/selection handles and object hits; Space-pan and higher-priority
  tool interactions have priority. Exclude the corner squares from the edge band,
  preserving the existing corner-start marquee gesture. While a group is entered,
  page-edge movement is unavailable: presses retain the group's existing editing
  scope. These restrictions preserve established selection gestures; the Document
  tab's X/Y fields remain the explicit page-move route.
  Page interiors retain ordinary selection/marquee behaviour. For overlapping
  frames, prefer the active frame, then the last drawn frame. Hover uses `move`.
  A press activates the frame; movement starts after 4 CSS px. Capture the pointer.
- Freeze the frame's original geometry and its wholly contained object set at
  press. Move the frame and that set by the same preview delta. Partial overlaps
  and pasteboard objects stay. Determine inclusion from rectangle bounds. Shared layers remain shared; do not reparent objects. Domain
  handling of groups, locks and clone transforms belongs in the ADR.
- **(Proposal)** `Move frame only` is per-page session state, reset on New/Open,
  not a document edit or undo step. Read it at drag start. On, only the frame
  moves; off, the contained set moves too. Its accessible checked state and tooltip
  name the state; no explanatory label is added to the panel.
- Release commits one `Move page` step; Escape, pointer cancellation, zero delta
  or a refused operation writes nothing. Undo restores frame and moved contents
  together. **(Proposal)** keyboard/single-pointer equivalent: active-page X and Y
  value fields in the Document tab, using the same frame-move operation and switch,
  global document units, and existing value-field input rules. Labels name coordinates.

## Rulers and selection context (criteria 8, 12)

- Ruler zero follows the active page's top-left: display `global x − page.x` and
  `global y − page.y`, converted to the chosen document unit. Keep existing tick
  spacing, negative values, screen positions, pointer marks and ruler hit behaviour.
  This is a view offset; object coordinates and clone transforms remain global.
- **(Proposal)** record page context on a canvas pointer press, not hover: the
  active page if it contains the press, otherwise the last drawn page containing
  it, otherwise pasteboard. A press on another page activates it without panning.
  Completing a move updates context at its final canvas position. Panel presses,
  wheel zoom and keyboard navigation do not invent a canvas pointer position.
- Ctrl/Cmd+A in Select uses the active-page context only if the last canvas context
  still identifies that active page. Include every selectable object whose bounds
  overlap its rectangle, including partial overlaps; otherwise use the existing
  all-drawing selection. Preserve current hidden/locked/group-scope filtering.
  Initial context is unset, hence existing all-drawing behaviour until canvas use.
  Changing active page through the list does not turn a stale context into the new
  page's context. Node-tool Select all and text-field Select all remain unchanged.

## Review evidence required

Check criteria 1–14 against current-format save/open, differently sized pages, add/wrap, rename,
delete/undo, reorder without movement, cross-page clones, page-edge movement,
frame-only movement, active-page sizing, local rulers and scoped Select all.
Include keyboard-only reorder/move, cancellation, last-page protection, overlapping
frames, long names, coarse-pointer controls and the existing minimum window size.
Exercise 200 pages for add, switch, list navigation and draw; record the measured
budget and environment in the implementation plan. Review findings and the
provenance of validation evidence are recorded below.

## Implementation review — 2026-10-29

**APPROVE.** Reviewed the current `PagesBlock`, `PageFrames`, `DocumentSection`,
page display types and document-panel hook, plus `target/browser-check/pages.png`
and `target/browser-check/many-pages.png`. No remaining concrete UX blocker was
found in this review's keyboard, cancellation, focus, reorder and theme scope.

Resolved findings: Escape reaches row drag cancellation; rename completion and
cancellation restore row focus; deletion retains a surviving focus target;
position fields refresh with display-unit changes; auxiliary row controls do not
create hundreds of Tab stops; active/insertion markers use the editor accent;
the frame-only switch has a visible thumb; unavailable reorder actions are dimmed;
inactive canvas labels remain opaque at 12px while only the outline is faded.

The lead reports Chromium checks passing for add, rename/Escape and F2 focus,
active-page width, reorder and grip cancellation, frame-only movement, deletion
protection, unit conversion and page reveal. The lead also reports 200 rows and
frames, first/last switching and rectangle creation passing in a 37.5-second
automation sequence. That duration is the whole sequence, not a per-action
latency measurement. The UX reviewer did not independently drive a browser or
run builds/tests during this review.

The mostly-pasteboard deletion screenshot is consistent with the documented
intersection-based reveal rule: the surviving frame still intersects the left
viewport edge. It does not require a new fit/zoom or delete-navigation behaviour.
The corner-marquee, entered-group and tool-priority restrictions above preserve
existing gestures. Final automated gate results remain the tester's responsibility.

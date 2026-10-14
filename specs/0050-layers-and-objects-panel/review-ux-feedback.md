# UX review of the 0050 customer feedback (2026-10-14)

Branch `story/layers-and-objects-panel`, head `dc2924f`. Reviewer: ux-engineer, 2026-10-14.
Scope: only the changes of the customer feedback of 2026-10-14 (criteria 3, 6, 20, 26, 35, 38,
55 to 57 and the UX notes "Tab", "Opacity slot is reserved", "Toolbar", "Row action"), checked
against `specification.md` and `docs/design-system.md` ("Properties panel: Layers tab").
Earlier full review: `review-ux.md`.

**Verdict: CHANGES REQUESTED.** The four changes the customer asked for work: the Initial Layer,
the Opacity slot at the bottom with a live drag, the toolbar instead of the footer, and the lock
cursor without a message. Three small must-fix items are left: two tooltips stack on top of each
other over the Opacity field, the toolbar does not behave like a toolbar for the keyboard, and the
toolbar's moves are not announced while Delete layer is announced twice.

## How this was tested

As in `review-ux.md`: WebKitGTK 2.54 `MiniBrowser` under Xvfb, window resized to exactly
800 x 600 CSS px, real pointer and key input from `xdotool`, the `npm run build` of `dc2924f`
served by a review-only page server with a small script to read the DOM (outside the repo,
deleted afterwards). **Not verified:** that WebKitGTK actually paints the SVG lock cursor (an Xvfb
screenshot does not contain the pointer); the computed CSS `cursor` value was read instead, and it
falls back to `not-allowed`. Chromium, WebView2 and WKWebView not checked.

## Results

| Check | Result |
|---|---|
| (a) First object in an empty document | Drawing a rectangle makes the row "Initial Layer" (level 1, expanded, active bar, `aria-current="true"`) with "Rectangle …" selected inside it, in the same frame as the object. The subject line reads "1 object". Understandable: the layer appears as the direct result of drawing and holds what was drawn (`fb-02`). **PASS** |
| (a) One undo | One Ctrl+Z removes both the rectangle and the layer; the tab returns to "No objects yet." and the canvas notice reads "Undid: Draw rectangle (1 rectangle)." (`fb-03`). Redo brings both back. **PASS**. History row wording: should-fix 6 |
| (b) Opacity at the bottom | The slot is 64 px (y 500 to 564), flush with the bottom of the panel's content box, the same edge where the History tab's "Wipe object history" ends. Field 244 px wide, caption under it ("1 object", "Layer Initial Layer", "Locked"). **PASS** |
| (b) Nothing targeted | After Escape the field and caption leave, the 64 px stay, and the list keeps y 80 and 420 px (measured before and after): no jump. **PASS** |
| (b) Live drag | With a layer targeted, the stroke of its objects fades on the canvas during the drag, before the release (`fb-04`, before and during). The rows are not written during the drag; the release writes one step ("opacity 15 percent"), and one Ctrl+Z restores 100 %. **PASS** |
| (b) Tooltip | Two tooltips open over each other on hover and on drag: the field's own ("Layer or object opacity: 24 % … Ctrl+Backspace: reset.") and the wrapper's ("Fades fill and stroke separately …"). The text is unreadable (`fb-05`). **Must-fix 1** |
| (c) Toolbar layout | `role="toolbar"` "Layers" at y 44: New layer (x 532), Duplicate (564), Move up (596), Move down (628), Delete layer (748, right end). Every button 28 x 28, 4 px apart, 16 px glyph, no text label, `aria-label` equal to the tooltip, tooltip `side="bottom"`. The footer is gone. **PASS** |
| (c) Dimmed, not hidden | Empty document: only New layer applies; the other four are at 40 % with `aria-disabled="true"` and stay focusable; no button moves when states change. Move up and Move down follow the target (sole child: both dimmed; bottom child: Move down dimmed). A press on a dimmed button does nothing (Space on a dimmed Move up: no change). Tooltip shows on a dimmed button too. **PASS** |
| (c) Actions | Move down, Move up, Duplicate (copy directly above, selected), New layer ("Layer 1", in front, name in edit, Escape keeps it), Move down on a layer target, Duplicate and Delete layer on a layer target all do what criteria 34, 36, 56 and 57 say. Focus stays on the pressed button. **PASS** |
| (c) Keyboard | Tab order: Layers tab, New layer, Duplicate, Move up, Move down, Delete layer, the list, the Opacity field. Focus ring 2 px `--editor-accent`, visible (`fb-06`). But five Tab stops and no arrow keys in a `role="toolbar"`. **Must-fix 2** |
| (c) Announcements | Move up and Move down from the toolbar announce nothing (Alt+Up and Alt+Down announce "Moved … to position …"); Delete layer is announced twice, by the canvas notice and by the tab's own live region. **Must-fix 3** |
| (d) Lock cursor, own lock | Over the outline of a locked rectangle the Select tool's cursor is the arrow with a padlock (`fb-08`, rendered from the CSS value at 4x), hotspot 3,3, fallback `not-allowed`. Over its hollow interior, which a press does not hit, the arrow. **PASS** |
| (d) Lock cursor, through the layer | After locking "Initial Layer", its unlocked children show the same lock cursor; empty canvas shows the arrow. **PASS** |
| (d) No message | A press on a locked object (own lock and layer lock) selects nothing, posts nothing to any `role="status"` or `role="alert"`, and acts as a press on empty canvas. **PASS** |
| Row control order | Name, opacity readout, row action, eye, lock, as amended. Eye (x 736) and lock (x 760) form one column on every row. The readout does not: on a layer row it sits 24 px further left than on an object row, because only layer and clone rows have the action slot (`fb-07`). Should-fix 4 |
| Customer rule: no sections for operations | **Met.** Every operation is a toolbar button or a row action with a visible trigger; nothing opens, slides in or grows. The Initial Layer row appears only as the result of drawing. The Opacity slot keeps its height whatever is targeted |

## Must-fix

1. **Two tooltips over the Opacity field.** `frontend/src/components/panel/LayersTab.tsx` line 644
   wraps the `ValueField` in a `Tooltip` with `OPACITY_TOOLTIP`, and the field opens its own
   tooltip. Both open together and overlap (`fb-05`). Fix: remove the wrapper and give the text
   "Fades fill and stroke separately. Overlapping parts show through." to the field's own tooltip
   as an extra muted line, so there is one tooltip. The Style tab's field is not affected.
2. **The toolbar has five Tab stops.** `role="toolbar"` tells a screen reader user to use the
   arrow keys, but Left and Right do nothing and every button is its own Tab stop
   (`LayersTab.tsx`, `ToolbarButton`). Fix: roving tabindex as the Reference control of the move
   chip: one Tab stop (the last focused button, New layer at first), Left and Right move between
   the five buttons without wrapping, Home and End to the first and last, dimmed buttons stay in
   the order because they are focusable. The design system's "Toolbar" entry gets the same
   sentence in a follow-up docs change (ux-engineer; not part of this review commit).
3. **Toolbar announcements.** Move up and Move down (`step_layer_target`) announce nothing, so a
   screen reader user does not learn that the press worked or where the node went; Delete layer is
   read twice, because the canvas notice is a `role="status"` and the tab's `sr-only` region
   repeats the text. Fix: the toolbar's Move up and Move down post the same announcement as Alt+Up
   and Alt+Down ("Moved Ellipse sv2ny2 to position 1 of 3"; a press that changes nothing is
   announced as criterion 32 says); results that go to the canvas notice slot are not repeated in
   the tab's live region.

## Should-fix

4. **Readout column.** The opacity readout of a layer row and of an object row are 24 px apart
   (`fb-07`). Options: (A) keep it: eye and lock align, which is what the eye scans, and object
   names keep their width; (B) reserve the 24 px action slot on every row, so the readouts form a
   column, at the cost of 24 px of name width on every object row (level 1: 104 px, 76 px with a
   readout; "Rectangle 3f9a1c" no longer fits). Recommendation and default: A. A question for the
   customer, since the customer asked to check the alignment.
5. **"Layer Layer 1".** With default names the Opacity caption reads "Layer Layer 1" and the
   notice "Deleted layer Layer 1." (`LayersTab.tsx` caption, `curvyo-ui-core/src/layer_text.rs`).
   Fix: the caption shows only the name after its `Layers` glyph ("Layer 1", "Cut"); the glyph
   already says layer. Notices keep "layer" as the noun.
6. **History row of the first object.** The Object scope row of the step that made the Initial
   Layer reads "Draw rectangle · with 1 other object": the layer is counted as an object
   (`curvyo-document-core/src/history/object_rows.rs`). Fix: a layer made in the same step is not
   counted, or the row says "and the layer Initial Layer".
7. **Duplicate of a layer keeps the name.** Duplicating "Layer 1" gives two layers named
   "Layer 1", which the caption and notices cannot tell apart. Proposal: the copy is named
   "Layer 1 copy" (Inkscape and Illustrator do the same). A new rule, so the product owner decides;
   default: keep the name.

## Screenshots (`ux-screens/`)

- `fb-01-empty-document.png`: the empty tab, toolbar with only New layer applying.
- `fb-02-initial-layer.png`: after the first rectangle, Initial Layer and its row.
- `fb-03-one-undo.png`: after one Ctrl+Z, both gone.
- `fb-04-opacity-live-drag.png`: canvas before (top) and during (bottom) an Opacity drag.
- `fb-05-opacity-double-tooltip.png`: must-fix 1.
- `fb-06-toolbar-focus.png`: keyboard focus on Duplicate, dimmed Move up and Move down.
- `fb-07-row-readouts.png`: row order and the readout offset of should-fix 4.
- `fb-08-lock-cursor-4x.png`: the lock cursor on the canvas tone and on a dark fill, 4x.
- `fb-09-locked-layer-slot.png`: a locked layer targeted, the slot reads "Locked".

# 0025 Raster trace: UX notes

> **Section 20 (customer decisions 2026-10-29, #216) overrides every earlier section where they differ**: no Node limit, no Full-size preview, Threshold in percent, an info button, the counts line inside Advanced, the two toggles at the front of Advanced, value fields from the shared `ValueField`. Passages below that it replaced are marked "(section 20)".

Author: ux-engineer. Date: 2026-10-27, slices 2 and 3 added 2026-10-28 (sections 11 to 17). Status: for Ready. Builds on `ux-proposal.md` and
`ux-mockup.html` (accepted by the customer 2026-10-14) and `docs/design-system.md`. Where this file
differs from the proposal, the difference is listed in section 9. Own choices are marked **(Proposal)**.

Customer rules applied: operations are tools with an options bar, nothing in the right panel; no
operating tips or hint labels (2026-10-16), only labels that name a control or a state; no dropdowns on
full bars (rare functions go to an expandable second row); no dialogs, inline confirmation only.

## 1. Where the tool lives: the rail is full

Re-check against the built rail (`frontend/src/components/ToolRail.tsx`, `ImportCard.tsx`, row "Tool
rail architecture"): column A is Tools 7 buttons (312 px), Group (92), Path tools (48), Import (48),
ending **536 px** below the viewport top; the usable height at the 640 px Tauri minimum is 560 px. The
proposal's "Image tools" card (+56 px) would end at **592 px**, 32 px too tall. Any further button in
column A ends at 580 px (20 px too tall). Column B is full (12 buttons). The design system already
names the cap: a new family of commands goes to the Select bar or the panel; a third column is a
customer decision. The "too short" rule (cards below Tools scroll) is for the browser build only and
must not become the normal state of the Tauri window.

**Decided (customer, 2026-10-27, Frage 61):** no new rail card. The Trace button belongs in the top bar
of the Select tool when raster images are selected (one or several); the toolbar is tidied later in polish.

- **Key `I`** selects the Trace tool in every state (a tool letter like `B`, `N`, `P`); ignored in text
  fields and the text editor. Ctrl+I / Cmd+I stays Import. If no image is selected, `I` shows the empty
  bar (section 2).
- **A text button "Trace"** in the Select bar, shown only when the selection is images only (one or several),
  **appended last in a group of its own** (the "Linked clone" pattern: a new control is appended, no
  earlier one moves). 28 px outline button as "Linked clone". Press = choose the tool. Tab order: last
  in the bar. Measure at 800 x 600 with the panel open (bar about 324 px): it wraps whole.
- While the tool is active no rail button is pressed; the bar's first element is the **state label
  "Trace"** (text, 12 px, `--toolbar-icon`), which names the mode. The Select button is not pressed
  either. Escape, `S`, or choosing any rail tool leaves.
- Rejected: a second button in Path tools or beside Import (580 px, over the cap); a flyout (design
  system rejects them); column B (commands on the selection, full).

The customer chose this option; spec criterion 1 is updated. A rail button is out of scope until the
toolbar polish.

## 2. Bar states (replaces the Select bar in place, overlay row)

| State | Bar |
|---|---|
| One image selected | Full bar (section 3), preview at once on a reduced copy, full size follows. |
| Several images selected (only images) | Same bar, label "Tracing 1 of 3 images"; the preview shows the image clicked last; Enter traces all with the same settings, one undo step. |
| Nothing selected | One row: state label, text "Click an image to trace it." and text button "Import image..." (same action as the rail Import button; focus rules of the Import card). |
| Something else selected | One row: "The selection is not an image. Click an image to trace it." Nothing refused. |

The two sentences are the empty state and name what the tool waits for; they are not tips. Several
images selected is slice 1 (customer, Frage 61): the label says "Tracing 1 of 3 images" only when 2 or
more are selected. **(Proposal)**

## 3. Bar contents, slice 1 (black and white)

Pill as the Select bar: rows 36 px (28 px controls), `text-sm`, 25 % dividers, wraps by whole groups,
left after `--rail-right` + 12. Nothing is drawn that has one choice: **no Mode, Preset, View, Advanced
in slice 1** (an empty expander is never drawn). Order:

| Row at 800 px, panel open (about 356 px) | Contents | Width |
|---|---|---|
| 1 | State label "Trace"; **Threshold** value field (0 to 100 %, section 20; label inside, `--value-fill` bar, starts at the automatic value and shows it) | 48 + 8 + 168 |
| 2 | **Smooth** (0 to 100, default 60, same field) | 168 |
| 3 | **Specks** ("Ignore below", mm, 0 to 10, default 0.5, muted pixel hint "~ 2 px" (the field shows the mm) that follows the image's size on screen) | 212 |
| 4 | State row (section 20): only while a state is active; the counts line lives in Advanced; no Create paths button (section 19) | about 330 |

Arithmetic (label + Threshold 224, Smooth 168 and Specks 212 do not pair within 356) gives 4 rows (156 px) with the panel open, as in the mockup; with the panel
collapsed (about 628 px) Threshold, Smooth and Specks share row 1 and the result row is row 2 (84 px).
The first row never moves; the result row is last. **(Proposal)** Measure at build.

Slice 2 puts the Preset button in row 1 beside the state label and Advanced beside Specks; the exact
rows are in section 11 (superseding the "row above" sketch of slice 1). The fields move once, in that
PR, and the PR description says so. **(Proposal)**

Fields: Up/Down 1, Shift 10, Specks 0.1 / 1 / Ctrl 0.01 mm, Home/End, type and Enter (criterion 5);
Enter in a field returns focus to the canvas. Hit target 28 px, focus ring 2 px `--editor-accent`.
Values are remembered as an app setting. A change of value is not a history step.

## 4. Preview look

- The image is dimmed to 35 %: a white wash on the light theme, the canvas colour at 65 % on the dark
  theme. Drawn over the image only, under the outline.
- Outline: **hollow, 1.5 px screen-space, `--preview-new`**, over a 1 px white casing
  (`--selection-casing`), holes outlined too, no fills (row "Live preview outline"). Overlay layer, never
  a node; culled to the viewport on pan and zoom, no re-trace.
- While a job runs the previous outline stays. After 400 ms: status "Tracing..." and a 2 px progress
  line under the bar (`role="progressbar"`, `aria-busy` on the bar). After 1.5 s a text button "Stop"
  appears in the result row (slice 3 for the failure card; Stop is slice 1 because cancel is
  criterion 17) **(Proposal)**. A stale outline drawn at 50 % carries the state text "Preview is out of
  date" (words, not opacity alone).
- Counts after a settled trace: "7 paths, 412 nodes, 0.4 s"; muted second line "38 specks ignored"
  when specks were dropped; they are the last row of the Advanced block, not of the bar (section 20). No tooltip advice on the node count.
- The View pill and the `\` peek are slice 3. Until then, to compare, the maker leaves the tool with
  Escape (the dim and the outline go) and enters with `I`; nothing extra is drawn **(Proposal)**.

## 5. Create paths (the command; no button since 2026-10-29, section 19)

- Routes: Enter with focus on the canvas, Ctrl+Enter in a field, double-click on the image. Escape: first
  press stops a running job, second leaves the tool; no dialog, nothing was changed.
- If only the reduced-copy outline is settled, Create **waits for the full-size trace**: the
  status reads "Tracing..." and the bar keeps `aria-busy`; the group is inserted when the trace
  finishes. A new press during that wait is ignored. **(Proposal)**
- On commit: one step "Trace image" does everything of criterion 13: group "Trace of <name>" directly
  above the image, one compound path (black, no stroke), the image stays visible, group selected, Select
  tool, focus to the canvas. The Select bar shows. Ctrl+Z reverses all of it.
- No notice (decided by the customer, 2026-10-31: no notice after creating paths); the new group being
  selected is the feedback. The counts are the Advanced status line and the Layers row tooltip.
- Zero paths: Enter does nothing, the status gives the reason (section 7). One path covering
  the whole image: allowed.

## 6. Source image stays visible afterwards (Frage 54, decided A)

The image is untouched (visible, unlocked); the paths sit on top on the pixels they came from (criterion
14). The maker hides or moves the image by the usual means (Layers eye, Select). No switch in the bar and
no extra notice text.

## 7. States and wording (permit over refuse; no tips)

| Situation | Text |
|---|---|
| Image loading | Status "Loading image..."; Enter waits. |
| 0 paths | "0 paths at this Threshold: too low." or "too high." (direction computed from the histogram). **(Proposal)** replaces the spec's "Try moving Threshold toward lighter.", an instruction. |
| One shape | "1 path covering the whole image." Enter creates. |
| Over 100 000 nodes (slice 3) | "412 000 nodes. Slow to edit." Enter creates. |
| Failure (slice 3) | `--field-invalid` card: "Trace failed: <reason>. Your drawing is unchanged." with Retry. |

Removed from the proposal because they are hints: the Photo-preset advice line, the "Fewer nodes edit
and cut faster" tooltip, "raise Smooth or Specks", the tooltip second line of the rail button.

## 8. Keyboard, focus, accessibility

- `I` selects the tool; with the canvas focused and an image selected Tab enters the bar: Trace (Select
  bar only), then Threshold, Smooth, Specks, the expander (Stop when shown). Shift+Tab reverses; Tab
  from the last stop returns to the canvas.
- Enter creates, Escape cancels, Ctrl+Enter from a field. Every state change not visible on the canvas is
  text in the status row, announced politely once per settled result.
- Contrast: outline 3:1 against the dimmed image through the casing; text `--toolbar-icon` on
  `--toolbar-bg`; stale state in words. No motion, so reduced-motion needs no case. Hit targets 28 px.
- Large documents: the preview is an overlay of cached paths, cull on pan and zoom (criterion 8); the
  bar stays responsive while a worker traces (criteria 10, 17).
- Hand off to the implementer: bar width and wrap measured at 800 x 600 with the panel open and closed
  (screenshots in the PR), contrast checked in both themes.

## 9. Differences from `ux-proposal.md` and the spec

1. No "Image tools" card; key `I` plus a Select bar button (Frage 61). Spec criterion 1 and
   `ux-proposal.md` sections 1 and 11.1 are superseded (decided).
2. No second tooltip line (spec criterion 1): operating tip.
3. 0-path and whole-image texts lose their instruction (spec criterion 15).
4. State label "Trace" at the start of the bar, because the rail shows no pressed button.
5. Create waits for the full-size trace instead of using the reduced copy.
6. Slice 2 puts Preset beside the state label in row 1 and Advanced beside Specks (one-time shift, section 11).

## 10. Decided by the customer (2026-10-27)

**Frage 61 (TRACE-RAIL):** "Belongs in the top bar of the Select tool when a raster image is selected;
several can be selected. The toolbar needs tidying later in polish; first we make chips." Option A
stands, extended to several selected images (section 2). Colours are postponed (Frage 50 A), so the
Mode pill, Colours field and Photo preset of the proposal are not drawn.

## 11. Slice 2: Preset button, presets, bar rows (criteria 37, 40, 59 to 66)

**Rows** (inner width 344 px at 356; label 48, Preset 132, View 78, Threshold and Smooth 168, Specks 212,
Advanced 96, gap 8). Groups wrap whole; the first row never moves, the result row is last. Measure at build.

| Panel open (4 rows, 156 px) | Contents |
|---|---|
| 1 | State label "Trace", **Preset** button (132 px; View joins here in slice 3: 48+132+78+16 = 274) |
| 2 | Threshold and Smooth side by side (168 + 8 + 168 = 344: no slack, so they wrap to two rows if the build measures wider) |
| 3 | Specks and the **expander** (212 + 13 + 25 + 12 + 28 = 290, section 19) |
| 4 | Status, Stop while a job runs (section 19) |

Panel closed (about 628 px, inner 616): 3 rows; row 1 label, Preset, View, Threshold (450); row 2 Smooth,
Specks, expander (about 430); the result row 3. **(Proposal)**

**Preset button.** Text button, 28 px, outline as "Linked clone", shows the preset name or "Custom",
`aria-haspopup="menu"`, `aria-expanded`, the chevron of the accepted mockup (it names a menu; it is not a
rare-function dropdown). Name "Preset" is the accessible name, not drawn.

**Menu.** Opens below the button, 4 px gap, left edges aligned, 232 px wide, flips above when it would leave
the viewport; `--toolbar-bg`, `--panel-elevation-shadow`, 8 px radius. `role="menu"` with five
`menuitemradio` rows in the order of criterion 59 (Logo / Line art, Photo, Silhouette for cutting/carving,
Engraving), each a name (semibold) and its second line (muted, `aria-describedby`): "Sharp corners, clean
curves.", "Many shapes. Photos trace roughly.", "Closed, simple, few nodes.", "Keeps fine lines." (the
customer's texts, Frage 64 A); they describe the result, not operating tips. "Silhouette for cutting/carving"
is the longest name; the build checks that it fits the 232 px menu on one line. Enter or Down on the button opens and puts focus on the
checked item, in Custom on the preset the values came from (marked as current by `--editor-accent-hover`,
not checked); Up and Down move without wrap, Home and End, Enter or click picks and closes, Escape closes
the menu only and focus returns to the button; a click outside closes it. The pick is not a history step
and sets the values of criterion 60 only (never Threshold or the switches); focus returns to
the button, not the canvas, so the maker can keep adjusting.

**Custom.** The label comes from the stored state `{ preset, edited, values }` (architect, 2026-10-28;
no value comparison anywhere). A pick sets `preset` and `edited = false`; any edit of Smooth, Specks,
Corners, Upscale or Denoise sets `edited = true`, and the button then shows "Custom". The menu marks and
focuses `preset`, the preset the values came from. Threshold, Invert and Ignore background are not preset values and never cause "Custom". Putting the values back by hand stays
"Custom"; picking the preset again clears it (criterion 40).

**Status and notice.** A pick prints nothing: no notice, the status line does not name the preset
(criterion 66); the preview and counts update after the 150 ms debounce. With several images the pick
applies to all (label "Tracing 1 of 3 images" unchanged).

## 12. Advanced block (criteria 41, 43, 44)

- **Expander** (replaced 2026-10-29, section 19): the Text bar's `ExpandButton`, `ChevronsDown` / `ChevronsUp`,
  28 px, `aria-expanded`, never hidden or dimmed. Open and closed is remembered as an app setting (own key,
  not the text bar's). No text button, no dropdown caret.
- **Block** below the result row inside the same pill, separated by a 25 % divider; the bar only grows
  downward, nothing above moves. Two columns of 168 px, rows of 36 px, in this order: Denoise (3-segment
  pill Off, Low, High) and Upscale (Auto, Off); Corners (value field, 0 to 100). **Section 20 replaces this order**: row 1 starts with the two icon toggles, Node limit and Full-size preview no longer exist. Labels are drawn in full (the
  mockup's "No backgr." is cut or unclear). **(Proposal)**
- **Honest controls.** A control the tracer cannot honour is not drawn (criterion 41); no placeholder.
- Full-size preview and Node limit are gone (section 20). Invert and Ignore background have no status text.
- Escape in an Advanced field returns focus to the canvas; Tab leaves the block to Stop (when shown) or the canvas.

## 13. Slice 3: View pill and peek (criterion 51)

- **Pill** right of Preset in row 1, `role="radiogroup"` "View", three 25 x 28 px icon segments (16 px
  glyphs, the Select bar's pill look, selected = `--active-bg`): **Overlay**, **Trace**, **Image**. Tooltips
  name the segment (Overlay, Trace, Image); the Image tooltip adds the key as a key name ("Image (\)"), the
  a key name on a control, not a tip. Arrows move and select, no Tab stop per segment.
- **Overlay** is section 4. **Trace** draws the result with its fills (black, as Create paths makes it), the
  image hidden, no dim, no outline; on a dark document ground the shapes get the 1 px `--selection-casing`
  edge so black stays visible. **Image** shows the original at full strength, no outline. Zoom and pan work
  everywhere; none of this touches the document.
- **Peek.** Holding `\` shows Image while held and returns on release; the Image segment shows pressed
  while held (the pill always tells the truth). Key repeat is ignored; release, window blur and tool exit
  restore the chosen view. Shift+`\` cycles Overlay, Trace, Image. Both ignored in text fields.
- The chosen view is not remembered: entering the tool always starts on Overlay, so an Image view cannot
  look like "the preview is gone". **(Proposal)**
- In Trace view a stale preview is drawn at 50 % with the words of section 14; while a job runs after
  400 ms the same.

## 14. Stale preview, Stop, large images, failure (criteria 52 to 55)

- **Running** (after 400 ms, section 4): outline at 50 %, status "Tracing...", muted second line "Preview
  is out of date" (the mockup's computing state), progress line, **Stop** after 1.5 s.
- **Stopped** (Stop or first Escape; section 21.2 replaces the words with the **Preview** button): outline at 50 %.
  Changing a value, Enter or a double-click on the image traces again; Enter while stale is "trace again", never Create.
  **(Proposal, 2026-10-29)** The row is the status only; Tab has nothing to stop on.
- **Over 4 Mpx:** the preview is the reduced copy; the muted second line reads "Preview from a reduced
  copy", with "; 38 specks ignored" appended when both apply (about 280 px, fits). Counts are those of the
  reduced copy; the success notice carries the full-size counts. Enter shows the progress line and Stop
  at once (no 1.5 s delay: the wait is known), status "Tracing at full size...". Escape stops; the document
  is untouched until the trace finishes. **(Proposal)**
- **Over 100,000 nodes:** status "412,000 nodes. Slow to edit." (comma separators), Enter creates;
  no confirmation.
- **Failure card.** Under the bar, 8 px gap below its bottom edge (below the Advanced block when open),
  same left edge and width, the refusal look of the Action notice: `--popover` ground, 1 px
  `--field-invalid` ring, 12 px alert glyph, text "Trace failed: <reason>. Your drawing is unchanged.",
  `role="alert"`, plus a **Retry** text button (28 px). Red is never the only cue (glyph, words, ring).
  Reasons are plain ("out of memory at 36 Mpx", "internal error"); no stack, no code. It does not take
  focus; Retry is the last Tab stop; Enter on the canvas also retries. It stays until the next successful
  trace, a changed value, another image or leaving the tool; values stay. Enter retries, not creates.
  A worker that died is replaced on Retry. **(Proposal)**
- **Pan and zoom** with a big preview: cached outline, culled (criterion 56); no state text.

## 15. Keyboard and accessibility, slices 2 and 3 (criterion 57)

Tab: Trace (Select bar), Preset, View (one stop, arrows inside), Threshold, Smooth, Specks, Advanced, the
Advanced controls when open, Stop (when shown), Retry (when shown). Shift+Tab
reverses. The menu follows the menu pattern of section 11. `\` and Shift+`\` work with focus on the
canvas or a bar button. Every state change not visible on the canvas is text in the status row or the
alert card, announced once. Hit targets 28 px, focus ring 2 px `--editor-accent`. Contrast of muted text
and the 50 % outline checked in both themes. Nothing animates, so reduced motion needs no case.

## 16. Slice-1 leftovers (criterion 68)

(a) The Layers row of the trace group carries the tooltip text "Traced: 7 paths,
412 nodes." (no canvas notice since 2026-10-31). (b) The wash of section 4 is checked in both themes by a test (white wash on light; canvas
colour at 65 % on dark). No design change.

## 17. Mockup comparison (criterion 67)

Mockup `ux-mockup.html` sections 1 and 2, planned differences. The implemented tool is compared at 800 x 600,
panel open and closed, in the review; this list is the starting point and the PR adds what the build shows.
"Fixed" = the build follows the mockup or a later rule; "Show at demo" = a deliberate difference the
customer may veto. Colours, Mode, Photo: excluded (Later).

| # | Mockup | Build | Status |
|---|---|---|---|
| 1 | Rail card "Image tools", tooltip "Turn an image into paths. Enter creates them, Esc cancels." | Key `I` and a Select bar button; no tooltip text | Fixed (Frage 61; no tips, 2026-10-16) |
| 2 | Presets Logo / Line art, Photo, Silhouette, Engraving | The same four; Silhouette is named "Silhouette for cutting/carving"; menu mechanics as mockup | Fixed (criterion 59, Frage 64 A) |
| 3 | Preset button on row 1 with the Mode pill | Preset beside the state label "Trace"; no Mode | Fixed |
| 4 | No state label | Label "Trace" starts row 1 (no rail button pressed) | Show at demo |
| 5 | Specks hint "~ 6 px" | "~ 2 px" live from the image | Fixed |
| 6 | "9 180 nodes" | "9,180 nodes" | Fixed |
| 7 | "0 paths. Try a lighter Threshold." | "0 paths at this Threshold: too low." (or too high); no instruction | Show at demo (criterion 67: spec wording wins unless the customer picks the mockup's) |
| 8 | Advanced labels "No backgr.", "Limit 20000 nodes" | "Ignore background" (icon toggle); no Node limit (section 20) | Fixed |
| 9 | Advanced has no Full-size preview | None either (customer Frage 74 A) | Fixed |
| 10 | Computing: "Tracing..." and "Preview is out of date" with Stop | Same; after Stop Enter or a value change traces again (no button) | Show at demo |
| 11 | Failure: red text with Create paths beside | Card with glyph, ring, Retry, under the bar; Enter retries | Fixed (red not the only cue) |
| 12 | After Create: image hidden, notice "Source image hidden." | Image stays visible (Frage 54 A); notice without that sentence | Fixed |
| 13 | Menu second lines (Logo / Line art "Sharp corners, clean curves.") | Kept, with the customer's texts (criterion 59) | Fixed |
| 14 | Bar "four rows" | 4 rows panel open if Threshold and Smooth pair, else 5; 3 closed | Fixed after measuring |
| 15 | View pill | Slice 3; chosen view not remembered | Show at demo |
| 16 | Advanced button with a chevron | The Text bar's `ChevronsDown` expander (section 19, customer 2026-10-29) | Fixed |
| 17 | Specks value "0.50mm" with the unit in the field, label "Specks" | Label "Ignore below", value without unit, muted "< 1 px" beside it; "~ 0 px" at small values | Show at demo |
| 18 | Threshold and Smooth as label plus white value box on a fill | One fill field, label inside, value right (section 3) | Fixed |
| 19 | Panel open: bar four rows | Measured 5 rows at 800 x 600 (bar 324 px wide, Threshold and Smooth do not pair); panel closed 3 rows as planned | Fixed (row 14 resolved: five) |
| 20 | Menu: current item shown by fill only | Fill plus check mark (not colour alone) | Fixed |
| 21 | Zero paths: one-line status | At panel open "0 paths at this Threshold: too low." wraps to two lines and the bar grows 12 px | Show at demo |
| 22 | Advanced fits its pill at 800 x 600 | Panel open: the button ends 50 px right of the pill, over the right panel | Superseded by the 28 px expander (section 19); measure again |
| 23 | Advanced block: Denoise Off / Low / High, two columns inside the pill | "High" clipped (pill container 104 px, segments need 121 px), panel open also Upscale, Node limit, Ignore background overflow the pill | To fix (review findings B2, B3) |

Reviewed at 800 x 600, panel open and closed: bar rows, Preset menu (keyboard, Custom), Advanced block, zero-path
status, Create paths, Layers tooltip, Tab order. Not exercised: computing and Stop, failure card, several
images, nothing selected, dark theme, Full-size preview over 4 Mpx.

## 18. Open for the lead

Answered: the Preset menu's second lines are the customer's own texts (Frage 64 A, 2026-10-28); they stay.

## 19. Revision 2026-10-29 (customer feedback on #216; overrides sections 3, 5, 11, 12, 14, 15 where they differ)

Customer: Advanced with a double arrow down "like for fonts"; the switches as icon toggles; the Create paths
button goes, Return does it; "Ignore below" stays as it is. No hint text is added (rule 2026-10-16).

**Expander.** Advanced is the Text bar's `ExpandButton` (`frontend/src/components/TextBar.tsx`, spec 0084):
28 x 28, `ChevronsDown` closed / `ChevronsUp` open (Lucide, 16 px, 1.5 px), `--toolbar-icon`, hover
`--editor-accent-hover`, no pressed fill, `aria-expanded`, `aria-controls` = the Advanced block's id, Tooltip
`side="bottom"`, Enter/Space keep focus on it, a mouse press returns focus to the canvas. Tooltip and name
"More trace options" / "Fewer trace options" (the pair of `expandLabel`); the word "Advanced" is no longer drawn.
It sits after Specks behind a 25 px divider, the last control of row 1's last line. While closed and any Advanced
value is not its default (Invert, Ignore background on; Denoise, Upscale, Corners changed), the 6 px accent dot and the tooltip's second line "Changed: Invert, Node limit" (also its
`aria-describedby`). **(Proposal)** Implementation: generalise `ExpandButton` (props: `open`, `onToggle`,
`controls`, the two labels, `changed: string | null`; no `TextBarState`) and use it in both bars; do not copy it.

**Icon toggles** (two since section 20, at the front of Advanced row 1; the look and wiring of the Glyphs button in `TextBar.tsx` and Italic in `StyleGroup.tsx`): one
row at the end of the Advanced block, 28 x 28, 4 px apart, `rounded-md`, 16 px icon at 1.5 px, off:
`--toolbar-icon` with hover `--editor-accent-hover`; on: `--toolbar-icon-active-bg` / `--toolbar-icon-active-fg`.
`aria-pressed`, no label drawn, Tooltip `side="bottom"` with the full label, `aria-label` the same. Arrows
are not used; each is a Tab stop. Space/Enter toggle and keep focus; Escape returns focus to the canvas.
Both keep their behaviour from section 12; they are not preset values (never "Custom").

| Toggle | Lucide icon | Tooltip and name |
|---|---|---|
| Invert | `Contrast` | Invert |
| Ignore background | `ImageMinus` | Ignore background |

(`ScanSearch` is free again; `Eraser`, `Maximize2` and `Expand` are in use elsewhere with other meanings, so they are not reused.)

**Create paths without a button.**
- Enter with focus on the canvas, Ctrl+Enter in a field and a double-click on the image create; Escape,
  waiting for the full-size trace, the single undo step and the notice stay as section 5. Enter on a focused
  bar button presses that button (Preset, expander, toggles, Stop), as everywhere. Enter in a value field
  commits it and returns focus to the canvas; one more Enter creates.
- **Learning it:** the bar is `role="toolbar"` named "Trace", with an `aria-description` / hidden
  `aria-describedby` text "Enter creates the paths". Nothing is drawn. The tool's status text after a settled
  trace ("7 paths, 412 nodes, 0.4 s") stays as the result row. **(Proposal; if the lead reads an off-screen
  description as a hint, drop it and put the question to the customer.)**
- **0 paths:** nothing to disable; Enter does nothing and the status already gives the reason (section 7).
- **Stopped / stale:** no Trace again button; Enter, a double-click on the image or any value change traces.
  The row shows "Stopped" and the muted "Preview is out of date" only. **(Proposal)**
- **Failure:** the card keeps its Retry button; Enter retries too.

**Rows at 800 x 600, panel open (bar about 324 px, inner 308).** Row 1: label "Trace", Preset, View (as before);
row 2: Threshold, Smooth (168 + 8 + 168 = 344 does not fit 308: they stay on two rows, measured); row 3: Specks
(212) and the expander (212 + 13 + 25 + 12 + 28 = 290, fits); row 4: status (and Stop while running). Rows
closed/open counts are unchanged from the measured build (5 rows at 800 x 600 with the panel open, 3 with it
closed); the result row loses the 112 px button, so "7 paths, 412 nodes, 0.4 s" and the zero-path text have the full width
(the zero-path text still wraps once at 356 px). Advanced block: 3 rows (Denoise | Upscale; Corners | Node limit;
the three toggles), 108 px instead of 144; at 800 x 600 panel open the bar ends about 304 px (before: 340)
below its top. Measure and put the screenshots in the PR.

**Tab order:** Trace (Select bar), Preset, View, Threshold, Smooth, Specks, expander, then when open Denoise,
Upscale, Corners, Invert, Ignore background (order: section 20.4), then Stop (when shown), Retry (when shown).

## 20. Revision 2026-10-29, second round (customer decisions on #216; overrides all earlier sections)

Decisions: Node limit gone from the UI; Full-size preview gone (Frage 74 A); Threshold in percent (75 A); Corners
unchanged (76 C); an info button (77 A, **a deliberate exception to the no-hint rule of 2026-10-16**: it shows
only when asked, never by itself, and no other hint is added); counts only in Advanced (78 B); the two toggles
first in Advanced; value fields behave like the rest of the app. The PO updates `specification.md`; the
"Over 4 Mpx" reduced-copy line of section 14 stays a state until the PO says otherwise.

### 20.1 Value fields: the shared `ValueField`

The Trace bar uses `frontend/src/components/panel/ValueField.tsx` with `useValueDrag` and `SliderTip` (the Pencil
bar's Min/Max already use it at 72 px; the Trace fields set 168 px). `TraceField.tsx` shrinks to an adapter from
`TraceParam` (label, range, step, unit) to its props, or goes; its own pointer and typing code is deleted.
Behaviour is then exactly the app's (design system "Value field"): a click sets the value at the pointer and
commits once on release; a drag moves it (3 px, 8 px touch; the reduced copy is traced while it moves, full size on
release); a double-click, Enter or F2 on the focused field opens the number entry (Enter commits and returns
focus to the canvas, Escape or a press elsewhere closes without writing; one more Enter on the canvas creates);
arrows 1, Shift 10, Home/End; Escape in a drag drops it. Ctrl+Enter creates, also in the entry. No reset slot in
the bar. The slider tooltip ("Click to set, double-click to type") is the app's existing convention for every
slider, not a new hint. A click is not a history step. Accessible names "Threshold", "Smooth", "Ignore below",
"Corners"; `aria-valuetext` carries the unit ("40 percent", "0.50 millimetres").

### 20.2 Threshold in percent

Shown, dragged, stepped and typed as **brightness 0 to 100 %**, whole numbers, unit "%" (field as Opacity); the
engine keeps luma 0 to 255 exactly: `luma = round(p x 255 / 100)`, shown `p = round(luma x 100 / 255)`. All 101
percentages map to distinct luma. Arrows 1 %, Shift 10 %, Ctrl 1 % (no finer step); drag and click on the track
step 1 %. Typed "40", "40 %" and "40,5" are accepted and rounded to whole percent; out of range is clamped to 0 or
100 without a message (as today). **Automatic (Otsu) start:** the field shows the rounded percent of the exact
luma (luma 128 shows 50 %), looks like any other value (no "Auto", no marker), and the exact luma is used until the
maker changes the field; the first step then starts from the shown percent. Changing the image re-computes it.
Tooltip line 1 "Threshold: 50 %". Brightness runs black to white; the field does not say which side becomes black.

### 20.3 Info button and the Properties panel page

**Superseded by section 21.1:** the page is a tab of the strip; the bar button and the X are gone. Only the page text below stands.

- **Button.** Lucide `Info`, 16 px, 1.5 px stroke, 28 x 28, `rounded-md`, the icon-toggle look of section 19 (off
  `--toolbar-icon`, hover `--editor-accent-hover`, on `--toolbar-icon-active-bg` / `--toolbar-icon-active-fg`).
  Placed **in row 1 right after the state label** ("Trace", 4 px, the button), before Preset: label + button
  80 px, row 1 = 80 + 8 + 132 + 8 + 78 = 306 of 308 at 800 x 600 panel open. Measure; if View wraps, it joins
  row 2 and the bar is one row taller, accepted. `aria-pressed` (it toggles a page), name and tooltip
  "About Trace" (Tooltip `side="bottom"`), `aria-controls` the page. A mouse press returns focus to the canvas; a
  key press (Enter, Space) keeps focus on the button. Never dimmed.
- **Page.** While pressed, the Properties panel body shows the page in place of the active tab (the strip and
  collapse control stay; the active tab keeps its look, no tab shows the page). Header row as every tab: the name
  "About Trace" at the left, a 28 px `X` close button ("Close") at the right. Under it a `region` named "About
  Trace", 244 px wide, scrolling with the body: one intro paragraph, then a definition list, 14 px, terms
  semibold `--toolbar-icon`, descriptions `--panel-muted-fg` (5.0:1), 4 px between a term and its text, 12 px
  between entries. Not a live region; opening it announces nothing and takes no focus on a mouse press; with the
  keyboard focus stays on the button, and the region is a Tab stop (`tabindex="0"`) only when it scrolls.
- **Text (Proposal; the customer reads and changes it).**
  Intro: "Trace turns a picture into outlines you can edit, cut or engrave. The outline is drawn over the image;
  Enter creates it as paths and keeps the image."
  Preset: "Sets Smooth, Ignore below, Corners, Denoise and Upscale for a kind of image."
  View: "Overlay shows the outline over the image, Trace shows the result alone, Image shows the original."
  Threshold: "Brightness at which a pixel counts as dark: 0 % is black, 100 % is white; darker pixels become shapes."
  Smooth: "Higher values give calmer curves with fewer nodes; lower values follow the pixels more closely."
  Ignore below: "Shapes smaller than this size are left out."
  Denoise: "Removes grain from the image before tracing."
  Upscale: "Enlarges small images first so thin lines survive; Auto decides by size."
  Corners: "Higher values keep more sharp corners instead of rounding them." (the implementer checks the direction
  against the tracer and corrects the sentence, not the control.)
  Invert: "Traces the light areas instead of the dark ones."
  Ignore background: "Leaves out the colour that surrounds the subject."
  Entries follow the bar's order; the Advanced entries are always listed, whether Advanced is open or not.
- **Closes** by the button again, the page's X, Escape with focus in the page or on the button (focus to the
  canvas), a press on any panel tab or Shift+Ctrl+D/F/H/L (that tab opens), selecting another tool or an
  object that ends the Trace bar, and Escape-to-leave-the-tool. When it closes it restores the tab and the
  shown or hidden state the panel had when it opened (the File tab's inline check rule). Not remembered: the
  tool always starts with it closed.
- **Panel collapsed.** Pressing the button shows the body, as any tab press does; closing the page hides it again.
  **Tabs.** The strip's arrows still move and, while the page shows, activate a tab, which closes the page (button
  releases at once). The page is not a tab, so the strip gains no icon.

### 20.4 Counts in Advanced, states in a state row

- **Counts** ("7 paths, 412 nodes, 0.4 s", muted "38 specks ignored") are the **last row of the Advanced block**,
  tabular figures, not a live region (counts are not announced; the Create notice still is). Closed Advanced shows
  none. A stale or running trace keeps the last counts muted at 50 % in the same words as the outline.
- **State row** (**superseded by section 21.2 and 21.3**: transient states moved into the progress slot, the rest is the notice row; the list below is the old one): exists only while a state is active and sits under the main
  rows, above the Advanced block's divider, `role="status"` polite in an always-mounted container that has no
  height when empty. It holds exactly: "Loading image...", "Tracing..." with **Stop** (both from 1.5 s; the 2 px
  progress line and `aria-busy` start at 400 ms, no row yet, so the bar does not grow for a quick trace; a change
  of the section 4 timing, Proposal), "Tracing at full size..." with Stop at once, "Stopped" with the muted
  "Preview is out of date", "0 paths at this Threshold: too low." / "too high.", "1 path covering the whole image.",
  "412,000 nodes. Slow to edit.", the muted "Preview from a reduced copy". Advanced open or closed does not matter;
  these are states, not counts. With the row gone the bar loses 36 px; nothing above it ever moves. **Failure
  card:** unchanged, under the whole bar (below Advanced when open), `role="alert"`, Retry.
- Tab order (superseded by 21.2): Trace (Select bar), About, Preset, View, Threshold, Smooth, Specks, expander, then when open Invert,
  Ignore background, Denoise, Upscale, Corners, then Stop (when shown), Retry (when shown).

### 20.5 Advanced block and rows at 800 x 600

Rows of 36 px: **row 1** the two toggles (28 px, 4 px apart, 60 px) at the very front, 8 px, then Denoise (168);
**Upscale** and **Corners** (168 each); the counts row last. Panel open (inner 308): toggles + Denoise (236),
Upscale, Corners, counts = 4 rows, 144 px. Panel closed (616): toggles + Denoise + Upscale (412), Corners, counts
= 3 rows (a pair of Corners and the counts is not drawn: the counts row is always its own).

| Bar at 800 x 600 | Panel open | Panel closed |
|---|---|---|
| Main rows, at rest | 4 (156 px): [Trace, About, Preset, View], Threshold, Smooth, [Specks, expander] | 2 (84 px): row 1 + Threshold (482), Smooth + Specks + expander (about 430) |
| With a state row | 5 (192 px) | 3 (120 px) |
| Advanced open, added | 4 rows + divider, about 152 px: bar about 308 px at rest, 344 px with a state row | 3 rows, about 116 px |

(Section 19's counts of 5 and 3 included the result row; at rest the bar is now one row shorter.) Measure at
build; screenshots in the PR.

## 21. Revision 2026-10-29, third round (customer feedback on #216; overrides sections 3, 14, 15, 19 and 20.3 to 20.5 where they differ)

Customer: (1) the info becomes its own entry in the right panel; (2) a stopped preview needs a **Preview**
button; (3) Frage 79 TRACE-PROGRESS = B: progress in row 1, Advanced stays at the far right. No hint text is added.

### 21.1 The info tab (replaces the bar button and the pressed-button page of 20.3)

- **Tab.** Lucide `Info`, the strip's look (28 x 28, 16 px, 1.5 px stroke, `aria-selected` pressed look, 4 px pitch),
  last in the strip after Fonts. Name and tooltip "About Trace", no shortcut (**Proposal**: one more key is not
  worth it for a page read once). The page is the 20.3 page (intro and definition list, text unchanged) with the
  header row "About Trace" at the left; the `X` close button goes, since a tab is closed by choosing another one.
- **When it exists (Proposal).** While the Trace tool is active, whatever is selected, the empty "Click an image
  to trace it" bar included, so the tab does not come and go with the selection. It leaves the tree with the tool
  (the way the Fonts tab's block is conditional, appended last so no other tab moves). Not dimmed: it is either
  there or not.
- **The bar's "i" button is kept (customer correction 2026-10-29: "keep the i button there, please"; the removal
  proposed here is superseded).** Lucide `Info`, 28 x 28, `aria-pressed`, name and tooltip "About Trace", right after
  the "Trace" label. Pressing it shows the panel if hidden and selects the About Trace tab; it is pressed whenever
  that tab is shown (also when chosen in the strip); pressing it again returns to the previous tab and
  shown/hidden state (the stored state below). Row 1 does not get the 32 px back: the label group is
  44 + 6 + 28 again, so the fit figures in 21.2 lose 34 px (measure at build; the row wraps by whole items). Tab order
  has "About Trace" first, as in 20.3.
- **Auto-switch (0043 "Active tab").** Like History, Layers and File it never switches by itself: entering the
  Trace tool does not open it, a selection change neither opens nor leaves it, and the tool always starts with
  another tab active. Choosing it (click, Enter or Space on the focused tab, arrows in the tablist) shows the page and
  stores the tab and the shown or hidden state the panel had. Leaving the tool (another tool, a selection that
  ends the Trace bar, Escape-to-leave) while it is active returns to that stored tab and state; if that tab
  no longer applies, the "Active tab" rule picks (Document with no selection, Style with one).
- **Keyboard.** Tablist as every tab: Tab lands on the active tab, arrows move, Enter/Space activate; a key press
  keeps focus on the strip, a mouse press returns focus to the canvas. Escape with focus in the page or on the
  tab returns focus to the canvas and leaves the page open. The page region is a Tab stop only when it scrolls.
  Shift+Ctrl+D/F/H/L switch to their tabs as always.
- **Panel collapsed.** The strip shows the tab (none pressed). Pressing it shows the body with the page, as any tab
  press does; on leaving the tool the panel hides again if it was hidden before.

### 21.2 Row 1 and the progress slot (Frage 79 B; replaces the 2 px progress line and `aria-busy` bar line of section 4)

- **Order (flow, wraps by whole items):** "Trace" label, Preset, View, Threshold, Smooth, Specks, **slot**, expander.
  Gaps: 6 px inside Trace-Preset-View, **4 px between Threshold and Smooth** (they move closer; no divider between
  them), 8 px elsewhere. The expander has `margin-left: auto`, so it is the last item of the last line, at the far right. The
  slot has no width while empty: at rest nothing differs from section 20.5 except that the label stands alone
  (with the kept "i" button, 21.1: row 1 = about 44 + 6 + 28 + 6 + 132 + 6 + 78 = 300).
- **The slot holds exactly one of four things, 28 px high:**
  1. **Running:** a 75 px pill (1 px `--toolbar-icon` border at 40 %, `rounded-md`) whose ground fills from the left
     with `--editor-accent` at 25 % to the percentage; the text "42 %" (12 px, tabular figures, `--toolbar-icon`)
     at the left; and **×** (Lucide `X`, 16 px, 28 x 28 hit area, inside the pill's right end): name and
     tooltip "Stop (Esc)". Pill `role="progressbar"`, name "Tracing", `aria-valuenow`; no motion beyond the step
     of the number. Without a fraction (Loading image) the text is "..." and the fill is empty. Slot width 75.
  2. **Stopped (Stop, ×, first Escape):** a muted pill (no red; 1 px `--toolbar-icon` border at 40 %, 12 px
     "Stopped") followed by the same 28 px text button **Retry** as in Failed: one control, one name, one Tab stop,
     for both states. Retry traces again, as Enter and a double-click on the image do. Tooltip of the pill (a state,
     not a hint) "Preview is out of date". **(Proposal, 2026-10-29, reconciles the customer's "Preview" button and his
     "Retry after Stop":** two buttons doing one thing are one too many; the label is Retry, because he asked for it
     last and Failed needs the same word. The label "Preview" is the alternative: if the customer prefers it, rename
     it in the Stopped state only.) The words "Preview is out of date" are no longer drawn.
  3. **Failed:** a red pill, 1 px `--field-invalid` ring, 12 px alert glyph and the word "Failed", followed by the
     28 px text button **Retry** (about 100 px together). The reason ("out of memory at 36 Mpx", "internal error") is
     the tooltip and the text of a visually hidden `role="alert"` ("Trace failed: <reason>. Your drawing is
     unchanged."). Red is never the only cue (glyph, word, ring). The failure card of section 14 goes. Enter on the
     canvas retries. It stays until a successful trace, a changed value, another image or leaving the tool.
  4. **Nothing.**
- **Timing.** The slot shows the pill with **×** together at **400 ms** of work (the outline dims to 50 % at the same
  moment); a quicker trace shows nothing. The 1.5 s delay for Stop and the "Tracing..." text are gone. Over 4 Mpx and
  at "Tracing at full size" the pill shows at once (the wait is known). Loading image: pill with "..." at 400 ms.
  At the end the slot empties and the outline is at 100 %; no "done" flash.
- **Fit at 800 x 600.** Panel closed (616): row 1 Trace, Preset, View, Threshold, Smooth = 614 (measure; if it
  does not fit, Smooth wraps to row 2 and the rows below still have room); row 2 Specks + slot (75, 64 or 100) +
  expander = 212 + 8 + 100 + 8 + 28 = 356, fits, so **no growth** while tracing. Panel open (308): row 1 Trace,
  Preset, View; rows 2 and 3 Threshold, Smooth; row 4 Specks (212) and expander (28); the slot needs 75 to 100 more,
  so while it shows it wraps with the expander to a **fifth row** (the bar grows by 36 px, accepted: "wraps as
  usual"). Measure at build; screenshots in the PR. If the customer dislikes the growth: the slot could sit after
  Smooth on row 3 (fits: 168 + 8 + 75), at the cost of Specks moving when closed; not chosen.
- **Tab order:** Trace (Select bar), "i" About Trace, Preset, View, Threshold, Smooth, Specks, the slot's control (Stop or Retry, when shown), expander, then when open Invert, Ignore background, Denoise, Upscale, Corners. Escape stops a
  running trace first, then leaves, as before.

### 21.3 The state row becomes the notice row (replaces 20.4's "State row")

The slot takes the transient states (Loading image, Tracing, Tracing at full size, Stopped, Failed). What is left are
**results the maker should read**, and these stay as one muted line under the main rows, above the Advanced divider,
`role="status"` polite, no height when empty: "0 paths at this Threshold: too low." / "too high.", "1 path covering the
whole image.", "Too much detail: raise Ignore below.", "412,000 nodes. Slow to edit.", "Preview from a reduced
copy" (with "; 38 specks ignored" when both apply). It is named the **notice row**; it holds no controls, so the bar
grows by 36 px only when one of these applies (as the state row did). Counts stay in Advanced only (Frage 78 B).

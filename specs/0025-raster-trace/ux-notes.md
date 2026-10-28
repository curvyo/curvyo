# 0025 Raster trace: UX notes

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
| Several images selected (only images) | Same bar, label "Tracing 1 of 3 images"; the preview shows the image clicked last; Create paths traces all with the same settings, one undo step. |
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
| 1 | State label "Trace"; **Threshold** value field (0 to 255, label inside, `--value-fill` bar, starts at the automatic value and shows it) | 48 + 8 + 168 |
| 2 | **Smooth** (0 to 100, default 60, same field) | 168 |
| 3 | **Specks** ("Ignore below", mm, 0 to 10, default 0.5, muted pixel hint "0.50 mm  ~ 2 px" that follows the image's size on screen) | 212 |
| 4 | Status text (tabular figures, `role="status"`) and **Create paths** (112 px, accent fill) | about 330 |

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
- Status after a settled trace: "7 paths, 412 nodes, 0.4 s"; muted second line "38 specks ignored"
  when specks were dropped. No tooltip advice on the node count.
- The View pill and the `\` peek are slice 3. Until then, to compare, the maker leaves the tool with
  Escape (the dim and the outline go) and enters with `I`; nothing extra is drawn **(Proposal)**.

## 5. Create paths

- Routes: the button (tooltip "Create paths (Enter)": a key name on a control, not a tip), Enter with
  focus on the canvas or a bar button, Ctrl+Enter in a field, double-click on the image. Escape: first
  press stops a running job, second leaves the tool; no dialog, nothing was changed.
- If only the reduced-copy outline is settled, Create paths **waits for the full-size trace**: the
  status reads "Tracing..." and the button shows `aria-busy`; the group is inserted when the trace
  finishes. A new press during that wait is ignored. **(Proposal)**
- On commit: one step "Trace image" does everything of criterion 13: group "Trace of <name>" directly
  above the image, one compound path (black, no stroke), the image stays visible, group selected, Select
  tool, focus to the canvas. The Select bar shows. Ctrl+Z reverses all of it.
- Notice (Action notice next to the rail, 3 s, `role="status"`): "Traced: 7 paths, 412 nodes." With
  several images: "Traced 3 images: 21 paths, 1,204 nodes."
- Zero paths: Create paths is `aria-disabled`, the status gives the reason (section 7). One path covering
  the whole image: allowed.

## 6. Source image stays visible afterwards (Frage 54, decided A)

The image is untouched (visible, unlocked); the paths sit on top on the pixels they came from (criterion
14). The maker hides or moves the image by the usual means (Layers eye, Select). No switch in the bar and
no extra notice text.

## 7. States and wording (permit over refuse; no tips)

| Situation | Text |
|---|---|
| Image loading | Status "Loading image..."; Create paths waits. |
| 0 paths | "0 paths at this Threshold: too low." or "too high." (direction computed from the histogram). **(Proposal)** replaces the spec's "Try moving Threshold toward lighter.", an instruction. |
| One shape | "1 path covering the whole image." Create paths works. |
| Over 100 000 nodes (slice 3) | "412 000 nodes. Slow to edit." Create paths works. |
| Failure (slice 3) | `--field-invalid` card: "Trace failed: <reason>. Your drawing is unchanged." with Retry. |

Removed from the proposal because they are hints: the Photo-preset advice line, the "Fewer nodes edit
and cut faster" tooltip, "raise Smooth or Specks", the tooltip second line of the rail button.

## 8. Keyboard, focus, accessibility

- `I` selects the tool; with the canvas focused and an image selected Tab enters the bar: Trace (Select
  bar only), then Threshold, Smooth, Specks, Create paths (Stop when shown). Shift+Tab reverses; Tab
  from Create paths returns to the canvas.
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
5. Create paths waits for the full-size trace instead of using the reduced copy.
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
| 3 | Specks and **Advanced** (212 + 8 + 96 = 316) |
| 4 | Status, Stop or Trace again, Create paths |

Panel closed (about 628 px, inner 616): 3 rows; row 1 label, Preset, View, Threshold (450); row 2 Smooth,
Specks, Advanced (492); the result row 3. **(Proposal)**

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
and sets the values of criterion 60 only (never Threshold, Node limit or the switches); focus returns to
the button, not the canvas, so the maker can keep adjusting.

**Custom.** The label comes from the stored state `{ preset, edited, values }` (architect, 2026-10-28;
no value comparison anywhere). A pick sets `preset` and `edited = false`; any edit of Smooth, Specks,
Corners, Upscale or Denoise sets `edited = true`, and the button then shows "Custom". The menu marks and
focuses `preset`, the preset the values came from. Threshold, Node limit, Invert, Ignore background and
Full-size preview are not preset values and never cause "Custom". Putting the values back by hand stays
"Custom"; picking the preset again clears it (criterion 40).

**Status and notice.** A pick prints nothing: no notice, the status line does not name the preset
(criterion 66); the preview and counts update after the 150 ms debounce. With several images the pick
applies to all (label "Tracing 1 of 3 images" unchanged).

## 12. Advanced block (criteria 41, 43, 44)

- **Button** "Advanced", text, 28 px, same row as Specks, `aria-expanded`, never hidden or dimmed. Pressed =
  open. Open and closed is remembered as an app setting. This is the customer's "second row for rare
  functions": no dropdown, no icon.
- **Block** below the result row inside the same pill, separated by a 25 % divider; the bar only grows
  downward, nothing above moves. Two columns of 168 px, rows of 36 px, in this order: Denoise (3-segment
  pill Off, Low, High) and Upscale (Auto, Off); Corners (value field, 0 to 100) and Node limit (number field,
  empty = none, thousands separator on blur); Invert and Ignore background (a 28 px high `role="switch"`
  control, label left, switch right, the whole row is the target); Full-size preview. 4 rows, 144 px; the bar
  ends about 340 px below its top at 800 x 600, inside the 560 px minimum. Labels are drawn in full (the
  mockup's "No backgr." and "Limit" are cut or unclear). **(Proposal)**
- **Honest controls.** A control the tracer cannot honour is not drawn (criterion 41); no placeholder.
- **Full-size preview over 4 Mpx** (criterion 52) cannot be honoured: the switch is `aria-disabled`, off, with
  the muted state text "Over 4 Mpx" beside the label. **(Proposal)**
- **Node limit** outcomes use the status text of criterion 43, no extra control; the field keeps what the
  maker typed. Invert and Ignore background have no status text.
- Escape in an Advanced field returns focus to the canvas; Tab leaves the block to Create paths.

## 13. Slice 3: View pill and peek (criterion 51)

- **Pill** right of Preset in row 1, `role="radiogroup"` "View", three 25 x 28 px icon segments (16 px
  glyphs, the Select bar's pill look, selected = `--active-bg`): **Overlay**, **Trace**, **Image**. Tooltips
  name the segment (Overlay, Trace, Image); the Image tooltip adds the key as a key name ("Image (\)"), the
  pattern of "Create paths (Enter)". Arrows move and select, no Tab stop per segment.
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
- **Stopped** (Stop or first Escape): status "Stopped", the same muted line, outline at 50 %; Stop is
  replaced in the same slot by **Trace again**. Create paths is `aria-disabled` (its reason is the status).
  Changing a value, Enter, or Trace again traces; Enter while stale is "Trace again", never Create.
  **(Proposal)** The slot order is status, Stop or Trace again, Create paths; Tab follows it.
- **Over 4 Mpx:** the preview is the reduced copy; the muted second line reads "Preview from a reduced
  copy", with "; 38 specks ignored" appended when both apply (about 280 px, fits). Counts are those of the
  reduced copy; the success notice carries the full-size counts. Create paths shows the progress line and Stop
  at once (no 1.5 s delay: the wait is known), status "Tracing at full size...". Escape stops; the document
  is untouched until the trace finishes. **(Proposal)**
- **Over 100,000 nodes:** status "412,000 nodes. Slow to edit." (comma separators), Create paths works;
  no confirmation.
- **Failure card.** Under the bar, 8 px gap below its bottom edge (below the Advanced block when open),
  same left edge and width, the refusal look of the Action notice: `--popover` ground, 1 px
  `--field-invalid` ring, 12 px alert glyph, text "Trace failed: <reason>. Your drawing is unchanged.",
  `role="alert"`, plus a **Retry** text button (28 px). Red is never the only cue (glyph, words, ring).
  Reasons are plain ("out of memory at 36 Mpx", "internal error"); no stack, no code. It does not take
  focus; Retry is the last Tab stop; Enter on the canvas also retries. It stays until the next successful
  trace, a changed value, another image or leaving the tool; values stay. Create paths is `aria-disabled`.
  A worker that died is replaced on Retry. **(Proposal)**
- **Pan and zoom** with a big preview: cached outline, culled (criterion 56); no state text.

## 15. Keyboard and accessibility, slices 2 and 3 (criterion 57)

Tab: Trace (Select bar), Preset, View (one stop, arrows inside), Threshold, Smooth, Specks, Advanced, the
Advanced controls when open, Stop or Trace again (when shown), Create paths, Retry (when shown). Shift+Tab
reverses. The menu follows the menu pattern of section 11. `\` and Shift+`\` work with focus on the
canvas or a bar button. Every state change not visible on the canvas is text in the status row or the
alert card, announced once. Hit targets 28 px, focus ring 2 px `--editor-accent`. Contrast of muted text
and the 50 % outline checked in both themes. Nothing animates, so reduced motion needs no case.

## 16. Slice-1 leftovers (criterion 68)

(a) The Layers row of the trace group carries the tooltip text of the success notice ("Traced: 7 paths,
412 nodes."). (b) The wash of section 4 is checked in both themes by a test (white wash on light; canvas
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
| 5 | Specks hint "~ 6 px" | "0.50 mm  ~ 2 px" live from the image | Fixed |
| 6 | "9 180 nodes" | "9,180 nodes" | Fixed |
| 7 | "0 paths. Try a lighter Threshold." | "0 paths at this Threshold: too low." (or too high); no instruction | Show at demo (criterion 67: spec wording wins unless the customer picks the mockup's) |
| 8 | Advanced labels "No backgr.", "Limit 20000 nodes" | "Ignore background", "Node limit", empty = none | Fixed |
| 9 | Advanced has no Full-size preview | Switch added (criterion 41); disabled over 4 Mpx | Fixed |
| 10 | Computing: "Tracing..." and "Preview is out of date" with Stop | Same; after Stop the slot becomes "Trace again" | Show at demo (new button) |
| 11 | Failure: red text with Create paths beside | Card with glyph, ring, Retry, under the bar; Create paths disabled | Fixed (red not the only cue) |
| 12 | After Create: image hidden, notice "Source image hidden." | Image stays visible (Frage 54 A); notice without that sentence | Fixed |
| 13 | Menu second lines (Logo / Line art "Sharp corners, clean curves.") | Kept, with the customer's texts (criterion 59) | Fixed |
| 14 | Bar "four rows" | 4 rows panel open if Threshold and Smooth pair, else 5; 3 closed | Fixed after measuring |
| 15 | View pill | Slice 3; chosen view not remembered | Show at demo |
| 16 | Advanced button with a chevron | Text button, pressed fill, no icon (section 12; no dropdown icons, customer rule) | Fixed |
| 17 | Specks value "0.50mm" with the unit in the field, label "Specks" | Label "Ignore below", value without unit, muted "0.10 mm  ~ 0 px" beside it; "~ 0 px" at small values | Show at demo |
| 18 | Threshold and Smooth as label plus white value box on a fill | One fill field, label inside, value right (section 3) | Fixed |
| 19 | Panel open: bar four rows | Measured 5 rows at 800 x 600 (bar 324 px wide, Threshold and Smooth do not pair); panel closed 3 rows as planned | Fixed (row 14 resolved: five) |
| 20 | Menu: current item shown by fill only | Fill plus check mark (not colour alone) | Fixed |
| 21 | Zero paths: one-line status | At panel open "0 paths at this Threshold: too low." wraps to two lines and the bar grows 12 px | Show at demo |
| 22 | Advanced fits its pill at 800 x 600 | Panel open: the button ends 50 px right of the pill, over the right panel | To fix (review finding B1) |
| 23 | Advanced block: Denoise Off / Low / High, two columns inside the pill | "High" clipped (pill container 104 px, segments need 121 px), panel open also Upscale, Node limit, Ignore background overflow the pill | To fix (review findings B2, B3) |

Reviewed at 800 x 600, panel open and closed: bar rows, Preset menu (keyboard, Custom), Advanced block, zero-path
status, Create paths, Layers tooltip, Tab order. Not exercised: computing and Stop, failure card, several
images, nothing selected, dark theme, Full-size preview over 4 Mpx.

## 18. Open for the lead

Answered: the Preset menu's second lines are the customer's own texts (Frage 64 A, 2026-10-28); they stay.

# 0025 Raster trace: UX notes

Author: ux-engineer. Date: 2026-10-27. Status: for Ready. Builds on `ux-proposal.md` and
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

Slice 2 inserts Mode, Preset and View as a row **above** these (mockup order, "Look" first); the fields
move down once, in that PR, and the PR description says so. Advanced joins row 3 then, below row 4
(proposal 3.3). **(Proposal)**

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
6. Slice 2 inserts the Look row above the slice 1 rows (one-time shift).

## 10. Decided by the customer (2026-10-27)

**Frage 61 (TRACE-RAIL):** "Belongs in the top bar of the Select tool when a raster image is selected;
several can be selected. The toolbar needs tidying later in polish; first we make chips." Option A
stands, extended to several selected images (section 2). Colours are postponed (Frage 50 A), so the
Mode pill, Colours field and Photo preset of the proposal are not drawn.

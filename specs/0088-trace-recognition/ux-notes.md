# 0088 Trace recognition, Part A (Recognise text): UX notes

Aligned 2026-10-31 with the customer's answers to Frage 101 to 106 (passages marked "Changed 2026-10-31"):
guessed letters are reported, not outlined; the Faces readout shows tiers; the dashed leftover overlay is gone.

Status: Ready (was Draft). Owner: ux-engineer, 2026-10-30. Written against the specification (Frage 70 to 73
decided), `adrs.md` (decisions 5, 6, 11; REC-PARTIAL C, REC-FACES B tiers, REC-NEAREST A as decided 2026-10-31), the Trace bar
(`0025` §21, `0089`), the Select bar rows of `docs/design-system.md` (Stroke to path, Linked clone, Action notice,
Hidden not disabled) and the Fonts tab (`0075`). Everything marked **(Proposal)** is mine and vetoable at the demo.
Part B (shapes) will reuse this bar with a second label and its 0.2 mm default; nothing here blocks that.
No hint texts: every string names a control or states a state or a result (customer, 2026-10-16). Only
the questions in §9 are open, each with a default.

## 1. Entry point (Proposal)

Recognition is an operation with a value, so by the customer's rule it is a **tool with an options bar**, not a
dialog and not a panel section. The rail is full (22 buttons, `0025` §1), so it follows the Trace precedent
(Frage 61): **a text button "Recognise text" in the Select bar that chooses the Recognise tool**, bar below.

- **Button:** the look of "Linked clone" (28 px, 1px `--toolbar-icon` at 60 % outline, `rounded-[5px]`, 14 px
  label, no glyph), **appended last in a group of its own**, after Linked clone, so no earlier control moves.
  Hidden, not disabled (Changed 2026-10-31, lead, permit over refuse: closed paths without fill count too, e.g. a text after
  Object to path; a closed outline is read as filled): in the tree while the selection holds at least one closed path, compound path or group
  (a traced group counts); absent with another tool than Select, with nothing selected, or with images or texts
  only. Press = choose the tool; the mouse press returns focus to the canvas, Space/Enter keep it on the button.
  Tooltip (Radix, 400 ms, bottom, one line): "Recognise text". No chord (Proposal; the operation is rare and one
  press away; Tab reaches the button last in the Select bar, then Enter).
- **Not in the Trace bar.** The Trace bar is the fullest bar in the app (8 to 12 rows at 800 x 600); a second
  action there would move its finish button. A traced result is selected after Create paths, so leaving Trace
  (Escape) and pressing "Recognise text" is two steps. The trace bar's notice row stays free of cross-tool
  offers (no tips).
- No menu entry (there is no menu bar since `0051`), no context menu entry, no rail button.

## 2. The Recognise bar

Replaces the Select bar in place, overlay row, left-aligned at `--rail-right` + 12, `role="toolbar"` "Recognise
text", pill look and 36 px rows of the Trace bar. One flow of whole items, 8 px apart, wrapping by whole items.

| # | Item | Width | Notes |
|---|---|---|---|
| 1 | State label "Recognise text" | about 100 | 12 px, `--toolbar-icon`, not a control; names the mode (no rail button is pressed, as in Trace) |
| 2 | **Tolerance** | 168 | the shared `ValueField` (`0025` §20.1), unit `mm` shown, default 0.3, 0.05 to 2.00, step 0.05 (Proposal), two decimals, drag-to-scrub like Smooth; criterion 6 |
| 3 | **Faces** readout | about 130 | Changed 2026-10-31 (Frage 102 B, 105 A; wording is a Proposal). 12 px muted text, a state, not a control, not a Tab stop. Before a run: "5 face sources". After a run, in the notice row's place of the Select bar it is gone (the bar closes), so the matched tier is named in the result notice (§4); after "nothing within tolerance" it reads "Searched: system fonts". Tooltip lists the order: "Document, favourites, installed, included, system fonts" (Changed 2026-10-31, lead: no hint text) No selector; the order is fixed |
| 4 | **Finish button** "Recognise" | 75 x 28 | always blue like Trace's finish (`--editor-accent` fill, white 14 px text), same shape at rest and while running; Enter on the canvas does the same |

At 800 x 600, panel open (inner 308): row 1 = label + Tolerance = 276; row 2 = Faces + button = 213, button
right-aligned. **Two rows, 72 px**; panel closed (616): **one row, 36 px** (100 + 168 + 130 + 75 + 24 = 497).
No Advanced block and no expander: there is nothing rare to hide (a second row is only for full bars). The
notice row (§4) adds 36 px under the flow when it holds a result. The PR measures and reports these.

**Canvas.** The selection stays as it is, drawn as in the Select tool. There is **no live preview**
(Proposal): recognition is a one-shot with a fixed result, 0.3 s per line (ADR 8); a preview that must run the
matcher on every tolerance change would run into the 1.3 s first run and would invite scrubbing that waits. The
tolerance is a number the maker sets, runs, reads the deviation of and, if needed, undoes and sets again.
The one canvas cue before the run is the existing selection outline of the paths that will be read; the tool has
no handles because the operation has no position, only a tolerance (handles would be decoration).

**States of the bar (same position, nothing else changes):**

| Selection | Bar |
|---|---|
| Closed paths (filled or not) or groups with paths | normal; the label "Recognise text" |
| Nothing selected | label and text "No paths selected.", button absent |
| Only images, texts, clones, open paths | label and text "No closed paths selected.", button absent |

Both texts state what is there, not what to do. Nothing is refused (permit over refuse): objects that cannot be
read (images, open paths, outlines of more than 4,000 points) are skipped and counted in the result (§4).

## 3. Run, progress, stop

- **Start:** the blue button, or Enter on the canvas or in the Tolerance field (a field commits first).
- **Running:** the Trace button behaviour, copied so it needs no explanation: after **400 ms** the button shows
  "42 %" and an x (Lucide `X` 16 px, 28 x 28 hit area, name and tooltip "Stop (Esc)") inside the same blue
  75 x 28 shape, for at least **300 ms**; a quicker run shows nothing. `role="progressbar"` named
  "Recognising", `aria-valuenow`; the document is not touched until the end, so Stop has nothing to undo.
  **Dependency for the implementer/architect:** ADR 8 runs recognition synchronously in the editor session.
  A synchronous call cannot paint a percentage or take Escape. Proposal: the run yields to the event loop
  between lines and between faces (the 64-face first run is the long part), reporting `lines done / lines`. If
  the architect keeps it one blocking call, the button shows "..." without x after 400 ms and the limitation is
  filed in `docs/technical-debt.md`; no other UX changes.
- **Stopped** (x or the first Escape): no change to the drawing; the button turns into the muted "Stopped" pill
  plus **Retry** (same as Trace §21.2 item 2). **Failed**: the red-ringed pill with alert glyph and the word
  "Failed", Retry, reason in the tooltip and in a visually hidden `role="alert"` ("Recognise text failed:
  <reason>. Your drawing is unchanged."). Both clear on a changed Tolerance or a selection change.
- Escape order: stops a running job first, then leaves the tool back to Select with the selection kept.

## 4. Result feedback

Success and "nothing within tolerance" are different outcomes and are shown in different places.

**Something was recognised (leaves the tool):** the bar returns to the Select bar (selection = the new text
objects and the leftover path, §5) and the sentence appears as the **Action notice** under the "Recognise text"
button (4 px below, right-aligned, `role="status"`, 8 s because it is over 70 characters, or until the next
action). It carries what criteria 3 and 6 and the ADR need: count, face, **largest deviation**, leftovers.

| Case | Notice |
|---|---|
| All sure, one face | "Recognised 12 letters as Source Sans 3 Bold (included fonts), largest deviation 0.12 mm." |
| Some guessed (Changed 2026-10-31, REC-PARTIAL C) | "Recognised 12 letters as Source Sans 3 Bold (included fonts), largest deviation 0.12 mm; 2 letters guessed: e (5), l (9)." Positions are 1-based in the text; more than 5 guessed: "... e (5), l (9), t (12), a (14), r (20) and 3 more." The deviation covers the sure letters only |
| Several lines, same face | "Recognised 28 letters in 2 lines as Source Sans 3 Bold (document fonts), largest deviation 0.14 mm; 2 letters guessed: e (5), l (9)." For several lines the position counts within that line's text |
| Several faces | "Recognised 28 letters in 2 lines, 2 faces (Source Sans 3 Bold, document fonts; Great Vibes, favourites), largest deviation 0.14 mm; 2 letters guessed: ..." |
| Lines or shapes left (under 50 % sure, merged letters) | appended: "; 3 outlines left as paths." |
| Face not installed, within tolerance | the same sentence with the nearest face; no extra wording (the face is named, the deviation is stated) |
| Skipped objects | appended as in Stroke to path: "1 image and 2 open paths were left as they are." |

**Nothing within tolerance (REC-NEAREST A; stays in the tool, nothing changes):** the **notice row** of the
Recognise bar (muted 12 px, `role="status"`, no height when empty, 36 px when it shows; the Trace bar's notice
row pattern): "No text within 0.30 mm; nearest face Source Serif 4, 0.84 mm." The tolerance field is right above,
so the maker can raise it and press Recognise again; the sentence is cleared by any change of Tolerance, of the
selection, or by a new run. If no line was found at all (nothing letter-shaped): "No text lines found." The
number is shown with two decimals and the tolerance with the field's precision, so the numbers can be compared.

**Faces of a result.** The recognised text is an ordinary text object (`0074`): Font, Size and the text bar work
unchanged. The "pick another face from the 0062 list" of criterion 3 needs no new control: the Select bar's
Font button changes the face of the selected texts. After a face change the deviation is no longer guaranteed;
nothing is recomputed or shown (the maker chose it). Size, spacing and angle are the registers of `0074`/`0079`
and appear in the Text controls as for any text.

## 5. Guessed letters are reported, not drawn (Changed 2026-10-31, Frage 101 C and 106 B)

No dashed outline and no canvas overlay. After a run the selection is **everything the run produced**: the text
objects and, if any, the leftover compound path of outlines that were not converted (lines under 50 % sure, merged
letters; still in the layer order at its place, original fill and stroke, ADR 6). The Select bar subject names it
("2 texts, 1 compound path").

Guessed letters are marked in three places, all existing UI (Proposal, vetoable at the demo):
1. the result notice (§4) lists letter and position: "2 letters guessed: e (5), l (9)";
2. the new texts are selected, so the maker can open the Text tool or the Glyph table (`0082`) straight away;
3. while the session record lives (ADR 6: until the first edit of that text, a selection change by the maker, or
   undo), the Glyph table and the Text tool show the guessed characters with their ordinary selection highlight.
   Nothing is saved with the file; no colour, no token, no hint text.

The leftover is one compound path, so the next moves are existing ones: Break apart (Path card), Recognise text
again with a higher tolerance (already recognised texts are skipped).

## 6. Undo

One run is **one history step "Recognise text"** (criterion 6; ADR 14): the original paths come back exactly,
with their selection; Redo restores the texts and the leftover. The History tab row has that name. A stopped,
failed or empty run adds no step. The tolerance value is a per-user preference stored in the webview
(`curvyo.recognise-tolerance`), not in the project, not an undo step; Undo of the run does not revert it.

## 7. Keyboard and accessibility

- **Tab order:** the Select bar's last button "Recognise text" (entry); in the tool: Tolerance, the finish
  button (or Stop, or Retry), then the canvas. The Faces readout and the label are not stops.
- **Keys:** Enter runs (canvas, field, button); Escape stops, then leaves; Up/Down in the Tolerance field step
  0.05, Shift+Up/Down 0.5, Page Up/Down 0.5 (the `ValueField` rules). No letter key to enter the tool and no
  chord (Frage 104 default).
- **Focus:** entering with the mouse returns focus to the canvas; with Space/Enter it goes to Tolerance. After
  a run leaves the tool, focus goes to the canvas (mouse) or the Select bar's "Recognise text" button (key),
  where the notice is anchored.
- **Announcements:** the live region exists from the start; the result sentence is the text change, once.
  Running is a `progressbar`; Failed has the `alert` text. No colour-only cue: Failed has glyph, word and ring;
  guessed letters are listed in words in the notice (Changed 2026-10-31). Contrast: `--toolbar-icon` on `--toolbar-bg` as the Trace bar (measured there);
  white on `--editor-accent` for the button as Trace's. Hit targets 28 px as every bar.
- **Themes:** light and dark use existing tokens only; this feature adds no token.

## 8. Behaviour with large documents (checked at review)

- 500 selected traced paths: the run is bounded by ADR 8; the bar shows progress after 400 ms and Stop works at
  line granularity. The Select bar button stays in the tree (counting closed paths is cheap).
- Outlines over 4,000 sample points, open paths, images and texts are skipped, not an error: counted in the
  result ("... were left as they are").
- Selected objects spanning several layers or locked layers: locked objects are skipped and counted like the
  others; the result stays one step.
- Offline: no network state to show; nothing in the bar mentions it (criterion 5 is tested, not displayed).

## 9. Decided by the customer (2026-10-31; replaces the open questions)

- Frage 104 REC-ENTRY = A: text button in the Select bar, own bar with tolerance and button (this draft).
- Frage 105 REC-SCOPE = A: readout only, no selector; it names the tiers / the tier that matched (§2, §4).
- Frage 106 REC-MARK = B: no dashed outline; affected glyphs are selected and reported (§5).
- Frage 101 REC-PARTIAL = C and 102 REC-FACES = B (tiers) shape §4 and §5; see the specification.

## 10. Proposed additions to `docs/design-system.md` (architect/lead apply at build, not done here)

Rows "Recognise bar", "Recognise text button" (Select bar, last group, Hidden not disabled), "Recognise notices",
"Guessed letters" (notice list plus session highlight in Glyph table and Text tool; Changed 2026-10-31, no overlay), and the tolerance preference key; Keyboard table: Recognise
text has no key. No new token.

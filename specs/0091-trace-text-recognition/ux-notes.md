# 0091 Trace text recognition: UX notes

Owner: ux-engineer, 2026-11-01. Builds on `specs/0089-trace-quality-and-colour/ux-notes.md` §22 (tidy-up: bar holds the
straightforward path, Advanced the rest) and §23 (no success notice), `specs/0088-trace-recognition/ux-notes.md` §4 and §5
(guessed letters), `docs/design-system.md` "Interaction conventions" (no success notices, no hint texts). Every choice marked
**(Proposal)** is mine and vetoable at the demo. No tokens, no new component.

Principle: the feature adds one switch, no bar item. Its result is the text objects themselves (the feedback), the cleaner
trace, and one line in the Advanced status line. Nothing is announced when it worked.

## 1. The switch "Recognise text"

- Where: Advanced, options row (22.3 item 5), **third icon toggle** after Invert and Drop border shapes (**Proposal**: same
  `TraceToggle` look: 28 x 28, `ICON_BUTTON_CLASS`, pressed = `--toolbar-icon-active-bg` / `-fg`, no label drawn, 4 px gap).
  Icon: Lucide `ScanText` 16 px, 1.5 px stroke (not `Type`: that is the Text tool). Name and tooltip "Recognise text" (400 ms,
  `side="bottom"`), `aria-pressed`. State by fill and `aria-pressed`, not colour alone (as the other two).
- **Off by default** (customer, Frage 116 B a, 2026-11-01), an app setting like the other Trace settings, never in the document (criterion 17).
- "Recognise shapes" is **not drawn** until 0088 Part B: no disabled twin, no placeholder, no hint label.
- While Multicolour is on, the switch "Recognise text" is **not drawn** in slice 1 (a control that cannot be honoured is not
  drawn; slice 2 brings it). The setting keeps its value and returns when Multicolour is turned off. (Proposal, lead 2026-11-01)
- Not in the main bar, not in presets (**Proposal**: presets stay pure Trace settings; the switch is global like Invert).
- Turning it on or off marks the preview stale like any setting and re-previews. Turning it on for an image without cached
  lines runs the OCR (section 3); with cached lines it costs no OCR time (criterion 16).

## 2. Preview: what each view shows

The preview equals the result (ADR 0023 §4), so recognised text is shown as what Create paths will make.

| View | With Recognise text on (accepted lines exist) |
|---|---|
| **Trace** | The traced paths **without** the letters (in-painted, shape whole under text, 113 A), plus each accepted line drawn as text in its nearest font, black (Slice 2: its palette colour), above the paths, at its final place, size and rotation. Same renderer as the document, so no second look. A line set in the nearest font looks like any other; its guessed state is not drawn (Frage 106 B: no overlay). |
| **Overlay** | As today (dimmed image under the result), result as in Trace. The image beneath still shows the letters; the drawn text lies on them. |
| **Mask** | The binary image handed to the main trace, pixel-equal as `0089` §4 and criterion 21: the letters are **absent** (filled from their surroundings), which is the proof they are excluded. **(Proposal)** The pixels left out (the lines' masks) are drawn in a third tone, flat mid-grey `#808080` (3.9:1 on white, 5.3:1 against black, so it is a luminance step), so "was my text captured" is answerable. Only those pixels differ from the exact mask; with Recognise text off or no accepted line the Mask is exactly as before. No legend: the status line (section 4) names the lines. |
| **Image** | The source, untouched; `\` peek unchanged. |

- Stale preview: all of the above at 50 % with the existing words. Over 4 Mpx: the reduced copy, quads scaled by its factor,
  the existing muted line "Preview from a reduced copy".
- Lines the OCR found but did not accept (confidence under 0.6, no letter or digit) are traced as paths and not marked.
- No line boxes, no confidence overlay, no click on a line (out of scope: "overlay or preview markers").

## 3. Progress: reading the text

The OCR is a stage of the first preview of an image in the session (ADR 0091 decision 1), up to about 5 s at 2000 x 2000 px,
once per image. It reuses the existing slot, not a new surface.

- **Inside the blue Trace button**, the "running" slot as built: the bar fill and the tabular percent, the X stop button, the
  button stays 75 px wide. The percent is one progress from 0 to 100 % over the whole job (reading, font fit, trace; customer,
  Frage 116 B b, 2026-11-01, criterion 25). It never runs backwards and does not restart at 0 for the trace stage.
- The progressbar's `aria-label` is "Reading text" during the OCR stage and "Tracing" after (a state name, not a hint). The
  first fetch of the weights, if it is visible at all (bundled, so about 0.2 s), uses the existing "Loading" state.
- Advanced status line (muted, while running only): "Reading text". Gone when the stage ends. Nothing in the bar's notice row;
  "Tracing 1 of 3 images" stays as built and covers each image's own OCR in turn.
- The previous result stays on the canvas at 50 % (stale) until the first preview is ready; the preview does **not** show a
  text-less trace first and swap in the text later (**Proposal**: no flicker of letters filled, then redrawn).
- **Stop** (X, or Esc as built): the OCR job is terminated, nothing is cached, the document is unchanged and the slot shows
  the existing "Stopped" with Retry (criterion 19). Retry runs the OCR again.
- **Failure** (model cannot be loaded or read; should not happen with bundled files): **permit over refuse (Proposal)**: the
  trace runs as if Recognise text were off and the bar notice row says "Text recognition failed. Traced without text." It stays
  until the next preview. This is the one notice of this feature that belongs in the bar: it changes how the result must be
  read and the maker cannot see it with Advanced closed. The slot shows no "Failed".
- Later previews of the same image (any Trace setting changed): no OCR stage, no "Reading text", nothing different from today.

## 4. Advanced status line

The status line (22.3 item 2) keeps its two lines: counts, then the muted line. **(Proposal)** the lines found go into the
**muted line**, after the existing segments, joined by " · ", and nowhere else:

| State | Muted line (example with 38 specks) |
|---|---|
| All accepted lines set | "38 specks ignored · 3 text lines set" |
| Some not accepted | "38 specks ignored · 3 of 5 text lines set" |
| Found, none accepted | "0 of 2 text lines set" |
| Exactly one | "1 text line set" |
| No line found, or switch off | nothing added; identical to Recognise text off, no notice (criterion 18) |

- Counts line unchanged ("7 paths, 412 nodes, 0.4 s"); the text objects are not counted as paths.
- 50 % while stale, no height when empty, `role="status"` polite as built. The whole thing is a count of what the preview
  holds, not a report that something worked; after Create paths the line is the same as before it (no new line appears).
- No notice after Create paths (§23): the selection and the History step are the feedback.

## 5. Guessed letters, including lines in the nearest font (114 B)

Only a **warning** appears, and only when at least one letter is guessed (criterion 5); none when all are sure. Same slot,
lifetime and Glyph table / Text tool highlight as `0088` §4 and §5 (session record, ends at the first edit). One notice, up to
two sentences, partial guesses first. Positions are 1-based within that line's text (several lines: as `0088` §4 table).

| Case | Wording |
|---|---|
| Some letters guessed in a matched line | "2 letters guessed: e (5), l (9)." (`0088` wording and the "and 3 more" cut after 5, unchanged) |
| One line set in the nearest font | "1 line set in the nearest font, all letters guessed: "Müller & Söhne"." |
| Several | "2 lines set in the nearest font, all letters guessed: "Müller & Söhne", "Open daily"." |
| More than 2 | `... "A", "B" and 3 more.` |
| Both | "2 letters guessed: e (5), l (9). 1 line set in the nearest font, all letters guessed: "Open daily"." |

- Quoted text is the OCR string, cut at 24 characters with "…" (so a line is identifiable, not reproduced).
- Letters of whole-guessed lines are not repeated by position (14 numbers would be noise); the quoted line is the pointer, the
  highlight in the Glyph table / Text tool shows each letter.
- Strings go into `curvyo-ui-core` (`image_clipboard_text`-style text module), with the counts as arguments, not built in
  the frontend.

## 6. What Create paths selects

- Selection after Create paths (**Proposal**): the new "Trace of" group **and** the text objects of this trace, so Delete,
  Move and Undo reach the whole result; one history step (criterion 11), one undo removes all.
- Layers: text objects sit above the group (113 A), named by their text (the ordinary text-object name). The group row
  tooltip stays "Traced: 7 paths, 412 nodes." (§23).
- With several images: each image's group and texts as one set per image, all selected, as the groups are today.
- Focus returns to the canvas as built. No notice.

## 7. Keyboard

- Switch: one Tab stop in the options row, after Drop border shapes and before Denoise; Space and Enter toggle (Enter does
  **not** create, it belongs to the control); Escape returns focus to the canvas; Ctrl+Enter creates. No letter shortcut
  (**Proposal**: a rarely flipped setting; matches Invert and Drop border shapes).
- Stop: Escape while running stops the OCR/trace as built; order unchanged (stop, disarm pipette, leave tool).
- Rest of the Tab order unchanged (22.4); Advanced still moves nothing above it. About Trace (`i`) gets one entry for
  Recognise text in the Advanced order, describing the control (reads the text first, sets it in the nearest font, leaves it
  out of the paths); a test compares it with `docs/tracing-images.md`.

## 8. Impact at 800 x 600 and large images

- **Main bar: nil.** No new item, same rows, the Trace button stays 75 px; the percent and X fit as today.
- **Advanced:** the third toggle makes the toggle group 92 px (was 60). Panel closed (588): 92 + 3 x 168 + 24 = 620 > 588,
  so Corners wraps: one more row (36 px), Advanced about 186 px instead of 150. Panel open (308): 92 + 8 + 168 = 268 fits,
  no change. Single colour Brightness is the case measured; the PR re-measures. Allowed escape: the toggle may share the
  Output row if it fits (implementer's call, as the Output / Invert rule of 22.7). The block's own `min(60vh, 360px)` scroll
  applies as built.
- Large images: OCR runs on the full working image regardless of the reduced preview copy, so the first-preview wait is the
  OCR time at any size (progress shown, stop available); later previews do not wait.

## 9. Strings (exact)

Control and state names: "Recognise text" (toggle, tooltip, aria-label); "Reading text" (progressbar label, muted status
line); "N text line(s) set", "N of M text lines set" (muted status line); "Text recognition failed. Traced without text."
(bar notice); the guessed warnings of section 5. No other new strings. No tooltip beyond the name, no second line.

## 10. Docs and spec changes for the implementer

- `docs/design-system.md`: add the toggle to the "Trace Advanced button and block" row (order of 22.3 item 5, third icon
  toggle), the Mask third tone to the "View pill / Mask" description, and the guessed-line wording to the Glyph table and
  recognise rows. No new row for the status line.
- `specification.md` "UX notes": replace the placeholder with a pointer to this file.
- `docs/tracing-images.md` and the About Trace strings: the Recognise text entry (section 7).

## 11. Questions

None open. The two I would look at first at the demo: the **grey tone in Mask** (section 2; veto it and Mask stays strictly
the main trace's binary image, letters simply missing) and the **selection of group plus texts** (section 6; veto it and
only the group is selected as today).

## 12. Review checklist (for the UI review)

Switch present, third in the options row, off by default, absent "Recognise shapes"; Trace view shows text objects at their
final place and no letter holes; Mask pixel-equal except the grey left-out pixels; the OCR percent in the Trace button with
stop working and nothing cached after stop; no notice when nothing is guessed and none after Create paths; the warning wording
of section 5; one undo removes group and texts; Tab order and Escape; main bar identical at 800 x 600; contrast of the grey
(3.9:1 on white) and of the toggle in light and dark.

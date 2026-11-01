# 0090 Grid: UX notes

Status: Draft for Ready. Owner: ux-engineer. Authority: `specification.md` (customer decisions of
2026-10-31). References: `docs/design-system.md`, `specs/0060-snapping/ux-notes.md` (called "0060 §n"
below; the Grid entry and panel copy its rules unless stated), `PanelStrip.tsx`, `SnappingSection.tsx`.
**(Proposal)** marks reversible choices. Customer rule 2026-10-16: no operating tips or hint labels;
labels that name a control or a state stay. Everything here follows it, so there are no field tooltips.

## 1. Rail entry

- One persistent **Grid** tab in the Properties strip, directly after Snapping, before contextual
  About Trace. Lucide `Grid3x3`, 16px, 1.5px absolute stroke, same 28px hit target (36px coarse),
  inset focus ring, `role="tab"`, accessible name `Grid`.
- Plain click, Enter, Space: open the Grid pane, never change the shown switch. Shift+click toggles the
  **active page's** grid shown/hidden, keeping the displayed pane, scroll and collapse state. Shift+Enter
  and Shift+Space are the keyboard form, once per press, ignoring repeats and suppressing the synthetic
  click and Space scroll. All of 0060 §1.2 applies (modified gesture intercepted before the tab
  primitive's activation; pointer returns focus to the canvas, keyboard keeps it on the tab).
- Pane is sticky across selection, tool and group changes. It follows the **active page**: switching
  page while the pane is open shows that page's values in the same frame; the rail state follows too.
- State table as 0060 §1.3, "shown" replacing "enabled": subtle `--editor-accent-hover` ground plus
  editor-accent glyph plus the small check badge when shown and not selected; strong selected-tab look
  when the pane is open, badge in the foreground colour when also shown. `aria-selected` means pane open
  only. Accessible description `Shown` / `Hidden`; do not add a second switch role to the tab.
- Tooltip: `Grid — on` / `Grid — off` (names the state, like `Snapping — on`). **(Proposal)** no shortcut
  line, per the customer rule. Note for the lead: the current Snapping tooltip still carries
  "Shift-click or Shift+Enter/Space to toggle." If that line is to go, it goes in a separate fix, not here.
- Shift toggle announces `Grid shown` / `Grid hidden` once through the existing status mechanism.

## 2. Grid panel

Fixed header `Grid`, 280px body / 244px content, one vertical scroll area, same fieldset-disabled-during-
canvas-drag rule as Snapping. All controls are always enabled, also with the grid hidden (criterion 3).

| Order | Control |
|---|---|
| 1 | `Page` row: label left, page name right in `--panel-muted-fg`, one line, ellipsis, full name in the accessible name. Read-only text, not a control, not focusable. Without a user name: `Page 2` (position). |
| 2 | `Grid` switch (`PanelSwitch`, `role="switch"`, name `Grid`): shown / hidden. Touch and unmodified-keyboard alternative to Shift+click. |
| 3 | `Mode`: `role="radiogroup"`, two equal buttons `Adaptive`, `Fixed` (28px, the target-toggle look, `aria-checked`). |
| 4 | `Scale`: radiogroup `Linear`, `Logarithmic`. |
| 5 | `Spacing`: number field, display unit suffix (`mm`), `EntryField` pattern. |
| 6 | `Subdivision`: integer field, no suffix. |
| 7 | `Angle`: number field, suffix `°`. |
| 8 | `Origin X`, `Origin Y`: two fields in one row (label inside the left edge like the Select bar fields, 118px each), display unit. Measured from the page's top-left corner. |
| 9 | Button `Reset to page corner`, full width, 28px. |

Tab order is visual order. Labels 14px `--toolbar-icon`, 8px row gap (the Snapping rhythm, 16px between
groups). No explanatory paragraph, banner or preview thumbnail.

**Fields** follow 0060 §1.4: commit once on Enter, Tab or edited blur; unchanged text writes nothing;
Enter commits and returns focus to the canvas, Tab commits and advances, Escape restores the stored
value. Pointer use of radios, switch and Reset returns focus to the canvas; keyboard use keeps it.
Rejected text writes nothing, marks the field invalid (`--field-invalid`, `aria-invalid`) and shows the
validation chip. After every commit the stored, normalized value is shown. Undo and redo refresh all
controls, the rail badge and the dirty indicator in the same update.

- Spacing: any finite number above 0, no upper bound; display unit per `0015`.
- Subdivision: whole number 2 or more.
- Angle: any finite number; stored modulo 360 and shown in -180 to 180 (**Proposal**, as the polygon
  angle in `0072`), clockwise positive, one decimal at most: `270` is shown as `-90`.
- Origin: any finite number, negative allowed (the grid may start outside the page).
- **Reset to page corner** sets X and Y to 0 as one step; it keeps focus when activated by keyboard.
  When the origin is already 0, 0 it uses `aria-disabled="true"` (dimmed, no action, stays focusable)
  rather than `disabled`, so a keyboard press never drops focus.
- Mode and Scale are never greyed out. Subdivision stays editable in Fixed mode (it still defines the
  major lines) and in Logarithmic scale (it sets the decade length, criterion 8).

**Notices** (validation chip, at most 70 characters):

| Field | Text | Chars |
|---|---|---|
| Spacing, empty or not a number | `Enter a number greater than 0` | 29 |
| Spacing, 0 or less | `Spacing must be greater than 0` | 30 |
| Subdivision | `Enter a whole number, 2 or more` | 31 |
| Angle, origin | `Enter a number` | 14 |

**History** (one step per change; opening the pane, switching page, reading defaults write nothing):

| Action | Step label |
|---|---|
| Switch or Shift toggle | `Show grid` / `Hide grid` |
| Mode | `Set grid mode` |
| Scale | `Set grid scale` |
| Spacing | `Set grid spacing` |
| Subdivision | `Set grid subdivision` |
| Angle | `Set grid angle` |
| Origin X or Y | `Set grid origin` |
| Reset to page corner | `Reset grid origin` |

Each step names its page in the History detail line when the document has more than one page
(`Page 2`). Undo restores the active page if the step changed another page (**Proposal**, otherwise the
change would be invisible).

## 3. Snapping panel additions

- **Targets** (0060 §1.4 row 2) become five toggles in the same two-column layout, in the order
  Objects, Guides, Nodes, Page, **Grid**; Grid sits alone in the third row, left. Same pressed-state
  look, same history step (`set_document_snapping`), editable with snapping off.
- New group directly below, legend `Grid snaps with`, `role="radiogroup"` of three full-width 28px
  options stacked, one per row: `Centre`, `Box corners and edges`, `Nodes`. Default `Centre`.
- **(Proposal)** the group is **always visible and always enabled**, also with the Grid target off or no
  grid shown. Reason: 0060 keeps every control editable when its owner is off; a group that appears and
  disappears moves the Capture and Release fields under the maker's pointer. The Grid target button
  stays pressed even where no grid is shown; criterion 10 makes it pull only where a grid is shown, and
  the canvas (grid visible or not) shows that. No extra note.
- Change of choice is one step (`set_document_snapping`); Alt inversion, Shift, axes and zones are
  those of 0060. The X and Y axis buttons keep their names; for a rotated grid they act on the grid's own
  axes (criterion 13) with no relabelling.
- Snap point does not apply to resize, Node tool, Pen or shape creation (criterion 11); the group does
  not change for these tools and shows no "n/a" state.

## 4. The grid on the canvas

**Layer order**: pasteboard, page background, **grid (clipped to that page's rectangle)**, artwork,
editor overlays, snap marker. The clip is exact to the page's device-pixel rectangle; no line spills
onto the pasteboard, no border is added. Not hit-testable, not picked by the eyedropper, absent from
every export and print.

**Colour (Proposal; `docs/design-system.md` not edited yet).** The page background is a stored document
property (`0040`, any colour, or None with the checkerboard), so a fixed grid colour cannot hold. The
grid takes an **ink chosen from the page background**: dark ink on a light ground, white on a dark one
(whichever of the two has the higher contrast with the stored background; relative luminance
threshold about 0.18). Both grid levels are one ink at two opacities. New tokens:

| Token | Value | Used for |
|---|---|---|
| `--grid-ink` | `--toolbar-icon` (`#3A3A3F`) | Ink on a light page ground |
| `--grid-ink-on-dark` | `#FFFFFF` | Ink on a dark page ground |
| `--grid-minor` | ink at 20 % | Minor lines |
| `--grid-major` | ink at 42 % | Major lines |

On the default `--canvas-bg` (`#E8E8EB`) that is about `#C9C9CD` (1.35:1) minor and `#A5A5A8` (2.0:1)
major. Like `--axis-guide` they are informational and exempt from 3:1: a grid must recede behind
artwork, and its position is also readable from the snap marker and the rulers. Worst case is a mid-grey
ground (`#767676`, about 2.2:1 major, 1.5:1 minor with the better ink). The tester verifies five grounds
(default, white, black, `#767676`, None with checkerboard) in a Rust unit test and a screenshot. On None
the grid is drawn over the checkerboard with the light-ground ink; the major opacity is the readable
level there. A dark UI theme swaps nothing: the choice follows the page, not the theme. The Rust
constants live in `curvyo-render-core::theme` next to `REFERENCE_MARK`.

**Lines**: 1 device px for minor and major alike (strength comes from opacity, not width, so lines never
blur at fractional widths); axis-aligned lines are snapped to device-pixel centres, rotated lines are
anti-aliased. Constant screen width at every zoom. Solid, never dashed (dashed is guides).

**Fade between levels** (criterion 5): per frame, the finer level has opacity `t x minor`, `t` rising
0 to 1 as its screen spacing goes from 4 to 12 px. Lines of the current minor level rise from minor to
major opacity with the same `t`; lines already major stay major. No line's opacity ever jumps between
frames, so wheel and pinch zoom never flicker. Fixed mode (criterion 6) fades each level out the same way
between 4 and 12 px instead of cutting at 4 px (**Proposal**, no popping). Logarithmic grid: decade
lines `--grid-major`, others `--grid-minor`; a line closer than 4 px to its neighbour is omitted and
fades in on the same 4 to 12 px ramp.

**Contrast with artwork**: the grid sits under all objects, so it never covers a stroke; objects with
transparency show it through, which is intended. Selection blue, guides (green) and the snap marker (red)
keep their colours and are drawn above it.

**Snap marker** is that of 0060 §2 unchanged: `--reference-mark` line (1px, white casing) plus 12px
cross. For a grid **line** the marker line is the matched grid line over 24 screen px each side of the
cross, clipped to the page (**Proposal**; a full page-length red line over a visible grid would be
heavy and unreadable). For an **intersection** two such segments and the cross. Lifecycle, no-label rule,
hover feedback in drawing tools and Alt inversion are 0060 §2–3 and §10–11.

## 5. Behaviour the canvas must feel

- Shift+click on the rail with several pages toggles only the active page; the other pages' grids do not
  change visibly. A page that is not active keeps drawing its own grid.
- Zoom is smooth: no frame in which the number of drawn lines or their opacity jumps.
- Page add copies the grid of the size source page (criterion 1) with no extra feedback.
- A moving point on the pasteboard or over a page with a hidden grid gets no grid pull and no marker.
- With 10,000 objects the grid costs the same as in an empty document (criterion 9).
- No status-bar, toast or live-region message for a grid snap, as for every snap.

## 6. Texts

| Where | Text |
|---|---|
| Rail tab | `Grid`; tooltip `Grid — on` / `Grid — off` |
| Status after Shift toggle | `Grid shown` / `Grid hidden` |
| Panel rows | `Page`, `Grid`, `Mode`, `Scale`, `Spacing`, `Subdivision`, `Angle`, `Origin X`, `Origin Y` |
| Options | `Adaptive`, `Fixed`, `Linear`, `Logarithmic` |
| Button | `Reset to page corner` |
| Snapping panel | target `Grid`; legend `Grid snaps with`; options `Centre`, `Box corners and edges`, `Nodes` |
| Notices | as the table in §2 |

No German strings in the UI. "Centre" follows the British spelling already used for the 0060 reference
points; if the UI uses "Center" elsewhere, the lead aligns this to the app's one spelling before build.

## 7. Acceptance map

| Criterion | Where fixed |
|---|---|
| 1 | §2 defaults and no-write opening, §5 page add |
| 2 | §1 |
| 3 | §2 layout, fields, notices, history |
| 4 | §4 layer order, clip, colour, lines |
| 5 | §4 fade |
| 6 | §4 fade (fixed mode) |
| 7 | §3 axes note; angle field in §2 |
| 8 | §2 (subdivision in log), §4 (log levels) |
| 9 | §5 |
| 10 | §3 targets |
| 11 | §3 snap-point group |
| 12 | §4 marker, §5 |
| 13 | §3 axes note, §4 marker |
| 14 | §4 marker |
| 15 | §2 history, §5 |

## 8. UI review checklist

Rail: all four shown/selected combinations expanded and collapsed; plain click vs Shift+click vs
Shift+Enter/Space, no double toggle, repeat suppressed, no transient pane change; pane follows the active
page (values, badge, header row); sticky across tools. Panel: every control with the grid hidden; Escape,
Enter, Tab, blur commit rules; the four notices and no write on refusal; angle normalization; negative
origin; Reset keeps focus and is a no-op at 0, 0; one history step per change with the right label;
undo from rail focus and native undo in fields; fieldset inert during a canvas drag. Snapping: five
targets, snap-point radiogroup keyboard (arrows, one tab stop), group visible with Grid off, history.
Canvas: colour on five grounds, 1 device px lines, fade at 4 to 12 px without jumps through wheel zoom,
fixed and log modes, rotation, clip at the page edge and on a second page with other settings, absent
from export, marker on line and intersection, no marker on the pasteboard. Panel fit at the minimum window
size, coarse-pointer access without Shift.

## 9. Open points for the lead

1. Log-grid fade and merge rule and the marker length (§4) are Proposals; the spec does not fix them.
2. Page names: does `0059` give pages a user name? §2 row 1 assumes name or position.
3. Snap point `Nodes` for objects without nodes (text, image, group of such): the spec is silent.
4. Snapping tooltip shortcut line vs the customer rule (§1).

## 10. UX review (2026-11-01)

Verdict: **CHANGES REQUESTED** (one finding). Reviewed in Chromium against the dev server; screenshots
in the session scratchpad `ux0090/`.

Checked and fine: Grid rail entry after Snapping (icon, tooltip `Grid — on` / `Grid — off`, click opens
the pane, Shift+click toggles without opening, Shift+Enter and Shift+Space toggle once, shown state is the
subtle ground plus check badge, `aria-description` Shown / Hidden); pane rows, order and copy; all four
notices with the exact texts and `aria-invalid`; angle 270 shown as -90, -720.55 stored modulo 360;
negative origin; Reset to page corner disabled at 0, 0 and resets otherwise; page name follows the active
page; default grid at 100 %, 500 %, 20 % and 2500 % with the expected cell sizes, no jumps between steps;
fixed mode, rotated 30 degrees, logarithmic scale; clip to the page; white ink on a dark page ground;
two pages with different grids; Snapping pane with five targets, `Grid snaps with` group visible, Centre
default; a moved rectangle shows the snap marker lines around its grid-aligned centre.

Findings:

1. Horizontal grid lines render as dashes (alternating gaps) at zoom levels where line positions fall
   between device pixels (seen at 23 % and 40 %; vertical lines stay solid; 100 % is solid). Lines must be
   solid and device-pixel snapped (section 4, "Lines"). Fix in the render code (Rust, not changed in this
   review). Screenshots: `ux0090/45-zoom-23.png`, `ux0090/45-zoom-40.png`, `ux0090/40-two-pages.png`.

Not blocking, for the implementer's call:

- Origin X / Y labels sit outside the fields; section 2 asks for the label inside the left edge. The
  row fits at 280px, so it is left as built.
- The Angle `°` suffix reads very small next to the number.
- The Snapping tab tooltip still carries the shift-click hint line (open point 4); not part of this story.

## Amendment: Grid & Rulers and the origin corner (2026-11-01)

Spec criteria 16 to 19; overrides §1 and §2 where they differ.

- **Name**: rail entry and panel header `Grid & Rulers` **(Proposal)**. Tooltip `Grid & Rulers — on`
  / `Grid & Rulers — off`; accessible name `Grid and rulers` (screen readers read "&" unevenly).
  Shift+click still only shows or hides the grid; the status line stays `Grid shown` / `Grid hidden`.
- **Origin corner**: directly under the `Page` row, label `Origin` left, on the right a small page
  pictogram (56 x 72 px, outline in the field-border colour) with one round radio on each corner
  (14 px hit area, 8 px dot; the chosen dot filled in `--editor-accent`). `role="radiogroup"`,
  name `Origin corner`; radios named `Top left`, `Top right`, `Bottom left`, `Bottom right`. One
  Tab stop on the chosen corner, arrow keys move and choose in reading order, pointer use returns
  focus to the canvas. Works with the grid hidden. One history step per change ("Change grid").
- **Removed**: rows 8 and 9 of §2 (`Origin X`, `Origin Y`, `Reset to page corner`) and their
  notice row; the `Angle, origin` notice applies to the angle only.
- **Rulers** count from the active page's corner, positive into the page (customer, ORIGIN-AXES A):
  from a bottom corner the vertical ruler counts upward, from a right corner the horizontal ruler
  counts to the left. Labels stay to the right of (below) their tick.
- **Typed move**: Absolute X and Y of the move chip show and take positions measured from the same
  corner (customer, ORIGIN-FIELDS B); Relative keeps right and down positive.

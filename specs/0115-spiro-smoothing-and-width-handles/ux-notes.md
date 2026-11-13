# UX notes

## Spiro

No new controls. The point is under the pointer at every event and the curve is the one the points give, the same one stored and reopened (amended 2026-11-12: the hold that let the point wait and rejoin is gone, the customer found it slow). Behind a neighbour the curve turns over to the other side at once. No notice.

## Edit width

- Tool bar (Select and Node), last control: a toggle "Edit width" with the Taper both glyph, shown only when one stroked path is selected. Pressed (accent) and reading "Done" while editing; pressing it again leaves the mode (it replaces the separate "Width · Done" chip). At least 96 px wide, so the bar does not move when it turns into Done. In other tools it is a chip of its own while editing.
- Style panel keeps the Width profile row and the point width field (shown while editing with a selected point).
- 800 × 600: the toggle wraps with the bar's other groups; it stays inside the window.

## Diamonds

- 16 px corner to corner (was 11), white fill, 1.5 px `--node-stroke` ring, 1.5 px white casing, the selected one filled with the accent. Ghost diamond on hover at half opacity. Hit radius 11 px.

## Typed width

- Double-click a diamond: a small field at the diamond (the `0052` entry chip look), value in mm with up to 3 decimals, selected. Enter commits, Escape cancels, a click outside cancels.

## Wheel

- Over a diamond the wheel changes the width (0.5 mm per notch, Shift 0.1 mm), shows the live mm readout and does not zoom or scroll, also with Ctrl or Meta held and for a trackpad pinch (lead decision 2026-11-12, Proposal). Elsewhere the wheel zooms as before.

Check at 800 × 600 and 1440 × 1000.

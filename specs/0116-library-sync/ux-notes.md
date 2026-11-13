# 0116 Library sync: UX notes

ux-engineer, 2026-11-13. Written from the customer request of 2026-11-13 (a per-item symbol
toggle switch on every font row, machine row and material row that turns sync of that one
item on/off across the maker's paired devices of `0096`), the built device sync
(`specs/0096-own-devices-sync/ux-notes.md`: My devices block, `PanelSwitch` "Sync to my
devices", arrival notices, the armed Share-tab switch), `docs/design-system.md`
("Properties panel: My devices and the sync switch", "Share tab since `0105`",
"Interaction conventions": Switch, armed trash, Hidden-not-disabled, row-control slots) and
the built rows: `FontTreeRows.tsx` (family row: trash, star, face count), `MachineProfileList.tsx`
(Duplicate, Rename, work area, Delete), `MaterialRow.tsx` (name, trash, arrows, line 2).
Customer rule 2026-10-16: no operating tips or hint labels; labels that name a control or a
state stay. Customer rule 2026-10-31: no success notices. Texts with state or numbers are
formatted in Rust (`curvyo-ui-core`). **(Proposal)** marks reversible choices.

The three libraries are **per-user, not per-workspace**: their toggles work whether or not a
workspace is open, and the per-workspace switch "Sync to my devices" of the Share tab
governs workspaces only. Nothing of `0096` moves.

## 0. What is reused

| Part | Reused from | Change in this slice |
|---|---|---|
| Symbol toggle in a dense row | Star of the font family row, Show/Output of an operation row (`aria-pressed` button, not a Tab stop) | New glyph, new meaning; no new pattern |
| Pressed look | Icon toggles app-wide (`aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)]`) | none |
| Row keys | Machine list (`Ctrl+D`, F2, Delete on the focused row), materials list (F2, Delete, arrows) | One more key, `S` (section 2) |
| Greyed with a reason | Share tab (`aria-disabled`, 40 %, reason as tooltip and `aria-description`) plus one visible state line | Applied to a row control |
| State glyphs | Collaboration pill (`Radio` / `CloudOff` / `RefreshCw` by connection state) | Same swap inside the toggle |
| Arrival notice | `0096` canvas notice ("Workspace Box job arrived.", status, 5 s, at most 70 characters, name cut to 24) | Same slot, item names |
| Switch wording | "Sync to my devices" (the Share tab switch) | Same words |

No new token. New rows for `docs/design-system.md` when this ships: "library sync toggle"
and the three placement notes in the Fonts/Machine/Materials rows.

## 1. The toggle

- **Control.** One symbol toggle per row: a `button` with `aria-pressed`, 16 px Lucide
  glyph at 1.5 px stroke, in the row's own control slot — `h-7 w-6` (24 x 28) in the font
  family row and the machine row (as their trash and buttons), `h-6 w-6` in the material
  row (as `CONTROL` there). Hit target = the slot, at or above the 24 px minimum, same as
  the row's other controls. Not a text button, not a `PanelSwitch` track (a track with a
  thumb does not fit a 26-28 px row line and is the pattern for named panel settings).
- **Glyph.** Lucide `CloudSync`: the cloud carries "to my devices" (the Share tab's own
  noun), the circling arrows "sync". Not `RefreshCw` (Try again, busy only) and not
  `CloudOff` (offline); the states below swap the glyph for a reason, the control itself is
  the cloud.
- **Looks.** **Off (not syncing):** `CloudSync` outline in `--panel-muted-fg` (the
  unpressed star's look, 3.1:1), hover `--editor-accent-hover`, focus-visible 2 px
  `--editor-accent` ring as the row controls. **On, in sync:** pressed look
  `--toolbar-icon-active-bg` ground with `--toolbar-icon-active-fg` glyph (the accent of
  every pressed toggle; on 4.7:1, off 8:1 — state by ground and glyph, never colour alone).
  **On, syncing** (this item's upload or arrival is running): same ground, `RefreshCw`
  glyph, `aria-busy`. **On, offline or refused:** same ground, `CloudOff` glyph — the row
  says what is true of *this* item; the app-wide offline state stays where it is (Server
  field line, collaboration pill). `aria-pressed` stays true through both; the accessible
  name is constant.
- **Name and tooltip.** Accessible name "Sync `<name>` to my devices" (names the control
  and its subject, as "Favourite: `<family>`"). Tooltip (Radix, 400 ms, `side="left"`,
  as the row's other controls): line 1 "Sync to my devices (S)"; line 2, muted, by state:
  "In sync with 2 devices." / "Syncing..." / "Offline." / (off: no second line). A state
  line in a tooltip is the Link-corners toggle's established form.
- **Placement.** Row end, immediately before the trash slot, in all three rows; the star
  keeps its slot right of the trash and is never replaced or moved:
  - **Font family row** (Curvyo block only): chevron, name, current mark, **sync**,
    trash, star, face count. Face rows, the Missing row, system families ("This computer")
    and not-installed Document Fonts rows get no toggle — only installed Curvyo fonts are
    the maker's per-user library (where the trash is, the toggle is; where it is not, it
    is not).
  - **Machine row**: name/area, Duplicate, Rename, work area, **sync**, Delete. Builtin
    profiles (Generic laser) carry no controls today and get none: they ship with the app,
    there is nothing of the maker's to sync (Proposal).
  - **Material row, line 1**: name, **sync**, trash, arrows. Damaged and "Needs a newer
    Curvyo" rows have no trash and no toggle; line 2 already states why they cannot be
    used, and they cannot be synced either.
- **Semantics of off.** Off stops this one item syncing; the copies already on the other
  devices stay there as local items, exactly as a switched-off workspace stays as local
  files (`0096` criterion 13). Turning it back on resumes (see section 5 for a device that
  still holds its copy).
- **New items default on** (Proposal): a font installed, a profile created or a material
  saved while paired syncs unless the maker switches it off — "syncs to my devices" is the
  normal state of a per-user library; a whole *workspace* stays opt-in (`0096`
  SYNC-SCOPE B) because it is heavier and may hold things the maker does not want on the
  server.

## 2. Keyboard, focus, hit order

- The toggle is **not a Tab stop**: each list keeps its one roving stop (the tree, the
  listbox, the list), and row actions are the row's own keys — the machine list already
  works this way for Duplicate (`Ctrl+D`), Rename (F2) and Delete.
- **`S` on the focused row toggles that row's sync** in all three lists (Proposal: one key,
  the same in each; `S` is free — tool letters are canvas-gated, and each list already
  swallows its keys, F2/Delete/arrows/Enter; during an in-place rename `S` types). The
  tooltip names it (section 1). The fonts tree has no row-action key yet; `S` is its
  first, and the star stays as built.
- A mouse press keeps the focus on the row (`onMouseDown` preventDefault, as the trash and
  star); a key press stays on the row. DOM order = visual order, so the toggle between
  name and trash reads in place.
- Space and Enter on the *row* keep their meanings (pick, open, select); the switch is
  reached by `S`, never by overloading them.

## 3. No paired device yet

- The toggles stay in the rows (the maker sees the control exists) and are greyed: 40 %
  opacity, `aria-disabled`, a press does nothing; the stored on/off state is still shown
  (greyed), so the maker sees what will sync once paired. Reason as tooltip and
  `aria-description`: "Pair a device to sync." (the Share tab's disabled pattern).
- Not tooltip-only: each of the three lists shows **one visible state line** while this
  device is in no device group — 12 px `--panel-muted-fg`, `role="status"`: "Pair a device
  to sync fonts." under the Curvyo block header; "Pair a device to sync machine profiles."
  above the profile list inside the Machine block; "Pair a device to sync materials."
  under the Materials section header. It is a state line (shown exactly while it applies),
  not a tip: it leaves in the frame the first device joins (the `0096` pairing), as the
  one-device warning leaves with the second device.

## 4. Switching off needs no armed second press

No arm on the per-row switch. One line why: turning one item off removes nothing and
rotates no keys — it is undone by pressing the switch back on, so there is nothing for a
second press to guard; the workspace switch keeps its armed off because it removes every
paired device from the room and renews keys (`0096`). The Share tab's armed pattern, look
and 400 ms guard are untouched.

## 5. Arrivals, sync states, failures

- **Arriving.** Items arrive without input, like workspaces. Canvas notice slot
  (`role="status"`, 5 s, at most 70 characters, names cut to 24): one item
  "Font Source Serif 4 arrived." / "Machine profile `<name>` arrived." /
  "Material Slate tile arrived."; a batch counts per kind, kinds with none left out:
  "3 fonts, 2 machine profiles and 1 material arrived." The rows appear in their lists
  with the toggle already pressed. A family or profile whose name already exists on this
  device   follows the spec's merge rule; the notice names the final row. A device that
  still holds its old copy of a switched-off-then-on item merges its local
  edits as an offline edit would (the spec's criterion 10) — no decision row.
- **On the row.** Syncing / in sync / offline are carried by the toggle's glyph swap
  (section 1): `RefreshCw` while this item transfers, plain pressed `CloudSync` when it is
  done, `CloudOff` while the server cannot be reached or the item was refused. The
  tooltip's second line states the same in words.
- **Failures.** No success notice (customer rule 2026-10-31) — the pressed look is the
  feedback. A toggle press that the group could not carry out does not flip the switch and
  shows one refusal line under the row (Action notice, `--field-invalid`, 8 s, the `0081`
  notice slot under the font row; the material and machine rows use the same anchored
  line): "Could not sync `<name>`: cannot reach the server." Retries are the group's
  (at once when the network returns, else every 30 s), no Try-again button on a row.

## 6. The Share tab

- **No "My libraries" block** (Proposal). The Share tab shows labels, states and real
  errors only (`0105`); a counts block for the three libraries would be a summary with no
  control on it,   stating a state every library row already states where the maker edits
  it — chrome, not a control, in the calmest tab of the app. Frage 217 offers the block to
  the customer; the recommendation is not to build it.
- The **per-workspace switch "Sync to my devices" stays as is**, its place, its armed off
  and its lines unchanged; it governs the open workspace, never a library. The My devices
  block, pairing, Join and the device rows are untouched.

## 7. Layout at 800 x 600 and 1440 x 1000

- The toggle takes **one existing-slot width (24 px) inside the trailing control cluster**
  of each row; the row's height, the panel's width and the list's scroll width do not
  change. The row never widens: the name line (`min-w-0 flex-1 truncate`) gives up at most
  24 px and truncates earlier — the fonts list is the long one, and a family name that
  already truncates today keeps its full name in its tooltip, as now.
- Checked at 800 x 600 and 1440 x 1000 (measured at build): Fonts tab, Curvyo block open
  with 40 families, panel shown and collapsed; Machine block open with the profile list at
  five rows; Materials with 30 records — every row one line high as today, the toggle
  column aligned per row type, nothing clipped, no horizontal scroll, the three greyed
  state lines inside their headers.

## 8. Proposals (F1 to F4, vetoable at the demo)

- F1: `S` as the row key of the sync toggle in all three lists (section 2).
- F2: builtin machine profiles carry no toggle (section 1).
- F3: new items default on (section 1).
- F4: no Share-tab block (section 6; Frage 217).

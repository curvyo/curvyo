# 0151 Generator UI polish: UX preview

ux-engineer, 2026-11-27. A preview, not UX notes: nothing here is built or Ready. **Everything in this
file is (Proposal)**, to be decided by the customer. Written after the customer's try of 2026-11-27:

> Nothing should be on automatically in the generators. Right now clicking Generators puts the box
> straight into my hand; I want to pick one first, and clicking it again should switch it off. Today
> you first place a basic generator and then change it; I find that not intuitive yet. The adjustments
> are in the Style tab, which I actually don't find bad.

Read against the shipped UI (`frontend/src/components/GeneratorsBar.tsx`,
`frontend/src/components/panel/GeneratorBlock.tsx`, `frontend/src/hooks/useGenerators.ts`,
`curvyo-editor-wasm/src/session/generators.rs` and `generators/source.rs`), the UX notes of `0141`
(sections 1, 2, 15), `0147` (1, 2), `0149` (1, 2), `0144` (1, 2, 11.2 to 11.4), `0155`, and
`docs/design-system.md` (rows "Escape cascade", "Pick mode", "Place a shape by click", "Hidden, not
disabled", "Rail tooltip and notice placement", "No popups in the properties panel", "No operating
tips", "Drag ghost", "Value field", the tab rows "Active tab" and "States").

Mockup of the recommended concept (section 3):
`/tmp/claude-1000/-home-marc-curvyo/10a500d5-50f8-4db7-97a4-b180a9add59e/scratchpad/generator-preview.html`
(static, light theme, no scripts; the lead moves it to `docs/design-drafts/` if it is kept).

## 0. What is shipped, and where it rubs

1. Rail button **Generators** (More tools card) makes the tool active. The Generators bar shows one radio
   group: Box, Box from path, Code, Gear, Rounded box. **One segment is always pressed**: Box at first,
   later the last choice of the session (`generator_kind()` falls back to `FIRST_KIND`).
2. The ghost of the pressed kind **at its factory values** follows the pointer at once. A click places
   it, selects it, returns to Select and shows the Style tab.
3. Only now the Generator block appears (first block of the Style tab), with the knobs on the canvas.

Where it rubs:

- **A.** A box is in the hand before anything was chosen (the customer's first point). A radio group
  cannot be empty, and its arrows choose as they move, so the bar cannot express "none".
- **B.** The maker decides the place before he has seen one number. The object lands at its factory
  size (a 100 mm box, 24 teeth) whatever he wants, then grows or shrinks around a centre he picked for
  another size.
- **C.** For a code the order is worst: place a QR code with "https://example.com", then switch the
  symbology, then type the content.
- **D.** Every new object starts from factory values, also the fifth box in 3 mm plywood.
- **E.** Three places for one job: the bar picks, the canvas places, the panel sets. The customer likes
  the panel part, so that one stays.

## 1. The minimal change (buildable now)

What the customer asked for, nothing more. It fixes A only.

### 1.1 Behaviour

1. **Armed or not.** The tool has one armed generator or none. **Entering the tool arms none**, from
   every tool and every time; no last choice is kept. After a placement the tool returns to Select as
   today, so the next entry again starts with none.
2. **The bar** keeps its pill, segments, glyphs, widths, order and pressed look. It stops being a radio
   group: `role="group"` named "Generator", each segment a toggle button with `aria-pressed`; at most one
   is pressed.
   - A press on an unpressed segment arms it and releases any other.
   - **A press on the pressed segment releases it** (none armed).
   - One Tab stop (the pressed segment, else the first). Left, Right, Home and End move the focus and
     **do not arm**; Space or Enter on the focused segment toggles it. After a mouse press the focus
     returns to the canvas, as built.
3. **Nothing armed:** no ghost, no hover marks, the arrow cursor (not the crosshair). A press, a drag
   and Enter on the canvas do nothing and write nothing; no notice. Pan and zoom as in every tool. The
   panel does not change.
4. **Armed:** exactly as built. Ghost at the pointer (Box from path: the pick hover instead), crosshair,
   a click, a press-drag-release or Enter places; then Select and the Style tab.
5. **Escape**, one step per press, by the "Escape cascade" row ("clear the tool's state, else switch to
   Select"; the Trace precedent "a pressed region tool is released, then the tool is left"):
   1. an armed generator is released, the tool and its bar stay (with the button down too: the
      release then places nothing);
   2. else (none armed) the tool is left for Select.
   Today's single Escape to Select becomes two when something is armed. The alternative "Escape always
   leaves at once" is one press shorter and makes the unarmed state reachable by the mouse only; not
   recommended, because the customer asked for "off" as a state of its own.
6. **Replace outline** (`0149` section 4) still arms Box from path itself: it is a command of the panel,
   not an automatic choice. Escape, or a press on the pressed segment, ends it and returns to **Select
   with the box still selected** (it goes back where it came from, as Escape does today).
7. **Code** stays one segment. Which symbology it arms is still the last one placed or switched to in
   the session (QR code first): that is what "Code" means, not an armed state. Its glyph follows as built.
8. **A kind switched off while armed** (its extension disabled): none armed, not the first of the bar.
9. No hint label, no notice, no new token, no new component (the Trace Mask group already uses
   exclusive `aria-pressed` toggles in a bar).

**Cost for the maker:** three presses per object (rail, segment, canvas) where the last-used kind took
two. Accepted, it is what was asked.

### 1.2 Build pointers

- `curvyo-editor-wasm/src/session/generators.rs`: an empty `GeneratorsState.kind` means none;
  `generator_kind()` loses the `FIRST_KIND` and first-enabled fallbacks; `set_generator_kind(kind)`
  with the armed kind releases it; `generators_draw_list`, `place_generated_at`, `place_generated_key`
  and `pick_cursor` answer "nothing" for none.
- `session/mod.rs` `set_tool`: entering `Tool::Generators` from another tool clears `kind`;
  `begin_replace_outline` (`generators/source.rs`) sets its kind after the switch, not before.
- `session/keys.rs` (the Escape cascade): an armed generator is tool state, so the press answers
  `EscapeStep::ClearedState` and the next one `LeftTool`; with `replace` set, `LeftTool` at once.
- `frontend/src/components/GeneratorsBar.tsx`: the `ToggleGroup` (Radix radio group, cannot be emptied)
  is replaced by toggle buttons in the same look; `frontend/src/components/Canvas.tsx`: the crosshair
  only while a pointer-placed kind is armed.
- Size: small. One session change, one bar component, no format, schema or crate change.

### 1.3 Checks for the tester

Entering the tool: no segment pressed, empty ghost list, a canvas click adds no undo step, Enter ignored.
Press Gear: pressed, ghost. Press Gear again: released, no ghost. Press Box then Gear: only Gear pressed.
Arrow keys across the bar arm nothing. Escape armed: released, tool still Generators; Escape again:
Select. Place, re-enter: none pressed. Replace outline: Box from path pressed; Escape: Select, the box
selected. A disabled kind while armed: none.

### 1.4 Lines this supersedes when accepted

`0141` section 1 ("`role="radiogroup"`", "arrows move and select", "the last choice is kept for the
session (first use: Box)") and section 2 ("Escape leaves the tool"); `0147` section 1 last item; `0149`
section 1.2 (the radio group wording, "Escape leaves the tool"); the design-system rows "QR code block"
and "Barcode block" where they say "radio group". `0144` section 11.2 ("what it places") stays.

## 2. Concepts for a better flow

The minimal change leaves B to E. Three concepts, each on top of section 1.

### Concept A: set up, then place (recommended)

The Generator block the customer likes shows **when a generator is armed**, for the object that does
not exist yet. The ghost is drawn from the block's values. Placing pins it.

1. Generators tool, nothing armed (section 1). Mockup frame 1.
2. Press **Gear**. The Style tab shows (the stated exception of `0141` section 2 moves from "on placing"
   to "on arming"), subject line **"New gear"**, and under it the Gear block exactly as for a placed
   gear: same groups, Value fields, switches, strips, reset slots, clamps. No Stroke or Fill sections
   yet (there is no object). A selection on the canvas stays as it is; the tab shows the new object
   while one is armed.
3. Move over the canvas: the ghost is the gear **with the block's values**, true size. Change Teeth in
   the panel: the ghost **rests where the pointer left the canvas** (the middle of the visible canvas
   if it never was there) and is redrawn at every tick of the drag, in the ghost's reduced drawing
   (`0147` section 2, `0144` section 2). Mockup frame 2.
4. A click on the canvas, or Enter with the focus on the canvas, places it: one step "Add gear",
   selected, Select tool. **The block does not move**: the subject line turns to "Gear", Stroke and Fill
   join below, the knobs appear. Mockup frame 3. A maker in a hurry clicks right after step 2 and has
   today's flow.
5. A second press on Gear, or Escape, releases it: ghost and block go, the tab returns to what it
   showed before. The Escape cascade gets one step in front: a field being typed is restored first.

Per kind: **Code** shows Symbology and Content before placing, so the maker picks EAN-13, types the
number and places the right code (fixes C; the bar glyph follows the strip). **Box from path** shows
its block too; while it is armed and one eligible outline is selected, the corner marks of `0149`
section 3 are drawn on it with the block's Corner angle, so Enter's result is visible before Enter.

Which values a new object starts with is a question (section 5, GEN-VALUES): the factory values as
today, or the values last used for that kind in this session (fixes D; the reset slots show at a glance
which fields differ from the factory value).

- **For:** fixes B, C and optionally D with no new surface: the same block, the same fields, the same
  tab the customer accepts. No popup, no flyout, no hint label. One mental model before and after
  placing; nothing jumps at the click. The fast path costs nothing extra.
- **Against:** the Style tab now also shows something that is not selected ("New gear" must carry that;
  it names a state, so it is allowed). The pointer travels bar, panel, canvas when the maker sets up
  first. The ghost of a big box is regenerated on every tick (the Draft of ADR 0083 section 13.13 and
  the reduced ghost keep it cheap; a CPU-time test per kind).
- **Cost: medium.** Session: a pending record for the armed kind that the existing value, preview and
  panel calls address while nothing is placed; the ghost built from it; its resting point. Frontend:
  the Style tab enabled while armed, the subject line, the tab switch on arming, presses in the panel
  must not end the placement. No format change, no schema change, no new crate, no new component.

### Concept B: draw it to size

Armed, the maker **drags the object open** like a rectangle: press, drag, release. The drag sets the
generator's main size (gear: outer diameter in whole teeth; code: width; box: inner width and depth),
a readout at the pointer names it ("30 teeth · ⌀ 64 mm"); a plain click places at the current values.
Everything else stays in the Style tab.

- **For:** the most direct way to a size; matches Rectangle, Ellipse and Polygon ("Place a shape by
  click"); nothing new in the panel.
- **Against:** a box, a rounded box and a path box are a **flat layout of five or six parts**, not one
  shape: a dragged rectangle that means "inner width and depth" while plates spread far past it is
  harder to read than a number. Thickness, kerf, symbology and content cannot be dragged, so the panel
  step remains. Makers who cut to a material think in typed numbers. Today's press-drag-release
  ("move the ghost, place at the release") changes its meaning.
- **Cost: medium to large.** Each generator declares which parameters a drag writes (an amendment to
  ADR 0080 section 2), a drag preview per kind, new readouts; no use for Box from path.

### Concept C: generators in the library, with presets

Generators leave the rail and the bar and become a **Generators set of the Library** (`0137`, `0152`):
a tile per generator with a drawn preview, and under each the maker's own presets ("Box 3 mm plywood",
"Gear m2 z24") and the last few used. The Library's press model applies: press selects, double-click,
Enter or a drag out puts it on the page.

- **For:** a picture says "Rounded box" better than a 16px glyph and a word; scales to many generators
  and third-party ones (the bar is 520px with five and wraps under 900px); presets end "start from
  the basic one" for good; one place for everything that is brought onto a page; gives back a rail
  slot (column A is over its budget since `0141`, Frage 327).
- **Against:** the largest change and the furthest from what was asked: it removes an accepted tool,
  and "pick one, press again for off" has no meaning in a gallery. A pane for five items is heavy
  today. Presets must be stored somewhere (document, workspace or library): a persistence decision for
  the customer and an ADR. The adjusting still happens after placing unless A is built too.
- **Cost: large.** Library source and tiles for generators, preset storage and its ADR, drag-out
  placing, removal of the tool and bar, previews per generator.

### Looked at and dropped

- **A popover or dialog with the parameters before placing:** against "No popups in the properties
  panel" and "no flyout from the rail", and it would be a second place for the same fields.
- **The main parameters as number fields in the bar:** two places for one value, and the bar is full.
- **A press on a segment inserts at once in the middle of the view (no placing step):** shortest, but
  it is the opposite of "pick one, switch it off again".
- **The kind strip inside the panel instead of the bar** (like Symbology): everything in one column,
  but every tool's choices sit in its bar, and the strip would not survive the placing (a placed gear
  cannot become a box).

## 3. Recommendation

1. **Now: section 1** as its own small story. It is what was asked and it is the first step of A.
2. **Next: concept A.** It answers "place a basic one, then change it" by letting the maker change it
   first, keeps the Style tab he likes, and adds no surface and no rule. With GEN-VALUES at B it also
   ends the factory-value restart.
3. **Later, when there are more generators or third-party ones: C's presets**, as a Library feature in
   front of A's block (a preset is a set of values for the pending block). Not before.
4. **Not B** for the boxes. A drag-to-size for the gear and the codes alone could follow A if the
   customer misses it; A does not block it.

The rest of `0151`'s reserved scope (the look of the Generator block, the canvas marks of the rounded
box and the path box, `0147` section 10.12) is not touched by this preview.

## 4. The mockup

`/tmp/claude-1000/-home-marc-curvyo/10a500d5-50f8-4db7-97a4-b180a9add59e/scratchpad/generator-preview.html`:
three frames of concept A at 1180 x 660px, 80 % zoom, tokens of `docs/design-system.md`.

1. Tool on, nothing armed: bar without a pressed segment, no ghost, Document tab.
2. Gear armed: Style tab "New gear", Teeth being dragged (32, reset slot shown), the ghost at rest on
   the page in the "Drag ghost" look.
3. Placed: the same block under "Gear", Stroke and Fill below, selection box, corner handles, the Bore
   knob, Select bar.

The rail, the Select bar, the Document tab and the Stroke and Fill sections are drawn in outline only.
It was checked for structure and the drawings were rendered; it was **not opened in a browser** (the
browser was reserved for another agent).

## 5. Questions for the customer (for the lead to number)

**GEN-FLOW: which flow after the minimal change?**
A: set up, then place (concept A). B: draw it to size (concept B). C: generators in the library with
presets (concept C). D: nothing further.
Recommendation: A. Default if unanswered: D (only section 1 is built).

**GEN-VALUES (only with concept A): which values does a new object start with?**
A: the factory values, every time (as today). B: the values last used for that kind in this session;
the reset slots lead back to the factory values.
Recommendation: B. Default if unanswered: A.

**GEN-ESC (section 1): Escape with a generator armed.**
A: releases the generator, a second Escape leaves the tool. B: leaves the tool at once.
Recommendation and default: A.

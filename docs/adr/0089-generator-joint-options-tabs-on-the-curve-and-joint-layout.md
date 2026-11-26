# ADR 0089: Generator joint options: tabs on the curve and joint layout

**Status:** Proposed 2026-11-26 by the architect. Slice 1 (decision 1, tighter windows) is
decided: the customer answered Frage 385 (JOINT-WINDOW) with A on 2026-11-26. Decision 2 follows
Frage 381 (JOINT-BEND) at its default A, decision 3 follows Fragen 382 to 384 at their defaults
A; all three are open. Not `needs-customer` by topic (no crate, no dependency, no
`format_version` change); its product choices are those Fragen. Spec:
[`0155`](../../specs/0155-generator-joint-options/specification.md). Amends
[ADR 0082](0082-generator-plate-joints-corner-rule-and-rounded-box.md) §1, §5, §6, §9 and §11, and
[ADR 0083](0083-box-from-a-closed-path.md) §13.4, §13.6 and §13.8, each by reference from here.
Adds one field to `Param` in `curvyo-extension-core` (decision 2, amends
[ADR 0080](0080-generated-objects-extension-trait-and-parametric-instances.md) §2 by note).

**In plain words (for the customer).** Where a tab sits on a curve, the wall today stays flat over
a stretch about twice as long as the tab, so a circle box comes out as a hexagon. First, that flat
stretch shrinks to the tab and its two bridges, and the slits start right beside it (decided).
Second, an option lets the slits run on past the tab: the tab stands on the true curve, the slot
in the wall sits between shortened slits, and the wall bends round all the way. New boxes get it,
boxes you saved before keep the flat window until you choose otherwise. Third, an Advanced group
sets how many tabs or fingers an edge has, how wide they are, and whether they spread evenly or
sit in the middle. At the defaults every box stays exactly as it is today.

## Context

Read against `curvyo-generators-core` at `e0fc2ebe` and the golden
`tests/fixtures/rounded_box_circle.txt`; the measurements are in
[`specs/0155-generator-joint-options/adrs.md`](../../specs/0155-generator-joint-options/adrs.md),
Findings.

- **Who is affected.** Windows on bends exist only where the straights hold fewer than four tabs
  (ADR 0082 §5): the circle, stadiums with short straights, path-box curves (heart, shield,
  ellipse). The default rounded rectangle (120 x 80, r 15) and the curved top's back tabs sit on
  straights and do not change.
- **The flat is longer than the window.** The window chord l = 2t + 2t_w already holds a bridge
  t_w either side of the tab. The band beside it starts one more bridge away (`wall_bands.rs`,
  `lead`/`trail`), and its first column stands half a pitch in (`slit_bend::slots`; ADR 0083
  §13.6 the same). The unslit stretch across a tab is about pitch + 2t + 4t_w: 25.7 mm on the
  60 x 60 circle, six flats, every column bending 30°. That, more than the window, is what the
  customer saw.
- **One default per parameter.** `Param.default` is both the value a new object is written with
  (`Param::defaults`, used only by `ui-core::instance` and `instance_source` to create objects)
  and the value a missing or invalid stored key reads as (`Param::read`). Frage 381 A asks for
  two different values.
- **Unknown keys survive an edit.** Every generator builds its normalised params from a clone of
  the stored map (`raw = request.params.clone()`), so a build that does not know a key keeps it.
- **Leftover.** `boxes/schema.rs` still offers the `joint` Choice (`fingers` | `none`) with its
  `ShownWhen` on Finger width and Clearance, and `boxes/mod.rs` reads it, although ADR 0082 §3
  removes it.

## Options

**The flat across a tab on a bend**

| Option | Why it won or lost |
|---|---|
| **A0. Tighter windows: the band starts at the chord's end, the columns are laid from that end (chosen, Frage 385 A)** | No key, no contract. The flat falls to 2t + 2t_w + w_s (about 12.3 mm, sag 0.6 mm on r 30); the circle keeps 18 to 24 columns instead of 12. Changes five goldens on purpose. |
| **A1. Tabs on the curve, by option (chosen, Frage 381 pending)** | No flat at all; the plate stays the true arc. Softer joint (the tab bears on bridges t_w that twist with the hinge); needs a test cut. |
| Tabs only on straights when any exist (threshold 2, not 4) | Helps stadiums, not the circle or the heart. Folded into decision 3's typed count. |
| Edge notches instead of slots on bends | At the defaults the slots already are edge notches (`base_inset` 0, ring flush); removes no flat. |

**Tab shape on the curve**

| Option | Why it won or lost |
|---|---|
| **Rectangle on the normal (chosen, Proposal)** | Sides parallel to the arc normal at the tab's centre, root on the arc, tip the concentric arc r + t_w (flush outside; a straight tip stands 0.1 mm proud). Never binds in the slot. |
| Sector with radial sides (spec criterion 2 today) | Fits only a slot that bends uniformly, but the slot's island stays flat. Wider at the tip by about 2t t_w / r (0.6 mm at r 30), so it binds. |

**New boxes on, stored boxes off (Frage 381 A)**

| Option | Why it won or lost |
|---|---|
| **A `Param.initial` value for new objects; `default` stays what a missing key reads (chosen)** | One optional field, read in one place (`Param::defaults`). The block shows the stored value, so an old box shows the flat window and keeps it on edits. |
| Default `curve` | An old box shows "curve" and switches on its next edit, against spec criterion 1. |
| Rewrite old records on open | ADR 0033 §3: a read never rewrites. |
| The generator reads an absent key as `window` itself | Two readings of one key: the block (`Param::read`) would show `curve` while the box is cut with windows. |

**Where the joint layout lives**

| Option | Why it won or lost |
|---|---|
| **One pure 1-D layout at crate level, used by tabs and fingers (chosen, Proposal)** | One rule for all three generators; both mating parts read the same list per edge, so they cannot disagree. |
| A pattern per generator (`fingers.rs`, `tab_layout.rs` each) | Three copies of the centred rule and its clamps; the mating tests would have to cover each. |

## Decision

1. **Tighter windows (slice 1; decided, Frage 385 A, 2026-11-26).** No key, no contract change.
   - *Window* (amends ADR 0082 §5 "Flat window" and ADR 0083 §13.8): the chord of length
     2b + 2t_w (b the half width of the widest tab, ADR 0083 §13.8's w; 2t + 2t_w in ADR 0082 §5)
     holds the bridges. The band beside it starts at the chord's end: `wall_bands.rs` drops the extra
     `lead`/`trail` bridge at an end that meets a window. The tab-count reductions (ADR 0082 §5,
     "fewer than two slit columns plus their bridges"; ADR 0083 §13.8, "`min_band` and its
     bridges") count the bridge inside the chord, not a second one.
   - *Columns* (amends ADR 0082 §6 and ADR 0083 §13.6), **only at a band end that meets a
     window**: the outer column stands w_s/2 from that end (w_s the slit width; Wave adds its swing,
     period / 8), not half a step. The keep-clear test against that window is w_s/2 + swing, the
     bridge lying inside the chord. An end that meets a straight, a corner flat, the seam margin
     or a strip end keeps half a step, so a band without a window is byte-identical. Count: the
     rounding of each walk stays (ADR 0082 §6 even pitch: round, at least 2; ADR 0083 §13.6
     integral walk: floor, at least 2), taken over the stretch between the two outer columns.
     With Wall slits All round (one band over the strip) each tab window counts as a
     window-facing end on both its sides, so "on a circle both choices are the same" (ADR 0082
     §5) still holds.
   - *Guard* (ADR 0083 §13.4): changes on purpose for `rounded_box_circle.txt`,
     `path_box_circle.txt`, `path_box_heart.txt`, `path_box_shield.txt`, `path_box_ellipse.txt`,
     re-blessed in the slice-1 commit; every other golden stays byte-identical.
2. **Tabs on the curve, option `bend_tabs` (slice 2; Frage 381 at default A).**
   - *Key* (file format, ADR 0034 §5): Choice `bend_tabs`, ids `window` | `curve`, in the Slits
     group of the rounded box and the path box, shown with the Flat top only (the curved top has
     no joint on a bent span, so spec criterion 6 holds with no row); not in the Box. A Choice, not a
     Switch: the two ids name two layouts, and an unknown id reads `window` with the Dependent
     notice of ADR 0082 §6, as an unknown slit kind does. The UX notes may draw it as a toggle
     labelled "Bend through tabs" (spec criterion 1).
   - *Contract*: `Param` gains `initial: Option<ParamValue>`, the value of a new object when it
     differs from `default`; `Param::defaults` writes `initial.unwrap_or(default)`, `Param::read`
     is unchanged. `bend_tabs`: `default` `window`, `initial` `curve`. Source-breaking 0.x
     (every `Param` literal gains `initial: None`), CHANGELOG; the serialised manifest gains the
     field and the TS reader ignores it (a Choice offers no reset). Frage 381 B: `initial` None.
     Frage 381 C: no key, the window goes.
   - *Plate*: no window and no chord in the footprint; the tab is a rectangle on the normal
     (Options). On the strip, `slit_bend::Window` gains a height span `y0..y1` (the slot's rows);
     a column within w_s/2 + swing + t_w of the slot along the strip is clipped, not dropped: it
     stops t_w from the slot's rows (Offset: the crossing slot shortened or dropped; Straight
     clipped on its run-out side becomes a closed slot). Every ligament round the slot is at
     least t_w, the pattern's bridge.
   - *Mating* (spec criterion 5, the bound): plates on the footprint within the kerf, no sag
     term; tab centre to slot centre within c/2 by neutral-line arc length (`Layout::push` drops
     its chord-for-arc term on `curve`). The rigid island over the slot (width 2b + 2t_w) stands
     tangent at the tab and leaves a gap of at most (b + t_w)² / 2r under its ends, outside the
     plate, never an interference (0.6 mm at t = t_w = 3, r 30).
   - *Limit* (spec criterion 7, Proposal): where a slot cannot keep t_w to two unclipped columns
     on each side, the span takes fewer tabs (Dependent notice on `bend_tabs` naming the radius);
     at one tab, that tab falls back to the window of decision 1 with the same notice.
   - *Code*: crate-internal (`tab_layout.rs`, `tab_runs.rs` place tabs on arcs without splitting
     the footprint; `wall_plates::walk` gains tabs on a `Bend`; `slit_bend.rs` clips by height).
3. **Joint layout (slices 3 and 4; Fragen 382 to 384 at default A).**
   - *Module*: `joint_layout.rs` at crate level, pure 1-D: `layout(length, margins, count,
     width, pattern, gap) -> Vec<Joint { centre, width }>`. Tabs take the list as their places; a
     finger joint takes it as the mate's pieces with the owner's pieces between and at both ends,
     so the corner rule (ADR 0082 §1) holds by construction. Both mating parts read one list.
   - *Keys* (file format; one group function in crate `schema.rs`, used by the Box, the rounded
     box with both tops and the path box): `joint_pattern` Choice `even` (default) | `centred`;
     `joint_count` automatic Count (0 = today's rule; typed = per side, a side being a straight
     or a box edge, the whole loop when there is no straight); `joint_width` automatic Length
     (0 = 2t, the tab width of ADR 0082 §5); the Box keeps `finger_width` as its width key and
     shows it once (spec criterion 12, a rename would lose stored values); `joint_gap` automatic
     Length (0 = 2 × width), `ShownWhen::Is { joint_pattern, centred }`. An unknown pattern id
     reads `even` with a notice. Frage 384 B instead: ids `even`, `one_centred`, `two_centred`,
     `fewer_at_ends`, no `joint_gap`.
   - *Centred* (Proposal): the group sits about the edge's middle; count 1 is the middle; with
     two or more the gap between the middle pair (or either side of the middle joint) is g and
     each next gap outward adds g. If it does not fit inside the margins, g is lowered (notice).
   - *Constraints* (ADR 0082 §1, §5): margins at both ends at least the mating thickness and,
     for tabs, t_w from a straight's ends; a count, width or gap that does not fit is clamped
     with a Dependent notice, never refused. The seam straight takes the next even count, and
     the pattern is mirrored about the seam (one centred becomes two). Bent spans keep the even
     rule and read `joint_width`; a closed loop has no ends and is always even. **A typed count
     that gives at least two tabs per plate on straights suppresses tabs on bends (Proposal,
     KURZ-ID JOINT-COUNT, Frage number from the PO).** Path-box corner fingers (angled, ADR 0083
     §5) stay even.
   - *Guard*: all keys at default give today's output byte for byte; the path-box "manifest
     unchanged" test (ADR 0082 §11) changes on purpose.
4. **The `joint` Choice goes in slice 3 (lead decision, Proposal).** As ADR 0082 §3 decided:
   remove `JOINT`, the `joint` row and its two `ShownWhen`, and the read in `boxes/mod.rs`. A
   stored `joint: none` box opens as stored and gets fingers on its next edit; the stored key is
   kept and ignored.
5. **`format_version` unchanged in every slice.** The new keys are data, a missing key reads its
   default, the parts stay closed outlines (ADR 0080 §5). An older Curvyo opens a newer file as
   stored, not refused (spec criterion 19); an edit there regenerates by its own rules and keeps
   the keys it does not know.
6. **Tests** (amends ADR 0082 §9). Slice 1: re-blessed goldens; the outer column of a
   window-facing end at w_s/2 + swing; a band without a window unchanged. Slice 2: the uncut-band
   test does not count a tab slot as a bending cut and holds on every bent span; every clipped
   column keeps t_w from its slot; plates on the circle within the kerf, with five thicknesses;
   a new box stores `curve`, a record without the key reads `window`; goldens
   `rounded_box_circle_curve.txt`, `path_box_heart_curve.txt`. Slices 3 and 4: `joint_layout`
   unit tests per pattern (centre within 0.01 mm at count 1, no overlap, margins); the mating
   test per edge and pattern; occupancy (ADR 0082 §9) per pattern; one golden per pattern.

## Consequences

- The circle and path-box curves bend far rounder in slice 1 with no new field.
- A1 trades a stiffer joint for a round wall; risks are cracks at slot corners in brittle stock
  (MDF) and at tight radii, and a locally flat island at the plate's height. Test-cut at the demo.
- Spec `0155` needs these edits by the PO: criterion 2 "radial sides" → "sides parallel to the
  normal at the tab's centre", "nearest slit at least the minimum spacing" → "clipped columns keep
  t_w"; criterion 3 "off = today's output" → "off = slice 1's window output"; criterion 5's bound
  as decision 2; criterion 6 (no row on the curved top); Frage 385 recorded; a Frage number for JOINT-COUNT.
- `Param.initial` is available to the next option that must leave stored objects alone.
- Slicing (Proposal): A0, A1, B for tabs (rounded box, path box, curved-top back tabs, and the
  `joint` removal), B for fingers (Box, curved-top plates). Each is visible in the demo.

## Customer questions

Frage 385 (JOINT-WINDOW): answered A 2026-11-26. Open with defaults: Frage 381 (JOINT-BEND, A),
382 (JOINT-SIZE, A), 383 (JOINT-SCOPE, A), 384 (JOINT-PATTERNS, A), and JOINT-COUNT (decision 3,
number from the PO). Texts in the spec.

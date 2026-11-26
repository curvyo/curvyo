# ADRs for the generator joint options

Status: Draft (architect, 2026-11-25; ADR 0087 written 2026-11-26), written beside the PO's first draft of `specification.md`,
so criteria are named by topic, not by number. Read against `curvyo-generators-core` at `e0fc2ebe`
(`tab_layout.rs`, `tab_runs.rs`, `wall.rs`, `wall_bands.rs`, `slit_bend.rs`, `footprint.rs`,
`wall_plates.rs`, `fingers.rs`, `rounded_box/`, `path_box/`, `boxes/`) and the golden
`tests/fixtures/rounded_box_circle.txt`. Decisions marked **(Proposal)** are the architect's and
vetoable at the demo. Customer questions carry a KURZ-ID only; the PO assigns their numbers
(381 to 384 are reserved).

- [ADR 0087](../../docs/adr/0087-generator-joint-options-tabs-on-the-curve-and-joint-layout.md)
  (new, Proposed 2026-11-26): decides this feature. Decision 1 tighter windows (A0, decided,
  Frage 385 A), decision 2 option `bend_tabs` with `Param.initial` (A1, Frage 381 default A),
  decision 3 the shared `joint_layout.rs` and its keys (Part B, Fragen 382 to 384 default A,
  JOINT-COUNT needs a Frage number), decision 4 the `joint` Choice removed in the B-tabs slice
  (lead, Proposal), decision 5 no `format_version` change. Amends ADR 0082 §1, §5, §6, §9, §11
  and ADR 0083 §13.4, §13.6, §13.8 by reference.
- [ADR 0082](../../docs/adr/0082-generator-plate-joints-corner-rule-and-rounded-box.md): the
  corner rule (§1), the tab-and-slot rule and the flat window (§5), the slit pattern (§6), the
  `automatic` Length and Count (§7). Amended as above.
- [ADR 0083](../../docs/adr/0083-box-from-a-closed-path.md): windows on runs (§13.8), run
  columns (§13.6), the byte-identical guard (§13.4). Amended as above.
- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md)
  and [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §5: new keys are data,
  an old file reads the defaults, keys and option ids are file format. Unchanged.

## Findings (2026-11-25)

1. **Who is affected.** Windows on bends exist only where the straights hold fewer than four
   tabs (`MIN_STRAIGHT_TABS`): the circle, stadiums with short straights, path-box curves
   (heart, shield, ellipse). The default rounded rectangle (120 x 80, r 15) has tabs on its
   straights only; the curved top's tabs sit on the back straight. Neither changes.
2. **The flat is much longer than the window.** The window chord is l = 2t + 2t_w (12 mm at
   t = t_w = 3), but the band next to it starts one more bridge away (`wall_bands.rs`, `lead` /
   `trail`) and its first column stands half a pitch in (`slit_bend::slots`, ADR 0083 §13.6 the
   same). The unslit stretch across a tab is about pitch + 2t + 4t_w. Measured in
   `rounded_box_circle.txt` (60 x 60, r 30): 6 tabs, 12 slit columns in pairs 6.9 mm apart,
   25.7 mm between column centres across each tab. The wall is a hexagon: six flats of about
   25 mm (sag about 2.6 mm on r 30) and every column bends 30 degrees, so the hinge is also
   over-strained there, not only faceted. This, more than the window itself, is what the
   customer saw.

## Part A: slits through the tab windows

- **A0, tighter windows (no option; Proposal, but a change of accepted output, so a customer
  question, KURZ-ID `JOINT-WINDOW`).** The window's chord already holds the bridges t_w either
  side of the tab, so the band beside it starts at the chord's end with no further bridge, and
  the columns are laid from the band's ends (first and last column w/2 in), not half a pitch in.
  The unslit stretch falls to 2t + 2t_w + w (about 12.3 mm, sag about 0.6 mm on r 30), the
  columns keep the pitch the radius asks for (18 to 24 on the circle above, not 12). No key,
  no contract change; re-blesses `rounded_box_circle.txt`, `path_box_circle.txt`,
  `path_box_heart.txt`, `path_box_shield.txt` and `path_box_ellipse.txt` (the guard of ADR 0083
  §13.4 changes on purpose). Recommended as the first slice, whatever A1 becomes.
- **A1, tabs on the curve (an option).** The plate edge stays the true arc, no window and no
  chord in the footprint; the strip keeps its full column pitch through the tab's stretch.
  - **Slots and slits.** `slit_bend::Window` gains a height span `y0..y1` (the slot's rows). A
    column within w/2 + t_w of the slot along the strip is clipped, not dropped: it stops one
    bridge t_w from the slot's rows (Offset: the crossing slot shortened or dropped; Wave: the
    swing counts in the half width, as today). A Straight column clipped on its run-out side
    becomes a closed slot. At the defaults (`base_inset` 0, ring flush) both slots are edge
    notches, so the clipped zone is only the notch depth plus t_w at each edge.
  - **Minimum bridge t_w** round every slot, the pattern's own bridge (ADR 0082 §6). The slot
    does not weaken the strip below the hinge it sits in (every ligament is still at least t_w),
    but the joint is softer: today the tab bears on 12 mm of solid wood, here on t_w links that
    also twist with the hinge. Risks: cracks at the slot's corners in brittle stock (MDF) or at
    tight radii; a stiff island about (2t + 2t_w) wide over the slot's rows, so the wall stays
    round above and below it but is locally a chord at the plate's height. Test-cut at the demo.
  - **Tab shape: a rectangle on the normal (Proposal).** Sides parallel to the arc's normal at
    the tab's centre, root on the arc, tip the concentric arc r + t_w (flush outside; a straight
    tip would stand 0.1 mm proud). Rejected: a sector tab with radial sides. It fits only a slot
    that bends uniformly; the slot's island stays flat, and a sector is wider at the tip by
    about 2t t_w / r (0.6 mm at r 30) and would bind. A rectangle never binds.
  - **Mating.** "Plate on the footprint" loses its sag term: the kerf only. Tab centre to slot
    centre within c/2, by neutral-line arc length (exact, no chord-for-arc term in
    `Layout::push`). The island leaves a gap of at most (t + t_w)^2 / 2r under its ends
    (0.6 mm at r 30), inside the box, never an interference.
  - **Code.** Crate-internal only: `tab_layout.rs` / `tab_runs.rs` place tabs on arcs without
    splitting the footprint; `wall_plates::walk` gains tabs on a `Bend`; `slit_bend.rs` clips by
    `Window` height. No `curvyo-extension-core` change, no `format_version`.
  - **Tests that change.** ADR 0082 §9's uncut-band test must not count a tab slot as a bending
    cut (the slot holds a rigid tab); every clipped column keeps t_w from its slot; plates on the
    circle within the kerf; goldens `rounded_box_circle_curve.txt`, `path_box_heart_curve.txt`.
  - **Key (Proposal, file format):** Choice `bend_tabs`, `window` (default) | `curve`, in the
    Slits group of the rounded box (Flat) and the path box; an unknown id reads `window`, as an
    unknown slit kind does. Customer question `JOINT-CURVE`: A an option, default window
    (recommended until a test cut holds); B replace the window; C A0 only.
- **Alternatives weighed.** Shorter windows: that is A0, the cheap half of the gain. Tabs only
  on straights when any exist (threshold 2 instead of 4): helps stadiums, not the circle or the
  heart; a product choice, folded into Part B's count rule below. Edge notches instead of slots
  on bends: at the defaults the slots already are edge notches; forcing e = 0 on bends removes
  no flat. Rejected as separate options.

## Part B: joint layout (count, size, pattern)

- **One pure 1-D layout, shared (Proposal).** `joint_layout.rs` at crate level (or grown in
  `fingers.rs`): `layout(length, margins, count, width, pattern) -> Vec<(centre, width)>`. Tabs
  take the list as their places; a finger joint takes it as the mate's pieces with the owner's
  pieces between, so the odd count and the owner's end pieces hold by construction. Both mating
  parts read the same list per edge, or the mating tests break.
- **Contract: none.** A `Choice`, an automatic `Count` and an automatic `Length` exist (ADR 0082
  §7). Keys (Proposal, file format): `joint_pattern` (`even` default; further ids with the
  customer, question `JOINT-PATTERN`: one centred, two centred closer, fewer at the ends),
  `joint_count` (automatic: today's rule; typed: per side, a side being a straight or a box
  edge, the whole loop when there is none), `joint_width` (automatic: 2t). The Box keeps
  `finger_width` as its width key (a rename would lose stored values). One group function in
  crate `schema.rs`, used by the Box, the rounded box (both tops) and the path box. An unknown
  pattern id reads `even` with a notice, as ADR 0082 §6. Defaults give today's output byte for
  byte (the guard). The "path box manifest unchanged" test of ADR 0082 §11 changes on purpose.
- **Constraints from the corner rule (ADR 0082 §1).**
  - Margins at both ends at least the mating thickness (the owner's corner cube); a count or
    width that does not fit is clamped (Dependent notice), never refused.
  - The seam straight needs an even count so the seam falls between two tabs; "one centred"
    there becomes two tabs mirrored about the seam (Proposal).
  - A typed count also decides the "fewer than four straight tabs, so windows on bends" trigger:
    proposed, a typed count suppresses bend windows when it gives at least two tabs per plate
    (question `JOINT-COUNT`, default that rule).
  - Bends and windows keep the even rule; patterns apply to straights and box edges only.
  - Path-box corner joints (angled fingers, tip bound of ADR 0083 §5) stay `even` in the first
    slice; patterns there are a later step.
- **Found while reading:** `boxes/schema.rs` still offers the `joint` Choice and its
  `ShownWhen`, although ADR 0082 §3 removes it. Does not block; the new ADR places the Box's
  keys beside it and the lead decides whether its removal rides along.

## Feature-local decisions

1. 2026-11-25: **`format_version` unchanged** in every slice: the new keys are data, an older
   file reads their defaults, the parts stay closed outlines (ADR 0080 §5).
2. 2026-11-25: **Slicing (for the lead, Proposal).** Slice 1 A0 (tighter windows); slice 2 A1
   (`bend_tabs`); slice 3 Part B for tabs (rounded box, path box, curved-top back tabs); slice 4
   Part B for finger joints (Box, curved-top plates). Each slice is visible in the demo.
3. 2026-11-26: **A0 decided** (customer, Frage 385 JOINT-WINDOW A): the band starts at the
   window's end, its outer column at half a slit width (plus a Wave's swing), only at ends that
   meet a window (ADR 0087 decision 1).
4. 2026-11-26: **The Box's leftover `joint` Choice is removed in the B-tabs slice** (lead,
   Proposal; ADR 0087 decision 4, as ADR 0082 §3 decided).
5. 2026-11-26: **For the PO** (ADR 0087 Consequences): criterion 2 tab sides parallel to the
   normal, clipped columns keep t_w; criterion 3 "off" means slice 1's window output; criterion 5
   bound as ADR 0087 decision 2; criterion 6 no row on the curved top; Frage 385 recorded; a Frage number for JOINT-COUNT. Slices named
   by content: A0, A1, B tabs, B fingers.

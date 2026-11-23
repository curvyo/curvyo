# ADRs for parametric generators

Status: Accepted (customer, 2026-11-23: Frage 326 GEN-STORED A, Frage 327 GEN-RAIL: More tools card). Frage 277 A, 278 A and EXT-NEXT A applied. One new ADR (ADR 0080, Accepted). Decisions marked **(Proposal)** are
the architect's and vetoable at the demo.

- [ADR 0080](../../docs/adr/0080-generated-objects-extension-trait-and-parametric-instances.md)
  (new): the `Extension` trait (`manifest`, `notice`, `generate`; registry of `&dyn Extension`);
  the generator contract `(kind, params, seed, edit) -> normalised params, outline, handles,
  notices`, pure, `libm` (criteria 2 to 4); the crate `curvyo-generators-core` (8); the leaf
  `shape = "generated"` with one atomic `generated` and `generated_frame` (5, 7); the unknown or
  disabled kind kept and drawn from the stored outline (7); one `instance.rs` shared with the
  sketch instances of `0099` (schema, edit, memo, handles, scale, Convert to paths; 1, 5, 6); labels
  (1, 6); tests and the CPU-time test (2 to 4, 7).
- [ADR 0034](../../docs/adr/0034-extension-contract-compiled-in.md) §3, §4, §8: amended by 0080
  (trait, `Contribution::Generator`, four parameter kinds, closed-path type); §1, §2, §5 to §7
  unchanged (crate naming, dependency rules, stable keys, `extensions.toml`, licence).
- [ADR 0033](../../docs/adr/0033-manufacturing-data-in-the-project-file.md) §2 to §4: the pattern
  followed (`<extension id>/<kind>`, one atomic map, extension validates on read, unknown kept).
- [ADR 0029](../../docs/adr/0029-live-pattern-fill.md) §5, §6, §11: determinism, the memo compared
  in full, previews without a write; not its derive-on-read storage (0080 option B).
- [ADR 0016](../../docs/adr/0016-clone-overrides.md), [ADR 0045](../../docs/adr/0045-clones-of-groups-texts-images-and-movable-placed-pages.md):
  a clone of a generated object reads its outline as of any outline root; the sketch-side override
  `instance_params` lands with `0099`'s revision.
- [ADR 0011](../../docs/adr/0011-workspace-and-crate-layout.md) §2, §3: note for the new crate
  and `ui-core → generators-core`.

## Feature-local decisions

1. 2026-11-23: **Generators entry.** The create tools list `Contribution::Generator` kinds of the
   enabled extensions; placing writes the leaf with its defaults, the seed from the new node id and
   `generated_frame` from the release point, selected, in one commit `add_generated`. Where the
   entry lives (GEN-RAIL, decided: the More tools card), the ghost, the anchor point and the Properties layout are the UX notes'.
2. 2026-11-23: **Gear parameters** (keys are file format, ADR 0034 §5; groups and rows as the UX
   notes §5): `module`, `teeth` (Count 6..200), `pitch_diameter` (stored, kept equal to module x
   teeth by the generator; `Edit::Param("pitch_diameter")` writes Module = Pitch diameter / Teeth),
   `pressure_angle` (Angle, 20 deg), `bore` (0 = none), `keyway` (Switch, shown while bore > 0),
   `keyway_width`, `keyway_depth` (shown while keyway is on), `web` (Choice `solid` | `spokes`),
   `spokes` (Count 2..12), `rim_width`, `hub_diameter` (shown with spokes). The windows between
   spokes are always rounded by the generator (no field). Dependent and knock-on clamps (bore below
   the root circle, hub inside the rim, keyway inside the hub) are reported as `Clamp::Dependent`.
3. 2026-11-23: **Box parameters:** `inner_width`, `inner_depth`, `inner_height`, `thickness`,
   `kerf` (default 0), `joint` (Choice `fingers` | `none`), `finger_width` (default 2 x thickness at
   creation) and `clearance` (default 0; both shown with fingers), `lid` (Choice `none` | `flat` |
   `fingers`), `gap` (the layout gap). One part per plate (five without a lid), laid out side by
   side.
4. 2026-11-23: **What a scale changes** (criterion 5, as the UX notes §8): gear `module`,
   `pitch_diameter`, `bore`, `keyway_width`, `keyway_depth`, `rim_width`, `hub_diameter`; box
   `inner_width`, `inner_depth`, `inner_height`. Kept: `teeth`, `pressure_angle`, `spokes`,
   `thickness`, `kerf`, `finger_width`, `clearance`, `gap`; so a box after a resize is near, not
   exactly, the dragged size.
5. 2026-11-23: **Curves.** Involute flanks as cubics fitted to the exact involute within 0.01 mm;
   circles and window corners as cubics within 0.01 mm as well (one tolerance constant in the
   crate).
6. 2026-11-23: **Budget** of criterion 4 confirmed at 100 ms thread CPU, measured on the session's
   parameter edit including the commit and the frame's canvas view (ADR 0080 §8). No budget is
   set for a handle or scale drag preview; the implementer measures it and reports the number in
   the PR.
7. 2026-11-23: **`format_version`:** next free at merge (54 on main today). **For the lead:** one row
   in the `format_version` table of `specs/README.md`.

## Answers to the UX notes §14 item 3

- The schema elements of UX notes §4 are in ADR 0080 §2: integer (`Count`), angle, `step` and
  `decimals` on lengths and angles, choice, switch, `shown_when`, handle anchors (`Generated::handles`),
  bound report (`Clamp`). A derived length is an ordinary stored length the generator keeps
  consistent; no own element kind.
- An unknown generator record **survives a save** unchanged (ADR 0080 §5); the notice wording of §12
  stands. A scale of such an object is refused with a status notice (UX: wording, please).

## Spec issues (resolved by the product owner, 2026-11-23: all five are written into the criteria of `specification.md`)

1. Criterion 7 "the paths are recomputed": with Frage 326 A the saved outline opens as saved and a
   test asserts that regenerating from the saved numbers gives the same outline; the outline is
   rebuilt on the next parameter change.
2. Criterion 7 "opens with the last paths as plain paths": the object is drawn, cut, moved,
   rotated, copied, deleted and converted from its stored outline and the save keeps its record;
   it cannot be resized or changed by number; it is not converted on open, so a build that knows
   the kind gets its parameters back.
3. Criterion 3 "in one group": the live box is one object (one leaf with one outline per plate,
   not enterable, UX notes §8); Convert to paths gives the group of plates.
4. Criterion 5: resize is proportional only (UX notes §8); the lengths that scale are feature
   decision 4. Please state both in the criterion.
5. Criterion 4: also name the dependent and knock-on clamps (feature decision 2) as notices, not
   only the single-value bounds.

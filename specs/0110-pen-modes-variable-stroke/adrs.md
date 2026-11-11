# Architecture decisions — pen modes and variable width

## Customer direction, 2026-11-07

The customer approved CURVES A (retain editable Spiro and B-spline control
points), WIDTH A (canvas width controls), OUTPUT A (existing centre-line
manufacturing output; explicit Stroke to path for the visible outline), and
STATUS A (remove the bar without a coordinate replacement). The detailed
implementation choices below are proposals, subject to review at the demo.

## Dependencies

- [ADR 0002](../../docs/adr/0002-document-model-units-and-svg-round-trip.md)
  §6 currently limits paths to lines and cubic Béziers; the new curve model
  extends that decision. Primitives remain primitives.
- [ADR 0003](../../docs/adr/0003-geometry-kernel-booleans-offsetting-vcarving.md):
  pure geometry, explicit tolerances and the existing integer kernel.
- [ADR 0004](../../docs/adr/0004-persistence-and-cross-machine-sync.md) §9:
  bump the document format so older readers cannot silently lose either feature.
- [ADR 0009](../../docs/adr/0009-concurrent-editing-semantics.md) §3: retain
  independently mergeable path anchors and commit edits through the journal.
- [ADR 0015](../../docs/adr/0015-node-references-and-ancestor-resolved-state.md)
  and [0016](../../docs/adr/0016-clone-overrides.md): linked clones resolve
  their source geometry and may have their own style.
- [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md):
  model and format approved by the customer on 2026-11-07.

## Implementation boundaries (Proposal, 2026-11-07)

The document owns editable curve and profile data; pure geometry derives
Bézier/outline geometry for rendering, hit testing, conversion and export.
The editor composes the results; `render-core` does not gain a dependency on
`geometry-core`. Keep all core crates wasm-compatible. No new crate is
approved. Any additional dependency needs a licence, wasm and `cargo deny`
review before use; no particular Spiro implementation has yet been selected.

The centre-line rule applies to this profile, not to the separate brush feature:
`0033-stroke-brushes` explicitly specifies cutting generated brush pieces. Do
not silently substitute either behaviour for the other. `0032-pen-tablet-input`
still describes flattened pressure outlines; using this profile for future
pressure input requires updating its criteria when that story is resumed.

## Renumbering, 2026-11-10

This ADR was drafted as 0045; `main` took 0045 for another decision and reserved 0046 and 0047, so it is now [0048](../../docs/adr/0053-editable-curves-and-width-profiles.md). The spec was drafted as 0106; `main` took 0106 (`move-versus-edit`), so this story is 0110.

## Outline replaced, 2026-11-10

- [ADR 0053](../../docs/adr/0053-editable-curves-and-width-profiles.md), amendment 2026-11-10 (customer option A): the variable-width outline is sampled, offset along the normal and united in the existing integer kernel at 0.01 mm, without certification; it runs synchronously in the session within ≤ 2 ms native / ≤ 4 ms wasm for typical paths; only degenerate input is refused; the certified kernel, the Spiro spike tests and the width worker are deleted. The Spiro worker section is replaced by synchronous evaluation in the session. Model and format 40 unchanged.
- 2026-11-10 note: Stroke to path of a varying profile yields line segments within 0.01 mm, not curves (debt entry in `docs/technical-debt.md`).

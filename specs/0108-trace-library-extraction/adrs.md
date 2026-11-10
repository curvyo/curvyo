# ADRs for the trace library extraction

Architect, 2026-11-10.

- [0048](../../docs/adr/0048-vectrace-crate.md) (Proposed): the crate `vectrace`, what moves and
  what stays, units (`Mm`, `Angle`, `PixelMap` with `mul_add`), the `Mask` seam, the public API
  (`Job`/`run`, `wire::read`, `Primitive`) and the licence. It is the target of slices 1 to 4.
- [0023](../../docs/adr/0023-raster-trace-crate-pipeline-and-worker.md) and
  [0028](../../docs/adr/0028-ocr-text-in-trace.md): extended by notes. The pipeline, the
  determinism rules, the OCR API and the `rayon` exception keep applying, now in `vectrace`.
- [0011](../../docs/adr/0011-workspace-and-crate-layout.md): one crate is added and two are
  removed (note). [0006](../../docs/adr/0006-license.md): `vectrace` is MIT OR Apache-2.0 (note).

There is no `format_version` change and no new external dependency. Inventory and the task list
for slices 1 to 3: [`inventory.md`](inventory.md).

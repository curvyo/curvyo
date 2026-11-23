# Changelog of the extension contract

The contract is `0.x` and unstable (ADR 0034 §5). Parameter keys and fixture kinds an extension
publishes are written into project files (ADR 0033 §3): renaming one is a format change with a
migration in that extension.

## Unreleased (`specs/0141-parametric-generators`, ADR 0079)

- The `Extension` trait (`manifest`, `notice`, `generate`); the registry holds `&'static dyn Extension`.
- `Contribution::Generator(GeneratorKind)` with `ParamGroup`s; the generator contract
  (`GenerateRequest`, `Edit`, `Generated`, `Part`, `ClosedPath`, `Cubic`, `Handle`, `Clamp`, `Seed`).
- Parameter kinds `Count`, `Angle`, `Switch`, `Choice`; `Length` gains `step` and `decimals`; `Param`
  gains `shown_when`. Stored encodings `{"deg": n}`, `{"count": n}`, `{"on": b}`, `{"choice": "id"}`.

## Unreleased (`specs/0026-machine-profile`)

- Block kind `MachineProfile { label }`: the machine profile picker and the copy's fields, drawn
  by the shell; the extension only places it.
- Removed (no user left): block kind `Params`, `HostKey`, `ParamKind::Display`,
  `ParamValue::Text`, and `WorkbenchContribution::{machine_name, work_width_mm, work_height_mm}`;
  a new manufacturing's machine and work area come from the machine profile (ADR 0039 §8).

## 0.1.0 (2026-11-02)

- `Manifest`, `Contribution::Workbench` with tabs, toolbox cards and fixture kinds.
- Block kinds `Params`, `FixtureList`, `Notice`.
- Parameter kinds `Length { min, max }` and `Display` (a value the host shows, not edited).
- Stored encoding of a length: `{"mm": <number>}`.
- `FixtureDrawing`: open line segments in page-local millimetres with one line width.

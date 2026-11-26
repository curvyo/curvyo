# Changelog of the extension contract

The contract is `0.x` and unstable (ADR 0034 §5). Parameter keys and fixture kinds an extension
publishes are written into project files (ADR 0033 §3): renaming one is a format change with a
migration in that extension.

## Unreleased (`specs/0141-parametric-generators`, ADR 0080)

- The `Extension` trait (`manifest`, `notice`, `generate`); the registry holds `&'static dyn Extension`.
- `Contribution::Generator(GeneratorKind)` with `ParamGroup`s; the generator contract
  (`GenerateRequest`, `Edit`, `Generated`, `Part`, `ClosedPath`, `Cubic`, `Handle`, `Clamp`, `Seed`).
- Parameter kinds `Count`, `Angle`, `Switch`, `Choice`; `Length` gains `step` and `decimals`; `Param`
  gains `shown_when`. Stored encodings `{"deg": n}`, `{"count": n}`, `{"on": b}`, `{"choice": "id"}`.

- ADR 0082 §7 (amended by the architect): `Param` gains `track: Option<Track>` (`from`, `to`, `Curve::Linear` or `Log`): the span of the slider a number
  is drawn on (`0141` revision 2 criterion 9). Source-breaking for code that builds a `Param` literal.
- `ParamKind::Length` and `Count` gain `automatic: bool` (a stored 0 passes `Param::accept`, any other value keeps
  the bounds); `Generated` gains `automatic` (the values used for parameters stored as 0), ADR 0082 §7
  (`0147`). Source-breaking for code that builds a `ParamKind` or `Generated` literal.
- `ShownWhen::AllOn { keys }`: a row drawn only while several switches are on (`0147`: Ring thickness and
  Finger hollow need Lid and Lid seat).
- ADR 0082 §7 revision 5 (`0147`): `Edit::Draft` (only outer outlines are wanted, for a ghost; never
  stored), `Generated.states` (a word for a value at a limit that is a shape, "full round"),
  `Generated.limits` (the largest value a dependent bound allows now) and `Bound::DependentMinimumFor`
  (a minimum that a choice sets, "the least the Wave pattern allows"). Source-breaking for code that builds
  a `Generated` literal or matches `Bound` or `Edit` exhaustively.

- ADR 0082 §7 revision 6 (`0147` slice 2, the curved top): `ShownWhen::All { conditions }` and `ShownWhen::Any { conditions }` over
  `&'static [ShownWhen]`, so a row can hang on a switch and a choice at once (Lid on and Top shape Flat). Run time only, never
  stored. Source-breaking for code that matches `ShownWhen` exhaustively.

- ADR 0083 §2, §11 (`0149` path box): `ParamKind::Path` and `ParamValue::Path(ClosedPath)`, stored
  `{"path": [x0, y0, c1x, c1y, c2x, c2y, x, y, ...]}` (shape check: 2 to 2,000 cubics, finite, within
  1e7 mm); `ClosedPath::{flat, from_flat}`; `GeneratorKind.source: Option<Source>` with
  `Source { param, command }` and `GeneratorKind::segment`; `Generated.warnings: Vec<Warning>` and
  `Generated.marks: Marks` (`Mark::{Corner, Bend, Piece}`), run time only, never stored. Source-breaking
  for code that builds a `GeneratorKind` or `Generated` literal or matches `ParamKind` or `ParamValue`
  exhaustively.

- ADR 0083 §13.14, §13.16 (`0149` slice 2, curves): `Mark::Seam { at, inward }` and the new meaning of `Mark::Bend`
  (one mark per stretch of one tightness under a slit band); the warning ids `cover_rounded`, `cover_offset_failed`, `fillet_failed` and `bend_too_short` are added and
  `outline_curved` is removed. Source-breaking for code that matches `Mark` exhaustively.

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

- ADR 0089 decision 2 (`0155` slice 2): `Param` gains `initial: Option<ParamValue>`, the value a new object is written
  with when it differs from `default` (`Param::defaults` writes `initial` first; `Param::read` is unchanged, so a
  stored object without the key reads `default`). Serialised only when set. Source-breaking for code that builds a
  `Param` literal.

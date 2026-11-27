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

- ADR 0082 §10 (`0147` slice 3, Detach part): `Part` gains `key: Cow<'static, str>`, a stable key unique within one
  result, so an operation can hold `member.<node>#<key>` for one part: 1 to 15 characters of `a-z`, `0-9` and `-`; the host
  refuses a generation whose keys break that or repeat. A gear `gear`, a Box `bottom`, `front`, `back`, `left`, `right`,
  `lid`, a rounded box `strip`, `base`, `cover`, `ring`, `lid` (and in Curved `left`, `right`, `front`), a path box
  `wall-<n>` first. Source-breaking for code that builds a `Part` literal. The host stores the keys beside the parts
  (`format_version` 65).

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

## Unreleased (`specs/0144-qr-and-barcodes`, ADR 0091)

- `ParamKind::Text { multiline }` / `ParamValue::Text` (stored `{"text": "..."}`, at most 8,192 bytes) and
  `ParamKind::Percent { min, max }` / `ParamValue::Percent` (stored `{"pct": n}`); `ShownWhen::Full { key }`
  (a percentage at its maximum); `Bound::DependentFor(subject)` ("the most error correction M allows").
  Source-breaking for an exhaustive `match` on `ParamKind`, `ParamValue`, `ShownWhen` or `Bound`. No
  `format_version` change: values are opaque to document-core.
- Host rule: a generator that cannot encode its input answers `parts: []` plus a `Warning`; the host keeps the
  stored parts and writes the new params (ADR 0091 §3).

## Unreleased (`specs/0144-qr-and-barcodes` slice 2, ADR 0092)

- `Warning` gains `param: Option<&'static str>` (the row a lasting line stands under when the generator answers no
  parts) and `subject: Option<&'static str>` (the words naming the cause); `GeneratorKind` gains `scanned: bool`
  (the contrast check), `family: Option<Family>` (`Family { id, segment, label }`: one bar segment and a Symbology
  group per family) and `resize: Resize` (`Proportional` or `Free`).
- `ParamKind::Font` / `ParamValue::Font(Option<FontRef>)` (stored `{"font": null}` or `{"font": {"family", "style",
  "face_hash", "face_index"}}`), `ParamKind::Ratio { min, max, step, decimals }` / `ParamValue::Ratio(f64)` (stored
  `{"ratio": x}`); `ParamKind::Percent` bounds and `ParamValue::Percent` become `i32` (stored `{"pct": n}` unchanged,
  a negative number now reads).
- `GenerateRequest.glyphs: Option<&GlyphTable>` with `GlyphTable { glyphs }` and `Glyph { paths, advance }` (outlines
  of one face at a size of 1 mm, built by the host); `Edit::Stretch { x, y }` for a `Resize::Free` kind.
- Source-breaking for struct literals of `GenerateRequest`, `Warning` and `GeneratorKind` and for exhaustive matches
  on `ParamKind`, `ParamValue` and `Edit`. No `format_version` change: values stay opaque to document-core.

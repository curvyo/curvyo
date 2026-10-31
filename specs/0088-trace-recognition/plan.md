# Plan for Trace recognition, Part A (Recognise text)

Implementer, 2026-10-31, on `story/text-recognition` (cut from `main` `d8b79e64`). Built against
`specification.md` criteria 1 to 10, `adrs.md` decisions 1 to 11 (as aligned 2026-10-31) and
`ux-notes.md`. Everything marked **(Proposal)** is mine and vetoable at the demo.

## Affected crates/modules

- `docs/adr/0011-workspace-and-crate-layout.md` §2: the dated note (text-core also matches
  outlines back to glyphs).
- **`curvyo-text-core`**, new folder `src/recognise/` (decision 2): `mod.rs` (stages in order and
  the stepping `Recogniser`; public: `Recogniser`, `LineMatch`, `RecognisedRun`, `NearestFace`),
  `sample.rs`, `blobs.rs`, `lines.rs`, `candidates.rs`, `descriptor.rs`, `shortlist.rs`,
  `distance.rs`, `size.rs`, `verify.rs`, `shape.rs`, `reference.rs`, `assemble.rs`, each under
  500 lines. `Face` gains a
  private `OnceLock` of reference glyphs and `recognition_ready`/`prepare_recognition`, so the job
  builds a cold face on a step of its own.
  Tests: `tests/recognise.rs` (synthetic harness: included faces, 5/10/24 mm, 0/7/-30 degrees,
  ±0.05 mm wobble, ground truth).
- **`curvyo-ui-core`**, new `text_recognise_plan.rs`: the subjects of a selection (filled
  outlines in document mm, skips counted), the five face tiers, and the write plan (texts and the
  leftover pieces). Ignored chain test `tests/recognise_reference.rs` (render, rasterise, trace,
  recognise; `CURVYO_TRACE_LOCAL` images with a local truth file).
- **`curvyo-editor-wasm`**: `session/recognise.rs` (the job: start, step, stop, one history step
  "Recognise text", the session record of guessed letters), `wasm_recognise.rs` (codes and counts;
  the sentences are the host's), the Select bar's availability flag, history name.
- **`frontend`**: `RecogniseBar.tsx`, `RecogniseTextButton.tsx`, `hooks/useRecognise.ts`,
  `lib/recogniseText.ts` (all sentences), wiring in the Select bar and the tool switch; glyph table
  and text tool highlight of the guessed characters; e2e `tests/e2e/recognise.test.mjs`.
- `docs/design-system.md`: rows of `ux-notes.md` §10.

## Design choices beyond the ADR (Proposal)

- Stepping: `Recogniser::new(outlines, tolerance)` does stages 1 and 2 once; `Recogniser::line(i,
  tier)` does stages 3 to 7 for one line and one tier; `recognise()` is the one-call wrapper of
  the ADR. The session runs one (line, tier) per step so the host can paint progress and take
  Escape between steps (UX §3); faces a tier needs that are not loaded are asked for through the
  existing wanted-files path and the step waits for them.
- Distance: point to polyline (segments in grid buckets), so the 0.05 mm sample spacing of the ADR
  is kept for the final check only; ranking uses a coarser spacing. Basic operations and `sqrt`.
- The baseline direction is refined once per line by a least-squares fit through the matched
  glyph origins (the principal axis of the centres is the first guess).

## Tasks

- [x] 1. ADR 0011 §2 dated note (adrs.md decision 1).
- [x] 2. text-core `sample`, `blobs`, `lines` with unit tests (AC 1, 2; decision 3 stages 1, 2).
- [x] 3. text-core `descriptor`, `distance`, face descriptor cache (decision 4, 5).
- [x] 4. text-core `verify`, `assemble`, `Recogniser`/`recognise`: segmentation, shortlist,
      verification, 15 % margin, guessed letters, 50 % line rule, re-layout final check (AC 1, 2,
      3, 4, 7).
- [x] 5. Synthetic harness `tests/recognise.rs`: accuracy, wrongly accepted = 0, determinism twice,
      CPU time vs decision 8, hold-out family (AC 1, 2, 3, 4, 5, 7).
- [x] 6. ui-core `text_recognise_plan`: subjects and skips, tiers with cap 64 and earliest-tier
      dedupe, tier stop at 80 % / fallback at 50 %, the write plan with style and place (AC 3, 6,
      8, 9).
- [x] 7. editor-wasm job, one history step "Recognise text", guessed record, wasm surface (AC 6, 7,
      8, 10).
- [x] 8. Frontend: button (hidden, not disabled), bar (tolerance, faces readout, finish, progress,
      stop/retry/failed), notices, glyph table and text tool highlight, persisted tolerance (AC 4,
      6, 7, 10).
- [x] 9. Chain harness (ignored) and the customer's logos with text from `tests/assets/trace-references/`
- [x] 10. Docs: design-system rows, technical debt for any missed budget; full gate.

## Built differently from the plan (Proposal)

- The Recognise bar is a host state over the Select tool, not a session tool: the session tool
  stays Select (no new `Tool` variant), any other tool closes the bar.
- A line needs at least two sure letters or digits to become text, and the nearest face is named
  from lines of several glyphs: the chain harness on the customer's logos showed decoration dots
  accepted as full stops (`adrs.md` 3a only had the 50 % rule).
- Lead decisions 2026-10-31: closed paths without fill are read as filled (a text after Object to
  path is stroked), and the nearest face's deviation is its largest per-letter one, not the
  median.
- Enter in the Tolerance field opens its number entry as in every `ValueField`; Ctrl+Enter runs
  (the Trace bar's rule).

## Functions over 60 lines (architect review 2026-10-31)

Split: `prepare` (into `ordered`, `runs`, `candidate`, `cuts`), `fit_face` (`trials`,
`segmentation`), `match_line` (`fits_of`), `assemble` (`spaced`, `anchor`, `check`),
`recognise_tiers` (`document_faces`, `later_tier`), `RecogniseJob::step` (`record`). Kept, each
one sequence that reads top to bottom:
- `lines::group_lines` (62): the axis, the bands and the line frames of one grouping pass.
- `distance::nearest` (69): one bucket-grid walk with its ring-by-ring early exit; splitting it
  costs the hot path a call per sample.
- `sample::sample_closed` (62): one pass over the path's segments with the spacing fallback.
- `text_recognise_plan::plan_recognise_write` (72): one walk over the runs, then one over the
  touched paths, writing one plan.
- `assemble::spaced` (65): the spaces, the letter spacing and the word spacing, each from the
  layout before it.
- `size::size_and_baseline` (63): the proposals, the vote and its recentring, one estimate.
- `verify::fit_at` (64): trials, segmentation and one judgement per letter (shape, fine distance).
- `Session::finish_recognise` (65): plan, write, then word each line, in that order.
`verify::fit`, `recognise_subjects` and `RecogniseOutcome::to_json` are at 60 lines or under.

## Results (2026-10-31 after the tester findings, native release, build VM)

- Synthetic harness (`curvyo-text-core/tests/recognise.rs`, 180 cases: 5 faces, 5/10/24 mm,
  0/7/-30 degrees, ±0.05 mm wobble), at 0.3 mm: letters right 1962/1980 (99.1 %), **wrong sure
  0**, guessed 41, lines left 0, face right 180/180, size error max 0.113 mm, deviation max
  0.298 mm. At 2.0 mm: 1962/1980, **wrong sure 0**, guessed 61, face right 178/180.
- Cross family (Serif against the Sans faces and back, 3/5/10 mm, 0.05 to 2.00 mm in 9 steps):
  **wrong sure 0**.
- Chain harness (render, 300/600 dpi, Logo trace, 7 included faces; 60 cases): 504/600
  (84.0 %), **wrong sure 0**; Sans and Serif 480/480; Great Vibes 24/120 (joined script stays
  paths). Hold-out: 15 lines left as paths, wrong sure 0.
- The customer's logos with text: 19 lines, 0 texts (their faces are not included), **wrong
  sure 0**.
- Tester sweep (7540 letters): wrong sure 0, wrong face 0; 120 mm 826 of 1885 sure (was 74);
  90 degrees 479 of 1508; 180 degrees 0 (upside-down text is out of scope, `adrs.md` decision 3
  stage 2).
- CPU (thread CPU): all targets of decision 8 met; one script line can take 2 s in one step
  (`docs/technical-debt.md`).

## Validation

- Synthetic harness numbers (characters right, wrong accepted = 0, left as paths, face right, size
  error, max deviation, thread CPU) reported in the PR.
- Determinism: the synthetic set run twice gives identical results.
- Chromium e2e for the bar, notices and undo, file by file.

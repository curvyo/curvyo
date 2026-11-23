# 0121 Print and cut drafts (ux-engineer, 2026-11-23)

Static screens for `specs/0121-print-and-cut/`; real tokens, 280 px panel body (244 px content). Dashed
outline in a draft = new or changed control. The notes are `specs/0121-print-and-cut/ux-notes.md`;
proposals there are P1 to P22, spec deltas E1 to E12. HTML only in this pass: the PNG beside each is
rendered at the next browser pass.

- `01-marks-on-drawing-page`: the Document tab of a Drawing page with **Add registration marks** under Fit
  to content; the marks object selected (one box, no handles); its Style tab block (Size, Line width, Add
  mark n); entered with one mark selected and the refusal at 2 marks.
- `02-print-and-cut-group-states`: the Operations tab group in six states: not measured with the machine
  connected; measuring (a busy read, a greyed Take head position with "Machine is moving"); fit OK; warn
  (error above the tolerance, scale off, a gone mark); typed only without a connector, placement differs
  after a hand move; too close, an invalid entry and a failed read.
- `03-laser-canvas-fit`: the laser page canvas before Apply (faint crosses, measured rings, the dashed fit
  outline, the focused row's mark) and after Apply.

No new colour: the guide green (`--guide`) for marks that never run, the selection blue for measured
positions and the preview, ink and muted ink; warn lines in the `OutputChecks` look.

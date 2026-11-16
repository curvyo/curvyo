# 0029 Laser job preview and output drafts (ux-engineer, 2026-11-16)

Static screens for `specs/0029-laser-job-preview-and-output/` (criteria 1 to 38, Fragen 219 to 226 answered); real tokens, 280px panel body plus its 32px strip, PNG beside each HTML. Dashed outline in a draft = new control. The toolpath, times and lengths are computed from one toy job (run order, inner first, nearest start, 2 Cut passes), so the readouts and the Output tab agree. The notes are `specs/0029-laser-job-preview-and-output/ux-notes.md`; proposals there are P1 to P22.

- `01-toolpath-view-and-play-bar`: the Route toggle after the Operations view toggle at the page name; the job drawn with the kind looks, travel dashed, numbered rings, arrowheads; the play bar under the page (whole job; scrubbed to 62 % with the head ring and the rest pale; playing in a narrow window with the bar stuck to the canvas edge).
- `02-output-tab-gate-states`: the Output tab in five states: ready with warns, blocked (Export off, tooltip names the first block), nothing runs on Generic laser, Home first on (Start from greyed, tooltip), Start from Head position (its warn).
- `03-air-assist-fixture-material-after-export`: Air assist as a switch row of the Machine block; the "Fixture jobs run with" pick open inline; right after Export (one notice, fixture jobs off, numbers move up).

Amended 2026-11-16 (customer review of PR #370, other output formats): the button reads "Export job file" and the Machine block has a read-only "Output: GRBL" row after Air assist; the screens are not redrawn, `ux-notes.md` 6.4 and 8 win.

No new colour: red (`--cut`, `--destructive`), pale orange (`--engrave-hatch`) and ink only; blue is the existing pressed and selected look.

# 0029 Laser job preview and output drafts (ux-engineer, 2026-11-16, amended 2026-11-17)

Static screens for `specs/0029-laser-job-preview-and-output/`; real tokens, 280px panel body plus its 32px strip, PNG beside each HTML. Dashed outline in a draft = new or changed control. The toolpath, times and lengths come from one toy job (run order, inner first, nearest start, 2 Cut passes), so the readouts and the rows agree. The notes are `specs/0029-laser-job-preview-and-output/ux-notes.md`; proposals there are P1 to P32.

- `01-toolpath-view-and-play-bar`: the Route toggle after the Operations view toggle at the page name; the job drawn with the kind looks, travel dashed, rings (the fixture jobs with the fixture glyph, each operation with its row number), arrowheads; the play bar under the page with a time-only readout (whole job; scrubbed to 62 % with the head ring and the rest pale; playing in a narrow window with the bar stuck to the canvas edge); the merged Operations tab beside it.
- `02-operations-tab-with-output`: the Operations tab with the output under the list, four states: ready with warns; Cut 2 unfolded (Own numbers only there) with the Start pick open (Machine origin, Head position, Home, then machine origin); blocked (Export off, tooltip names the first block); nothing runs on Generic laser.
- `03-air-assist-fixture-material-after-export`: the Machine block bug (the state row pushes Air assist down 36 px after its first press) and its fix (the state row's place is kept, switches at the right); the fixture material pick open inline in the Fixture jobs section; right after Export (one notice, fixture jobs off, ring numbers unchanged).

Amended 2026-11-17 (customer feedback on PR #371): no Output tab, no Head position warning, no metres in the readout, one Start pick instead of Start from and Home first. The earlier `02-output-tab-gate-states` is removed.

No new colour: red (`--cut`, `--destructive`), pale orange (`--engrave-hatch`) and ink only; blue is the existing pressed and selected look.

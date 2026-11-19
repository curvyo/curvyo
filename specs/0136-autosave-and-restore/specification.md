# Autosave and restore: little is lost in a crash, also for a project that was never saved

Status: Draft (decisions of 2026-11-19 applied; not Ready: `adrs.md` and UX notes are missing)
Priority: Should
Origin: Customer (2026-11-19, translated): "Autosave and restore: as little as possible should be lost on a crash, also when the project was never saved initially." Everything marked **(Proposal)** is the product owner's and vetoable at the demo.

## User value

As a maker who has drawn for an hour on a new, unnamed project I want the app to keep a recovery copy while I work and offer it back after a crash, a power cut or a closed tab, so that I lose seconds of work, not the afternoon.

**What other tools do.** Inkscape writes an emergency save when it crashes (not on a power cut) and, with a preference, autosaves into a folder. Affinity and Illustrator keep a recovery copy that they offer on the next start. LightBurn autosaves into a backup folder every few minutes. We do better: the copy follows the committed history steps (at most one write per 5 minutes, criterion 1), covers a never-saved project too, and the offer sits in the File tab, not in a dialog.

Today (`0001`, out of scope there; `R-SYS-010`, `0051`): a project is dirty when it differs from its saved file and the app asks before it throws that away. Nothing is written until the maker saves; a crash loses everything since the last save, and a never-saved project loses everything.

## Words

- **Recovery copy:** a second, hidden copy of the open project, written by the app. It is never the maker's file: Save and Save as stay the only ways to write the real file.
- **Recoverable:** a project that has a recovery copy newer than its saved file, or has no saved file at all.

## Acceptance criteria

1. Given a project with changes (saved or never saved), then a recovery copy of the whole project (all pages, the history, the document formats, the same content as Save writes) is written by **one rule** (Frage 266, the product owner's reading of the customer's "after every step, at most every 5 minutes"): a committed step is written at once (at the next idle moment, within 1 s) when the last write is 5 minutes or more ago; steps committed within the 5 minutes after a write are not written one by one, but exactly one trailing write happens when the 5 minutes expire with steps not yet written; given no step since the last write, nothing is written. **Worst-case loss:** the steps committed after the last write, which is less than 5 minutes of work (and only the step in flight, under 1 s, after a quiet period). A test with a fake clock drives the rule; timing tests measure thread CPU time of the write path, not wall time (`CLAUDE.md` §5). **Revisitable at the demo:** the 5 minutes are a constant of the core, not a setting (criterion 13).
2. Given a recovery copy being written, then the write never blocks the canvas: a drag of 60 fps on a project with 10,000 objects drops no frame because of it, and a crash during the write leaves the previous recovery copy intact (the new copy replaces the old one only when complete).
3. Given Save or Save as succeeds, then the recovery copy of this project is deleted; given Discard in the unsaved-changes check (`0051`) or a clean Close, then it is deleted too; given New, then no copy exists until the first step is committed.
4. Given the app starts and finds one or more recovery copies whose project is not open, then the File tab shows a block "Unsaved work from a crash" with one row per copy (the project's name or "Untitled", the time of the last step, the page count), and two icon buttons per row, Restore and Discard; Discard needs the inline second press (design system, inline delete confirm). No dialog, no popup.
5. Given Restore, then the project opens as an unsaved project (dirty, no file path, named "Untitled" or its name followed by " (recovered)" **(Proposal)**), history included, in one step; the row goes. The maker saves it where he wants; the recovery copy goes with that Save (criterion 3).
6. Given a recovery copy whose project had a saved file that has changed since (newer on disk than the copy's base), then the row says so ("File changed since") and Restore still opens the recovered state as a new unsaved project; it never overwrites the file.
7. Given a damaged recovery copy (truncated, or a newer format than this app), then it is listed as "Cannot be restored", Discard works, and nothing else is affected; the same named errors as opening a damaged project (`0001` criterion 7).
8. Given the maker has not changed anything since open or Save, then no recovery copy is written (a clean project is never recoverable).
9. Given the desktop app, then recovery copies live in the app's data folder, not beside the project, not in a workspace folder and not in a folder that is synced (`R-SYS-004`, `0095`, `0096`); given the browser build, then in the browser's own origin storage. Where they live is shown in the File tab block's help line only when a copy exists (no standing hint, `feedback_no_hint_labels`).
10. Given `R-SYS-007`, then a recovery copy never leaves the machine: it is not sent to the sync server, to a collaboration room or anywhere else; a project in a shared room keeps recovering from its own local copy of the room state, which the sync mechanism already holds, and this slice adds nothing to the network.
11. Given a project open in two windows or two browser tabs, then each has its own recovery copy (keyed by a per-session project id), and a copy of a project that is open in another window is not offered for restore.
12. Given the recovery copies of discarded or restored projects, then at most 20 copies and 200 MB exist; the oldest beyond that are deleted at start without a question **(Proposal)**.
13. Given the whole slice, then there is no preference, no setting and no switch for autosave (always on, `R-SYS-007` satisfied by criterion 10), and no toast after a successful write or restore; only a failed write (disk full, no permission) shows a notice in the File tab and the title area, once, until the next success.

## Out of scope

- Version history of saved files and restoring older saved states (the history of `0020`, `0042` is inside the project already).
- Cloud backup of recovery copies; a recovery copy of a project on a second device.
- Crash reporting or any telemetry about the crash (`R-SYS-007`).
- Recovering text typed in an unfinished entry chip or a drag in flight: the recovery copy holds committed steps only.

## Decisions

Customer, 2026-11-19:

- **Frage 265 (AUTOSAVE-WHERE): B.** Always in the app data folder; no hidden files in the maker's folders or in synced and shared folders (criterion 9).
- **Frage 266 (AUTOSAVE-INTERVAL): A and B combined.** A recovery copy after each step, but at most every 5 minutes. Exactly one rule is specified in criterion 1; it is the product owner's reading of "nach jedem Schritt, max alle 5 Minuten" (**Proposal**, vetoable at the demo): first step after a 5-minute gap written at once, plus one trailing write when the 5 minutes expire with unwritten steps; worst-case loss under 5 minutes.
- **Frage 267 (AUTOSAVE-OFFER): A.** An inline block in the File tab at the next start (criterion 4).

## Still open before Ready

- `adrs.md` (architect: recovery copy format and atomic replace, keying by per-session project id, browser origin storage, the 20 copies / 200 MB limit).
- UX notes (the File tab block of criterion 4).

## UX notes

(ux-engineer, before Ready.) The block of criterion 4 in the File tab; no popup.

## Links

Requirements: R-SYS-010 (extended), R-SYS-013 (new), R-SYS-007, R-SYS-002
Depends on: `0001`, `0051`, `0020` (the history is part of the copy)
Related: `0095` workspace, `0096` own-devices sync (their copies are not recovery copies)
PR: none yet

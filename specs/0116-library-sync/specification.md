# Fonts, machines and materials across my devices

Status: Ready (2026-11-13; ADR 0062 accepted 2026-11-13; Fragen 214 to 217 answered A)
Priority: Must
Origin: Customer (request of 2026-11-13, translated): "in curvyo installierte fonts und machines sollen über das sync feature übertragen werden können. dazu brauchen wir einen symbol toggle schalter an den fonts zum sync einschalten, ebenso an den machines." and "materials sollen natürlich auch gesynced werden ;)". Everything marked Proposal is the product owner's and vetoable at the demo.

## User value

As a maker who installs a font and sets up the K40 profile and the plywood material record on the workshop desktop I want them to appear on my laptop and my browser by themselves, so that I never re-install a font, re-type a machine profile or re-run a material test on the second computer, and my three libraries are the same everywhere.

## Problem today

- Each of the three per-user libraries is per-computer: Curvyo fonts are files in the app data dir fonts folder on desktop and bytes in IndexedDB `curvyo-fonts` in the browser, machine profiles are `machines/<uuid>.json` records (ADR 0038), material records are `materials/<uuid>.json` (ADR 0055); none of them leaves the computer. A font installed in the workshop is missing on the laptop; the same for the K40 profile and the material records. Copying the folders by hand risks divergence: two versions of the K40 profile with different limits, a material test judged on the laptop that the desktop never sees.
- The device sync of `0096` (Done) syncs workspaces only; its Out of scope deferred exactly this: "settings sync (... per-user font library), the machine profiles of `0026` ... each would be its own story" and, for slice 3, "the material library syncs the same way (R-MAT-005)".
- Standing requirements ask for it: R-SYS-004 ("Projects and the material/asset library sync across the maker's own computers"), R-FONT-006 (Curvyo's own font folder), R-MFG-001 (profile reused across projects), R-MAT-005 ("Material records sync across the maker's computers, same mechanism as R-SYS-004"), R-SYS-012 (my devices).
- Documents already carry their used fonts inside the file (`0107`); the maker's library itself does not travel.

## The field

- **Fusion 360**: the machine setup and the material library are attached to the Autodesk account and follow the maker to every computer — but only with an account and through their cloud. Lesson: the library follows the maker, not the computer. We do the same without the account.
- **LightBurn**: no library sync; "Transfer" copies a setup to a new computer once, and the material library is a `.mlib` file makers copy by hand. Lesson: hand-copied library files are exactly our divergence problem. We sync continuously, offline-capable and merged.
- **Adobe Creative Cloud**: fonts sync with the account and are activated per device. Lesson: per-device activation state exists — we keep the per-item sync switch per item instead, and the per-computer machine pick stays per computer (ADR 0038 decision 6).
- **Obsidian Sync** (from `0096`): selective sync per vault is understood. We do per item, with no password to lose.

What we do better: no account, no password, end-to end encrypted as `0096` is (the server sees only ciphertext), per-item on/off on every row, offline edits converge instead of diverging, a removed device loses the keys and keeps its local copies.

## Words

- **Library item**: one record of one of the three per-user libraries — a Curvyo font (file bytes plus its metadata), a machine profile record, a material record. System fonts, the shipped Included fonts and Generic laser are not library items (criterion 26).
- **Synced item**: an item whose per-item toggle is on. It travels through the device group of `0096` (paired devices, sealed list, ADR 0040) end to end encrypted; the mechanism, rooms and blob transfer are decided in ADR 0062 (accepted).
- **Per-item toggle**: the symbol switch on every font row, machine row and material row that turns sync of that one item on or off.
- Pairing, device group, removal, rotation: as built in `0096`.

## Slicing

| Slice | What the customer can try | Priority |
|---|---|---|
| 1 | Machines and materials: the per-item toggles, records arrive on the other device without input, live and offline edits converge, the conflict rule, switching off, delete, browser device, removal of a device (criteria 1 to 15, 21 to 27). | Must |
| 2 | Fonts including file bytes: the font file arrives in the fonts folder / IndexedDB with its metadata, tags and kept licence files, the size budget (criteria 16 to 20). | Must |

Why this order: machine and material records are small JSON files (under 16 KiB each, `0026` criterion 13) that can ride the built sealed-record path of `0096` unchanged in concept; fonts are blob transfers of up to megabytes that need a transfer budget, content addressing by the face's SHA-256 and the IndexedDB path in the browser — a different mechanism and risk, so it is its own slice. Materials follow the record format of `0112` (ADR 0058); if `0112` is not yet merged when slice 1 starts, machines go first and materials follow within the slice (the lead may cut).

## Acceptance criteria

Setup as `0096`: instances A and B of the desktop app, each with its own data directory and credential store, one `curvyo-sync-server`, and A and B paired as built in `0096` (criteria 1 to 3 there) unless a criterion says otherwise. Unless said otherwise both are online. A holds: one Curvyo font F1 (a TTF, family name `ZQX-FONT-7731`, with a tag `ZQX-TAG-7731`), one user machine profile P1 named `ZQX-MACH-7731` (not Generic laser), one material record M1 named `ZQX-MAT-7731` with one judged test; B holds none of them. The Included fonts and Generic laser exist on both (shipped). A browser instance R (as `0096` slice 1b) is used where a criterion names it. Criteria marked [ADR] depend on the mechanism of ADR 0062 (accepted); where the UI is named, `ux-engineer` fills `ux-notes.md`. Fragen 214 to 217 are answered (all A, 2026-11-13).

### The per-item toggles

1. Given a font row of the Curvyo group in the Fonts tab (`0075`), a machine row in the profile list of the Machine block (`0026` criterion 3) and a material row in the Materials tab (`0112`), then each row carries a symbol toggle switch with the tooltip "Sync to my devices" that is on or off. System fonts, Included fonts and Generic laser have no toggle (criterion 26).
2. Given no paired device (device sync of `0096` never set up), then every library toggle is disabled with the reason that a device has to be paired first (the UX notes own the wording), the app stays fully usable, and nothing about the libraries is sent anywhere (R-SYS-007).
3. Given the first pairing completes, then the device that already holds the device group shows the three libraries with every item's toggle on (Frage 214 A) behind one confirm: "Your fonts, machines and materials leave this computer encrypted, to your own computers only"; the other device receives the on-switched items without any input (criteria 5 and 16).
4. Given an item whose toggle is off, then nothing about it reaches the server: no record and no blob (checked in the server's data directory). A library item created afterwards starts with its toggle per Frage 214 (A: on) on every paired device.

### Machines and materials (slice 1)

5. Given P1's and M1's toggles on and B online, then P1 appears in B's profile list and M1 in B's Materials tab within 30 s on a LAN server (Proposal, as `0096` criterion 10) with the same values, a notice "1 machine and 1 material arrived from <A's name>" (Proposal), and no input on B.
6. Given B is closed or offline when a toggle is turned on, when B starts later and connects, then the item arrives as in criterion 5 without any action on A; A need not be online (the sealed record waits on the server). [ADR]
7. Given P1 synced, when A changes a field of P1 (for example the max feed rate), then B's record takes the change within 2 s (Proposal; the record list has no open-document latency budget), the record is still written as a whole file with a `modified` field (ADR 0038), and it creates no document history step anywhere (`0026` criterion 4 holds). The same holds for a material edit (a set, a judged test) on M1.
8. Given A and B both offline, when A edits P1's max power and B edits P1's origin corner, then on reconnect both records show both changes, with no duplicated record; a same-field conflict is criterion 9. The same holds for M1. [ADR]
9. Given A and B edit the same field of P1 (or M1) differently while both are offline, when both reconnect, then the record with the newer `modified` wins on both devices, the losing version is kept as a new record "<name> (from <other device's name>)", a notice of at most 70 characters names it, and no edit is lost (Frage 216 A). Both devices end with the same two records. [ADR]
10. Given P1 synced on A and B, when the maker turns its toggle off on A, then A stops publishing changes, the item stays on B as a local record with a notice ("Sync off, this copy stays here"), and nothing is deleted on B by the switch (Frage 215 A, as `0096` criterion 13 for workspaces). Turning it on again later merges B's local edits as an offline edit would (criterion 8).
11. Given a synced item, when the maker deletes it on A (font: the armed trash of `0081`; machine and material: their delete), then it is deleted on every paired device with a notice, and on a desktop a font's file goes to the OS trash (Proposal). Given the item's toggle is off, deletion stays local.
12. Given M1 arrives on B, then it is appended at the end of B's material list; the order of materials travels with the library, and two offline reorders converge to the newer one silently (order only, no copies; Proposal).
13. Given a browser device R in the group, then criteria 5 to 12 hold for R unchanged, with machine and material records in browser storage (`curvyo.machines/<id>`, `curvyo.materials/<id>`).
14. Given P1 arrives on B, then B's machine pick (`machine-choice.json`, ADR 0038 decision 6) is unchanged by the arrival: the last pick stays per computer and never leaves it.
15. Given P1's record, then its connection fields (connector address, port, baud rate) travel with the record to the maker's own paired devices only, and no password, key or pairing secret is ever part of a record or a sync blob (`0026` criterion 9 holds).

### Fonts (slice 2)

16. Given F1's toggle on and B or R online, then F1's file bytes arrive — on a desktop into the app data dir fonts folder (listed by the fontdb scan) and in R into IndexedDB `curvyo-fonts` with the same SHA-256 per face (a test compares the hashes) — within 30 s per font on a LAN server (Proposal), with no input on the receiving device, and the row shows in the Curvyo group with star, tags and Cuttable as on A. A collection's file is transferred once, not per face. [ADR]
17. Given F1 synced, then its metadata (star, tags, Cuttable) travels with it and later changes on either device converge; a metadata change both devices made offline takes the newer one with a notice naming the field, no copies (the file bytes are content-addressed by hash and never conflict; Proposal). A tag renamed in the Tags list (`0075`) on A renames on B for every font whose toggle is on and stays local for the rest (Proposal).
18. Given a font installed from a .zip whose licence and readme were kept (`0083`), then those kept files travel with the font (Proposal).
19. Given a font file larger than the per-file budget or a library larger than the total budget (the architect proposes both in ADR 0062 decision 6; Proposal: 16 MiB per font file, 512 MiB per library), then the toggle refuses to switch on with the reason and the numbers, the font stays local and usable, and no partial transfer happens.
20. Given a first sync of 20 Curvyo fonts totalling 40 MiB between A and B on a LAN server, then it completes within 60 s (Proposal) and costs at most 2 s of thread CPU time on the receiving device (Proposal; one test under `curvyo-editor-wasm/tests/cpu_time/`, CLAUDE.md §5; budgets confirmed by the architect in ADR 0062).

### Removal, later devices, what the server sees

21. Given the maker removes device B (Remove in My devices, `0096` criteria 23 and 24), then the key rotation runs as built for the device group and its rooms, B keeps every library item that reached it as a local, usable copy with the notice of `0096` criterion 24, no envelope written afterwards can be opened with B's keys, and B's offline library edits are not merged (`0096` criterion 25). [ADR]
22. Given B chooses "Leave my devices" (`0096` criterion 27), then the same outcome holds for the libraries and the remaining devices rotate without a prompt. [ADR]
23. Given a third device C pairs through B (`0096` criterion 6), then C receives the current state of every on-switched item of all three libraries without input; it does not receive off-switched items or any history beyond the current state. [ADR]
24. Given the server's stored data after criteria 5 to 23 (database files, logs, captured request bodies), then none of these is found: `ZQX-FONT-7731` (the family name), the bytes of F1's file, `ZQX-MACH-7731`, `ZQX-MAT-7731`, `ZQX-TAG-7731`, the material's note text. The server sees, as in `0096` criterion 30, only ciphertext rooms, record and blob sizes and connection times; this exposure is the confirm of criterion 3 and the privacy statement.
25. Given every library record and blob on the server, then it contains no key material; the device keys and epoch keys stay where `0096` criteria 3 and 31 put them (`0096` criterion 31 holds unchanged).
26. Given any device, then Generic laser, the Included fonts and the system fonts never sync: they are built-in or per-computer and have no toggle (criterion 1); nothing about them is written to the server by this feature.
27. Given an off-switched item at any time, then the server's data directory holds no record or blob of it (criterion 4, checked on the server side after every switch-off).

## Out of scope

- System fonts and Included fonts (not per-user items; they are per-computer or shipped with every copy).
- Sharing any of the three libraries with other makers: the workspace stays the only unit of sharing (ADR 0044); a library never goes to a workspace room. Fonts used by a document already travel inside the file (`0107`).
- Machine connector pairing secrets and credentials (`0064`, ADR 0035): the stored connection fields sync (criterion 15), secrets never do.
- Settings sync beyond these three libraries (shortcuts, recent files, the whole rest of `0096`'s deferred list): each is its own story.
- The per-computer machine pick (criterion 14; ADR 0038 decision 6) and any per-device activation state.
- A conflict view beyond the kept copy of criterion 9 (no three-way merge UI).
- The mobile companion, pairing without a server, a hosted instance, quotas and TLS (all `0096` Out of scope unchanged).
- Conflict handling for libraries synced by an outside folder-sync tool (Nextcloud and similar, R-SYS-004's additional option): if a maker syncs the data dir by hand, that is their tool's conflict handling, not ours.
- Import and export of library files as a sync alternative (`0026` Out of scope stays).

## UX notes

In [`ux-notes.md`](ux-notes.md) (ux-engineer, 2026-11-13).

Hints from the product owner, not decisions: the per-item toggle is a small symbol switch at the end of each font, machine and material row, off with the reason when no device is paired; the arrival notice is one line with a count, not a dialog; the confirm of criterion 3 sits where `0096`'s criterion 8 confirm sits.

## Customer decisions

- 2026-11-13 (request, translated): the installed fonts, machines and materials are to travel through the device sync, with a symbol toggle switch on the fonts, the machines and the materials.
- 2026-11-13 (with the merge of PR #354): Fragen 214 to 217 answered with their defaults, all A.
  - 214 TOGGLE-DEFAULT A: the toggles default on — the first sync writes every existing item on behind the one confirm (criterion 3), and a new item starts on (criterion 4); the toggle is there to switch single items off.
  - 215 OFF-REACH A: switching one item off keeps the copies on the other devices as local items with a notice (criterion 10), as `0096` criterion 13 does for workspaces.
  - 216 RECORD-CONFLICT A: on reconnect the newer `modified` wins on both devices and the losing version is kept as "<name> (from <device name>)" with a notice; no edit is lost (criterion 9).
  - 217 LIB-BLOCK A: no "My libraries" block in the Share tab; the sync state stands on every font, machine and material row (`ux-notes.md` section 6).

No customer question is open. The Proposal items of ADR 0062 (the budgets 16 MiB per font file and 512 MiB total, material photos as blobs) were part of the answered Fragen and stand accepted.

## Links

Requirements: R-SYS-004, R-SYS-012, R-FONT-006, R-MFG-001, R-MAT-005 in `docs/requirements.md`.
ADRs: 0038 (machine records, `modified`, machine-choice per computer), 0040 (device group), 0044 (workspace the only unit of sharing), 0047 (document fonts, context), 0055/0058 (material records), and `docs/adr/0062-synced-per-user-libraries.md` (rooms, blob transfer, budgets; accepted 2026-11-13) in `docs/adr/`.
Related specs: `specs/0096-own-devices-sync/` (the device sync this rides on), `specs/0026-machine-profile/`, `specs/0028-material-test-library/` and `specs/0112-test-grid-and-materials/`, `specs/0075-font-library/`, `specs/0107-document-fonts/` (fonts inside documents).
PR: spec #354.

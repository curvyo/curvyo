# Machine profile: describe your laser once, pick it for every laser manufacturing

Status: Ready (2026-11-03; Frage 122/123 open with defaults A/A; Machine block as a host-drawn block kind, adrs.md note 4)
Priority: Must
Origin: Customer (R-MFG-001, MVP slice 13: "define a machine profile per physical device and reuse it across projects"; customer picked this story on 2026-11-03). Everything marked Proposal is the product owner's and vetoable at the demo.

## User value

As a maker with a laser I want to describe my machine once (bed size, origin corner, speed and power limits, how to reach it) and pick it for every laser manufacturing in every project, so that a new project starts with my bed and my limits, and a file I send to someone still shows the bed it was made for.

**One-line meaning (Proposal, to confirm at the demo):** a machine profile is a named description of one physical laser: its bed, its origin corner, its limits and how to reach it. It is not a material, not a job and not a fixture.

**What other tools do.** LightBurn: a "Device" per machine, created by a wizard or imported from a `.lbdev` file, kept per user in LightBurn's settings, with work area, origin corner, controller type, laser mode and connection; the project (`.lbrn2`) stores the chosen device name and bed size only, so a file opened on another computer asks you to pick a device. Inkscape and Ink/Stitch have no machine notion. Fusion 360 keeps machines in a per-user library and copies the used one into the document. We take the Fusion idea: **a per-user library plus a copy of what was used inside the file**, so the file never depends on the library and still opens on a computer without your machines. Where we are better than LightBurn: a file shows the bed it was made for even on a foreign computer, and the pick is one click in the Laser tab, not a dialog.

## Acceptance criteria

1. Given the Laser tab of a laser manufacturing (`0093`), then it has a **Machine** block (inline, no popup) that shows the chosen profile and these fields, with the defaults of a new profile (Proposal):
   - Name (text, 1 to 60 characters; duplicates are allowed, a profile is identified by an id, not by its name).
   - Work area width and height: **400 x 300 mm**, each from 1 to 10,000 mm (the range accepted in `0093`; unchanged).
   - Origin corner: one of four corner buttons, default **bottom-left** (the usual GRBL home).
   - Max feed rate: **6,000 mm/min**, a whole number from 1 to 100,000.
   - Max power value (the controller's S-max): **1,000**, a whole number from 1 to 65,535.
   - Laser mode (dynamic power, GRBL `$32`): switch, default **on**.
   - Connection (criterion 9).
   A value typed outside its range is set to the nearest limit with a notice of at most 70 characters; a fraction typed for feed, power or baud rate is rounded to the nearest whole number (Proposal), with the same notice; nothing is refused (permit over refuse).
2. Given the app, then a built-in profile **Generic laser** exists with exactly the defaults of criterion 1. It cannot be renamed, edited in the library or deleted; **Duplicate** (a symbol button) makes an editable user profile named "Generic laser copy" (numbered as in criterion 3 when that name is taken). There are no other built-in presets (Proposal: no branded machine list, because we cannot test hardware we do not own; presets come as a later slice from what makers ask for).
3. Given the Machine block, when the maker uses **Save as profile**, then a user profile is added to the library with the current values of the block and the manufacturing now points at it. When the name in the block equals the name of a profile already in the library (the built-in included), the new profile is saved as "<name> copy", and if that is taken too as "<name> copy 2", "<name> copy 3", and so on (cut to fit 60 characters, the suffix kept); Duplicate names its result the same way. Typing a name by hand (Name field, rename) may still duplicate another (criterion 1). Given a user profile, then it can be renamed, duplicated and deleted from a profile list in the block (an inline list, one row per profile: name, work area). Deleting uses the armed trash button of `0081` (first press arms, second press after 400 ms deletes); the manufacturings that used it keep their copy (criterion 6) and show "Not in your profiles" (criterion 8). Library changes are not project history: when the maker presses Undo after **Save as profile**, the manufacturing's copy goes back to its previous profile id and values, but the profile that Save as profile added stays in the library (delete it from the list if unwanted).
4. Given a user profile, then the profiles are **kept per user on this machine, one record (one small file) per profile**, in the data directory (desktop) or one browser-storage entry per profile (browser), the record written on every change to that profile and all records read at start. The profile last picked (criterion 10) is remembered on this computer, separately from the records. A profile is not part of any project, a collaborator never sees it, and writing it creates no document history step. There is no network access and no account (`R-SYS-002`, `R-SYS-007`). A profile created, renamed or deleted is still so after the app is closed and started again.
5. Given a laser manufacturing and at least one user profile, when the maker picks a profile from the list, then (a) the manufacturing's copy takes the profile's values, (b) the output page takes the profile's work area and the placed content keeps its place relative to the page's top-left corner (as `0093` criterion 3), and (c) all of it is **one undo step**. Picking the profile already chosen changes nothing and adds no step.
6. Given a manufacturing, then the project file stores a **copy of the chosen profile** (name, origin corner, max feed rate, max power value, laser mode; the work area is the output page's size, never a second number) and the profile's id as a reference, all stored as one atomic value (the id sits inside the copy). The copy, not the library, is what the manufacturing uses: editing or deleting the library profile later does not change any existing manufacturing. The connection data is **not** part of the copy and never leaves the library (so a shared file carries no network address or port name; Proposal, Frage 123).
7. Given a manufacturing whose copy differs from its library profile (the library changed, or the maker edited the copy), then the block shows "Differs from profile" and two symbol buttons: **Update profile** (overwrites the library profile with the copy; not for the built-in) and **Reset from profile** (the copy takes the library values). Update overwrites a library record that has no undo, so it uses the armed press of `0081`: the first press arms, a second press after 400 ms overwrites, and it disarms after 4 s, on Escape or when focus leaves. Reset changes only the manufacturing's copy, is one undo step and asks nothing. The comparison includes the name, so renaming a library profile makes every copy that uses it show "Differs from profile"; this is intended (Reset takes the new name, Update keeps the old one in the library). Editing a field of the Machine block changes the manufacturing's copy only (one undo step per edit; a work area edit moves the output page as in `0093` criterion 3); it never writes the library by itself.
8. Given a file whose manufacturing refers to a profile id the library does not have (another computer, deleted profile), when it is opened, then it opens without error, the Machine block shows the copy's name with "Not in your profiles", every field of the copy is editable, and **Save as profile** adds it to this library under the same id. Nothing in the file is changed by opening it, and no network lookup is tried.
9. Given the connection fields of a user profile (stored data only, **no device I/O**, no test button; the connector is `0064`), then they are: Connector address (text, empty by default, up to 200 characters), Controller port (text such as `/dev/ttyUSB0` or `COM3`, empty by default, up to 200 characters; Proposal) and Baud rate (whole number, default 115200, 1 to 4,000,000; Proposal). They are edited in the Machine block only when the chosen profile is a user profile that exists in the library; for the built-in and for a "Not in your profiles" copy they are shown as unavailable with the reason. No password, key or pairing secret is ever stored in a profile (pairing is `0064`).
10. Given **Add manufacturing: Laser** (`0093`), then the new manufacturing starts with the profile last picked on this computer (remembered per user, not per project), else Generic laser (also when that profile no longer exists). Only a pick made by this maker is remembered, not one arriving from a collaborator. The whole action stays one undo step (`0093` criterion 2) and the output page has that profile's work area.
11. Given the origin corner of the chosen profile, then the Laser canvas draws a small marker at that corner of the output page (view only, not output, not selectable); changing the corner moves the marker at once and moves no content. In this slice the corner and the limits are stored and shown only: nothing clamps, converts or rejects a value because of them (power and speed come with `0027`, `0028`, the gate and G-code with `0029`).
12. Given a project with a manufacturing that has a user profile id, an edited copy and a 600 x 400 mm work area, when it is saved and reopened, then profile id, copy values and work area are identical, and the file passes the golden-file test `machine_profile_v<next free at merge>.curvyo` (document-core). "Differs from profile" is derived on every read, never stored; a separate session test (editor-wasm) with a fixture record in the library checks that the block shows "Differs from profile" for that file and not when the copy equals the record. A copy with a damaged value (non-finite number, a text where a number belongs, a name empty or over 60 characters) reads as the default of that field, is shown, and is never rewritten on read (ADR 0033 §3). A damaged name reads "Generic laser" (default, not truncated), and the block shows the notice "Name was damaged, showing Generic laser" (Proposal, so it is not mistaken for a real pick). A format-35 file (placeholder machine name) opens as Generic laser with that name and writes nothing until the first edit.
13. Given the profile records are damaged in part, when the app starts, then every good record loads and each bad one is skipped: a record that is not valid JSON, has a wrong type or an out-of-range value, an id that differs from its file name, or is over 16 KiB; and a record with a `format_version` newer than the build knows. One notice of at most 70 characters counts the skipped records, and a skipped file is left untouched: never rewritten, renamed or deleted. The rest of the library stays editable; there is no whole-library read-only state. Files whose name is not `<uuid>.json` are ignored.
14. Given 200 profiles in the library, when the Laser tab is opened, then the profile list is usable within 100 ms of thread CPU time and reading all 200 records at start takes within 200 ms of thread CPU time (budgets confirmed by the architect; one test under `curvyo-editor-wasm/tests/cpu_time/` holds them).
15. Given the whole slice, then there is no network access, no G-code, no device I/O and no telemetry; a profile record contains no project content, and a project file contains no library entry the manufacturing does not use.

## Questions for the customer

Frage 122 (PROFILE-HOME): Where do machine profiles live?
- A (recommended, criteria 4 and 6): a per-user library on this machine (one record per profile), plus a copy of the used values in the project file. The file opens correctly anywhere; your library is yours.
- B: only a per-user library; the file stores just the profile name and the bed size (LightBurn's way). Smaller files, but a foreign computer shows no limits.
- C: only inside the project (no library). Nothing to manage, but you retype your machine in every project, which breaks R-MFG-001 "reuse across projects".
Default if unanswered: A. (`0063` already says "global, per user"; the copy in the file is the new part.)

Frage 123 (PROFILE-CONN): Is the connection data (connector address, port) copied into the project file?
- A (recommended, criterion 6): no, it stays in the per-user library; a file you send carries no address of your network.
- B: yes, it travels with the file so a second computer of yours connects at once. Leaks your LAN address into shared files.
Default if unanswered: A.

## Out of scope

- Connecting, a connection test button, device status, framing, sending a job, pairing and encryption: `0064`. Only the fields are stored here.
- Syncing the library between your computers or to the cloud (the per-user file is local; sync is its own slice, `R-SYS-002` stays met without it). Import and export of a profile file (a `.lbdev` import is a Could for later).
- Enforcing or using the limits: clamping power and speed (`0027`, `0028`), the machine and material gate and G-code (`0029`). The origin corner changes no coordinates here.
- Machine families other than laser (cutting plotter, CNC, embroidery) and their fields; the library holds laser profiles only, with a `kind` field so other families can follow (`CLAUDE.md` §6: no machine abstraction before the second family).
- Controller dialects other than GRBL; controller selection (the field is implied).
- Branded presets for known machines, a profile setup wizard, bed auto-detection, rotary and multi-axis settings, air assist, lens or tube data.
- Fixtures moving into the profile (WB-FIXTURE-HOME stays in the manufacturing; moving them is the machine setup stage of `0063`) and a material per profile.
- Profile choice per page or per layer, several machines in one manufacturing.
- Undo of library changes (the library is not project history; delete is protected by the armed button).

## UX notes

See `ux-notes.md` (Machine block layout, inline profile list, the two states, origin-corner control and marker, notices).

## Links

Requirements: R-MFG-001, R-SYS-002, R-SYS-007, R-EXT-003 (the profile is the laser extension's data)
Depends on: `specs/0093-laser-workbench-shell/` (Done, PR #255: placeholder profile, Laser tab, owned output page), `specs/0081-inline-delete-confirm/`, `specs/0045-document-formats-library/` and `specs/0075-font-library/` (the per-user file pattern)
Parent: `specs/0063-manufacturing-workbenches/` (stage 2, setup choice per manufacturing)
Follow-ups: `0027`, `0028`, `0029` (use the limits and the origin), `0064` (reads the connection fields)
ADRs: `adrs.md`; ADR 0036 (profile records, one JSON file per profile) and ADR 0037 (copy in the project file; amends ADR 0033 §1 and §7), both `needs-customer` for Frage 122/123
Format: `format_version` bump, next free at merge (36 today); `mfg_machine_name` is replaced by one atomic `mfg_profile` map (id inside it; ADR 0037)
PR: -

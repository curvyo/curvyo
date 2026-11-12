# ADR 0058: Material records format 2: one material for all lasers with sets per laser, tests by 5-character id with note and one photo file, retired ids, format-1 read path

**Status:** Proposed 2026-11-12, `needs-customer` (persistence: a new record format, a new per-user file and photo files in the data directory). Built on the customer's decisions of 2026-11-12: Frage 191 replaced (judged in Materials by one tap), 192 A (one best cell), 193 B and 203 (a note and one photo of at most 600 px, stored next to the record, from a file or the browser camera, croppable), 194 amended (a set holds power, speed, passes and interval only), a material shared by several lasers with its own sets per laser (also typed by hand), test ids 5 characters `0-9A-Z` unique against every id in the store including deleted tests, double ids from unsynced computers accepted. **Amends** [ADR 0055](0055-material-records.md) §1 to §4 and §8 (record format, reading, writing, goldens) and §9 (the journal also holds photo bytes); its location, order file, envelope, crates and undo rules stay. The grid side is [ADR 0057](0057-test-grid-object-with-derived-operations.md). Spec: [`0112`](../../specs/0112-test-grid-and-materials/specification.md) criteria 16 to 31, 33. Feature notes: [`0112` adrs.md](../../specs/0112-test-grid-and-materials/adrs.md).

## Context

A format-1 record is tied to one profile and keeps its tests as history entries with UUID ids. The redesign makes the record shared by all lasers of the maker, with sets per laser, and makes a test a record of its own kind: a short id burned into the sheet, the recipe, the laser, the state, the best cell, a note and a photo. Ids must never be reused, even after a test or its whole material is deleted. Core stays free of files, clocks, cameras and pixels (`CLAUDE.md` §6).

## Options

**Where the photo lives**

- **A. A separate file next to the record, named by material and test id (chosen).** The record stays small text under its size limit; a note edit never rewrites the image; the folder sync of ADR 0038 carries it.
- **B. Base64 inside the record.** Every edit rewrites up to a megabyte; 200 materials would not fit the browser's `localStorage`. Rejected.

**Where the ids of deleted tests are kept**

- **A. One store-wide file `retired-test-ids.json`, append-only (chosen).** One place to check, also for tests of a deleted material whose record file is gone. Two computers retiring ids offline make a conflict copy that the reader ignores; an id can then be assigned twice, which the customer accepted.
- **B. Tombstones inside each record, and a tombstone record for a deleted material.** No shared file, but deleted materials leave files forever and every list read skips them. Rejected.

**Old records**

- **A. Read format 1 into the format-2 model, write format 2 on the next change (chosen, ADR 0038 rule "reads never write").**
- **B. Rewrite all records at the first load.** A read that writes. Rejected.

## Decision

1. **Files** (amends ADR 0055 §1). Desktop, in `materials/` under the data directory: the record `<uuid>.json`, its photos `<uuid>.<test id>.jpg` or `.png`; next to `materials/`: `retired-test-ids.json`. Browser: records and the retired file in `localStorage` as before (`curvyo.materials/<id>`, `curvyo.retired-test-ids`); photos in IndexedDB, database `curvyo-material-photos`, key = the desktop file name (`localStorage` cannot hold them). The record reader still ignores every name that is not `<uuid>.json`.
2. **Record format 2** (canonical writer as ADR 0055 §2: this key order, two-space indent, trailing newline; read, write, read is the identity):
   ```json
   {
     "format_version": 2,
     "id": "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c",
     "kind": "laser",
     "modified": 1762948800000,
     "name": "Birch plywood",
     "thickness_mm": 3.0,
     "lasers": [
       { "machine": { "id": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f", "name": "Diode 20 W" },
         "sets": {
           "cut": { "power_pct": 60.0, "speed_mm_min": 1950, "passes": 1, "source": { "test": "7K2QX" } },
           "engrave": { "power_pct": 30.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.08,
                        "source": "typed" } } }
     ],
     "tests": [
       { "id": "7K2QX", "created": 1762948800000,
         "machine": { "id": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f", "name": "Diode 20 W" },
         "recipe": { "kind": "cut",
                     "columns": { "param": "power", "min": 20.0, "max": 100.0, "count": 5 },
                     "rows": { "param": "speed", "min": 600, "max": 6000, "count": 5 },
                     "fixed": { "passes": 1 }, "cell_mm": 10.0, "crosshatch": false },
         "lasered": null, "best": { "column": 2, "row": 1 }, "note": "Clean edge at 60 %.",
         "photo": "jpg" }
     ],
     "history": [
       { "id": "5d1e7a2b-0c3f-4e6d-8a9b-7c6d5e4f3a2b", "date": 1762948800000, "event": "set",
         "machine": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f", "kind": "cut",
         "set": { "power_pct": 60.0, "speed_mm_min": 1950, "passes": 1 }, "source": { "test": "7K2QX" } }
     ]
   }
   ```
   - `lasers`: at most 16 entries, one per profile id (a UUID or `generic-laser`), in the order sets were first added; `machine.name` is for display only. `sets` keys `cut`, `score`, `engrave`; numbers as ADR 0054 §1; `interval_mm` (ADR 0056 §5) required on `engrave`, refused on the others. `source` is `{ "test": "<test id>" }` (a test of this record, of that machine and kind), `"typed"`, or `{ "history": "<entry id>" }` (only from a format-1 record, decision 6).
   - `tests`: at most 200, newest last, ids unique in the record. `recipe` is the grid recipe of ADR 0057 §1 without `id` and `material`. `lasered` Unix milliseconds or `null`; `best` absent or a cell inside the recipe's counts; `note` at most 500 characters; `photo` absent, `"jpg"` or `"png"` (the file of decision 1).
   - `history`: append-only as ADR 0055 §2; `event` is `set` (with `set`, `source`) or `removed`; plus `tested` entries read from format 1 (kept, never written new). At most 500.
   - **Judging** (`0112` criterion 24): a tap writes `best` on the test and the laser's set of the recipe's kind from that cell (power, speed, passes; for Engrave the cell's values over the recipe's `fixed`), source the test, and appends a `set` entry; one document step "Judge test" with the ADR 0055 §9 journal. Only the four set numbers are copied, never an operation-only setting (P13).
3. **The test record and the grid** (ADR 0057). A grid's test is written into the picked material at the pick ("Edit test grid" step): id, created, the page's machine, the recipe. While the test is not Lasered every bar change rewrites its recipe in the same step; a pick of another material moves the record (removed from the first, added to the second, its id not retired). After Lasered, a change of the grid makes a new test and leaves the old one. Deleting the grid leaves the test.
4. **Retired ids.** `retired-test-ids.json` = `{ "format_version": 1, "retired": ["4H8ZP", ...] }`, sorted, at most 20,000 ids (Proposal). Deleting a test adds its id; deleting a material adds the ids of all its tests and removes their photos. Append-only: undo of a delete restores the test and leaves its id retired (harmless: the taken set of ADR 0057 §4 is live ids plus retired ids). A damaged file reads as empty with one notice and is never rewritten by a read; the next retire writes the ids it knows. The id set the session passes to `test_id` is: every live test id, every retired id, every grid id of the open project.
5. **Photos at the host boundary.** The frontend takes the file or the camera image (`getUserMedia`, offered only when a camera and permission exist), crops it in a canvas, scales the long side to at most 600 px and encodes JPEG at quality 0.85 (a PNG source stays PNG). It hands the bytes to the session. Core (`curvyo-laser-core/src/material_photo.rs`, "checks a photo's bytes and names its file") checks the JPEG or PNG signature, reads width and height from the PNG `IHDR` or the JPEG `SOF` header (no pixel decoding), refuses a long side above 600 px or more than 1 MiB, and returns the host write `WritePhoto { name, bytes }` or `RemovePhoto { name }` beside the record write. A photo that is missing or does not display shows "Photo not found"; the test stays usable. No image is sent anywhere (R-SYS-002, R-SYS-007).
6. **Reading** (ADR 0055 §3 applied). Format 2 as above; damage as ADR 0055 §3 plus: a duplicate laser entry, a set on the wrong kind, a missing or extra `interval_mm`, a source naming no fitting test, a test id that is not 5 characters of `0-9A-Z` or appears twice, a `best` outside the recipe, a note over 500 characters, a `photo` value other than `jpg` or `png`, more than 200 tests. The record text limit rises from 256 KiB to **512 KiB**, because 200 tests with full notes and 500 history entries in the pretty form pass 256 KiB (the maximal golden must read and fit). **Format 1** reads into the same model: one laser entry from `machine`, its sets with no interval (an Engrave set without interval is allowed only from format 1 and keeps the operation's own interval, ADR 0056 §5), no tests, its history kept (`tested` entries with grid and cell); a set whose source names a `tested` entry reads as `{ "history": "<entry id>" }` and is shown "Test of <date>". It is written as format 2 at the maker's next change of that material, never on read. A record above format 2 is skipped with its notice and never written.
7. **Operation link** (amends ADR 0054 §2 "tied to this profile id"): a linked operation is live when the record loads and has a `lasers` entry with the page copy's profile id that holds the kind's set; otherwise stored, and "Material has no <kind> set for this laser, using stored numbers" (`0112` criterion 21). `MaterialLink` is unchanged.
8. **Undo journal** (amends ADR 0055 §9): an entry may also hold one photo's `before` and `after` bytes; the 16 MiB limit counts them. Order, the retired file and the remembered grid values are not undo steps.
9. **Remembered grid values** (ADR 0055 §6): `test-grid-choice.json` moves to `format_version` 2 holding the new recipe per machine; a format-1 file reads as empty (a preference, nothing lost).
10. **Goldens** in `curvyo-laser-core/tests/fixtures/materials/`: `format2_full.json` (two lasers, typed and tested sets, an Engrave with interval, three tests: one Lasered with best cell, note and photo, one not Lasered, one Engrave with Crosshatch; `set` and `removed` entries; reads, writes back byte for byte), `format2_one_laser.json`, `format2_max.json` (16 lasers, 200 tests, 500 entries, notes of 500 characters: reads and fits 512 KiB), one damaged file per new damage rule of decision 6, `newer_format.json` moves to format 3, the format-1 goldens (`full.json`, `generic_laser.json`, `no_sets.json`, `history_max.json` and the damaged ones) still read, `full.json` reads as `format1_as_format2.json` would write; `retired_ids.json`, `retired_ids_damaged.json`; `photos/7k2qx_600.jpg`, `photos/wide_601.png`, `photos/not_an_image.jpg` (for `material_photo`).

## Consequences

- A material deleted on one computer is gone on all synced computers with its photos; its ids stay retired there once the retired file has synced.
- The data directory gains image files; the browser gains an IndexedDB database. Both stay on the machine (R-SYS-007); a browser that refuses the write keeps the record unchanged with a notice, as ADR 0055.
- A format-2 record is skipped by a build that reads only format 1, with "newer version". Accepted: development build.
- Two computers judging the same test offline produce a conflict copy of the record, as any concurrent record edit (the "Record conflicts" debt in `docs/technical-debt.md`).
- `curvyo-library-core` stays deferred (ADR 0055 §7): still one machine family, no merge.
- Rejected and why: photos inside the record (B, size and rewrite cost), tombstones per record (B, files forever), rewrite on load (B, a read that writes).

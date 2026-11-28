# ADR 0097: Second, third and fourth machine family: cutting plotter, embroidery and CNC crates, shared records in `curvyo-library-core`, the Ink/Stitch port as a GPL-3 crate, device I/O and the build order

**Status:** Proposed 2026-11-27 (architect); its `needs-customer` parts answered by the customer
the same day: Frage 430 A (Curvyo stays AGPL for good, the GPL port is accepted with the relicensing
consequence), 429 A (MF-BRANCH-FILES), 430 A (MF-EMB-READ), 431 A (MF-TOOLS-DIR), MF-REC-KIND default
A. Accepted as a whole with the `story/machine-families` PR. Asked by the customer request of 2026-11-27
(`docs/design-drafts/2026-11-machine-families/plan.md`, Fragen 416 to 427) and the change of the same
day: a thin end-to-end slice through all three families first, then the rest. Specs `0159`
(plotter), `0160` (embroidery), `0161` (CNC). Adds four crates to [ADR 0011](0011-workspace-and-crate-layout.md)
§2; realises the `curvyo-library-core` record slot of ADR 0004 §8 / ADR 0011 §2 that
[ADR 0038](0038-machine-profile-records.md) and [ADR 0055](0055-material-records.md) §7 deferred to
"a second machine family"; amends ADR 0038 decisions 2 and 3 (the `kind` rule) and ADR 0062
decision 10 (the merge's home) by notes. **`needs-customer`** (answered): decision 4 (MF-REC-KIND,
default A), decision 5 (Frage 433 A), decision 6 (Frage 431 A), decision 7 (Fragen 421 C, 430 A;
mechanics **(Proposal)**), decision 9 (Frage 432 A). Everything else is architect scope.
Note 2026-11-28: decision 3 ("no shared operation type") is amended for views only by
[ADR 0098](0098-one-output-page-for-every-machine-family.md) (proposed): the laser's view types serve every
family, dispatched by an enum; domain operation, job and timeline types stay per family, and there is still no trait.

## Context

Curvyo has one machine family, the laser. Its extension crate `curvyo-laser-core` (32 k lines)
also holds what every family needs: the per-user record envelope (`record_envelope`), the record
merge of library sync (`library_merge`, which calls the laser readers directly), and the record
`kind`, which readers accept only as `"laser"`. `CLAUDE.md` §6 forbade a machine abstraction before
the second family; it now arrives with a third and fourth at once. ADR 0042 already made an output
page's kind its manufacturing's process id, so a family is "one more process id, with no new kind
value". `CURRENT_FORMAT_VERSION` is 68 (`curvyo-document-core/src/document.rs`). Machine records are
at format 8, material records at 5. `curvyo-library-core` exists but so far holds only the online
library protocols of ADR 0085.

The questions: where the shared parts go and when, what each family owns, how the GPL-3 Ink/Stitch
port sits in an AGPL workspace, where device I/O and the virtual GRBL go, and the order the
customer's thin-slice-first change requires.

### Options considered

- **Shared records.** **A. Move them into `curvyo-library-core` (chosen):** this is the crate
  ADR 0011 §2 created for "the record model of ADR 0004 §6 ... its `merge`". **B. A new
  `curvyo-records-core`:** a fifth crate for three modules, contradicting ADR 0011. **C. Leave them
  in `curvyo-laser-core`, with the other families depending on it:** a family would depend on
  another family's 32 k lines. A and C both keep the file formats unchanged; B loses to A on crate
  count.
- **Family field.** **A. Keep the stored key `kind` and give it the values `laser`, `cutter`,
  `embroidery` and `cnc` (chosen).** **B. Add a `family` key:** that is a second copy of `kind`, the
  shape ADR 0042 option B rejected for pages.
- **Crate per family against one `curvyo-machines-core`:** the domains share almost no code (knife
  offsets, stitches, end-mill toolpaths), and one crate would gather three responsibilities. One
  crate per family wins (`CLAUDE.md` §5 and §6).
- **Ink/Stitch.** **A. Port it into its own GPL-3.0-or-later crate (customer, Frage 421 C).**
  **B. Port it into `curvyo-embroidery-core`:** GPL and AGPL code would mix in one crate, with no
  visible boundary. **C. Clean-room algorithms:** these lose to Ink/Stitch on quality (the plan's
  biggest risk).
- **Virtual GRBL.** **A. A pure responder and planner in `curvyo-connector-core`, the height map in
  `curvyo-cnc-core` (chosen).** **B. All of it in `curvyo-cnc-core`:** the laser and CutcutGo
  (Frage 416) speak GRBL too, and the connector would then depend on a family crate. **C. All of it
  in `curvyo-connector-io`:** it would be native only, so the browser build could neither preview
  nor time a job.

## Decision

1. **Process ids** (ADR 0042, no new `PageKind` value): `curvyo.cutter`, `curvyo.embroidery` and
   `curvyo.cnc`. Each family crate has a `manifest.rs` like `curvyo-laser-core/src/manifest.rs` with
   its `PROCESS_ID` and page word ("Cutter", "Embroidery", "CNC"; UX may rename them, **(Proposal)**).
   They are registered in `curvyo-ui-core/src/extensions.rs` and can be switched off like the laser.
   `add_output_page` already takes the word (`NewOutputPage`); the history label becomes "Add <word>
   page". Mats, hoops and clamps are fixtures with the existing `<process>/<kind>` fixture strings
   (`curvyo.cutter/mat-12x12`, `curvyo.embroidery/hoop-150x250`). The project copy of the profile
   stays the opaque `mfg_profile` of ADR 0039, typed by the family.

2. **Crates.** Every crate below is pure, `#![forbid(unsafe_code)]`, without a filesystem, clock or
   threads, and builds for wasm32 (decision 10).

   | Crate | Holds | Depends on | Licence |
   |---|---|---|---|
   | `curvyo-cutter-core` | built-in profiles, mats, the Cut, Draw, Score and later tools, drag-knife offset, overcut, the HPGL, GPGL and CutcutGo-GRBL writers, mat-scale SVG and DXF export mapping | document-core, geometry-core, extension-core | AGPL-3.0-or-later |
   | `curvyo-embroidery-core` | the stitch model (stitch, jump, trim, colour change, stop, tie), hoops, thread colours, stitch counts and time, travel and trims, format writers (DST first) and readers | document-core, geometry-core, extension-core, inkstitch-core | AGPL-3.0-or-later |
   | `curvyo-inkstitch-core` | the Ink/Stitch port: running and bean stitch, tatami fill, satin column and underlays first; later contour, guided, meander, circular, linear-gradient fill, ripple, auto-run and auto-satin | `kurbo`, `i_overlay`, `thiserror` only; **no `curvyo-*` crate** | **GPL-3.0-or-later** |
   | `curvyo-cnc-core` | built-in profiles, stock, tools, the Profile, Pocket, Drill and V-carve toolpaths (V-carve on `geometry-core::medial_axis`, ADR 0063), G-code writers per dialect (GRBL, grblHAL, Mach3, Mach4, LinuxCNC; arcs as IJK), the 2.5D height map and its warnings | document-core, geometry-core, extension-core | AGPL-3.0-or-later |

   A family crate never depends on another family crate. `curvyo-editor-wasm` glues families to
   the connector, the records and each other.

3. **Shared and per family.**
   - **Shared already:** the output page, the manufacturing node, placements, fixtures and the
     profile copy (document-core); booleans, offsets, flattening and the medial axis
     (geometry-core); fonts and single-line fonts (text-core); the extension manifest route
     (ADR 0034); the connector protocol and its `Controller` trait (four implementations after
     decision 8: fake, GRBL, HPGL, GPGL).
   - **Shared from step G3** (decision 11): the record envelope, the merge, the `kind` dispatch and the
     `connection` value (connector address, port, baud), in `curvyo-library-core`.
   - **Per family:** the profile body and its built-ins, material or tool bodies, operations,
     the job, stitch or toolpath model, writers, time estimate and preview data.
   - **Not built** (`CLAUDE.md` §5): no `Machine` or `Family` trait, no shared job or timeline type,
     no shared operation type, no generic `Profiles<T>`. Each such step needs a second family that
     needs the same code; a dated note in that spec's `adrs.md` records the move. The likely first
     move is the laser's `job_moves` and `job_timeline` (moves without acceleration), if the
     plotter's playback slice needs exactly that shape. The first move actually taken is the path order (inner paths first, then the nearest start) into
     `curvyo-geometry-core::path_order`, used by all four families (`specs/0159-cutting-plotter/adrs.md` PL4).

4. **Records** (`needs-customer`, MF-REC-KIND default A, built in step G3). `curvyo-library-core` gains
   `record_envelope` (moved unchanged), `record_merge` (moved from `library_merge`) and `record_kind`
   (the `Family` value read from `kind`). The merge takes the outcomes of reading the incoming and
   the local text (`Result<modified, MergeReason>` each) instead of calling a reader. That needs no
   trait and no closure, and `modified` is still read only by the family's own reader (the "never a
   second field" rule of ADR 0062). Laser keeps its record bodies; the laser goldens guard that
   nothing changes. The crate description and the `lib.rs` doc name both module groups.
   - **The envelope is** `format_version`, `id`, `kind`, `modified` and `name`. The work area and
     everything else go in the family body: each family maps its body to the page size (cutter:
     mat or media; embroidery: the hoop field; CNC: X and Y travel).
   - **`kind` first:** after the envelope gate, a reader sorts by `kind`. A record of another known
     family is skipped silently (never a notice, never written). An unknown kind is skipped with the
     "newer" notice. This amends ADR 0038 decision 3 ("a kind other than `laser` is damage").
   - **Versions per kind:** a new kind's first `format_version` is one above the highest version of
     its directory when it is introduced (machines: 9 while laser is at 8). After that each kind
     counts on its own. This was checked against the code: the envelope gates the version before
     the body is read, so every build before this ADR sees a cutter, embroidery or CNC record as
     **Newer** (skipped, never written; in `library_merge` `SkipIncoming` or `LocalNewer`), never as
     Damaged or Older. This amends ADR 0038 decision 2 ("other families add their kind with a new
     `format_version`").
   - **Directories:** machine profiles of every family stay in `machines/`. Cutter materials and
     embroidery fabrics are records with their kind in `materials/`. Library sync needs no list
     change for either (the `machines` and `materials` maps carry any record text).

5. **CNC tools** (`needs-customer`, Frage 433 A, step D1). They go in a new directory,
   `tools/<uuid>.json` (kind `cnc`, browser key `curvyo.tools/<id>`), with `tools.rs` Tauri
   commands that move text only, and a new `tools` map in `curvyo-document-core/src/library_list.rs`.
   The list format goes up when the first tool entry is set, as fonts did (ADR 0040 decision 11), so
   a library without tools stays readable by older builds. Feeds per stock material live in the
   tool record (spec `0161` sets the fields).

6. **Project `format_version`** (`needs-customer` for the branch files, Frage 431 A).
   - **What needs a bump:** an older build would misread a stored shape or produce wrong output.
     A process id alone needs none: an older build lists the page as "Unknown kind", never writes
     its manufacturing (ADR 0033 §4, ADR 0042 §2) and saves it back unchanged. A shape that
     document-core reads for every process (operation nodes, `op_options`, page registers) takes
     the family's bump.
   - **Numbers:** at most one bump per family, taken in build order as N+1, N+2 and N+3 for cutter,
     embroidery and CNC (today 69, 70 and 71). The numbers are fixed at the final PR (the next
     free at merge, as commit `101c47e4` already renumbered once). A family's later slices on this
     branch extend its reserved number and re-cut its goldens. Projects saved by a branch build
     the customer looked at may therefore not open after the merge.

7. **The Ink/Stitch port** (Frage 421 C; mechanics **(Proposal)**).
   - Ink/Stitch is GPL-3.0 (repository licence), and its file headers say "Licensed under the GNU
     GPL version 3.0 or later" (checked in `lib/stitches/fill.py`, 2026-11-27).
   - `curvyo-inkstitch-core/Cargo.toml` sets `license = "GPL-3.0-or-later"` itself, not from the
     workspace. The crate directory holds the GPL-3.0 text as `LICENSE` and a `NOTICE.md`: the
     upstream URL, the pinned upstream commit, and for each ported file its upstream path, the
     upstream copyright line ("Copyright (c) 2010 Authors", "Authors: see git history") and the date
     and nature of the changes (GPLv3 §5a).
   - Each `.rs` file starts with `// SPDX-License-Identifier: GPL-3.0-or-later` and `// Ported from
     Ink/Stitch <path> at <commit>; Copyright (c) 2010 Authors (see git history); changed for Curvyo.`
   - The crate depends on no Curvyo crate: plain geometry (`kurbo`, mm, its own unit newtypes and
     explicit tolerances) goes in, stitch point lists come out. The licence boundary is then one
     crate edge, and the code stays shareable with Ink/Stitch as plain GPL.
   - Code of our own that is not derived from Ink/Stitch stays in `curvyo-embroidery-core`.
   - `deny.toml` gets `[[licenses.exceptions]] name = "curvyo-inkstitch-core", allow =
     ["GPL-3.0-or-later"]`. The allow-list of ADR 0006 §2 is unchanged, so a third-party GPL crate
     still fails CI.
   - **Combining:** GPLv3 §13 and AGPLv3 §13 allow combining the two. The app and wasm bundle are
     conveyed under AGPL-3.0-or-later, and the port's files stay GPL-3.0-or-later. Both licences
     require the source, which Curvyo publishes anyway, so ADR 0006 §1 is unchanged.
   - The app's licence view names Ink/Stitch and pyembroidery.
   - Ink/Stitch fonts are data: each enters the font library with its own licence record
     (ADR 0006 §5, Frage 422 C), never the crate.

8. **Device I/O** (`curvyo-connector-io`, native only; the protocol stays in
   `curvyo-connector-core`).
   - **Job payload:** `Job` gains a `dialect` (`gcode`, `hpgl`, `gpgl`), and `ControllerInfo`
     reports the controller's dialect. The app refuses to send a job of another dialect, and the
     controller refuses it too. This is a protocol change: `PROTOCOL_VERSION` goes from 0 to 1 with
     the plotter's send slice, and the customer's connector updates with the app.
   - **Controller kinds** in `connector.toml` `[controller] kind`:
     - `grbl-serial` and `fake` (existing);
     - `hpgl-serial` (on `serialport`, XON/XOFF or RTS/CTS from the profile; Vevor, USCutter,
       Skycut, Liyu, Roland, and every cutter behind a USB-serial bridge);
     - `gpgl-usb` (Silhouette Cameo 4/5, Portrait and Graphtec, on `nusb` behind a `usb` feature,
       like `serial`; status poll by ESC ENQ);
     - `cricut-usb` only if the capture slice of Frage 416 B succeeds, with its own note; no code
       before that.
   - **Virtual GRBL:** `curvyo-connector-core::virtual_grbl` is a pure responder (bytes in, time in,
     bytes out). It covers GRBL 1.1 and grblHAL: the 128-byte receive buffer, `ok`, `error:N`,
     `ALARM:N`, `?`, `!`, `~`, `0x18`, `$$` and `$H`, the accepted words per dialect (GRBL rejects
     `M6` with `error:20`), a planner with acceleration and junction deviation, and the executed
     segments with times. `curvyo-connector-io/examples/grbl_virtual.rs` serves it on a
     pseudo-terminal, as `examples/grbl_pty.rs` does today (spec `0161` criterion 15). The connector
     opens that path with its **existing** `grbl-serial` controller, so the real controller code is
     under test, and there is no new controller kind or `Controller` implementation for it. The editor runs the same responder in wasm on the
     generated G-code for the preview and the time (no connector needed). The 2.5D height map is
     `curvyo-cnc-core`'s; `editor-wasm` hands it the segments, so no `cnc-core` → `connector-core`
     edge exists. The 3D view is front-end drawing: no 3D dependency is decided here, and one would
     need its own ADR.
   - **GPGL source:** inkscape-silhouette is GPL-2.0 (GitHub's licence field). It is read as
     protocol documentation only, with no translated code (GPL-2.0-only is incompatible with
     AGPL-3.0). Graphtec's published GP-GL reference is the primary source.

9. **Reading embroidery files** (`needs-customer`, Frage 432 A, document model; after the MVP).
   Default: an imported design becomes one path per colour block with a Manual stitch operation
   that keeps every point as a stitch, so there is no new node kind. A stored stitch node would
   need its own document-model ADR.

10. **wasm32 and CI.** CI's `discover-core-crates` picks up the four `*-core` crates from `cargo
    metadata` with no workflow edit; the build, clippy and ban-list legs apply. `deny.toml` gets an
    `AGPL-3.0-or-later` exception for each new AGPL crate, as the 24 existing crates have. Every
    writer has goldens in `<crate>/tests/fixtures/` (HPGL, GPGL, mat SVG, DST, G-code per dialect),
    and the wasm build compares at least one per crate, as `curvyo-laser-core/tests/adjust_goldens.rs`
    does.

11. **Build order** (customer change of 2026-11-27: thin end-to-end slices first).

    | Step | Content | Needs |
    |---|---|---|
    | G1 | The four crate skeletons with their manifests, the `deny.toml` exceptions, the `extensions.rs` entries, notes on ADR 0011 §2 | nothing |
    | G2 | An output page per process: the word and history label, built-in profiles in code only (Cricut Maker 3, Silhouette Cameo 4, Generic HPGL cutter; W6 EU7 with the 150 x 250 and 100 x 100 hoops; Generic GRBL CNC), mats and hoops as fixtures, never written to files (ADR 0038 §4) | G1 |
    | G3 | Records into `curvyo-library-core` (decision 4); machine profile records of kinds `cutter`, `embroidery`, `cnc` (copy and edit as the laser's, with the `connection` value), synced through the existing `machines` map | G1, MF-REC-KIND default A |
    | M1 | Plotter MVP (`0159` slice 1): Cut and Draw, mat-scale Design Space SVG export, HPGL writer, goldens; file output only | G2, G3; next free format at merge if decision 6 applies |
    | M2 | Embroidery MVP (`0160` slice 1): the stitch model, running, tatami and simple satin from the port, the DST writer, goldens, export | G2, G3; next free if decision 6 applies |
    | M3 | CNC MVP (`0161` slice 1): the Profile operation, the GRBL writer, the virtual GRBL (pty example, and in the editor for time and preview), a 2D toolpath preview, Send through the connector's `grbl-serial` to the virtual GRBL | G2, G3; next free if decision 6 applies |
    | D1 | Cutter materials, embroidery fabrics, CNC tools (decision 5) | G3, MF-TOOLS-DIR |
    | D2 | Connector: the `dialect` field and protocol 1, `hpgl-serial`, `gpgl-usb` | G3 (sending needs a profile with a connection) |
    | D3 | Height map and 3D view; embroidery readers (decision 9); the remaining slices of each spec | as each needs |

    All three MVP specs let a profile be copied and edited, and `0161` sends to the virtual GRBL, so
    the record step G3 is MVP groundwork; it builds MF-REC-KIND's default A. All four questions are
    answered (2026-11-27), so no step waits on the customer.

12. **Dependencies** (licences read with `cargo info` on 2026-11-27; Python projects from PyPI and
    the GitHub licence API):

    | Candidate | Licence | Decision |
    |---|---|---|
    | `i_overlay` =9.0.1 (in tree) | MIT OR Apache-2.0 | Offsets and booleans of every family (ADR 0003 §3, §4); line clipping for tatami rows |
    | `spade` =2.15.1 (in tree) | MIT OR Apache-2.0 | V-carve through `geometry-core::medial_axis` (ADR 0063) |
    | `boostvoronoi` 0.12.1 | BSL-1.0 (on the allow-list) | Not added; the ADR 0063 escalation path if the V-carve goldens show the CDT axis is not enough |
    | `cavalier_contours` 0.9.0 | MIT OR Apache-2.0 | Not added; the candidate only if CNC needs offsets that keep arcs for G2/G3 IJK output |
    | `clipper2-rust` 1.3.0 | BSL-1.0 | Rejected: ADR 0003 §3's amendment (customer) replaced it with `i_overlay` |
    | `clipper2` 0.6.1 | MIT OR Apache-2.0 | Rejected: it binds the C++ library through `clipper2c-sys` (ADR 0003 option A) |
    | `petgraph` 0.8.3 | MIT OR Apache-2.0 | Candidate for the port's travel graphs (networkx upstream); added only when the port's own graph code would outgrow one module |
    | `nusb` 0.2.9 | Apache-2.0 OR MIT | Added with `gpgl-usb` (D2), pure Rust |
    | `rusb` 0.9.4 | MIT, binds libusb (LGPL-2.1, C) | Rejected: libusb is off the allow-list and needs a C toolchain |
    | `serialport` (in tree) | MPL-2.0 | `hpgl-serial` |
    | pyembroidery 1.5.1 (reference only) | MIT | Writers are our own; a transliterated function carries the MIT notice in its file and in the crate's notice |
    | Ink/Stitch (ported) | GPL-3.0-or-later | Decision 7 |
    | inkscape-silhouette (reference only) | GPL-2.0 | Protocol documentation only (decision 8) |

## Consequences

- The `CLAUDE.md` §6 rule "no universal machine abstraction before the second family" is met
  without building one: families share data shapes and crates, not traits.
- Six more file shapes get goldens (HPGL, GPGL, DST and later formats, G-code per dialect, records
  per kind, `tools/`).
- `curvyo-laser-core` gets smaller by about 600 lines at G3 and gains a dependency on
  `curvyo-library-core` (already in both builds; its `toml`, `sha2` and `flate2` are locked).
- Builds older than G3 show new-family records as "written by a newer version". The customer's
  devices are updated together.
- The connector protocol bump at D2 refuses older connectors at the handshake.
- The port crate is GPL-3.0-or-later code in an AGPL app. Anyone reusing it alone gets plain GPL.
  Upstream Ink/Stitch fixes are re-ported by hand against the pinned commit in `NOTICE.md`.
- **Relicensing (ADR 0006 §3):** the port is a derivative work whose copyright stays with the
  Ink/Stitch authors. While `curvyo-inkstitch-core` is in the app, Curvyo can no longer be
  relicensed or dual-licensed by its owner alone; that option returns only by removing or replacing
  the crate. Its single crate edge (no `curvyo-*` dependency, one dependent) keeps that removal
  cheap. The customer accepted this (Frage 430 A, 2026-11-27): Curvyo stays AGPL for good.
- One PR at the end (Frage 427 C) makes the final review large. The per-family ADRs (from 0096,
  when a slice needs a decision this one does not cover) keep each family's choices readable.

## Customer answers (2026-11-27)

- **Frage 430 A:** Curvyo stays AGPL-3.0-or-later for good; the GPL-3.0-or-later port is accepted
  with the relicensing consequence (decision 7, Consequences).
- **MF-REC-KIND, default A:** `kind` first, each new kind's versions start above the directory's
  highest (decision 4).
- **Frage 431 A (MF-BRANCH-FILES):** projects saved by a branch build are for looking only and may
  not open after the merge (decision 6).
- **Frage 432 A (MF-EMB-READ):** an imported embroidery design becomes paths per colour with a
  Manual stitch operation; no stitch node kind (decision 9).
- **Frage 433 A (MF-TOOLS-DIR):** CNC tools in `tools/` with a `tools` list map (decision 5).

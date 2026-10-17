# Extensions SDK: write an extension in Rust without TSX, and add tools, processes, simulations and panels

Status: Draft (a sketch for the customer; not Ready: route and licence decided by the customer 2026-10-17; the UI question and the first extension are open)
Priority: Should
Origin: Customer (request of 2026-10-17, translated; extends ADR 0005 and R-EXT-001/002): "It should be possible to write an extension in Rust without TSX. I do not yet know how that works with an unstable ABI. Perhaps the extension can simply be given as a dependency and is compiled in firmly. We must see what is possible and how to continue, whether an SDK is needed or whatever. It need not necessarily be Rust, although that would be easiest. You should be able to add new toolboxes, new manufacturing processes and simulations, add something to the property panel, and also extend existing toolboxes. You could also add a new vectorizer, other drawing tools, etc. The manufacturing parts are to be built as extensions and can be quite elaborate, e.g. a laser cutter with all its settings, importing images and other vectors, maybe extra laser fonts, material tests with results saved in a material library... As the first extension I imagine something simple; I had a good idea just now, but it slipped my mind; I will write when it comes back."

## User value

As a developer (the customer, or a maker who codes) I want to add a feature to Curvyo in Rust, with no knowledge of the TypeScript front end, so that I can build a new tool, a vectorizer or a whole machine process without forking the editor.

As a maker I want those additions to appear in the places I already know (a toolbox card, a Properties tab, the workbench switcher) and to be switched on and off in the app without hand-editing files (`R-EXT-002`). There is no extension marketplace: the aim is that others can extend Curvyo easily without changing the core (customer, 2026-10-17).

**What other tools do.** Inkscape: extensions are separate Python programs fed an SVG on stdin (slow, no UI of their own beyond a generated dialog); the customer's complaint about them is the reason ADR 0005 exists. Blender: Python add-ons in the same process, with full access to everything. VS Code: extensions in a separate host process, with a declared contribution manifest (commands, views, settings) and a stable API. Figma: plugins in a sandbox with a message channel, UI in an iframe. Zed and Lapce: wasm extensions. LightBurn: no public extension model.

## What ADR 0005 already decides, and what has changed

ADR 0005 (accepted 2026-10-02): wasm components behind a versioned WIT interface, a Rust SDK crate (`curvyo-plugin`), the host ships with the first machine family, the interface is `0.x` and unstable until a freeze trigger, plugin edits go through the command journal, no network or filesystem at first. This sketch adds three things the customer has now said: (1) an **ABI-free route** (compile the extension in), (2) **contributions of the application's own kinds** (toolbox, process, simulation, panel, extending existing toolboxes) not only "paths in, paths out", (3) the **manufacturing parts themselves are extensions**. The tracing note in `specs/0025-raster-trace/research.md` section 10 already collects a seam for one case: a pure function with a declared parameter schema that the application renders. That seam is the common core of every route below.

## The routes (compared; the customer's decision is above)

| Route | How an extension is delivered | For | Against |
|---|---|---|---|
| A. Compiled-in crates | A Cargo dependency behind a feature (or a workspace member), built into a Curvyo binary by whoever wants it; registered in one list. | No ABI problem, full Rust API, the fastest, trivially testable, no new runtime. Our own manufacturing parts can be built this way first. | Needs a Rust toolchain; no in-app install; no sandbox; no browser build unless the feature builds for wasm32; every extension is linked into the program, which under AGPL-3.0-or-later (ADR 0006) means it is part of the work and must be AGPL-compatible. |
| B. Wasm components (WIT) | A `.wasm` file the app loads at run time (`wasmtime` on the desktop, the browser's engine in the web build). | Stable interface independent of compiler versions, sandbox by default, same in browser and desktop, language-agnostic. Already accepted in ADR 0005. | The interface must be designed and held stable; large dependency; big data (images, display lists) crosses a boundary; UI contributions need a declarative form (below); slower for heavy simulation unless the host offers help. |
| C. Out-of-process | A separate program (like the machine connector, `0064`) spoken to by a framed protocol. | Any language, a crash cannot take the editor down, a clear licence boundary, OS permissions apply. | Not in the browser; start-up and copy cost; a protocol to keep stable; no sandbox beyond what the OS gives. |
| D. Scripting | An embedded language (Rhai, Lua) running text files. | Lowest barrier, no compile step. | A second API surface; weak for compute; ADR 0005 rejected it as the primary model; sandbox needs care. |

## How an extension adds UI without TSX (the hard part)

The application's front end (TSX) is where toolboxes, Properties tabs and tools are drawn. It is built with **extension points: a registry** for toolbox cards and buttons, Properties tabs and blocks, and tools, which the built-in features also use, so that a new entry is one registration and not an edit of the shell (decided, customer 2026-10-17; see "Decided" below).

An extension never ships TypeScript. It declares **contributions** as data, and the application draws them:

- **Parameters:** a typed schema (name, type, unit, range, default, group) that the application renders as value fields, switches and groups in the places it already has (the top bar of a tool, a Properties tab). This is ADR 0005 item 8 and the seam of `0025`.
- **Contribution points:** a toolbox card or a button added to an existing card; a Properties tab or a block in an existing tab; a workbench (`0063`: toolbox, tabs, simulation); a vectorizer for the Trace tool; a command in the Select bar; a font pack or a material pack (data).
- **Canvas drawing:** the extension returns a **display list** (the shape of render-core's input: paths, colours, transforms), never pixels or HTML; simulations are drawn by the application from it.
- **Edits:** only through the command journal (ADR 0005 item 5), one history step per invocation.
- Where the schema is not enough (a custom widget), the answer is "a new schema element in the application", not "extension TSX" (Question 1).

The customer rules bind extensions too: no popups, symbols with tooltips, no panel section that appears for an operation, permit over refuse.

## Acceptance criteria (as far as clear)

1. Given the SDK, then an extension author writes only Rust and a manifest file; no TypeScript, no JSX, no front-end build (the front end's registry is the application's part, not the author's). A worked example builds with one command.
2. Given an extension, then it declares which contribution points it uses; the application shows them only where it said, and an extension that fails to load leaves the rest of the app working with a quiet entry in the extension list.
3. Given a toolbox, then an extension can add a card of its own and add a tool button to an existing card (the card says which extension added it); two extensions that add to one card are ordered by a rule (installation order, then name), not by chance.
4. Given the Properties panel, then an extension can add a tab or a block from a parameter schema; the layout rules of `docs/design-system.md` are applied by the application.
5. Given a manufacturing process, then an extension can register a workbench with machine settings, roles, a job generator and a simulation (`0063`); the laser workbench of the MVP is built on the same contract.
6. Given a vectorizer, then an extension can offer a method to the Trace tool of `0025` with its parameter schema; the built-in tracer uses the same entry shape.
7. Given an extension that reads a file (import) or writes one (export), then it receives bytes and returns data; the platform crate does the file I/O (`CLAUDE.md` §6).
8. Given security, then an extension has no network and no filesystem unless the maker grants it for that extension, with the ask shown before first use (data leaving the machine, `CLAUDE.md` §3); routes that have no sandbox (A) are marked as trusted code in the list.
9. Given the interface, then it is versioned and marked unstable (`0.x`) with a changelog until the freeze trigger of ADR 0005 item 3.
10. Given the extension list in the app, then every extension (compiled in, as decided) is listed with its name and version, and can be enabled and disabled there (`R-EXT-002`); there is no browse, search or install from the app. Dynamic loading at run time is not required.
11. Given the first extension, then it is chosen by the customer (open placeholder below) and proves the SDK end to end.

## Out of scope

- A marketplace or store for extensions, ratings, payment (the customer does not want a marketplace, 2026-10-17).
- Loading extensions at run time (not required; wasm components stay an option of ADR 0005, unchanged).
- Extensions that change the document model or the file format.
- Native dynamic libraries (`cdylib`): rejected in ADR 0005, unchanged.
- The decision of the interface itself (WIT definitions); that follows the route decision.

## Architect notes (2026-10-17)

- Whatever the route, the contract is the same data: manifest, parameter schema, a pure function over document subsets, a display list, a command batch. Build that contract inside the repository first as an ordinary Rust interface used by first-party code (the laser workbench, the built-in tracer), then put a transport (compiled-in registry, wasm, process) behind it. This follows the rule "no trait without a second implementation": the contract is extracted when the second implementation (the first third-party route) exists.
- Licence: decided below, every extension is AGPL-3.0-or-later, so the question of derivative works across a wasm or process boundary does not arise. State it in a short follow-up of ADR 0006.
- ADR 0005 item 2 ("the host ships in the MVP") is **not moved**. The decision only records that dynamic loading is not required and that the front-end registry is the requirement. Wasm is itself "kind of compiled in" for the customer.

## Decided (customer, 2026-10-17)

- **EXT-ROUTE:** not the proposed route. Extensions may be compiled in as dependencies; they need not be loaded dynamically (wasm is also "kind of compiled in"). The front end (TSX) must be prepared so that things are easy to add: extension points / a registry for toolboxes, panels and tools. ADR 0005 item 2 is not moved.
- **EXT-LICENCE:** extensions are AGPL-3.0-or-later too, open and staying open. No extension marketplace; the aim is only that others can extend the software easily without always changing the core. `R-EXT-002` is reworded accordingly.

## Open questions (customer; each has a default)

Frage 1 (EXT-UI): Is a declarative contribution (schema plus display list) enough for the UI?
- A (default, recommended): yes; new element kinds are added to the application when a real extension needs them.
- B: extensions may also ship a web component for their panels (rich UI, but this is TSX by another name and weakens the design system).
- Default if unanswered: A.

## First extension (open placeholder)

The customer has a first extension in mind ("something simple"), which he will name when it comes back to him. This sketch reserves the place; the extension is chosen, specified and built as the first user of the SDK. Until then, the built-in tracer and the laser workbench are the contract's first users.

## Links

Requirements: R-EXT-001, R-EXT-002 (existing); R-EXT-003 to R-EXT-005 (new)
Depends on: ADR 0005 (extended), ADR 0006 (licence), `specs/0025-raster-trace/research.md` section 10 (the tracing extension seam)
Related: `specs/0063-manufacturing-workbenches/`, `specs/0064-machine-connector/`, `specs/0062-fonts/` (font packs), `specs/0033-stroke-brushes/` (plugin brushes), `docs/design-system.md`
ADRs: a note on ADR 0005 (dynamic loading not required; item 2 unchanged) and a short follow-up on ADR 0006 (extensions are AGPL-3.0-or-later); the front-end registry is covered by the UI-framework ADR or a short ADR when scheduled
PR: -

// Tester cases for `specs/0098-page-kinds` in the front end: the Add page group (criterion 1), the kind
// chip and the row reading (6), no switcher and no Laser tab (7, 13), the Machine block's read-only work
// area (13), the Ruler action of a user profile only (15), the registry keyed by kind with a dummy
// second output kind (18 a), Laser out of the Kind switch when its extension is off (18 b), and the
// session the browser loads (needs `npm run build:wasm`). Rendered on the server through the vite
// harness of `panelStrip.test.ts`.

import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { after, before, test } from "node:test";

import { pageReading } from "../src/lib/pages.ts";
import { type WorkbenchList, kindOptions, registerWorkbench } from "../src/workbenches/registry.ts";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  const { createServer } = await import("vite");
  server = (await createServer({
    configFile: new URL("../vite.config.ts", import.meta.url).pathname,
    server: { middlewareMode: true },
    appType: "custom",
    logLevel: "silent",
  })) as unknown as Server;
});

after(async () => {
  await server.close();
});

const SRC = path.resolve(import.meta.dirname, "..", "src");
const source = (rel: string) => readFileSync(path.join(SRC, rel), "utf8");
const BINDINGS = path.join(SRC, "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };

const KINDS = [
  { id: "drawing", label: "Drawing", icon: "PenTool", workbench: "vector" },
  { id: "sketch", label: "Sketch", icon: "DraftingCompass", workbench: "sketch" },
  { id: "curvyo.laser", label: "Laser", icon: "Zap", workbench: "manufacturing" },
];
const PROFILES = [
  { id: "generic-laser", name: "Generic laser", width: 400, height: 300, builtin: true, chosen: false },
  { id: "u1", name: "Garage", width: 600, height: 400, builtin: false, chosen: true },
];

async function render(module: string, name: string, props: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule(module);
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(loaded[name] as never, props),
  );
  return dom.renderToString(element as never);
}

function byRole(html: string, role: string): string[] {
  return [...html.matchAll(new RegExp(`<[a-z]+ [^>]*role="${role}"[^>]*>`, "g"))].map((m) => m[0]);
}

const noop = () => undefined;

// ---- criterion 1: the Add page group ------------------------------------------------------------

test("AC1: the Add page group is an inline group with a Kind radiogroup Drawing | Sketch | Laser", async () => {
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds: KINDS,
    profiles: PROFILES,
    onAdd: noop,
    onCancel: noop,
  });
  const [group] = byRole(html, "group");
  assert.match(group, /aria-label="Add page"/);
  assert.doesNotMatch(html, /role="dialog"|role="menu"|aria-modal/, "no popup");
  const [kinds] = byRole(html, "radiogroup");
  assert.ok(kinds, "a Kind switch");
  const radios = byRole(html, "radio");
  assert.equal(radios.length, 3);
  const text = html.replace(/<[^>]+>/g, " ");
  // ADR 0098 (UX unify 3): the kinds are symbol buttons, the word is the name and the tooltip.
  for (const word of ["Drawing", "Sketch", "Laser"]) {
    assert.match(html, new RegExp(`aria-label="${word}"`), word);
  }
  for (const word of ["Add", "Cancel"]) {
    assert.match(text, new RegExp(`\\b${word}\\b`), word);
  }
  // Drawing the first time (no stored kind on the server).
  const checked = radios.filter((r) => /aria-checked="true"/.test(r));
  assert.equal(checked.length, 1);
  assert.match(checked[0], /data-kind="drawing"/);
  assert.equal(byRole(html, "listbox").length, 0, "the machine list only for Laser");
});

test("AC1: one Tab stop in the Kind switch (roving)", async () => {
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds: KINDS,
    profiles: PROFILES,
    onAdd: noop,
    onCancel: noop,
  });
  const radios = byRole(html, "radio");
  const stops = radios.filter((r) => !/tabindex="-1"/.test(r));
  assert.equal(stops.length, 1);
});

test("AC1/18b: with only Drawing and Sketch registered, Laser is absent, not disabled", async () => {
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds: KINDS.slice(0, 2),
    profiles: PROFILES,
    onAdd: noop,
    onCancel: noop,
  });
  assert.equal(byRole(html, "radio").length, 2);
  assert.doesNotMatch(html, /Laser/);
  assert.doesNotMatch(html, /aria-disabled="true"|disabled=""/);
});

test("AC1: the kind is remembered on this computer and Escape/Enter are wired", () => {
  const text = source("components/panel/AddPageGroup.tsx");
  assert.match(text, /localStorage\.getItem/);
  assert.match(text, /localStorage\.setItem/);
  assert.match(text, /"Escape"/);
  assert.match(text, /"Enter"/);
  assert.match(text, /"drawing"/, "Drawing is the fallback");
});

// ---- criterion 6: the kind chip -----------------------------------------------------------------

test("AC6: a screen reader reads a row as '<name>, <kind> page'", () => {
  assert.equal(pageReading("Keychain", "Drawing"), "Keychain, drawing page");
  assert.equal(pageReading("Bed", "Laser"), "Bed, laser page");
  assert.equal(pageReading("Old", "Unknown kind"), "Old, unknown kind page");
});

test("AC6: the chip shows a glyph and the word, never colour alone", async () => {
  for (const [icon, word] of [
    ["PenTool", "Drawing"],
    ["DraftingCompass", "Sketch"],
    ["Zap", "Laser"],
    ["Zap", "Laser (off)"],
    ["CircleHelp", "Unknown kind"],
  ]) {
    const html = await render("/src/components/KindChip.tsx", "KindChip", { kind: { icon, word } });
    assert.match(html, /<svg/, `${word}: a glyph`);
    assert.match(html, new RegExp(`>${word.replace(/[()]/g, "\\$&")}<`), `${word}: the word`);
    assert.doesNotMatch(html, /tabindex|<button/, "static text");
  }
});

test("AC6: the glyphs are PenTool, DraftingCompass, Zap and a neutral one, all distinct", async () => {
  const icons = (await server.ssrLoadModule("/src/workbenches/iconNames.ts")) as {
    glyphOf: (name: string) => unknown;
  };
  const glyphs = ["PenTool", "DraftingCompass", "Zap", "CircleHelp"].map((n) => icons.glyphOf(n));
  assert.equal(new Set(glyphs).size, 4);
});

test("AC6: the header chip is on the page-scoped tabs only", () => {
  const text = source("components/panel/PropertiesPanel.tsx");
  const match = /const pageScoped = \[([^\]]+)\]/.exec(text);
  assert.ok(match, "a page-scoped tab list");
  const names = match[1].split(",").map((s) => s.trim().replace(/"/g, ""));
  // Superseded by `0113` and `0029`: the Fixtures tab is page-scoped too (`0029` has no Output tab).
  assert.deepEqual(names.sort(), ["document", "fixtures", "grid", "layers", "materials", "parameters", "sketches", "snapping", "style"]);
});

test("AC6: the chip is in the page row and the canvas label", () => {
  for (const file of ["components/panel/PagesBlock.tsx", "components/PageFrames.tsx"]) {
    assert.match(source(file), /<KindChip/, file);
  }
});

// ---- criterion 7: no switcher ---------------------------------------------------------------------

test("AC7: there is no workbench switcher anywhere in the front end", () => {
  const files: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "wasm-bindings") walk(full);
      } else if (/\.tsx?$/.test(entry.name)) files.push(full);
    }
  };
  walk(SRC);
  for (const file of files) {
    const text = readFileSync(file, "utf8");
    assert.doesNotMatch(text, /WorkbenchSwitcher|set_workbench|setWorkbench\(/, file);
    assert.doesNotMatch(text, /["`>]Add manufacturing/, file);
  }
});

// ---- criterion 13: the Machine block's work area ---------------------------------------------
// ADR 0098 (UX unify 4.1): the page's Machine group is the pick, the "…" and the state row; the
// work area is typed only in a maker's machine in the Machines tab.

test("AC13: the Machine group has no width or height field and no format presets", async () => {
  const g = globalThis as Record<string, unknown>;
  const hadWindow = "window" in g;
  g.window ??= {};
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachineGroup", {
    picker: { family: "laser", id: "generic-laser", name: "Generic laser", state: null, profiles: [] },
    workbenches: { output: {} },
    onOpenMachines: noop,
    onReturnFocus: noop,
  });
  if (!hadWindow) delete g.window;
  const text = html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");
  assert.match(html, /aria-label="Edit machine Generic laser"/);
  assert.doesNotMatch(html, /aria-label="(Width|Height)[^"]*"/, "no width or height fields");
  assert.doesNotMatch(text, /\bA4\b|\bLetter\b/, "no format presets");
});

// ---- criterion 15: Edit work area only on user profiles -------------------------------------

test("AC15: a maker's laser is edited in the Machines tab (rename, delete), Generic laser only duplicated", async () => {
  const fields = (builtin: boolean) => [
    { key: "width", label: "Width", value: builtin ? "400" : "600", unit: "mm", kind: builtin ? "text" : "number" },
    { key: "height", label: "Height", value: builtin ? "300" : "400", unit: "mm", kind: builtin ? "text" : "number" },
  ];
  const row = (id: string, name: string, builtin: boolean) => ({
    id,
    name,
    line2: "",
    builtin,
    fields: fields(builtin),
    connection: null,
  });
  const g = globalThis as Record<string, unknown>;
  const hadWindow = "window" in g;
  g.window ??= {};
  const machines = () => {
    const view = {
      sections: [{ family: "laser", word: "Laser", active: true, notices: [], rows: [row("generic-laser", "Generic laser", true), row("u1", "Garage", false)] }],
      follow: null,
    };
    return render("/src/components/workbench/MachinesTab.tsx", "MachinesTab", {
      workbenches: { machines: { view, sizeApi: () => ({}) } },
      onReturnFocus: noop,
    });
  };
  const html = await machines();
  if (!hadWindow) delete g.window;
  assert.match(html, /aria-label="Garage, "/);
  assert.match(html, /aria-label="Duplicate Generic laser"/);
  assert.match(html, /aria-label="Delete Garage"/);
  assert.doesNotMatch(html, /aria-label="Delete Generic laser"/, "hidden, not disabled");
});

// ---- criterion 18 a: the registry keyed by kind ------------------------------------------------

test("AC18a: a dummy second output kind registered through the same entry point is offered", () => {
  registerWorkbench({ kind: "vector", rail: () => null });
  registerWorkbench({ kind: "sketch", rail: () => null });
  registerWorkbench({ kind: "dummy-output", rail: () => null });
  const list = {
    active: "vector",
    kinds: [
      ...KINDS.slice(0, 2),
      { id: "test.plotter", label: "Plotter", icon: "Box", workbench: "dummy-output" },
      { id: "test.unregistered", label: "Nothing", icon: "Box", workbench: "nobody" },
    ],
    manifests: [],
    extensions: [],
    page: { id: "first", name: "Page 1", kind: "drawing", icon: "PenTool", word: "Drawing", off: false },
  } as unknown as WorkbenchList;
  assert.deepEqual(
    kindOptions(list).map((k) => k.label),
    ["Drawing", "Sketch", "Plotter"],
  );
});

test("AC18a: the shell names no laser process", () => {
  for (const file of [
    "components/panel/PropertiesPanel.tsx",
    "components/panel/PagesBlock.tsx",
    "components/panel/AddPageGroup.tsx",
    "components/PageFrames.tsx",
    "workbenches/registry.ts",
  ]) {
    assert.doesNotMatch(source(file), /curvyo\.laser|=== "laser"|"Laser"/, file);
  }
});

// ---- criterion 22: no network ----------------------------------------------------------------------

test("AC22: the new page-kind components open no network connection", () => {
  for (const file of [
    "components/panel/AddPageGroup.tsx",
    "components/KindChip.tsx",
    "components/workbench/MachinesTab.tsx",
  ]) {
    assert.doesNotMatch(source(file), /fetch\(|WebSocket|XMLHttpRequest|sendBeacon/, file);
  }
});

// ---- the session the browser loads -----------------------------------------------------------------

type Session = {
  workbenches_json: () => string;
  pages_json: () => string;
  add_page_of_kind: (kind: string, profile: string) => string;
  activate_page: (id: string) => boolean;
  activate_adjacent_page: (forward: boolean) => boolean;
  set_extension_enabled: (id: string, enabled: boolean) => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(11n);
}

test("AC2-4/6/8: pages of each kind in the browser session carry kind word and follow activation", skipWasm, async () => {
  const s = await open();
  const sketch = s.add_page_of_kind("sketch", "");
  const laser = s.add_page_of_kind("curvyo.laser", "");
  assert.notEqual(sketch, "");
  assert.notEqual(laser, "");
  const rows = JSON.parse(s.pages_json()) as { id: string; name: string; kindWord: string; active: boolean }[];
  assert.deepEqual(
    rows.map((r) => [r.name, r.kindWord]),
    [
      ["Page 1", "Drawing"],
      ["Sketch 1", "Sketch"],
      ["Laser 1", "Laser"],
    ],
  );
  assert.equal(JSON.parse(s.workbenches_json()).active, "manufacturing");
  assert.ok(s.activate_adjacent_page(false));
  assert.equal(JSON.parse(s.workbenches_json()).active, "sketch");
  assert.ok(s.activate_page("first"));
  assert.equal(JSON.parse(s.workbenches_json()).active, "vector");
});

test("AC18b: the extension off takes Laser out of the kind list and the chip reads 'Laser (off)'", skipWasm, async () => {
  const s = await open();
  const laser = s.add_page_of_kind("curvyo.laser", "");
  s.set_extension_enabled("curvyo.laser", false);
  const list = JSON.parse(s.workbenches_json()) as WorkbenchList;
  assert.deepEqual(
    list.kinds.map((k) => k.label),
    // ADR 0097 §1: the other machine families stay.
    ["Drawing", "Sketch", "Plotter", "Embroidery", "CNC"],
  );
  const row = (JSON.parse(s.pages_json()) as { id: string; kindWord: string }[]).find((r) => r.id === laser);
  assert.equal(row?.kindWord, "Laser (off)");
  assert.equal(s.add_page_of_kind("curvyo.laser", ""), "");
});

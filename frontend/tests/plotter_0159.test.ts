// `specs/0159-cutting-plotter` slice 1 in the front end: the Add page group listing the plotter
// profiles for Plotter, the mat labels reader, and the browser session's plotter page (needs
// `npm run build:wasm`). The page's panel is the shared output page of ADR 0098
// (`machine_families_0098.test.ts`).

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { after, before, test } from "node:test";

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
const BINDINGS = path.join(SRC, "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");
const skipWasm = { skip: !existsSync(WASM) };
const noop = () => undefined;

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

const text = (html: string) => html.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  plotter_profiles_json: () => string;
  set_plotter_mat: (size: string) => boolean;
  set_plotter_mat_grid: (on: boolean) => boolean;
  manufacturing_view_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(159n);
}

test("criteria 1 to 3: a new plotter page is the 12 x 12 in mat, set mat resizes it", skipWasm, async () => {
  const s = await open();
  assert.notEqual(s.add_page_of_kind("curvyo.cutter", "cricut-maker-3"), "");
  const view = JSON.parse(s.manufacturing_view_json()).plotter;
  assert.equal(view.profile.name, "Cricut Maker 3");
  assert.equal(view.mat.size, "12x12");
  assert.equal(view.profile.area, "292.1 × 292.1 mm");
  assert.ok(s.set_plotter_mat("12x24"));
  assert.match(JSON.parse(s.manufacturing_view_json()).status, /292\.1 × 596\.9 mm/);
  const profiles = JSON.parse(s.plotter_profiles_json()) as { name: string }[];
  assert.deepEqual(
    profiles.map((p) => p.name),
    ["Cricut Maker 3", "Generic HPGL cutter"],
  );
});

test("criterion 2: the Add page group lists the plotter profiles for Plotter", async () => {
  const kinds = [
    { id: "drawing", label: "Drawing", icon: "PenTool", workbench: "vector" },
    { id: "curvyo.cutter", label: "Plotter", icon: "Slice", workbench: "manufacturing" },
  ];
  const plotter = [
    { id: "cricut-maker-3", name: "Cricut Maker 3", width: 304.8, height: 304.8, builtin: true, chosen: true },
  ];
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds,
    profiles: [{ id: "generic-laser", name: "Generic laser", width: 400, height: 300, builtin: true, chosen: true }],
    familyProfiles: { "curvyo.cutter": plotter },
    onAdd: noop,
    onCancel: noop,
  });
  // Drawing is the first kind on the server, so no machine list yet.
  assert.doesNotMatch(html, /role="listbox"/);
  assert.match(html, /aria-label="Plotter"/);
});

test("criterion 5: the mat labels reader keeps only well-formed labels", async () => {
  const mod = await server.ssrLoadModule("/src/lib/matLabels.ts");
  const read = mod.readMatLabels as (text: string) => unknown[];
  assert.equal(read('[{"text":"12","x":1,"y":2,"size":9,"turned":true},{"text":1}]').length, 1);
  assert.deepEqual(read("nope"), []);
});

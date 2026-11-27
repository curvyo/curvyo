// `specs/0161-cnc-output` slice 1 in the front end: the CNC panel drawn from the session's view
// (Machine, Stock, Tools, Operations, Output with the play bar; laser rows absent), the Add page
// machine list, the play helper, and the browser session's CNC page (needs `npm run build:wasm`).
// Rendered on the server through the vite harness.

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

const API = {
  pick: () => true,
  saveAs: () => "",
  setConnector: () => "",
  setMachine: () => "",
  setStock: () => "",
  addTool: () => "",
  setTool: () => "",
  removeTool: () => true,
  press: () => "",
  removeSelection: () => true,
  remove: () => true,
  setNumber: () => "",
  play: noop,
  send: noop,
};

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  cnc_view_json: () => string;
  cnc_profiles_json: () => string;
  edit_cnc_stock: (field: string, text: string) => string;
  toggle_toolpath_view: (page: string) => boolean;
  manufacturing_view_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(161n);
}

const props = (view: unknown) => ({
  view,
  api: API,
  onExport: noop,
  onToggleToolpath: noop,
  onSelectItems: noop,
  onReturnFocus: noop,
});

test("criteria 1 to 4: a new CNC page has the Generic GRBL router, its work area, the stock and one tool", skipWasm, async () => {
  const s = await open();
  assert.notEqual(s.add_page_of_kind("curvyo.cnc", "generic-grbl-router"), "");
  const view = JSON.parse(s.cnc_view_json());
  assert.equal(view.profile.name, "Generic GRBL router");
  assert.equal(view.profile.workArea, "300 × 300 mm");
  assert.match(JSON.parse(s.manufacturing_view_json()).status, /300\.0 × 300\.0 mm/);
  assert.equal(view.stock.material, "Plywood");
  assert.equal(view.tools[0].name, "Flat 3.175 mm");
  assert.equal(s.edit_cnc_stock("width_mm", "400"), "Width is 1 to 300 mm.");
  const profiles = JSON.parse(s.cnc_profiles_json()) as { name: string }[];
  assert.deepEqual(profiles.map((p) => p.name), ["Generic GRBL router"]);
});

test("criteria 5, 11, 12: the panel has the two Profile buttons, stock, tools and the output, no laser rows", skipWasm, async () => {
  const s = await open();
  const page = s.add_page_of_kind("curvyo.cnc", "");
  assert.ok(s.toggle_toolpath_view(page));
  const view = JSON.parse(s.cnc_view_json());
  // The panel's Send row reads `isTauri` from the platform module, which looks at `window`.
  const g = globalThis as { window?: unknown };
  g.window = {};
  const html = await render("/src/components/workbench/CncPanel.tsx", "CncPanel", props(view)).finally(() => {
    delete g.window;
  });
  for (const word of ["Machine", "Work area", "Z travel", "Safe height", "Spindle max", "GRBL", "Stock", "Thickness", "Material", "Tools", "Flat 3.175 mm", "Profile outside", "Profile inside", "Export job file", "GRBL G-code", "Nothing runs"]) {
    assert.match(text(html), new RegExp(word), word);
  }
  assert.doesNotMatch(html, /Laser mode|Air assist|Max power|Mat grid|Hoop/, "laser and other family rows absent");
});

test("criterion 11: the play bar shows the time and Z readout", async () => {
  const cncView = await server.ssrLoadModule("/src/lib/cncView.ts");
  const at = cncView.cncPositionAt as (start: number, seconds: number, speed: number, length: number) => number;
  assert.equal(at(10, 2, 4, 100), 18);
  assert.equal(at(10, 100, 16, 100), 100);
});

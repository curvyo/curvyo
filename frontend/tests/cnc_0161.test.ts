// `specs/0161-cnc-output` slice 1 in the front end: the browser session's CNC page, its stock and
// tools, and the Add page machine list (needs `npm run build:wasm`). The page's panel is the shared
// output page of ADR 0098 (`machine_families_0098.test.ts`).

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

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
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

test("criteria 1 to 4: a new CNC page has the Generic GRBL router, its work area, the stock and one tool", skipWasm, async () => {
  const s = await open();
  assert.notEqual(s.add_page_of_kind("curvyo.cnc", "generic-grbl-router"), "");
  const view = JSON.parse(s.manufacturing_view_json()).cnc;
  assert.equal(view.profile.name, "Generic GRBL router");
  assert.equal(view.profile.workArea, "300 × 300 mm");
  assert.match(JSON.parse(s.manufacturing_view_json()).status, /300\.0 × 300\.0 mm/);
  assert.equal(view.stock.material, "Plywood");
  assert.equal(view.tools[0].name, "Flat 3.175 mm");
  assert.equal(s.edit_cnc_stock("width_mm", "400"), "Width is 1 to 300 mm.");
  const profiles = JSON.parse(s.cnc_profiles_json()) as { name: string }[];
  assert.deepEqual(profiles.map((p) => p.name), ["Generic GRBL router"]);
});


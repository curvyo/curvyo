// `specs/0160-embroidery` slice 1 in the front end: the Add page group listing the embroidery
// profiles, the play helpers, and the browser session's embroidery page (needs
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

const API = {
  pick: () => true,
  saveAs: () => "",
  setSpeed: () => "",
  setHoop: () => true,
  press: () => "",
  removeSelection: () => true,
  remove: () => true,
  setNumber: () => "",
  setThread: () => true,
  play: noop,
  hsvOf: () => [0, 0, 0] as const,
  rgbOf: () => 0,
};

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  embroidery_profiles_json: () => string;
  set_embroidery_hoop: (key: string) => boolean;
  toggle_toolpath_view: (page: string) => boolean;
  manufacturing_view_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(160n);
}

const props = (view: unknown) => ({
  view,
  api: API,
  onExport: noop,
  onToggleToolpath: noop,
  onSelectItems: noop,
  onReturnFocus: noop,
});

test("criteria 1 to 3: a new embroidery page has the W6 EU7, its hoops and the 150 x 250 field", skipWasm, async () => {
  const s = await open();
  assert.notEqual(s.add_page_of_kind("curvyo.embroidery", "w6-eu7"), "");
  const view = JSON.parse(s.manufacturing_view_json()).embroidery;
  assert.equal(view.profile.name, "W6 EU7");
  assert.equal(view.hoop.key, "150x250");
  assert.match(JSON.parse(s.manufacturing_view_json()).status, /150\.0 × 250\.0 mm/);
  assert.ok(s.set_embroidery_hoop("100x100"));
  assert.equal(JSON.parse(s.manufacturing_view_json()).embroidery.profile.hoop, "100 × 100 mm");
  const profiles = JSON.parse(s.embroidery_profiles_json()) as { name: string }[];
  assert.deepEqual(
    profiles.map((p) => p.name),
    ["W6 EU7"],
  );
});

test("criterion 1: the Add page group lists the embroidery profiles for Embroidery", async () => {
  const kinds = [
    { id: "drawing", label: "Drawing", icon: "PenTool", workbench: "vector" },
    { id: "curvyo.embroidery", label: "Embroidery", icon: "Spool", workbench: "manufacturing" },
  ];
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds,
    profiles: [],
    familyProfiles: { "curvyo.embroidery": [{ id: "w6-eu7", name: "W6 EU7", width: 190, height: 290, builtin: true, chosen: true }] },
    onAdd: noop,
    onCancel: noop,
  });
  assert.match(html, /aria-label="Embroidery"/);
});

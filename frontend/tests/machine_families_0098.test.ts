// ADR 0098 in the front end: every family page answers the laser's Operations tab, output group
// and tabs; the Machines tab and the Machine group decode and draw; the Kind switch of Add page is
// symbol buttons. The session parts need `npm run build:wasm`.

import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { after, before, test } from "node:test";

import { decodeMachinesView } from "../src/lib/machinesView.ts";
import { decodeOperationsText } from "../src/lib/operationsView.ts";
import { decodeOutputGroupText } from "../src/lib/outputView.ts";

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
  const element = react.createElement(tooltip.TooltipProvider as never, null, react.createElement(loaded[name] as never, props));
  return dom.renderToString(element as never);
}

type Session = {
  add_page_of_kind: (kind: string, profile: string) => string;
  operations_view_json: () => string;
  output_group_json: () => string;
  machines_view_json: () => string;
  manufacturing_view_json: () => string;
};

async function open(): Promise<Session> {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    WasmSession: new (peer: bigint) => Session;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  return new bindings.WasmSession(98n);
}

test("ADR 0098 decision 3: a family page decodes in the laser's Operations and Output views", skipWasm, async () => {
  const s = await open();
  for (const [kind, extension] of [
    ["curvyo.cutter", "svg"],
    ["curvyo.embroidery", "dst"],
    ["curvyo.cnc", "nc"],
  ] as const) {
    assert.notEqual(s.add_page_of_kind(kind, ""), "");
    const operations = decodeOperationsText(s.operations_view_json());
    assert.ok(operations, kind);
    assert.deepEqual(operations.rows, []);
    const output = decodeOutputGroupText(s.output_group_json());
    assert.ok(output, kind);
    assert.equal(output.start, null, "no Start pick on a family page");
    assert.equal(output.export.extension, extension);
  }
});

test("ADR 0098 decision 4: the Machines tab lists every family, the page's first", skipWasm, async () => {
  const s = await open();
  s.add_page_of_kind("curvyo.embroidery", "");
  const view = decodeMachinesView(s.machines_view_json());
  assert.ok(view);
  assert.equal(view.sections[0].family, "embroidery");
  assert.ok(view.sections[0].active);
  assert.deepEqual(new Set(view.sections.map((x) => x.family)), new Set(["laser", "plotter", "embroidery", "cnc"]));
  const picker = JSON.parse(s.manufacturing_view_json()).picker;
  assert.equal(picker.name, "W6 EU7");
  assert.equal(picker.state, null);
});

test("UX unify 4.1: the Machine group has the pick, the … and the state row", async () => {
  const workbenches = { output: { pick: () => true, reset: () => true, saveAs: () => "" } };
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachineGroup", {
    picker: { family: "cnc", id: "x", name: "Router", state: "missing", profiles: [] },
    workbenches,
    onOpenMachines: noop,
    onReturnFocus: noop,
  });
  assert.match(html, /aria-label="Edit machine Router"/);
  assert.match(html, /Not in your machines/);
  assert.match(html, /aria-label="Save as machine"/);
});

test("UX unify 3: the Kind switch is symbol buttons named by the kind word", async () => {
  const kinds = [
    { id: "drawing", label: "Drawing", icon: "pen-tool", workbench: "vector" },
    { id: "curvyo.cnc", label: "CNC", icon: "drill", workbench: "manufacturing" },
  ];
  const html = await render("/src/components/panel/AddPageGroup.tsx", "AddPageGroup", {
    kinds,
    profiles: [],
    familyProfiles: { "curvyo.cnc": [] },
    onAdd: () => "",
    onCancel: noop,
  });
  assert.match(html, /role="radio"[^>]*aria-label="CNC"|aria-label="CNC"[^>]*role="radio"/);
  assert.doesNotMatch(html, /<span class="truncate">CNC<\/span>/);
});

test("UX unify 6 and 8: a family row's unfolded body draws its white fields and strips", async () => {
  const row = { id: { peer: "1", counter: 2 }, name: "Profile 1", thread: null };
  const cnc = await render("/src/components/panel/FamilyBody.tsx", "FamilyBodyView", {
    row,
    body: {
      family: "cnc",
      fields: { side: "outside", direction: "climb", depth_mm: 10, tool: "t1", feed_mm_min: 800, plunge_mm_min: 300 },
      tools: [{ id: "t1", name: "Flat 3.175 mm", line2: "" }],
    },
    onEdit: () => "",
    onReturnToRow: noop,
  });
  assert.match(cnc, /aria-label="Side"/);
  assert.match(cnc, /aria-label="Depth, mm"/);
  const stitch = await render("/src/components/panel/FamilyBody.tsx", "FamilyBodyView", {
    row: { ...row, thread: "#112233" },
    body: { family: "embroidery", kind: "running", fields: { stitch_length_mm: 2.5, repeats: 1, thread: "#112233" } },
    onEdit: () => "",
    onReturnToRow: noop,
  });
  assert.match(stitch, /aria-label="Repeats"/);
  assert.match(stitch, /#112233/);
});

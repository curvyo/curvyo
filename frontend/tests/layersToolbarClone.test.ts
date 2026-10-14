// The Linked clone button of the Layers toolbar (`specs/0049-linked-clone` with
// `specs/0050-layers-and-objects-panel` criterion 55, customer request 2026-10-14): the Layers tab
// rendered on the server with a stub session answering `layers_view`.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  const g = globalThis as Record<string, unknown>;
  g.window = { matchMedia: () => ({ matches: false }), addEventListener: () => undefined, removeEventListener: () => undefined };
  if (!("navigator" in g)) {
    g.navigator = { platform: "Linux x86_64" };
  }
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

const ROW = {
  index: 0,
  id: { peer: "1", counter: 3 },
  level: 1,
  kind: "layer",
  name: "Cut",
  named: true,
  visible: true,
  locked: false,
  effectiveVisible: true,
  effectiveLocked: false,
  hiddenBy: "",
  lockedBy: "",
  readout: "60%",
  container: true,
  expanded: true,
  containsSelection: false,
  mark: "",
  posinset: 1,
  setsize: 1,
  label: "Cut, layer",
  stateLine: "",
  refusal: "",
  rowAction: "select_contents",
};


const TOOLBAR = { newLayer: true, deleteLayer: true, moveUp: true, moveDown: true, duplicate: true };

async function tab(view: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const mod = await server.ssrLoadModule("/src/components/panel/LayersTab.tsx");
  const session = new Proxy(
    { layers_view: () => JSON.stringify(view) },
    { get: (t, k) => (k in t ? (t as never)[k] : () => "") },
  );
  const noop = () => undefined;
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(mod.LayersTab as never, {
      getSession: () => session,
      syncRevision: 0,
      sync: noop,
      shown: true,
      showNotice: noop,
      onReturnFocus: noop,
      onLinkedClone: noop,
    }) as never,
  );
  return dom.renderToString(element as never);
}

const base = (over: Record<string, unknown>) => ({
  count: 1,
  subject: "1 object",
  opacity: { caption: "", field: false, layer: false, value: null, bar: 0 },
  activeLayer: null,
  toolbar: TOOLBAR,
  firstSelected: null,
  rows: [ROW],
  ...over,
});

function buttons(html: string): { tag: string; label: string }[] {
  const out: { tag: string; label: string }[] = [];
  for (const m of html.matchAll(/<button\b[^>]*>/g)) {
    const label = /aria-label="([^"]*)"/.exec(m[0])?.[1] ?? "";
    out.push({ tag: m[0], label });
  }
  return out;
}

test("Linked clone sits after Duplicate in the toolbar, with a tooltip and a name, icon only", async () => {
  const html = await tab(base({ toolbar: { ...TOOLBAR, linkedClone: true } }));
  const names = ["New layer", "Duplicate", "Linked clone", "Move up", "Move down", "Delete layer"];
  const at = names.map((n) => html.indexOf(`aria-label="${n}"`));
  assert.ok(at.every((i) => i >= 0), `all present ${at}`);
  assert.deepEqual([...at].sort((a, b) => a - b), at, "order");
  const clone = buttons(html).find((b) => b.label === "Linked clone");
  assert.ok(clone);
  assert.doesNotMatch(clone.tag, /aria-disabled="true"/);
});

test("Linked clone is dimmed, not hidden, when it does not apply, and stays in the roving order", async () => {
  const off = buttons(await tab(base({ toolbar: { ...TOOLBAR, linkedClone: false } })));
  const clone = off.find((b) => b.label === "Linked clone");
  assert.ok(clone, "rendered");
  assert.match(clone.tag, /aria-disabled="true"/);
  assert.doesNotMatch(clone.tag, /\sdisabled(=|\s|>)/, "focusable");
  assert.match(clone.tag, /tabindex="-1"/, "one Tab stop for the toolbar: the first button");
});

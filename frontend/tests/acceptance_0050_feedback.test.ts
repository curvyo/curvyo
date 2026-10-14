// Tester cases for the customer feedback of 2026-10-14 on `specs/0050-layers-and-objects-panel`
// (criteria 3, 6, 9, 20, 26, 34, 36, 38, 55): the Layers tab rendered on the server with a stub
// session that answers `layers_view` with the JSON the spec describes. Written from the spec.

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

const TOOLBAR_ON = { newLayer: true, deleteLayer: true, moveUp: true, moveDown: true, duplicate: true };
const TOOLBAR_OFF = { newLayer: true, deleteLayer: false, moveUp: false, moveDown: false, duplicate: false };

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
    }) as never,
  );
  return dom.renderToString(element as never);
}

const base = (over: Record<string, unknown>) => ({
  count: 1,
  subject: "1 object",
  opacity: { caption: "", field: false, layer: false, value: null, bar: 0 },
  activeLayer: null,
  toolbar: TOOLBAR_OFF,
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

test("the toolbar has the five buttons in order, as role=toolbar named Layers (criterion 55)", async () => {
  const html = await tab(base({ toolbar: TOOLBAR_ON }));
  assert.match(html, /role="toolbar"[^>]*aria-label="Layers"|aria-label="Layers"[^>]*role="toolbar"/);
  const names = ["New layer", "Duplicate", "Move up", "Move down", "Delete layer"];
  const at = names.map((n) => html.indexOf(`aria-label="${n}"`));
  assert.ok(at.every((i) => i >= 0), `all present ${at}`);
  assert.deepEqual([...at].sort((a, b) => a - b), at, "order");
  assert.ok(html.indexOf('role="toolbar"') < html.indexOf('role="treegrid"'), "toolbar above the list");
  // no text label: the visible text of the toolbar has none of the names
  const bar = html.slice(html.indexOf('role="toolbar"'), html.indexOf('role="treegrid"'));
  const text = bar.replace(/^[^>]*>/, "").replace(/<[^>]*(>|$)/g, "");
  assert.equal(text.trim(), "", `icon-only: ${text}`);
});

test("inapplicable buttons are aria-disabled, still rendered and still buttons, never hidden (criterion 55)", async () => {
  const off = buttons(await tab(base({ toolbar: TOOLBAR_OFF })));
  for (const n of ["New layer", "Duplicate", "Move up", "Move down", "Delete layer"]) {
    const b = off.find((x) => x.label === n);
    assert.ok(b, `${n} is rendered`);
    const disabled = /aria-disabled="true"/.test(b.tag);
    assert.equal(disabled, n !== "New layer", `${n} disabled=${disabled}`);
    assert.doesNotMatch(b.tag, /\sdisabled(=|\s|>)/, `${n} stays focusable (not the disabled attribute)`);
  }
  const on = buttons(await tab(base({ toolbar: TOOLBAR_ON })));
  for (const b of on.filter((x) => x.label !== "")) {
    assert.doesNotMatch(b.tag, /aria-disabled="true"/, b.label);
  }
});

test("each toolbar button has a text tooltip (criterion 55)", async () => {
  const html = await tab(base({ toolbar: TOOLBAR_ON }));
  for (const n of ["New layer", "Duplicate", "Move up", "Move down", "Delete layer"]) {
    assert.ok(html.includes(n), n);
  }
});

test("no hint, no footer, no panel section for an operation (criteria 3, 26)", async () => {
  const html = await tab(base({}));
  assert.doesNotMatch(html, /Select an object or a layer/);
  assert.doesNotMatch(html, /<footer/);
  assert.doesNotMatch(html, /role="contentinfo"/);
  const empty = await tab(base({ count: 0, rows: [], subject: "0 objects" }));
  assert.doesNotMatch(empty, /Select an object or a layer/);
  assert.match(empty, /No objects yet\./);
  assert.match(empty, /Draw something or add a layer\./);
  assert.match(empty, /role="toolbar"/, "toolbar stays on an empty document (criterion 9)");
});

test("the row controls run opacity, select contents, eye, lock after the name (criteria 6, 38)", async () => {
  const html = await tab(base({}));
  const row = html.slice(html.indexOf('role="row"'));
  const idx = [
    row.indexOf(">Cut<"),
    row.indexOf("60%"),
    row.indexOf('aria-label="Select contents"'),
    row.search(/aria-label="(Hide|Show)[^"]*"/),
    row.search(/aria-label="(Lock|Unlock)[^"]*"/),
  ];
  assert.ok(idx.every((i) => i >= 0), `all found ${idx}`);
  assert.deepEqual([...idx].sort((a, b) => a - b), idx, `order ${idx}`);
});

test("the opacity slot is after the list with its caption, and keeps its place when the field is absent (criterion 3, 26)", async () => {
  const withField = await tab(
    base({ opacity: { caption: "Layer Cut", field: true, layer: true, value: 60, bar: 0.6 }, activeLayer: ROW.id }),
  );
  assert.ok(withField.indexOf("Layer Cut") > withField.indexOf('role="treegrid"'), "below the list");
  assert.match(withField, /Opacity/);
  const without = await tab(base({}));
  assert.ok(without.includes("role=\"status\""));
  const locked = await tab(base({ opacity: { caption: "Locked", field: false, layer: false, value: null, bar: 0 } }));
  assert.ok(locked.includes("Locked"));
});

test("the lock cursor hint maps to a cursor and nothing else does (criterion 20)", async () => {
  const { cursorForHint } = (await server.ssrLoadModule("/src/lib/cursors.ts")) as {
    cursorForHint: (h: string) => string | undefined;
  };
  assert.ok(cursorForHint("locked"));
  for (const h of ["default", "move", "rotate", ""]) {
    assert.notEqual(cursorForHint(h), cursorForHint("locked"), h);
  }
});

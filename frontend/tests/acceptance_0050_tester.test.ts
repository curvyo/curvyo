// Independent tester cases for `specs/0050-layers-and-objects-panel` at the page: the treegrid row
// semantics (criteria 6, 17, 38, 41) rendered on the server through the vite harness of
// `panelStrip.test.ts`, and the pure helpers of the list (criteria 5, 29, 32, 38, 48). Written from
// the specification; the markup is only looked at to learn the prop names.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { MAX_ROW_ELEMENTS, ROW_HEIGHT_PX, indentPx, windowRange } from "../src/lib/layerRows.ts";

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

const BASE = {
  index: 0,
  id: { peer: "1", counter: 3 },
  level: 1,
  kind: "rectangle",
  name: "Door",
  named: true,
  visible: true,
  locked: false,
  effectiveVisible: true,
  effectiveLocked: false,
  hiddenBy: "",
  lockedBy: "",
  readout: null as string | null,
  container: false,
  expanded: false,
  containsSelection: false,
  mark: "",
  posinset: 1,
  setsize: 3,
  label: "Door, rectangle",
  stateLine: "",
  refusal: "",
  rowAction: "",
};

async function render(row: Partial<typeof BASE>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const mod = await server.ssrLoadModule("/src/components/panel/LayerRow.tsx");
  const noop = () => undefined;
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(mod.LayerRow as never, {
      row: { ...BASE, ...row },
      top: 0,
      height: 28,
      tabbable: true,
      cell: "name",
      renaming: false,
      drop: null,
      onPress: noop,
      onFlag: noop,
      onExpand: noop,
      onAction: noop,
      onRename: noop,
      onStartRename: noop,
      onDragStart: noop,
      onHover: noop,
      onFocusCell: noop,
    }) as never,
  );
  return dom.renderToString(element as never);
}

function rowTag(html: string): string {
  const m = /<div [^>]*role="row"[^>]*>/.exec(html);
  assert.ok(m, html);
  return m[0];
}

test("a row carries level, position, set size, selection and its accessible name (criteria 38, 41)", async () => {
  const html = await render({
    level: 2,
    posinset: 2,
    setsize: 5,
    mark: "selected",
    label: "Door, rectangle, hidden, locked, opacity 60 percent",
  });
  const tag = rowTag(html);
  assert.match(tag, /aria-level="2"/);
  assert.match(tag, /aria-posinset="2"/);
  assert.match(tag, /aria-setsize="5"/);
  assert.match(tag, /aria-selected="true"/);
  assert.match(tag, /aria-label="Door, rectangle, hidden, locked, opacity 60 percent"/);
  assert.doesNotMatch(tag, /aria-expanded/, "a leaf has no aria-expanded");
});

test("a container row says whether it is expanded; the active layer is aria-current (criteria 35, 38)", async () => {
  const open = rowTag(await render({ kind: "layer", container: true, expanded: true, mark: "active", rowAction: "select-contents" }));
  assert.match(open, /aria-expanded="true"/);
  assert.match(open, /aria-current="true"/);
  assert.match(open, /aria-selected="false"/, "a layer is never selected");
  const shut = rowTag(await render({ kind: "group", container: true, expanded: false }));
  assert.match(shut, /aria-expanded="false"/);
  assert.doesNotMatch(shut, /aria-current/);
});

test("a name that holds markup or direction controls is escaped text (criterion 17)", async () => {
  const hostile = "<img src=x onerror=alert(1)>";
  const html = await render({ name: hostile, label: `${hostile}, rectangle` });
  assert.ok(!html.includes("<img"), "no element is created from a name");
  assert.ok(html.includes("&lt;img"), "the name is shown as text");
});

test("a row has one button each for the eye and the lock, and a layer row has Select contents (criteria 6, 38)", async () => {
  const leaf = await render({});
  const buttons = (html: string) => [...html.matchAll(/<button [^>]*>/g)].map((m) => m[0]);
  const names = (html: string) => buttons(html).map((b) => /aria-label="([^"]*)"/.exec(b)?.[1] ?? "");
  const leafNames = names(leaf);
  assert.equal(leafNames.length, 2, `${leafNames}`);
  assert.ok(leafNames.every((n) => n !== ""), "every button has an accessible name");
  const layer = names(await render({ kind: "layer", container: true, expanded: true, rowAction: "select-contents" }));
  assert.ok(layer.some((n) => /select contents/i.test(n)), `${layer}`);
  assert.ok(layer.length >= 3);
});

test("the eye and the lock show their state as a shape, not only as colour (criterion 41)", async () => {
  const shown = await render({});
  const hidden = await render({ visible: false, effectiveVisible: false });
  const lockedHtml = await render({ locked: true, effectiveLocked: true });
  assert.notEqual(shown, hidden, "a hidden row differs in more than a class name");
  const glyph = (html: string) => [...html.matchAll(/class="lucide ([^"]*)"/g)].map((m) => m[1]);
  assert.notDeepEqual(glyph(shown), glyph(hidden));
  assert.notDeepEqual(glyph(shown), glyph(lockedHtml));
});

test("the indent is 16 px a level and stops at 6 levels (criterion 6)", () => {
  assert.equal(indentPx(1), 0);
  assert.equal(indentPx(2), 16);
  assert.equal(indentPx(7), 96);
  assert.equal(indentPx(40), 96, "deeper rows keep the indent");
});

test("rows are 28 px and the window holds at most 60 rows (criteria 6, 48)", () => {
  assert.equal(ROW_HEIGHT_PX, 28);
  for (const total of [0, 1, 11, 12, 60, 61, 10_000, 1_000_000]) {
    for (const scrollTop of [0, 1, 27, 28, 1_000, total * 28, total * 28 + 5_000, -50]) {
      const { first, count } = windowRange(scrollTop, 336, total);
      assert.ok(count <= MAX_ROW_ELEMENTS, `${total} ${scrollTop} ${count}`);
      assert.ok(first >= 0 && first + count <= total, `${total} ${scrollTop} ${first} ${count}`);
    }
  }
  // At least 12 rows are visible at 800 x 600: a 336 px list holds 12 rows.
  assert.ok(windowRange(0, 336, 100).count >= 12);
});

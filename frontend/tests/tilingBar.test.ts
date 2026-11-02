// The tiling bar's Apply button (`specs/0069-tiling` UX notes §2.1): `aria-disabled` while the
// session refuses the count and while any field holds an invalid text. Rendered on the server
// through the vite harness; the invalid-field wiring is checked in the source.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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

const triple = [0, 0, 0];
const recipe = {
  layout: "grid", columns: 5, rows: 3, spacing_x: 5, spacing_y: 5, pitch: false, mirror_rows: false,
  mirror_columns: false, offset: "none", shift: 50, count: 6, radius: 20, start: 0, sweep: 360,
  centre_x: 0, centre_y: 0, face_centre: false, rotation: triple, scale: triple, offset_x: triple,
  offset_y: triple, opacity: triple, hue: triple, seed: 1, copies: false, keep_source: true,
};

async function barHtml(level: string, layout = "grid", imagesCopies = false): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const bar = await server.ssrLoadModule("/src/components/TilingBar.tsx");
  const tiling = {
    open: true, slot: "edit", recipe: { ...recipe, layout },
    status: { count: 15, level, images_copies: imagesCopies, refusal: "", reopened: false },
    entry: null,
    firstFieldRef: { current: null },
  };
  return dom.renderToString(react.createElement(bar.TilingBar as never, { tiling } as never) as never);
}

async function applyButton(level: string): Promise<string> {
  const html = await barHtml(level);
  const match = /<button[^>]*>Apply<\/button>/.exec(html);
  assert.ok(match, "Apply is rendered");
  return match[0];
}

test("Apply is enabled for a valid recipe and disabled when the count is refused", async () => {
  assert.match(await applyButton("fine"), /aria-disabled="false"/);
  assert.match(await applyButton("refused"), /aria-disabled="true"/);
});

test("Apply is also disabled while any field holds an invalid text", () => {
  const source = readFileSync(new URL("../src/components/TilingBar.tsx", import.meta.url), "utf8");
  assert.match(source, /report\(id, error !== null\)/);
  assert.match(source, /applyBlocked = refused \|\| invalidIds\.size > 0/);
  assert.match(source, /aria-disabled=\{applyBlocked\}/);
  assert.match(source, /!applyBlocked && tiling\.apply\(\)/);
});

test("UX notes §12.2: no Cancel, seed or count text; Apply last; Grid and Circle as named icons", async () => {
  for (const layout of ["grid", "circle"]) {
    const html = await barHtml("fine", layout);
    assert.doesNotMatch(html, />Cancel</);
    assert.doesNotMatch(html, /Seed|New random seed/);
    assert.doesNotMatch(html, /instances?</, "no count text");
    assert.match(html, /role="radio"[^>]*aria-label="Grid"/);
    assert.match(html, /role="radio"[^>]*aria-label="Circle"/);
    const buttons = [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map((m) => m[1]);
    assert.equal(buttons.at(-1), "Apply", "Apply comes last in the DOM");
  }
  const circle = await barHtml("fine", "circle");
  assert.match(circle, /aria-label="Count"/);
  assert.match(circle, /Face centre/);
  for (const gone of ["Radius, millimetres", "Start angle, degrees", "Sweep, degrees", "Centre X, millimetres", "Centre Y, millimetres"]) {
    assert.doesNotMatch(circle, new RegExp(`aria-label="${gone}"`), `${gone} left the bar`);
  }
});

test("UX notes §12.3: the notice slot shows only while a notice is active", async () => {
  assert.doesNotMatch(await barHtml("fine"), /role="status"[^>]*aria-label/);
  assert.match(await barHtml("simplified"), /role="status"[^>]*aria-label="Over 10,000 instances\. The preview is simplified\."/);
  assert.match(await barHtml("fine", "grid", true), /aria-label="Images become copies\."/);
  const refused = await barHtml("refused");
  assert.match(refused, /role="alert"[^>]*>Too many instances\. 100,000 at most\.</, "the refusal stays readable");
});

test("UX notes §12.5 (2026-11-02): the Variance blocks sit side by side and wrap on a narrower bar", () => {
  const source = readFileSync(new URL("../src/components/TilingBar.tsx", import.meta.url), "utf8");
  assert.match(source, /@min-\[500px\]:flex-row @min-\[500px\]:flex-wrap/);
});

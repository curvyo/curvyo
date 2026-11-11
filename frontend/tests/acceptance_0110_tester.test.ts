// Tester acceptance for `specs/0110-pen-modes-variable-stroke` criterion 1, rendered on the
// server through the vite harness: one icon button per mode with an accessible name and a
// tooltip, no text labels, the active mode checked, Bézier by default, Smoothing for Freehand
// alone. Criterion 7 (no bottom bar): the App source no longer mounts a status bar.

import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
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

async function render(mode: string | null): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const bar = await server.ssrLoadModule("/src/components/PenModeBar.tsx");
  const session =
    mode === null
      ? null
      : {
          pen_mode: () => mode,
          pen_smoothing: () => 0.4,
          pen_mode_refusal: () => "",
          set_pen_mode: () => true,
          set_pen_smoothing: () => true,
        };
  const editor = { getSession: () => session, containerRef: { current: null } };
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(bar.PenModeBar as never, { editor }) as never,
  );
  return dom.renderToString(element as never);
}

function radios(html: string): { label: string; checked: boolean; text: string }[] {
  return [...html.matchAll(/<button([^>]*role="radio"[^>]*)>([\s\S]*?)<\/button>/g)].map((m) => ({
    label: /aria-label="([^"]*)"/.exec(m[1])?.[1] ?? "",
    checked: /aria-checked="true"/.test(m[1]),
    text: m[2].replace(/<[^>]*>/g, "").trim(),
  }));
}

test("c1 one icon button per mode with an accessible name and no text label", async () => {
  const items = radios(await render("spiro"));
  assert.deepEqual(
    items.map((i) => i.label),
    ["Bézier", "Spiro", "B-spline", "Lines", "Freehand"],
  );
  for (const item of items) {
    assert.equal(item.text, "", `${item.label} shows no text`);
  }
  assert.deepEqual(
    items.filter((i) => i.checked).map((i) => i.label),
    ["Spiro"],
    "the active mode is highlighted",
  );
});

test("c1 every mode has a tooltip", async () => {
  const { PEN_MODES } = (await server.ssrLoadModule("/src/lib/penModes.ts")) as {
    PEN_MODES: { label: string; tooltip: string }[];
  };
  assert.equal(PEN_MODES.length, 5);
  for (const mode of PEN_MODES) {
    assert.ok(mode.tooltip.length > mode.label.length, mode.label);
  }
});

test("c1 Bézier is the default when the session has not said otherwise", async () => {
  const items = radios(await render(null));
  assert.deepEqual(
    items.filter((i) => i.checked).map((i) => i.label),
    ["Bézier"],
  );
});

test("c1 Smoothing is shown for Freehand alone", async () => {
  for (const mode of ["bezier", "spiro", "bspline", "lines", "freehand"]) {
    const html = await render(mode);
    assert.equal(html.includes("Freehand smoothing"), mode === "freehand", mode);
  }
});

test("c7 the bottom status bar is gone", () => {
  const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(app, /<StatusBar\b/);
  assert.equal(existsSync(new URL("../src/components/StatusBar.tsx", import.meta.url)), false);
});

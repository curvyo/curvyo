// The Grid & Rulers rail entry and pane (`specs/0090-grid` criteria 2, 3, 10, 11 and 18, UX notes
// §1 to §3 and the amendment), rendered on the server through the same vite harness as `panelStrip.test.ts`.

import assert from "node:assert/strict";
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

const TABS = [
  { name: "document", label: "Document", tooltip: "Document (Shift+Ctrl+D)", enabled: true },
  { name: "snapping", label: "Snapping", tooltip: "Snapping", enabled: true },
  { name: "grid", label: "Grid & Rulers", tooltip: "Grid & Rulers", enabled: true },
];

const noop = () => undefined;

async function render(child: (react: typeof import("react")) => unknown): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const tabs = await server.ssrLoadModule("/src/components/ui/tabs.tsx");
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(
      tabs.Tabs as never,
      { value: "document", orientation: "vertical" },
      child(react) as never,
    ) as never,
  );
  return dom.renderToString(element as never);
}

async function strip(gridShown: boolean): Promise<string> {
  const panelStrip = await server.ssrLoadModule("/src/components/panel/PanelStrip.tsx");
  return render((react) =>
    react.createElement(panelStrip.PanelStrip as never, {
      tabs: TABS,
      active: "document",
      open: true,
      bodyId: "body",
      fileDirty: false,
      snappingEnabled: false,
      onToggleSnapping: () => false,
      gridShown,
      onToggleGrid: () => !gridShown,
      onToggle: noop,
      onActivate: noop,
      onReturnFocus: noop,
    }),
  );
}

function tab(html: string, label: string): string {
  const match = new RegExp(`<button [^>]*role="tab"[^>]*aria-label="${label}"[^>]*>`).exec(html)
    ?? new RegExp(`<button [^>]*aria-label="${label}"[^>]*role="tab"[^>]*>`).exec(html);
  assert.ok(match, `${label} tab`);
  return match[0];
}

test("the Grid & Rulers entry follows Snapping and says whether the grid is shown", async () => {
  const hidden = await strip(false);
  const name = "Grid and rulers";
  assert.ok(hidden.indexOf('aria-label="Snapping"') < hidden.indexOf(`aria-label="${name}"`));
  assert.match(tab(hidden, name), /aria-description="Hidden"/);
  assert.doesNotMatch(tab(hidden, name), /editor-accent-hover\)] !text/);
  const shown = await strip(true);
  assert.match(tab(shown, name), /aria-description="Shown"/);
  assert.match(tab(shown, name), /bg-\[var\(--editor-accent-hover\)\]/);
  assert.match(shown, /role="status"/);
});

const VIEW = {
  pageName: "Coasters",
  shown: false,
  mode: "adaptive",
  scale: "linear",
  spacing: "5",
  subdivision: "5",
  angle: "0",
  origin: "top_left",
  unit: "mm",
};

async function pane(view = VIEW): Promise<string> {
  const section = await server.ssrLoadModule("/src/components/panel/GridSection.tsx");
  const grid = {
    view,
    toggle: () => false,
    setShown: () => false,
    setMode: () => false,
    setScale: () => false,
    setField: () => "unchanged",
    setOrigin: () => false,
  };
  return render((react) =>
    react.createElement(section.GridSection as never, { grid, disabled: false, onReturnFocus: noop }),
  );
}

test("the Grid pane names the page and holds every control, all enabled with the grid hidden", async () => {
  const html = await pane();
  assert.match(html, /Coasters/);
  assert.match(html, /role="switch"[^>]*aria-label="Grid"|aria-label="Grid"[^>]*role="switch"/);
  for (const group of ["Mode", "Scale"]) {
    assert.match(html, new RegExp(`role="radiogroup" aria-label="${group}"`));
  }
  for (const option of ["Adaptive", "Fixed", "Linear", "Logarithmic"]) {
    assert.match(html, new RegExp(`role="radio"[^>]*>${option}<`));
  }
  for (const label of ["Spacing", "Subdivision", "Angle"]) {
    assert.match(html, new RegExp(`aria-label="${label}"`));
  }
  assert.doesNotMatch(html, /Origin X|Origin Y|Reset to page corner/);
  assert.doesNotMatch(html, /<fieldset disabled/);
});

test("the checked mode and the chosen corner are the one Tab stop of their groups", async () => {
  const html = await pane();
  assert.match(html, /aria-checked="true" tabindex="0"[^>]*>Adaptive</);
  assert.match(html, /aria-checked="false" tabindex="-1"[^>]*>Fixed</);
  const moved = await pane({ ...VIEW, origin: "bottom_left", mode: "fixed" });
  assert.match(moved, /aria-checked="true" tabindex="0"[^>]*>Fixed</);
  assert.match(moved, /aria-label="Bottom left" aria-checked="true" tabindex="0"/);
  assert.match(moved, /aria-label="Top left" aria-checked="false" tabindex="-1"/);
});

test("the origin corner is a radio group of four corners under the page name", async () => {
  const html = await pane();
  const group = html.indexOf('role="radiogroup" aria-label="Origin corner"');
  assert.ok(group > html.indexOf("Coasters"), "under the page name");
  assert.ok(group < html.indexOf('role="switch"'), "before the Grid switch");
  for (const corner of ["Top left", "Top right", "Bottom left", "Bottom right"]) {
    assert.match(html, new RegExp(`role="radio" aria-label="${corner}"`));
  }
  assert.match(html, /aria-label="Top left" aria-checked="true" tabindex="0"/);
});

test("the notices are at most 70 characters", async () => {
  const hook = await server.ssrLoadModule("/src/hooks/useGrid.ts");
  const messages = hook.GRID_MESSAGES as Record<string, string>;
  assert.deepEqual(Object.keys(messages).sort(), ["number", "spacing_number", "spacing_positive", "subdivision"]);
  for (const text of Object.values(messages)) {
    assert.ok(text.length <= 70, text);
  }
});

test("the Snapping pane has a Grid target and the Grid snaps with choice", async () => {
  const section = await server.ssrLoadModule("/src/components/panel/SnappingSection.tsx");
  const snapping = {
    view: {
      enabled: false, suspended: false, hasDrag: false, objects: true, guides: true, nodes: true,
      page: true, grid: true, gridPoint: "centre", x: true, y: true, capturePx: 8, releasePx: 14,
    },
    toggle: () => false, setTargets: () => false, setAxes: () => false, setZones: () => false,
    setGrid: () => false, setGridPoint: () => false,
  };
  const html = await render((react) =>
    react.createElement(section.SnappingSection as never, { snapping, onReturnFocus: noop }),
  );
  assert.match(html, /aria-pressed="true"[^>]*>Grid</);
  assert.match(html, /role="radiogroup" aria-label="Grid snaps with"/);
  assert.match(html, /aria-checked="true"[^>]*>Centre</);
  for (const option of ["Box corners and edges", "Nodes"]) {
    assert.match(html, new RegExp(`aria-checked="false"[^>]*>${option}<`));
  }
  // The five targets in order: Objects, Guides, Nodes, Page, Grid.
  const order = ["Objects", "Guides", "Nodes</button>", "Page", ">Grid<"].map((t) => html.indexOf(t));
  assert.deepEqual([...order].sort((a, b) => a - b), order);
});

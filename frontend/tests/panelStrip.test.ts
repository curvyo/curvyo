// The strip's semantics (`specs/0051-sidebar-and-no-menu/` criteria 2, 5, 6, 7): the tablist is
// vertical and named "Panel", exactly one tab is selected while the body is shown and none while it
// is hidden, the collapse control says what it does and the dimmed Style tab is disabled. Rendered
// on the server through the same vite harness as `tooltipProvider.test.ts`.

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
  { name: "style", label: "Style", tooltip: "Style: select an object first", enabled: false },
  { name: "history", label: "History", tooltip: "History (Shift+Ctrl+H)", enabled: true },
];

async function strip(open: boolean): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const tabs = await server.ssrLoadModule("/src/components/ui/tabs.tsx");
  const panelStrip = await server.ssrLoadModule("/src/components/panel/PanelStrip.tsx");
  const noop = () => undefined;
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(
      tabs.Tabs as never,
      { value: "document", orientation: "vertical" },
      react.createElement(panelStrip.PanelStrip as never, {
        tabs: TABS,
        active: "document",
        open,
        bodyId: "body",
        onToggle: noop,
        onActivate: noop,
        onReturnFocus: noop,
      }),
    ) as never,
  );
  return dom.renderToString(element as never);
}

/** The attributes of every element with the given role. */
function byRole(html: string, role: string): string[] {
  return [...html.matchAll(new RegExp(`<[a-z]+ [^>]*role="${role}"[^>]*>`, "g"))].map((m) => m[0]);
}

test("the tablist is vertical and named Panel; each tab is a tab with an accessible name", async () => {
  const html = await strip(true);
  const [list] = byRole(html, "tablist");
  assert.match(list, /aria-orientation="vertical"/);
  assert.match(list, /aria-label="Panel"/);
  const tabs = byRole(html, "tab");
  assert.equal(tabs.length, 3);
  assert.deepEqual(
    tabs.map((tab) => /aria-label="([^"]+)"/.exec(tab)?.[1]),
    ["Document", "Style", "History"],
  );
  for (const tab of tabs) {
    assert.match(tab, /aria-controls="/);
  }
});

test("with the body shown exactly the active tab is selected", async () => {
  const tabs = byRole(await strip(true), "tab");
  assert.deepEqual(
    tabs.map((tab) => /aria-selected="(true|false)"/.exec(tab)?.[1]),
    ["true", "false", "false"],
  );
});

test("with the body hidden no tab is selected", async () => {
  const tabs = byRole(await strip(false), "tab");
  assert.deepEqual(
    tabs.map((tab) => /aria-selected="(true|false)"/.exec(tab)?.[1]),
    ["false", "false", "false"],
  );
});

test("the dimmed Style tab is disabled and says so", async () => {
  const [, style] = byRole(await strip(true), "tab");
  assert.match(style, /disabled=""/);
  assert.match(style, /aria-disabled="true"/);
});

test("the collapse control names what it does and says whether the body is shown", async () => {
  const shown = await strip(true);
  assert.match(shown, /aria-label="Hide panel"[^>]*aria-expanded="true"|aria-expanded="true"[^>]*aria-label="Hide panel"/);
  assert.match(shown, /aria-controls="body"/);
  const hidden = await strip(false);
  assert.match(hidden, /aria-label="Show panel"[^>]*aria-expanded="false"|aria-expanded="false"[^>]*aria-label="Show panel"/);
});

// The File tab (`specs/0051-sidebar-and-no-menu/` criteria 15, 16, 24, 25) rendered on the server
// through the vite harness: the project block, the buttons of each build, the version line and the
// inline check that replaces the six buttons.

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

const noop = () => undefined;

interface Overrides {
  desktop?: boolean;
  fileName?: string | null;
  path?: string | null;
  dirty?: boolean;
  check?: { id: number; kind: string; origin: string; previous: null } | null;
}

async function render(overrides: Overrides = {}): Promise<string> {
  const { desktop = true, fileName = null, path = null, dirty = false, check = null } = overrides;
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const fileTab = await server.ssrLoadModule("/src/components/panel/FileTab.tsx");
  const project = {
    info: { fileName, path },
    dirty,
    check,
    press: noop,
    saveFromCheck: noop,
    discard: noop,
    cancel: noop,
  };
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(fileTab.FileTab as never, {
      project,
      desktop,
      version: "0.4.0",
      tooltipFor: (command: string) => `tip ${command}`,
      onReturnFocus: noop,
    }) as never,
  );
  return dom.renderToString(element as never);
}

/** The visible text of every button, in order. */
function buttons(html: string): string[] {
  return [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map((m) => m[1]);
}

test("an untitled clean project: name, status, six buttons, Quit, version", async () => {
  const html = await render();
  assert.match(html, />Untitled</);
  assert.match(html, />Not saved yet</);
  assert.deepEqual(buttons(html), ["New", "Open…", "Save", "Save as…", "Close", "Template", "Import SVG…", "Quit"]);
  assert.match(html, />Curvyo 0.4.0</);
});

test("a saved project shows its file name and whether it changed", async () => {
  const clean = await render({ fileName: "Poster.curvyo", path: "/home/m/Poster.curvyo" });
  assert.match(clean, />Poster\.curvyo</);
  assert.match(clean, />No changes since the last save</);
  const dirty = await render({ fileName: "Poster.curvyo", path: "/home/m/Poster.curvyo", dirty: true });
  assert.match(dirty, />Unsaved changes</);
});

test("the browser build has no Save as, Quit or divider", async () => {
  const html = await render({ desktop: false });
  assert.deepEqual(buttons(html), ["New", "Open…", "Save", "Close", "Template", "Import SVG…"]);
  assert.doesNotMatch(html, /Quit/);
  assert.doesNotMatch(html, /Save as/);
});

test("the buttons name the command they run and carry no text of their own for the key", async () => {
  const html = await render();
  for (const command of ["new", "open", "save", "save-as", "close", "quit"]) {
    assert.match(html, new RegExp(`data-file-command="${command}"`));
  }
  assert.doesNotMatch(html, /Ctrl\+/);
});

test("the File tab holds no popup, menu or dialog (criterion 24)", async () => {
  for (const html of [await render(), await render({ check: { id: 1, kind: "open", origin: "outside", previous: null }, dirty: true })]) {
    assert.doesNotMatch(html, /role="(dialog|alertdialog|menu|listbox|menuitem)"/);
    assert.doesNotMatch(html, /aria-haspopup/);
  }
});

test("the check replaces the six buttons in place with a sentence and three buttons", async () => {
  const html = await render({
    fileName: "Poster.curvyo",
    path: "/home/m/Poster.curvyo",
    dirty: true,
    check: { id: 1, kind: "open", origin: "outside", previous: null },
  });
  assert.deepEqual(buttons(html), ["Cancel", "Discard", "Save", "Import SVG…", "Quit"]);
  assert.match(
    html,
    /role="group" aria-label="Poster\.curvyo has unsaved changes\. Save them before you open another project\? Discarded changes cannot be brought back\."/,
  );
  // Discard starts out not armed: it ignores presses for the first 400 ms.
  assert.match(html, /aria-disabled="true"[^>]*>Discard</);
});

test("an untitled project says This project has", async () => {
  const html = await render({
    dirty: true,
    check: { id: 2, kind: "quit", origin: "outside", previous: null },
  });
  assert.match(html, /This project has unsaved changes\. Save them before you quit\?/);
});

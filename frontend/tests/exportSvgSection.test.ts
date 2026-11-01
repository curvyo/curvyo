// Export SVG… in the File tab (`specs/0024-svg-import-export` criteria 4 and 147, `ux-notes.md`
// "SVG export: UX notes") rendered on the server through the vite harness: the button, its
// tooltip, the four switches with their exact labels, all off the first time, in one group.

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

async function render(): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const fileTab = await server.ssrLoadModule("/src/components/panel/FileTab.tsx");
  const section = await server.ssrLoadModule("/src/components/panel/ExportSvgSection.tsx");
  const project = {
    info: { fileName: null, path: null },
    dirty: false,
    check: null,
    press: noop,
    saveFromCheck: noop,
    discard: noop,
    cancel: noop,
    exportSvg: noop,
  };
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(fileTab.FileTab as never, {
      project,
      desktop: true,
      version: "0.4.0",
      tooltipFor: (command: string) => `tip ${command}`,
      onReturnFocus: noop,
      exportSvg: react.createElement(section.ExportSvgSection as never, { project, onReturnFocus: noop }),
    }) as never,
  );
  return dom.renderToString(element as never);
}

test("c4 c147 Export SVG… sits below the block with four switches, all off", async () => {
  const html = await render();
  const labels = [...html.matchAll(/<button[^>]*>(?:<span[^>]*>(?:<span[^>]*><\/span>)?<\/span>)?([^<]*)<\/button>/g)].map(
    (m) => m[1],
  );
  assert.deepEqual(labels, [
    "New",
    "Open…",
    "Save",
    "Save as…",
    "Close",
    "Template",
    "Export SVG…",
    "Text as paths",
    "Strokes as paths",
    "Selection only",
    "Active page only",
    "Quit",
  ]);
  assert.match(html, /role="group" aria-label="Export options"/);
  assert.equal([...html.matchAll(/role="switch" aria-checked="false"/g)].length, 4);
  assert.doesNotMatch(html, /aria-checked="true"/);
});

// `specs/0131` UX notes §2 and §4: the offer row's texts follow the display unit of the Work area
// line, and the offer row is absent from the browser build (no host, no row).

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { machineReportsText, workAreaStateText } from "../src/lib/workAreaText.ts";

test("the offer text uses the unit of the Work area line", () => {
  assert.equal(machineReportsText(400, 850, "mm"), "Machine reports 400 × 850 mm");
  assert.equal(machineReportsText(400, 850, "cm"), "Machine reports 40 × 85 cm");
  assert.equal(workAreaStateText(400, 850, "mm"), "Work area 400 × 850 mm");
});

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

test("the browser build has no offer row and no place (criterion 5)", async () => {
  const g = globalThis as Record<string, unknown>;
  g.window ??= {};
  const react = await import("react");
  const dom = await import("react-dom/server");
  const loaded = await server.ssrLoadModule("/src/components/workbench/MachineSizeOffer.tsx");
  const html = dom.renderToString(
    react.createElement(loaded.MachineSizeOffer as never, {
      profileId: "u1",
      address: "127.0.0.1:7878",
      unit: "mm",
      api: { sizeReported: () => true, sizeOffer: () => [400, 850], useSize: () => true },
      width: 400,
      height: 300,
      onFocusProfile: () => {},
      onUsed: () => {},
    }) as never,
  );
  assert.equal(html, "");
});

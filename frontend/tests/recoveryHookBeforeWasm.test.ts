// The recovery hook (`specs/0136-autosave-and-restore/`) mounted before the wasm module is up must
// not touch wasm in its first render: the schedule is made once a session exists. A first render
// that threw left every fresh load of the app a blank page.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

type Server = {
  ssrLoadModule(url: string): Promise<Record<string, unknown>>;
  close(): Promise<void>;
};
let server: Server;

before(async () => {
  (globalThis as { window?: unknown }).window ??= globalThis;
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

test("the first render before wasm init does not throw", async () => {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const mod = await server.ssrLoadModule("/src/hooks/useRecovery.ts");
  const useRecovery = mod.useRecovery as (deps: unknown) => { rows: unknown[] };
  const deps = {
    editor: { getSession: () => null },
    project: { info: { path: null } },
    panelOpen: { setOpen: () => undefined },
    panel: { pressTab: () => undefined },
  };
  function Probe() {
    const api = useRecovery(deps);
    return react.createElement("span", null, String(api.rows.length));
  }
  const html = dom.renderToString(react.createElement(Probe));
  assert.match(html, /<span>0<\/span>/);
});

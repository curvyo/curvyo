// The badge of a drop that places a whole page live (`specs/0154-library-into-slots` slice 2,
// UX notes V5).
import assert from "node:assert/strict";
import { after, before, test } from "node:test";

type Server = { ssrLoadModule(url: string): Promise<Record<string, unknown>>; close(): Promise<void> };
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

type Placed = { placedBadge: (words: string) => string };

test("a slot badge says a live placed page and keeps the scale words", async () => {
  const mod = (await server.ssrLoadModule("/src/lib/intoSlot.ts")) as unknown as Placed;
  assert.equal(mod.placedBadge("Into Slot 2 \u00b7 50\u00a0%"), "Placed page into Slot 2 \u00b7 50\u00a0%");
});

test("refusals and the fill badge are left alone", async () => {
  const mod = (await server.ssrLoadModule("/src/lib/intoSlot.ts")) as unknown as Placed;
  assert.equal(mod.placedBadge("Slot 2 is filled"), "Slot 2 is filled");
  assert.equal(mod.placedBadge("Fill 3 of 3 slots"), "Fill 3 of 3 slots");
});

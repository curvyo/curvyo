// The armed Replace of a filled slot keeps its source alive (`specs/0154-library-into-slots`
// criterion 7, architect review): the reader is freed only after the Replace has put the copy,
// or when the notice goes unpressed, never while the Replace can still be pressed.

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

type IntoSlot = {
  putIntoSlot: (ctx: unknown, source: unknown, slot: string, real: boolean, replace?: boolean, release?: () => void) => boolean;
  setSlotReplace: (next: unknown) => void;
  currentSlotReplace: () => { replace: () => void } | null;
};

const load = async () => (await server.ssrLoadModule("/src/lib/intoSlot.ts")) as unknown as IntoSlot;

const source = { reader: {}, from: { documentName: "d", pageName: "p", pageId: "first" }, whole: true, roots: [] };

/** A session whose slot is filled until a put with `replace`; it records the puts. */
function fakeSession(puts: boolean[], freed: () => boolean) {
  return {
    put_into_slot: (_r: unknown, _roots: string, _p: unknown, _c: string, _s: string, _real: boolean, replace: boolean) => {
      assert.equal(freed(), false, "the reader is alive while the put reads it");
      puts.push(replace);
      return JSON.stringify(replace ? { refused: null, notice: null } : { refused: "Slot 1 is filled.", notice: null });
    },
  };
}

test("the Replace of a filled slot puts with a live reader and frees it once afterwards", async () => {
  const mod = await load();
  let freed = 0;
  const puts: boolean[] = [];
  const session = fakeSession(puts, () => freed > 0);
  const ctx = { getSession: () => session, sync: () => {}, showNotice: () => {} };
  assert.equal(mod.putIntoSlot(ctx, source, "s1", false, false, () => (freed += 1)), false);
  assert.deepEqual(puts, [false]);
  assert.equal(freed, 0, "the refused put keeps the reader for the Replace");
  const entry = mod.currentSlotReplace();
  assert.ok(entry, "the armed Replace is shown");
  entry.replace();
  assert.deepEqual(puts, [false, true], "the Replace puts with replace set");
  assert.equal(freed, 1, "freed once, after the Replace's put");
  mod.setSlotReplace(null);
  assert.equal(freed, 1, "clearing afterwards frees nothing twice");
});

test("a Replace notice that goes unpressed frees the reader", async () => {
  const mod = await load();
  let freed = 0;
  const session = fakeSession([], () => freed > 0);
  const ctx = { getSession: () => session, sync: () => {}, showNotice: () => {} };
  mod.putIntoSlot(ctx, source, "s1", false, false, () => (freed += 1));
  assert.equal(freed, 0);
  mod.setSlotReplace(null);
  assert.equal(freed, 1, "the notice going frees the reader");
});

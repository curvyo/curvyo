// Enter on a slot row of the Into slot list puts into that slot and can never run "Fill slots"
// (`specs/0154-library-into-slots` criterion 10, UX review 2026-11-26).
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

type Row = { kind: "fill" | "slot"; id: string };
type Keys = {
  intoSlotRows: (targets: unknown) => Row[];
  intoSlotKey: (key: string, repeat: boolean, focus: number, count: number) => { take?: number; move?: number } | null;
};

const slot = (id: string, filled: boolean) => ({ id, name: id, shape: "rect", size: "30 × 30 mm", filled, outcome: null });
const targets = {
  holders: [
    { id: "h1", name: "Holder 1", hidden: false, filled: 1, of: 3, slots: [slot("a", true), slot("b", false), slot("c", false)] },
    { id: "h2", name: "Holder 2", hidden: false, filled: 0, of: 1, slots: [slot("d", false)] },
  ],
  selected: null,
};

test("Enter on every slot row takes that slot row, never a Fill row", async () => {
  const keys = (await server.ssrLoadModule("/src/lib/intoSlotKeys.ts")) as unknown as Keys;
  const rows = keys.intoSlotRows(targets);
  assert.deepEqual(
    rows.map((r) => `${r.kind}:${r.id}`),
    ["fill:h1", "slot:a", "slot:b", "slot:c", "fill:h2", "slot:d"],
  );
  rows.forEach((row, focus) => {
    if (row.kind !== "slot") {
      return;
    }
    const action = keys.intoSlotKey("Enter", false, focus, rows.length);
    assert.ok(action && action.take !== undefined, "Enter takes");
    const taken = rows[action.take ?? -1];
    assert.equal(taken?.kind, "slot", `Enter on ${row.id} takes a slot row`);
    assert.equal(taken?.id, row.id);
  });
});

test("a held Enter, the one that opened the list, takes nothing", async () => {
  const keys = (await server.ssrLoadModule("/src/lib/intoSlotKeys.ts")) as unknown as Keys;
  assert.equal(keys.intoSlotKey("Enter", true, 0, 6), null);
  assert.equal(keys.intoSlotKey(" ", true, 0, 6), null);
  assert.deepEqual(keys.intoSlotKey("ArrowDown", false, 5, 6), { move: 5 });
});

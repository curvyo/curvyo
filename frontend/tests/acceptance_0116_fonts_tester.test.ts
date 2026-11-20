// Tester acceptance cases for `specs/0116-library-sync` slice 2's frontend (fonts: criteria 1,
// 2, 16, 17, 18 and 19 as they touch the browser and the rows), written from the specification,
// the UX notes and ADR 0062 before the font rows and the browser font host were read. The Rust
// halves (the journeys, the budgets, the server scans) are
// `curvyo-editor-wasm/tests/acceptance_0116_fonts_tester.rs` and
// `curvyo-sync-core/tests/acceptance_0116_fonts_tester.rs`. Seams not reachable here (reported):
// the IndexedDB write of a synced font without a fake IndexedDB in the dev dependencies, and the
// SSR of the family row (its props need a full Curvyo font listing).

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { familyKey } from "../src/lib/fontRows.ts";
import type { Row } from "../src/lib/fontRows.ts";
import { onFontTreeKey, type FontTreeKeyContext } from "../src/lib/fontTreeKeys.ts";
import { familySyncOn, travellingDocuments, type FontFileInfo, type FontToggleRow } from "../src/lib/fontSyncModel.ts";
import type { FontFamily } from "../src/lib/textText.ts";

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

/** A Curvyo family as the Fonts tab lists it: one file, one face (criterion 1). */
const ZQX = { block: "curvyo", family: "ZQX-FONT-7731", faces: [] } as unknown as FontFamily;

/** The file of the family (its faces share the host id). */
const FILE: FontFileInfo = { id: "ZQX-FONT-7731.ttf", faces: 1 } as unknown as FontFileInfo;

// ---- criterion 1 and 4: the family toggle reads on only when every file is synced ----------------

test("c1/c4: a family reads on when its file's entry is on, off when it has no entry or an off one", () => {
  const hashes = new Map<string, string>([[FILE.id, "sha-zqx"]]);
  const on: FontToggleRow[] = [{ id: "sha-zqx", sync: true }];
  const off: FontToggleRow[] = [{ id: "sha-zqx", sync: false }];
  assert.equal(familySyncOn([FILE], hashes, on), true, "an on entry reads on");
  assert.equal(familySyncOn([FILE], hashes, off), false, "an off entry reads off (criterion 4)");
  assert.equal(familySyncOn([FILE], hashes, []), false, "no entry: never synced, reads off (UX notes F5)");
  assert.equal(familySyncOn([FILE], new Map(), on), false, "a file not yet hashed reads off");
  assert.equal(familySyncOn([], hashes, on), false, "a family without files has no toggle state");
});

test("c1: a family of two files reads on only when both are on", () => {
  const second: FontFileInfo = { id: "ZQX-FONT-7731-Bold.ttf", faces: 1 } as unknown as FontFileInfo;
  const hashes = new Map<string, string>([
    [FILE.id, "sha-a"],
    [second.id, "sha-b"],
  ]);
  assert.equal(familySyncOn([FILE, second], hashes, [{ id: "sha-a", sync: true }, { id: "sha-b", sync: true }]), true);
  assert.equal(familySyncOn([FILE, second], hashes, [{ id: "sha-a", sync: true }, { id: "sha-b", sync: false }]), false);
});

// ---- criterion 1 and UX notes section 2: no S key in the fonts tree (customer, FONT-SKEY C) -----

function keyEvent(key: string) {
  return {
    key,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    preventDefault: noop,
    stopPropagation: noop,
  } as unknown as Parameters<typeof onFontTreeKey>[0];
}

function ctxFor(row: Row, calls: string[]): FontTreeKeyContext {
  const stub = (name: string) => () => {
    calls.push(name);
  };
  return {
    rows: [row],
    focusable: [row],
    activeKey: row.key,
    pageRows: 5,
    focusRow: noop,
    toggle: stub("toggle"),
    pickFamily: stub("pick"),
    pickFace: stub("face"),
    star: stub("star"),
    pressRemove: stub("remove"),
  } as unknown as FontTreeKeyContext;
}

test("c1: S in the fonts tree types into the search, Delete keeps its meaning", () => {
  const row = { key: familyKey(ZQX), kind: "family", family: ZQX, tags: [], open: false, height: 52 } as unknown as Row;
  const calls: string[] = [];
  const typed = (key: string) => ({ ...keyEvent(key) });
  const ctx = { ...ctxFor(row, calls), typeInSearch: (c: string) => calls.push(`type:${c}`) };
  onFontTreeKey(typed("s"), ctx);
  assert.deepEqual(calls, ["type:s"], "S is a letter of the search, not a sync key (customer decision FONT-SKEY C)");
  calls.length = 0;
  onFontTreeKey(keyEvent("Delete"), ctxFor(row, calls));
  assert.deepEqual(calls, ["remove"], "Delete keeps its meaning (the family's trash)");
});

// ---- criterion 18: the kept licence and readme files travel with the font -------------------------

test("c18: a zip font's text licence and readme travel; the folder's names are listed", () => {
  const folders = [
    {
      name: "ZQX-PACK-7731",
      documents: [
        { name: "OFL.txt", size: 1200 },
        { name: "README.md", size: 800 },
      ],
    },
  ];
  const travelling = travellingDocuments("ZQX-PACK-7731", folders);
  assert.ok(travelling.includes("OFL.txt"), "the licence file travels");
  assert.ok(travelling.includes("README.md"), "the readme travels");
});

test("c18: a font outside a folder has no documents to travel", () => {
  assert.deepEqual(travellingDocuments(null, []), [], "no folder, nothing travels");
});

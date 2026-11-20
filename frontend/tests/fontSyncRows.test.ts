// The fonts' sync in the page (`specs/0116-library-sync` slice 2, criteria 16 to 19, UX notes
// sections 1 to 3): the file and family rules, the Curvyo family row's toggle (before its trash,
// pressed look, greyed reason while no device is paired, none on system and Included rows), the
// no key in the fonts tree (type-to-search keeps `S`), and the view's font rows. The row renders on the server through
// the vite harness of `librarySyncRows.test.ts`; the pure rules load without a bundler.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { decodeLibrarySync } from "../src/lib/librarySyncView.ts";
import { familyKey, type Row } from "../src/lib/fontRows.ts";
import { onFontTreeKey, type FontTreeKeyContext } from "../src/lib/fontTreeKeys.ts";
import { HashSetStore, newIdsOfHash, startsOnItsOwn, syncedIdsToTrash, familySyncAny, familySyncOn, filesOfFamily, groupFiles, travellingDocuments } from "../src/lib/fontSyncModel.ts";
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

const FACES = [
  { id: "a1", family: "Lobster", index: 0, stamp: "10-1", file: "Lobster.ttf", size: 10, folder: null },
  { id: "c1", family: "Collection", index: 0, stamp: "30-1", file: "Pack.ttc", size: 30, folder: "Pack" },
  { id: "c1", family: "Collection Bold", index: 1, stamp: "30-1", file: "Pack.ttc", size: 30, folder: "Pack" },
];

test("the faces of one host id are one file, a collection's families share it (criterion 16)", () => {
  const files = groupFiles(FACES);
  assert.equal(files.length, 2);
  assert.deepEqual(files[1].families, ["Collection", "Collection Bold"]);
  assert.equal(filesOfFamily(files, "Collection Bold")[0].id, "c1");
  assert.equal(filesOfFamily(files, "Lobster").length, 1);
});

test("a family reads on only when every file is hashed and has an entry that is on", () => {
  const files = filesOfFamily(groupFiles(FACES), "Lobster");
  const hashes = new Map([["a1", "h1"]]);
  assert.equal(familySyncOn(files, hashes, [{ id: "h1", sync: true }]), true);
  assert.equal(familySyncOn(files, hashes, [{ id: "h1", sync: false }]), false, "a marker is off");
  assert.equal(familySyncOn(files, hashes, []), false, "never synced reads off");
  assert.equal(familySyncOn(files, new Map(), [{ id: "h1", sync: true }]), false, "not hashed yet");
  assert.equal(familySyncOn([], hashes, [{ id: "h1", sync: true }]), false);
});

test("any file on makes a press switch the family off, a family refused in part included", () => {
  const files = filesOfFamily(groupFiles([...FACES, { ...FACES[0], id: "a2", file: "Lobster-Bold.ttf" }]), "Lobster");
  const hashes = new Map([["a1", "h1"], ["a2", "h2"]]);
  const rows = [{ id: "h1", sync: true }];
  assert.equal(familySyncOn(files, hashes, rows), false, "the toggle reads off");
  assert.equal(familySyncAny(files, hashes, rows), true, "but a press switches off");
  assert.equal(familySyncAny(files, hashes, []), false);
  assert.equal(familySyncAny([], hashes, rows), false);
});

test("a hash set is kept in the storage and survives a blocked one", () => {
  const kept = new Map<string, string>();
  const storage = { getItem: (k: string) => kept.get(k) ?? null, setItem: (k: string, v: string) => void kept.set(k, v) };
  const first = new HashSetStore("k", storage);
  first.add("h1");
  first.add("h2");
  first.delete("h2");
  const second = new HashSetStore("k", storage);
  assert.equal(second.has("h1"), true);
  assert.equal(second.has("h2"), false);
  kept.set("k", "not json");
  assert.equal(new HashSetStore("k", storage).has("h1"), false, "damaged text starts empty");
  const blocked = new HashSetStore("k", { getItem: () => { throw new Error("blocked"); }, setItem: () => { throw new Error("blocked"); } });
  blocked.add("h3");
  assert.equal(blocked.has("h3"), true, "in memory only");
  assert.equal(new HashSetStore("k", null).has("x"), false);
});

test("the text documents of a font folder travel, a PDF does not (criterion 18)", () => {
  const folders = [{ name: "Pack", documents: [{ name: "OFL.txt", size: 9 }, { name: "Guide.PDF", size: 9 }] }];
  assert.deepEqual(travellingDocuments("Pack", folders), ["OFL.txt"]);
  assert.deepEqual(travellingDocuments(null, folders), []);
  assert.deepEqual(travellingDocuments("Other", folders), []);
});

test("the view carries the font entries, an older build's view none", () => {
  const base = { machines: [], materials: [], online: true, acked: true, state: "x" };
  assert.deepEqual(decodeLibrarySync(JSON.stringify({ ...base, fonts: [{ id: "h1", sync: true }] }))?.fonts, [
    { id: "h1", sync: true },
  ]);
  assert.deepEqual(decodeLibrarySync(JSON.stringify(base))?.fonts, []);
  assert.equal(decodeLibrarySync(JSON.stringify({ ...base, fonts: [{ id: 1 }] })), null);
});

function family(block: FontFamily["block"]): FontFamily {
  return { block, family: "Lobster", faces: [] };
}

/** One key on a tree whose only row is the family; the calls made. */
function press(key: string, fam: FontFamily, modifier = false): string[] {
  const calls: string[] = [];
  const row: Row = { key: familyKey(fam), kind: "family", family: fam, tags: [], open: false, height: 52 };
  const ctx: FontTreeKeyContext = {
    rows: [row],
    focusable: [row],
    activeKey: row.key,
    pageRows: 10,
    focusRow: noop,
    toggle: noop,
    pickFamily: noop,
    pickFace: noop,
    star: noop,
    pressRemove: noop,
    isArmed: () => false,
    openAddTag: noop,
    focusTagLine: noop,
    returnFocus: noop,
    typeInSearch: (c) => calls.push(`type:${c}`),
  };
  onFontTreeKey(
    {
      key,
      ctrlKey: modifier,
      metaKey: false,
      altKey: false,
      shiftKey: false,
      preventDefault: noop,
      stopPropagation: noop,
    } as unknown as React.KeyboardEvent<HTMLElement>,
    ctx,
  );
  return calls;
}

test("S types into the search on every row: the fonts tree has no sync key (FONT-SKEY C)", () => {
  assert.deepEqual(press("s", family("curvyo")), ["type:s"]);
  assert.deepEqual(press("S", family("curvyo")), ["type:S"]);
  assert.deepEqual(press("s", family("system")), ["type:s"]);
  assert.deepEqual(press("s", family("curvyo"), true), [], "Ctrl+S is the app's save");
});

async function renderRow(props: Record<string, unknown>): Promise<string> {
  // The row imports the font face loader, which reaches the host module: it needs a `window`.
  (globalThis as Record<string, unknown>).window ??= { addEventListener: noop, removeEventListener: noop };
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule("/src/components/panel/FontTreeRows.tsx");
  const fam = { block: "curvyo", family: "Lobster", faces: [{ id: "a1", index: 0, block: "curvyo", family: "Lobster", style: "Regular", faceIndex: 0, loaded: false }] };
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(loaded.FamilyRow as never, {
      rowKey: "curvyo:Lobster",
      family: fam,
      open: false,
      focused: false,
      holdsCurrent: false,
      favourite: false,
      tagNames: [],
      tagLine: null,
      style: {},
      onFocus: noop,
      onPick: noop,
      onToggle: noop,
      onStar: noop,
      ...props,
    } as never),
  );
  return dom.renderToString(element as never);
}

const VIEW = { fonts: [], machines: [], materials: [], online: true, acked: true, state: "In sync with 2 devices." };
const TRASH = { target: "font:a", armed: false, label: "Remove Lobster", tip: "Remove", onPress: noop };

test("a Curvyo family row carries the toggle before its trash and its star keeps its slot", async () => {
  const html = await renderRow({ sync: { on: true, view: VIEW, onToggle: noop }, trash: TRASH });
  assert.match(html, /aria-label="Sync Lobster to my devices"/);
  assert.match(html, /aria-pressed="true"/);
  assert.ok(html.indexOf("Sync Lobster to my devices") < html.indexOf("Remove Lobster"), "sync before trash");
  assert.ok(html.indexOf("Remove Lobster") < html.indexOf("Favourite: Lobster"), "trash before the star");
  assert.doesNotMatch(html, /aria-disabled/);
});

test("the row names its sync state, the tooltip names no key, and a greyed row says nothing", async () => {
  const on = await renderRow({ sync: { on: true, view: VIEW, onToggle: noop }, trash: TRASH });
  assert.match(on, /aria-label="Lobster[^"]*, synced"/);
  const off = await renderRow({ sync: { on: false, view: VIEW, onToggle: noop }, trash: TRASH });
  assert.doesNotMatch(off, /aria-label="[^"]*synced"/);
  const greyed = await renderRow({ sync: { on: true, view: null, onToggle: noop }, trash: TRASH });
  assert.doesNotMatch(greyed, /aria-label="[^"]*synced"/);
  assert.doesNotMatch(on, /\(S\)/);
});

test("without a paired device the toggle is greyed with its reason and shows the stored state", async () => {
  const html = await renderRow({ sync: { on: true, view: null, onToggle: noop }, trash: TRASH });
  assert.match(html, /aria-disabled="true"/);
  assert.match(html, /aria-description="Pair a device to sync\."/);
  assert.match(html, /opacity-40/);
});

test("a family without the sync prop (system, Included, Document Fonts) has no toggle", async () => {
  const html = await renderRow({});
  assert.doesNotMatch(html, /to my devices/);
});

test("a font deleted elsewhere whose copy stays is not switched on and published again", () => {
  const deleted = new HashSetStore("d", null);
  assert.equal(startsOnItsOwn("h1", [], new Set(), deleted), true, "a new install starts on");
  deleted.add("h1");
  assert.equal(startsOnItsOwn("h1", [], new Set(), deleted), false, "the kept copy of a deleted font stays off");
  assert.equal(startsOnItsOwn("h2", [], new Set(), deleted), true, "another font is not held back");
  assert.equal(startsOnItsOwn("h2", [{ id: "h2", sync: false }], new Set(), deleted), false, "a marker leaves it alone");
  assert.equal(startsOnItsOwn("h2", [], new Set(["h2"]), deleted), false, "a refusal is not tried again");
  deleted.delete("h1");
  assert.equal(startsOnItsOwn("h1", [], new Set(), deleted), true, "pressed on again: allowed");
  const stored: Record<string, string> = {};
  const storage = { getItem: (k: string) => stored[k] ?? null, setItem: (k: string, v: string) => { stored[k] = v; } };
  new HashSetStore("k", storage).add("h1");
  assert.equal(startsOnItsOwn("h1", [], new Set(), new HashSetStore("k", storage)), false, "survives a restart");
});

test("only the files written through sync go to the trash, by their id", () => {
  const hashes = new Map([["a/synced.ttf", "h"], ["b/mine.ttf", "h"], ["c/other.ttf", "x"]]);
  const via = new HashSetStore("v", null);
  via.add("a/synced.ttf");
  const listed = [...hashes.keys()];
  assert.deepEqual(syncedIdsToTrash(listed, hashes, "h", via), ["a/synced.ttf"], "his copy in another folder stays");
  via.retain(new Set(["b/mine.ttf"]));
  assert.deepEqual(syncedIdsToTrash(listed, hashes, "h", via), [], "an id that went is forgotten: his later install stays");
  assert.deepEqual(newIdsOfHash(listed, new Set(["b/mine.ttf"]), hashes, "h"), ["a/synced.ttf"], "the file a sync write created");
});

// The library sync's rows (`specs/0116-library-sync` criteria 1 to 4, UX notes sections 1 to 3):
// the toggle in the machine and material rows (the builtin, damaged and newer rows get none),
// the greyed states with their reason line, the glyph states from the view, and the first-sync
// confirm of criterion 3. The pure rules run directly; the rows render on the server through
// the vite harness of `panelStrip.test.ts`.

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import {
  decodeLibrarySync,
  firstSyncAnswer,
  firstSyncAsked,
  librarySyncGlyph,
  librarySyncOn,
} from "../src/lib/librarySyncView.ts";

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

function view(over: Record<string, unknown> = {}) {
  return JSON.stringify({
    machines: [{ id: "u1", sync: true }],
    materials: [{ id: "m1", sync: false }],
    online: true,
    acked: true,
    state: "In sync with 2 devices.",
    ...over,
  });
}

test("the view decodes with every field checked, a wrong shape to null", () => {
  const decoded = decodeLibrarySync(view());
  assert.deepEqual(decoded, {
    fonts: [],
    machines: [{ id: "u1", sync: true }],
    materials: [{ id: "m1", sync: false }],
    online: true,
    acked: true,
    state: "In sync with 2 devices.",
  });
  assert.equal(decodeLibrarySync("{ not json"), null);
  assert.equal(decodeLibrarySync('{"machines": 1}'), null);
  assert.equal(decodeLibrarySync(view({ online: "yes" })), null);
  assert.equal(decodeLibrarySync(view({ machines: [{ id: "u1" }] })), null);
});

test("an item without an entry has its own plain state, an off entry stays off", () => {
  const decoded = decodeLibrarySync(view())!;
  assert.equal(librarySyncOn(decoded, "machines", "u1"), true);
  assert.equal(librarySyncOn(decoded, "materials", "m1"), false);
  assert.equal(librarySyncOn(decoded, "machines", "never-seen"), false, "never synced: the plain cloud, a press turns it on");
  assert.equal(librarySyncOn(null, "machines", "u1"), true, "no group: the stored state shows, on by default");
});

test("the glyph follows the view: cloud, RefreshCw while transferring, CloudOff offline", () => {
  assert.equal(librarySyncGlyph(true, false, true, true), "cloud");
  assert.equal(librarySyncGlyph(true, false, true, false), "transferring");
  assert.equal(librarySyncGlyph(true, false, false, true), "offline");
  assert.equal(librarySyncGlyph(true, false, false, false), "offline");
  assert.equal(librarySyncGlyph(false, false, false, false), "cloud", "an off toggle keeps the cloud");
  assert.equal(librarySyncGlyph(true, true, false, false), "cloud", "a greyed toggle keeps the cloud");
});

test("the first-sync confirm runs the first sync only when confirmed (criterion 3)", () => {
  let calls = 0;
  let declined = 0;
  const sync = () => {
    calls += 1;
  };
  const decline = () => {
    declined += 1;
  };
  firstSyncAnswer(true, sync, decline);
  assert.deepEqual([calls, declined], [1, 0]);
  firstSyncAnswer(false, sync, decline);
  assert.deepEqual([calls, declined], [1, 1], "declining syncs nothing and is told");
});

test("the first-sync confirm shows only in a group without entries until it is answered", () => {
  const empty = decodeLibrarySync(view({ machines: [], materials: [] }))!;
  const full = decodeLibrarySync(view())!;
  assert.equal(firstSyncAsked(empty, ""), true);
  assert.equal(firstSyncAsked(empty, "yes"), false, "confirming ends it");
  assert.equal(firstSyncAsked(empty, "no"), false, "declining ends it");
  assert.equal(firstSyncAsked(full, ""), false, "another device's first sync ends it");
  assert.equal(firstSyncAsked(null, ""), false, "no group: nothing to ask");
  const fonts = decodeLibrarySync(view({ machines: [], materials: [], fonts: [{ id: "h1", sync: true }] }))!;
  assert.equal(firstSyncAsked(fonts, ""), false, "a group with only font entries has synced already");
});

// ---- the rows, rendered --------------------------------------------------------------------------

async function render(module: string, name: string, props: Record<string, unknown>): Promise<string> {
  const react = await import("react");
  const dom = await import("react-dom/server");
  const tooltip = await server.ssrLoadModule("/src/components/ui/tooltip.tsx");
  const loaded = await server.ssrLoadModule(module);
  const element = react.createElement(
    tooltip.TooltipProvider as never,
    null,
    react.createElement(loaded[name] as never, props as never),
  );
  return dom.renderToString(element as never);
}

const PROFILES = [
  { id: "generic-laser", name: "Generic laser", width: 400, height: 300, builtin: true, chosen: false },
  { id: "u1", name: "Garage", width: 600, height: 400, builtin: false, chosen: true },
];

/** ADR 0098 (UX unify 4.2): the machine rows and their sync toggles live in the Machines tab. */
function machinesTabProps(profiles: typeof PROFILES, library: unknown) {
  // The tab's module chain (the connector and size offer) reads the host at load.
  (globalThis as Record<string, unknown>).window ??= {};
  const rows = profiles.map((p) => ({
    id: p.id,
    name: p.name,
    line2: `${p.width} × ${p.height} mm · GRBL`,
    builtin: p.builtin,
    fields: [],
    connection: null,
  }));
  const view = { sections: [{ family: "laser", word: "Laser", active: true, notices: [], rows }], follow: null };
  return { workbenches: { machines: { view, sizeApi: () => ({}) } }, onReturnFocus: noop, library };
}

const GREYED = { view: null, switch: noop, firstSync: noop };

function syncButtons(html: string): string[] {
  return [...html.matchAll(/<button[^>]*aria-label="Sync [^"]+"[^>]*>/g)].map((m) => m[0]);
}

test("without a paired device every machine toggle greys with its reason, the builtin gets none", async () => {
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachinesTab", machinesTabProps(PROFILES, GREYED));
  const buttons = syncButtons(html);
  assert.equal(buttons.length, 1, "one toggle, the user profile's");
  assert.match(buttons[0], /aria-label="Sync Garage to my devices"/);
  assert.match(buttons[0], /aria-pressed="true"/);
  assert.match(buttons[0], /aria-disabled="true"/);
  assert.match(buttons[0], /aria-description="Pair a device to sync\."/);
  assert.match(buttons[0], /opacity-40/);
  assert.doesNotMatch(html, /Sync Generic laser/, "the builtin ships with the app");
  assert.match(html, /Pair a device to sync machine profiles\./);
  assert.match(html, /role="status"[^>]*>Pair a device to sync machine profiles\./);
});

test("with a group the machine toggles show the stored state and the stream's state", async () => {
  const library = {
    view: JSON.parse(view({ machines: [{ id: "u1", sync: false }], acked: false, state: "Syncing..." })),
    switch: noop,
    firstSync: noop,
  };
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachinesTab", machinesTabProps(PROFILES, library));
  const [button] = syncButtons(html);
  assert.match(button, /aria-pressed="false"/, "the stored off state shows");
  assert.doesNotMatch(button, /aria-busy/, "an off toggle keeps the cloud through every state");
  assert.doesNotMatch(button, /aria-disabled/);
  assert.doesNotMatch(html, /Pair a device to sync machine profiles\./, "the line leaves with the first device");
  assert.match(button, /text-\[var\(--panel-muted-fg\)\]/, "off is the outlined cloud");
});

test("an on toggle is busy while this item transfers and presses on the active ground", async () => {
  const library = {
    view: JSON.parse(view({ acked: false, state: "Syncing..." })),
    switch: noop,
    firstSync: noop,
  };
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachinesTab", machinesTabProps(PROFILES, library));
  const [button] = syncButtons(html);
  assert.match(button, /aria-pressed="true"/);
  assert.match(button, /aria-busy="true"/, "RefreshCw while this item transfers");
  assert.match(button, /aria-pressed:bg-\[var\(--toolbar-icon-active-bg\)\]/);
  assert.match(button, /aria-pressed:text-\[var\(--toolbar-icon-active-fg\)\]/);
  assert.doesNotMatch(button, /aria-disabled/);
});

test("an on, in-sync toggle carries the pressed look and no busy state", async () => {
  const on = {
    view: JSON.parse(view()),
    switch: noop,
    firstSync: noop,
  };
  const html = await render("/src/components/workbench/MachinesTab.tsx", "MachinesTab", machinesTabProps(PROFILES, on));
  const [button] = syncButtons(html);
  assert.match(button, /aria-pressed="true"/);
  assert.doesNotMatch(button, /aria-busy/);
  assert.doesNotMatch(button, /aria-disabled/);
});

const MATERIAL_ROWS = [
  { state: "loaded", id: "m1", name: "Birch", thicknessMm: 3, line: "1 laser · 2 sets", tests: 2 },
  { state: "damaged", id: "m2", reason: "Not a record" },
  { state: "newer", id: "m3", name: "Future" },
];

function materialsApi() {
  const view = {
    pageLaser: null,
    rows: MATERIAL_ROWS,
    unfolded: null,
    formOpen: false,
    nextName: "Material 2",
    loadNotices: [] as string[],
  };
  return {
    view,
    notice: null,
    draft: { name: null, thickness: "" },
    setDraft: noop,
    openForm: noop,
    cancelForm: noop,
    add: () => ({ ok: true, id: "" }),
    unfold: noop,
    rename: noop,
    remove: noop,
    move: noop,
    setThickness: () => "",
    toggleLaser: noop,
    openLaserPick: noop,
    pickLaser: noop,
    addSet: () => "",
    editSet: () => "",
    deleteSet: () => "",
    openTest: noop,
    judge: () => "",
    setNote: noop,
    setPhoto: () => "",
    removePhoto: () => "",
    deleteTest: () => "",
    place: () => "",
    photo: () => Promise.resolve(new Uint8Array()),
    showNotice: noop,
  };
}

test("a loaded material row carries the toggle before its trash, damaged and newer rows none", async () => {
  const html = await render("/src/components/panel/MaterialsTab.tsx", "MaterialsTab", {
    api: materialsApi(),
    library: GREYED,
    onReturnFocus: noop,
  });
  const buttons = syncButtons(html);
  assert.equal(buttons.length, 1, "only the readable row's toggle");
  assert.match(buttons[0], /aria-label="Sync Birch to my devices"/);
  assert.match(buttons[0], /aria-disabled="true"/);
  assert.match(html, /role="status"[^>]*>Pair a device to sync materials\./);
  const at = html.indexOf('aria-label="Sync Birch to my devices"');
  const trash = html.indexOf('aria-label="Delete Birch"');
  assert.ok(at >= 0 && trash >= 0 && at < trash, "the toggle sits immediately before the trash");
});

test("a material toggle with the group on shows the stored state and is not greyed", async () => {
  const html = await render("/src/components/panel/MaterialsTab.tsx", "MaterialsTab", {
    api: materialsApi(),
    library: { view: JSON.parse(view({ online: false, state: "Offline." })), switch: noop, firstSync: noop },
    onReturnFocus: noop,
  });
  const [button] = syncButtons(html);
  assert.match(button, /aria-pressed="false"/, "m1's stored off state");
  assert.doesNotMatch(button, /aria-disabled/);
  assert.doesNotMatch(html, /Pair a device to sync materials\./);
});

test("the first-sync confirm sits in the pairing block: the sentence, confirm and decline", async () => {
  const html = await render("/src/components/panel/DeviceChoices.tsx", "LibrarySyncConfirm", {
    sentence: "Your machines and materials leave this computer encrypted, to your own computers only.",
    onSync: noop,
    onDecline: noop,
  });
  const groups = [...html.matchAll(/<div[^>]*role="group"[^>]*>/g)].map((m) => m[0]);
  assert.ok(groups.some((g) => /aria-label="Sync to my devices"/.test(g)));
  assert.match(html, /Your machines and materials leave this computer encrypted, to your own computers only\./);
  assert.match(html, />Sync libraries<\/button>/);
  assert.match(html, />Keep local<\/button>/);
  assert.doesNotMatch(html, /Not now/, "the workspace pick keeps the one Not now");
});

// Tester acceptance cases for `specs/0116-library-sync` slice 1's frontend (criteria 1 to 4 and
// 13), written from the specification and the UX notes before the implementation's rows were
// read: the per-row symbol toggles (the builtin machine and the damaged and newer material rows
// get none), the S key, the greyed reason lines while no device is paired, the first-sync
// confirm of criterion 3, and the browser host's storage keys of criterion 13. The Rust halves
// (the journeys, the confirm's `library_first_sync` writing every item, the server scans) are
// `curvyo-editor-wasm/tests/acceptance_0116_tester.rs` and
// `curvyo-sync-core/tests/acceptance_0116_tester.rs`. The rows render on the server through the
// vite harness of `panelStrip.test.ts`; the pure rules load without a bundler.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { after, before, test } from "node:test";

import { firstSyncAnswer, librarySyncOn, type LibrarySyncView } from "../src/lib/librarySyncView.ts";
import { materialKeyAction } from "../src/lib/materialRows.ts";
import { profileRowKey } from "../src/lib/machineBlockKeys.ts";

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
  // The pairing block's words come from the wasm bindings; give the server's own instance its
  // module, as the flow tests do.
  const bindings = (await server.ssrLoadModule("/src/wasm-bindings/curvyo_editor_wasm.js")) as {
    default: (input: { module_or_path: Buffer }) => Promise<unknown>;
  };
  await bindings.default({
    module_or_path: readFileSync(new URL("../src/wasm-bindings/curvyo_editor_wasm_bg.wasm", import.meta.url)),
  });
});

after(async () => {
  await server.close();
});

const noop = () => undefined;

/** The group's library view as the glue writes it (`library_view_json`). */
function view(over: Record<string, unknown> = {}): LibrarySyncView {
  return JSON.parse(
    JSON.stringify({
      machines: [
        { id: "generic-laser", sync: true },
        { id: "u1", sync: true },
        { id: "u2", sync: false },
      ],
      materials: [{ id: "m1", sync: true }],
      online: true,
      acked: true,
      state: "In sync with 2 devices.",
      ...over,
    }),
  );
}

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

function syncButtons(html: string): string[] {
  return [...html.matchAll(/<button[^>]*aria-label="Sync [^"]+"[^>]*>/g)].map((m) => m[0]);
}

const PROFILES = [
  { id: "generic-laser", name: "Generic laser", width: 400, height: 300, builtin: true, chosen: false },
  { id: "u1", name: "Garage", width: 600, height: 400, builtin: false, chosen: true },
  { id: "u2", name: "Shed", width: 300, height: 200, builtin: false, chosen: false },
];

const LIST_PROPS = {
  rows: PROFILES,
  onPick: noop,
  onDuplicate: () => "",
  onRename: noop,
  onDelete: noop,
  onWorkArea: () => "",
  onClose: noop,
};

const GREYED = { view: null, switch: noop, firstSync: noop };

const MATERIAL_ROWS = [
  { state: "loaded", id: "m1", name: "Birch", thicknessMm: 3, line: "1 laser · 2 sets", tests: 2 },
  { state: "damaged", id: "m2", reason: "Not a record" },
  { state: "newer", id: "m3", name: "Future" },
];

function materialsApi() {
  const apiView = {
    pageLaser: null,
    rows: MATERIAL_ROWS,
    unfolded: null,
    formOpen: false,
    nextName: "Material 2",
    loadNotices: [] as string[],
  };
  return {
    view: apiView,
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

// ---- criterion 1: the per-row toggles ----------------------------------------------------------------

test("c1: every user machine row carries the symbol toggle, the builtin none, not a Tab stop", async () => {
  const html = await render("/src/components/workbench/MachineProfileList.tsx", "MachineProfileList", {
    ...LIST_PROPS,
    library: { view: view(), switch: noop, firstSync: noop },
  });
  const buttons = syncButtons(html);
  assert.equal(buttons.length, 2, "one toggle per user profile, none for the builtin");
  assert.match(buttons[0], /aria-label="Sync Garage to my devices"/);
  assert.match(buttons[0], /aria-pressed="true"/, "u1's entry is on");
  assert.match(buttons[1], /aria-label="Sync Shed to my devices"/);
  assert.match(buttons[1], /aria-pressed="false"/, "u2's stored off state shows (criterion 4)");
  assert.doesNotMatch(html, /Sync Generic laser/, "the builtin ships with the app (criterion 26)");
  for (const button of buttons) {
    assert.match(button, /tabindex="-1"/, "the toggle is not a Tab stop (UX notes §2)");
    assert.doesNotMatch(button, /aria-disabled/);
  }
  // The toggle sits immediately before the row's Delete (UX notes §1).
  for (const name of ["Garage", "Shed"]) {
    const at = html.indexOf(`aria-label="Sync ${name} to my devices"`);
    const trash = html.indexOf(`aria-label="Delete ${name}"`);
    assert.ok(at >= 0 && trash >= 0 && at < trash, `${name}'s toggle sits before its Delete`);
  }
});

test("c1: a readable material row carries the toggle, damaged and newer rows none", async () => {
  const html = await render("/src/components/panel/MaterialsTab.tsx", "MaterialsTab", {
    api: materialsApi(),
    library: GREYED,
    onReturnFocus: noop,
  });
  const buttons = syncButtons(html);
  assert.equal(buttons.length, 1, "only the readable row's toggle");
  assert.match(buttons[0], /aria-label="Sync Birch to my devices"/);
  assert.doesNotMatch(html, /Sync Future/, "a newer-format row cannot be synced");
  const at = html.indexOf('aria-label="Sync Birch to my devices"');
  const trash = html.indexOf('aria-label="Delete Birch"');
  assert.ok(at >= 0 && trash >= 0 && at < trash, "the toggle sits immediately before the trash");
});

// ---- criterion 2: no paired device yet ----------------------------------------------------------------

test("c2: without a paired device the toggles grey with their reason and the state lines show", async () => {
  const machines = await render("/src/components/workbench/MachineProfileList.tsx", "MachineProfileList", {
    ...LIST_PROPS,
    library: GREYED,
  });
  const [button] = syncButtons(machines);
  assert.match(button, /aria-pressed="true"/, "the stored state still shows, on by default");
  assert.match(button, /aria-disabled="true"/, "a press does nothing");
  assert.match(button, /aria-description="Pair a device to sync\."/);
  assert.match(button, /opacity-40/);
  assert.match(machines, /role="status"[^>]*>Pair a device to sync machine profiles\./);
  const materials = await render("/src/components/panel/MaterialsTab.tsx", "MaterialsTab", {
    api: materialsApi(),
    library: GREYED,
    onReturnFocus: noop,
  });
  assert.match(materials, /role="status"[^>]*>Pair a device to sync materials\./);
  // With a group the state lines leave in the frame the first device joins.
  const paired = await render("/src/components/workbench/MachineProfileList.tsx", "MachineProfileList", {
    ...LIST_PROPS,
    library: { view: view(), switch: noop, firstSync: noop },
  });
  assert.doesNotMatch(paired, /Pair a device to sync machine profiles\./);
});

// ---- criterion 4: a new item starts on (Frage 214 A) ---------------------------------------------------

test("c4: an item without an entry is plain until it is published, an off entry stays off", () => {
  // Corrected 2026-11-19 (audit): an item that never synced used to read as on, so a declined
  // first sync looked synced; a new item gets its entry on its first publish (the confirmed
  // first sync, or its toggle) and reads on from then.
  const paired = view();
  assert.equal(librarySyncOn(paired, "machines", "made-afterwards"), false, "no entry: plain, a press turns it on");
  assert.equal(librarySyncOn(paired, "machines", "u2"), false);
  assert.equal(librarySyncOn(paired, "materials", "m1"), true);
  assert.equal(librarySyncOn(null, "machines", "u1"), true, "no group: on by default");
});

// ---- the S key (UX notes §2) --------------------------------------------------------------------------

test("the S key toggles the focused row's sync in both lists, the same key in each", () => {
  assert.deepEqual(profileRowKey("S", false, 0, 3), { kind: "sync" });
  assert.deepEqual(profileRowKey("s", false, 1, 3), { kind: "sync" });
  assert.equal(profileRowKey("s", true, 0, 3), null, "no chord: Ctrl+S is not the toggle");
  assert.deepEqual(profileRowKey("F2", false, 0, 3), { kind: "rename" }, "F2 keeps its meaning");
  assert.deepEqual(profileRowKey("Delete", false, 0, 3), { kind: "delete" }, "Delete keeps its meaning");
  assert.equal(materialKeyAction("S", false, false), "sync");
  assert.equal(materialKeyAction("s", false, true), "sync", "S also switches while armed");
  assert.equal(materialKeyAction("F2", false, false), "rename");
  assert.equal(materialKeyAction("Delete", false, false), "arm", "Delete arms as before");
});

// ---- criterion 3: the first-sync confirm ----------------------------------------------------------------

test("c3: the confirm sits in the pairing block with the spec's sentence, Sync libraries and Not now", async () => {
  const html = await render("/src/components/panel/DeviceChoices.tsx", "PickWorkspaces", {
    label: "Desktop",
    frozenWarning: "",
    offered: [{ id: "w1", name: "Box job", documents: 2, memberOnly: false }],
    synced: [],
    libraryConfirm:
      "Your machines and materials leave this computer encrypted, to your own computers only.",
    onLibrarySync: noop,
    onLibraryDecline: noop,
    onDone: noop,
  });
  const groups = [...html.matchAll(/<div[^>]*role="group"[^>]*>/g)].map((m) => m[0]);
  assert.ok(groups.some((g) => /aria-label="Sync to my devices"/.test(g)), "the confirm's own group");
  assert.match(
    html,
    /Your machines and materials leave this computer encrypted, to your own computers only./,
    "the spec's sentence",
  );
  assert.match(html, />Sync libraries<\/button>/);
  assert.match(html, />Keep local<\/button>/);
  assert.equal(html.match(/>Not now<\/button>/g)?.length, 1, "the workspaces' own Not now is the only one");
});

test("c3: only confirming runs the first sync, declining writes nothing", () => {
  let calls = 0;
  firstSyncAnswer(true, () => {
    calls += 1;
  });
  assert.equal(calls, 1);
  firstSyncAnswer(false, () => {
    calls += 1;
  });
  assert.equal(calls, 1, "declining syncs nothing");
});

// ---- criterion 13: the browser host's storage keys ------------------------------------------------------

/** A localStorage stand-in, as the browser's own. */
class MemoryStorage {
  private map = new Map<string, string>();
  get length(): number {
    return this.map.size;
  }
  key(index: number): string | null {
    return [...this.map.keys()][index] ?? null;
  }
  getItem(key: string): string | null {
    return this.map.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
  removeItem(key: string): void {
    this.map.delete(key);
  }
  *entries(): Iterable<[string, string]> {
    yield* this.map;
  }
}

/** The browser's window for this test's host loads: the localStorage stand-in. */
const storage = new MemoryStorage();

test("c13: the browser host keeps machine and material records under their own keys", async () => {
  const window = {
    localStorage: storage,
    matchMedia: () => ({ matches: false }),
    addEventListener: noop,
    removeEventListener: noop,
  };
  (globalThis as Record<string, unknown>).window = window;
  const host = (await server.ssrLoadModule("/src/platform/host.ts")) as {
    invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
  };
  const machineText = '{ "format_version": 4, "name": "ZQX-MACH-7731" }';
  await host.invoke("write_machine_profile", { id: "u1", text: machineText });
  const materialText = '{ "format_version": 2, "name": "ZQX-MAT-7731" }';
  await host.invoke("write_material", { id: "m1", text: materialText });
  const keys = [...storage.entries()].map(([key]) => key);
  assert.deepEqual(keys, ["curvyo.machines/u1", "curvyo.materials/m1"], "the keys of ADR 0062 decision 8");
  const machines = (await host.invoke("list_machine_profiles")) as { name: string; text: string }[];
  assert.deepEqual(machines, [{ name: "u1.json", text: machineText }]);
  const materials = (await host.invoke("list_materials")) as { name: string; text: string }[];
  assert.deepEqual(materials, [{ name: "m1.json", text: materialText }]);
  await host.invoke("remove_machine_profile", { id: "u1" });
  assert.deepEqual([...storage.entries()].map(([key]) => key), ["curvyo.materials/m1"]);
});

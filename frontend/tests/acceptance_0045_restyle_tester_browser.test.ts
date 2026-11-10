// Tester's independent Chromium check of `specs/0045-document-formats-library` Part H (the
// 2026-11-10 restyle, criteria 36 to 44), written from the specification and `ux-notes.md`:
// Tab order of the symbol buttons, one armed trash at a time, disarm on a press elsewhere and on
// focus leaving the row, the 400 ms guard, rename by blur (commit and revert), the user file in
// browser storage after a rename and after a new group, two formats of one size with one current
// row, Import without a success notice and a refused Import with its notice, the broken file
// without write controls, and the Pages block without position fields on a two-page document.
// Harness as in `acceptance_0045_formats_panel_browser.test.ts`, plus the CDP file chooser.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5462;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0045-restyle-tester");
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 60_000): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch(() => null);
    if (got !== null) {
      return got;
    }
    await sleep(200);
  }
  throw new Error(`timed out waiting for ${what}`);
}

const chromes: ChildProcess[] = [];
process.on("exit", () => chromes.forEach((c) => c.kill("SIGKILL")));

type Reply = { result?: { result?: { value?: unknown }; exceptionDetails?: unknown } };

const KEYS: Record<string, { code: string; vk: number }> = {
  Delete: { code: "Delete", vk: 46 },
  F2: { code: "F2", vk: 113 },
  Enter: { code: "Enter", vk: 13 },
  Escape: { code: "Escape", vk: 27 },
  Tab: { code: "Tab", vk: 9 },
  Home: { code: "Home", vk: 36 },
  End: { code: "End", vk: 35 },
  ArrowLeft: { code: "ArrowLeft", vk: 37 },
  ArrowUp: { code: "ArrowUp", vk: 38 },
  ArrowRight: { code: "ArrowRight", vk: 39 },
  ArrowDown: { code: "ArrowDown", vk: 40 },
};

async function session(profile: string) {
  const portFile = path.join(profile, "DevToolsActivePort");
  rmSync(portFile, { force: true });
  const chrome = spawn(
    BROWSER!,
    ["--headless=new", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
    { stdio: "ignore" },
  );
  chromes.push(chrome);
  const port = await waitFor("DevTools port", async () =>
    existsSync(portFile) && readFileSync(portFile, "utf8").includes("\n") ? readFileSync(portFile, "utf8").split("\n")[0] : null,
  );
  const target = await waitFor("page target", async () => {
    const list = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as { type: string; webSocketDebuggerUrl: string }[];
    return list.find((t) => t.type === "page") ?? null;
  });
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let id = 0;
  const pending = new Map<number, (v: Reply) => void>();
  const closers = new Set<() => void>();
  const events = new Set<(method: string, params: unknown) => void>();
  ws.onclose = () => closers.forEach((fail) => fail());
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as { id?: number; method?: string; params?: unknown };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
    } else if (msg.method) {
      events.forEach((listen) => listen(msg.method!, msg.params));
    }
  };
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const mine = id;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method} in 180 s`)), 180_000);
      pending.set(mine, (v) => {
        clearTimeout(timer);
        resolve(v);
      });
      closers.add(() => {
        clearTimeout(timer);
        reject(new Error(`the browser closed the connection during ${method}`));
      });
      ws.send(JSON.stringify({ id: mine, method, params }));
    });
  const evaluate = async (expression: string): Promise<unknown> => {
    const out = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (out.result?.exceptionDetails) {
      throw new Error(JSON.stringify(out.result.exceptionDetails));
    }
    return out.result?.result?.value;
  };
  const point = async (sel: string): Promise<[number, number]> => {
    const c = JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        e.scrollIntoView({ block: 'center' });
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
    assert.ok(c, `${sel} is on the page`);
    return c;
  };
  const press = async (x: number, y: number, count: number) => {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
    for (let n = 1; n <= count; n += 1) {
      await send("Input.dispatchMouseEvent", { type: "mousePressed", x, y, button: "left", clickCount: n });
      await send("Input.dispatchMouseEvent", { type: "mouseReleased", x, y, button: "left", clickCount: n });
    }
    await sleep(250);
  };
  const clickSel = async (sel: string) => {
    const [x, y] = await point(sel);
    await press(x, y, 1);
  };
  const dblclickSel = async (sel: string) => {
    const [x, y] = await point(sel);
    await press(x, y, 2);
  };
  const hover = async (sel: string) => {
    const [x, y] = await point(sel);
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: x + 1, y });
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
  };
  const key = async (k: string) => {
    const { code, vk } = KEYS[k];
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code, windowsVirtualKeyCode: vk });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code, windowsVirtualKeyCode: vk });
    await sleep(250);
  };
  const type = async (text: string) => {
    await send("Input.insertText", { text });
    await sleep(150);
  };
  const close = async () => {
    await sleep(300);
    await send("Browser.close").catch(() => undefined);
    ws.close();
    await new Promise((r) => {
      if (chrome.exitCode !== null) {
        r(null);
      } else {
        chrome.once("exit", r);
        setTimeout(() => {
          chrome.kill("SIGKILL");
          r(null);
        }, 5000);
      }
    });
  };
  await send("Page.enable");
  // A changed document asks before it unloads; the test leaves it.
  events.add((method) => {
    if (method === "Page.javascriptDialogOpening") {
      void send("Page.handleJavaScriptDialog", { accept: true });
    }
  });
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  /** Presses `sel` and answers the file chooser it opens with `file`. */
  const chooseFile = async (sel: string, file: string) => {
    await send("Page.setInterceptFileChooserDialog", { enabled: true });
    const opened = new Promise<number>((resolve) => {
      const listen = (method: string, params: unknown) => {
        if (method === "Page.fileChooserOpened") {
          events.delete(listen);
          resolve((params as { backendNodeId: number }).backendNodeId);
        }
      };
      events.add(listen);
    });
    await clickSel(sel);
    const backendNodeId = await opened;
    await send("DOM.setFileInputFiles", { files: [file], backendNodeId });
    await sleep(600);
  };
  return { send, evaluate, clickSel, dblclickSel, hover, key, type, close, chooseFile };
}

type S = Awaited<ReturnType<typeof session>>;

const q = (sel: string) => JSON.stringify(sel);
const exists = async (s: S, sel: string) => (await s.evaluate(`!!document.querySelector(${q(sel)})`)) as boolean;
const count = async (s: S, sel: string) => (await s.evaluate(`document.querySelectorAll(${q(sel)}).length`)) as number;
const label = (name: string) => `[aria-label=${JSON.stringify(name)}]`;
const activeLabel = async (s: S) => (await s.evaluate(`document.activeElement?.getAttribute('aria-label') ?? ''`)) as string;
/** Presses the button with the visible text `word` inside `scope`. */
const clickText = async (s: S, scope: string, word: string) => {
  await s.evaluate(`(() => { const b = [...document.querySelector(${q(scope)}).querySelectorAll('button')].find((e) => e.textContent.trim() === ${q(word)}); b.setAttribute('data-e2e', 'x'); })()`);
  await s.clickSel("[data-e2e=x]");
  await s.evaluate(`document.querySelectorAll('[data-e2e]').forEach((e) => e.removeAttribute('data-e2e'))`);
};
const focusSel = (s: S, sel: string) => s.evaluate(`(() => { const e = document.querySelector(${q(sel)}); e.focus(); if (e.select) e.select(); })()`);


const SEED = `format = 1
favourites = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "u-key-ring", "u-coaster"]

[[group]]
id = "u-laser"
name = "Laser"
default_orientation = "portrait"

  [[group.preset]]
  id = "u-key-ring"
  name = "Key ring"
  short_side = 90
  long_side = 90
  unit = "mm"

  [[group.preset]]
  id = "u-coaster"
  name = "Coaster"
  short_side = 90
  long_side = 90
  unit = "mm"
`;

const STORE = "curvyo.document-formats";
const stored = async (s: S) => (await s.evaluate(`localStorage.getItem(${q(STORE)}) ?? ''`)) as string;

async function openApp(s: S, seed: string | null) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await exists(s, "[role=tab][aria-label^=Document]")) ? true : null));
  await s.evaluate(seed === null ? `localStorage.removeItem(${q(STORE)})` : `localStorage.setItem(${q(STORE)}, ${q(seed)})`);
  await s.send("Page.reload");
  await sleep(500);
  await waitFor("the app again", async () => ((await exists(s, "[role=tab][aria-label^=Document]")) ? true : null));
  await sleep(1200);
  await s.clickSel("[role=tab][aria-label^=Document]");
  await sleep(300);
}

async function expand(s: S) {
  await s.evaluate(`[...document.querySelectorAll('button[aria-controls]')].find((b) => b.textContent.startsWith('All formats')).setAttribute('data-e2e', 'disc')`);
  if ((await s.evaluate(`document.querySelector('[data-e2e=disc]').getAttribute('aria-expanded')`)) !== "true") {
    await s.clickSel("[data-e2e=disc]");
  }
}

const row = (name: string) => `[data-rk^="fmt:"]:has(button[aria-label^=${JSON.stringify(`Apply ${name},`)}])`;
const isSelected = async (s: S, name: string) =>
  ((await s.evaluate(`document.querySelector(${q(row(name))}).className`)) as string).includes("value-fill");
const isArmed = async (s: S, name: string) =>
  ((await s.evaluate(`document.querySelector(${q(row(name))}).className`)) as string).includes("destructive-soft");
const visibleText = async (s: S) => (await s.evaluate(`document.body.innerText`)) as string;

async function run(body: (s: S) => Promise<void>) {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await body(s);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
}

test("0045 restyle tester: Tab order, one armed trash, disarm rules, 400 ms guard", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await run(async (s) => {
    await openApp(s, SEED);
    await expand(s);
    // Criterion 38: with a user format all three, in the order Add, Import, Export, each 28 x 28.
    const names = (await s.evaluate(`(() => { const d = document.querySelector('[data-e2e=disc]'); return [...d.parentElement.querySelectorAll('button')].filter((b) => b !== d).map((b) => { const r = b.getBoundingClientRect(); return b.getAttribute('aria-label') + ':' + r.width + 'x' + r.height; }); })()`)) as string[];
    assert.deepEqual(names, ["Add format:28x28", "Import formats...:28x28", "Export my formats...:28x28"]);
    // Tab order: disclosure, Add, Import, Export, then one stop in the list body.
    await focusSel(s, "[data-e2e=disc]");
    await s.key("Tab");
    assert.equal(await activeLabel(s), "Add format");
    await s.key("Tab");
    assert.equal(await activeLabel(s), "Import formats...");
    await s.key("Tab");
    assert.equal(await activeLabel(s), "Export my formats...");
    await s.key("Tab");
    assert.equal(await s.evaluate(`!!document.activeElement.closest('[role=group][aria-label=Formats]')`), true, "the list body follows");
    await s.key("Tab");
    assert.equal(await s.evaluate(`!!document.activeElement.closest('[role=group][aria-label=Formats]')`), false, "the list body is one Tab stop");

    // The 400 ms guard: a second press 250 ms after the first does nothing.
    await s.clickSel(label("Delete Key ring"));
    assert.equal(await isArmed(s, "Key ring"), true);
    await s.clickSel(label("Press again to delete Key ring."));
    assert.equal(await exists(s, row("Key ring")), true, "a press inside 400 ms does not delete");
    // One armed trash at a time: arming Coaster disarms Key ring.
    await sleep(500);
    await s.clickSel(label("Delete Coaster"));
    assert.equal(await isArmed(s, "Coaster"), true);
    assert.equal(await isArmed(s, "Key ring"), false, "one trash armed at a time");
    assert.equal(await exists(s, label("Delete Key ring")), true);
    // A press anywhere else disarms (here: on the "All formats" disclosure's neighbour, a group name).
    await s.clickSel(`[data-rk^="grp:"]:has(button[aria-label="Fold group Laser"]) span.truncate`);
    await sleep(200);
    assert.equal(await isArmed(s, "Coaster"), false, "a press elsewhere disarms");
    assert.equal(await exists(s, row("Coaster")), true);
    // Focus leaving the row disarms: Delete arms, Down moves to the next row.
    await focusSel(s, `${row("Key ring")} [data-control=apply]`);
    await s.key("Delete");
    assert.equal(await isArmed(s, "Key ring"), true, "Delete arms");
    await s.key("ArrowDown");
    assert.equal(await isArmed(s, "Key ring"), false, "focus left the row");
    // Enter on apply does not delete.
    await focusSel(s, `${row("Coaster")} [data-control=apply]`);
    await s.key("Enter");
    assert.equal(await exists(s, row("Coaster")), true);
    // The status region is polite and its armed text was announced.
    assert.ok((await count(s, "[role=status][aria-live=polite]")) >= 1);
    // Nothing deleted so far: the file still has both.
    assert.match(await stored(s), /u-key-ring[\s\S]*u-coaster/);
  });
});

test("0045 restyle tester: one size twice, rename by blur, new group portrait, import notices, broken file", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await run(async (s) => {
    await openApp(s, SEED);
    await expand(s);
    // Criterion 39: apply the second 90 x 90; only the first in list order has the selected look.
    await s.clickSel(`${row("Coaster")} [data-control=apply]`);
    await sleep(300);
    assert.equal(Number(await s.evaluate(`document.querySelector('input[aria-label="Document width"]').value`)), 90);
    assert.equal(await isSelected(s, "Key ring"), true, "the first match is current");
    assert.equal(await isSelected(s, "Coaster"), false, "the second match has the normal look");
    assert.ok((await visibleText(s)).includes("Key ring"), "the subject names a format");

    // Criterion 27 / 39: importing the seed again adds nothing and shows nothing.
    const dir = mkdtempSync(path.join(SCRATCH, "files-"));
    const good = path.join(dir, "laser.toml");
    writeFileSync(good, SEED);
    const rowsBefore = await count(s, '[data-rk^="fmt:"]');
    await s.chooseFile(label("Import formats..."), good);
    await sleep(500);
    assert.equal(await count(s, '[data-rk^="fmt:"]'), rowsBefore, "nothing added the second time");
    assert.ok(!/Imported|Skipped/.test(await visibleText(s)), "no success notice after an import");
    assert.equal(await count(s, `${row("Coaster")}`), 1);
    // Criterion 43: blur commits a valid change.
    await focusSel(s, `${row("Key ring")} [data-control=apply]`);
    await s.key("F2");
    await waitFor("the rename field", async () => ((await exists(s, label("Name of Key ring"))) ? true : null));
    await s.type("Ring");
    await s.evaluate(`document.activeElement.blur()`);
    await waitFor("renamed by blur", async () => ((await exists(s, row("Ring"))) ? true : null));
    const file = await stored(s);
    assert.match(file, /id = "u-key-ring"\s*\n\s*name = "Ring"/, file);
    // Blur with an invalid name reverts without a chip and writes nothing.
    await focusSel(s, `${row("Ring")} [data-control=apply]`);
    await s.key("F2");
    await waitFor("the rename field", async () => ((await exists(s, label("Name of Ring"))) ? true : null));
    await s.type("coaster");
    await s.evaluate(`document.activeElement.blur()`);
    await sleep(300);
    assert.equal(await exists(s, "input[aria-label^='Name of']"), false, "the field closed");
    assert.equal(await exists(s, row("Ring")), true, "the old name stays");
    const alertText = async () => (await s.evaluate(`[...document.querySelectorAll("[role=alert]")].map((e) => e.textContent.trim()).filter(Boolean).join(" || ")`)) as string;
    assert.equal(await alertText(), "", "no chip, no notice");
    assert.equal(await stored(s), file, "nothing written");
    // Rename of a group by F2 on its fold button; Escape cancels, nothing written.
    await focusSel(s, `[data-rk^="grp:"] button[aria-label="Fold group Laser"]`);
    await s.key("F2");
    await waitFor("the group rename field", async () => ((await exists(s, label("Name of group Laser"))) ? true : null));
    await s.type("Paper");
    await s.key("Enter");
    assert.equal(await exists(s, label("Name of group Laser")), true, "a built-in group's name is taken");
    assert.notEqual(await alertText(), "", "with its chip");
    await s.key("Escape");
    assert.equal(await exists(s, "input[aria-label^='Name of']"), false);
    assert.equal(await stored(s), file, "nothing written");

    // Criterion 42: a group made from the form reads portrait in the user file.
    await s.clickSel(label("Add format"));
    await waitFor("Name focused", async () => ((await activeLabel(s)) === "Format name" ? true : null));
    await s.type("Board");
    await focusSel(s, label("Format width"));
    await s.type("300");
    await focusSel(s, label("Format height"));
    await s.type("200");
    await clickText(s, "form[aria-labelledby]", "New group…");
    assert.equal(await count(s, "form[aria-labelledby] [role=radio][aria-label*=ortrait], form[aria-labelledby] [role=radio][aria-label*=andscape]"), 0);
    await focusSel(s, label("Group name"));
    await s.type("Kitchen");
    await clickText(s, "form[aria-labelledby]", "Add");
    await waitFor("the form to close", async () => (!(await exists(s, "form[aria-labelledby]")) ? true : null));
    const withGroup = await stored(s);
    assert.match(withGroup, /name = "Kitchen"\s*\n\s*default_orientation = "portrait"/, withGroup);
    assert.ok(!/Added|Saved/.test(await visibleText(s)), "no success notice");

    // A refused import keeps its notice.
    const bad = path.join(dir, "bad.toml");
    writeFileSync(bad, "format = 1\n[[group]]\nid =");
    const afterGood = await stored(s);
    await s.chooseFile(label("Import formats..."), bad);
    await waitFor("the refusal", async () => ((await visibleText(s)).includes("Could not import bad.toml:") ? true : null), 5000);
    assert.ok((await visibleText(s)).includes("Nothing was changed."));
    assert.equal(await stored(s), afterGood, "a refused import writes nothing");

    // Criterion 6 / 38: a broken file has no Add, Import, Export, trash or star.
    await openApp(s, "format = 1\n[[group]\n");
    await expand(s);
    assert.ok((await visibleText(s)).includes("Your formats file could not be read:"));
    assert.equal(await count(s, `${label("Add format")}, ${label("Import formats...")}, ${label("Export my formats...")}`), 0);
    assert.equal(await count(s, "button[aria-label^='Delete'], [data-control=star], [data-control=edit]"), 0);
    assert.equal(await stored(s), "format = 1\n[[group]\n", "the broken file is not changed");
    rmSync(dir, { recursive: true, force: true });
  });
});

test("0045 restyle tester: Pages block of a two-page document and the Unit row", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await run(async (s) => {
    await openApp(s, null);
    await s.clickSel(`button${label("Add page")}`);
    await waitFor("the Add page group", async () => ((await exists(s, `[role=group]${label("Add page")}`)) ? true : null));
    await s.key("Enter");
    await sleep(500);
    assert.equal(await count(s, "[aria-label^='Reorder ']"), 2, "two pages");
    // Criterion 44: no position field and no unit word for one.
    assert.equal(await count(s, "input[aria-label*='Page X'], input[aria-label*='Page Y'], input[aria-label^='X'], input[aria-label^='Y']"), 0);
    const pages = (await s.evaluate(`document.querySelector('section[aria-label=Pages]').innerText`)) as string;
    assert.ok(!/Page X|Page Y|Position/.test(pages), pages);
    // The Unit row stays and still changes how Width is shown.
    await s.clickSel("[role=radio][aria-label='cm, centimetres']");
    await sleep(300);
    assert.equal(Number(await s.evaluate(`document.querySelector('input[aria-label="Document width"]').value`)), 21);
    await s.clickSel("[role=radio][aria-label='mm, millimetres']");
    await sleep(300);
    assert.equal(Number(await s.evaluate(`document.querySelector('input[aria-label="Document width"]').value`)), 210);
  });
});

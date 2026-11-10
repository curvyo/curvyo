// `specs/0045-document-formats-library` Part H (criteria 35 to 44) in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the
// symbol buttons of the "All formats" row, armed delete of a format and of a group, inline rename,
// no lower bar and no "Opens as", the Origin picker of the Grid & Rulers pane with its keys, and
// the Pages block without X and Y. Harness as in `acceptance_0089_f_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5245;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0045-panel");
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
  ws.onclose = () => closers.forEach((fail) => fail());
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as { id?: number };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
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
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, clickSel, dblclickSel, hover, key, type, close };
}

type S = Awaited<ReturnType<typeof session>>;

const q = (sel: string) => JSON.stringify(sel);
const exists = async (s: S, sel: string) => (await s.evaluate(`!!document.querySelector(${q(sel)})`)) as boolean;
const count = async (s: S, sel: string) => (await s.evaluate(`document.querySelectorAll(${q(sel)}).length`)) as number;
const text = async (s: S, sel: string) => (await s.evaluate(`document.querySelector(${q(sel)})?.textContent ?? ''`)) as string;
const label = (name: string) => `[aria-label=${JSON.stringify(name)}]`;
const activeLabel = async (s: S) => (await s.evaluate(`document.activeElement?.getAttribute('aria-label') ?? ''`)) as string;
const tooltip = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.textContent).join('|')`)) as string;
/** Presses the button with the visible text `word` inside `scope`. */
const clickText = async (s: S, scope: string, word: string) => {
  await s.evaluate(`(() => { const b = [...document.querySelector(${q(scope)}).querySelectorAll('button')].find((e) => e.textContent.trim() === ${q(word)}); b.setAttribute('data-e2e', 'x'); })()`);
  await s.clickSel("[data-e2e=x]");
  await s.evaluate(`document.querySelectorAll('[data-e2e]').forEach((e) => e.removeAttribute('data-e2e'))`);
};
const focusSel = (s: S, sel: string) => s.evaluate(`(() => { const e = document.querySelector(${q(sel)}); e.focus(); if (e.select) e.select(); })()`);

async function openApp(s: S) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await exists(s, "[role=tab][aria-label^=Document]")) ? true : null));
  await sleep(1200);
}

const row = (name: string) => `[data-rk^="fmt:"]:has(button[aria-label^=${JSON.stringify(`Apply ${name},`)}])`;
const groupHeader = (name: string) => `[data-rk^="grp:"]:has(button[aria-label=${JSON.stringify(`Fold group ${name}`)}])`;

/** Adds a format through the form; `group` is a new group name when `isNew`. */
async function addFormat(s: S, name: string, width: string, height: string, group: string, isNew: boolean) {
  await s.clickSel(label("Add format"));
  await waitFor("the name field focused", async () => ((await activeLabel(s)) === "Format name" ? true : null));
  await focusSel(s, label("Format name"));
  await s.type(name);
  await focusSel(s, label("Format width"));
  await s.type(width);
  await focusSel(s, label("Format height"));
  await s.type(height);
  if (isNew) {
    await clickText(s, "form[aria-labelledby]", "New group…");
    await focusSel(s, label("Group name"));
    await s.type(group);
  } else {
    await clickText(s, "form[aria-labelledby]", group);
  }
  await clickText(s, "form[aria-labelledby]", "Add");
  await waitFor("the form to close", async () => (!(await exists(s, "form[aria-labelledby]")) ? true : null));
}

test("0045 Part H in Chromium: symbol buttons, armed delete, inline rename, no lower bar", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openApp(s);
    await s.clickSel("[role=tab][aria-label^=Document]");
    await sleep(300);

    // Collapsed, the row is the disclosure alone (criterion 38).
    assert.equal(await exists(s, label("Add format")), false);
    await s.evaluate(`[...document.querySelectorAll('button[aria-controls]')].find((b) => b.textContent.startsWith('All formats')).setAttribute('data-e2e', 'x')`);
    await s.clickSel("[data-e2e=x]");
    await s.evaluate(`document.querySelectorAll('[data-e2e]').forEach((e) => e.removeAttribute('data-e2e'))`);

    // Criterion 38: Add and Import as 28 x 28 symbol buttons, no Export without a user format,
    // tooltips equal to the names; they come before the list in the tree.
    assert.equal(await exists(s, label("Add format")), true);
    assert.equal(await exists(s, label("Import formats...")), true);
    assert.equal(await exists(s, label("Export my formats...")), false);
    const box = JSON.parse(
      (await s.evaluate(`(() => { const b = document.querySelector(${q(label("Add format"))}).getBoundingClientRect(); return JSON.stringify([b.width, b.height]); })()`)) as string,
    ) as number[];
    assert.deepEqual(box, [28, 28]);
    assert.equal(
      await s.evaluate(`(() => { const add = document.querySelector(${q(label("Add format"))}); const imp = document.querySelector(${q(label("Import formats..."))}); const list = document.querySelector('[role=group][aria-label=Formats]'); const f = Node.DOCUMENT_POSITION_FOLLOWING; return !!(add.compareDocumentPosition(imp) & f) && !!(imp.compareDocumentPosition(list) & f); })()`),
      true,
      "Add, Import, then the list",
    );
    await s.hover(label("Add format"));
    await waitFor("the tooltip", async () => ((await tooltip(s)).includes("Add format") ? true : null));

    // Criterion 40: nothing follows the last group; no text buttons "Add format" etc.
    assert.equal(
      await s.evaluate(`[...document.querySelectorAll('button')].some((b) => ['Add format', 'Import formats', 'Export my formats'].includes(b.textContent.trim()))`),
      false,
    );

    // Criterion 38: Add opens the form directly under the header row, focus in Name, pressed.
    await s.clickSel(label("Add format"));
    await waitFor("Name focused", async () => ((await activeLabel(s)) === "Format name" ? true : null));
    // Customer 2026-11-10: no tooltips on the Unit and Group rows of the form.
    for (const sel of ['[aria-label="Format unit"] [aria-label="cm, centimetres"]', 'form[aria-labelledby] [role=radio][aria-label^="Group "]']) {
      await s.hover(sel);
      await sleep(900);
      assert.equal(await tooltip(s), "", `no tooltip over ${sel}`);
    }
    assert.equal(await s.evaluate(`document.querySelector(${q(label("Add format"))}).getAttribute('aria-expanded')`), "true");
    assert.equal(
      await s.evaluate(`(() => { const f = document.querySelector('form[aria-labelledby]'); const g = document.querySelector('[role=group][aria-label=Formats]'); return !!(f.compareDocumentPosition(g) & Node.DOCUMENT_POSITION_FOLLOWING); })()`),
      true,
      "the form is above the first group",
    );
    // Criterion 42: no "Opens as", also for a new group.
    await clickText(s, "form[aria-labelledby]", "New group…");
    assert.equal(await exists(s, label("Group name")), true);
    assert.equal(await s.evaluate(`document.body.innerText.includes('Opens as')`), false);
    await focusSel(s, label("Format name"));
    await s.key("Escape");
    await waitFor("the form to close", async () => (!(await exists(s, "form[aria-labelledby]")) ? true : null));
    assert.equal(await activeLabel(s), "Add format", "focus returns to Add");

    // Make two user formats with the size of A4 between them (criterion 39: nothing refuses).
    await addFormat(s, "Key ring", "30", "50", "Laser", true);
    assert.equal(await activeLabel(s), "Add format");
    await addFormat(s, "Coaster", "90", "90", "Laser", false);
    await addFormat(s, "A4 twin", "210", "297", "Laser", false);
    assert.equal(await exists(s, label("Export my formats...")), true, "Export appears with a user format");
    // No success notice anywhere (customer rule).
    const alerts = (await s.evaluate(`[...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent).join('|')`)) as string;
    assert.ok(!/Added|Saved|Imported/.test(alerts), alerts);

    // Criterion 35: user rows have edit and trash, built-in rows none.
    assert.equal(await exists(s, `${row("Key ring")} ${label("Edit Key ring")}`), true);
    assert.equal(await exists(s, `${row("Key ring")} ${label("Delete Key ring")}`), true);
    assert.equal(await count(s, `${row("A4")} button[aria-label^=Delete], ${row("A4")} button[aria-label^=Edit]`), 0);
    // The trash is not a Tab stop; the Pencil, star and apply are in the roving set.
    assert.equal(await s.evaluate(`document.querySelector(${q(label("Delete Key ring"))}).tabIndex`), -1);

    // Criterion 36: the first press arms (soft red, new name, nothing deleted), 4 s disarms.
    await s.clickSel(label("Delete Key ring"));
    assert.equal(await exists(s, label("Press again to delete Key ring.")), true);
    assert.equal(await exists(s, row("Key ring")), true);
    assert.match(
      (await s.evaluate(`document.querySelector(${q(row("Key ring"))}).className`)) as string,
      /destructive-soft/,
    );
    assert.equal(await exists(s, "[role=alertdialog], [role=dialog]"), false);
    await waitFor("disarm after 4 s", async () => ((await exists(s, label("Delete Key ring"))) ? true : null), 8000);
    assert.equal(await exists(s, row("Key ring")), true);
    // Escape disarms (consumed).
    await s.clickSel(label("Delete Key ring"));
    await s.key("Escape");
    assert.equal(await exists(s, label("Delete Key ring")), true, "Escape disarms");
    // A press inside the 400 ms guard does nothing; the press after it deletes.
    await s.clickSel(label("Delete Coaster"));
    await sleep(500);
    await s.clickSel(label("Press again to delete Coaster."));
    await waitFor("Coaster gone", async () => (!(await exists(s, row("Coaster"))) ? true : null));
    assert.match(
      (await s.evaluate(`[...document.querySelectorAll('[role=status]')].map((e) => e.textContent).join('|')`)) as string,
      /Coaster deleted\./,
    );

    // Criterion 36: Delete on the keyboard arms, a second Delete deletes; focus goes on.
    await focusSel(s, `${row("A4 twin")} [data-control=apply]`);
    await s.key("Delete");
    assert.equal(await exists(s, label("Press again to delete A4 twin.")), true);
    await sleep(500);
    await s.key("Delete");
    await waitFor("A4 twin gone", async () => (!(await exists(s, row("A4 twin"))) ? true : null));
    assert.ok((await activeLabel(s)).startsWith("Apply Key ring,"), "focus on the remaining row's apply");

    // Built-in rows do nothing on Delete and F2.
    await focusSel(s, `${row("A4")} [data-control=apply]`);
    await s.key("Delete");
    await s.key("F2");
    assert.equal(await exists(s, row("A4")), true);
    assert.equal(await exists(s, "input[aria-label^='Name of']"), false);

    // Criterion 43: double-click starts the rename; Enter confirms; the id and list order stay.
    await s.dblclickSel(`${row("Key ring")} span.truncate`);
    await waitFor("the rename field", async () => ((await exists(s, label("Name of Key ring"))) ? true : null));
    assert.equal(await s.evaluate(`document.activeElement?.getAttribute('aria-label')`), "Name of Key ring");
    assert.equal(await s.evaluate(`(() => { const e = document.activeElement; return e.selectionStart === 0 && e.selectionEnd === e.value.length; })()`), true, "text selected");
    await s.type("Keyring");
    await s.key("Enter");
    await waitFor("renamed", async () => ((await exists(s, row("Keyring"))) ? true : null));
    assert.equal(await exists(s, "input[aria-label^='Name of']"), false);
    assert.equal(await activeLabel(s), (await s.evaluate(`document.querySelector(${q(row("Keyring"))}).querySelector('[data-control=apply]').getAttribute('aria-label')`)) as string, "focus on the apply button");
    // F2 starts it, Escape cancels without a write.
    await focusSel(s, `${row("Keyring")} [data-control=apply]`);
    await s.key("F2");
    await waitFor("the rename field", async () => ((await exists(s, label("Name of Keyring"))) ? true : null));
    await s.type("Other");
    await s.key("Escape");
    assert.equal(await exists(s, row("Keyring")), true, "Escape keeps the old name");
    assert.equal(await exists(s, "input[aria-label^='Name of']"), false);
    // An existing name keeps the field open with the chip.
    await addFormat(s, "Coaster", "90", "90", "Laser", false);
    await focusSel(s, `${row("Keyring")} [data-control=apply]`);
    await s.key("F2");
    await s.type("coaster");
    await s.key("Enter");
    assert.equal(await exists(s, label("Name of Keyring")), true, "the field stays open");
    assert.match(await text(s, "input[aria-label='Name of Keyring'] + [role=alert]"), /This group already has Coaster/);
    await s.key("Escape");

    // A group renames the same way.
    await s.dblclickSel(`${groupHeader("Laser")} span.truncate`);
    await waitFor("the group rename field", async () => ((await exists(s, label("Name of group Laser"))) ? true : null));
    await s.type("Cut");
    await s.key("Enter");
    await waitFor("group renamed", async () => ((await exists(s, groupHeader("Cut"))) ? true : null));

    // Criterion 37: the group's armed name says how many formats go; the second press deletes.
    await s.clickSel(label("Delete group Cut"));
    assert.equal(await exists(s, label("Press again to delete group Cut. Its 2 formats go too.")), true);
    await sleep(500);
    await s.clickSel(label("Press again to delete group Cut. Its 2 formats go too."));
    await waitFor("group gone", async () => (!(await exists(s, groupHeader("Cut"))) ? true : null));
    assert.equal(await exists(s, label("Export my formats...")), false, "Export goes with the last user format");
    // The group has no edit button and the list ends with a group header or a row (criterion 40).
    assert.equal(
      await s.evaluate(`(() => { const g = document.querySelector('[role=group][aria-label=Formats]'); const last = g.lastElementChild; return !!last.querySelector('[data-rk]') && !g.nextElementSibling; })()`),
      true,
    );
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

test("0045 Part H in Chromium: the Origin picker of Grid & Rulers and the Pages block", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openApp(s);

    // Criterion 44: the Pages block has no Page X / Page Y field; the Unit row stays.
    await s.clickSel("[role=tab][aria-label^=Document]");
    await sleep(300);
    assert.equal(await count(s, "input[aria-label^='Page X'], input[aria-label^='Page Y']"), 0);
    assert.equal(await exists(s, "[role=radio][aria-label='mm, millimetres']"), true);

    // Criterion 41: the control of the laser Machine block.
    await s.clickSel("[role=tab][aria-label^='Grid']");
    await sleep(300);
    const group = "[role=radiogroup][aria-label='Origin corner']";
    assert.equal(await count(s, `${group} [role=radio]`), 4);
    assert.equal(
      await s.evaluate(`[...document.querySelectorAll(${q(`${group} [role=radio]`)})].every((r) => { const b = r.getBoundingClientRect(); return b.width === 28 && b.height === 28; })`),
      true,
    );
    assert.equal(await count(s, `${group} [role=radio][tabindex="0"]`), 1, "one Tab stop");
    const checked = async () => (await s.evaluate(`document.querySelector(${q(`${group} [aria-checked=true]`)}).getAttribute('aria-label')`)) as string;
    assert.equal(await checked(), "Top left");
    await s.hover(`${group} [aria-label='Bottom right']`);
    await waitFor("the corner tooltip", async () => ((await tooltip(s)).includes("Bottom right") ? true : null));
    await focusSel(s, `${group} [aria-checked=true]`);
    await s.key("ArrowRight");
    assert.equal(await checked(), "Top right");
    assert.equal(await activeLabel(s), "Top right", "a move chooses and focuses");
    await s.key("ArrowRight");
    assert.equal(await checked(), "Top right", "no wrap");
    await s.key("ArrowDown");
    assert.equal(await checked(), "Bottom right");
    await s.key("ArrowRight");
    assert.equal(await checked(), "Bottom right", "Right stays on Bottom right");
    await s.key("ArrowUp");
    assert.equal(await checked(), "Top right");
    await s.key("End");
    assert.equal(await checked(), "Bottom right");
    await s.key("Home");
    assert.equal(await checked(), "Top left");
    // A pointer press chooses and gives the keyboard back to the canvas.
    await s.clickSel(`${group} [aria-label='Bottom left']`);
    assert.equal(await checked(), "Bottom left");
    assert.equal(await s.evaluate(`!!document.activeElement?.closest(${q(group)})`), false, "focus left the picker");
    // The old pictogram is gone.
    assert.equal(await s.evaluate(`document.querySelector(${q(group)}).className.includes('72px')`), false);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

// `specs/0109-page-autolayout` in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the customer's case of
// "Page 1" beside a laser page. Switching Page 1 to Landscape moves the laser page by the width
// gain (criterion 1) with its own rectangle (criterion 14); Page 1's rectangle and one on the
// pasteboard stay (criteria 15, 17); the screen-reader region says "1 page moved." (ux-notes §7);
// one Ctrl+Z restores everything (criterion 26); deleting Page 1 closes the hole (criteria 30, 35).
// Harness as in `acceptance_0045_formats_panel_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5246;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0109-autolayout");
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
      ws.send(JSON.stringify({ id: mine, method, params }));
    });
  const evaluate = async (expression: string): Promise<unknown> => {
    const out = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (out.result?.exceptionDetails) {
      throw new Error(JSON.stringify(out.result.exceptionDetails));
    }
    return out.result?.result?.value;
  };
  const click = async (x: number, y: number) => {
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1 });
    }
    await sleep(400);
  };
  const clickSel = async (sel: string) => {
    const c = JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        e.scrollIntoView({ block: 'center' });
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
    assert.ok(c, `${sel} is on the page`);
    await click(c[0], c[1]);
  };
  const drag = async (from: [number, number], to: [number, number]) => {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: from[0], y: from[1] });
    await send("Input.dispatchMouseEvent", { type: "mousePressed", x: from[0], y: from[1], button: "left", buttons: 1, clickCount: 1 });
    for (let i = 1; i <= 8; i++) {
      const x = from[0] + ((to[0] - from[0]) * i) / 8;
      const y = from[1] + ((to[1] - from[1]) * i) / 8;
      await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "left", buttons: 1 });
    }
    await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: to[0], y: to[1], button: "left", buttons: 0, clickCount: 1 });
    await sleep(500);
  };
  /** A key press; `modifiers` 2 is Ctrl. */
  const key = async (k: string, vk: number, modifiers = 0) => {
    const code = k.length === 1 ? `Key${k.toUpperCase()}` : k;
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code, windowsVirtualKeyCode: vk, modifiers });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code, windowsVirtualKeyCode: vk, modifiers });
    await sleep(400);
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
  await send("Emulation.setDeviceMetricsOverride", { width: 1600, height: 900, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, click, clickSel, drag, key, close };
}

type S = Awaited<ReturnType<typeof session>>;
type Frame = { name: string; x: number; y: number; w: number; h: number };

const q = (sel: string) => JSON.stringify(sel);
const exists = async (s: S, sel: string) => (await s.evaluate(`!!document.querySelector(${q(sel)})`)) as boolean;
const label = (name: string) => `[aria-label=${JSON.stringify(name)}]`;

/** The page frames on screen, in CSS px, by the page name in their label. */
async function frames(s: S): Promise<Frame[]> {
  return JSON.parse(
    (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[data-page-frames] > div')].map((e) => {
      const b = e.getBoundingClientRect();
      return { name: e.querySelector('[data-page-label] span')?.textContent ?? '', x: b.x, y: b.y, w: b.width, h: b.height };
    }))`)) as string,
  ) as Frame[];
}
const frame = async (s: S, name: string) => {
  const f = (await frames(s)).find((p) => p.name === name);
  assert.ok(f, `the frame of ${name}`);
  return f;
};
const focusCanvas = (s: S) => s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
const tab = async (s: S, name: string) => {
  await s.clickSel(`[role=tab][aria-label^="${name}"]`);
  await sleep(300);
};
const selectedRows = async (s: S) => (await s.evaluate(`document.querySelectorAll('[role=row][aria-level][aria-selected=true]').length`)) as number;
const movedStatus = async (s: S) => (await s.evaluate(`[...document.querySelectorAll('p[role=status].sr-only')].map((e) => e.textContent).join('|')`)) as string;

/** Whether a Select click at `at` selects an object (the Layers tab must be open). A press on a
 * page of another kind may first activate that page (`specs/0098` criterion 8), so a second
 * press decides. */
async function hits(s: S, at: [number, number]): Promise<boolean> {
  await focusCanvas(s);
  await s.key("Escape", 27);
  await s.key("s", 83);
  let hit = false;
  for (let press = 0; press < 3 && !hit; press++) {
    await s.click(at[0], at[1]);
    hit = await waitFor("a selection", async () => ((await selectedRows(s)) > 0 ? true : null), 1500).catch(() => false);
  }
  await s.key("Escape", 27);
  return hit;
}

test("0109 in Chromium: Page 1 to Landscape moves the laser page and its drawing, one undo, delete closes the hole", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await exists(s, "[role=tab][aria-label^=Document]")) ? true : null));
    await sleep(1200);
    await tab(s, "Document");

    // The laser page of the customer's report, placed by Add page right of Page 1.
    await s.clickSel(`button${label("Add page")}`);
    await waitFor("the Add page group", async () => ((await exists(s, `[role=group]${label("Add page")}`)) ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[role=group][aria-label="Add page"] [role=radio]')].find((r) => r.textContent.trim() === 'Laser').click()`);
    await sleep(300);
    await s.evaluate(`[...document.querySelectorAll('[role=group][aria-label="Add page"] button')].find((b) => b.textContent.trim() === 'Add').click()`);
    await waitFor("two frames", async () => ((await frames(s)).length === 2 ? true : null));
    await sleep(500);
    const laserName = (await frames(s)).find((f) => f.name !== "Page 1")!.name;

    // Zoom out (Ctrl+wheel) so both pages and the pasteboard around them are on screen.
    await focusCanvas(s);
    const canvas = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x + r.width / 2, r.y + r.height / 2]; })())`)) as string,
    ) as [number, number];
    for (let i = 0; i < 12 && (await frames(s)).some((f) => f.x < 300 || f.x + f.w > 1250 || f.y < 200); i++) {
      const all = await frames(s);
      const left = Math.min(...all.map((f) => f.x));
      const right = Math.max(...all.map((f) => f.x + f.w));
      const width = right - left;
      if (width > 700) {
        await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: canvas[0], y: canvas[1], deltaX: 0, deltaY: 200, modifiers: 2 });
      } else {
        // Small enough: pan the pages to the middle of the canvas.
        const dx = canvas[0] - (left + right) / 2;
        const dy = canvas[1] + 60 - (all[0].y + all[0].h / 2);
        await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: canvas[0], y: canvas[1], deltaX: -dx, deltaY: -dy, modifiers: 0 });
      }
      await sleep(400);
    }
    const page1 = await frame(s, "Page 1");
    const laser = await frame(s, laserName);
    const mm = page1.w / 210;
    assert.ok(Math.abs(laser.x - (page1.x + page1.w + 29.7 * mm)) < 2, "the laser page is the gap right of Page 1");

    // A rectangle on the laser page (its own item; the laser page is active after Add).
    await s.key("r", 82);
    const ownAt: [number, number] = [laser.x + 40 * mm, laser.y + 40 * mm];
    await s.drag(ownAt, [ownAt[0] + 30 * mm, ownAt[1] + 30 * mm]);
    await s.key("Escape", 27);
    // Back on Page 1: a rectangle on it and one on the pasteboard above it.
    await tab(s, "Document");
    await focusCanvas(s);
    await s.key("s", 83);
    await s.click(page1.x + 100 * mm, page1.y + 250 * mm);
    await s.key("r", 82);
    const drawingAt: [number, number] = [page1.x + 30 * mm, page1.y + 30 * mm];
    await s.drag(drawingAt, [drawingAt[0] + 30 * mm, drawingAt[1] + 30 * mm]);
    await s.key("Escape", 27);
    await s.key("r", 82);
    const pasteAt: [number, number] = [page1.x + 30 * mm, page1.y - 50 * mm];
    await s.drag(pasteAt, [pasteAt[0] + 30 * mm, pasteAt[1] + 20 * mm]);
    await s.key("Escape", 27);
    assert.ok(pasteAt[1] > 0, "the pasteboard rectangle is on screen");

    // Points on the left edges of the three rectangles.
    const ownEdge: [number, number] = [ownAt[0], ownAt[1] + 15 * mm];
    const drawingEdge: [number, number] = [drawingAt[0], drawingAt[1] + 15 * mm];
    const pasteEdge: [number, number] = [pasteAt[0], pasteAt[1] + 10 * mm];
    await tab(s, "Layers");
    assert.equal(await hits(s, ownEdge), true, "fixture: the laser rectangle");
    assert.equal(await hits(s, drawingEdge), true, "fixture: the Page 1 rectangle");
    assert.equal(await hits(s, pasteEdge), true, "fixture: the pasteboard rectangle");

    // Criterion 1: Page 1 turns about its centre; the laser page moves 43.5 mm right.
    await tab(s, "Document");
    await s.clickSel(label("Orientation Landscape"));
    await waitFor("Page 1 in landscape", async () => (Math.abs((await frame(s, "Page 1")).w - 297 * mm) < 2 ? true : null), 10_000);
    const turned = await frame(s, laserName);
    assert.ok(Math.abs(turned.x - (laser.x + 43.5 * mm)) < 2, `the laser page moved 43.5 mm: ${laser.x} -> ${turned.x}`);
    assert.ok(Math.abs(turned.y - laser.y) < 1, "on x only");
    const landscape = await frame(s, "Page 1");
    assert.ok(Math.abs(turned.x - (landscape.x + landscape.w + 29.7 * mm)) < 2, "still exactly the gap away");
    assert.equal(await movedStatus(s), "1 page moved.", "the screen-reader region only");

    // Criteria 14, 15, 17: the laser page's rectangle went with it; the others stayed.
    await tab(s, "Layers");
    // Page 1 is active: its own rectangles first, then the laser page's.
    assert.equal(await hits(s, drawingEdge), true, "Page 1's rectangle stays");
    assert.equal(await hits(s, pasteEdge), true, "the pasteboard rectangle stays");
    const ownMoved: [number, number] = [ownEdge[0] + 43.5 * mm, ownEdge[1]];
    assert.equal(await hits(s, ownMoved), true, "the laser rectangle moved with its page");
    assert.equal(await hits(s, ownEdge), false, "and is no longer at its old place");

    // Criterion 26: one Ctrl+Z restores every frame and object.
    await focusCanvas(s);
    await s.key("z", 90, 2);
    await waitFor("Page 1 in portrait", async () => (Math.abs((await frame(s, "Page 1")).w - 210 * mm) < 2 ? true : null), 10_000);
    const undone = await frame(s, laserName);
    assert.ok(Math.abs(undone.x - laser.x) < 1, "the laser page is back");
    assert.equal(await hits(s, ownEdge), true, "the laser rectangle is back");

    // Criteria 30, 35: deleting Page 1 closes the hole: the laser page starts where it started.
    await tab(s, "Document");
    const del = label("Delete page: Page 1");
    await s.clickSel(del);
    await waitFor("one frame", async () => ((await frames(s)).length === 1 ? true : null), 10_000);
    const closed = await frame(s, laserName);
    assert.ok(Math.abs(closed.x - page1.x) < 2, `the laser page closed the hole: ${page1.x} vs ${closed.x}`);
    assert.equal(await movedStatus(s), "1 page moved.");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

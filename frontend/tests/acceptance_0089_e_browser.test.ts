// `specs/0089-trace-quality-and-colour` slice E in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the bar order
// (criteria 22, 22a, `ux-notes.md` §24.2: Preset and the Mask group, Method and its fields, Smooth and
// Tolerance, Ignore below, the expander, the blue Trace button; nothing before the method's fields
// moves between methods), at 1280 x 800 and 800 x 600. The Mask view and its Shift+\\ cycle of the
// first version of this file are retired with the views (UX notes §24.1; the bar has one view).
// Harness as in `acceptance_0089_c_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5207;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089e");
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
  const requests: string[] = [];
  const choosers: number[] = [];
  const pending = new Map<number, (v: Reply) => void>();
  const closers = new Set<() => void>();
  ws.onclose = () => closers.forEach((fail) => fail());
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as {
      id?: number;
      method?: string;
      params?: { request?: { url: string }; backendNodeId?: number };
    };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
    }
    if (msg.method === "Network.requestWillBeSent" && msg.params?.request) {
      requests.push(msg.params.request.url);
    }
    if (msg.method === "Page.fileChooserOpened" && msg.params?.backendNodeId !== undefined) {
      choosers.push(msg.params.backendNodeId);
    }
  };
  // A call the browser never answers (a crashed or closed page) fails with its name instead of
  // leaving the test pending until its timeout.
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const mine = id;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method} in 180 s: ${JSON.stringify(params).slice(0, 200)}`)), 180_000);
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
  const click = async (x: number, y: number) => {
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1 });
    }
    await sleep(300);
  };
  const key = async (k: string, code: number, modifiers = 0) => {
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: `Key${k.toUpperCase()}`, windowsVirtualKeyCode: code, modifiers });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: `Key${k.toUpperCase()}`, windowsVirtualKeyCode: code, modifiers });
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
  /** Presses `sel`, waits for the file chooser and gives it `file`. */
  const choose = async (sel: string, file: string) => {
    const before = choosers.length;
    await clickSel(sel);
    const node = await waitFor("the file chooser", async () => (choosers.length > before ? choosers[choosers.length - 1] : null), 10_000);
    await send("DOM.setFileInputFiles", { files: [file], backendNodeId: node });
    await sleep(1500);
  };
  const tab = async (name: string) => {
    await clickSel(`[role=tab][aria-label^="${name}"]`);
    await sleep(300);
  };
  const statusText = async () =>
    (await evaluate(`[...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent).join(' | ')`)) as string;
  const dialogText = async () =>
    (await evaluate(`(() => { const d = document.querySelector('[role=alertdialog]'); return d ? d.innerText : ''; })()`)) as string;
  const layerRows = async () =>
    JSON.parse((await evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-level]')].map((r) => r.getAttribute('aria-label')))`)) as string) as string[];
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
  await send("Network.enable");
  await send("Page.setInterceptFileChooserDialog", { enabled: true });
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  const chooserCount = () => choosers.length;
  return { send, evaluate, click, key, clickSel, choose, tab, statusText, dialogText, layerRows, close, requests, chooserCount, choosers };
}


/** A `width` x `height` RGB PNG coloured by `colour` (`[r, g, b]`), 96 dpi. */
function rgbPng(width: number, height: number, colour: (x: number, y: number) => [number, number, number]): Buffer {
  const chunk = (kind: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const all = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(all) >>> 0);
    return Buffer.concat([len, all, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const rows = Array.from({ length: height }, (_, y) => Buffer.from([0, ...Array.from({ length: width }, (_, x) => colour(x, y)).flat()]));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A red block (x 30 to 90) and a black block (x 110 to 170), y 20 to 80, on white, 200 x 100. */
const BLOCKS = rgbPng(200, 100, (x, y) => {
  if (y < 20 || y >= 80) return [255, 255, 255];
  if (x >= 30 && x < 90) return [255, 0, 0];
  if (x >= 110 && x < 170) return [0, 0, 0];
  return [255, 255, 255];
});

const BAR = "[role=toolbar][aria-label=Trace]";

type S = Awaited<ReturnType<typeof session>>;

async function openTrace(s: S, file: string, width: number, height: number) {
  await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [aria-label="More trace options"]')`)) ? true : null));
  await sleep(1200);
}

const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};

const escape = async (s: S) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await sleep(400);
};

/** The bar's items in the order criterion 22 names, with their screen boxes. */
const BAR_ITEMS: [string, string][] = [
  ["i", `${BAR} button[aria-label="About Trace"]`],
  ["Preset", `${BAR} button[aria-label=Preset]`],
  ["Mask group", `${BAR} [data-trace-mask]`],
  ["Method", `${BAR} button[aria-label=Method]`],
  ["method field", `${BAR} [role=spinbutton][aria-label=Threshold], ${BAR} [role=spinbutton][aria-label="Colour tolerance"]`],
  ["Smooth", `${BAR} [role=spinbutton][aria-label=Smooth]`],
  ["Tolerance", `${BAR} [role=spinbutton][aria-label=Tolerance]`],
  ["Ignore below", `${BAR} [role=spinbutton][aria-label="Ignore below"]`],
  ["expander", `${BAR} button[aria-expanded][aria-controls]`],
  ["Trace button", `${BAR} [data-trace-slot]`],
];

type Box = { x: number; y: number; w: number; h: number };
const barBoxes = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify(${JSON.stringify(BAR_ITEMS)}.map(([name, sel]) => { const e = document.querySelector(sel); const b = e ? e.getBoundingClientRect() : null;
      return [name, b ? { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) } : null]; }))`)) as string,
  ) as [string, Box | null][];

/** Whether `boxes` read in order: each on a later line, or on the same line further right. */
const inReadingOrder = (boxes: [string, Box | null][]) => {
  for (let i = 1; i < boxes.length; i++) {
    const [pn, p] = boxes[i - 1];
    const [n, b] = boxes[i];
    assert.ok(p && b, `${pn} and ${n} are drawn`);
    const sameLine = Math.abs(p.y + p.h / 2 - (b.y + b.h / 2)) < 14;
    assert.ok(sameLine ? b.x >= p.x + p.w - 1 : b.y + b.h / 2 > p.y + p.h / 2, `${n} comes after ${pn}: ${JSON.stringify([p, b])}`);
  }
};

test("0089 E in Chromium: the bar order for every method, 1280 x 800 and 800 x 600", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const file = path.join(SCRATCH, "blocks.png");
  writeFileSync(file, BLOCKS);
  const profiles: string[] = [];
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const rows: string[] = [];
    for (const [width, height] of [[1280, 800], [800, 600]]) {
      // A fresh browser per size: leaving a project with an image asks before unloading.
      const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
      profiles.push(profile);
      const s = await session(profile);
      closeAll = s.close;
      await openTrace(s, file, width, height);
      for (const open of [true, false]) {
        const panelOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
        if (panelOpen !== open) {
          await s.clickSel(`[aria-label="${panelOpen ? "Hide panel" : "Show panel"}"]`);
          await sleep(600);
        }
        let first: [string, Box | null][] | null = null;
        for (const method of ["Brightness", "Remove background", "Pick colours"]) {
          await pickMethod(s, method);
          if (method === "Pick colours") {
            await escape(s);
          }
          const boxes = await barBoxes(s);
          const where = `${width} x ${height}, panel ${open ? "open" : "closed"}, ${method}`;
          inReadingOrder(boxes);
          // The finish button is the last item, at the far right of the bar.
          const bar = JSON.parse((await s.evaluate(`JSON.stringify(document.querySelector('${BAR}').getBoundingClientRect())`)) as string) as { right: number; height: number };
          const finish = boxes[boxes.length - 1][1]!;
          assert.ok(Math.abs(finish.x + finish.w - (bar.right - 8)) < 1.5, `${where}: the Trace button at the far right`);
          // Nothing before the method's own fields moves when the method changes.
          first ??= boxes;
          for (let i = 0; i <= 4; i++) {
            assert.deepEqual(boxes[i], first[i], `${where}: ${boxes[i][0]} keeps its place`);
          }
          rows.push(`${where}: bar ${Math.round(bar.height)} px`);
        }
        await pickMethod(s, "Brightness");
      }
      await closeAll();
      closeAll = async () => undefined;
    }
    console.log(rows.join("\n"));
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    for (const profile of profiles) {
      removeProfile(profile);
    }
  }
});

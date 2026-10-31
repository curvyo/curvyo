// `specs/0089-trace-quality-and-colour` slice C in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Method menu and
// its fields (criterion 14), the pipette on the canvas with the colour chip (criteria 16, 17;
// Frage 89 A), the Esc order, Drop border shapes (Frage 88 A), the finish button (customer
// 2026-10-30: blue, "Trace", no flicker), and the bar at 800 x 600 for every method. Harness as
// in `acceptance_0025_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5203;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089c");
const IMAGES = path.resolve(FRONTEND, "..", "curvyo-image-core", "tests", "fixtures", "images");
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


/** The trace worker's answers held back `window.__delay` ms, and a worker that dies at start while
 * `window.__failWorker` is set (as the slice 3 tester does): the slot's states on demand. */
const SLOW_WORKER = `(() => {
  const Real = window.Worker;
  window.__failWorker = false;
  window.__delay = 0;
  const set = Object.getOwnPropertyDescriptor(Real.prototype, 'onmessage').set;
  window.Worker = class extends Real {
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
        if (window.__delay > 0 && event.data && event.data.kind === 'trace') setTimeout(() => fn(event), window.__delay);
        else fn(event);
      });
    }
    constructor(url, options) {
      if (window.__failWorker) super(URL.createObjectURL(new Blob(['throw new Error("boom")'], { type: 'text/javascript' })), options);
      else super(url, options);
    }
  };
})();`;

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

/** Three pale blocks on white across the middle row: #FFF2A8, #D9D9D9 and #DCD6CF. */
const PALE = rgbPng(300, 100, (x, y) => {
  if (y < 20 || y >= 80) return [255, 255, 255];
  if (x >= 10 && x < 90) return [0xff, 0xf2, 0xa8];
  if (x >= 110 && x < 190) return [0xd9, 0xd9, 0xd9];
  if (x >= 210 && x < 290) return [0xdc, 0xd6, 0xcf];
  return [255, 255, 255];
});

const BAR = "[role=toolbar][aria-label=Trace]";

type S = Awaited<ReturnType<typeof session>>;

/** Opens the app with the pale picture imported and the Trace tool chosen. */
async function openTrace(s: S, file: string, width: number, height: number) {
  await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await s.send("Page.addScriptToEvaluateOnNewDocument", { source: SLOW_WORKER });
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("pale.png")) ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [aria-label="More trace options"]')`)) ? true : null));
  await sleep(800);
}

const canvasRect = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string,
  ) as [number, number, number, number];

const move = async (s: S, x: number, y: number) => {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
  await sleep(60);
};

const chip = async (s: S) => (await s.evaluate(`document.querySelector('[data-colour-chip]')?.textContent ?? ''`)) as string;

const press = async (s: S, x: number, y: number, modifiers = 0) => {
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, modifiers });
  }
  await sleep(500);
};

const escape = async (s: S) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await sleep(400);
};

const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};

const fieldNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=spinbutton]')].map((i) => i.getAttribute('aria-label')).filter((n) => n !== 'Corners'))`)) as string) as string[];

const pipettePressed = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-pipette]')?.getAttribute('aria-pressed') ?? 'none'`)) as string;

const swatches = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-swatch]')].map((e) => e.getAttribute('data-trace-swatch')))`)) as string) as string[];

/** The counts of the Advanced block once a trace settled with `paths` paths. */
const settled = async (s: S, paths: RegExp) =>
  waitFor(
    `counts ${paths}`,
    async () => {
      const t = (await s.evaluate(`document.querySelector('${BAR} [data-trace-counts]')?.textContent ?? ''`)) as string;
      return paths.test(t) ? t : null;
    },
    30_000,
  );

test("0089 C in Chromium: Method, pipette, chip, Esc order, finish button", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "pale.png");
  writeFileSync(file, PALE);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, file, 1280, 800);

    // Customer 2026-10-30: the finish button is blue and reads "Trace".
    const finish = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot=none]'); const r = b.getBoundingClientRect();
      return { text: b.textContent, bg: getComputedStyle(b).backgroundColor, width: Math.round(r.width), height: Math.round(r.height) }; })())`)) as string) as { text: string; bg: string; width: number; height: number };
    assert.deepEqual([finish.text, finish.bg, finish.width, finish.height], ["Trace", "rgb(47, 111, 238)", 75, 28]);
    assert.equal(await s.evaluate(`[...document.querySelectorAll('button')].some((b) => b.textContent.trim() === 'Create paths')`), false);

    // Criterion 14: Brightness shows the Threshold; the Method menu lists the three methods.
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await settled(s, /^\d+ paths?,/);
    assert.deepEqual(await fieldNames(s), ["Threshold", "Smooth", "Tolerance", "Ignore below"]);
    await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
    await sleep(300);
    const items = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].map((e) => [e.querySelector('span').textContent.replace('✓', '').trim(), e.getAttribute('aria-checked')]))`)) as string) as [string, string][];
    assert.deepEqual(items, [["Brightness", "true"], ["Remove background", "false"], ["Pick colours", "false"]]);
    await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
    await sleep(300);
    // Advanced: the renamed toggle (Frage 88 A).
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} button[aria-label="Drop border shapes"]')`), true);

    // Pick colours: the pipette is armed on entering, the Colour tolerance takes the Threshold's
    // place; the chip names the colour under the pointer.
    await pickMethod(s, "Pick colours");
    assert.deepEqual(await fieldNames(s), ["Colour tolerance", "Smooth", "Tolerance", "Ignore below"]);
    assert.equal(await pipettePressed(s), "true", "armed on entering Pick colours");
    const [cx, cy, cw, ch] = await canvasRect(s);
    const found = new Map<string, number>();
    for (let x = cx + 40; x < cx + cw - 40; x += 6) {
      await move(s, x, cy + ch / 2);
      const t = await chip(s);
      const hex = t.match(/#[0-9A-F]{6}/)?.[0];
      if (hex && !found.has(hex)) {
        found.set(hex, x);
      }
      if (!hex && t.includes("No colour here") && !found.has("none")) {
        found.set("none", x);
      }
    }
    console.log("chip over the canvas:", JSON.stringify([...found.entries()]));
    for (const hex of ["#FFF2A8", "#D9D9D9", "#DCD6CF", "#FFFFFF", "none"]) {
      assert.ok(found.has(hex), `the chip shows ${hex}`);
    }
    // A click picks; Shift+click adds; the pipette stays armed.
    const at = (hex: string) => found.get(hex)! + 15;
    await press(s, at("#FFF2A8"), cy + ch / 2);
    await waitFor("the first swatch", async () => ((await swatches(s)).join() === "#FFF2A8" ? true : null));
    await settled(s, /^1 path,/);
    await press(s, at("#D9D9D9"), cy + ch / 2, 8);
    await waitFor("the second swatch", async () => ((await swatches(s)).join() === "#FFF2A8,#D9D9D9" ? true : null));
    await settled(s, /^2 paths,/);
    assert.equal(await pipettePressed(s), "true", "Pick colours stays armed");
    assert.equal(await s.evaluate(`document.querySelectorAll('[role=toolbar][aria-label=Trace]').length`), 1, "the presses selected nothing else");
    // A plain click replaces the picked colours.
    await press(s, at("#DCD6CF"), cy + ch / 2);
    await waitFor("the replaced swatch", async () => ((await swatches(s)).join() === "#DCD6CF" ? true : null));
    await settled(s, /^1 path,/);
    // Esc disarms the pipette, the tool stays; the next Esc leaves the tool.
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await escape(s);
    assert.equal(await pipettePressed(s), "false", "Esc disarms");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR}')`), true, "the tool stays");
    // Remove a swatch: select it, its Remove button.
    await s.clickSel(`${BAR} [data-trace-swatch="#DCD6CF"]`);
    await s.clickSel(`${BAR} button[aria-label="Remove #DCD6CF"]`);
    await waitFor("no swatch", async () => ((await swatches(s)).length === 0 ? true : null));
    await waitFor("no colour picked", async () => ((await s.statusText()).includes("0 paths: no colour picked.") ? true : null));

    // Remove background: Auto pressed with the border colour; the three blocks are traced.
    await pickMethod(s, "Remove background");
    assert.deepEqual(await fieldNames(s), ["Colour tolerance", "Smooth", "Tolerance", "Ignore below"]);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [data-trace-background]').textContent`), "#FFFFFF");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} button[aria-pressed]')].find((b) => b.textContent === 'Auto').getAttribute('aria-pressed')`), "true");
    assert.equal(await pipettePressed(s), "false");
    await settled(s, /^3 paths,/);
    // A background pick ends after one click and releases Auto.
    await s.clickSel(`${BAR} [data-trace-pipette]`);
    assert.equal(await pipettePressed(s), "true");
    await press(s, at("#D9D9D9"), cy + ch / 2);
    await waitFor("the picked background", async () => ((await s.evaluate(`document.querySelector('${BAR} [data-trace-background]').textContent`)) === "#D9D9D9" ? true : null));
    assert.equal(await pipettePressed(s), "false", "one pick in Remove background");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} button[aria-pressed]')].find((b) => b.textContent === 'Auto').getAttribute('aria-pressed')`), "false");

    // Customer 2026-10-30: a quick job never shows its progress; a job past 400 ms shows it in
    // the blue button and holds it at least 300 ms.
    await pickMethod(s, "Brightness");
    await settled(s, /paths?,/);
    await s.evaluate(`window.__slots = []; new MutationObserver(() => { const e = document.querySelector('${BAR} [data-trace-slot]'); const k = e ? e.getAttribute('data-trace-slot') : 'gone';
      if (window.__slots.at(-1)?.[0] !== k) window.__slots.push([k, performance.now()]); }).observe(document.querySelector('${BAR}'), { subtree: true, childList: true, attributes: true });`);
    const slotsDuring = async (delay: number) => {
      await s.evaluate(`window.__delay = ${delay}; window.__slots = [];`);
      await pickMethod(s, "Pick colours");
      await sleep(delay + 1200);
      await pickMethod(s, "Brightness");
      await sleep(delay + 1200);
      return JSON.parse((await s.evaluate(`JSON.stringify(window.__slots)`)) as string) as [string, number][];
    };
    const quick = await slotsDuring(0);
    assert.ok(!quick.some(([k]) => k === "running"), `a quick job flashes nothing: ${JSON.stringify(quick)}`);
    const slow = await slotsDuring(480);
    const shown = slow.flatMap(([k, t], i) => (k === "running" ? [(slow[i + 1]?.[1] ?? t) - t] : []));
    console.log("progress shown (ms):", shown.map(Math.round));
    assert.ok(shown.length > 0, `a job past 400 ms shows its progress: ${JSON.stringify(slow)}`);
    assert.ok(shown.every((ms) => ms >= 290), `held at least 300 ms: ${shown}`);
    const running = JSON.parse((await s.evaluate(`(async () => { window.__delay = 3000; document.querySelector('${BAR} button[aria-label=Method]').click(); await new Promise((r) => setTimeout(r, 200));
      [...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes('Pick colours')).click();
      await new Promise((r) => setTimeout(r, 1200)); const e = document.querySelector('${BAR} [data-trace-slot=running]'); if (!e) return 'null'; const r = e.getBoundingClientRect();
      return JSON.stringify({ bg: getComputedStyle(e).backgroundColor, width: Math.round(r.width), height: Math.round(r.height), stop: !!e.querySelector('button[aria-label="Stop (Esc)"]'), text: e.textContent }); })()`)) as string) as { bg: string; width: number; height: number; stop: boolean; text: string } | null;
    assert.ok(running, "the running button");
    assert.deepEqual([running.bg, running.width, running.height, running.stop], ["rgb(47, 111, 238)", 75, 28, true], JSON.stringify(running));
    await s.evaluate(`window.__delay = 0; [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await escape(s);
    await waitFor("Stopped", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=stopped]')`)) ? true : null));
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-slot=stopped] button')].map((b) => b.textContent).join()`), "Retry");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

test("0089 C in Chromium at 800x600: every method's bar keeps inside its edge, panel open and closed", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "pale.png");
  writeFileSync(file, PALE);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, file, 800, 600);
    const measure = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => { const bar = document.querySelector('${BAR}'); const box = bar.getBoundingClientRect();
        const over = [...bar.querySelectorAll('button, input, [role=radiogroup], [role=spinbutton], [role=listbox]')].filter((e) => e.getBoundingClientRect().width > 0 && e.getBoundingClientRect().right > box.right + 0.5)
          .map((e) => e.getAttribute('aria-label') || e.textContent);
        const slot = bar.querySelector('[data-trace-slot]').getBoundingClientRect();
        const ex = bar.querySelector('button[aria-expanded][aria-controls]').getBoundingClientRect();
        return { over, height: Math.round(box.height), inner: Math.round(box.width - 16), slotRight: Math.abs(slot.right - (box.right - 8)) < 1.5, slotAfterExpander: slot.left >= ex.right - 0.5 || slot.top > ex.top + 10 }; })())`)) as string) as
        { over: string[]; height: number; inner: number; slotRight: boolean; slotAfterExpander: boolean };
    const rows: string[] = [];
    for (const method of ["Brightness", "Remove background", "Pick colours"]) {
      await pickMethod(s, method);
      if (method === "Pick colours") {
        await escape(s);
      }
      for (const open of [true, false]) {
        const panelOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
        if (panelOpen !== open) {
          await s.clickSel(`[aria-label="${panelOpen ? "Hide panel" : "Show panel"}"]`);
          await sleep(600);
        }
        const m = await measure();
        rows.push(`${method}, panel ${open ? "open" : "closed"}: inner ${m.inner} px, bar ${m.height} px`);
        assert.deepEqual(m.over, [], `${method}, panel ${open ? "open" : "closed"}: controls past the edge`);
        assert.ok(m.slotRight && m.slotAfterExpander, `${method}: the finish button at the far right after the expander`);
        const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
        writeFileSync(path.join(SCRATCH, `c-${method.replace(/ /g, "-")}-${open ? "open" : "closed"}-800x600.png`), Buffer.from(shot.result?.data ?? "", "base64"));
      }
    }
    console.log(rows.join("\n"));
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

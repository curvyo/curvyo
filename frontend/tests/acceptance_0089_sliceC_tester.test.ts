// Tester cases for `specs/0089-trace-quality-and-colour` slice C in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written
// from the specification and the customer's finish-button change of 2026-10-30, and kept apart
// from `acceptance_0089_c_browser.test.ts`: this file covers what that one does not (Drop border
// shapes in effect, Enter creates, Stop and failure lead to Retry, the 400 ms / 300 ms timing
// against the job's own start, the 8-swatch limit, Esc order, remembered method, overlap checks
// at 800 x 600, hint text). Harness as in `acceptance_0025_slice3_tester.test.ts`.


import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5231;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-sliceC-tester");
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
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve) => {
      id += 1;
      pending.set(id, resolve);
      ws.send(JSON.stringify({ id, method, params }));
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

async function launch(preload?: string) {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
  await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
  const s = await session(profile);
  // UX notes 20.4 (2026-10-29): the counts are the Advanced block's last row; the block starts
  // open unless a test closed it (it is remembered).
  await s.send("Page.addScriptToEvaluateOnNewDocument", { source: "if (localStorage.getItem('curvyo.trace-advanced-open') === null) localStorage.setItem('curvyo.trace-advanced-open', '1');" });
  if (preload) {
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: preload });
  }
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  const close = async () => {
    await s.close();
    vite.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  };
  return { s, close };
}

/** A worker that dies at start while `window.__failWorker` is set (the real `onerror` path), and
 * whose trace answers arrive `window.__delay` ms late: the page-side stand-in for a slow machine. */
const WORKER_FAULT = `(() => {
  const Real = window.Worker;
  window.__failWorker = false;
  window.__delay = 0;
  window.__workers = 0;
  const set = Object.getOwnPropertyDescriptor(Real.prototype, 'onmessage').set;
  window.__jobs = [];
  window.Worker = class extends Real {
    postMessage(message, transfer) {
      if (message && message.type === 'trace') window.__jobs.push({ id: message.id, start: performance.now(), w: message.width, h: message.height });
      return super.postMessage(message, transfer);
    }
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
        if (event.data && event.data.kind === 'trace') { const j = window.__jobs.find((x) => x.id === event.data.id); if (j) j.end = performance.now(); }
        if (window.__delay > 0 && event.data && event.data.kind === 'trace') setTimeout(() => fn(event), window.__delay);
        else fn(event);
      });
    }
    constructor(url, options) {
      window.__workers += 1;
      if (window.__failWorker) {
        super(URL.createObjectURL(new Blob(['throw new Error("boom")'], { type: 'text/javascript' })), options);
      } else {
        super(url, options);
      }
    }
  };
})();`;


const BAR = "[role=toolbar][aria-label=Trace]";
type S = Awaited<ReturnType<typeof session>>;

const press = async (s: S, k: string, vk: number, modifiers = 0) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await sleep(350);
};
const clickAt = async (s: S, x: number, y: number, modifiers = 0) => {
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, modifiers });
  }
  await sleep(500);
};
const focusCanvas = (s: S) => s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
const canvasRect = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as [number, number, number, number];
const chip = async (s: S) => (await s.evaluate(`document.querySelector('[data-colour-chip]')?.textContent ?? ''`)) as string;
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const counts = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-counts]')?.textContent ?? ''`)) as string;
const pathsNow = async (s: S, n: number) =>
  waitFor(`${n} paths`, async () => {
    const t = await counts(s);
    return new RegExp(`^${n} paths?,`).test(t) ? t : null;
  }, 15_000).catch(async (e) => {
    throw new Error(`${e.message}; counts "${await counts(s)}", status "${await s.statusText()}", bar "${(await s.evaluate(`document.querySelector('${BAR}')?.innerText ?? ''`)) as string}"`);
  });
const slotKind = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-slot]')?.getAttribute('data-trace-slot') ?? 'gone'`)) as string;
const pressed = async (s: S, sel: string) => (await s.evaluate(`document.querySelector('${BAR} ${sel}')?.getAttribute('aria-pressed') ?? 'none'`)) as string;
const swatches = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-swatch]')].map((e) => e.getAttribute('data-trace-swatch')))`)) as string) as string[];
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const fieldValue = async (s: S, name: string) =>
  (await s.evaluate(`(document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]')?.getAttribute('aria-valuetext') ?? '').split(' ')[0]`)) as string;

/** Imports `png` as `name` and opens the Trace bar on it (Advanced open, Layers tab). */
async function openOn(s: S, name: string, png: Buffer, failWorker = false) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  const before = (await s.layerRows()).length;
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).length > before ? true : null));
  if (failWorker) await s.evaluate(`window.__failWorker = true`);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
}

/** A white 200 x 200 picture with a black bar on the left border (20 px wide, touching it) and a
 * black 40 px square in the middle: two shapes, one touches the border, white is most of the border. */
const FRAME = rgbPng(200, 200, (x, y) => {
  const bar = x < 20 && y >= 60 && y < 140;
  const square = x >= 80 && x < 120 && y >= 80 && y < 120;
  return bar || square ? [0, 0, 0] : [255, 255, 255];
});

/** Nine distinct colour blocks on white (40 px wide, 10 px apart) across the middle. */
const NINE = ["#FF0000", "#00AA00", "#0000FF", "#FFAA00", "#AA00AA", "#00AAAA", "#888800", "#444444", "#FF77AA"];
const NINE_PNG = rgbPng(480, 100, (x, y) => {
  const i = Math.floor((x - 10) / 50);
  if (y < 20 || y >= 80 || x < 10 || i < 0 || i > 8 || (x - 10) % 50 >= 40) return [255, 255, 255];
  const hex = NINE[i];
  return [1, 3, 5].map((k) => parseInt(hex.slice(k, k + 2), 16)) as [number, number, number];
});

/** Scans the canvas row with the pipette armed; returns the screen x of each colour's chip. */
async function scanColours(s: S): Promise<Map<string, number>> {
  const [cx, cy, cw, ch] = await canvasRect(s);
  const found = new Map<string, number>();
  for (let x = cx + 20; x < cx + cw - 20; x += 4) {
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y: cy + ch / 2 });
    await sleep(40);
    const hex = (await chip(s)).match(/#[0-9A-F]{6}/)?.[0];
    if (hex && !found.has(hex)) found.set(hex, x + 6);
  }
  return found;
}

const withApp = async (fn: (s: S) => Promise<void>, width = 1280, height = 800) => {
  const { s, close } = await launch(WORKER_FAULT);
  try {
    await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await sleep(500);
    await fn(s);
  } finally {
    await close();
  }
};

// ------------------------------------------------------------------ Drop border shapes, Enter

test("0089C tester: Drop border shapes drops the shape on the border in both colour-free methods; Enter creates", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "frame.png", FRAME);
    await pathsNow(s, 2);
    const toggle = `button[aria-label="Drop border shapes"]`;
    assert.equal(await pressed(s, toggle), "false");
    await s.clickSel(`${BAR} ${toggle}`);
    await pathsNow(s, 1);
    assert.equal(await pressed(s, toggle), "true");
    await s.clickSel(`${BAR} ${toggle}`);
    await pathsNow(s, 2);
    // Remove background: the same toggle, the same effect (Auto takes the white border).
    await pickMethod(s, "Remove background");
    await pathsNow(s, 2);
    await s.clickSel(`${BAR} ${toggle}`);
    await pathsNow(s, 1);
    // The old name is gone from the bar.
    assert.equal(await s.evaluate(`/Ignore background/i.test(document.querySelector('${BAR}').innerHTML)`), false);
    // Enter creates the paths: one more layer row, one history step.
    const rowsBefore = (await s.layerRows()).length;
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await waitFor("created paths", async () => ((await s.layerRows()).length > rowsBefore ? true : null), 20_000);
    await sleep(500);
    const history = await (async () => {
      await s.tab("History");
      const t = (await s.evaluate(`document.querySelector('[role=tabpanel]')?.innerText ?? ''`)) as string;
      await s.tab("Layers");
      return t;
    })();
    console.log("history after Enter:", JSON.stringify(history.slice(0, 200)));
    const rows = await s.layerRows();
    console.log("layer rows after Enter:", JSON.stringify(rows));
  });
});

// ------------------------------------------------------------------ the finish button

/** Records every change of the slot with the time, and the time of each worker job's start. */
const WATCH = `window.__slots = []; window.__obs?.disconnect(); window.__obs = new MutationObserver(() => {
  const e = document.querySelector('${BAR} [data-trace-slot]'); const k = e ? e.getAttribute('data-trace-slot') : 'gone';
  if (window.__slots.at(-1)?.[0] !== k) window.__slots.push([k, performance.now()]);
}); window.__obs.observe(document.querySelector('${BAR}'), { subtree: true, childList: true, attributes: true }); window.__jobs.length = 0;
  cancelAnimationFrame(window.__raf); window.__frames = []; (function f() { if (document.querySelector('${BAR} [data-trace-slot=running]')) window.__frames.push(performance.now()); window.__raf = requestAnimationFrame(f); })();`;

type Slots = [string, number][];
/** Nudges the Threshold to a new value (a new job), waits, and returns the slot history. */
async function jobWithDelay(s: S, delay: number, value: string): Promise<{ slots: Slots; start: number | null; frames: number[] }> {
  await s.evaluate(WATCH);
  await s.evaluate(`window.__delay = ${delay}`);
  await typeInto(s, "Threshold", value);
  await sleep(delay + 1500);
  const slots = JSON.parse((await s.evaluate(`JSON.stringify(window.__slots)`)) as string) as Slots;
  const start = (await s.evaluate(`window.__jobs.length ? window.__jobs[window.__jobs.length - 1].start : null`)) as number | null;
  const frames = JSON.parse((await s.evaluate(`JSON.stringify(window.__frames)`)) as string) as number[];
  await s.evaluate(`window.__delay = 0`);
  return { slots, start, frames };
}
const runningSpans = (slots: Slots) => slots.flatMap(([k, t], i) => (k === "running" ? [{ at: t, ms: (slots[i + 1]?.[1] ?? t) - t }] : []));

/** Jobs that end just after the 400 ms mark: when the progress first shows (ms after the job's
 * start) and for how long it stays (MutationObserver and animation-frame readings). */
async function shortOverTrials(s: S) {
  const heldMs: number[] = [];
  const heldFrames: number[] = [];
  const firstShown: number[] = [];
  for (const [delay, value] of [[380, "45"], [400, "75"], [420, "55"], [450, "65"], [330, "35"]] as [number, string][]) {
    const over = await jobWithDelay(s, delay, value);
    for (const sp of runningSpans(over.slots)) {
      heldMs.push(Math.round(sp.ms));
      firstShown.push(Math.round(sp.at - (over.start ?? 0)));
    }
    if (over.frames.length) heldFrames.push(Math.round(over.frames[over.frames.length - 1] - over.frames[0] + 16.7));
  }
  return { heldMs, heldFrames, firstShown };
}

test("0089C tester: the Trace button is blue, 75 x 28, reads Trace; progress after 400 ms of the job, held for 300 ms", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "frame.png", FRAME);
    await pathsNow(s, 2);
    const look = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot=none]'); const r = b.getBoundingClientRect();
      return { tag: b.tagName, text: b.textContent.trim(), bg: getComputedStyle(b).backgroundColor, fg: getComputedStyle(b).color, w: r.width, h: r.height,
        accent: getComputedStyle(document.documentElement).getPropertyValue('--editor-accent').trim() }; })())`)) as string) as { tag: string; text: string; bg: string; fg: string; w: number; h: number; accent: string };
    console.log("finish button:", JSON.stringify(look));
    assert.equal(look.tag, "BUTTON");
    assert.equal(look.text, "Trace");
    assert.equal(look.fg, "rgb(255, 255, 255)");
    assert.deepEqual([Math.round(look.w), Math.round(look.h)], [75, 28]);
    const accent = await s.evaluate(`(() => { const e = document.createElement('i'); e.style.color = 'var(--editor-accent)'; document.body.appendChild(e); const c = getComputedStyle(e).color; e.remove(); return c; })()`);
    assert.equal(look.bg, accent, "filled with --editor-accent");

    // A short job (answer within the 400 ms): no running state at all, nothing else flickers.
    const quick = await jobWithDelay(s, 0, "60");
    assert.ok(!quick.slots.some(([k]) => k === "running" || k === "stopped" || k === "failed"), `no flash on a short job: ${JSON.stringify(quick.slots)}`);
    const mid = await jobWithDelay(s, 200, "40");
    assert.ok(!mid.slots.some(([k]) => k === "running"), `no flash on a 250 ms job: ${JSON.stringify(mid.slots)}`);

    // A job that ends just after the 400 ms mark: it may or may not show, but never before 400 ms
    // of the job (the hold of 300 ms is its own test below).
    const firstShown = (await shortOverTrials(s)).firstShown;
    console.log("jobs ending around 400 ms: first shown after (ms of job)", JSON.stringify(firstShown));
    assert.ok(firstShown.every((ms) => ms >= 380), `never shown before 400 ms of the job: ${JSON.stringify(firstShown)}`);

    // A long job: appears at about 400 ms, shows a percent and the x inside the same blue button.
    const long = await jobWithDelay(s, 1800, "50");
    const l = runningSpans(long.slots);
    console.log("long job:", JSON.stringify(l.map((x) => ({ afterStart: Math.round(x.at - (long.start ?? 0)), shownMs: Math.round(x.ms) }))));
    assert.ok(l.length >= 1, `progress is shown for a long job: ${JSON.stringify(long.slots)}`);
    assert.ok(l[0].at - (long.start ?? 0) >= 380 && l[0].at - (long.start ?? 0) <= 800, `appears after about 400 ms: ${l[0].at - (long.start ?? 0)}`);
    assert.ok(l[0].ms >= 290);
    // While it runs: same button, same shape, percent and x inside.
    await s.evaluate(`window.__delay = 4000`);
    await typeInto(s, "Threshold", "80");
    await waitFor("running", async () => ((await slotKind(s)) === "running" ? true : null), 5000);
    const run = JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('${BAR} [data-trace-slot=running]'); const r = e.getBoundingClientRect();
      return { bg: getComputedStyle(e).backgroundColor, w: Math.round(r.width), h: Math.round(r.height), x: !!e.querySelector('button[aria-label="Stop (Esc)"]'), text: e.textContent.trim() }; })())`)) as string) as { bg: string; w: number; h: number; x: boolean; text: string };
    console.log("running:", JSON.stringify(run));
    assert.deepEqual([run.bg, run.w, run.h, run.x], [look.bg, 75, 28, true]);
    assert.match(run.text, /\d+\s*%|^…|^\.\.\./, "a percentage (or the first report pending)");
    // Stop leads to Stopped + Retry; Retry traces again and the button returns to Trace.
    await s.clickSel(`${BAR} [data-trace-slot=running] button[aria-label="Stop (Esc)"]`);
    await waitFor("Stopped", async () => ((await slotKind(s)) === "stopped" ? true : null), 5000);
    const stopped = (await s.evaluate(`document.querySelector('${BAR} [data-trace-slot=stopped]').innerText`)) as string;
    assert.match(stopped, /Stopped/);
    assert.match(stopped, /Retry/);
    assert.equal(await s.evaluate(`getComputedStyle(document.querySelector('${BAR} [data-trace-slot=stopped]')).backgroundColor`), look.bg, "Stopped stays blue");
    await s.evaluate(`window.__delay = 0`);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-slot=stopped] button')].find((b) => b.textContent === 'Retry').click()`);
    await waitFor("back to Trace", async () => ((await slotKind(s)) === "none" ? true : null), 20_000);
    await pathsNow(s, 2);
  });
});

test("0089C tester: once shown, the progress stays at least 300 ms (customer 2026-10-30)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "frame.png", FRAME);
    await pathsNow(s, 2);
    const { heldMs, heldFrames } = await shortOverTrials(s);
    console.log("progress visible for (ms): MutationObserver", JSON.stringify(heldMs), "animation frames", JSON.stringify(heldFrames));
    assert.ok(heldMs.length > 0, "at least one job crossed 400 ms");
    assert.ok(heldMs.every((ms) => ms >= 290), `held at least 300 ms: ${JSON.stringify(heldMs)}`);
  });
});

test("0089C tester: a failed job shows Failed with Retry; Retry recovers; Enter and a click on Trace both create", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "frame.png", FRAME, true);
    await waitFor("Failed", async () => ((await slotKind(s)) === "failed" ? true : null), 20_000);
    const failed = (await s.evaluate(`document.querySelector('${BAR} [data-trace-slot=failed]').innerText`)) as string;
    assert.match(failed, /Failed/);
    assert.match(failed, /Retry/);
    const bg = await s.evaluate(`getComputedStyle(document.querySelector('${BAR} [data-trace-slot=failed]')).backgroundColor`);
    assert.equal(bg, "rgb(47, 111, 238)", "Failed stays in the blue shape");
    // Enter while Failed must not pretend to create from a failed preview: report what happens.
    const rowsFailed = (await s.layerRows()).length;
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await sleep(800);
    console.log("Enter while Failed: rows", rowsFailed, "->", (await s.layerRows()).length, "slot", await slotKind(s));
    await s.evaluate(`window.__failWorker = false`);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-slot=failed] button')].find((b) => b.textContent === 'Retry').click()`);
    await waitFor("recovered", async () => ((await slotKind(s)) === "none" ? true : null), 20_000);
    await pathsNow(s, 2);
    // A click on the blue Trace button creates, like Enter.
    const before = (await s.layerRows()).length;
    await s.clickSel(`${BAR} [data-trace-slot=none]`);
    await waitFor("created by click", async () => ((await s.layerRows()).length > before ? true : null), 20_000);
  });
});

// ------------------------------------------------------------------ the pipette (Frage 89 A)

test("0089C tester: the pipette in Pick colours: armed, Shift+click adds up to 8, a plain click replaces, Esc order, method change disarms", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "nine.png", NINE_PNG);
    await pickMethod(s, "Pick colours");
    const pipette = "[data-trace-pipette]";
    assert.equal(await pressed(s, pipette), "true", "armed on entering Pick colours");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} ${pipette}').getAttribute('aria-label')`), "Pick colour from the image");
    const found = await scanColours(s);
    for (const hex of NINE) assert.ok(found.has(hex), `the chip shows ${hex}; saw ${[...found.keys()]}`);
    const [, cy, , ch] = await canvasRect(s);
    const y = cy + ch / 2;
    await clickAt(s, found.get(NINE[0])!, y);
    assert.deepEqual(await swatches(s), [NINE[0]]);
    for (let i = 1; i < 8; i++) await clickAt(s, found.get(NINE[i])!, y, 8);
    assert.deepEqual(await swatches(s), NINE.slice(0, 8), "Shift+click adds up to 8");
    assert.equal(await pressed(s, pipette), "true", "still armed after the picks");
    // The ninth: nothing is added and nothing breaks; the pipette stays armed. UX notes 5
    // (Proposal) also want the words "8 colours picked" in the status: watched, reported, not asserted.
    await s.evaluate(`window.__seen8 = false; window.__w8 = setInterval(() => { if (/8 colours picked/i.test(document.body.innerText)) window.__seen8 = true; }, 25)`);
    await clickAt(s, found.get(NINE[8])!, y, 8);
    await sleep(1500);
    console.log("UX notes 5 status '8 colours picked' seen after the ninth pick:", await s.evaluate(`clearInterval(window.__w8), window.__seen8`));
    assert.deepEqual(await swatches(s), NINE.slice(0, 8), "the ninth pick adds nothing");
    assert.equal(await pressed(s, pipette), "true");
    // The same colour twice is not added twice.
    await clickAt(s, found.get(NINE[0])!, y, 8);
    assert.equal((await swatches(s)).filter((h) => h === NINE[0]).length, 1);
    // A click outside the image does nothing (pasteboard).
    const [cx] = await canvasRect(s);
    await clickAt(s, cx + 6, cy + 6);
    assert.equal((await swatches(s)).length, 8, "a click off the image keeps the swatches");
    // A plain click replaces all with one.
    await clickAt(s, found.get(NINE[3])!, y);
    assert.deepEqual(await swatches(s), [NINE[3]]);
    await pathsNow(s, 1);
    // Esc order: a running trace stops first, then the pipette disarms, then the tool is left.
    await s.evaluate(`window.__delay = 4000`);
    await clickAt(s, found.get(NINE[1])!, y, 8);
    await waitFor("running", async () => ((await slotKind(s)) === "running" ? true : null), 8000);
    await focusCanvas(s);
    await press(s, "Escape", 27);
    await waitFor("stopped", async () => ((await slotKind(s)) === "stopped" ? true : null), 8000);
    assert.equal(await pressed(s, pipette), "true", "the first Esc only stops the trace");
    await s.evaluate(`window.__delay = 0`);
    await focusCanvas(s);
    await press(s, "Escape", 27);
    assert.equal(await pressed(s, pipette), "false", "the second Esc disarms");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR}')`), true, "the tool is still open");
    // Arm again by the button, then choosing another method disarms it.
    await s.clickSel(`${BAR} ${pipette}`);
    assert.equal(await pressed(s, pipette), "true");
    await pickMethod(s, "Brightness");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} ${pipette}')`), false, "no pipette in Brightness");
    await pickMethod(s, "Pick colours");
    // UX notes 5 arm the pipette on entering when nothing is picked; with a swatch present it is
    // reported, not required.
    console.log("pipette armed on re-entering Pick colours with a swatch present:", await pressed(s, pipette));
    if ((await pressed(s, pipette)) !== "true") await s.clickSel(`${BAR} ${pipette}`);
    assert.equal(await pressed(s, pipette), "true");
    // The third Esc leaves the tool.
    await focusCanvas(s);
    await press(s, "Escape", 27);
    await focusCanvas(s);
    await press(s, "Escape", 27);
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR}')`), false, "Esc after the pipette is off leaves the tool");
  });
});

test("0089C tester: Remove background takes exactly one pick, with or without Shift", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "nine.png", NINE_PNG);
    await pickMethod(s, "Remove background");
    const pipette = "[data-trace-pipette]";
    assert.equal(await pressed(s, pipette), "false", "not armed on entering Remove background");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} ${pipette}').getAttribute('aria-label')`), "Pick background colour from the image");
    await s.clickSel(`${BAR} ${pipette}`);
    assert.equal(await pressed(s, pipette), "true");
    const found = await scanColours(s);
    const [, cy, , ch] = await canvasRect(s);
    const bgText = () => s.evaluate(`document.querySelector('${BAR} [data-trace-background]').textContent`);
    assert.equal(await bgText(), "#FFFFFF");
    await clickAt(s, found.get(NINE[2])!, cy + ch / 2);
    assert.equal(await bgText(), NINE[2]);
    assert.equal(await pressed(s, pipette), "false", "disarmed after one pick");
    // A further click on the canvas is not a pick.
    await clickAt(s, found.get(NINE[4])!, cy + ch / 2);
    assert.equal(await bgText(), NINE[2]);
    // With Shift: still one pick.
    await s.clickSel(`${BAR} ${pipette}`);
    await clickAt(s, found.get(NINE[5])!, cy + ch / 2, 8);
    assert.equal(await bgText(), NINE[5]);
    assert.equal(await pressed(s, pipette), "false", "Shift does not keep it armed in Remove background");
    // Esc ends an armed pipette without picking.
    await s.clickSel(`${BAR} ${pipette}`);
    await focusCanvas(s);
    await press(s, "Escape", 27);
    assert.equal(await pressed(s, pipette), "false");
    assert.equal(await bgText(), NINE[5]);
    // Auto returns to the detected border colour.
    await s.evaluate(`[...document.querySelectorAll('${BAR} button[aria-pressed]')].find((b) => b.textContent === 'Auto').click()`);
    await sleep(600);
    assert.equal(await bgText(), "#FFFFFF");
  });
});

// ------------------------------------------------------------------ remembered settings

test("0089C tester: the method and its tolerance are remembered, picked colours are not", { skip: !BROWSER, timeout: 400_000 }, async () => {
  // A reload would stop at the unsaved-changes dialog; the page here ignores beforeunload.
  const { s, close } = await launch(WORKER_FAULT + `;(() => { const add = EventTarget.prototype.addEventListener; EventTarget.prototype.addEventListener = function (t, ...r) { if (t === "beforeunload") return undefined; return add.call(this, t, ...r); }; window.onbeforeunload = null; })();`);
  try {
    await s.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
    await openOn(s, "nine.png", NINE_PNG);
    await pickMethod(s, "Pick colours");
    const found = await scanColours(s);
    const [, cy, , ch] = await canvasRect(s);
    await clickAt(s, found.get(NINE[0])!, cy + ch / 2);
    assert.deepEqual(await swatches(s), [NINE[0]]);
    console.log("step: picked");
    await typeInto(s, "Colour tolerance", "47");
    console.log("step: typed");
    assert.equal(await fieldValue(s, "Colour tolerance"), "47");
    await sleep(800);
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    console.log("step: reloaded");
    await openOn(s, "nine.png", NINE_PNG);
    console.log("step: reopened");
    const method = (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').textContent`)) as string;
    console.log("after reload: method", JSON.stringify(method), "tolerance", await fieldValue(s, "Colour tolerance"), "swatches", JSON.stringify(await swatches(s)));
    assert.match(method, /Pick colours/, "the method is remembered");
    assert.equal(await fieldValue(s, "Colour tolerance"), "47", "the colour tolerance is remembered");
    assert.deepEqual(await swatches(s), [], "picked colours belong to the session of the tool");
    // Each method keeps its own value: Remove background has not taken the 47.
    await pickMethod(s, "Remove background");
    console.log("Remove background tolerance:", await fieldValue(s, "Colour tolerance"));
    assert.notEqual(await fieldValue(s, "Colour tolerance"), "47");
  } finally {
    await close();
  }
});

// ------------------------------------------------------------------ layout at 800 x 600, hint text

const HINT = /\b(click|press|drag|tap|hold|try|use|tip|hint|you can|choose|select a|to (pick|add|remove|create))\b/i;

test("0089C tester: at 800 x 600 nothing overlaps, clips or leaves the window; row heights as built; no hint text", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "nine.png", NINE_PNG);
    const measure = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => {
        const bar = document.querySelector('${BAR}'); const box = bar.getBoundingClientRect();
        const vis = (e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden'; };
        const items = [...bar.querySelectorAll('button, [role=spinbutton], [role=radiogroup], [role=listbox]')].filter(vis)
          .filter((e) => !e.parentElement.closest('button, [role=spinbutton], [role=radiogroup], [role=listbox]'));
        const name = (e) => e.getAttribute('aria-label') || e.textContent.trim().slice(0, 20);
        const overlaps = [];
        for (let i = 0; i < items.length; i++) for (let j = i + 1; j < items.length; j++) {
          const a = items[i].getBoundingClientRect(); const b = items[j].getBoundingClientRect();
          const w = Math.min(a.right, b.right) - Math.max(a.left, b.left); const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
          if (w > 1 && h > 1) overlaps.push(name(items[i]) + ' x ' + name(items[j]));
        }
        const outside = items.filter((e) => { const r = e.getBoundingClientRect(); return r.left < box.left - 0.5 || r.right > box.right + 0.5 || r.left < 0 || r.right > innerWidth + 0.5 || r.top < 0 || r.bottom > innerHeight + 0.5; }).map(name);
        const clipped = [...bar.querySelectorAll('button, [role=spinbutton], span, label')].filter(vis).filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow !== 'visible' && e.clientWidth > 0).map(name);
        const texts = [...bar.querySelectorAll('*')].filter((e) => e.children.length === 0 && vis(e)).map((e) => e.textContent.trim()).filter(Boolean);
        const labels = items.map((e) => e.getAttribute('aria-label') || '').filter(Boolean);
        const slot = bar.querySelector('[data-trace-slot]').getBoundingClientRect();
        const lastBottom = Math.max(...items.map((e) => e.getBoundingClientRect().bottom));
        return { overlaps, outside, clipped, texts, labels, height: Math.round(box.height), top: Math.round(box.top), bottom: Math.round(box.bottom), vh: innerHeight, vw: innerWidth,
          slotRightGap: Math.round(box.right - slot.right), insideBar: lastBottom <= box.bottom + 0.5 }; })())`)) as string) as {
        overlaps: string[]; outside: string[]; clipped: string[]; texts: string[]; labels: string[]; height: number; top: number; bottom: number; vh: number; vw: number; slotRightGap: number; insideBar: boolean };
    // The as-built heights of ux-notes.md ("Slice C as built"), Advanced closed, no picks.
    const asBuilt: Record<string, [number, number]> = { Brightness: [252, 108], "Remove background": [288, 108], "Pick colours": [288, 144] };
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Fewer trace options"]')?.click()`);
    await sleep(400);
    const all: string[] = [];
    const hints: string[] = [];
    for (const method of ["Brightness", "Remove background", "Pick colours"]) {
      await pickMethod(s, method);
      if (method === "Pick colours") {
        await focusCanvas(s);
        await press(s, "Escape", 27);
      }
      for (const open of [true, false]) {
        const panelOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
        if (panelOpen !== open) {
          await s.clickSel(`[aria-label="${panelOpen ? "Hide panel" : "Show panel"}"]`);
          await sleep(700);
        }
        const m = await measure();
        const tag = `${method}, panel ${open ? "open" : "closed"}`;
        all.push(`${tag}: bar ${m.height} px high at y ${m.top}..${m.bottom} of ${m.vh}, finish button ${m.slotRightGap} px from the right edge`);
        assert.deepEqual(m.overlaps, [], `${tag}: overlapping controls`);
        assert.deepEqual(m.outside, [], `${tag}: controls outside the bar or window`);
        assert.deepEqual(m.clipped, [], `${tag}: clipped text`);
        assert.ok(m.insideBar, `${tag}: every control inside the bar's box`);
        assert.ok(m.top >= 0 && m.bottom <= m.vh, `${tag}: the bar is inside the window`);
        assert.ok(Math.abs(m.slotRightGap - 8) <= 2, `${tag}: finish button at the right edge (gap ${m.slotRightGap})`);
        const expected = asBuilt[method][open ? 0 : 1];
        assert.ok(Math.abs(m.height - expected) <= 4, `${tag}: bar height ${m.height} px, notes say ${expected} px`);
        for (const t of [...m.texts, ...m.labels]) if (HINT.test(t)) hints.push(`${tag}: "${t}"`);
        await s.send("Page.captureScreenshot", { format: "png" }).then((r) => writeFileSync(path.join(SCRATCH, `tester-${method.replace(/ /g, "-")}-${open ? "open" : "closed"}.png`), Buffer.from(((r as { result?: { data?: string } }).result?.data) ?? "", "base64")));
      }
    }
    console.log(all.join("\n"));
    console.log("hint-like strings:", JSON.stringify([...new Set(hints)]));
    assert.deepEqual([...new Set(hints)], [], "no operating hint text in the bar");
  }, 800, 600);
});

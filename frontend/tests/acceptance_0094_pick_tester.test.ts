// Tester cases for `specs/0094-masked-composite-trace` slice 1b, Pick mask paths (criteria 34 to 41
// and 43; criterion 42 needs slice 2), written from the specification and UX notes section 11 before
// the implementation was read. Chromium via DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium.
// Harness as in `acceptance_0094_tester.test.ts`; page-side wasm imports use `?own-instance`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5396;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0094-pick-tester");
const realSleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
/** Scales every fixed pause; a case that only needs the events to land can shorten them. */
let pace = 1;
const sleep = (ms: number) => realSleep(ms * pace);

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 60_000): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch(() => null);
    if (got !== null) {
      return got;
    }
    await realSleep(200);
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

/** Decodes an 8-bit RGB or RGBA non-interlaced PNG (a DevTools screenshot) to RGBA. */
function decodePng(buf: Buffer): { width: number; height: number; data: Uint8Array } {
  let at = 8;
  let width = 0;
  let height = 0;
  let channels = 4;
  const idat: Buffer[] = [];
  while (at < buf.length) {
    const len = buf.readUInt32BE(at);
    const kind = buf.toString("latin1", at + 4, at + 8);
    const body = buf.subarray(at + 8, at + 8 + len);
    if (kind === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      channels = body[9] === 6 ? 4 : 3;
    } else if (kind === "IDAT") {
      idat.push(body);
    }
    at += 12 + len;
  }
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * channels;
  const out = new Uint8Array(width * height * 4);
  const prev = new Uint8Array(stride);
  const cur = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    for (let i = 0; i < stride; i++) {
      const x = raw[y * (stride + 1) + 1 + i];
      const a = i >= channels ? cur[i - channels] : 0;
      const b = prev[i];
      const c = i >= channels ? prev[i - channels] : 0;
      let v = x;
      if (filter === 1) v = x + a;
      else if (filter === 2) v = x + b;
      else if (filter === 3) v = x + ((a + b) >> 1);
      else if (filter === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        v = x + (pa <= pb && pa <= pc ? a : pb <= pc ? b : c);
      }
      cur[i] = v & 255;
    }
    for (let px = 0; px < width; px++) {
      out[(y * width + px) * 4] = cur[px * channels];
      out[(y * width + px) * 4 + 1] = cur[px * channels + 1];
      out[(y * width + px) * 4 + 2] = cur[px * channels + 2];
      out[(y * width + px) * 4 + 3] = channels === 4 ? cur[px * channels + 3] : 255;
    }
    prev.set(cur);
  }
  return { width, height, data: out };
}

async function launch(preload?: string) {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = startVite(PORT, FRONTEND);
  await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
  const s = await session(profile);
  // The Advanced block starts open unless a test closed it (it is remembered).
  await s.send("Page.addScriptToEvaluateOnNewDocument", { source: "if (localStorage.getItem('curvyo.trace-advanced-open') === null) localStorage.setItem('curvyo.trace-advanced-open', '1');" });
  if (preload) {
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: preload });
  }
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  const close = async () => {
    await s.close();
    await stopVite(vite, PORT);
    removeProfile(profile);
  };
  return { s, close };
}

const BAR = "[role=toolbar][aria-label=Trace]";
type S = Awaited<ReturnType<typeof session>>;

const press = async (s: S, k: string, vk: number, modifiers = 0) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await sleep(350);
};
const canvasRect = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as [number, number, number, number];
const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
type Shot = ReturnType<typeof decodePng>;
const at = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};

const ADV = `${BAR} [role=group][aria-label=Advanced]`;
const EXPANDER = `${BAR} button[aria-expanded][aria-controls]`;
const HINT = /\b(click|press|drag|tap|hold|try|use|tip|hint|you can|choose|select a|to (pick|add|remove|create))\b/i;

async function setAdvanced(s: S, open: boolean) {
  const now = (await s.evaluate(`document.querySelector('${EXPANDER}')?.getAttribute('aria-expanded')`)) === "true";
  if (now !== open) {
    await s.evaluate(`document.querySelector('${EXPANDER}').click()`);
    await sleep(500);
  }
}

const ERR_HOOK = "window.__errs = []; window.addEventListener('error', (e) => window.__errs.push(String(e.message) + ' ' + (e.error && e.error.stack || ''))); window.addEventListener('unhandledrejection', (e) => window.__errs.push('rej ' + String(e.reason && e.reason.stack || e.reason)));";
const withApp = async (fn: (s: S) => Promise<void>, preload = ERR_HOOK, width = 1280, height = 800) => {
  const { s, close } = await launch(preload);
  try {
    await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await sleep(500);
    await fn(s);
  } finally {
    await close();
  }
};

// ------------------------------------------------------------------ helpers

const focusCanvas = (s: S) => s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);

/** The named controls of the bar outside the Advanced block, in document order. */
const barControls = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelector('${BAR}').querySelectorAll('button, [role=spinbutton], [role=radiogroup]')]
    .filter((e) => !e.closest('[role=group][aria-label=Advanced]') && (e.matches('[role=radiogroup]') || !e.closest('[role=radiogroup]')))
    .map((e) => e.getAttribute('aria-label') || e.textContent.trim()))`)) as string) as string[];

/** Every user-visible string of the bar and its Advanced block: labels, titles and text. */
const barStrings = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...new Set([...document.querySelector('${BAR}').querySelectorAll('*')].flatMap((e) =>
    [e.getAttribute('aria-label'), e.getAttribute('title'), e.children.length === 0 ? e.textContent.trim() : ''].filter(Boolean)))])`)) as string) as string[];

const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${ADV} [data-trace-counts]')?.textContent ?? ''`)) as string;
const settledCounts = (s: S) => waitFor("a trace result", async () => (/^(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);


// ------------------------------------------------------------------ 0094 fixtures and helpers
/** M1 of the spec at `n` px (the tester's `support_0094` arithmetic, pixel centres in M1's own 1000 px frame). */
function m1Ink(u: number, v: number): "paper" | "disc" | "square" | "bar" | "grey" {
  if (Math.abs(u - 900) <= 50 && Math.abs(v - 100) <= 50) return "grey";
  if ((u - 250) * (u - 250) + (v - 500) * (v - 500) <= 150 * 150) return "disc";
  if (Math.abs(u - 750) <= 150 && Math.abs(v - 500) <= 150) return "square";
  if (v >= 480 && v <= 520) return "bar";
  return "paper";
}
const M1_PNG = rgbPng(400, 400, (x, y) => {
  const ink = m1Ink((x + 0.5) * 2.5, (y + 0.5) * 2.5);
  return ink === "paper" ? [255, 255, 255] : ink === "grey" ? [128, 128, 128] : [0, 0, 0];
});

/** The screen box of the picture: the bar of M1 is the longest dark horizontal run on screen and
 * spans the picture's full width at its middle; the picture is square. */
async function pictureBox(s: S) {
  const img = await shot(s);
  const [cx0, , cw] = await canvasRect(s);
  const right = Math.min(img.width, cx0 + cw) - 1;
  let best = { len: 0, x0: 0, x1: 0, y: 0 };
  for (let y = 0; y < img.height; y++) {
    // A run may have gaps of up to 6 px (a selection mark drawn over the picture).
    let start = -1;
    let last = -100;
    for (let x = 0; x < img.width; x++) {
      const [r, g, b] = at(img, x, y);
      if (r < 40 && g < 40 && b < 40) {
        if (x - last > 7) start = x;
        last = x;
        if (x - start + 1 > best.len) best = { len: x - start + 1, x0: start, x1: x, y };
      }
    }
  }
  // The run's end can stop at a mark drawn over the picture; its right edge is the row's last dark pixel.
  for (let x = right; x > best.x1; x--) {
    const [r, g, b] = at(img, x, best.y);
    if (r < 40 && g < 40 && b < 40) {
      best.x1 = x;
      break;
    }
  }
  const w = best.x1 - best.x0;
  return { x0: best.x0, x1: best.x1, y0: best.y - w / 2, y1: best.y + w / 2, cx: (best.x0 + best.x1) / 2, cy: best.y };
}

async function drag(s: S, from: [number, number], to: [number, number]) {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: from[0], y: from[1] });
  await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: from[0], y: from[1], button: "left", buttons: 1, clickCount: 1 });
  const steps = pace < 1 ? 3 : 8;
  for (let i = 1; i <= steps; i++) {
    const x = from[0] + ((to[0] - from[0]) * i) / steps;
    const y = from[1] + ((to[1] - from[1]) * i) / steps;
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "left", buttons: 1 });
  }
  await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: to[0], y: to[1], button: "left", buttons: 0, clickCount: 1 });
  await sleep(500);
}

const shiftClick = async (s: S, x: number, y: number) => {
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, modifiers: 8 });
  }
  await sleep(600);
};

const maskState = async (s: S) =>
  (await s.evaluate(`(() => { const g = document.querySelector('${BAR} [role=radiogroup][aria-label=Mask]'); if (!g) return 'absent';
    return [...g.querySelectorAll('[role=radio]')].map((r) => r.textContent.trim() + '=' + r.getAttribute('aria-checked')).join(','); })()`)) as string;
const advText = async (s: S) => (await s.evaluate(`document.querySelector('${ADV}')?.textContent ?? ''`)) as string;

// ------------------------------------------------------------------ 0094 slice 1b helpers
const PICK = `${BAR} button[aria-label="Pick mask paths"]`;
const CLEAR = `${BAR} button[aria-label="Clear mask"]`;
const NOTICE = "Use Object to path for text; one image at a time.";

const pickState = async (s: S) => (await s.evaluate(`document.querySelector('${PICK}')?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
const clearPresent = async (s: S) => (await s.evaluate(`!!document.querySelector('${CLEAR}')`)) as boolean;
const noticeShown = async (s: S) => (await s.evaluate(`document.querySelector('${BAR}').textContent.includes(${JSON.stringify(NOTICE)})`)) as boolean;
const arm = async (s: S) => {
  await s.evaluate(`document.querySelector('${PICK}').click()`);
  await sleep(300);
};
const selectedRows = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-level]')].filter((r) => r.getAttribute('aria-selected') === 'true').map((r) => r.getAttribute('aria-label')))`)) as string) as string[];
/** The panel's own selection count under Opacity ("3 objects"); the row list may be virtualised. */
const selectionCount = async (s: S) =>
  Number((await s.evaluate(`(() => { const all = [...document.querySelectorAll('*')].filter((e) => e.children.length === 0 && /^\\d+ objects?$/.test(e.textContent.trim())); return all.length ? all[all.length - 1].textContent.trim().split(' ')[0] : '-1'; })()`)) as string);
/** The first segment of the status text of the Advanced block, e.g. "Inside 2 paths". */
const maskSegment = async (s: S) => ((await advText(s)).match(/(Inside|Outside) \d+ paths?/) ?? [""])[0];
const clickAt = async (s: S, p: [number, number], modifiers = 0) => {
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x: p[0], y: p[1], button: "left", clickCount: 1, modifiers });
  }
  await sleep(500);
};

const pipette = `${BAR} button[aria-label^="Pick colour from the image"]`;
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
/** The bar's text without its sr-only polite status regions. */
const barText = async (s: S) =>
  (await s.evaluate(`(() => { const c = document.querySelector('${BAR}').cloneNode(true); c.querySelectorAll('[role=status], .sr-only').forEach((e) => e.remove()); return c.textContent; })()`)) as string;
const canvasCursor = async (s: S) =>
  (await s.evaluate(`(() => { const c = [...document.querySelectorAll('canvas')].map((e) => e.closest('[tabindex]')).find(Boolean); return c ? getComputedStyle(c).cursor : ''; })()`)) as string;
const historyText = async (s: S) => {
  await s.tab("History");
  // With one object selected the panel offers its own history too: read the document's.
  await s.evaluate(`[...document.querySelectorAll('[role=tabpanel][data-state=active] [role=radio], [role=tabpanel][data-state=active] button')].find((b) => b.textContent.trim() === 'Document')?.click()`);
  await sleep(300);
  const text = ((await s.evaluate(`document.querySelector('[role=tabpanel][data-state=active]')?.textContent ?? ''`)) as string).replace(/^DocumentObject/, "");
  await s.tab("Layers");
  return text;
};
/** Enter as the browser sends it to a focused button: a keyDown carrying the carriage return. */
const pressEnterOnButton = async (s: S) => {
  await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
  await sleep(350);
};
const activeName = async (s: S) =>
  (await s.evaluate(`(() => { const e = document.activeElement; return e ? (e.getAttribute('aria-label') || e.textContent.trim() || e.tagName) : ''; })()`)) as string;
/** Waits until the Advanced counts line stops changing for 1.2 s and returns it. */
async function stableCounts(s: S) {
  let last = "";
  let same = 0;
  const until = Date.now() + 60_000;
  while (Date.now() < until) {
    await realSleep(pace < 1 ? 400 : 600);
    const now = await advancedStatus(s);
    same = now === last && /\d+ paths?/.test(now) ? same + 1 : 0;
    last = now;
    if (same >= 2) {
      return now;
    }
  }
  throw new Error(`the counts never settled: ${last}`);
}

/** Clicks the Layers row named `label` (with Ctrl to add it to the selection). */
const rowClick = async (s: S, label: string, ctrl = false) => {
  const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('[role=row][aria-level]')].find((e) => e.getAttribute('aria-label') === ${JSON.stringify(label)}); if (!r) return null; const b = r.getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number] | null;
  assert.ok(c, `the row ${label} is listed`);
  await clickAt(s, c, ctrl ? 2 : 0);
};
/** Draws two rectangles and selects exactly them through the Layers rows. */
async function twoRects(s: S, a: [[number, number], [number, number]], b: [[number, number], [number, number]]) {
  const known = new Set(await s.layerRows());
  await drawWith(s, "r", a[0], a[1]);
  await drawWith(s, "r", b[0], b[1]);
  await toSelect(s);
  const fresh = (await s.layerRows()).filter((r) => r && !known.has(r));
  assert.equal(fresh.length, 2, `two new rows: ${JSON.stringify(fresh)}`);
  await rowClick(s, fresh[0]);
  await rowClick(s, fresh[1], true);
  assert.equal(await selectionCount(s), 2);
}

type Kind = "ellipse" | "polygon" | "star" | "path" | "rect" | "compound" | "group";
type Scene = { box: Awaited<ReturnType<typeof pictureBox>>; spots: Record<string, [number, number]>; imageAt: [number, number]; imageRow: string };

/** The Select tool, nothing selected. */
const toSelect = async (s: S) => {
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("s", 83);
};
const drawWith = async (s: S, tool: "e" | "r" | "polygon" | "star", from: [number, number], to: [number, number]) => {
  // Selection handles would take the press: select nothing first.
  await s.clickSel('button[aria-label^="Select tool"]');
  await clickAt(s, [900, 1000]);
  if (tool === "polygon" || tool === "star") {
    await s.clickSel('button[aria-label^="Polygon/star"]');
    await s.evaluate(`[...document.querySelectorAll('button[aria-pressed]')].find((b) => b.textContent.trim() === ${JSON.stringify(tool === "star" ? "Star" : "Polygon")}).click()`);
  } else {
    await s.key(tool, tool === "e" ? 69 : 82);
  }
  await drag(s, from, to);
};

/**
 * Imports M1 (and optionally a second picture, a sketch and a text), then draws `kinds` in
 * slots down the left edge of the picture, each reaching 40 px outside it and about 100 px in.
 * Ends in the Select tool with the picture selected. `spots[kind]` is a point on the object
 * outside the picture.
 */
async function buildScene(s: S, kinds: Kind[], extras: { second?: boolean; sketch?: boolean; text?: boolean } = {}): Promise<Scene> {
  await s.tab("Layers");
  const spots: Record<string, [number, number]> = {};
  if (extras.sketch) {
    await s.clickSel('button[aria-label^="Sketch"][aria-pressed]');
    await sleep(800);
    await s.clickSel('button[aria-label="Rectangle"]');
    await drag(s, [170, 820], [260, 880]);
    await s.clickSel('button[aria-label="Select"]');
    await s.clickSel('button[aria-label="Vector"]');
    await sleep(600);
    await s.tab("Layers");
    spots.sketch = [215, 850];
  }
  if (extras.second) {
    const file = path.join(SCRATCH, "second.png");
    writeFileSync(file, rgbPng(80, 80, () => [40, 40, 40]));
    await s.choose("[data-import-button]", file);
    await waitFor("the second image", async () => ((await s.layerRows()).some((r) => r?.includes("second.png")) ? true : null));
    const canvas = await canvasRect(s);
    await drag(s, [canvas[0] + canvas[2] / 2, canvas[1] + canvas[3] / 2], [210, 1000]);
    spots.second = [210, 1000];
  }
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const slot = (i: number) => box.y0 + 10 + 46 * i;
  const x0 = box.x0;
  let i = 0;
  for (const kind of kinds) {
    const y = slot(i);
    const yc = y + 17;
    i += 1;
    if (kind === "ellipse" || kind === "rect") {
      await drawWith(s, kind === "ellipse" ? "e" : "r", [x0 - 40, y], [x0 + 100, y + 36]);
      spots[kind] = [x0 - 40, yc];
    } else if (kind === "polygon" || kind === "star") {
      // These tools drag from the centre outwards.
      await drawWith(s, kind, [x0 + 60, yc], [x0 + 78, yc]);
      await toSelect(s);
      let found: [number, number] | null = null;
      for (let dx = x0 + 36; dx <= x0 + 66 && !found; dx += 2) {
        // A click inside the selected object keeps it, so deselect before each try.
        for (const [x, y] of [[900, 1000], [dx, yc]]) {
          await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
          for (const type of ["mousePressed", "mouseReleased"]) {
            await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1 });
          }
        }
        await sleep(250);
        if ((await selectedRows(s)).some((r) => r.toLowerCase().startsWith(kind))) {
          found = [dx, yc];
        }
      }
      assert.ok(found, `the outline of the ${kind} is hit on its centre line`);
      spots[kind] = found;
    } else if (kind === "path") {
      await press(s, "Escape", 27);
      await s.key("b", 66);
      await clickAt(s, [x0 - 40, yc]);
      await clickAt(s, [x0 + 100, yc]);
      await press(s, "Enter", 13);
      spots.path = [x0 - 20, yc];
    } else if (kind === "compound") {
      await twoRects(s, [[x0 - 40, y], [x0 + 20, y + 36]], [[x0 + 60, y], [x0 + 120, y + 36]]);
      await s.clickSel('button[aria-label="Combine"]');
      await sleep(500);
      spots.compound = [x0 - 40, yc];
    } else {
      await twoRects(s, [[x0 - 40, y], [x0 + 20, y + 36]], [[x0 + 60, y], [x0 + 120, y + 36]]);
      await s.clickSel('button[aria-label^="Group ("]');
      await sleep(500);
      spots.group = [x0 - 40, yc];
    }
  }
  if (extras.text) {
    await press(s, "Escape", 27);
    await s.key("t", 84);
    await clickAt(s, [box.x1 + 40, box.y0 + 60]);
    await s.send("Input.insertText", { text: "Hi" });
    await press(s, "Escape", 27);
    spots.text = [box.x1 + 50, box.y0 + 70];
  }
  await toSelect(s);
  return { box, spots, imageAt: [box.x1 - 40, box.cy], imageRow: "m1.png, image" };
}

/** Selects the picture alone and opens the Trace bar (Advanced open). */
async function enterTrace(s: S, scene: Scene) {
  await clickAt(s, [scene.box.cx + 100, scene.box.y0 + 330]);
  await waitFor("the picture alone", async () => (JSON.stringify(await selectedRows(s)) === JSON.stringify([scene.imageRow]) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 15_000);
  await sleep(1000);
  await setAdvanced(s, true);
}
const inTrace = async (s: S) => (await s.evaluate(`!!document.querySelector('${BAR}')`)) as boolean;

/** The Trace bar with Advanced open is 270 px tall and floats over the canvas; a taller window keeps the picture below it. */
const withTall = (fn: (s: S) => Promise<void>) => withApp(fn, ERR_HOOK, 1280, 1100);

// ------------------------------------------------------------------ cases

test("0094 c34, c35, c41, c43: the button follows View, toggles by press, Space and Enter, shows the cursor, says nothing in the bar", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"]);
    await enterTrace(s, scene);
    await setAdvanced(s, false);
    // c34
    assert.deepEqual(await barControls(s), ["About Trace", "Preset", "View", "Pick mask paths", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"]);
    assert.equal(await pickState(s), "false");
    assert.equal(await maskState(s), "absent", "no Inside / Outside segments");
    assert.equal(await clearPresent(s), false, "no Clear mask");
    assert.equal(await s.evaluate(`document.querySelector('${PICK}').getAttribute('aria-keyshortcuts')`), null, "no shortcut");
    const size = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = document.querySelector('${PICK}').getBoundingClientRect(); return [r.width, r.height]; })())`)) as string) as number[];
    assert.ok(size[0] >= 28 && size[1] >= 28, `target size ${size}`);
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: scene.box.cx, y: scene.box.cy });
    await sleep(300);
    const idle = await canvasCursor(s);
    const textIdle = await barText(s);
    // c35: press
    await s.clickSel(PICK);
    assert.equal(await pickState(s), "true");
    assert.match(await s.statusText(), /Pick mask paths on/, "polite status");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: scene.box.cx, y: scene.box.cy });
    await sleep(300);
    const armedCursor = await canvasCursor(s);
    assert.notEqual(armedCursor, idle, `the pick-mask cursor differs from the tool's own: ${armedCursor}`);
    assert.match(armedCursor, /url\(/, "a custom cursor image");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: scene.box.cx, y: scene.box.cy });
    assert.equal(await canvasCursor(s), armedCursor, "the same over the picture");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: scene.box.x1 + 150, y: scene.box.y1 + 100 });
    assert.equal(await canvasCursor(s), armedCursor, "the same over empty canvas");
    assert.equal(await barText(s), textIdle, "no text appears in the bar while armed");
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
    // second press
    await s.clickSel(PICK);
    assert.equal(await pickState(s), "false");
    assert.match(await s.statusText(), /Pick mask paths off/);
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: scene.box.cx, y: scene.box.cy });
    await sleep(300);
    assert.equal(await canvasCursor(s), idle, "the tool's own cursor again over the canvas");
    // Space and Enter toggle, the key press keeps focus on the button
    await s.evaluate(`document.querySelector('${PICK}').focus()`);
    await press(s, " ", 32);
    assert.equal(await pickState(s), "true", "Space arms");
    assert.equal(await activeName(s), "Pick mask paths", "focus stays after a key press");
    await pressEnterOnButton(s);
    assert.equal(await pickState(s), "false", "Enter disarms");
    await press(s, " ", 32);
    assert.equal(await pickState(s), "true");
    // Escape from the button disarms and returns focus to the canvas
    await press(s, "Escape", 27);
    assert.equal(await pickState(s), "false");
    assert.ok(await inTrace(s), "the tool stays");
    assert.notEqual(await activeName(s), "Pick mask paths", "focus left the button");
    // a mouse press returns focus to the canvas
    await s.clickSel(PICK);
    assert.notEqual(await activeName(s), "Pick mask paths", "a mouse press returns focus to the canvas");
    await s.clickSel(PICK);
    // Tab: View, then the button
    await s.evaluate(`(document.querySelector('${BAR} [role=radiogroup][aria-label=View] [role=radio][tabindex="0"]') ?? document.querySelector('${BAR} [role=radiogroup][aria-label=View] [role=radio]')).focus()`);
    await press(s, "Tab", 9);
    assert.equal(await activeName(s), "Pick mask paths", "Tab from View lands on the button");
    await press(s, "Tab", 9);
    assert.equal(await activeName(s), "Method", "no mask: the next stop is Method");
  });
});

test("0094 c41: with a mask the Tab order is View, Pick mask paths, Mask, Clear mask, Method", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"]);
    await enterTrace(s, scene);
    await setAdvanced(s, false);
    await shiftClick(s, scene.spots.rect[0], scene.spots.rect[1]);
    await waitFor("the Mask item", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    assert.deepEqual((await barControls(s)).slice(0, 7), ["About Trace", "Preset", "View", "Pick mask paths", "Mask", "Clear mask", "Method"]);
    await s.evaluate(`(document.querySelector('${BAR} [role=radiogroup][aria-label=View] [role=radio][tabindex="0"]') ?? document.querySelector('${BAR} [role=radiogroup][aria-label=View] [role=radio]')).focus()`);
    const seen: string[] = [];
    for (let i = 0; i < 4; i++) {
      await press(s, "Tab", 9);
      seen.push(await activeName(s));
    }
    assert.deepEqual(seen.map((n) => (n === "Inside" || n === "Outside" ? "Mask" : n)), ["Pick mask paths", "Mask", "Clear mask", "Method"]);
    // 28 + 4 + 128 + 4 + 28 = 192 px for the group
    const widths = JSON.parse((await s.evaluate(`JSON.stringify([${JSON.stringify(PICK)}, '${BAR} [role=radiogroup][aria-label=Mask]', ${JSON.stringify(CLEAR)}].map((q) => { const r = document.querySelector(q).getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)]; }))`)) as string) as number[][];
    assert.equal(widths[0][0], 28, "button width");
    assert.equal(widths[2][0], 28, "Clear width");
    assert.ok(widths.every(([w, h]) => w >= 28 && h >= 28), `every target at least 28 px: ${JSON.stringify(widths)}`);
  });
});

test("0094 c36, c35, c43: picks add and remove, the mode stays armed, the image and the path are selected, no success notice", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect", "ellipse"]);
    await enterTrace(s, scene);
    await stableCounts(s);
    const whole = await advancedStatus(s);
    const before = await s.layerRows();
    await arm(s);
    await clickAt(s, scene.spots.rect);
    await waitFor("the Mask item", async () => ((await maskState(s)) === "Inside=true,Outside=false" ? true : null), 10_000);
    assert.equal(await pickState(s), "true", "still armed");
    const sel = await selectedRows(s);
    assert.equal(sel.length, 2, `the image plus the path: ${JSON.stringify(sel)}`);
    assert.ok(sel.includes(scene.imageRow) && sel.some((r) => r.includes("Rectangle")), JSON.stringify(sel));
    assert.ok(await clearPresent(s), "Clear mask appears");
    await sleep(2500);
    assert.match(await advText(s), /Inside 1 path(?!s)/);
    const inside = await stableCounts(s);
    assert.notEqual(inside, whole, "the preview is traced inside the mask");
    // second path, then arming off keeps the mask (c35)
    await clickAt(s, scene.spots.ellipse);
    await waitFor("two paths", async () => (/Inside 2 paths/.test(await advText(s)) ? true : null), 20_000);
    await arm(s);
    assert.equal(await pickState(s), "false");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "disarming leaves the mask");
    assert.match(await advText(s), /Inside 2 paths/);
    await arm(s);
    // removal: a second click on the same path
    await clickAt(s, scene.spots.ellipse);
    await waitFor("one path again", async () => (/Inside 1 path(?!s)/.test(await advText(s)) ? true : null), 20_000);
    assert.equal(await pickState(s), "true", "armed after a removal");
    assert.ok(!(await selectedRows(s)).some((r) => r.includes("Ellipse")), "the removed path leaves the selection");
    await clickAt(s, scene.spots.rect);
    await waitFor("no mask", async () => ((await maskState(s)) === "absent" ? true : null), 20_000);
    assert.equal(await clearPresent(s), false, "Clear mask disappears with the last path");
    assert.equal(await pickState(s), "true", "armed after the last removal");
    assert.deepEqual(await s.layerRows(), before, "a pick creates nothing");
    assert.doesNotMatch(await s.statusText(), /created|done|success|added|removed/i, "no success notice");
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
  });
});

test("0094 c37, c41: a pick, Shift+click and the Select route give the same mask for an open path, a compound path, primitives and a group", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    pace = 0.1;
    const kinds: Kind[] = ["path", "compound", "rect", "ellipse", "polygon", "star", "group"];
    const scene = await buildScene(s, kinds);
    const rows = await s.layerRows();
    console.log(JSON.stringify(rows));
    for (const word of ["compound", "group", "polygon", "star", "ellipse", "rect"]) {
      assert.ok(rows.some((r) => r?.toLowerCase().includes(word)), `a ${word} row exists: ${JSON.stringify(rows)}`);
    }
    await enterTrace(s, scene);
    // Route 1: the button.
    await arm(s);
    const picked: string[] = [];
    for (const kind of kinds) {
      await clickAt(s, scene.spots[kind]);
      await waitFor(`${kind} added`, async () => {
        const seg = await maskSegment(s);
        return seg !== "" && seg !== picked[picked.length - 1] ? seg : null;
      }, 20_000);
      picked.push(await maskSegment(s));
    }
    console.log("picks", JSON.stringify(picked));
    const counts = (t: string) => Number(/(\d+) path/.exec(t)?.[1]);
    picked.forEach((seg, i) => i > 0 && assert.ok(counts(seg) > counts(picked[i - 1]), `each pick adds: ${JSON.stringify(picked)}`));
    const sig = async () => ({ mask: await maskState(s), seg: await maskSegment(s), counts: await stableCounts(s), sel: await selectionCount(s) });
    const byPick = await sig();
    assert.equal(byPick.mask, "Inside=true,Outside=false");
    assert.equal(await selectionCount(s), 1 + kinds.length, "the image plus seven objects are selected");
    // Clear mask: only the picture stays selected, the paths are still there.
    await s.clickSel(CLEAR);
    await waitFor("cleared", async () => ((await maskState(s)) === "absent" ? true : null), 10_000);
    assert.deepEqual(await selectedRows(s), [scene.imageRow]);
    assert.deepEqual((await s.layerRows()).sort(), rows.sort(), "the paths are untouched");
    // Route 2: Shift+click in the Trace tool.
    await arm(s);
    assert.equal(await pickState(s), "false");
    const shifted: string[] = [];
    for (const kind of kinds) {
      await shiftClick(s, scene.spots[kind][0], scene.spots[kind][1]);
      shifted.push(await maskSegment(s));
    }
    await waitFor("Inside again", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    const byShift = await sig();
    assert.deepEqual(byShift, byPick, "Shift+click = the picks");
    // Route 3: the Select route.
    await focusCanvas(s);
    await s.key("s", 83);
    await waitFor("the bar closes", async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);
    await clickAt(s, [scene.box.x1 + 150, scene.box.y1 + 100]);
    await clickAt(s, [scene.box.cx + 100, scene.box.y0 + 330]);
    for (const kind of kinds) {
      await clickAt(s, scene.spots[kind], 8);
    }
    assert.equal(await selectionCount(s), 1 + kinds.length, "image and seven objects selected");
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await sleep(1000);
    await setAdvanced(s, true);
    await waitFor("Inside", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    assert.deepEqual(await sig(), byPick, "the Select route = the picks");
  }).finally(() => {
    pace = 1;
  });
});

test("0094 c38: text, a second image and a sketch are refused with the row; the picture and empty canvas do nothing; a double-click on the picture creates nothing while armed", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"], { second: true, sketch: true, text: true });
    const rows = await s.layerRows();
    for (const word of ["text", "image", "sketch"]) {
      assert.ok(rows.some((r) => r?.endsWith(`, ${word}`)), `${word} exists: ${JSON.stringify(rows)}`);
    }
    await enterTrace(s, scene);
    await arm(s);
    for (const [name, at] of [["text", scene.spots.text], ["second image", scene.spots.second], ["sketch", scene.spots.sketch]] as const) {
      await clickAt(s, at);
      await waitFor(`${name} notice`, async () => ((await noticeShown(s)) ? true : null), 5000);
      assert.equal(await pickState(s), "true", `${name}: still armed`);
      assert.equal(await maskState(s), "absent", `${name}: not added`);
      assert.deepEqual(await selectedRows(s), [scene.imageRow], `${name}: not selected`);
    }
    // The row replaces nothing the button needs: the button is still under the pointer.
    assert.ok(await s.evaluate(`!!document.querySelector('${PICK}')`), "the button is still there");
    // The picture and empty canvas change nothing (the row stays as it is).
    await clickAt(s, [scene.box.cx + 100, scene.box.y0 + 330]);
    await clickAt(s, [scene.box.x1 + 160, scene.box.y1 + 100]);
    assert.deepEqual(await selectedRows(s), [scene.imageRow]);
    assert.equal(await maskState(s), "absent");
    assert.equal(await pickState(s), "true");
    // The next accepted pick clears the row.
    await clickAt(s, scene.spots.rect);
    await waitFor("the Mask item", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    assert.equal(await noticeShown(s), false, "the next accepted pick clears the row");
    assert.ok(await s.evaluate(`!!document.querySelector('${PICK}')`));
    // A refusal after a mask exists leaves the mask alone.
    await clickAt(s, scene.spots.text);
    await waitFor("notice again", async () => ((await noticeShown(s)) ? true : null), 5000);
    assert.equal(await maskState(s), "Inside=true,Outside=false");
    assert.equal((await selectedRows(s)).length, 2);
    // Leaving the mode clears the row.
    await arm(s);
    assert.equal(await noticeShown(s), false, "the row clears when the mode ends");
    // A double-click on the picture creates nothing while armed (and does outside the mode).
    await s.clickSel(CLEAR);
    await arm(s);
    const before = await s.layerRows();
    for (const clickCount of [1, 2]) {
      for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
        await s.send("Input.dispatchMouseEvent", { type, x: scene.box.cx + 100, y: scene.box.y0 + 330, button: "left", clickCount });
      }
    }
    await sleep(2500);
    assert.equal(await pickState(s), "true");
    assert.deepEqual(await s.layerRows(), before, "no paths created by a double-click while armed");
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await waitFor("Enter still creates", async () => ((await s.layerRows()).some((r) => r?.startsWith("Trace of")) ? true : null), 40_000).catch(() => {
      assert.fail("Enter should still create the paths (UX notes 11.3)");
    });
  });
});

/** Selects the picture row and opens Trace (after the tool was left). */
async function reenter(s: S, scene: Scene) {
  await rowClick(s, scene.imageRow);
  await waitFor("Trace in the context bar", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
}
const barGone = (s: S, what: string) => waitFor(what, async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);

test("0094 c39: the first Escape only disarms; the next leaves the tool; S, another tool and Create paths disarm and the next entry starts disarmed", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"]);
    await enterTrace(s, scene);
    // Escape: only the mode ends.
    await arm(s);
    await clickAt(s, scene.spots.rect);
    await waitFor("the mask", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await sleep(2000);
    const counts = await stableCounts(s);
    await press(s, "Escape", 27);
    assert.equal(await pickState(s), "false", "the first Escape disarms");
    assert.ok(await inTrace(s), "the tool stays");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "the mask is kept");
    assert.equal(await selectionCount(s), 2, "the selection is kept");
    assert.equal(await stableCounts(s), counts, "the preview is kept");
    await press(s, "Escape", 27);
    await barGone(s, "the second Escape leaves the tool");
    // S while armed.
    await reenter(s, scene);
    assert.equal(await pickState(s), "false", "entry starts disarmed");
    await arm(s);
    await focusCanvas(s);
    await s.key("s", 83);
    await barGone(s, "S leaves the tool");
    await reenter(s, scene);
    assert.equal(await pickState(s), "false", "after S: disarmed on the next entry");
    // Another tool while armed.
    await arm(s);
    await s.clickSel('button[aria-label^="Ellipse tool"]');
    await barGone(s, "another tool leaves Trace");
    await s.clickSel('button[aria-label^="Select tool"]');
    await reenter(s, scene);
    assert.equal(await pickState(s), "false", "after another tool: disarmed on the next entry");
    // Create paths while armed.
    await arm(s);
    await settledCounts(s);
    await s.clickSel(`${BAR} button[data-trace-slot], ${BAR} [data-trace-slot] button`);
    await barGone(s, "Create paths leaves the tool");
    assert.ok((await s.layerRows()).some((r) => r?.startsWith("Trace of")), "paths were created");
    await toSelect(s);
    await reenter(s, scene);
    assert.equal(await pickState(s), "false", "after Create paths: disarmed on the next entry");
  });
});

test("0094 c40, c43: Clear mask empties the mask, keeps only the picture selected and the paths in the document, is no history step, no notice, and leaves the armed state alone", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect", "ellipse", "star"]);
    await enterTrace(s, scene);
    await arm(s);
    for (const kind of ["rect", "ellipse", "star"]) {
      await clickAt(s, scene.spots[kind]);
    }
    await waitFor("three paths", async () => (/Inside 3 paths/.test(await advText(s)) ? true : null), 20_000);
    assert.equal(await selectionCount(s), 4);
    const rows = await s.layerRows();
    const history = await historyText(s);
    const strings = await barStrings(s);
    const clear = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${CLEAR}').getBoundingClientRect(); return [b.width, b.height]; })())`)) as string) as number[];
    assert.ok(clear[0] >= 28 && clear[1] >= 28, `Clear mask target ${clear}`);
    await s.clickSel(CLEAR);
    await waitFor("no mask", async () => ((await maskState(s)) === "absent" ? true : null), 10_000);
    assert.equal(await clearPresent(s), false, "the button disappears");
    assert.deepEqual(await selectedRows(s), [scene.imageRow], "only the picture is selected");
    assert.equal(await selectionCount(s), 1);
    assert.deepEqual(await s.layerRows(), rows, "the three paths are still there, same ids");
    assert.equal(await historyText(s), history, "no history step");
    assert.equal(await pickState(s), "true", "the armed state is unchanged");
    assert.deepEqual((await barStrings(s)).filter((x) => !strings.includes(x) && /clear|remov|mask|pick|notice/i.test(x)), [], "no notice text in the bar");
    assert.doesNotMatch(await s.statusText(), /cleared|removed|done|success/i, "no notice");
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
    // Disarmed stays disarmed.
    await arm(s);
    await shiftClick(s, scene.spots.rect[0], scene.spots.rect[1]);
    await waitFor("the mask", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await s.clickSel(CLEAR);
    await waitFor("no mask", async () => ((await maskState(s)) === "absent" ? true : null), 10_000);
    assert.equal(await pickState(s), "false", "disarmed stays disarmed");
    assert.equal(await historyText(s), history, "still no history step");
  });
});

test("0094 pipette: pick mode and the pipette are never on together (Proposal of the implementer)", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"]);
    await enterTrace(s, scene);
    await pickMethod(s, "Pick colours");
    const pip = async () => (await s.evaluate(`document.querySelector('${pipette}')?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
    assert.notEqual(await pip(), "absent", "the pipette button is in the bar");
    // Choosing Pick colours arms the pipette by itself.
    const first = await pip();
    await arm(s);
    const afterArm = [await pip(), await pickState(s)];
    await s.clickSel(pipette);
    const afterPipette = [await pip(), await pickState(s)];
    console.log(JSON.stringify({ first, afterArm, afterPipette }));
    assert.equal(first, "true", "the pipette arms");
    assert.deepEqual(afterArm, ["false", "true"], "arming pick mask disarms the pipette");
    assert.deepEqual(afterPipette, ["true", "false"], "arming the pipette disarms pick mode");
  });
});

test("0094 c43: at 800 x 600 with the panel open the group wraps as a whole item, every target at least 28 px, no overlap, no clipping, no horizontal scroll", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withTall(async (s) => {
    const scene = await buildScene(s, ["rect"]);
    await enterTrace(s, scene);
    await setAdvanced(s, false);
    const measure = async (label: string) => {
      const m = JSON.parse((await s.evaluate(`JSON.stringify((() => {
        const bar = document.querySelector('${BAR}'); const br = bar.getBoundingClientRect();
        const items = [...bar.querySelectorAll('button, [role=spinbutton], [role=radiogroup]')].filter((e) => !e.closest('[role=radiogroup]') || e.matches('[role=radiogroup]')).filter((e) => e.getBoundingClientRect().width > 0)
          .map((e) => { const r = e.getBoundingClientRect(); return { n: e.getAttribute('aria-label') || e.textContent.trim(), x: r.x, y: r.y, w: r.width, h: r.height }; });
        const radios = [...bar.querySelectorAll('[role=radio]')].map((e) => { const r = e.getBoundingClientRect(); return { n: e.getAttribute('aria-label') || e.textContent.trim(), w: r.width, h: r.height }; });
        return { bar: { x: br.x, y: br.y, w: br.width, h: br.height, sw: bar.scrollWidth, cw: bar.clientWidth }, items, radios, doc: [document.documentElement.scrollWidth, innerWidth], mid: [window.innerWidth, window.innerHeight] };
      })())`)) as string) as { bar: { x: number; y: number; w: number; h: number; sw: number; cw: number }; items: { n: string; x: number; y: number; w: number; h: number }[]; radios: { n: string; w: number; h: number }[]; doc: number[]; mid: number[] };
      assert.deepEqual(m.mid, [800, 600], `${label}: the window`);
      assert.ok(m.bar.sw <= m.bar.cw + 1, `${label}: the bar does not scroll or clip horizontally (${m.bar.sw} > ${m.bar.cw})`);
      assert.ok(m.doc[0] <= m.doc[1], `${label}: no horizontal page scroll ${m.doc}`);
      for (const it of m.items) {
        assert.ok(it.x >= m.bar.x - 0.5 && it.x + it.w <= m.bar.x + m.bar.w + 0.5, `${label}: ${it.n} inside the bar's width`);
        assert.ok(it.h >= 27.5, `${label}: ${it.n} is at least 28 px high (${it.h})`);
        if (it.n === "Pick mask paths" || it.n === "Clear mask") {
          assert.ok(it.w >= 27.5, `${label}: ${it.n} is at least 28 px wide`);
        }
      }
      for (const r of m.radios) {
        assert.ok(r.h >= 27.5 && r.w >= 27.5, `${label}: radio ${r.n} ${r.w} x ${r.h}`);
      }
      for (let i = 0; i < m.items.length; i++) {
        for (let j = i + 1; j < m.items.length; j++) {
          const a = m.items[i];
          const b = m.items[j];
          const ox = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
          const oy = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
          assert.ok(!(ox > 1 && oy > 1), `${label}: ${a.n} and ${b.n} overlap`);
        }
      }
      return m;
    };
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await sleep(800);
    const bare = await measure("no mask");
    assert.ok(bare.items.some((i) => i.n === "Pick mask paths"), "the button is in the bar");
    await s.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 1100, deviceScaleFactor: 1, mobile: false });
    await sleep(800);
    await shiftClick(s, scene.spots.rect[0], scene.spots.rect[1]);
    await waitFor("the mask", async () => ((await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await sleep(800);
    const masked = await measure("masked");
    const group = ["Pick mask paths", "Mask", "Clear mask"].map((n) => masked.items.find((i) => i.n === n));
    assert.ok(group.every(Boolean), "all three parts of the group are there");
    const centres = group.map((i) => i!.y + i!.h / 2);
    assert.ok(Math.max(...centres) - Math.min(...centres) <= 3, `the group stays on one line: ${centres}`);
    const width = group[2]!.x + group[2]!.w - group[0]!.x;
    assert.ok(Math.abs(width - 192) <= 2, `the group is 192 px wide: ${width}`);
  });
});

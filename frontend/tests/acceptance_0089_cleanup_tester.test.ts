// Tester cases for the Trace bar cleanup of 2026-11-06 (`specs/0089-trace-quality-and-colour/ux-notes.md` §24) in the
// real app (Chromium via DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written
// from §24 before the code was read: no View pill and no view keys (24.1), the bar order and the 150 / 194 px
// fields (24.2, 24.7), the last Advanced row Corners | Parts | Analyse (24.3), the path-mask group always drawn and
// never dimmed, Clear mask without paths a no-op (24.4), no analysis or status line and one hidden status region
// that says them once, in order (24.5), no operating hint text. Harness as in `acceptance_0094_tester.test.ts`;
// page-side wasm imports use their own instances (`?own-instance`), never the app's.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5489;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-cleanup-tester");
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

/** Imports `png` as `name` and opens the Trace bar on it; Advanced is set to `advanced`. */
async function openOn(s: S, name: string, png: Buffer, advanced = false) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  const before = (await s.layerRows()).length;
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).length > before ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  await setAdvanced(s, advanced);
}

async function setAdvanced(s: S, open: boolean) {
  const now = (await s.evaluate(`document.querySelector('${EXPANDER}')?.getAttribute('aria-expanded')`)) === "true";
  if (now !== open) {
    await s.evaluate(`document.querySelector('${EXPANDER}').click()`);
    await sleep(500);
  }
}

const withApp = async (fn: (s: S) => Promise<void>, preload = "", width = 1280, height = 800) => {
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

const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);
const settledCounts = (s: S) => waitFor("a trace result", async () => (/(^|\. )(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);

const BRIGHTNESS_BAR = ["About Trace", "Preset", "Pick mask paths", "Mask", "Clear mask", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"];

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
  for (let i = 1; i <= 8; i++) {
    const x = from[0] + ((to[0] - from[0]) * i) / 8;
    const y = from[1] + ((to[1] - from[1]) * i) / 8;
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

/** Imports M1, draws a rectangle over `part` of it ("left" half, or "outside" right of it), selects both with Ctrl+A and opens Trace. */
async function maskedOn(s: S, part: "left" | "outside" = "left") {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  if (part === "left") {
    await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
  } else {
    await drag(s, [box.x1 + 40, box.cy - 20], [box.x1 + 90, box.cy + 20]);
  }
  await s.key("s", 83);
  await focusCanvas(s);
  await s.key("a", 65, 2);
  await waitFor("two objects selected", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  return box;
}
const maskState = async (s: S) =>
  (await s.evaluate(`(() => { const g = document.querySelector('${BAR} [role=radiogroup][aria-label=Mask]'); if (!g) return 'absent';
    return [...g.querySelectorAll('[role=radio]')].map((r) => r.textContent.trim() + '=' + r.getAttribute('aria-checked')).join(','); })()`)) as string;
const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
// §24.5: the muted "Inside 1 path" segment is announced by the hidden region only.
const advText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
/** The mask paths of the active part, as the hidden region announces them ("Inside 2 paths"); 0 without. */
const maskPaths = async (s: S) => Number((await advText(s)).match(/(?:Inside|Outside) (\d+) paths?/)?.[1] ?? 0);

// ------------------------------------------------------------------ cleanup helpers

/** 420 x 300: thin black strokes on white, a line drawing. */
const LINES = rgbPng(420, 300, (x, y) => {
  for (let k = 1; k < 6; k++) {
    if (Math.abs(y - (45 * k + 14 * Math.sin(x / 40 + k))) < 1.2) return [10, 10, 10];
  }
  return Math.abs(x - y) < 1.4 ? [10, 10, 10] : [255, 255, 255];
});
/** 300 x 300: one black square and five isolated one-pixel specks on white. */
const SPECKS = rgbPng(300, 300, (x, y) => {
  if (x >= 100 && x < 200 && y >= 100 && y < 200) return [0, 0, 0];
  return [[20, 20], [250, 30], [30, 260], [270, 270], [60, 150]].some(([px, py]) => px === x && py === y) ? [0, 0, 0] : [255, 255, 255];
});

const field = (name: string) => `${BAR} [role=spinbutton][aria-label="${name}"]`;
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${field(name)}').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const setOutput = async (s: S, label: string) => {
  await s.evaluate(`[...document.querySelectorAll('${ADV} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === ${JSON.stringify(label)}).click()`);
  await sleep(1000);
};
const setPanel = async (s: S, open: boolean) => {
  const now = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
  if (now !== open) {
    await s.clickSel(`[aria-label="${now ? "Hide panel" : "Show panel"}"]`);
    await sleep(700);
  }
};
// `ux-notes.md` §27 (customer 2026-11-09): the notice row is gone; a former notice is said by the hidden region only.
const noticeText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-state]')?.textContent ?? ''`)) as string;
const rectOf = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect();
    return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom }; })())`)) as string) as { x: number; y: number; w: number; h: number; right: number; bottom: number } | null;
const hiddenRegion = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? 'absent'`)) as string;
const historyText = async (s: S) => {
  await s.tab("History");
  await s.evaluate(`[...document.querySelectorAll('[role=tabpanel][data-state=active] [role=radio], [role=tabpanel][data-state=active] button')].find((b) => b.textContent.trim() === 'Document')?.click()`);
  await sleep(300);
  const text = ((await s.evaluate(`document.querySelector('[role=tabpanel][data-state=active]')?.textContent ?? ''`)) as string).replace(/^DocumentObject/, "");
  await s.tab("Layers");
  return text;
};
const selectionCount = async (s: S) =>
  Number((await s.evaluate(`(() => { const all = [...document.querySelectorAll('*')].filter((e) => e.children.length === 0 && /^\\d+ objects?$/.test(e.textContent.trim())); return all.length ? all[all.length - 1].textContent.trim().split(' ')[0] : '-1'; })()`)) as string);

const MASK_STOPS = ["Pick mask paths", "Mask", "Clear mask"];

// ------------------------------------------------------------------ 24.1: no View pill, no views, no keys

test("0089 §24.1: there is no View pill, no Mask, Trace or Image view, no peek and no cycle key; `\\` and Shift+`\\` are free", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await settledCounts(s);
    assert.equal(await s.evaluate(`document.querySelectorAll('[role=radiogroup][aria-label=View]').length`), 0, "no View pill");
    const names = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [aria-label], ${BAR} [role=radio]')].map((e) => (e.getAttribute('aria-label') || e.textContent).trim()))`)) as string;
    for (const gone of ["View", "Overlay", "Mask view", "Trace view", "Image view", "Image (\\\\)"]) {
      assert.ok(!(JSON.parse(names) as string[]).includes(gone), `${gone} is not a control of the bar`);
    }
    // The keys reach no handler of the tool: neither is consumed (defaultPrevented stays false) and nothing changes.
    const before = await hiddenRegion(s);
    await focusCanvas(s);
    const consumed = JSON.parse((await s.evaluate(`JSON.stringify((() => {
      const target = document.activeElement ?? document.body;
      return [['\\\\', false], ['|', true], ['\\\\', true]].map(([k, shift]) => {
        const down = new KeyboardEvent('keydown', { key: k, code: 'Backslash', shiftKey: shift, bubbles: true, cancelable: true });
        target.dispatchEvent(down);
        const up = new KeyboardEvent('keyup', { key: k, code: 'Backslash', shiftKey: shift, bubbles: true, cancelable: true });
        target.dispatchEvent(up);
        return down.defaultPrevented || up.defaultPrevented;
      });
    })())`)) as string) as boolean[];
    assert.deepEqual(consumed, [false, false, false], "backslash and Shift+backslash are not taken by the Trace tool");
    await sleep(500);
    assert.equal(await hiddenRegion(s), before, "nothing changed");
    assert.equal(await s.evaluate(`document.querySelector('${BAR}') !== null`), true, "the tool stays");
  });
});

test("0089 §24.1, §24.6: the strings of the views, the status line and the analysis row are gone from the sources, the user page and the design system", () => {
  const read = (p: string) => readFileSync(path.resolve(FRONTEND, "..", p), "utf8");
  const page = read("docs/tracing-images.md");
  const text = read("frontend/src/lib/traceText.ts");
  const design = read("docs/design-system.md");
  const rows = design.split("\n").filter((l) => /^\| Trace /.test(l)).join("\n");
  for (const [name, body] of [["docs/tracing-images.md", page], ["traceText.ts", text], ["the Trace rows of docs/design-system.md", rows]] as const) {
    // The design system says what is gone ("no Mask, Trace or Image view, no peek", §24.8), so only the pill and the keys are searched there.
    const gones = name.startsWith("the Trace rows") ? ["View pill", "Hold `\\`", "Shift+`\\`", "Image (\\)"] : ["View pill", "Mask view", "Trace view", "Image view", "Hold \\ ", "Shift+\\", "peek at the image", "Image (\\)"];
    for (const gone of gones) {
      assert.ok(!body.includes(gone), `${name} does not mention "${gone}"`);
    }
  }
  for (const file of ["useTraceViewMode.ts"]) {
    assert.equal(existsSync(path.resolve(FRONTEND, "src", "hooks", file)), false, `${file} is deleted`);
  }
  const sources: string[] = [];
  const walk = (dir: string) => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(p);
      else if (/\.tsx?$/.test(entry.name)) sources.push(p);
    }
  };
  walk(path.resolve(FRONTEND, "src"));
  const dead = /\b(TRACE_VIEWS|isPeekKey|isCycleKey|ViewPill|useTraceViewMode|setViewMode)\b/;
  assert.deepEqual(sources.filter((p) => dead.test(readFileSync(p, "utf8"))).map((p) => path.relative(FRONTEND, p)), [], "no source names the view machinery any more");
  const entries = text.match(/TRACE_INFO_ENTRIES[\s\S]*?\];/)![0];
  for (const entry of ["View:", "Status:", "Analysis:"]) {
    assert.ok(!entries.includes(`"${entry}`), `About Trace has no ${entry} entry`);
  }
  assert.ok(entries.includes('"Analyse:'), "About Trace keeps one Analyse entry");
});

// ------------------------------------------------------------------ 24.2, 24.7: order and widths

test("0089 §24.2, §24.7: the value fields are 150 px wide, Ignore below 171 px (its value needs it, lead 2026-11-06; §25.1: no readout, so the item is the field alone), Colours 120 px, the Denoise and Upscale cells at least 150 px", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    const w = async (sel: string) => Math.round((await rectOf(s, sel))?.w ?? -1);
    for (const name of ["Threshold", "Smooth", "Tolerance", "Corners"]) {
      assert.equal(await w(field(name)), 150, `${name} is 150 px wide`);
    }
    assert.equal(await w(field("Ignore below")), 171, "Ignore below is 171 px wide");
    assert.equal((await rectOf(s, field("Smooth")))!.h, 28, "hit height unchanged");
    // §25.1: the pixel readout and its slot are gone: the item is the 171 px field alone.
    const ignore = (await rectOf(s, field("Ignore below")))!;
    const row = JSON.parse((await s.evaluate(`JSON.stringify((() => { const f = document.querySelector('${field("Ignore below")}'); let n = f; while (n && n.parentElement && n.parentElement.getAttribute('data-trace-row') === null) n = n.parentElement;
      const r = n.getBoundingClientRect(); return { w: r.width, x: r.x }; })())`)) as string) as { w: number; x: number };
    assert.ok(Math.abs(row.w - 171) <= 1, `Ignore below, no readout, is ${row.w} px`);
    assert.deepEqual(JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} *')].filter((e) => e.children.length === 0 && !e.closest('.sr-only') && /^[~<] ?\\d+ px$/.test(e.textContent.trim())).map((e) => e.textContent))`)) as string), [], "no '~ 2 px' or '< 1 px' readout anywhere in the bar");
    assert.equal(Math.round(ignore.x - row.x), 0, "the field starts the item");
    // Denoise and Upscale cells in Advanced.
    for (const name of ["Denoise", "Upscale"]) {
      const cell = JSON.parse((await s.evaluate(`JSON.stringify((() => { const g = document.querySelector('${ADV} [role=radiogroup][aria-label=${name}]'); const c = g.parentElement; const r = c.getBoundingClientRect(); return r.width; })())`)) as string) as number;
      console.log(`${name} cell: ${cell} px`);
      assert.ok(cell >= 149.5, `the ${name} cell is at least 150 px (min-w, UX notes §24.3 item 3): ${cell}`);
    }
    await setOutput(s, "Multicolour");
    await settledCounts(s);
    assert.equal(await w(field("Colours")), 120, "Colours stays 120 px");
    await pickMethod(s, "Remove background");
    assert.equal(await w(field("Colour tolerance")), 150, "Colour tolerance is 150 px");
    // No label or value clips at 150 px: every text of a field fits its box.
    const clipped = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=spinbutton]')].filter((f) => [...f.querySelectorAll('*')].some((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow !== 'visible')).map((f) => f.getAttribute('aria-label')))`)) as string) as string[];
    assert.deepEqual(clipped, [], "no field label or value clips");
  });
});

test("0089 §24.2: the bar order is i, Preset, Mask group, Method, main field, Smooth, Tolerance, Ignore below, expander, Trace; the Mask group is 192 px (28 + 4 + 128 + 4 + 28) and one flow item", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, false);
    await pickMethod(s, "Brightness");
    assert.deepEqual(await barControls(s), BRIGHTNESS_BAR);
    const group = (await rectOf(s, `${BAR} [data-trace-mask]`))!;
    assert.ok(Math.abs(group.w - 192) <= 1, `the group is ${group.w} px`);
    const preset = (await rectOf(s, `${BAR} button[aria-label=Preset]`))!;
    const method = (await rectOf(s, `${BAR} button[aria-label=Method]`))!;
    const sameLine = (a: { y: number; h: number }, b: { y: number; h: number }) => Math.abs(a.y + a.h / 2 - (b.y + b.h / 2)) < 14;
    assert.ok(sameLine(preset, group) ? group.x >= preset.right - 1 : group.y > preset.y, "the group follows Preset");
    assert.ok(sameLine(group, method) ? method.x >= group.right - 1 : method.y > group.y, "Method follows the group");
    for (const stop of ["Pick mask paths", "Clear mask"]) {
      const b = (await rectOf(s, `${BAR} [aria-label="${stop}"]`))!;
      assert.ok(b.w >= 28 && b.h >= 28, `${stop} is at least 28 x 28`);
      assert.ok(b.y >= group.y - 1 && b.bottom <= group.bottom + 1, `${stop} is inside the group's line`);
    }
    const pill = (await rectOf(s, `${BAR} [role=radiogroup][aria-label=Mask]`))!;
    assert.ok(Math.abs(pill.w - 128) <= 1, `Inside / Outside is ${pill.w} px`);
  });
});

// ------------------------------------------------------------------ 24.3: the last Advanced row

test("0089 §24.3: the Advanced block ends with Corners | Parts | Analyse; Parts has a muted label that is no Tab stop; Analyse is at the far right; the group grows 32 + 4 + chips + 4 + 28 + 4 + 28", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    const corners = (await rectOf(s, field("Corners")))!;
    const parts = (await rectOf(s, `${ADV} [data-trace-parts]`))!;
    const analyse = (await rectOf(s, `${ADV} [data-trace-analyse]`))!;
    const adv = (await rectOf(s, ADV))!;
    // At 1280 x 800 the whole row is one line, in this order.
    assert.ok(Math.abs(corners.y + corners.h / 2 - (parts.y + parts.h / 2)) < 14 && parts.x >= corners.right - 1, "Parts follows Corners on the line");
    assert.ok(analyse.x >= parts.right - 1, "Analyse follows Parts");
    assert.ok(Math.abs(analyse.w - 75) <= 1 && Math.abs(analyse.h - 28) <= 1, `Analyse is ${analyse.w} x ${analyse.h}`);
    assert.ok(adv.right - analyse.right <= 8, `Analyse is at the far right of its line (gap ${adv.right - analyse.right})`);
    assert.ok(Math.abs(parts.w - 128) <= 1, `one part: ${parts.w} px`);
    // The label: muted, 12 px, aria-hidden, no Tab stop; the radiogroup keeps its name.
    const label = JSON.parse((await s.evaluate(`JSON.stringify((() => { const l = document.querySelector('${ADV} [data-trace-parts] > span[aria-hidden]'); const cs = getComputedStyle(l);
      return { text: l.textContent, size: cs.fontSize, opacity: cs.opacity, w: l.getBoundingClientRect().width, tab: l.matches('button, [tabindex]'), name: document.querySelector('${ADV} [data-trace-parts] [role=radiogroup]').getAttribute('aria-label') }; })())`)) as string) as
      { text: string; size: string; opacity: string; w: number; tab: boolean; name: string };
    assert.deepEqual([label.text, label.size, label.tab, label.name], ["Parts", "12px", false, "Parts"]);
    assert.ok(Number(label.opacity) < 1 && Math.round(label.w) === 32, `muted label, 32 px slot: ${JSON.stringify(label)}`);
    // The Parts row is no longer the first row; the analysis row and the status line are not there.
    const first = (await rectOf(s, `${ADV} [data-trace-output-row]`))!;
    assert.ok(first.y < parts.y, "the Output row comes first, Parts last");
    assert.equal(await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-analysis], ${ADV} [data-trace-counts]').length`), 0);
    // Widths: 2 parts 160, 3 parts 192.
    for (const [n, want] of [[2, 160], [3, 192]] as const) {
      await s.clickSel(`${ADV} [data-trace-add-part]`);
      await sleep(600);
      const w = (await rectOf(s, `${ADV} [data-trace-parts]`))!.w;
      assert.ok(Math.abs(w - want) <= 1, `${n} parts: ${w} px, want ${want}`);
      assert.equal(await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-part]').length`), n);
    }
    // Tab stops: Parts is one stop, Add part a stop, Remove part none, Analyse last.
    await focusCanvas(s);
    const stops: string[] = [];
    for (let i = 0; i < 40; i++) {
      await press(s, "Tab", 9);
      const name = (await s.evaluate(`(() => { const e = document.activeElement; if (!e || e === document.body || !e.closest('${BAR}')) return 'end'; const g = e.closest('[role=radiogroup]'); return g ? g.getAttribute('aria-label') : (e.getAttribute('aria-label') || e.textContent.trim()); })()`)) as string;
      if (name === "end") break;
      stops.push(name);
    }
    const tail = stops.slice(stops.indexOf("Corners"));
    assert.deepEqual(tail, ["Corners", "Parts", "Add part", "Analyse"], `the last stops: ${JSON.stringify(stops)}`);
    assert.ok(!stops.some((n) => /^Remove part|^Keep/.test(n)), "Remove part is no Tab stop");
    assert.equal(stops.filter((n) => n === "Parts").length, 1, "the chips are one stop");
    assert.ok(!stops.includes("Part 2 of 3"), "the bar's 'Part 2 of 3' label is no stop");
    // Hint text: none.
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
  });
});

// ------------------------------------------------------------------ 24.4: the mask group is always there

test("0089 §24.4: the Mask group is drawn for every method and state, never dimmed or aria-disabled, Inside on entry, and the bar does not move when the first path is picked", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, false);
    const undimmed = async (tag: string) => {
      const info = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-mask] button')].map((b) => { const cs = getComputedStyle(b); let o = 1; for (let n = b; n && n !== document.body; n = n.parentElement) o *= Number(getComputedStyle(n).opacity);
        return { name: b.getAttribute('aria-label') || b.textContent.trim(), ariaDisabled: b.getAttribute('aria-disabled'), disabled: b.disabled, opacity: o, pe: cs.pointerEvents }; }))`)) as string) as
        { name: string; ariaDisabled: string | null; disabled: boolean; opacity: number; pe: string }[];
      assert.deepEqual(info.map((i) => i.name), ["Pick mask paths", "Inside", "Outside", "Clear mask"], `${tag}: the three controls are drawn`);
      for (const i of info) {
        assert.equal(i.ariaDisabled, null, `${tag}: ${i.name} has no aria-disabled`);
        assert.equal(i.disabled, false, `${tag}: ${i.name} is enabled`);
        assert.equal(i.opacity, 1, `${tag}: ${i.name} is not dimmed`);
        assert.notEqual(i.pe, "none", `${tag}: ${i.name} takes the pointer`);
      }
    };
    for (const method of ["Brightness", "Remove background", "Pick colours"]) {
      await pickMethod(s, method);
      await undimmed(method);
      assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [aria-checked=true]')[0].textContent`), "Inside", `${method}: Inside on entry`);
    }
    await pickMethod(s, "Brightness");
    await setOutput(s, "Multicolour").catch(() => undefined);
    await undimmed("Multicolour");
    await setAdvanced(s, true);
    await undimmed("Advanced open");
  });
});

test("0089 §24.4: the first mask path changes neither the bar's width, nor its height, nor the place of any control", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const file = path.join(SCRATCH, "m1.png");
    writeFileSync(file, M1_PNG);
    await s.tab("Layers");
    await s.choose("[data-import-button]", file);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
    await sleep(800);
    const box = await pictureBox(s);
    const h = box.y1 - box.y0;
    await focusCanvas(s);
    await press(s, "Escape", 27);
    await s.key("r", 82);
    await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
    await s.key("s", 83);
    // Enter the tool with the picture alone first: no mask path.
    await s.evaluate(`document.activeElement?.blur?.()`);
    await s.clickSel(`[role=row][aria-label*="m1.png"]`);
    await waitFor("Trace offered", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await sleep(1000);
    await setAdvanced(s, false);
    await settledCounts(s);
    const survey = async () =>
      (await s.evaluate(`JSON.stringify((() => { const bar = document.querySelector('${BAR}').getBoundingClientRect();
        const items = [...document.querySelectorAll('${BAR} button, ${BAR} [role=spinbutton], ${BAR} [role=radiogroup]')].filter((e) => !e.closest('[role=group][aria-label=Advanced]') && (e.matches('[role=radiogroup]') || !e.closest('[role=radiogroup]')))
          .map((e) => { const r = e.getBoundingClientRect(); return [e.getAttribute('aria-label') || e.textContent.trim(), Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]; });
        return { bar: [Math.round(bar.x), Math.round(bar.y), Math.round(bar.width), Math.round(bar.height)], items }; })())`)) as string;
    const before = JSON.parse(await survey()) as { bar: number[]; items: [string, number, number, number, number][] };
    assert.equal(await maskPaths(s), 0, "no mask path yet");
    // Select the picture and the rectangle: Shift+click on the rectangle outside the picture adds its path.
    await shiftClick(s, box.x0 - 30, box.cy);
    await waitFor("one mask path", async () => ((await maskPaths(s)) === 1 ? true : null), 20_000);
    await settledCounts(s);
    const after = JSON.parse(await survey()) as { bar: number[]; items: [string, number, number, number, number][] };
    assert.deepEqual(after.bar, before.bar, "the bar's box did not change");
    assert.deepEqual(after.items, before.items, "no control moved or changed size");
    assert.equal(await barControls(s).then((c) => c.slice(0, 6).join("|")), ["About Trace", "Preset", ...MASK_STOPS, "Method"].join("|"));
  });
});

test("0089 §24.4: with no mask path Inside / Outside is selectable and changes nothing, and Clear mask is a no-op: no state change, no notice, no history step, focus stays", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, false);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    const status = await hiddenRegion(s);
    const history = await historyText(s);
    const selection = await selectionCount(s);
    await clickMask(s, "Outside");
    await sleep(900);
    assert.equal(await maskState(s), "Inside=false,Outside=true", "Outside is selectable with no mask");
    await settledCounts(s);
    assert.equal(await hiddenRegion(s), status, "the trace and its announcement are the same, Inside or Outside alike");
    assert.equal(await historyText(s), history, "Inside / Outside is no history step");
    await clickMask(s, "Inside");
    await sleep(600);
    assert.equal(await maskState(s), "Inside=true,Outside=false");
    // Clear mask by key: nothing happens and focus stays on the button.
    await s.evaluate(`document.querySelector('${BAR} [data-trace-clear-mask]').focus()`);
    for (const [k, vk, text] of [["Enter", 13, "\r"], [" ", 32, " "]] as const) {
      await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: k, code: k === " " ? "Space" : k, windowsVirtualKeyCode: vk, text });
      await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k === " " ? "Space" : k, windowsVirtualKeyCode: vk });
      await sleep(400);
      assert.equal(await s.evaluate(`document.activeElement?.hasAttribute('data-trace-clear-mask')`), true, `${JSON.stringify(k)}: focus stays on Clear mask`);
    }
    // And by mouse: a press with nothing to clear leaves the mask group's state alone.
    await s.clickSel(`${BAR} [data-trace-clear-mask]`);
    await sleep(500);
    assert.equal(await maskState(s), "Inside=true,Outside=false");
    assert.equal(await hiddenRegion(s), status, "no state change");
    assert.equal(await noticeText(s), "", "no notice");
    assert.equal(await historyText(s), history, "no history step");
    assert.equal(await selectionCount(s), selection, "the selection is the same");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [data-trace-clear-mask]').getAttribute('aria-disabled')`), null, "never aria-disabled");
  });
});

test("0089 §24.4: with a path Clear mask removes it and a key press keeps focus on the button (it does not disappear); Outside survives a cleared mask until the next entry", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await maskedOn(s, "left");
    await settledCounts(s);
    assert.ok((await maskPaths(s)) > 0, "a mask path");
    await clickMask(s, "Outside");
    await sleep(600);
    await s.evaluate(`document.querySelector('${BAR} [data-trace-clear-mask]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
    await waitFor("no mask path", async () => ((await maskPaths(s)) === 0 ? true : null), 15_000);
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [data-trace-clear-mask]')`), true, "Clear mask is still drawn");
    assert.equal(await s.evaluate(`document.activeElement?.hasAttribute('data-trace-clear-mask')`), true, "focus stays on Clear mask after a key press");
    assert.equal(await maskState(s), "Inside=false,Outside=true", "the choice is kept for the first path added later");
  });
});

test("0089 §24.4, §27: a mask outside the picture dims Trace with the reason as its description and announcement, nothing is drawn, and the Mask group stays", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await maskedOn(s, "outside");
    await waitFor("the reason", async () => (((await traceButton(s)) as { reason: string } | null)?.reason === "0 paths: the mask is outside the image." ? true : null), 30_000);
    const trace = (await traceButton(s)) as { disabled: string | null; reason: string };
    assert.equal(trace.disabled, "true");
    assert.equal(await noticeText(s), "", "not drawn");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [data-trace-mask]')`), true, "the group stays");
    assert.ok((await hiddenRegion(s)).includes("0 paths: the mask is outside the image."), "the hidden region says it");
  });
});

// ------------------------------------------------------------------ 24.5: one hidden status region

test("0089 §24.5: one always-mounted hidden polite region; the analysis, the mask, the counts and the specks in that order joined by '. '; each change announced once", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "specks.png", SPECKS, true);
    await pickMethod(s, "Brightness");
    await waitFor("specks ignored", async () => (/specks? ignored/.test(await hiddenRegion(s)) ? true : null), 60_000);
    for (const open of [true, false]) {
      await setAdvanced(s, open);
      assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-hidden-status]').length`), 1, `one region, Advanced ${open ? "open" : "closed"}`);
      assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [role=status].sr-only[data-trace-hidden-status][aria-live=polite]').length`), 1);
    }
    const text = await hiddenRegion(s);
    const segs = text.split(". ");
    // analysis line, counts, specks (the mask segment is empty without paths)
    assert.match(segs[0], /^(Custom · )?[A-Z][A-Za-z ]+( · .+)?$/, `first the analysis line: ${text}`);
    const at = (re: RegExp) => segs.findIndex((x) => re.test(x));
    assert.ok(at(/^\d+ paths?, [\d,]+ nodes?, [\d.]+ s$/) > 0, `the counts: ${text}`);
    assert.ok(at(/specks? ignored/) > at(/^\d+ paths?, /), `the specks after the counts: ${text}`);
    assert.ok(!/Inside|Outside/.test(text), "no mask segment without a path");
    // Nothing of it is drawn.
    const drawn = (await s.evaluate(`(() => { const c = document.querySelector('${BAR}').cloneNode(true); c.querySelectorAll('.sr-only').forEach((e) => e.remove()); document.body.appendChild(c); const t = c.innerText; c.remove(); return t; })()`)) as string;
    for (const gone of [/specks? ignored/, /\d+ paths?, [\d,]+ nodes?/, /regions? joined/, /shapes? set/, /text lines? set/, /Isolating|Merging|excluded/]) {
      assert.doesNotMatch(drawn, gone, `${gone} is not drawn`);
    }
  });
});

test("0089 §24.5: a settled edit is announced once, not while a value is dragged, and a mask adds its segment between the analysis and the counts", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await maskedOn(s, "left");
    await settledCounts(s);
    const text = await hiddenRegion(s);
    const segs = text.split(". ");
    const mask = segs.findIndex((x) => /^Inside 1 path$/.test(x));
    const counts = segs.findIndex((x) => /^\d+ paths?, /.test(x));
    assert.ok(mask >= 1 && counts > mask, `analysis, mask, counts in order: ${text}`);
    assert.ok(!segs.slice(0, mask).some((x) => /path/.test(x)), "the analysis line comes first");
    // Record every text the region takes.
    await s.evaluate(`(() => { const e = document.querySelector('${BAR} [data-trace-hidden-status]'); window.__seen = []; window.__mo = new MutationObserver(() => window.__seen.push(e.textContent)); window.__mo.observe(e, { childList: true, characterData: true, subtree: true }); })()`);
    await typeInto(s, "Smooth", "55");
    await settledCounts(s);
    await sleep(1500);
    const seen = JSON.parse((await s.evaluate(`JSON.stringify(window.__seen)`)) as string) as string[];
    console.log("announcements after one typed edit:", JSON.stringify(seen));
    const changes = seen.filter((t, i) => i === 0 || t !== seen[i - 1]);
    assert.deepEqual(changes, seen, "the region is never set twice to the same text");
    assert.ok(seen.length >= 1 && seen.length <= 3, `one edit, at most the analysis turning Custom and the new result: ${seen.length}`);
    assert.equal(new Set(seen.filter((t) => /\d+ paths?, /.test(t))).size, new Set(seen).size - (seen.some((t) => !/\d+ paths?, /.test(t)) ? 1 : 0), "each announced result is a distinct settled one");
    // Dragging a value announces nothing until it ends.
    await s.evaluate(`window.__seen.length = 0`);
    const smoothBefore = await s.evaluate(`document.querySelector('${field("Smooth")}').getAttribute('aria-valuetext')`);
    const regionBefore = await hiddenRegion(s);
    const box = (await rectOf(s, field("Smooth")))!;
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: box.x + box.w * 0.2, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    let midDrag = 0;
    for (let i = 0; i < 40; i++) {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box.x + box.w * (0.2 + 0.6 * ((i % 20) / 20)), y: box.y + box.h / 2, button: "left" });
      await sleep(25);
      if (i === 30) midDrag = ((await s.evaluate(`window.__seen.length`)) as number);
    }
    assert.equal(midDrag, 0, "nothing is announced while the value is dragged");
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: box.x + box.w * 0.5, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    await settledCounts(s);
    await sleep(1500);
    const after = JSON.parse((await s.evaluate(`JSON.stringify(window.__seen)`)) as string) as string[];
    assert.notEqual(await s.evaluate(`document.querySelector('${field("Smooth")}').getAttribute('aria-valuetext')`), smoothBefore, "the drag changed the value");
    // A drag whose result prints the same counts changes no text; otherwise the settled result is announced.
    assert.ok(after.length >= 1 || (await hiddenRegion(s)) === regionBefore, "the settled result is announced after the drag");
    assert.deepEqual(after.filter((t, i) => i === 0 || t !== after[i - 1]), after, "once per change after the drag");
  });
});

test("0089 §24.5: the analysis line is announced and never drawn", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    const drawn = async () =>
      (await s.evaluate(`(() => { const c = document.querySelector('${BAR}').cloneNode(true); c.querySelectorAll('.sr-only').forEach((e) => e.remove()); document.body.appendChild(c); const t = c.innerText; c.remove(); return t; })()`)) as string;
    assert.match(await hiddenRegion(s), /^(Custom · |[A-Z])/, "an analysis line is announced");
    assert.doesNotMatch(await drawn(), /Custom ·|automatic Threshold|Type unclear|Analysing/, "no analysis line is drawn");
  });
});

test("0089 §24.5, §27: an empty Multicolour result dims Trace with the reason as tooltip, description and announcement, draws no text, and Advanced has no status line (\"No colours found.\" is covered by traceMulticolour.test.ts)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "white.png", rgbPng(120, 120, () => [255, 255, 255]), true);
    await pickMethod(s, "Brightness");
    await setOutput(s, "Multicolour");
    const reason = async () => ((await traceButton(s)) as { reason: string } | null)?.reason ?? "";
    await waitFor("the reason", async () => (/^0 paths/.test(await reason()) || (await reason()) === "No colours found." ? true : null), 30_000);
    const notice = await reason();
    console.log("white picture, Multicolour, reason:", JSON.stringify(notice));
    const trace = (await traceButton(s)) as { disabled: string | null; reason: string };
    assert.equal(trace.disabled, "true", "Trace is dimmed");
    assert.equal(await noticeText(s), "", "not drawn");
    assert.ok((await hiddenRegion(s)).includes(notice), "the hidden region says it");
    assert.equal(await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-counts]').length`), 0, "no status line in Advanced");
  });
});

// ------------------------------------------------------------------ 24.2: the bar at 800 x 600

test("0089 §24.2, §24.3: at 800 x 600 with the panel open and closed, Advanced open, the bar and the block wrap by whole groups, never scroll sideways, and every target is at least 28 px", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    const survey = async (tag: string) => {
      const m = JSON.parse((await s.evaluate(`JSON.stringify((() => {
        const bar = document.querySelector('${BAR}'); const b = bar.getBoundingClientRect();
        const targets = [...bar.querySelectorAll('button, [role=spinbutton], [role=radio]')].filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && !e.closest('.sr-only'); });
        const small = targets.filter((e) => { const r = e.getBoundingClientRect(); return r.width < 28 - 0.5 || r.height < 28 - 0.5; }).map((e) => (e.getAttribute('aria-label') || e.textContent.trim()) + ' ' + Math.round(e.getBoundingClientRect().width) + 'x' + Math.round(e.getBoundingClientRect().height));
        const outside = targets.filter((e) => { const r = e.getBoundingClientRect(); return r.right > b.right + 0.5 || r.left < b.left - 0.5 || r.right > innerWidth + 0.5; }).map((e) => e.getAttribute('aria-label') || e.textContent.trim());
        // Whole groups: the Mask group's three controls, and Pick mask paths..Clear mask, share one line; so do the Parts label, chips and buttons of one part.
        const line = (sel) => { const e = document.querySelector(sel); if (!e) return null; const ys = [...e.querySelectorAll('button')].map((x) => Math.round(x.getBoundingClientRect().top)); return [...new Set(ys)].length; };
        const sc = document.scrollingElement;
        return { small, outside, mask: line('[data-trace-mask]'), parts: line('[data-trace-parts]'), hscroll: sc.scrollWidth > innerWidth + 1, barScroll: bar.scrollWidth > bar.clientWidth + 1, bottom: b.bottom, vh: innerHeight, height: Math.round(b.height) };
      })())`)) as string) as { small: string[]; outside: string[]; mask: number | null; parts: number | null; hscroll: boolean; barScroll: boolean; bottom: number; vh: number; height: number };
      assert.deepEqual(m.small, [], `${tag}: every target is at least 28 x 28`);
      assert.deepEqual(m.outside, [], `${tag}: nothing past the bar's edge`);
      assert.equal(m.mask, 1, `${tag}: the Mask group is one line`);
      if (m.parts !== null) {
        assert.equal(m.parts, 1, `${tag}: the Parts group is one line (1 part)`);
      }
      assert.equal(m.hscroll, false, `${tag}: no horizontal scroll of the page`);
      assert.equal(m.barScroll, false, `${tag}: no horizontal scroll in the bar`);
      return m.height;
    };
    for (const open of [true, false]) {
      await setPanel(s, open);
      for (const advanced of [false, true]) {
        await setAdvanced(s, advanced);
        await survey(`panel ${open ? "open" : "closed"}, Advanced ${advanced ? "open" : "closed"}`);
      }
    }
    // Three parts, panel open: Parts and Analyse share a line, Parts is not split (§24.3 table).
    await setPanel(s, true);
    await setAdvanced(s, true);
    for (let i = 0; i < 2; i++) {
      await s.clickSel(`${ADV} [data-trace-add-part]`);
      await sleep(600);
    }
    await survey("3 parts");
    const parts = (await rectOf(s, `${ADV} [data-trace-parts]`))!;
    const analyse = (await rectOf(s, `${ADV} [data-trace-analyse]`))!;
    assert.ok(Math.abs(analyse.y - parts.y) < 14 && analyse.x >= parts.right - 1, "Parts (192) and Analyse (75) share a line: 192 + 8 + 75 = 275 fits 304");
    // Eight parts: the group wraps inside itself (label and chips first, Add and Remove on a second line), one Tab stop group.
    for (let i = 0; i < 5; i++) {
      await s.clickSel(`${ADV} [data-trace-add-part]`);
      await sleep(500);
    }
    assert.equal(await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-part]').length`), 8);
    const eight = (await rectOf(s, `${ADV} [data-trace-parts]`))!;
    const add = (await rectOf(s, `${ADV} [data-trace-add-part]`))!;
    const lastChip = (await rectOf(s, `${ADV} [data-trace-part="8"]`))!;
    assert.ok(eight.w <= 304 + 1, `8 parts fit the block: ${eight.w}`);
    assert.ok(add.y >= lastChip.bottom - 1, "Add part wraps to a second line (7 to 8 parts)");
  }, "", 800, 600);
});

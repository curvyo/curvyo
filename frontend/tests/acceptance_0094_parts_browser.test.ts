// Chromium cases for `specs/0094-masked-composite-trace` slice 2, Parts and Join (criteria 18, 19,
// 24, 28, 29, 32, 42, 43; UX notes §3 to §5, §8; `specs/0089` UX notes §24.3: the Parts group in the
// Advanced block's last row, after Corners, then Analyse): chip 1, picks into part 1, Add part with
// picks into part 2, a chip pick loads its mask, Create paths makes one group in one step, the
// inline confirm removes a part, Add part stops at 8, the last row at 800 x 600. Set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built. The harness is the one of
// `acceptance_0094_pick_browser.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5397;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0094-parts");
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

/** The bar's one hidden status region (`specs/0089` UX notes §24.5): it says the counts and "2 parts". */
const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);
const settledCounts = (s: S) => waitFor("a trace result", async () => (/(^|\. )(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);


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
const maskState = async (s: S) =>
  (await s.evaluate(`(() => { const g = document.querySelector('${BAR} [role=radiogroup][aria-label=Mask]'); if (!g) return 'absent';
    return [...g.querySelectorAll('[role=radio]')].map((r) => r.textContent.trim() + '=' + r.getAttribute('aria-checked')).join(','); })()`)) as string;
const advText = advancedStatus;

// ------------------------------------------------------------------ slice 1b helpers

const PICK = `${BAR} button[aria-label="Pick mask paths"]`;
const CLEAR = `${BAR} button[aria-label="Clear mask"]`;
const pressedState = async (s: S) => (await s.evaluate(`document.querySelector('${PICK}')?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
const hasClear = async (s: S) => (await s.evaluate(`!!document.querySelector('${CLEAR}')`)) as boolean;
const canvasCursor = async (s: S) =>
  (await s.evaluate(`(() => { const c = [...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0];
    const e = c.closest('[style*="cursor"]'); return e ? e.style.cursor : ''; })()`)) as string;
const focusedName = async (s: S) => (await s.evaluate(`(() => { const e = document.activeElement; return e ? (e.getAttribute('aria-label') || e.textContent.trim()) : ''; })()`)) as string;
const traceRows = async (s: S) => (await s.layerRows()).filter((r) => r?.startsWith("Trace of")).length;
const openTrace = async (s: S) => {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
};

/** Imports M1, draws rectangle A over its left half and a small rectangle B over its bottom right
 * corner (the top is under the bar), selects the picture alone and opens Trace. Returns where a press hits A's and B's edges
 * outside the picture, and a point of the picture outside both. */
async function pickSetup(s: S) {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const w = box.x1 - box.x0;
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  await drag(s, [box.x1 + 30, box.y1 - 0.25 * h], [box.cx + 0.2 * w, box.y1 - 0.02 * h]);
  await waitFor("two rectangles", async () => ((await s.layerRows()).filter((r) => r?.endsWith("rectangle")).length === 2 ? true : null), 10_000);
  await s.key("s", 83);
  await focusCanvas(s);
  await press(s, "Escape", 27);
  const image: [number, number] = [box.cx + 0.3 * w, box.cy - 0.1 * h];
  await s.click(image[0], image[1]);
  await waitFor("the picture alone selected", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await openTrace(s);
  return { box, a: [box.x0 - 30, box.cy] as [number, number], b: [box.x1 + 30, box.y1 - 0.1 * h] as [number, number], image };
}

// ------------------------------------------------------------------ slice 2 cases

const PARTS_GROUP = `${ADV} [role=radiogroup][aria-label=Parts]`;
const ADD = `${ADV} button[aria-label="Add part"]`;
const TRASH = `${ADV} [data-trace-remove-part]`;
const chipNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PARTS_GROUP} [role=radio]')].map((c) => c.getAttribute('aria-label')))`)) as string) as string[];
const partLabel = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-part-label]')?.textContent ?? ''`)) as string;
const rectangles = async (s: S) => (await s.layerRows()).filter((r) => r?.endsWith("rectangle")).length;

test("0094 c18, c19, c24, c32, c42: chip 1, picks into each part, a chip pick loads its mask, one group in one step", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const at = await pickSetup(s);
    await setAdvanced(s, true);
    // c18: the Parts group with its label, chip 1 (the whole image) and Add part.
    assert.equal(await s.evaluate(`document.querySelector('${ADV} [data-trace-parts] > span[aria-hidden]')?.textContent`), "Parts");
    assert.deepEqual(await chipNames(s), ["Part 1 of 1, whole image, active"]);
    assert.equal(await s.evaluate(`document.querySelector('${ADD}')?.getAttribute('aria-disabled')`), null);
    assert.equal(await partLabel(s), "", "no part label with one part");
    // Part 1: rectangle A, picked.
    await s.clickSel(PICK);
    await s.click(at.a[0], at.a[1]);
    await waitFor("part 1 with A", async () => ((await chipNames(s))[0] === "Part 1 of 1, inside, 1 path, active" ? true : null), 10_000);
    // c19: Add part, active, no paths, muted; the pick mode stays armed (c42).
    await s.clickSel(ADD);
    await waitFor("chip 2", async () => ((await chipNames(s)).length === 2 ? true : null), 10_000);
    assert.deepEqual(await chipNames(s), ["Part 1 of 2, inside, 1 path", "Part 2 of 2, no paths, active"]);
    assert.equal(await pressedState(s), "true", "Add part leaves the mode armed");
    assert.equal(await partLabel(s), "", "`specs/0089` ux-notes §27: no 'Part 2 of 2' label");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "part 2 starts Inside, the group stays");
    // A pick goes into the active part only.
    await s.click(at.b[0], at.b[1]);
    await waitFor("part 2 with B", async () => ((await chipNames(s))[1] === "Part 2 of 2, inside, 1 path, active" ? true : null), 10_000);
    assert.equal((await chipNames(s))[0], "Part 1 of 2, inside, 1 path");
    await settledCounts(s);
    assert.match(await advText(s), /2 parts/, "the status segment");
    // A chip pick makes part 1 active with its mask; the mode stays armed.
    await s.clickSel(`${PARTS_GROUP} [data-trace-part="1"]`);
    await waitFor("part 1 active", async () => ((await chipNames(s))[0] === "Part 1 of 2, inside, 1 path, active" ? true : null), 10_000);
    assert.equal(await maskState(s), "Inside=true,Outside=false");
    assert.equal(await pressedState(s), "true");
    assert.equal(await partLabel(s), "");
    // c42: Clear mask empties part 1 only.
    await s.clickSel(CLEAR);
    await waitFor("part 1 whole again", async () => ((await chipNames(s))[0] === "Part 1 of 2, whole image, active" ? true : null), 10_000);
    assert.equal((await chipNames(s))[1], "Part 2 of 2, inside, 1 path");
    // c24: Create paths joins the parts into one group in one step; the mask paths stay.
    await s.clickSel(PICK);
    await settledCounts(s);
    await s.clickSel(`${BAR} button[data-trace-slot=none]`);
    await waitFor("one trace group", async () => ((await traceRows(s)) === 1 ? true : null), 60_000);
    assert.equal(await rectangles(s), 2, "the mask paths are unchanged");
    assert.doesNotMatch(await s.statusText(), /created|traced \d/i, "no success notice");
    await s.key("z", 90, 2);
    await waitFor("undone", async () => ((await traceRows(s)) === 0 ? true : null), 10_000);
    assert.equal(await rectangles(s), 2);
  });
});

test("0094 c18, c28, c29, c43: Add part stops at 8, the trash removes with the inline confirm, the row at 800 x 600", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(
    async (s) => {
      await pickSetup(s);
      await setAdvanced(s, true);
      for (let n = 2; n <= 8; n++) {
        await s.clickSel(ADD);
        await waitFor(`chip ${n}`, async () => ((await chipNames(s)).length === n ? true : null), 10_000);
      }
      assert.equal(await s.evaluate(`document.querySelector('${ADD}').getAttribute('aria-disabled')`), "true", "Add part is dimmed at 8");
      await s.clickSel(ADD);
      assert.equal((await chipNames(s)).length, 8);
      await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
      await sleep(800);
      // c43: every target at least 28 px, no horizontal scroll, the row wraps as whole items.
      const sizes = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${ADV} [data-trace-parts] button')].map((b) => { const r = b.getBoundingClientRect(); return [r.width, r.height]; }))`)) as string) as [number, number][];
      assert.ok(sizes.length === 10 && sizes.every(([w, h]) => w >= 28 && h >= 28), JSON.stringify(sizes));
      assert.equal(await s.evaluate(`document.documentElement.scrollWidth <= window.innerWidth`), true, "no horizontal scroll");
      // c28: the first press arms in soft red, the second after 400 ms removes the part.
      await s.clickSel(TRASH);
      assert.match((await s.evaluate(`document.querySelector('${ADV} [data-trace-parts]').className`)) as string, /destructive-soft/);
      assert.equal(await s.evaluate(`document.querySelector('${TRASH}').getAttribute('aria-label')`), "Press again to remove part 8");
      assert.equal((await chipNames(s)).length, 8, "the first press removes nothing");
      await sleep(500);
      await s.clickSel(TRASH);
      await waitFor("7 chips", async () => ((await chipNames(s)).length === 7 ? true : null), 10_000);
      assert.equal((await chipNames(s))[6], "Part 7 of 7, no paths, active", "the part before becomes active, chips renumber");
      assert.doesNotMatch((await s.evaluate(`document.querySelector('${ADV} [data-trace-parts]').className`)) as string, /destructive-soft/);
      // No hint text in the row.
      for (const text of await barStrings(s)) {
        assert.doesNotMatch(text, HINT, text);
      }
      // c29: leaving the tool and entering again leaves one part.
      await focusCanvas(s);
      await press(s, "Escape", 27);
      await s.key("s", 83);
      await openTrace(s);
      await setAdvanced(s, true);
      assert.equal((await chipNames(s)).length, 1);
    },
  );
});

test("0089 §24.3: at 800 x 600 with the panel open the last row is Corners | Parts | Analyse, wrapping by whole groups", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await pickSetup(s);
    await setAdvanced(s, true);
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await sleep(800);
    if (await s.evaluate(`!!document.querySelector('[aria-label="Show panel"]')`)) {
      await s.clickSel('[aria-label="Show panel"]');
      await sleep(600);
    }
    const lastRow = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => { const block = document.querySelector('${ADV}');
        const top = (e) => Math.round(e.getBoundingClientRect().top);
        const corners = block.querySelector('[role=spinbutton][aria-label=Corners]').parentElement;
        const parts = block.querySelector('[data-trace-parts]');
        const analyse = block.querySelector('[data-trace-analyse]');
        const lines = [...parts.querySelectorAll('button')].map(top).sort((a, b) => a - b).filter((t, i, all) => i === 0 || t - all[i - 1] > 10).length;
        const targets = [...block.querySelectorAll('button, [role=spinbutton]')].filter((b) => b.getBoundingClientRect().width > 0).map((b) => Math.min(b.getBoundingClientRect().width, b.getBoundingClientRect().height));
        const inner = block.getBoundingClientRect().right - parseFloat(getComputedStyle(block).paddingRight);
        return { cornersTop: top(corners), partsTop: top(parts), analyseTop: top(analyse), partsLines: lines,
          partsWidth: Math.round(parts.getBoundingClientRect().width), blockWidth: Math.round(block.clientWidth),
          analyseRight: Math.round(inner - analyse.getBoundingClientRect().right), minTarget: Math.min(...targets),
          blockScroll: block.scrollWidth > block.clientWidth, pageScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth }; })())`)) as string) as {
        cornersTop: number; partsTop: number; analyseTop: number; partsLines: number; partsWidth: number; blockWidth: number; analyseRight: number; minTarget: number; blockScroll: boolean; pageScroll: boolean;
      };
    const shots: Record<number, Awaited<ReturnType<typeof lastRow>>> = {};
    for (const n of [1, 3, 8]) {
      while ((await chipNames(s)).length < n) {
        const was = (await chipNames(s)).length;
        await s.clickSel(ADD);
        await waitFor(`chip ${was + 1}`, async () => ((await chipNames(s)).length === was + 1 ? true : null), 10_000);
      }
      await sleep(400);
      shots[n] = await lastRow();
      const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
      writeFileSync(path.join(SCRATCH, `parts-${n}-800x600.png`), Buffer.from(shot.result?.data ?? "", "base64"));
    }
    console.log("last row at 800 x 600, 1, 3 and 8 parts:", JSON.stringify(shots));
    for (const [n, r] of Object.entries(shots)) {
      assert.equal(r.blockScroll || r.pageScroll, false, `no horizontal scroll with ${n} parts`);
      assert.ok(r.minTarget >= 28, `every target at least 28 px with ${n} parts, got ${r.minTarget}`);
      assert.ok(r.analyseRight <= 1, `Analyse at the far right with ${n} parts, ${r.analyseRight} px short`);
      assert.ok(r.analyseTop >= r.partsTop, "Analyse after Parts");
    }
    // One part, 128 px: beside Corners; Analyse on its own line.
    assert.ok(Math.abs(shots[1].partsWidth - 128) <= 2, `one part is 128 px, got ${shots[1].partsWidth}`);
    // A 28 px control sits 4 px lower in its 36 px line than a 36 px item.
    assert.ok(Math.abs(shots[1].partsTop - shots[1].cornersTop) <= 6, "Corners and Parts share a line with one part");
    assert.ok(shots[1].analyseTop > shots[1].partsTop, "Analyse wraps to its own line with one part");
    // Three parts, 192 px: Parts on its own line, Analyse beside it.
    assert.ok(shots[3].partsTop > shots[3].cornersTop + 20, "Parts moves to its own line with three parts");
    assert.ok(Math.abs(shots[3].analyseTop - shots[3].partsTop) <= 4, "Parts and Analyse share a line with three parts");
    assert.equal(shots[3].partsLines, 1, "the group is one line with three parts");
    // Eight parts: the group wraps inside itself (Add and Remove on a second line), Analyse below.
    assert.equal(shots[8].partsLines, 2, "Add and Remove on a second line with eight parts");
    assert.ok(shots[8].analyseTop > shots[8].partsTop, "Analyse below the parts with eight parts");
  });
});

test("0089 §24.7: Ignore below keeps its label and the widest value apart by 10 px or more; the other fields are 150 px", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await pickSetup(s);
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await sleep(800);
    await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="Ignore below"]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "End", code: "End", windowsVirtualKeyCode: 35 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "End", code: "End", windowsVirtualKeyCode: 35 });
    await sleep(1500);
    const m = JSON.parse((await s.evaluate(`JSON.stringify((() => {
      const fields = [...document.querySelectorAll('${BAR} [role=spinbutton]')];
      const f = fields.find((e) => e.getAttribute('aria-label') === 'Ignore below');
      const [label, value] = [...f.querySelectorAll('span[aria-hidden]')].filter((x) => x.textContent.trim());
      const number = value.firstElementChild ?? value;
      const ctx = document.createElement('canvas').getContext('2d');
      ctx.font = getComputedStyle(number).font;
      return { text: value.textContent.trim(), gap: value.getBoundingClientRect().left - label.getBoundingClientRect().right,
        widest: ctx.measureText('99.99').width - ctx.measureText(number.textContent.trim()).width,
        widths: fields.map((e) => [e.getAttribute('aria-label'), Math.round(e.getBoundingClientRect().width)]) }; })())`)) as string) as
      { text: string; gap: number; widest: number; widths: [string, number][] };
    console.log("Ignore below at its largest value:", JSON.stringify(m));
    assert.match(m.text, /^10\.00\s*mm$/);
    assert.ok(m.gap - Math.max(0, m.widest) >= 10, `"99.99 mm" keeps 10 px from the label: gap ${m.gap}, wider by ${m.widest}`);
    for (const [name, width] of m.widths) {
      assert.equal(width, name === "Ignore below" ? 171 : 150, `${name} is ${width} px`);
    }
  });
});

test("0089 §27.8: with Multicolour a new part has no palette until it has paths; the palette is the active part's; a part another part covers shows none", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const at = await pickSetup(s);
    await setAdvanced(s, true);
    await s.evaluate(`[...document.querySelectorAll('${ADV} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await sleep(1500);
    const hexes = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-palette-swatch]')].map((e) => e.getAttribute('data-trace-palette-swatch')))`)) as string) as string[];
    await s.clickSel(PICK);
    await s.click(at.a[0], at.a[1]);
    await waitFor("part 1 with A", async () => ((await chipNames(s))[0] === "Part 1 of 1, inside, 1 path, active" ? true : null), 10_000);
    await waitFor("part 1 palette", async () => ((await hexes()).length >= 1 ? true : null), 30_000);
    // Add part: no paths, so nothing to trace and none of part 1's colours.
    await s.clickSel(ADD);
    await waitFor("chip 2", async () => ((await chipNames(s)).length === 2 ? true : null), 10_000);
    await waitFor("no palette for the part without paths", async () => ((await hexes()).length === 0 ? true : null), 15_000);
    // Its own paths: its own palette. Part 1 is covered by part 2 now and adds nothing.
    await s.click(at.a[0], at.a[1]);
    await waitFor("part 2 with A", async () => ((await chipNames(s))[1] === "Part 2 of 2, inside, 1 path, active" ? true : null), 10_000);
    await waitFor("part 2 palette", async () => ((await hexes()).length >= 1 ? true : null), 60_000);
    await s.clickSel(`${PARTS_GROUP} [data-trace-part="1"]`);
    await waitFor("part 1 adds nothing, no palette", async () => ((await hexes()).length === 0 ? true : null), 60_000);
  });
});

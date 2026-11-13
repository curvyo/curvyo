// Chromium cases C1 to C15 of `specs/0114-image-trace-mask/ux-notes.md` section 10 (the regions
// and the Lasso on the canvas, their keys and the History steps; C15, the turned image, is covered by
// `curvyo-editor-wasm/tests/acceptance_0114_parts.rs` c13 because the chip and the rotate handle did not turn the image under DevTools input), in the real app over DevTools;
// set CURVYO_E2E_BROWSER=/path/to/chromium and build the wasm bindings first. Screenshots at scale 1.
// Harness as in `acceptance_0094_parts_browser.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5405;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0114-canvas");
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

/** The bar's one hidden status region (`specs/0089` UX notes §24.5): it says the counts and "2 parts". */
const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
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

/** Presses the Mask group's Rectangle, drags `from` to `to` over the picture and releases the tool
 * (`specs/0114`: a region of the active part). */
async function drawRegion(s: S, from: [number, number], to: [number, number]) {
  await s.clickSel(`${BAR} [data-trace-mask-rect]`);
  await drag(s, from, to);
  await s.clickSel(`${BAR} [data-trace-mask-rect]`);
  await sleep(600);
}
const canvasCursor = async (s: S) =>
  (await s.evaluate(`(() => { const c = [...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0];
    const e = c.closest('[style*="cursor"]'); return e ? e.style.cursor : ''; })()`)) as string;
const openTrace = async (s: S) => {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
};


// ------------------------------------------------------------------ 0114 canvas helpers

const MASK = `${BAR} [data-trace-mask]`;
const RECT_BTN = `${MASK} [data-trace-mask-rect]`;
const LASSO_BTN = `${MASK} [data-trace-mask-lasso]`;
const SUB_BTN = `${MASK} [data-trace-mask-subtract]`;
type Box = { x0: number; x1: number; y0: number; y1: number; cx: number; cy: number };
const P = (b: Box, fx: number, fy: number): [number, number] => [b.x0 + fx * (b.x1 - b.x0), b.y0 + fy * (b.y1 - b.y0)];
const pressedOf = async (s: S, sel: string) => (await s.evaluate(`document.querySelector(${JSON.stringify(sel)})?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
const lum = ([r, g, b]: number[]) => 0.299 * r + 0.587 * g + 0.114 * b;

/** Imports M1, opens Trace with Advanced open, the History tab shown. */
async function traceOn(s: S) {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  await openTrace(s);
  await setAdvanced(s, true);
  await s.tab("History");
  return box;
}
const history = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=list][aria-label=History] [role=listitem]')].map((r) => r.textContent))`)) as string) as string[];
const count = async (s: S, label: string) => (await history(s)).filter((t) => t.includes(label)).length;

const mouse = (s: S, type: string, p: [number, number], extra: Record<string, unknown> = {}) => s.send("Input.dispatchMouseEvent", { type, x: p[0], y: p[1], ...extra });
const leftClick = async (s: S, p: [number, number], clickCount = 1) => {
  await mouse(s, "mouseMoved", p);
  await mouse(s, "mousePressed", p, { button: "left", buttons: 1, clickCount });
  await mouse(s, "mouseReleased", p, { button: "left", buttons: 0, clickCount });
  await sleep(250);
};
/** Two presses 40 ms apart at `p`: the app's own detector reads a double click from the timing. */
const doubleClick = async (s: S, p: [number, number]) => {
  await mouse(s, "mouseMoved", p);
  for (const n of [1, 2]) {
    await mouse(s, "mousePressed", p, { button: "left", buttons: 1, clickCount: n });
    await mouse(s, "mouseReleased", p, { button: "left", buttons: 0, clickCount: n });
    await sleep(40);
  }
  await sleep(300);
};
const rightClick = async (s: S, p: [number, number]) => {
  await mouse(s, "mouseMoved", p);
  await mouse(s, "mousePressed", p, { button: "right", buttons: 2, clickCount: 1 });
  await mouse(s, "mouseReleased", p, { button: "right", buttons: 0, clickCount: 1 });
  await sleep(400);
};
const hover = async (s: S, p: [number, number]) => {
  await mouse(s, "mouseMoved", p);
  await sleep(200);
};
/** Lasso L: three corners below the bar; returns them. */
const corners = (b: Box): [number, number][] => [P(b, 0.15, 0.55), P(b, 0.55, 0.6), P(b, 0.45, 0.9)];
const clickCorners = async (s: S, pts: [number, number][]) => {
  for (const p of pts) await leftClick(s, p);
};
const pressTool = async (s: S, sel: string, want: "true" | "false") => {
  if ((await pressedOf(s, sel)) !== want) await s.clickSel(sel);
  assert.equal(await pressedOf(s, sel), want);
};
const enter = (s: S) => press(s, "Enter", 13);
const esc = (s: S) => press(s, "Escape", 27);
const canvasFocus = async (s: S, b: Box) => {
  // A click on the pasteboard outside the picture would draw with a tool pressed: focus by script.
  void b;
  await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
};

// ------------------------------------------------------------------ C7, C10, C11, C12, C13

test("0114 C7, C10, C11, C12, C13: the four ways to finish a lasso, a right click, Esc, auto close and no create on a double click", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const box = await traceOn(s);
    await s.evaluate(`window.__ctx = []; window.addEventListener('contextmenu', (e) => window.__ctx.push(e.defaultPrevented));`);
    await pressTool(s, LASSO_BTN, "true");
    const L = corners(box);
    // C7: each finish gives one "Draw lasso".
    await clickCorners(s, L);
    await leftClick(s, [L[0][0] + 3, L[0][1] + 2]); // the first corner
    assert.equal(await count(s, "Draw lasso"), 1, "C7: the first corner finishes");
    await clickCorners(s, L.slice(0, 2));
    await doubleClick(s, L[2]); // the first click sets the third corner, the second adds none
    await sleep(300);
    assert.equal(await count(s, "Draw lasso"), 2, "C7: a double click finishes");
    await clickCorners(s, L);
    await canvasFocus(s, box);
    await enter(s);
    assert.equal(await count(s, "Draw lasso"), 3, "C7: Enter finishes");
    await clickCorners(s, L);
    await rightClick(s, P(box, 0.9, 0.95));
    assert.equal(await count(s, "Draw lasso"), 4, "C10: a right click finishes: one more step");
    assert.equal(await pressedOf(s, LASSO_BTN), "true", "C10: Lasso stays pressed");
    assert.deepEqual(await s.evaluate(`JSON.stringify(window.__ctx)`), JSON.stringify([true]), "C10: the contextmenu event was prevented");
    assert.equal(await s.evaluate(`!!document.querySelector('[role=menu]')`), false, "C10: no menu element");
    // A right press with no lasso in progress changes nothing.
    await rightClick(s, P(box, 0.9, 0.95));
    assert.equal(await count(s, "Draw lasso"), 4);
    assert.equal(await pressedOf(s, LASSO_BTN), "true", "a right press without a lasso keeps Lasso");
    // C11: Esc discards; the next Esc releases the tool.
    await clickCorners(s, L);
    await canvasFocus(s, box);
    await esc(s);
    assert.equal(await count(s, "Draw lasso"), 4, "C11: Esc stores nothing");
    assert.equal(await pressedOf(s, LASSO_BTN), "true", "C11: Lasso pressed after the first Esc");
    await esc(s);
    assert.equal(await pressedOf(s, LASSO_BTN), "false", "C11: released by the second Esc");
    // C12: other ends close and store.
    await pressTool(s, LASSO_BTN, "true");
    await clickCorners(s, L);
    await s.clickSel(RECT_BTN);
    await sleep(400);
    assert.equal(await count(s, "Draw lasso"), 5, "C12: pressing Rectangle closes the lasso");
    assert.equal(await pressedOf(s, RECT_BTN), "true");
    await pressTool(s, LASSO_BTN, "true");
    await clickCorners(s, L);
    await s.clickSel(`${ADV} button[aria-label="Add part"]`);
    await sleep(500);
    assert.equal(await count(s, "Draw lasso"), 6, "C12: Add part closes the lasso");
    await clickCorners(s, L);
    await clickChipN(s, 1);
    assert.equal(await count(s, "Draw lasso"), 7, "C12: picking a chip closes the lasso");
    await clickCorners(s, L);
    await canvasFocus(s, box);
    await s.key("s", 83);
    await sleep(600);
    assert.equal(await count(s, "Draw lasso"), 8, "C12: S leaves the tool and closes the lasso");
    // C13: a double click with Lasso and no lasso in progress creates no paths.
    await openTrace(s);
    await setAdvanced(s, true);
    await pressTool(s, LASSO_BTN, "true");
    await doubleClick(s, P(box, 0.5, 0.3));
    await sleep(600);
    assert.equal(await count(s, "Trace image"), 0, "C13: no Create paths");
    assert.equal(await count(s, "Draw lasso"), 8, "C13: a lasso of one corner is not stored");
  });
});

const clickChipN = (s: S, n: number) => s.evaluate(`[...document.querySelectorAll('${ADV} [role=radiogroup][aria-label=Parts] [role=radio]')][${n - 1}].click()`).then(() => sleep(600));

// ------------------------------------------------------------------ C5, C6, C8, C9

test("0114 C5, C6, C9: the Lasso in progress, its close target and the Remove badge", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    const box = await traceOn(s);
    await pressTool(s, LASSO_BTN, "true");
    const L = corners(box);
    await clickCorners(s, L);
    const pointer = P(box, 0.2, 0.95);
    await hover(s, pointer);
    const img = await shot(s);
    const accent = ([r, g, b]: number[]) => b > 200 && r < 110 && g < 160;
    // C5: a hollow 14 x 14 square at each corner: the edge pixel 7 px from the centre.
    for (const [i, c] of L.entries()) {
      const edge = [at(img, c[0] + 7, c[1]), at(img, c[0] - 7, c[1]), at(img, c[0], c[1] + 7), at(img, c[0], c[1] - 7)];
      assert.ok(edge.some(accent), `C5: corner ${i + 1} has its accent square edge: ${JSON.stringify(edge)}`);
    }
    // The side between the first two corners and the rubber band to the pointer are accent lines.
    const along = (a: [number, number], b: [number, number], t: number) => at(img, a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
    const sideHits = [0.3, 0.5, 0.7].filter((t) => accent(along(L[0], L[1], t)) || accent(at(img, L[0][0] + (L[1][0] - L[0][0]) * t, L[0][1] + (L[1][1] - L[0][1]) * t - 1)));
    assert.ok(sideHits.length >= 2, `C5: the side is an accent line (${sideHits.length} of 3 samples)`);
    const bandHits = [0.3, 0.5, 0.7].filter((t) => [0, 1, -1].some((d) => accent(at(img, L[2][0] + (pointer[0] - L[2][0]) * t, L[2][1] + (pointer[1] - L[2][1]) * t + d))));
    assert.ok(bandHits.length >= 2, `C5: the rubber band to the pointer is an accent line (${bandHits.length} of 3)`);
    // C9: a lasso with one corner shows the Remove badge when Subtract is pressed.
    await s.evaluate(`document.activeElement?.blur?.()`);
    // C6: the pointer within 16 px of the first corner: the closing side is dashed accent.
    await hover(s, [L[0][0] + 6, L[0][1] + 4]);
    const img2 = await shot(s);
    let dashed = 0;
    for (let t = 0.15; t < 0.9; t += 0.02) {
      const x = L[2][0] + (L[0][0] - L[2][0]) * t;
      const y = L[2][1] + (L[0][1] - L[2][1]) * t;
      if ([0, 1, -1].some((d) => accent(at(img2, x + d, y)) || accent(at(img2, x, y + d)))) dashed++;
    }
    assert.ok(dashed >= 8 && dashed <= 33, `C6: the closing side from the last corner to the first is dashed (${dashed} of 38 samples)`);
    // Esc discards; then Subtract with a one-corner lasso shows the badge.
    await canvasFocus(s, box);
    await esc(s);
    await pressTool(s, SUB_BTN, "true");
    await leftClick(s, L[0]);
    await hover(s, [L[0][0] + 60, L[0][1] + 20]);
    const badge = (await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('[data-move-badge=remove]'); if (!e) return null; const r = e.getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })())`)) as string;
    assert.ok(badge !== "null", "C9: the Remove badge is drawn with Subtract and a lasso in progress");
    const [bx, by] = JSON.parse(badge) as [number, number];
    assert.ok(Math.abs(bx - (L[0][0] + 60 - 16)) <= 4 && Math.abs(by - (L[0][1] + 20 + 16)) <= 4, `C9: the badge centre ${bx},${by} is 16 px left of and 16 px below the pointer`);
    await esc(s);
    await sleep(300);
    assert.equal(await s.evaluate(`!!document.querySelector('[data-move-badge=remove]')`), false, "C9: gone after Esc");
    // Rectangle with Subtract: the badge while the drag is held, gone after the release.
    await pressTool(s, RECT_BTN, "true");
    const a = P(box, 0.6, 0.15);
    const b = P(box, 0.85, 0.35);
    await mouse(s, "mouseMoved", a);
    await mouse(s, "mousePressed", a, { button: "left", buttons: 1, clickCount: 1 });
    await mouse(s, "mouseMoved", b, { button: "left", buttons: 1 });
    await sleep(300);
    assert.equal(await s.evaluate(`!!document.querySelector('[data-move-badge=remove]')`), true, "C9: the badge while the Subtract rectangle is dragged");
    await mouse(s, "mouseReleased", b, { button: "left", buttons: 0, clickCount: 1 });
    await sleep(400);
    assert.equal(await s.evaluate(`!!document.querySelector('[data-move-badge=remove]')`), false, "C9: gone after the release");
  });
});

// ------------------------------------------------------------------ C1 to C4, C8, C14

test("0114 C1 to C4, C8, C14: the washes, the region line, the selected rectangle and lasso, the cursors", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const box = await traceOn(s);
    const [rx0, ry0] = P(box, 0.5, 0.1);
    const [rx1, ry1] = P(box, 0.95, 0.95);
    await drawRegion(s, [rx0, ry0], [rx1, ry1]);
    await settledCounts(s);
    await sleep(800);
    const disc = P(box, 0.25, 0.42);
    const square = P(box, 0.75, 0.42);
    let img = await shot(s);
    // C1: Inside: the square inside the region keeps its ink, the disc outside is at 12 %.
    const inside = lum(at(img, ...square));
    const outside = lum(at(img, ...disc));
    assert.ok(outside > inside + 40, `C1: outside the region the picture is washed (disc ${Math.round(outside)} over square ${Math.round(inside)})`);
    // C3: a 1 px black line along the region's left edge.
    const edgeY = P(box, 0.5, 0.2)[1];
    const row = Array.from({ length: 9 }, (_, i) => lum(at(img, rx0 - 4 + i, edgeY)));
    // The line is 1 px wide at 100 % and may straddle two pixel columns: its darkness adds up to one black pixel.
    const darkness = row.reduce((n, v) => n + (255 - v), 0);
    assert.ok(darkness > 200 && darkness < 330 && Math.min(...row) < 140, `C3: a 1 px black line at the region's edge (darkness ${Math.round(darkness)}): ${JSON.stringify(row.map(Math.round))}`);
    // C2: Outside swaps the washes.
    await clickMask(s, "Outside");
    await settledCounts(s).catch(() => undefined);
    await sleep(1500);
    img = await shot(s);
    assert.ok(lum(at(img, ...square)) > lum(at(img, ...disc)) + 40, "C2: with Outside the square inside the region is washed and the disc keeps its ink");
    await clickMask(s, "Inside");
    await sleep(800);
    // C3: a subtracted region is dashed: runs of 4 on and 3 off along its top edge.
    await pressTool(s, SUB_BTN, "true");
    const sa = P(box, 0.58, 0.15);
    const sb = P(box, 0.9, 0.3);
    await drawRegion(s, sa, sb);
    await pressTool(s, SUB_BTN, "false");
    await sleep(600);
    img = await shot(s);
    const runs: number[] = [];
    let run = 0;
    for (let x = Math.round(sa[0]) + 3; x < Math.round(sb[0]) - 3; x++) {
      const dark = [-1, 0, 1, 2].some((d) => lum(at(img, x, Math.round(sa[1]) + d)) < 190);
      if (dark) run++;
      else if (run > 0) {
        runs.push(run);
        run = 0;
      }
    }
    const median = [...runs].sort((p, q) => p - q)[Math.floor(runs.length / 2)];
    assert.ok(runs.length >= 8 && median >= 3 && median <= 5, `C3: the subtracted region is dashed, runs ${JSON.stringify(runs)}`);
    // C14: no tool pressed, the cursor over R's body is the arrow; with Rectangle it is a crosshair.
    await pressTool(s, RECT_BTN, "true");
    await hover(s, P(box, 0.3, 0.8));
    assert.equal(await canvasCursor(s), "crosshair", "C14: crosshair with Rectangle pressed");
    await pressTool(s, RECT_BTN, "false");
    await hover(s, P(box, 0.7, 0.75));
    assert.match(await canvasCursor(s), /default|auto|^$/, "C14: the arrow over a region");
    // C4: a click inside the region selects it: the dashed accent box and 8 handles.
    await leftClick(s, P(box, 0.7, 0.75));
    await sleep(500);
    img = await shot(s);
    // The 1 px line may straddle two pixel rows: a blue cast (b - r) stands for the accent.
    const accent = ([r, , b]: number[]) => b - r > 60 && b > 180;
    let onEdge = 0;
    for (let x = Math.round(rx0) + 12; x < Math.round(rx1) - 12; x += 1) {
      if ([0, 1, -1].some((d) => accent(at(img, x, Math.round(ry1) + d)))) onEdge++;
    }
    const length = Math.round(rx1) - Math.round(rx0) - 24;
    assert.ok(onEdge > length * 0.35 && onEdge < length * 0.8, `C4: the dashed accent box along the region's bottom edge (${onEdge} of ${length} px)`);
    const corner = [at(img, rx1, ry1), at(img, rx1 - 4, ry1), at(img, rx1 + 4, ry1), at(img, rx1, ry1 - 4), at(img, rx1, ry1 + 4)];
    assert.ok(lum(corner[0]) > 225 && corner.slice(1).some(accent), `C4: a white 8 x 8 handle with an accent ring at the corner: ${JSON.stringify(corner)}`);
    // C14: the handle shows a resize cursor.
    await hover(s, [rx1, ry1]);
    assert.match(await canvasCursor(s), /url\(/, "C14: a resize cursor over a handle");
  });
});

// ------------------------------------------------------------------ C8, C15

test("0114 C8: a selected lasso has the Node tool's corner squares and no resize handles", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    const box = await traceOn(s);
    await pressTool(s, LASSO_BTN, "true");
    const L = corners(box);
    await clickCorners(s, L);
    await canvasFocus(s, box);
    await enter(s);
    await pressTool(s, LASSO_BTN, "false");
    await leftClick(s, P(box, 0.4, 0.68));
    await sleep(500);
    // Press corner 2: the selected corner is filled accent.
    await mouse(s, "mouseMoved", L[1]);
    await mouse(s, "mousePressed", L[1], { button: "left", buttons: 1, clickCount: 1 });
    await sleep(300);
    const img = await shot(s);
    const accentFill = ([r, g, b]: number[]) => r < 90 && g < 140 && b > 200;
    assert.ok(accentFill(at(img, L[1][0], L[1][1])), `C8: the pressed corner is filled accent: ${JSON.stringify(at(img, L[1][0], L[1][1]))}`);
    for (const i of [0, 2]) {
      const c = L[i];
      assert.ok(lum(at(img, c[0], c[1])) > 225, `C8: corner ${i + 1} is a white square: ${JSON.stringify(at(img, c[0], c[1]))}`);
      const edge = [at(img, c[0] + 7, c[1]), at(img, c[0] - 7, c[1]), at(img, c[0], c[1] + 7), at(img, c[0], c[1] - 7)];
      assert.ok(edge.some((p) => lum(p) < 110), `C8: its 1 px dark edge: ${JSON.stringify(edge)}`);
    }
    await mouse(s, "mouseReleased", L[1], { button: "left", buttons: 0, clickCount: 1 });
  });
});


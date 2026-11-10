// Tester cases written from `specs/0089-trace-quality-and-colour/ux-notes.md` §27 (customer 2026-11-09) before the code was read:
// no visible status or notice text in the Trace bar (the hidden region still says it), no Isolate, the palette pipette with
// Remove background and its pairing with the background pipette, every palette colour deletable down to no colour (nothing
// traced, Enter does nothing, a pipette pick starts again), one colour traced alone, the bar's rows at 800 x 600 (§27 gives no
// new figure; the rows of §25.1 stay, the notice row only costs nothing). Real app in Chromium (CURVYO_E2E_BROWSER).
// Harness as in `acceptance_0089_palette_row_tester.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5283;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-s27-tester");
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
    ["--headless=new", "--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=4,availablePointerTypes=4", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
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
    vite.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  };
  return { s, close };
}

const BAR = "[role=toolbar][aria-label=Trace]";
type S = Awaited<ReturnType<typeof session>>;

/** A click on `sel` at (`dx`, `dy`) from its middle (CDP modifiers 2 = Ctrl). */
const clickOn = async (s: S, sel: string, modifiers = 0, dx = 0, dy = 0) => {
  const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x: c[0] + dx, y: c[1] + dy, button: "left", clickCount: 1, modifiers });
  }
  await sleep(300);
};
const press = async (s: S, k: string, vk: number, modifiers = 0) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await sleep(350);
};
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};

/** Imports `png` as `name` and opens the Trace bar on it. */
async function openOn(s: S, name: string, png: Buffer) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  const before = (await s.layerRows()).length;
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).length > before ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
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

// ------------------------------------------------------------------ criterion 21: Mask, pixel by pixel

/** 64 x 64: a bar on the left border, a big square, a wide shape with a hole, and a 2 x 2 speck. */

// ------------------------------------------------------------------ pictures

/** 320 x 140: red, green and blue blocks, adjacent; two near-greys; a pale accent. White ground. */
const RED = [200, 30, 30];
const GREEN = [30, 160, 60];
const BLUE = [30, 60, 200];
const GREY_A = [0xd9, 0xd9, 0xd9];
const GREY_B = [0xdc, 0xd6, 0xcf];
const PALE = [0xff, 0xf2, 0xa8];
const PICTURE = rgbPng(320, 140, (x, y) => {
  if (x >= 10 && x < 22 && y >= 10 && y < 22) return PALE as [number, number, number];
  if (y >= 40 && y < 130) {
    if (x >= 20 && x < 100) return RED as [number, number, number];
    if (x >= 100 && x < 160) return y < 85 ? (GREEN as [number, number, number]) : (BLUE as [number, number, number]);
    if (x >= 190 && x < 240) return GREY_A as [number, number, number];
    if (x >= 240 && x < 290) return GREY_B as [number, number, number];
  }
  return [255, 255, 255];
});

const OUTPUT = `${BAR} [role=radiogroup][aria-label=Output]`;
const PALETTE = `${BAR} [role=group][aria-label=Palette]`;
const swatchNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PALETTE} [data-trace-palette-swatch]')].map((o) => o.getAttribute('aria-label')))`)) as string) as string[];
const openAdvanced = async (s: S) => {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [role=group][aria-label=Advanced]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
};
const multicolour = async (s: S) => {
  await openAdvanced(s);
  await s.evaluate(`[...document.querySelectorAll('${OUTPUT} [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
  await sleep(1500);
};
/** Multicolour with the Colours field fixed to 6 (the automatic count is a separate todo test). */
const sixColours = async (s: S) => {
  await multicolour(s);
  await typeInto(s, "Colours", "6");
  await waitFor("six swatches", async () => ((await swatchNames(s)).length === 6 ? true : null), 30_000);
};
const swatch = (h: string) => `${PALETTE} [data-trace-palette-swatch="${h}"]`;

// ------------------------------------------------------------------ helpers of this file

const hiddenStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const centre = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
const moveTo = async (s: S, sel: string, modifiers = 0) => {
  const c = await centre(s, sel);
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c[0], y: c[1], modifiers });
  await sleep(300);
};
const away = async (s: S) => {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 700 });
  await sleep(300);
};
const rects = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll(${JSON.stringify(sel)})].map((e) => { const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; }))`)) as string) as number[][];

/** Records every trace request (with a copy of its pixels) and answer of the trace worker. */
const RECORD = `(() => {
  window.__reqs = []; window.__resps = {};
  const Base = window.Worker;
  window.Worker = class extends Base {
    constructor(...a) { super(...a); this.addEventListener('message', (e) => { if (e.data && e.data.kind === 'trace') window.__resps[e.data.id] = Array.from(e.data.wire); }); }
    postMessage(m, t) { if (m && m.type === 'trace') window.__reqs.push({ id: m.id, settings: m.settings, map: m.map, width: m.width, height: m.height, rgba: m.rgba.slice() }); return super.postMessage(m, t); }
  };
})();`;
type Req = { id: number; settings: string; n: number; done: boolean };
const lastRequest = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = window.__reqs[window.__reqs.length - 1]; return r ? { id: r.id, settings: r.settings, n: window.__reqs.length, done: !!window.__resps[r.id] } : null; })())`)) as string) as Req | null;
const settled = async (s: S) =>
  waitFor("the last trace answered", async () => {
    const r = await lastRequest(s);
    return r?.done ? r : null;
  }, 60_000);
/** The app's answer to its last request, and the same pixels traced with `settings` on an own wasm instance. */
const wires = async (s: S, settings: string) =>
  JSON.parse(
    (await s.evaluate(`(async () => {
      const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
      await m.default();
      const r = window.__reqs[window.__reqs.length - 1];
      const mine = Array.from(m.trace_job(r.rgba, r.width, r.height, Float64Array.from(r.map), ${JSON.stringify(settings)}, { report() {} }));
      return JSON.stringify({ app: window.__resps[r.id], mine });
    })()`)) as string,
  ) as { app: number[]; mine: number[] };
const coloursN = async (s: S, n: number) => {
  await multicolour(s);
  await typeInto(s, "Colours", String(n));
  await waitFor(`${n} swatches`, async () => ((await swatchNames(s)).length === n ? true : null), 30_000);
  await sleep(1500);
};
const hexesOf = async (s: S) => (await swatchNames(s)).map((n) => n.slice(0, 7));

type Shot = { width: number; height: number; data: Uint8Array };
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

const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
const px = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};
const boxOf = (img: Shot, rect: number[], test: (rgb: number[]) => boolean) => {
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (let y = Math.ceil(rect[1]); y < rect[1] + rect[3]; y++) {
    for (let x = Math.ceil(rect[0]); x < rect[0] + rect[2]; x++) {
      if (test(px(img, x, y))) {
        x0 = Math.min(x0, x);
        y0 = Math.min(y0, y);
        x1 = Math.max(x1, x);
        y1 = Math.max(y1, y);
      }
    }
  }
  return { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 };
};

// ------------------------------------------------------------------ helpers of this file

const TWO = rgbPng(360, 200, (x, y) => {
  if (y < 40 || y >= 160) return [255, 255, 255];
  if (x >= 30 && x < 160) return [200, 30, 30];
  if (x >= 200 && x < 330) return [30, 60, 200];
  return [255, 255, 255];
});
const DARK = rgbPng(200, 200, () => [20, 20, 20]);
const WHITE = rgbPng(200, 200, () => [255, 255, 255]);
const HOLES = rgbPng(600, 600, (x, y) => (x % 2 === 1 && y % 2 === 1 ? [255, 255, 255] : [0, 0, 0]));
const isRed = ([r, g, b]: number[]) => r > 200 && r - g > 40 && r - b > 40;
const isBlue = ([r, , b]: number[]) => b > 200 && r > 120 && b - r > 30;
const PAL_PIPETTE = `${BAR} [data-trace-pipette][aria-label="Pick palette colour from the image"]`;
const BG_PIPETTE = `${BAR} [data-trace-pipette][aria-label="Pick background colour from the image"]`;

/** Every visible leaf text in the bar (not the hidden region, not tooltips). */
const visibleTexts = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} *')].filter((e) => e.children.length === 0 && !e.closest('.sr-only, [hidden], [aria-hidden=true]') && e.textContent.trim()).map((e) => e.textContent.trim()))`)) as string) as string[];
const FORBIDDEN =
  /No colours found|Analysis failed|Slow to edit|\d+ paths?\b|covering the whole|Too much detail|can't be traced|Same as|Part \d+ of \d+|Tracing \d+ of|reduced copy|too (low|high)|Isolat|no colour|None picked|\d+ picked|at this Threshold|mask is outside|could not be combined|one image at a time/;
const noStatusText = async (s: S, where: string) => {
  const texts = await visibleTexts(s);
  assert.deepEqual(texts.filter((t) => FORBIDDEN.test(t)), [], `${where}: no status text in the bar; saw ${JSON.stringify(texts)}`);
  assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-state]')].some((e) => e.textContent.trim() !== '' || e.getBoundingClientRect().height > 0)`), false, `${where}: no notice row`);
  assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-hidden-status]').length`), 1, `${where}: the hidden region is mounted`);
  assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [data-trace-hidden-status]').closest('.sr-only') || document.querySelector('${BAR} [data-trace-hidden-status]').classList.contains('sr-only')`), true, `${where}: and sr-only`);
};
const setAdv = async (s: S, open: boolean) => {
  const now = (await s.evaluate(`!!document.querySelector('${BAR} [role=group][aria-label=Advanced]')`)) as boolean;
  if (now !== open) {
    await s.evaluate(`document.querySelector('${BAR} button[aria-expanded][aria-controls]').click()`);
    await sleep(500);
  }
};
const pressed = async (s: S, sel: string) => (await s.evaluate(`document.querySelector(${JSON.stringify(sel)})?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
const canvasRect = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as number[];
const barBottom = async (s: S) => (await s.evaluate(`document.querySelector('${BAR}').getBoundingClientRect().bottom`)) as number;
const pickAt = async (s: S, colour: (rgb: number[]) => boolean, modifiers = 0) => {
  await sleep(800);
  const [cx, , cw, ch] = await canvasRect(s);
  const found = boxOf(await shot(s), [cx, (await barBottom(s)) + 4, cw, ch], colour);
  assert.ok(Number.isFinite(found.cx), "the block to pick is on screen");
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: found.cx, y: found.cy, modifiers });
  for (const type of ["mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x: found.cx, y: found.cy, button: "left", clickCount: 1, modifiers });
  }
  await sleep(900);
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 2 });
};
const settledText = (s: S, re: RegExp, ms = 90_000) => waitFor(`hidden region ${re}`, async () => (re.test(await hiddenStatus(s)) ? hiddenStatus(s) : null), ms);

// ------------------------------------------------------------------ 27.1: no visible status text

test("0089 §27.1 tester: a normal Multicolour result draws no counts / notice text, Advanced open or closed; the hidden region says the counts", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pic.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    await settled(s);
    const said = await settledText(s, /\d+ paths?/);
    console.log("hidden:", said);
    await noStatusText(s, "Advanced open");
    await setAdv(s, false);
    await noStatusText(s, "Advanced closed");
  }, RECORD);
});

test("0089 §27.1 tester: one image covering the whole picture, 'Too much detail' and an empty Multicolour result are announced, never drawn", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "dark.png", DARK);
    await pickMethod(s, "Brightness");
    await settledText(s, /1 path covering the whole image\./);
    await noStatusText(s, "covering the whole image");
    await setAdv(s, false);
    await noStatusText(s, "covering the whole image, closed");
  });
  await withApp(async (s) => {
    await openOn(s, "holes.png", HOLES);
    await pickMethod(s, "Brightness");
    await typeInto(s, "Ignore below", "0");
    await settledText(s, /Too much detail: raise Ignore below\./, 150_000);
    await noStatusText(s, "too much detail");
  });
  await withApp(async (s) => {
    await openOn(s, "white.png", WHITE);
    await pickMethod(s, "Remove background");
    await multicolour(s);
    await sleep(4000);
    console.log("hidden (white, Multicolour):", await hiddenStatus(s));
    await noStatusText(s, "white image, Multicolour");
    await pickMethod(s, "Brightness");
    await sleep(3000);
    console.log("hidden (white, Brightness):", await hiddenStatus(s));
    await noStatusText(s, "white image, Brightness");
  });
});

test("0089 §27.1 tester: Pick colours with nothing picked: Trace is aria-disabled with the reason as description, no visible text, Enter does nothing", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO);
    await pickMethod(s, "Pick colours");
    await sleep(2000);
    await noStatusText(s, "nothing picked");
    const t = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot]'); const d = b.getAttribute('aria-describedby'); return { disabled: b.getAttribute('aria-disabled'), reason: d ? document.getElementById(d)?.textContent : null }; })())`)) as string) as { disabled: string; reason: string };
    assert.equal(t.disabled, "true");
    assert.equal(t.reason, "0 paths: no colour picked.");
    assert.ok((await hiddenStatus(s)).includes("0 paths: no colour picked."));
  }, RECORD);
});

// ------------------------------------------------------------------ 27.2: Isolate is gone

test("0089 §27.2 tester: no word Isolate anywhere; Ctrl+hover over a swatch changes nothing on the canvas and in the tooltip", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "iso.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    await settled(s);
    await sleep(1500);
    assert.equal(await s.evaluate(`/isolat/i.test(document.body.innerHTML.replace(/<script[\\s\\S]*?<\\/script>/g, ''))`), false, "no Isolate in the DOM");
    const [cx, , cw, ch] = await canvasRect(s);
    const region = [cx, (await barBottom(s)) + 90, cw, ch - 100];
    const hex = (await hexesOf(s))[2];
    await away(s);
    const before = await shot(s);
    await moveTo(s, swatch(hex), 2);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 2 });
    await sleep(1500);
    const tip = (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.innerText).join('|')`)) as string;
    assert.ok(!/isolat/i.test(tip), tip);
    const during = await shot(s);
    let diff = 0;
    for (let y = Math.ceil(region[1]); y < region[1] + region[3]; y += 2) {
      for (let x = Math.ceil(region[0]); x < region[0] + region[2]; x += 2) {
        const a = px(before, x, y);
        const b = px(during, x, y);
        if (Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2]) > 6) diff++;
      }
    }
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 0 });
    assert.ok(diff < 20, `no dimming of other colours under Ctrl+hover: ${diff} changed samples`);
    await away(s);
  }, RECORD);
});

// ------------------------------------------------------------------ 27.5: the palette pipette with Remove background

test("0089 §27.5 tester: Remove background keeps the palette pipette; it and the background pipette arm one at a time; the palette one stays armed after a pick; leaving Multicolour disarms it", { skip: !BROWSER, timeout: 500_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO);
    await pickMethod(s, "Remove background");
    await multicolour(s);
    await waitFor("swatches", async () => ((await swatchNames(s)).length >= 2 ? true : null), 40_000);
    assert.equal(await pressed(s, PAL_PIPETTE), "false", "palette pipette present, not armed");
    assert.equal(await pressed(s, BG_PIPETTE), "false", "background pipette present, not armed");
    const pal = (await rects(s, PAL_PIPETTE))[0];
    assert.equal(Math.round(pal[2]), 28, "28 px");
    await s.clickSel(PAL_PIPETTE);
    assert.deepEqual([await pressed(s, PAL_PIPETTE), await pressed(s, BG_PIPETTE)], ["true", "false"]);
    await s.clickSel(BG_PIPETTE);
    assert.deepEqual([await pressed(s, PAL_PIPETTE), await pressed(s, BG_PIPETTE)], ["false", "true"], "pressing one disarms the other");
    await s.clickSel(PAL_PIPETTE);
    assert.deepEqual([await pressed(s, PAL_PIPETTE), await pressed(s, BG_PIPETTE)], ["true", "false"]);
    // A pick of the blue block with the palette pipette: stays armed; the background is not touched.
    const bgBefore = (JSON.parse((await settled(s)).settings) as { background?: unknown }).background;
    await pickAt(s, isBlue);
    await sleep(1500);
    assert.equal(await pressed(s, PAL_PIPETTE), "true", "stays armed after a pick");
    assert.equal(await pressed(s, BG_PIPETTE), "false");
    assert.equal((JSON.parse((await settled(s)).settings) as { background?: unknown }).background, bgBefore, "the background is unchanged");
    // Leaving Multicolour disarms it (with Remove background).
    await s.evaluate(`[...document.querySelectorAll('${OUTPUT} [role=radio]')].find((r) => r.textContent === 'Single colour').click()`);
    await sleep(1000);
    assert.equal(await s.evaluate(`document.querySelector('${PAL_PIPETTE}')?.getAttribute('aria-pressed') ?? 'gone'`) !== "true", true, "disarmed after leaving Multicolour");
    await multicolour(s);
    assert.notEqual(await pressed(s, PAL_PIPETTE), "true", "still disarmed when coming back");
  }, RECORD);
});

// ------------------------------------------------------------------ 27.4: down to no colour

test("0089 §27.4 tester: delete by keyboard down to no colour: focus lands on the canvas, nothing is traced, Enter and Ctrl+Enter do nothing, the Colours field keeps its value, the pipette adds a colour traced alone; Colours 1 is one colour", { skip: !BROWSER, timeout: 700_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO);
    await pickMethod(s, "Remove background");
    await multicolour(s);
    await waitFor("swatches", async () => ((await swatchNames(s)).length >= 2 ? true : null), 40_000);
    const colours = (await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="Colours"]').getAttribute('aria-valuenow') ?? document.querySelector('${BAR} [role=spinbutton][aria-label="Colours"]').textContent`)) as string;
    let left = (await swatchNames(s)).length;
    while (left > 0) {
      await s.evaluate(`document.querySelector('${PALETTE} [data-trace-palette-swatch][tabindex="0"]').focus()`);
      await press(s, "Delete", 46);
      left -= 1;
      await waitFor(`${left} swatches`, async () => ((await swatchNames(s)).length === left ? true : null), 40_000);
      await sleep(1200);
    }
    assert.equal(await s.evaluate(`document.querySelectorAll('${PALETTE} [data-trace-palette-swatch]').length`), 0);
    assert.equal(await s.evaluate(`document.querySelectorAll('${PALETTE} [tabindex="0"]').length`), 0, "no Tab stop");
    assert.equal(await s.evaluate(`!!document.activeElement?.closest?.('${PALETTE}')`), false, "focus is not in the empty row");
    console.log("active element after the last delete:", await s.evaluate(`document.activeElement?.tagName + ' ' + (document.activeElement?.getAttribute('aria-label') ?? '') + ' ' + (document.activeElement?.className?.toString().slice(0, 60) ?? '')`));
    await sleep(2500);
    const n = (await lastRequest(s))!.n;
    await press(s, "Enter", 13);
    await press(s, "Enter", 13, 2);
    await s.evaluate(`document.querySelector('${BAR} [data-trace-slot]').focus()`);
    await press(s, "Enter", 13);
    await sleep(2000);
    assert.equal((await lastRequest(s))!.n, n, "Enter / Ctrl+Enter trace nothing");
    const t = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot]'); const d = b.getAttribute('aria-describedby'); return { disabled: b.getAttribute('aria-disabled'), reason: d ? document.getElementById(d)?.textContent : null }; })())`)) as string) as { disabled: string; reason: string };
    assert.deepEqual(t, { disabled: "true", reason: "0 paths: no colour in the palette." });
    await noStatusText(s, "no colour left");
    assert.equal((await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="Colours"]').getAttribute('aria-valuenow') ?? document.querySelector('${BAR} [role=spinbutton][aria-label="Colours"]').textContent`)) as string, colours, "the Colours field keeps its value");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('canvas')].length > 0`), true, "no crash: the canvas is there");
    // The pipette adds the first colour; it is traced alone.
    await s.clickSel(PAL_PIPETTE);
    await pickAt(s, isRed);
    await waitFor("one swatch", async () => ((await swatchNames(s)).length === 1 ? true : null), 40_000);
    await sleep(2500);
    const one = JSON.parse((await settled(s)).settings) as { fixed: string[]; colours: number };
    assert.equal(one.fixed.length, 1);
    assert.equal(one.colours, 1);
    assert.equal((await lastRequest(s))!.n > n, true, "traced again");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [data-trace-slot]').getAttribute('aria-disabled')`), null, "Trace is usable again");
    // A new Colours value starts from the found palette again; Colours 1 is one colour.
    await typeInto(s, "Colours", "1");
    await waitFor("one swatch from a fresh find", async () => ((await swatchNames(s)).length === 1 ? true : null), 40_000);
    await sleep(2500);
    const w = await wires(s, (await settled(s)).settings);
    assert.deepEqual(w.app, w.mine, "wire of Colours 1 reproducible");
    assert.equal((await swatchNames(s)).length, 1, "Colours 1 stays one colour");
  }, RECORD);
});

// ------------------------------------------------------------------ 800 x 600

test("0089 §27 tester: at 800 x 600 the bar does not wrap beyond the rows of §24.2 (Brightness, Advanced closed: 6 rows panel open, §25.1) and never overflows sideways; no row for a notice", { skip: !BROWSER, timeout: 500_000 }, async () => {
  await withApp(
    async (s) => {
      await openOn(s, "narrow.png", PICTURE);
      await pickMethod(s, "Brightness");
      await sleep(3000);
      await setAdv(s, false);
      const rowsOf = async () =>
        JSON.parse((await s.evaluate(`JSON.stringify((() => { const bar = document.querySelector('${BAR}'); const tops = new Set(); for (const e of bar.querySelectorAll('button, [role=spinbutton]')) { if (e.closest('.sr-only, [hidden]')) continue; const r = e.getBoundingClientRect(); if (r.width > 0) tops.add(Math.round(r.top / 36)); } const b = bar.getBoundingClientRect(); return { rows: tops.size, height: Math.round(b.height), overflow: bar.scrollWidth > bar.clientWidth + 1 || document.scrollingElement.scrollWidth > innerWidth + 1 || b.right > innerWidth + 0.5 }; })())`)) as string) as { rows: number; height: number; overflow: boolean };
      const open = await rowsOf();
      console.log("panel open:", JSON.stringify(open));
      assert.equal(open.overflow, false);
      assert.equal(open.rows, 6, `panel open: 6 rows, 216 px (§25.1, still true under §27): ${JSON.stringify(open)}`);
      await noStatusText(s, "800x600 Brightness");
      const hide = await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`);
      if (hide) {
        await s.clickSel('[aria-label="Hide panel"]');
        await sleep(800);
        const closed = await rowsOf();
        console.log("panel closed:", JSON.stringify(closed));
        assert.equal(closed.overflow, false);
        assert.equal(closed.rows, 3, `panel closed: 3 rows (§24.2, §25.1): ${JSON.stringify(closed)}`);
        await s.clickSel('[aria-label="Show panel"]');
        await sleep(600);
      }
      // Remove background + Multicolour with swatches: the row stays inside the bar.
      await pickMethod(s, "Remove background");
      await coloursN(s, 5);
      const r = await rowsOf();
      console.log("Remove background, Multicolour, 5 colours:", JSON.stringify(r));
      assert.equal(r.overflow, false);
      const cells = await rects(s, `${PALETTE} [data-trace-palette-swatch]`);
      assert.equal(new Set(cells.map((c) => Math.round(c[1]))).size, 1, "the swatches stay on one line");
      await noStatusText(s, "800x600 Remove background Multicolour");
    },
    "",
    800,
    600,
  );
});

// ------------------------------------------------------------------ 27.3: pointer

test("0089 §27.3 tester: a plain click on the swatch body does nothing, Ctrl+click excludes and again brings it back, the x deletes; the Trace button is never asked", { skip: !BROWSER, timeout: 500_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "ptr.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    await settled(s);
    const hexes = await hexesOf(s);
    const n0 = (await lastRequest(s))!.n;
    // Plain click in a corner of the cell (outside the x): nothing.
    await clickOn(s, swatch(hexes[0]), 0, -9, -11);
    await sleep(1500);
    assert.equal((await swatchNames(s)).length, 6);
    assert.ok(!(await swatchNames(s))[0].endsWith(", excluded"));
    assert.equal((await lastRequest(s))!.n, n0, "nothing traced");
    // Ctrl+click anywhere on the cell, also on the x spot: Exclude, not delete.
    await clickOn(s, swatch(hexes[0]), 2);
    await waitFor("excluded", async () => ((await swatchNames(s))[0].endsWith(", excluded") ? true : null), 20_000);
    assert.equal((await swatchNames(s)).length, 6, "a Ctrl+click on the x spot excludes, it does not delete");
    await clickOn(s, swatch(hexes[0]), 2, -9, -11);
    await waitFor("back", async () => (!(await swatchNames(s))[0].endsWith(", excluded") ? true : null), 20_000);
    // The x: delete, the others stay in order; the keyboard goes back to the canvas.
    await clickOn(s, swatch(hexes[3]));
    await waitFor("five", async () => ((await swatchNames(s)).length === 5 ? true : null), 40_000);
    assert.deepEqual([...(await hexesOf(s))].sort(), hexes.filter((h) => h !== hexes[3]).sort());
    assert.equal(await s.evaluate(`!!document.activeElement?.closest?.('${PALETTE}')`), false, "after a pointer delete the focus is not on a swatch");
    assert.equal((await s.layerRows()).length > 0, true);
  }, RECORD);
});

// Tester cases for `specs/0089-trace-quality-and-colour` slice E in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written
// from the specification (criteria 21, 22, 22a, 23, 13 with Frage 85 B, 24) before the code was read,
// and kept apart from `acceptance_0089_e_browser.test.ts`: this file compares the Mask view pixel by
// pixel with an independently computed mask, measures overlap and clipping of the bar for every
// method, viewport and panel state with screenshots under `target/e2e-0089-sliceE-tester/`, delays
// worker answers out of order, seeds old and damaged stored settings, and compares the About Trace
// text with the user page. Harness as in `acceptance_0089_sliceC_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5241;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-sliceE-tester");
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

/** Trace answers (and mask answers) arrive `window.__delays[n]` ms late, in the order the jobs were
 * posted: the page-side stand-in for a slow worker whose answers come out of order. Progress
 * messages are not delayed. `window.__log` records each job and when its answer was delivered. */
const WORKER_DELAY = `(() => {
  const Real = window.Worker;
  window.__delays = [];
  window.__log = [];
  window.__sent = new Map();
  const set = Object.getOwnPropertyDescriptor(Real.prototype, 'onmessage').set;
  window.Worker = class extends Real {
    postMessage(message, transfer) {
      if (message && (message.type === 'trace' || message.type === 'mask')) {
        const delay = window.__delays.length > 0 ? window.__delays.shift() : 0;
        window.__sent.set(message.type + message.id, delay);
        window.__log.push({ type: message.type, id: message.id, delay, sent: Math.round(performance.now()) });
      }
      return super.postMessage(message, transfer);
    }
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
        const d = event.data;
        const delay = d && (d.kind === 'trace' || d.kind === 'mask') ? window.__sent.get(d.kind + d.id) ?? 0 : 0;
        if (delay > 0) setTimeout(() => { window.__log.push({ type: d.kind, id: d.id, delivered: Math.round(performance.now()) }); fn(event); }, delay);
        else fn(event);
      });
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
const canvasRect = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as [number, number, number, number];
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const counts = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-counts]')?.textContent ?? ''`)) as string;
const countsOf = (text: string) => text.match(/^(\d+) paths?/)?.[1] ?? "?";
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const fieldValue = async (s: S, name: string) =>
  (await s.evaluate(`(document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]')?.getAttribute('aria-valuetext') ?? '').split(' ')[0]`)) as string;
const viewChecked = async (s: S) =>
  (await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [aria-checked=true]')?.getAttribute('aria-label') ?? ''`)) as string;
const clickView = async (s: S, name: string) => {
  await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').click()`);
  await sleep(1000);
};
const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
const savePng = async (s: S, name: string) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  writeFileSync(path.join(SCRATCH, name), Buffer.from(out.result?.data ?? "", "base64"));
};
type Shot = ReturnType<typeof decodePng>;
const at = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};
const boxOf = (img: Shot, rect: number[], test: (rgb: number[]) => boolean) => {
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (let y = Math.ceil(rect[1]); y < rect[1] + rect[3]; y++) {
    for (let x = Math.ceil(rect[0]); x < rect[0] + rect[2]; x++) {
      if (test(at(img, x, y))) {
        x0 = Math.min(x0, x);
        y0 = Math.min(y0, y);
        x1 = Math.max(x1, x);
        y1 = Math.max(y1, y);
      }
    }
  }
  return { x0, y0, x1, y1, cx: (x0 + x1) / 2, cy: (y0 + y1) / 2 };
};
const barBottom = async (s: S) => (await s.evaluate(`document.querySelector('${BAR}').getBoundingClientRect().bottom`)) as number;
const isBlack = ([r, g, b]: number[]) => r < 40 && g < 40 && b < 40;

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
const MASK_SRC = (() => {
  const black = new Set<string>();
  const fill = (x0: number, y0: number, x1: number, y1: number) => {
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) black.add(`${x},${y}`);
  };
  fill(0, 24, 5, 40); // touches the left border
  fill(14, 8, 40, 30); // big square
  fill(46, 14, 47, 15); // 2 x 2 speck
  fill(20, 40, 55, 55); // wide shape
  for (let y = 45; y <= 50; y++) for (let x = 28; x <= 37; x++) black.delete(`${x},${y}`); // its hole
  return black;
})();
const MASK_PNG = rgbPng(64, 64, (x, y) => (MASK_SRC.has(`${x},${y}`) ? [0, 0, 0] : [255, 255, 255]));
/** A stored choice whose upscale is Off, so the traced raster is the source raster. */
const NO_UPSCALE = `if (localStorage.getItem('curvyo.trace-values') === null) localStorage.setItem('curvyo.trace-values', JSON.stringify({ preset: 'engraving', edited: false, values: { smooth: 80, tolerance: 0.35, specks: 0.1, corners: 70, upscale: 0, denoise: 0 } }));`;

/** Zooms with Ctrl+wheel around `where()` until `scale()` reaches `atLeast`. */
const zoomUntil = async (s: S, scale: () => Promise<number>, where: () => Promise<[number, number]>, atLeast: number) => {
  for (let i = 0; i < 12 && (await scale()) < atLeast; i++) {
    const [x, y] = await where();
    await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY: -240, modifiers: 2 });
    await sleep(500);
  }
};

test("0089E tester c21: the Mask view equals the mask computed from the picture, pixel by pixel, after Ignore below, Drop border shapes and Invert", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "mask64.png", MASK_PNG);
    // Where the 64 x 64 source sits on the screen: from the black extent in the Image view
    // (x 0..55, y 8..55 are the extreme black pixels).
    await clickView(s, "Image");
    const blackBox = async () => {
      const [cx, , cw, ch] = await canvasRect(s);
      const top = await barBottom(s);
      const bb = boxOf(await shot(s), [cx, top + 4, cw, ch - top], isBlack);
      return bb;
    };
    const scale = async () => ((await blackBox()).x1 - (await blackBox()).x0 + 1) / 56;
    const where = async (): Promise<[number, number]> => {
      const b = await blackBox();
      return [b.cx, b.cy];
    };
    await zoomUntil(s, scale, where, 4);
    // The bar has the 36 px analysis row of slice F and is taller than it was: pan until the whole
    // 64 x 64 source (its top row is 8 source rows above the black box) lies below the bar and
    // inside the canvas, so every one of the 4096 pixels is still sampled.
    for (let i = 0; i < 12; i++) {
      const bb = await blackBox();
      const sy0 = (bb.y1 - bb.y0 + 1) / 48;
      const [, cy, , ch] = await canvasRect(s);
      const top = bb.y0 - 8 * sy0;
      const bottom = bb.y0 + 56 * sy0;
      if (top >= (await barBottom(s)) + 2 && bottom <= cy + ch - 2) break;
      const [x, y] = await where();
      await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY: top < (await barBottom(s)) + 2 ? -120 : 120 });
      await sleep(400);
    }
    await savePng(s, "c21-image-view.png");
    const b = await blackBox();
    {
      const sy1 = (b.y1 - b.y0 + 1) / 48;
      assert.ok(b.y0 - 8 * sy1 >= (await barBottom(s)), "the whole source is below the bar");
    }
    const sx = (b.x1 - b.x0 + 1) / 56;
    const sy = (b.y1 - b.y0 + 1) / 48;
    console.log(`source pixel on screen: ${sx.toFixed(2)} x ${sy.toFixed(2)}, black box ${JSON.stringify(b)}`);
    assert.ok(sx >= 3 && Math.abs(sx - sy) < 0.2, "the source is shown with square pixels, big enough to sample");
    const centre = (i: number, j: number): [number, number] => [b.x0 + (i + 0.5) * sx, b.y0 + (j - 8 + 0.5) * sy];
    const expected = (drop: boolean, invert: boolean, specks: boolean) => {
      // 4-connected components of the black pixels (the shapes are two or more pixels apart).
      const kept = new Set<string>();
      const seen = new Set<string>();
      for (const start of MASK_SRC) {
        if (seen.has(start)) continue;
        const comp: [number, number][] = [];
        const todo = [start];
        seen.add(start);
        while (todo.length > 0) {
          const [x, y] = todo.pop()!.split(",").map(Number);
          comp.push([x, y]);
          for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
            const k = `${x + dx},${y + dy}`;
            if (MASK_SRC.has(k) && !seen.has(k)) {
              seen.add(k);
              todo.push(k);
            }
          }
        }
        const touches = comp.some(([x, y]) => x === 0 || y === 0 || x === 63 || y === 63);
        const speck = comp.length < 8;
        if ((drop && touches) || (specks && speck)) continue;
        for (const [x, y] of comp) kept.add(`${x},${y}`);
      }
      return (x: number, y: number) => (invert ? !MASK_SRC.has(`${x},${y}`) : kept.has(`${x},${y}`));
    };
    const compare = async (label: string, want: (x: number, y: number) => boolean) => {
      let wrong: string[] = [];
      for (let attempt = 0; attempt < 30; attempt++) {
        const img = await shot(s);
        wrong = [];
        for (let y = 0; y < 64; y++) {
          for (let x = 0; x < 64; x++) {
            const [px, py] = centre(x, y);
            const got = at(img, px, py);
            const w = want(x, y) ? 0 : 255;
            if (got[0] !== w || got[1] !== w || got[2] !== w) wrong.push(`${x},${y}`);
          }
        }
        if (wrong.length === 0) break;
        await sleep(500);
      }
      await savePng(s, `c21-${label.replace(/\W+/g, "-")}.png`);
      assert.deepEqual(wrong.slice(0, 12), [], `${label}: ${wrong.length} of 4096 pixels differ from the expected mask`);
    };

    await clickView(s, "Mask");
    assert.equal(await viewChecked(s), "Mask");
    // Ignore below 0.1 mm (the stored choice): nothing is a speck, nothing dropped.
    await compare("plain, Ignore below 0.1 mm", expected(false, false, false));
    // Ignore below 1 mm drops the 2 x 2 speck (0.5 mm), keeps the 27 x 23 square and the hole.
    await typeInto(s, "Ignore below", "1");
    await compare("Ignore below 1 mm", expected(false, false, true));
    await s.evaluate(`document.querySelector('${BAR} button[aria-label="Drop border shapes"]').click()`);
    await sleep(800);
    await compare("Ignore below 1 mm and Drop border shapes", expected(true, false, true));
    await s.evaluate(`document.querySelector('${BAR} button[aria-label="Drop border shapes"]').click()`);
    await typeInto(s, "Ignore below", "0");
    await s.evaluate(`document.querySelector('${BAR} button[aria-label=Invert]').click()`);
    await sleep(800);
    await compare("Invert alone", expected(false, true, false));

    // Upscale Auto traces at twice the resolution: the mask is the traced raster, so its content must
    // still lie where the source's black lies, within one traced pixel (half a source pixel).
    await s.evaluate(`document.querySelector('${BAR} button[aria-label=Invert]').click()`);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Upscale] [role=radio]')].find((r) => r.textContent.includes('Auto')).click()`);
    await sleep(1500);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [role=radiogroup][aria-label=Upscale] [aria-checked=true]')?.textContent.trim()`), "Auto", "Upscale is Auto");
    const up = await blackBox();
    await savePng(s, "c21-upscale-auto.png");
    for (const [k, got, want] of [["x0", up.x0, b.x0], ["y0", up.y0, b.y0], ["x1", up.x1, b.x1], ["y1", up.y1, b.y1]] as const) {
      assert.ok(Math.abs(got - want) <= sx / 2 + 1.5, `Upscale Auto: the mask's content ${k} ${got} is where the source's is (${want}), within a traced pixel`);
    }
  }, NO_UPSCALE);
});

// ------------------------------------------------------------------ criteria 22, 22a: bar order, overlap, clipping

/** 300 x 120: three 60 x 60 blocks of grey 30, 110 and 190 on white, 50 px apart. Threshold 25 %
 * takes the first, 60 % the first two, 85 % all three. */
const GREYS = [30, 110, 190];
const BLOCKS3 = rgbPng(300, 120, (x, y) => {
  const i = [20, 110, 200].findIndex((x0) => x >= x0 && x < x0 + 60);
  return i >= 0 && y >= 30 && y < 90 ? [GREYS[i], GREYS[i], GREYS[i]] : [255, 255, 255];
});

const HINT = /\b(click|press|drag|tap|hold|try|use|tip|hint|you can|choose|select a|to (pick|add|remove|create))\b/i;

type Box = { x: number; y: number; w: number; h: number };
const ORDER: [string, string][] = [
  ["i", `${BAR} button[aria-label="About Trace"]`],
  ["Preset", `${BAR} button[aria-label=Preset]`],
  ["View", `${BAR} [role=radiogroup][aria-label=View]`],
  ["Method", `${BAR} button[aria-label=Method]`],
  ["method field", `${BAR} [role=spinbutton][aria-label=Threshold], ${BAR} [role=spinbutton][aria-label="Colour tolerance"]`],
  ["Smooth", `${BAR} [role=spinbutton][aria-label=Smooth]`],
  ["Tolerance", `${BAR} [role=spinbutton][aria-label=Tolerance]`],
  ["Ignore below", `${BAR} [role=spinbutton][aria-label="Ignore below"]`],
  ["expander", `${BAR} button[aria-expanded][aria-controls]`],
  ["Trace button", `${BAR} [data-trace-slot]`],
];

const orderBoxes = async (s: S, extra: [string, string][] = []) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify(${JSON.stringify([...ORDER.slice(0, 5), ...extra, ...ORDER.slice(5)])}.map(([name, sel]) => { const e = document.querySelector(sel); const b = e ? e.getBoundingClientRect() : null;
      return [name, b ? { x: Math.round(b.x), y: Math.round(b.y), w: Math.round(b.width), h: Math.round(b.height) } : null]; }))`)) as string,
  ) as [string, Box | null][];

const barSurvey = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => {
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
      const outside = items.filter((e) => { const r = e.getBoundingClientRect(); return r.left < box.left - 0.5 || r.right > box.right + 0.5 || r.top < box.top - 0.5 || r.bottom > box.bottom + 0.5 || r.left < 0 || r.right > innerWidth + 0.5 || r.top < 0 || r.bottom > innerHeight + 0.5; }).map(name);
      const clipped = [...bar.querySelectorAll('button, [role=spinbutton], span, label')].filter(vis).filter((e) => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow !== 'visible' && e.clientWidth > 0).map(name);
      const leaves = [...bar.querySelectorAll('*')].filter((e) => e.children.length === 0 && vis(e));
      const texts = leaves.map((e) => e.textContent.trim()).filter(Boolean);
      const labels = [...bar.querySelectorAll('[aria-label]')].filter(vis).map((e) => e.getAttribute('aria-label'));
      const titles = [...bar.querySelectorAll('[title]')].map((e) => e.getAttribute('title'));
      const traceLabels = leaves.filter((e) => e.textContent.trim() === 'Trace' && !e.closest('[data-trace-slot]')).length;
      const slot = bar.querySelector('[data-trace-slot]');
      return { overlaps, outside, clipped, texts, labels, titles, traceLabels, height: Math.round(box.height), top: Math.round(box.top), bottom: Math.round(box.bottom), vh: innerHeight,
        slot: slot ? slot.getBoundingClientRect().toJSON() : null, bar: box.toJSON(), slotText: slot ? slot.textContent.trim() : '' };
    })())`)) as string,
  ) as {
    overlaps: string[]; outside: string[]; clipped: string[]; texts: string[]; labels: string[]; titles: string[]; traceLabels: number; height: number; top: number; bottom: number; vh: number;
    slot: { x: number; y: number; right: number; bottom: number; width: number; height: number } | null; bar: { right: number; left: number }; slotText: string;
  };

const afterInReading = (prev: [string, Box | null], next: [string, Box | null], where: string) => {
  const [pn, p] = prev;
  const [n, b] = next;
  assert.ok(p && b, `${where}: ${pn} and ${n} are drawn`);
  const sameLine = Math.abs(p.y + p.h / 2 - (b.y + b.h / 2)) < 14;
  assert.ok(sameLine ? b.x >= p.x + p.w - 1 : b.y + b.h / 2 > p.y + p.h / 2, `${where}: ${n} comes after ${pn}: ${JSON.stringify([p, b])}`);
};

test("0089E tester c22, c22a: order, no overlap, no clipping, no label before the i, the Trace button last at the far right: every method x 1280x800 and 800x600 x panel open and closed, Advanced closed and open", { skip: !BROWSER, timeout: 900_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "blocks3.png", BLOCKS3);
    const hints = new Set<string>();
    const report: string[] = [];
    for (const [width, height] of [[1280, 800], [800, 600]]) {
      await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
      await sleep(600);
      for (const open of [true, false]) {
        const panelOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
        if (panelOpen !== open) {
          await s.clickSel(`[aria-label="${panelOpen ? "Hide panel" : "Show panel"}"]`);
          await sleep(700);
        }
        let first: [string, Box | null][] | null = null;
        for (const method of ["Brightness", "Remove background", "Pick colours"]) {
          await pickMethod(s, method);
          if (method === "Pick colours") {
            await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
            await press(s, "Escape", 27);
          }
          for (const advanced of [false, true]) {
            const isOpen = (await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Fewer trace options"]')`)) as boolean;
            if (isOpen !== advanced) {
              await s.evaluate(`document.querySelector('${BAR} [aria-label="${isOpen ? "Fewer" : "More"} trace options"]').click()`);
              await sleep(500);
            }
            const where = `${width}x${height}, panel ${open ? "open" : "closed"}, ${method}, Advanced ${advanced ? "open" : "closed"}`;
            const m = await barSurvey(s);
            assert.deepEqual(m.overlaps, [], `${where}: overlapping controls`);
            assert.deepEqual(m.outside, [], `${where}: controls outside the bar or window`);
            assert.deepEqual(m.clipped, [], `${where}: clipped text`);
            assert.ok(m.top >= 0 && m.bottom <= m.vh, `${where}: the bar is inside the window (${m.top}..${m.bottom} of ${m.vh})`);
            assert.equal(m.traceLabels, 0, `${where}: no "Trace" state label outside the finish button`);
            assert.equal(m.slotText.replace(/\s+/g, ""), "Trace", `${where}: the finish button reads Trace at rest`);
            for (const t of [...m.texts, ...m.labels]) if (HINT.test(t)) hints.add(`${where}: "${t}"`);
            for (const t of m.titles) if (HINT.test(t)) hints.add(`(tooltip) "${t}"`);
            await savePng(s, `c22-${width}x${height}-${open ? "panel-open" : "panel-closed"}-${method.replace(/ /g, "-")}-adv-${advanced ? "open" : "closed"}.png`);
            if (!advanced) {
              const extra: [string, string][] =
                method === "Brightness"
                  ? []
                  : [["colour group", `${BAR} [aria-label^="Pick background"], ${BAR} [aria-label^="Pick colour"], ${BAR} [data-trace-swatch], ${BAR} [aria-label^="Picked colours"]`]];
              const boxes = await orderBoxes(s, extra);
              for (let i = 1; i < boxes.length; i++) {
                if (boxes[i][0] === "colour group" && !boxes[i][1]) continue;
                const prev = boxes[i - 1][0] === "colour group" && !boxes[i - 1][1] ? boxes[i - 2] : boxes[i - 1];
                afterInReading(prev, boxes[i], where);
              }
              const slot = m.slot!;
              assert.ok(Math.abs(slot.right - (m.bar.right - 8)) < 2, `${where}: the Trace button is at the far right of the bar (gap ${m.bar.right - slot.right})`);
              assert.equal(Math.round(slot.width), 75, `${where}: 75 px wide`);
              const exp = boxes.find(([n]) => n === "expander")![1]!;
              assert.ok(slot.y + slot.height / 2 > exp.y + exp.h / 2 - 14 && (Math.abs(slot.y - exp.y) < 14 ? slot.x >= exp.x + exp.w : true), `${where}: the button is right of the expander (or on a later line)`);
              // Nothing before the method's own fields moves when the method changes.
              first ??= boxes;
              for (let i = 0; i <= 3; i++) assert.deepEqual(boxes[i], first[i], `${where}: ${boxes[i][0]} keeps its place`);
              report.push(`${where}: bar ${m.height} px`);
            }
          }
          // leave Advanced closed for the next method's order read, as it was
          const isOpen = (await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Fewer trace options"]')`)) as boolean;
          if (isOpen) {
            await s.evaluate(`document.querySelector('${BAR} [aria-label="Fewer trace options"]').click()`);
            await sleep(400);
          }
        }
        await pickMethod(s, "Brightness");
      }
    }
    console.log(report.join("\n"));
    console.log("hint-like strings:", JSON.stringify([...hints]));
    assert.deepEqual([...hints].filter((h) => !h.startsWith("(tooltip)")), [], "no operating hint text in the bar");
  });
});

// ------------------------------------------------------------------ criterion 23: late answers

const pathsNow = async (s: S, n: number, ms = 15_000) =>
  waitFor(`${n} paths`, async () => ((await counts(s)).startsWith(`${n} path`) ? true : null), ms).catch(async (e) => {
    throw new Error(`${e.message}; counts "${await counts(s)}", status "${await s.statusText()}"`);
  });

/** Slice F: the analysis may already have chosen another method for a new image; these two tests need the
 * Threshold, so the precondition is set by hand (a manual change, which the analysis never overrides). */
const ensureBrightness = async (s: S) => {
  const now = (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]')?.textContent ?? ''`)) as string;
  if (!/Brightness/.test(now)) {
    await pickMethod(s, "Brightness");
  }
};

test("0089E tester c23: an answer for stale settings arrives late and is dropped; the preview equals what Enter creates; one history step", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "blocks3.png", BLOCKS3);
    await ensureBrightness(s);
    await typeInto(s, "Threshold", "60");
    await pathsNow(s, 2);
    // A (25 %: one path) is answered 3 s late; B (85 %: three paths) is typed 0.9 s after it and answered at once.
    await s.evaluate(`window.__delays = [3000]; window.__log.length = 0`);
    await typeInto(s, "Threshold", "25");
    await sleep(500);
    await typeInto(s, "Threshold", "85");
    const seen: string[] = [];
    const until = Date.now() + 8000;
    while (Date.now() < until) {
      seen.push(countsOf(await counts(s)));
      await sleep(100);
    }
    const log = (await s.evaluate(`JSON.stringify(window.__log)`)) as string;
    console.log("worker log:", log);
    console.log("path counts seen after B was typed:", [...new Set(seen)].join(" -> "));
    const jobs = JSON.parse(log) as { type: string; id: number; delay?: number; sent?: number; delivered?: number }[];
    const late = jobs.filter((j) => j.delay === 3000);
    console.log(late.length === 0 ? "note: no job carried the delay" : `the delayed job ${late[0].id} was ${jobs.some((j) => j.delivered !== undefined && j.id === late[0].id) ? "delivered late" : "never delivered (the page ended it)"}`);
    assert.ok(!seen.includes("1"), "the late answer of A (one path) was never shown");
    assert.equal(seen[seen.length - 1], "3", "B's answer is shown");

    // The preview equals the created result; one history step.
    const rowsBefore = (await s.layerRows()).length;
    await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await press(s, "Enter", 13);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 20000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
    await s.tab("Layers");
    const rowsAfter = (await s.layerRows()).length;
    assert.ok(rowsAfter > rowsBefore, "Enter created the paths");
    await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
    await sleep(800);
    assert.equal((await s.layerRows()).length, rowsBefore, "one undo removes the whole trace");
  }, WORKER_DELAY);
});

test("0089E tester c23: in the Mask view a stale mask arriving late is dropped", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "blocks3.png", BLOCKS3);
    await ensureBrightness(s);
    await typeInto(s, "Threshold", "60");
    await pathsNow(s, 2);
    await clickView(s, "Image");
    const rect = await canvasRect(s);
    const top = await barBottom(s);
    const region = [rect[0], top + 4, rect[2], rect[3] - top];
    const img = await shot(s);
    // The two dark blocks are found by their grey; the third (grey 190, like other chrome) lies as far
    // right of the second as the second of the first.
    const dark = GREYS.slice(0, 2).map((g) => boxOf(img, region, ([r, gg, b]) => Math.abs(r - g) < 3 && Math.abs(gg - g) < 3 && Math.abs(b - g) < 3));
    const centres = [...dark, { ...dark[1], cx: 2 * dark[1].cx - dark[0].cx }];
    assert.ok(centres.every((c) => Number.isFinite(c.cx)), `the three blocks are on screen ${JSON.stringify(centres)}`);
    assert.deepEqual(at(img, centres[2].cx, centres[2].cy), [190, 190, 190], "the third block is where it is expected");
    const state = (m: Shot) => centres.map((c) => { const v = at(m, c.cx, c.cy)[0]; return v === 0 ? "B" : v === 255 ? "W" : "g"; }).join("");
    await clickView(s, "Mask");
    await waitFor("the 60 % mask", async () => (state(await shot(s)) === "BBW" ? true : null), 15_000);
    await s.evaluate(`window.__delays = [3000]`);
    await typeInto(s, "Threshold", "25");
    await sleep(500);
    await typeInto(s, "Threshold", "85");
    const seen: string[] = [];
    const until = Date.now() + 8000;
    while (Date.now() < until) {
      seen.push(state(await shot(s)));
      await sleep(200);
    }
    console.log("mask states after B was typed:", [...new Set(seen)].join(" -> "), "worker log:", (await s.evaluate(`JSON.stringify(window.__log)`)) as string);
    assert.ok(!seen.includes("BWW"), "the late mask of A (only the first block) was never shown at full strength");
    assert.equal(seen[seen.length - 1], "BBB", "B's mask is shown");
    assert.equal(await viewChecked(s), "Mask");
  }, WORKER_DELAY);
});

// ------------------------------------------------------------------ criterion 13, Frage 85 B: stored settings

type Seed = { name: string; raw: string; colourTolerance?: string; preset?: RegExp; smooth?: string; tolerance?: string; specks?: string; corners?: string; method?: RegExp; ranges?: boolean };
const stored = (o: object) => JSON.stringify(o);
const DEFAULTS = { preset: /^Logo/, smooth: "80", tolerance: "0.5", specks: "0.5", corners: "70", method: /Brightness/ };
const SEEDS: Seed[] = [
  { name: "0025 Silhouette, unedited", raw: stored({ preset: "silhouette", edited: false, values: { smooth: 100, specks: 2, corners: 70, upscale: 1, denoise: 1 } }), preset: /^Silhouette/, smooth: "100", tolerance: "1.5", specks: "2", corners: "70", method: /Brightness/ },
  { name: "0025 Engraving, unedited", raw: stored({ preset: "engraving", edited: false, values: { smooth: 80, specks: 0.1, corners: 70, upscale: 0, denoise: 0 } }), preset: /^Engraving/, smooth: "80", tolerance: "0.35", specks: "0.1", corners: "70" },
  { name: "0025 Photo, unedited", raw: stored({ preset: "photo", edited: false, values: { smooth: 50, specks: 0.1, corners: 30, upscale: 1, denoise: 0 } }), preset: /^Photo/, smooth: "50", tolerance: "0.5", specks: "0.1", corners: "30" },
  { name: "0025 edited Logo with Node limit, Full-size preview and a Threshold in the old unit", raw: stored({ preset: "logo", edited: true, node_limit: 5000, full_preview: true, nodeLimit: 5000, fullPreview: true, threshold: 128, values: { smooth: 33, specks: 0.8, corners: 55, upscale: 0, denoise: 2, node_limit: 5000, full_preview: true, threshold: 128 } }), preset: /^Custom/, smooth: "33", tolerance: "0.5", specks: "0.8", corners: "55" },
  { name: "0025 slice 1: { smooth, specks }", raw: stored({ smooth: 40, specks: 1 }), preset: /^Custom/, smooth: "40", tolerance: "0.5", specks: "1", corners: "70" },
  { name: "corrupt: not JSON", raw: "not json{", ...DEFAULTS },
  { name: "corrupt: null", raw: "null", ...DEFAULTS },
  { name: "corrupt: array", raw: "[]", ...DEFAULTS },
  { name: "corrupt: string", raw: '"x"', ...DEFAULTS },
  { name: "corrupt: wrong container types", raw: stored({ preset: 42, values: "no" }), ...DEFAULTS },
  { name: "corrupt: wrong value types", raw: stored({ preset: "logo", edited: "yes", values: { smooth: "80", specks: null, corners: {}, tolerance: "x" }, method: "x", backgroundTolerance: "hi" }), preset: /^(Logo|Custom)/, smooth: "80", tolerance: "0.5", specks: "0.5", corners: "70", method: /Brightness/ },
  { name: "unknown preset key", raw: stored({ preset: "nope", edited: false, values: { smooth: 20 } }), smooth: "20", tolerance: "0.5" },
  { name: "out of range numbers", raw: stored({ preset: "logo", edited: true, values: { smooth: 1e9, specks: -5, corners: 1e12, tolerance: 99, upscale: 7, denoise: -1 } }), smooth: "100", tolerance: "3", specks: "0", corners: "100", ranges: true },
  { name: "unknown method index", raw: stored({ preset: "logo", edited: false, values: { smooth: 80, specks: 0.5, corners: 70, upscale: 1, denoise: 0 }, method: 9 }), method: /Brightness/ },
  { name: "Remove background with an out-of-range tolerance", raw: stored({ preset: "logo", edited: false, values: { smooth: 80, specks: 0.5, corners: 70, upscale: 1, denoise: 0 }, method: 1, backgroundTolerance: 250 }), method: /Remove background/, colourTolerance: "100" },
];

test("0089E tester c13 / Frage 85 B: stored settings of 0025 and damaged ones are read after a reload; Smooth keeps its number, Tolerance starts at the preset's", { skip: !BROWSER, timeout: 900_000 }, async () => {
  const reload = `;(() => { const add = EventTarget.prototype.addEventListener; EventTarget.prototype.addEventListener = function (t, ...r) { if (t === "beforeunload") return undefined; return add.call(this, t, ...r); }; window.onbeforeunload = null; })();`;
  await withApp(async (s) => {
    const failures: string[] = [];
    // Slice F (Frage 83 B, 86 A): a new image gets its Method and Colour tolerance from the analysis, so
    // a stored Method or Colour tolerance no longer shows. The analysis is deterministic (criterion 34):
    // blocks3.png with no stored settings gives the reference, and every seed must show exactly that.
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await s.evaluate(`localStorage.removeItem('curvyo.trace-values')`);
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1000);
    await openOn(s, "blocks3.png", BLOCKS3);
    const analysed = {
      method: (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]')?.textContent ?? ''`)) as string,
      colourTolerance: await fieldValue(s, "Colour tolerance"),
    };
    console.log("analysed reference:", JSON.stringify(analysed));
    for (const seed of SEEDS) {
      await s.send("Page.navigate", { url: `${ORIGIN}/` });
      await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
      await s.evaluate(`localStorage.setItem('curvyo.trace-values', ${JSON.stringify(seed.raw)})`);
      await s.send("Page.navigate", { url: `${ORIGIN}/` });
      await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
      await sleep(1000);
      await openOn(s, "blocks3.png", BLOCKS3);
      const read = {
        preset: (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Preset]')?.textContent ?? ''`)) as string,
        smooth: await fieldValue(s, "Smooth"),
        tolerance: await fieldValue(s, "Tolerance"),
        specks: await fieldValue(s, "Ignore below"),
        corners: await fieldValue(s, "Corners"),
        method: (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]')?.textContent ?? ''`)) as string,
        counts: await counts(s),
        colourTolerance: await fieldValue(s, "Colour tolerance"),
        legacy: (await s.evaluate(`/node limit|full-size/i.test(document.querySelector('${BAR}').innerText + [...document.querySelectorAll('${BAR} [aria-label]')].map((e) => e.getAttribute('aria-label')).join(' '))`)) as boolean,
        groups: (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=radiogroup]')].filter((g) => g.getAttribute('aria-label') !== 'View').map((g) => g.querySelectorAll('[aria-checked=true]').length))`)) as string,
      };
      const bad: string[] = [];
      if (seed.preset && !seed.preset.test(read.preset)) bad.push(`preset "${read.preset}"`);
      for (const k of ["smooth", "tolerance", "specks", "corners"] as const) if (seed[k] !== undefined && Number(read[k]) !== Number(seed[k])) bad.push(`${k} ${read[k]} (want ${seed[k]})`);
      if (read.method !== analysed.method) bad.push(`method "${read.method}" (the analysis chose "${analysed.method}")`);
      if (read.colourTolerance !== analysed.colourTolerance) bad.push(`Colour tolerance ${read.colourTolerance} (the analysis chose ${analysed.colourTolerance})`);
      if (read.legacy) bad.push("a Node limit or Full-size preview control is shown");
      if (seed.ranges) {
        const g = JSON.parse(read.groups) as number[];
        if (g.some((n) => n !== 1)) bad.push(`a choice group without exactly one checked option: ${read.groups}`);
      }
      if (!/^\d+ paths?/.test(read.counts)) bad.push(`no trace result ("${read.counts}")`);
      console.log(`${seed.name}: ${JSON.stringify(read)}${bad.length ? "  <-- " + bad.join("; ") : ""}`);
      if (bad.length) failures.push(`${seed.name}: ${bad.join("; ")}`);
    }
    assert.deepEqual(failures, [], "stored settings migrate and damaged ones fall back");
  }, reload);
});

// ------------------------------------------------------------------ criterion 24: About Trace and the user page

const REPO = path.resolve(FRONTEND, "..");

test("0089E tester c24: About Trace names the methods, the Mask view and the new geometry, matches the user page, and no stale name is left", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const page = readFileSync(path.join(REPO, "docs", "tracing-images.md"), "utf8");
  const design = readFileSync(path.join(REPO, "docs", "design-system.md"), "utf8");
  await withApp(async (s) => {
    await openOn(s, "blocks3.png", BLOCKS3);
    // Every control the bar shows for any method has a name in the app.
    const labels = new Set<string>();
    for (const method of ["Brightness", "Remove background", "Pick colours"]) {
      await pickMethod(s, method);
      for (const l of JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=spinbutton], ${BAR} button[aria-label=Preset], ${BAR} button[aria-label=Method], ${BAR} [role=radiogroup]')].map((e) => e.getAttribute('aria-label')))`)) as string) as string[]) labels.add(l);
    }
    await s.evaluate(`document.querySelector('${BAR} button[aria-label="About Trace"]').click()`);
    await sleep(600);
    const entries = JSON.parse(
      (await s.evaluate(`JSON.stringify([...document.querySelectorAll('dt')].map((dt) => [dt.textContent.trim(), (dt.nextElementSibling?.textContent ?? '').trim()]))`)) as string,
    ) as [string, string][];
    console.log("About Trace terms:", entries.map(([t]) => t).join(" | "));
    assert.ok(entries.length > 10, "the About Trace tab lists the bar's values");
    const terms = entries.map(([t]) => t);
    for (const want of ["View", "Mask", "Method", "Brightness", "Remove background", "Pick colours", "Colour tolerance", "Threshold", "Smooth", "Tolerance", "Corners", "Invert", "Drop border shapes"]) {
      assert.ok(terms.includes(want), `About Trace has an entry "${want}"`);
    }
    for (const stale of ["Node limit", "Full-size preview", "Ignore background"]) {
      assert.ok(!terms.includes(stale), `About Trace has no entry "${stale}"`);
      assert.ok(!page.includes(`**${stale}**`), `the user page has no "${stale}" setting`);
    }
    for (const l of labels) if (l !== "View") assert.ok(terms.includes(l), `the bar control "${l}" has an About Trace entry`);
    const body = (await s.evaluate(`document.body.innerText`)) as string;
    assert.ok(!/Create paths/.test(body), "no 'Create paths' label is left in the app");
    // Every About Trace entry is the same string on the user page.
    const missing = entries.filter(([t, x]) => !page.includes(`- ${t}: ${x}`)).map(([t, x]) => `${t}: ${x}`);
    assert.deepEqual(missing, [], "each About Trace entry appears word for word in docs/tracing-images.md");
  });
  for (const word of ["Mask", "Method", "Brightness", "Remove background", "Pick colours", "Colour tolerance", "Tolerance", "Smooth", "Drop border shapes"]) {
    assert.ok(page.includes(word), `docs/tracing-images.md mentions ${word}`);
  }
  for (const word of ["Mask", "Method", "Colour tolerance", "Drop border shapes"]) {
    assert.ok(design.includes(word), `docs/design-system.md mentions ${word}`);
  }
  assert.ok(!/Node limit.*(is|are) (shown|in)/i.test(page), "the user page does not describe a Node limit control");
});

test("0089E tester c24: docs/design-system.md describes the new geometry (Smooth and the Tolerance field in px)", () => {
  const design = readFileSync(path.join(REPO, "docs", "design-system.md"), "utf8");
  assert.ok(/Tolerance[^|]{0,120}\bpx\b/.test(design), "a Trace row names the Tolerance field with its unit px");
  assert.ok(!/Row 1 opens with the state label "Trace"/.test(design.split("\n").filter((l) => l.startsWith("| Trace bar |")).join("")) || /2026-10-30[^|]*Tolerance/.test(design), "the Trace bar row is brought up to the new bar");
});

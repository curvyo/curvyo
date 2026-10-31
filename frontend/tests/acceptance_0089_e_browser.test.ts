// `specs/0089-trace-quality-and-colour` slice E in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the View pill with Mask
// (criterion 21: four segments, Shift+\\ through four, the Mask view black where the tracer reads
// content and white elsewhere, following the settings) and the bar order (criteria 22, 22a: Preset
// and View, Method and its fields, Smooth and Tolerance, Ignore below, the expander, the blue Trace
// button; nothing before the method's fields moves between methods), at 1280 x 800 and 800 x 600.
// Harness as in `acceptance_0089_c_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
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

const viewChecked = async (s: S) =>
  (await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [aria-checked=true]')?.getAttribute('aria-label') ?? ''`)) as string;

const clickView = async (s: S, name: string) => {
  await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').click()`);
  await sleep(1200);
};

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

const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};

type Shot = ReturnType<typeof decodePng>;
const at = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};

/** The screen box of the pixels of `img` within the canvas whose colour passes `test`. */
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

const canvasRect = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string,
  ) as number[];

/** The bar's items in the order criterion 22 names, with their screen boxes. */
const BAR_ITEMS: [string, string][] = [
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

test("0089 E in Chromium: the Mask view and Shift+\\ through four views", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "blocks.png");
  writeFileSync(file, BLOCKS);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, file, 1280, 800);

    // The pill: four segments, Mask second with the Binary glyph, its tooltip name "Mask".
    const segs = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].map((r) => [r.getAttribute('aria-label'), r.querySelector('svg')?.getAttribute('class') ?? '']))`)) as string) as [string, string][];
    assert.deepEqual(segs.map(([n]) => n), ["Overlay", "Mask", "Trace", "Image"]);
    assert.match(segs[1][1], /lucide-binary/, "Mask uses the Binary glyph");
    assert.equal(await viewChecked(s), "Overlay");

    // Shift+\ steps through the four views in pill order.
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    const seen: string[] = [];
    for (let i = 0; i < 4; i++) {
      await s.evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: '|', shiftKey: true })) && true`);
      await s.evaluate(`window.dispatchEvent(new KeyboardEvent('keyup', { key: '|', shiftKey: true })) && true`);
      await sleep(300);
      seen.push(await viewChecked(s));
    }
    assert.deepEqual(seen, ["Mask", "Trace", "Image", "Overlay"]);

    // Where the blocks are on screen: the Image view.
    const rect = await canvasRect(s);
    await clickView(s, "Image");
    const image = await shot(s);
    const red = boxOf(image, rect, ([r, g, b]) => r > 200 && g < 60 && b < 60);
    const black = boxOf(image, rect, ([r, g, b]) => r < 40 && g < 40 && b < 40);
    assert.ok(Number.isFinite(red.cx) && Number.isFinite(black.cx), "both blocks are on screen");
    const between = [(red.x1 + black.x0) / 2, red.cy];
    console.log("blocks on screen:", JSON.stringify({ red, black }));

    // Criterion 21: the Mask view draws what the tracer reads: both blocks black (the red one
    // too: Brightness takes it), the image's white background white, no red, no outline.
    await clickView(s, "Mask");
    assert.equal(await viewChecked(s), "Mask");
    const mask = await shot(s);
    assert.deepEqual(at(mask, red.cx, red.cy), [0, 0, 0], "the red block is content");
    assert.deepEqual(at(mask, black.cx, black.cy), [0, 0, 0], "the black block is content");
    assert.deepEqual(at(mask, between[0], between[1]), [255, 255, 255], "the background is white");
    const redLeft = boxOf(mask, rect, ([r, g, b]) => r > 200 && g < 60 && b < 60);
    assert.ok(!Number.isFinite(redLeft.cx), "no red in the Mask view");
    // Around the blocks, inside the image's frame (its selection box is drawn at the frame).
    const around = [red.x0 - 4, red.y0 - 4, black.x1 - red.x0 + 8, black.y1 - red.y0 + 8];
    const blue = boxOf(mask, around, ([r, g, b]) => b > 180 && r < 120 && g < 170);
    assert.ok(!Number.isFinite(blue.cx), "no blue outline in the Mask view");
    writeFileSync(path.join(SCRATCH, "e-mask-1280x800.png"), Buffer.from(((await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } }).result?.data ?? "", "base64"));

    // The mask follows the settings: Pick colours with the red picked leaves the black block out.
    await pickMethod(s, "Pick colours");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: red.cx, y: red.cy });
    await sleep(100);
    for (const type of ["mousePressed", "mouseReleased"]) {
      await s.send("Input.dispatchMouseEvent", { type, x: red.cx, y: red.cy, button: "left", clickCount: 1 });
    }
    await waitFor("the red swatch", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-swatch="#FF0000"]')`)) ? true : null), 10_000);
    // The pointer leaves the canvas so the pipette's colour chip covers nothing.
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 2 });
    await waitFor("the mask of the picked red", async () => {
      const m = await shot(s);
      const r = at(m, red.cx, red.cy).join();
      const k = at(m, black.cx, black.cy).join();
      return r === "0,0,0" && k === "255,255,255" ? true : null;
    }, 20_000);
    assert.equal(await viewChecked(s), "Mask", "the view stays Mask while the settings change");

    // Back to Brightness: both blocks again; the peek shows the image while held.
    await pickMethod(s, "Brightness");
    await waitFor("the Brightness mask", async () => (at(await shot(s), black.cx, black.cy).join() === "0,0,0" ? true : null), 20_000);
    await s.evaluate(`window.dispatchEvent(new KeyboardEvent('keydown', { key: '\\\\' })) && true`);
    await sleep(500);
    assert.equal(await viewChecked(s), "Image", "the peek works in the Mask view");
    const peek = await shot(s);
    assert.ok(at(peek, red.cx, red.cy)[0] > 200 && at(peek, red.cx, red.cy)[1] < 60, "the peek shows the red image");
    await s.evaluate(`window.dispatchEvent(new KeyboardEvent('keyup', { key: '\\\\' })) && true`);
    await sleep(500);
    assert.equal(await viewChecked(s), "Mask");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

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

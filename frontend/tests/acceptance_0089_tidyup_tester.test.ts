// Tester cases for the Trace bar tidy-up of 2026-10-31 (`specs/0089-trace-quality-and-colour/ux-notes.md` §22 and the
// amended criteria 17, 22, 29, 31, 33 of 0089 and 11, 16, 44, 46, 57, 71 of 0025) in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written from the amended
// text: the bar contents per method, swatches and counts shown once, the notice row, the hidden live region, the
// disabled Trace button, the Tab order and the bar heights at 800 x 600. Harness as in
// `acceptance_0089_sliceF_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5271;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-tidyup-tester");
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
const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
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

// ------------------------------------------------------------------ pictures

/** 300 x 300: a pale yellow disc and a light grey square on white (the analysis keeps both). */
const PALE = rgbPng(300, 300, (x, y) => {
  if (Math.hypot(x - 100, y - 100) < 60) return [0xff, 0xf2, 0xa8];
  if (x >= 170 && x < 270 && y >= 150 && y < 250) return [0xd9, 0xd9, 0xd9];
  return [255, 255, 255];
});
/** 420 x 300: thin black strokes on white, a line drawing. */
const LINES = rgbPng(420, 300, (x, y) => {
  for (let k = 1; k < 6; k++) {
    if (Math.abs(y - (45 * k + 14 * Math.sin(x / 40 + k))) < 1.2) return [10, 10, 10];
  }
  return Math.abs(x - y) < 1.4 ? [10, 10, 10] : [255, 255, 255];
});
/** 360 x 200: a red and a blue block on white. */
const TWO = rgbPng(360, 200, (x, y) => {
  if (y < 40 || y >= 160) return [255, 255, 255];
  if (x >= 30 && x < 160) return [200, 30, 30];
  if (x >= 200 && x < 330) return [30, 60, 200];
  return [255, 255, 255];
});
/** 200 x 200 of near-black: one region covering the whole image. */
const DARK = rgbPng(200, 200, () => [20, 20, 20]);
/** 600 x 600: black with 90,000 isolated one-pixel white holes (more than the 65,000 of criterion 16). */
const HOLES = rgbPng(600, 600, (x, y) => (x % 2 === 1 && y % 2 === 1 ? [255, 255, 255] : [0, 0, 0]));
/** 300 x 300: one black square and five isolated one-pixel specks on white. */
const SPECKS = rgbPng(300, 300, (x, y) => {
  if (x >= 100 && x < 200 && y >= 100 && y < 200) return [0, 0, 0];
  return [[20, 20], [250, 30], [30, 260], [270, 270], [60, 150]].some(([px, py]) => px === x && py === y) ? [0, 0, 0] : [255, 255, 255];
});

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

const noticeText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-state]')?.textContent ?? 'missing'`)) as string;
const noticeHeight = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-state]')?.getBoundingClientRect().height ?? -1`)) as number;
const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${ADV} [data-trace-counts]')?.textContent ?? ''`)) as string;
const hiddenStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? 'absent'`)) as string;
const pickedLabel = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-picked-count]')?.textContent ?? 'none'`)) as string;
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);
const paletteSwatches = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[data-trace-palette-swatch]')].map((e) => e.getAttribute('data-trace-palette-swatch')))`)) as string) as string[];
const settledCounts = (s: S) => waitFor("a trace result", async () => (/^(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);
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
const barHeight = async (s: S) => (await s.evaluate(`Math.round(document.querySelector('${BAR}').getBoundingClientRect().height)`)) as number;

/** Shows the image, finds the first block of `colour` on screen and presses the pipette on it. */
async function pickAt(s: S, colour: (rgb: number[]) => boolean, modifiers = 0) {
  await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=Image]').click()`);
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
}
const isRed = ([r, g, b]: number[]) => r > 180 && g < 50 && b < 50;
const isBlue = ([r, g, b]: number[]) => b > 180 && r < 50 && g < 90;

const ROW = `${BAR} [data-trace-analysis]`;
const lineText = async (s: S) => (await s.evaluate(`document.querySelector('${ROW} [role=status]')?.textContent ?? ''`)) as string;
const waitState = (s: S, want: string, ms = 30_000) =>
  waitFor(`the analysis row to be ${want}`, async () => (((await s.evaluate(`document.querySelector('${ROW}')?.getAttribute('data-trace-analysis') ?? 'none'`)) as string) === want ? true : null), ms);

const BRIGHTNESS_BAR = ["About Trace", "Preset", "View", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"];

// ------------------------------------------------------------------ 0089 22 / 0025 71: the bar contents per method

test("0089 tidy-up c22: the bar holds About Trace, Preset, View, Method, the one main field, Smooth, Tolerance, Ignore below, the expander and Trace for Brightness, in Single and Multicolour", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    const hints: string[] = [];
    for (const output of ["Single colour", "Multicolour"]) {
      await setOutput(s, output);
      await settledCounts(s);
      hints.push(...(await barStrings(s)).filter((t) => HINT.test(t)));
      await setAdvanced(s, false);
      assert.deepEqual(await barControls(s), BRIGHTNESS_BAR, `Brightness, ${output}: the bar`);
      assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-analysis], ${BAR} [data-trace-counts], ${BAR} [data-trace-palette-swatch], ${BAR} [data-trace-swatch], ${BAR} [data-trace-pipette]').length`), 0, `Brightness, ${output}: no analysis row, result line, swatch or pipette in the bar`);
      await setAdvanced(s, true);
    }
    assert.deepEqual([...new Set(hints)], [], "no operating hint text");
  });
});

test("0089 tidy-up c22, c17: Remove background keeps the chip, hex, Auto and pipette group in the bar; Pick colours has the pipette and a count label and no swatches or Remove", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO, true);
    const hints: string[] = [];
    await pickMethod(s, "Remove background");
    for (const output of ["Single colour", "Multicolour"]) {
      await setOutput(s, output);
      hints.push(...(await barStrings(s)).filter((t) => HINT.test(t)));
      await setAdvanced(s, false);
      const bar = await barControls(s);
      console.log(`Remove background, ${output}:`, JSON.stringify(bar));
      assert.deepEqual(bar.slice(0, 5), ["About Trace", "Preset", "View", "Method", "Colour tolerance"]);
      const group = bar.slice(5, bar.indexOf("Smooth"));
      assert.ok(group.includes("Auto") && group.some((g) => /^Pick background colour/.test(g)), `the group has Auto and the pipette: ${JSON.stringify(group)}`);
      assert.deepEqual(bar.slice(bar.indexOf("Smooth")), ["Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"]);
      await setAdvanced(s, true);
    }
    await pickMethod(s, "Pick colours");
    for (const output of ["Single colour", "Multicolour"]) {
      await setOutput(s, output);
      await setAdvanced(s, false);
      const bar = await barControls(s);
      assert.deepEqual(bar, ["About Trace", "Preset", "View", "Method", "Colour tolerance", "Pick colour from the image", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"], `Pick colours, ${output}`);
      assert.equal(await pickedLabel(s), "None picked");
      assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-palette-swatch], ${BAR} [data-trace-swatch]').length`), 0, "no swatches with Advanced closed");
      await setAdvanced(s, true);
      hints.push(...(await barStrings(s)).filter((t) => HINT.test(t)));
    }
    // A pick: the label counts, the bar still has no swatch and no Remove button.
    await setOutput(s, "Single colour");
    await pickAt(s, isRed);
    assert.equal(await pickedLabel(s), "1 picked");
    await setAdvanced(s, false);
    const withPick = await barControls(s);
    assert.ok(!withPick.some((n) => /Remove|swatch/i.test(n)), `no Remove or swatch control in the bar: ${JSON.stringify(withPick)}`);
    assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-picked-count]')[0].matches('button, [tabindex]')`), false, "the count label is not a Tab stop");
    assert.deepEqual([...new Set(hints)], [], "no operating hint text");
  });
});

// ------------------------------------------------------------------ 0089 17, 29: swatches once; 0025 11: counts once

test("0089 tidy-up c17, c29: a picked colour is one swatch, in the Advanced strip only; the bar says '2 picked'; with Advanced closed no swatch is drawn", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO, true);
    await pickMethod(s, "Pick colours");
    await pickAt(s, isRed);
    await pickAt(s, isBlue, 8);
    await settledCounts(s);
    assert.equal(await pickedLabel(s), "2 picked");
    const hexes = await paletteSwatches(s);
    assert.equal(hexes.length, 2, `two swatches, once each: ${JSON.stringify(hexes)}`);
    assert.equal(new Set(hexes).size, 2);
    assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-swatch]').length`), 0, "no second (bar) swatch row");
    assert.equal(await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-palette-swatch]').length`), 2, "both in the Advanced block");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} *')].filter((e) => e.children.length === 0 && /(^| )(\\d+ picked|None picked)$/.test(e.textContent.trim())).length`), 1, "the count is stated once");
    // Delete on the focused swatch removes it (criterion 17), in Advanced.
    await s.evaluate(`document.querySelector('${ADV} [data-trace-palette-swatch]').focus()`);
    await press(s, "Delete", 46);
    await waitFor("one swatch", async () => ((await paletteSwatches(s)).length === 1 ? true : null));
    assert.equal(await pickedLabel(s), "1 picked");
    await setAdvanced(s, false);
    assert.equal((await paletteSwatches(s)).length, 0, "Advanced closed: no swatch anywhere (criterion 29)");
    assert.equal(await pickedLabel(s), "1 picked");
  });
});

test("0089 tidy-up c29, c22: Multicolour swatches of Brightness appear once, in Advanced, never in the bar", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pale.png", PALE, true);
    await pickMethod(s, "Brightness");
    await setOutput(s, "Multicolour");
    await settledCounts(s);
    const hexes = await paletteSwatches(s);
    assert.ok(hexes.length >= 1, `the strip has swatches: ${JSON.stringify(hexes)}`);
    assert.equal(new Set(hexes).size, hexes.length, "each swatch once");
    assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-swatch]').length`), 0);
    assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-palette-swatch]').length - document.querySelectorAll('${ADV} [data-trace-palette-swatch]').length`), 0, "all swatches are inside Advanced");
    await setAdvanced(s, false);
    assert.equal((await paletteSwatches(s)).length, 0, "not visible with Advanced closed");
  });
});

const COUNTS_RE = "\\\\d+ paths?, \\\\d+ nodes?";
const countsMentions = (s: S) =>
  s.evaluate(`(() => { const re = new RegExp('${COUNTS_RE}');
    const visible = (e) => { for (let n = e; n && n !== document.body; n = n.parentElement) { if (n.className && String(n.className).includes('sr-only')) return false; } return true; };
    return JSON.stringify([...document.querySelectorAll('*')].filter((e) => e.children.length === 0 && re.test(e.textContent) && visible(e)).map((e) => e.closest('[data-trace-counts]') ? 'advanced-status' : e.closest('[data-trace-state]') ? 'notice-row' : 'other')); })()`)
    .then((v) => JSON.parse(v as string) as string[]);

test("0025 tidy-up c11, c71: the result counts are drawn once, in the Advanced status line, and nowhere with Advanced closed; the hidden region announces them", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    assert.deepEqual(await countsMentions(s), ["advanced-status"], "Advanced open: once, in the status line");
    assert.equal(await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-hidden-status]').length`), 0, "no second live region while Advanced is open");
    const text = await advancedStatus(s);
    await setAdvanced(s, false);
    assert.deepEqual(await countsMentions(s), [], "Advanced closed: not drawn anywhere");
    assert.ok((await hiddenStatus(s)).includes(text.match(/\d+ paths?, \d+ nodes?, [\d.]+ s/)![0]), `the hidden region announces the counts: ${await hiddenStatus(s)}`);
  });
});

// ------------------------------------------------------------------ 0025 11, 16, 46, 71 / ux-notes 22.2: the notice row

test("0089 tidy-up 22.2: a normal settled trace leaves the notice row empty and without height, Advanced open or closed", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    for (const open of [true, false]) {
      await setAdvanced(s, open);
      assert.equal(await noticeText(s), "", `Advanced ${open ? "open" : "closed"}: no notice text`);
      assert.ok((await noticeHeight(s)) < 1, `no height: ${await noticeHeight(s)}`);
    }
  });
});

test("0089 tidy-up 22.2: '0 paths at this Threshold: too low.' is in the notice row, Trace is disabled, and the zero count is not repeated in Advanced", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    await typeInto(s, "Threshold", "0");
    await waitFor("the notice", async () => (/0 paths/.test(await noticeText(s)) ? true : null));
    assert.equal(await noticeText(s), "0 paths at this Threshold: too low.");
    assert.equal(await advancedStatus(s), "", "no counts line for an empty result");
    const visible = (await s.evaluate(`document.querySelector('${BAR}').innerText`)) as string;
    assert.equal(visible.match(/0 paths/g)?.length, 1, `"0 paths" appears once: ${visible}`);
    const trace = await traceButton(s);
    assert.equal(trace?.disabled, "true", "ux-notes 22.2: Trace is disabled for an empty result");
  });
});

test("0089 tidy-up 22.2: '1 path covering the whole image.' is in the notice row", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "dark.png", DARK, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    await typeInto(s, "Threshold", "128");
    await sleep(500);
    await waitFor("the notice", async () => (/covering/.test(await noticeText(s)) ? true : null));
    assert.equal(await noticeText(s), "1 path covering the whole image.");
    console.log("Advanced status with '1 path covering the whole image':", JSON.stringify(await advancedStatus(s)), JSON.stringify(await traceButton(s)));
  });
});

test("0089 tidy-up 22.2, c16: 'Too much detail: raise Ignore below.' is in the notice row (and the criterion's own wording)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "holes.png", HOLES, true);
    await pickMethod(s, "Brightness");
    await typeInto(s, "Ignore below", "0");
    await waitFor("the notice", async () => (/detail/i.test(await noticeText(s)) ? true : null), 120_000);
    assert.equal(await noticeText(s), "Too much detail: raise Ignore below.");
    assert.equal(await advancedStatus(s), "", "no counts for an unusable result");
  });
});

test("0089 tidy-up 22.2: 'Same as #xxxxxx' (a palette pick matching a colour) is a notice-row message for about 3 s; specks ignored and the counts stay in Advanced", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO, true);
    await pickMethod(s, "Brightness");
    await setOutput(s, "Multicolour");
    await settledCounts(s);
    await s.clickSel(`${ADV} [data-trace-pipette=palette]`);
    // The automatic palette already holds the red, so the first pick selects it and says so.
    await pickAt(s, isRed);
        await waitFor("Same as", async () => (/^Same as #[0-9A-F]{6}$/i.test(await noticeText(s)) ? true : null), 2500);
    assert.ok((await noticeHeight(s)) >= 30, "the notice row has a row of its own");
    await waitFor("the notice gone", async () => ((await noticeText(s)) === "" ? true : null), 6000);
    assert.ok((await noticeHeight(s)) < 1);
  });
  await withApp(async (s) => {
    await openOn(s, "specks.png", SPECKS, true);
    await pickMethod(s, "Brightness");
    await waitFor("specks ignored", async () => (/specks? ignored/.test(await advancedStatus(s)) ? true : null), 60_000);
    assert.equal(await noticeText(s), "", "the muted speck line is not in the notice row");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-state] *')].filter((e) => /specks? ignored/.test(e.textContent)).length`), 0);
  });
});

// ------------------------------------------------------------------ 0089 17 / ux-notes 22.2: the disabled Trace button

test("0089 tidy-up c17: Pick colours with nothing picked: Trace is aria-disabled with the reason as tooltip and aria-describedby, the label says None picked, the line is in Advanced only", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO, true);
    await pickMethod(s, "Pick colours");
    await waitFor("the Advanced line", async () => ((await advancedStatus(s)) === "0 paths: no colour picked." ? true : null));
    assert.equal(await pickedLabel(s), "None picked");
    assert.equal(await noticeText(s), "", "not in the notice row");
    const trace = await traceButton(s);
    assert.equal(trace?.disabled, "true");
    assert.ok(trace?.describedby, "aria-describedby is set");
    assert.equal(trace?.reason, "0 paths: no colour picked.", "the reason is the status line text");
    // The tooltip carries the reason.
    const box = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot]').getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as number[];
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box[0] - 3, y: box[1] });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box[0], y: box[1] });
    const tip = await waitFor("the tooltip", async () => (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.textContent).find((t) => t.includes('no colour picked')) ?? null`)) as string | null, 5000);
    assert.ok(tip.includes("0 paths: no colour picked."), tip);
    // Pressing it creates nothing.
    const rows = (await s.layerRows()).length;
    await s.clickSel(`${BAR} [data-trace-slot]`);
    await sleep(800);
    assert.equal((await s.layerRows()).length, rows, "a disabled Trace creates nothing");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR}')`), true, "the tool stays");
    // A pick enables it.
    await pickAt(s, isRed);
    await settledCounts(s);
    const after = await traceButton(s);
    assert.equal(after?.disabled, null, "enabled once a colour is picked");
    assert.equal(after?.describedby, null);
  });
});

// ------------------------------------------------------------------ 0025 71 / ux-notes 22.2: the hidden live region

test("0089 tidy-up 22.2, c71: with Advanced closed a visually hidden polite status region announces the analysis line, the counts and the Pick colours state; it is absent while Advanced is open", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, true);
    await waitState(s, "auto");
    await settledCounts(s);
    const line = await lineText(s);
    const counts = (await advancedStatus(s)).match(/\d+ paths?, \d+ nodes?, [\d.]+ s/)![0];
    assert.equal(await hiddenStatus(s), "absent");
    await setAdvanced(s, false);
    const region = JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('${BAR} [data-trace-hidden-status]'); const r = e.getBoundingClientRect();
      return { role: e.getAttribute('role'), live: e.getAttribute('aria-live'), w: r.width, h: r.height, text: e.textContent }; })())`)) as string) as { role: string; live: string; w: number; h: number; text: string };
    assert.equal(region.role, "status");
    assert.equal(region.live, "polite");
    assert.ok(region.w <= 1 && region.h <= 1, `visually hidden: ${region.w} x ${region.h}`);
    assert.ok(region.text.includes(line), `announces the analysis line "${line}": ${region.text}`);
    assert.ok(region.text.includes(counts), `announces the counts "${counts}": ${region.text}`);
    // It follows a change of the result.
    await typeInto(s, "Threshold", "0");
    await waitFor("0 paths announced", async () => ((await hiddenStatus(s)).includes(counts) ? null : true));
    // Pick colours with nothing picked.
    await pickMethod(s, "Pick colours");
    await waitFor("the no-colour state announced", async () => ((await hiddenStatus(s)).includes("0 paths: no colour picked.") ? true : null));
    assert.equal(await noticeText(s), "");
  });
});

// ------------------------------------------------------------------ 0025 57 / ux-notes 22.4: the Tab order

/** Names the focused element: a group's label for a radio group (one stop), else its label or text. */
const focusedStop = async (s: S) =>
  (await s.evaluate(`(() => { const e = document.activeElement; if (!e || e === document.body) return 'body';
    if (!e.closest('${BAR}')) return 'outside';
    const group = e.closest('[role=radiogroup]'); if (group) return group.getAttribute('aria-label');
    if (e.hasAttribute('data-trace-palette-swatch')) return 'swatch';
    if (e.hasAttribute('data-trace-palette-action')) return 'action ' + e.getAttribute('data-trace-palette-action');
    return e.getAttribute('aria-label') || e.textContent.trim(); })()`)) as string;

async function tabStops(s: S) {
  await focusCanvas(s);
  const stops: string[] = [];
  for (let i = 0; i < 40; i++) {
    await press(s, "Tab", 9);
    const name = await focusedStop(s);
    if (name === "body" || name === "outside") {
      break;
    }
    stops.push(name);
  }
  return stops;
}

test("0025 tidy-up c57: Tab order with Advanced closed follows ux-notes 22.4 for each method", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pale.png", PALE, false);
    await pickMethod(s, "Brightness");
    const brightness = await tabStops(s);
    console.log("Brightness:", JSON.stringify(brightness));
    assert.deepEqual(brightness, BRIGHTNESS_BAR);
    await pickMethod(s, "Remove background");
    const removal = await tabStops(s);
    console.log("Remove background:", JSON.stringify(removal));
    assert.deepEqual(removal.slice(0, 5), ["About Trace", "Preset", "View", "Method", "Colour tolerance"]);
    const group = removal.slice(5, removal.indexOf("Smooth"));
    assert.ok(group.indexOf("Auto") >= 0 && group.findIndex((g) => /^Pick background colour/.test(g)) > group.indexOf("Auto"), `Auto, then the pipette: ${JSON.stringify(group)}`);
    assert.deepEqual(removal.slice(removal.indexOf("Smooth")), ["Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"]);
    await pickMethod(s, "Pick colours");
    const pick = await tabStops(s);
    console.log("Pick colours:", JSON.stringify(pick));
    assert.deepEqual(pick, ["About Trace", "Preset", "View", "Method", "Colour tolerance", "Pick colour from the image", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"]);
  });
});

test("0025 tidy-up c57: Tab order with Advanced open continues Analyse, Output, Colours, palette pipette, strip, Invert, Drop border shapes, Recognise text (0091 ux-notes §7; not drawn in Multicolour, 0091 criterion 17), Denoise, Upscale, Corners", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pale.png", PALE, true);
    await pickMethod(s, "Brightness");
    await settledCounts(s);
    const single = await tabStops(s);
    console.log("Brightness, Single, Advanced open:", JSON.stringify(single));
    assert.deepEqual(single, [...BRIGHTNESS_BAR.slice(0, -2), "Fewer trace options", "Trace", "Analyse", "Output", "Invert", "Drop border shapes", "Recognise text", "Denoise", "Upscale", "Corners"]);
    await setOutput(s, "Multicolour");
    await settledCounts(s);
    const multi = await tabStops(s);
    console.log("Brightness, Multicolour, Advanced open:", JSON.stringify(multi));
    const tail = multi.slice(multi.indexOf("Trace") + 1);
    const strip = tail.filter((n) => n === "swatch" || n.startsWith("action "));
    assert.ok(strip.length >= 1 && strip.filter((n) => n === "swatch").length === 1, `the strip is one Tab stop: ${JSON.stringify(strip)}`);
    assert.deepEqual(tail.filter((n) => n !== "swatch" && !n.startsWith("action ")), ["Analyse", "Output", "Colours", "Pick palette colour from the image", "Invert", "Drop border shapes", "Denoise", "Upscale", "Corners"]);
    assert.ok(tail.indexOf("swatch") > tail.indexOf("Pick palette colour from the image") && tail.indexOf("swatch") < tail.indexOf("Invert"), "the strip sits between the palette pipette and Invert");
    // Pick colours has its pipette in the bar and none in Advanced.
    await pickMethod(s, "Pick colours");
    await setOutput(s, "Single colour");
    const pick = await tabStops(s);
    console.log("Pick colours, Advanced open:", JSON.stringify(pick));
    assert.equal(pick.filter((n) => /^Pick .*colour/.test(n)).length, 1, "one pipette per method");
    assert.ok(pick.indexOf("Pick colour from the image") < pick.indexOf("Smooth"));
  });
});

// ------------------------------------------------------------------ 0089 22 / ux-notes 22.1: the bar heights at 800 x 600

test("0089 tidy-up 22.1: bar heights at 800 x 600 with Advanced closed, panel open and closed (Brightness 252 / 108, Remove background 288 / 108, Pick colours 252 / 108 px)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO, false);
    const expected: Record<string, [number, number]> = { Brightness: [252, 108], "Remove background": [288, 108], "Pick colours": [252, 108] };
    const seen: string[] = [];
    for (const method of ["Brightness", "Remove background", "Pick colours"]) {
      await pickMethod(s, method);
      for (const open of [true, false]) {
        await setPanel(s, open);
        const height = await barHeight(s);
        seen.push(`${method}, panel ${open ? "open" : "closed"}: ${height} px`);
        assert.equal(await noticeText(s), "", `${method}: no notice row`);
        assert.ok(Math.abs(height - expected[method][open ? 0 : 1]) <= 4, `${method}, panel ${open ? "open" : "closed"}: ${height} px, ux-notes 22.1 says ${expected[method][open ? 0 : 1]} px`);
      }
    }
    console.log(seen.join("\n"));
    // Pick colours: Colour tolerance, the pipette and its label share a line with the panel open.
    await setPanel(s, true);
    const tops = JSON.parse((await s.evaluate(`JSON.stringify(['[aria-label="Colour tolerance"]', '[data-trace-pipette]', '[data-trace-picked-count]'].map((q) => { const r = document.querySelector('${BAR} ' + q).getBoundingClientRect(); return Math.round((r.top + r.bottom) / 2); }))`)) as string) as number[];
    assert.ok(Math.max(...tops) - Math.min(...tops) <= 3, `Colour tolerance, pipette and label on one line (centres): ${JSON.stringify(tops)}`);
  }, "", 800, 600);
});

test("0089 tidy-up 22.3: the Advanced block adds about 222 px (panel open) and 150 px (closed) to the Single colour Brightness bar at 800 x 600", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES, false);
    await pickMethod(s, "Brightness");
    await settledCounts(s).catch(() => undefined);
    const closed: Record<string, number> = {};
    for (const open of [true, false]) {
      await setPanel(s, open);
      closed[String(open)] = await barHeight(s);
    }
    await setAdvanced(s, true);
    await sleep(500);
    for (const open of [true, false]) {
      await setPanel(s, open);
      const height = await barHeight(s);
      const added = height - closed[String(open)];
      console.log(`Advanced open, panel ${open ? "open" : "closed"}: bar ${height} px, block ${added} px`);
      assert.ok(Math.abs(added - (open ? 222 : 150)) <= 40, `Advanced adds ${added} px, ux-notes 22.3 says about ${open ? 222 : 150}`);
    }
  }, "", 800, 600);
});

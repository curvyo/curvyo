// Tester cases for `specs/0025-raster-trace` slice 3 in the real app (Chromium via DevTools, set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the View pill and the
// peek, the progress slot (Stop, Stopped and Retry, Failed and Retry; round 3 of 2026-10-29) (a trace worker that dies at start, put
// in by a script that runs before the page: no production code is touched), a large image, several
// images, the 100,000-node warning and the keyboard route. Harness as in `acceptance_0025_slice2_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5227;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0025-slice3");
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
  const closing = new Set<() => void>();
  ws.onclose = () => closing.forEach((fail) => fail());
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
  // A call the browser never answers (it died or closed the connection) fails with its name
  // instead of leaving the test pending until the event loop is empty.
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const mine = id;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method} in 300 s`)), 300_000);
      pending.set(mine, (v) => {
        clearTimeout(timer);
        resolve(v);
      });
      closing.add(() => {
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


/** A `width` x `height` opaque RGB PNG (96 dpi by default: no pHYs). */
function png(width: number, height: number, rgb: [number, number, number]): Buffer {
  const chunk = (kind: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: width }, () => rgb).flat())]);
  const raw = Buffer.concat(Array.from({ length: height }, () => row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}


/** A grey 8-bit PNG; `grey(x, y)` is the grey of a pixel. */
function pngGrey(width: number, height: number, grey: (x: number, y: number) => number): Buffer {
  const chunk = (kind: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 0;
  const raw = Buffer.alloc((width + 1) * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      raw[y * (width + 1) + 1 + x] = grey(x, y);
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A ring (outer r 60, hole r 20) on white, 200 x 200 px. */
const ring = (x: number, y: number) => {
  const r = Math.hypot(x + 0.5 - 100, y + 0.5 - 100);
  return r < 60 && r > 20 ? 0 : 255;
};

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

type S = Awaited<ReturnType<typeof session>>;

/** Focus on the canvas, where the tool letters work. */
const focusCanvas = async (s: S) => {
  await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
};

const buttonByText = async (s: S, text: string) => {
  const found = await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25'));
    const b = [...document.querySelectorAll('button')].find((e) => e.offsetParent && e.textContent.trim() === ${JSON.stringify(text)});
    if (!b) return false; b.setAttribute('data-t25', '1'); return true; })()`);
  return found as boolean;
};
const clickText = async (s: S, text: string) => {
  assert.ok(await buttonByText(s, text), `a button "${text}" is on the page`);
  await s.clickSel("[data-t25]");
};
/** Creates the paths the way the bar has no button for: Enter on the canvas (UX notes 19). */
const createPaths = async (s: S) => {
  await focusCanvas(s);
  await s.key("Enter", 13);
};
/** Presses an icon button (no text) by its accessible name. */
const clickAria = async (s: S, label: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); const b = document.querySelector('${barQ} button[aria-label="${label}"]'); b.setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
};
const EXPAND_CLOSED = "More trace options";
const EXPAND_OPEN = "Fewer trace options";
const press = async (s: S, k: string, vk: number, modifiers = 0) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await sleep(250);
};
// The bar's fields are the app's value fields (Frage 78, UX notes 20.1): a spinbutton whose
// `aria-valuetext` carries the shown value; F2 opens the number entry. The Threshold is in percent.
const field = (name: string) => `[role=toolbar][aria-label=Trace] [role=spinbutton][aria-label="${name}"]`;
const fieldValue = async (s: S, name: string) =>
  (await s.evaluate(`(() => { const f = document.querySelector('${field(name)}'); return f ? (f.getAttribute('aria-valuetext') ?? '').split(' ')[0] : null; })()`)) as string | null;
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${field(name)}').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
// The state row's line (UX notes 20.4), else the counts line of the Advanced block, else an empty
// bar's words.
const barStatus = async (s: S) =>
  (await s.evaluate(`(() => { const bar = document.querySelector('[role=toolbar][aria-label=Trace]');
    const line = bar?.querySelector('[data-trace-line]')?.textContent.trim() ?? '';
    if (line) return line;
    const counts = bar?.querySelector('[data-trace-counts] > span')?.textContent.trim() ?? '';
    if (counts) return counts;
    return [...document.querySelectorAll('[role=status]')].map((e) => e.textContent.trim()).filter((t) => /Click an image|not an image/.test(t)).join(' | '); })()`)) as string;
const traceReady = async (s: S) =>
  waitFor("a settled trace", async () => {
    const t = await barStatus(s);
    return /\d+ paths?, [\d,]+ nodes?, [\d.]+ s|0 paths|covering the whole image|nodes\. Slow to edit/.test(t) ? t : null;
  });
const canvasRect = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string,
  ) as [number, number, number, number];
/** Counts screenshot pixels inside the canvas: near black, and near the preview blue. */
const pixels = async (s: S) => {
  const [cx, cy, cw, ch] = await canvasRect(s);
  const blue = JSON.parse(
    (await s.evaluate(`(() => { const e = document.createElement('i'); e.style.color = 'var(--preview-new)'; document.body.appendChild(e);
      const c = getComputedStyle(e).color; e.remove(); return JSON.stringify(c.match(/[\\d.]+/g).slice(0, 3).map(Number)); })()`)) as string,
  ) as number[];
  const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  const png = decodePng(Buffer.from(shot.result?.data ?? "", "base64"));
  // The 36 px analysis row of 0089 slice F makes the bar taller, so its blue finish button (the same
  // blue as the preview outline) now lies below canvas top + 80: sample below the bar's bottom edge.
  const barBottom = (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace]')?.getBoundingClientRect().bottom ?? 0`)) as number;
  let black = 0;
  let near = 0;
  for (let y = Math.max(Math.floor(cy) + 80, Math.ceil(barBottom) + 4); y < cy + ch; y++) {
    for (let x = Math.floor(cx) + 400; x < cx + cw - 20; x++) {
      const i = (y * png.width + x) * 4;
      const [r, g, b] = [png.data[i], png.data[i + 1], png.data[i + 2]];
      if (r < 40 && g < 40 && b < 40) black++;
      if (Math.abs(r - blue[0]) + Math.abs(g - blue[1]) + Math.abs(b - blue[2]) < 60) near++;
    }
  }
  return { black, blue: near, previewBlue: blue };
};
const selectedRows = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')).join('|')`)) as string;
const rowInfo = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-level]')].map((r) => [r.getAttribute('aria-label'), Number(r.getAttribute('aria-level'))]))`)) as string,
  ) as [string, number][];
const historyText = async (s: S) => {
  await s.tab("History");
  const t = (await s.evaluate(`(() => { const p = document.querySelector('[role=tabpanel]'); return p ? p.innerText : document.body.innerText; })()`)) as string;
  await s.tab("Layers");
  return t;
};
const countOf = (text: string, needle: string) => text.split(needle).length - 1;

async function importImage(s: S, name: string, grey: (x: number, y: number) => number, w = 200, h = 200) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, pngGrey(w, h, grey));
  const before = (await s.layerRows()).length;
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).length > before ? true : null));
}



/** Starts the app. `preload` runs before the page's own scripts (fault injection for the worker). */
async function launch(preload?: string) {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  // Its own process group: killing `npx` alone leaves vite holding the port for the next test.
  const vite = startVite(PORT, FRONTEND);
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
    await stopVite(vite, PORT);
    removeProfile(profile);
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

const CARD = "[...document.querySelectorAll('[role=alert]')].find((e) => /Trace failed/.test(e.textContent))";
const barQ = "[role=toolbar][aria-label=Trace]";
const bar = async (s: S) => (await s.evaluate(`document.querySelector('${barQ}')?.innerText ?? ''`)) as string;
const nodesOf = (status: string) => Number(/([\d,]+) nodes?/.exec(status)?.[1].replace(/,/g, ""));
const settled = (t: string) => /\d+ paths?, [\d,]+ nodes?, [\d.]+ s|0 paths|covering the whole image|nodes\. Slow to edit/.test(t);
const settle = async (s: S) => {
  await sleep(700);
  return waitFor("a settled trace", async () => {
    const t = await barStatus(s);
    return settled(t) ? t : null;
  }, 300_000);
};
const trace = async (s: S) => (await s.evaluate(`document.querySelector('${barQ}') !== null`)) as boolean;
const openTrace = async (s: S) => {
  await focusCanvas(s);
  await s.key("i", 73);
  await waitFor("the Trace bar", async () => ((await trace(s)) ? true : null));
};
const html = async (s: S, expr: string) => s.evaluate(expr);
/** The text of every button in the bar, visible ones only. */
const barButtons = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${barQ} button')].filter((b) => b.offsetParent).map((b) => (b.getAttribute('aria-label') || b.textContent).trim()))`)) as string,
  ) as string[];
const hasButton = async (s: S, name: string) => (await barButtons(s)).includes(name);
const busyAttr = async (s: S) => (await s.evaluate(`document.querySelector('${barQ}')?.getAttribute('aria-busy')`)) as string | null;
const hasProgress = async (s: S) => (await s.evaluate(`!!document.querySelector('${barQ} [role=progressbar], [role=progressbar]')`)) as boolean;
/** The x of the progress pill (UX notes 21.2). */
const STOP_X = "Stop (Esc)";
/** Clicks the pill's x. */
const clickStop = (s: S) => clickAria(s, STOP_X);
/** The text tooltip of the element `sel` in the bar, after a 700 ms hover. */
const tooltipOf = async (s: S, sel: string) => {
  const box = JSON.parse((await s.evaluate(`(() => { const r = document.querySelector('${barQ} ${sel}').getBoundingClientRect(); return JSON.stringify([r.x + r.width / 2, r.y + r.height / 2]); })()`)) as string) as [number, number];
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box[0], y: box[1] });
  await sleep(700);
  const text = (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((t) => t.textContent).join(' ')`)) as string;
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 1, y: 1 });
  await sleep(200);
  return text;
};
/** The slot of row 1: its state and geometry (UX notes 21.2). */
const slotInfo = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('${barQ} [data-trace-slot]'); if (!e) return null; const r = e.getBoundingClientRect();
      const sp = document.querySelector('${barQ} [role=spinbutton][aria-label="Ignore below"]').getBoundingClientRect();
      const ex = document.querySelector('${barQ} button[aria-expanded][aria-controls]').getBoundingClientRect();
      const pb = e.querySelector('[role=progressbar]');
      return { kind: e.getAttribute('data-trace-slot'), text: e.innerText.trim(), width: Math.round(r.width), height: Math.round(r.height), afterSpecks: r.top > sp.bottom - 2 || r.left >= sp.right,
        afterExpander: r.top > ex.bottom - 2 || r.left >= ex.right, progress: pb ? { name: pb.getAttribute('aria-label'), now: pb.getAttribute('aria-valuenow') } : null,
        buttons: [...e.querySelectorAll('button')].map((b) => [(b.getAttribute('aria-label') || b.textContent).trim(), Math.round(b.getBoundingClientRect().width), Math.round(b.getBoundingClientRect().height)]) }; })())`)) as string,
  ) as { kind: string; text: string; width: number; height: number; afterSpecks: boolean; afterExpander: boolean; progress: { name: string; now: string | null } | null; buttons: [string, number, number][] } | null;
/** The finish button at rest (0089 criterion 22a, customer 2026-10-30): always blue, "Trace", 75 x 28 px. */
const atRest = async (s: S, when: string) => {
  const slot = await slotInfo(s);
  assert.equal(slot?.kind, "none", `the finish button is idle ${when}`);
  assert.equal(slot?.text, "Trace", `it reads Trace ${when}`);
  assert.ok(Math.abs((slot?.width ?? 0) - 75) <= 2 && slot?.height === 28, `75 x 28 px ${when}: ${slot?.width} x ${slot?.height}`);
};
const mutedLines = async (s: S) => (await bar(s)).split("\n").map((l) => l.trim()).filter(Boolean);

/** Many small shapes, so a trace takes a while. */
const dense = (x: number, y: number) => (Math.sin(x * 0.21) * Math.cos(y * 0.17) + Math.sin((x + y) * 0.13) * 0.8 > 0.15 ? 0 : 255);
/** A grid of discs of radius 3 px every 8 px: hundreds of thousands of nodes when large. */
const discs = (x: number, y: number) => {
  const dx = (x % 8) - 3.5;
  const dy = (y % 8) - 3.5;
  return dx * dx + dy * dy < 9 ? 0 : 255;
};
const blobs = (x: number, y: number) => (Math.sin(x * 0.05) + Math.cos(y * 0.04) + Math.sin((x + y) * 0.03) > 0.4 ? 0 : 255);

const mouseWheel = async (s: S, x: number, y: number, deltaY: number, ctrl = true) => {
  await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x, y, deltaX: 0, deltaY, modifiers: ctrl ? 2 : 0 });
  await sleep(400);
};

const keyDownOnly = (s: S, key: string, code: string, vk: number, modifiers = 0, repeat = false) =>
  s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key, code, windowsVirtualKeyCode: vk, modifiers, autoRepeat: repeat });
const keyUpOnly = (s: S, key: string, code: string, vk: number, modifiers = 0) =>
  s.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: vk, modifiers });

const viewChecked = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].filter((r) => r.getAttribute('aria-checked') === 'true').map((r) => r.getAttribute('aria-label')).join(',')`)) as string;
const clickView = async (s: S, name: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
  await sleep(300);
};

test("0025s3 c51 in Chromium: the View pill, what each view draws, the peek, Shift+backslash (criterion 51)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    await openTrace(s);
    await settle(s);
    // The screenshot counts below assume the bar of slice 3: the Advanced block (open in this file
    // for the counts) is closed so its blue choices are not counted as outline.
    await clickAria(s, "Fewer trace options");
    await sleep(300);
    // The pill: radiogroup "View", three icon segments with tooltips, Overlay by default, in row 1.
    const names = (await s.evaluate(`[...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].map((r) => r.getAttribute('aria-label')).join(',')`)) as string;
    // Mask is the second segment since `specs/0089` criterion 21 (UX notes §4).
    assert.equal(names, "Overlay,Mask,Trace,Image");
    assert.equal(await viewChecked(s), "Overlay");
    const geo = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const g = document.querySelector('[role=radiogroup][aria-label=View]').getBoundingClientRect(); const p = document.querySelector('button[aria-label=Preset]').getBoundingClientRect(); return { sameRow: Math.abs(g.y + g.height / 2 - (p.y + p.height / 2)) < 6, rightOfPreset: g.x >= p.right, segs: [...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].map((r) => { const b = r.getBoundingClientRect(); return [Math.round(b.width), Math.round(b.height)]; }) }; })())`)) as string,
    ) as { sameRow: boolean; rightOfPreset: boolean; segs: number[][] };
    assert.ok(geo.sameRow && geo.rightOfPreset, `View sits right of Preset in row 1: ${JSON.stringify(geo)}`);
    console.log("View segments (w,h):", JSON.stringify(geo.segs));
    // Tooltips name the segment; the Image tooltip adds the key.
    const tipOf = async (name: string) => {
      await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').setAttribute('data-t25', '1'); })()`);
      const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-t25]').getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 600 });
      await sleep(600);
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c[0] - 2, y: c[1] });
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c[0], y: c[1] });
      await sleep(1200);
      return (await s.evaluate(`[...new Set([...document.querySelectorAll('[role=tooltip]')].map((t) => t.textContent))].join(' | ')`)) as string;
    };
    const tips = [await tipOf("Overlay"), await tipOf("Trace"), await tipOf("Image")];
    console.log("View tooltips:", JSON.stringify(tips));
    assert.match(tips[0], /Overlay/);
    assert.match(tips[1], /Trace/);
    assert.match(tips[2], /Image/);
    assert.match(tips[2], /\\/, "the Image tooltip names the key");

    // What each view draws (screenshot pixels): Overlay = dim picture + blue outline.
    const overlay = await pixels(s);
    await clickView(s, "Trace");
    assert.equal(await viewChecked(s), "Trace");
    const traceView = await pixels(s);
    await clickView(s, "Image");
    const imageView = await pixels(s);
    console.log("pixels overlay/trace/image:", JSON.stringify({ overlay, traceView, imageView }));
    assert.ok(overlay.blue > 100, `Overlay draws the outline: ${overlay.blue}`);
    assert.ok(overlay.black < imageView.black / 3, `Overlay dims the picture: ${overlay.black} vs ${imageView.black}`);
    assert.ok(traceView.blue < overlay.blue / 5, `Trace draws no outline: ${traceView.blue}`);
    assert.ok(traceView.black > imageView.black * 0.7, `Trace shows the result filled in black: ${traceView.black} vs ${imageView.black}`);
    assert.ok(imageView.blue < overlay.blue / 5, `Image draws no outline: ${imageView.blue}`);
    assert.ok(imageView.black > 1000, `Image shows the original at full strength: ${imageView.black}`);

    // Zoom works in every view (the black area changes with the zoom).
    const [cx, cy, cw, ch] = await canvasRect(s);
    for (const view of ["Overlay", "Trace", "Image"]) {
      await clickView(s, view);
      const a = await pixels(s);
      await mouseWheel(s, cx + cw * 0.6, cy + ch * 0.5, -300);
      const b = await pixels(s);
      await mouseWheel(s, cx + cw * 0.6, cy + ch * 0.5, 300);
      assert.ok(b.black !== a.black || b.blue !== a.blue, `zoom changes the picture in the ${view} view`);
    }
    await clickView(s, "Overlay");

    // The peek: hold backslash, the original shows and Image is pressed; release returns.
    await focusCanvas(s);
    await keyDownOnly(s, "\\", "Backslash", 220);
    await sleep(500);
    assert.equal(await viewChecked(s), "Image", "the pill tells the truth while the key is held");
    const held = await pixels(s);
    assert.ok(held.blue < overlay.blue / 5 && held.black > overlay.black * 3, `peek shows the original: ${JSON.stringify(held)}`);
    await keyDownOnly(s, "\\", "Backslash", 220, 0, true);
    await keyDownOnly(s, "\\", "Backslash", 220, 0, true);
    await sleep(200);
    assert.equal(await viewChecked(s), "Image", "key repeat changes nothing");
    await keyUpOnly(s, "\\", "Backslash", 220);
    await sleep(500);
    assert.equal(await viewChecked(s), "Overlay", "release returns to the chosen view");
    const back = await pixels(s);
    assert.ok(Math.abs(back.blue - overlay.blue) < overlay.blue * 0.2 + 50, "the outline is back");
    // Window blur restores.
    await keyDownOnly(s, "\\", "Backslash", 220);
    await sleep(300);
    assert.equal(await viewChecked(s), "Image");
    await s.evaluate(`window.dispatchEvent(new Event('blur'))`);
    await sleep(300);
    assert.equal(await viewChecked(s), "Overlay", "window blur restores the chosen view");
    await keyUpOnly(s, "\\", "Backslash", 220);
    // The peek from a bar button too.
    await s.evaluate(`document.querySelector('${barQ} [role=spinbutton][aria-label=Threshold]').focus()`);
    await keyDownOnly(s, "\\", "Backslash", 220);
    await sleep(300);
    const fromField = await viewChecked(s);
    await keyUpOnly(s, "\\", "Backslash", 220);
    console.log("peek with focus in the Threshold field (text field: expected ignored):", fromField);
    assert.equal(fromField, "Overlay", "ignored in text fields");
    await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); document.querySelector('button[aria-label=Preset]').setAttribute('data-t25', '1'); document.querySelector('button[aria-label=Preset]').focus(); })()`);
    await keyDownOnly(s, "\\", "Backslash", 220);
    await sleep(300);
    assert.equal(await viewChecked(s), "Image", "the peek works with focus on a bar button");
    await keyUpOnly(s, "\\", "Backslash", 220);
    await sleep(300);

    // Shift+backslash cycles Overlay -> Mask -> Trace -> Image -> Overlay (Mask since `specs/0089`
    // criterion 21).
    await focusCanvas(s);
    const seen: string[] = [];
    for (let i = 0; i < 4; i++) {
      await keyDownOnly(s, "|", "Backslash", 220, 8);
      await keyUpOnly(s, "|", "Backslash", 220, 8);
      await sleep(300);
      seen.push(await viewChecked(s));
    }
    assert.deepEqual(seen, ["Mask", "Trace", "Image", "Overlay"]);

    // Arrow keys in the pill move and select; one Tab stop; the view is not remembered.
    await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-checked=true]').focus()`);
    await press(s, "ArrowRight", 39);
    assert.equal(await viewChecked(s), "Mask");
    await press(s, "ArrowRight", 39);
    assert.equal(await viewChecked(s), "Trace");
    await press(s, "ArrowRight", 39);
    await press(s, "ArrowRight", 39);
    assert.equal(await viewChecked(s), "Overlay", "arrows wrap");
    await press(s, "ArrowLeft", 37);
    assert.equal(await viewChecked(s), "Image");
    const tabindexes = (await s.evaluate(`[...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].map((r) => r.tabIndex).join(',')`)) as string;
    assert.equal(tabindexes, "-1,-1,-1,0", "one Tab stop for the pill");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);
    await openTrace(s);
    await advancedOpen(s);
    await settle(s);
    assert.equal(await viewChecked(s), "Overlay", "entering the tool starts on Overlay");
    // Nothing of this touched the document: no history step beyond the import.
    const hist = await historyText(s);
    assert.ok(!/Trace image/.test(hist), "views write nothing");
  } finally {
    await close();
  }
});

/** Stop, "Stopped" with Retry in the slot: the checks that hold for every Stop (criterion 53, UX notes 21.2). */
async function stopAndAgain(s: S, label: string) {
  await clickStop(s);
  await sleep(600);
  const slot = await slotInfo(s);
  assert.equal(slot?.kind, "stopped", `${label}: the slot after Stop: ${JSON.stringify(slot)}`);
  assert.match(slot?.text ?? "", /^Stopped\s*Retry$/, label);
  assert.ok(!/Preview is out of date/.test(await bar(s)), `${label}: the words are no longer drawn`);
  assert.match(await tooltipOf(s, "[data-trace-slot=stopped] span"), /Preview is out of date/, `${label}: the pill's tooltip`);
  assert.ok(!(await hasButton(s, "Trace again")), `${label}: there is no Trace again button`);
  assert.ok(await hasButton(s, "Retry"), `${label}: Retry after Stop`);
  assert.ok(!(await hasButton(s, STOP_X)), label);
  assert.equal(await busyAttr(s), null, `${label}: the bar has no aria-busy`);
  assert.ok(!(await hasProgress(s)), `${label}: the progress pill is gone`);
  assert.ok(!/Trace image/.test(await historyText(s)), `${label}: nothing was written`);
}

test("0025s3 c53/c58 in Chromium: Stop, Retry, Enter while stale, Escape, the worker ends (criteria 10, 53, 58)", { skip: !BROWSER, timeout: 600_000 }, async () => {
  const { s, close } = await launch(WORKER_FAULT);
  try {
    await s.tab("Layers");
    await importImage(s, "blobs.png", blobs, 300, 300);
    const rowsBefore = await s.layerRows();
    await openTrace(s);
    await advancedOpen(s); // the counts, which `settle` reads, live in the Advanced block (0089 UX notes 20.4)
    await settle(s);
    await atRest(s, "at rest");
    await s.evaluate(`window.__delay = 6000`);
    // A change of value starts a job the slow worker answers after 6 s.
    await typeInto(s, "Smooth", "41");
    const t0 = Date.now();
    await waitFor("the progress pill", async () => ((await hasProgress(s)) ? true : null), 10_000);
    const shownAt = Date.now() - t0;
    console.log("pill shown", shownAt, "ms after the change");
    assert.ok(shownAt >= 400, `the pill waits for 400 ms of work: ${shownAt}`);
    assert.ok(await hasButton(s, STOP_X), "the x shows together with the pill");
    const running = await slotInfo(s);
    console.log("running slot:", JSON.stringify(running));
    assert.equal(running?.kind, "running");
    assert.equal(running?.progress?.name, "Tracing", "role=progressbar named Tracing");
    assert.match(running?.text ?? "", /^(\d{1,3} %|\.\.\.)$/, "the percentage or ...");
    assert.ok(Math.abs((running?.width ?? 0) - 75) <= 2 && running?.height === 28, `about 75 x 28 px: ${running?.width} x ${running?.height}`);
    assert.ok(running?.afterSpecks && running?.afterExpander, "the finish button is after Specks and after the expander (0089 criterion 22a)");
    assert.deepEqual(running?.buttons.map(([n, w, h]) => [n, w >= 28 && h >= 28]), [[STOP_X, true]], "the x is a 28 px target");
    assert.ok(!/Tracing\.\.\./.test(await bar(s)), "no Tracing... text");
    assert.equal(await busyAttr(s), null, "no aria-busy");
    // The finish button's tab order: right after the expander (0089 criterion 22a).
    const order = await barButtons(s);
    const expanderAt = order.findIndex((n) => n === EXPAND_CLOSED || n === EXPAND_OPEN);
    assert.ok(order.indexOf(STOP_X) >= 0 && order.indexOf(STOP_X) === expanderAt + 1, `the x comes right after the expander (the finish button is last): ${order}`);
    await stopAndAgain(s, "preview");
    assert.equal(await fieldValue(s, "Smooth"), "41", "values stay");
    // Enter while stale traces again, never Create.
    await s.evaluate(`window.__delay = 0`);
    await focusCanvas(s);
    await s.key("Enter", 13);
    await settle(s);
    assert.ok(!(await hasButton(s, "Retry")), "Enter traced again");
    assert.deepEqual(await s.layerRows(), rowsBefore, "Enter while stale never creates");
    // Stop again; Retry traces again; a stop ends the worker (a fresh one starts).
    await s.evaluate(`window.__delay = 6000`);
    await typeInto(s, "Smooth", "43");
    await waitFor("Stop", async () => ((await hasButton(s, STOP_X)) ? true : null), 10_000);
    const workersBefore = (await s.evaluate(`window.__workers`)) as number;
    await clickStop(s);
    await sleep(500);
    assert.ok(!(await hasButton(s, "Trace again")) && !(await hasButton(s, STOP_X)) && (await hasButton(s, "Retry")), "Retry replaces the x");
    await s.evaluate(`window.__delay = 0`);
    await clickText(s, "Retry");
    const again = await settle(s);
    console.log("settled after Retry:", again);
    assert.ok(!(await hasButton(s, "Retry")) && !(await hasButton(s, STOP_X)));
    assert.ok(!/out of date|Stopped/.test(await bar(s)), "the stale state is gone");
    await atRest(s, "after Retry");
    assert.ok(((await s.evaluate(`window.__workers`)) as number) > workersBefore, "Stop terminated the worker: Retry used a new one (criterion 58)");
    // A changed value while stale traces again too.
    await s.evaluate(`window.__delay = 6000`);
    await typeInto(s, "Smooth", "47");
    await waitFor("Stop", async () => ((await hasButton(s, STOP_X)) ? true : null), 10_000);
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(600);
    assert.ok(await trace(s), "first Escape keeps the tool");
    assert.match(await bar(s), /Stopped/);
    assert.ok(await hasButton(s, "Retry"), "first Escape leaves Stopped with Retry");
    await s.evaluate(`window.__delay = 0`);
    await typeInto(s, "Smooth", "48");
    await settle(s);
    assert.ok(!/Stopped|out of date/.test(await bar(s)), "changing a value traces again");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(400);
    assert.ok(!(await trace(s)), "Escape on a settled preview leaves the tool");
    assert.deepEqual(await s.layerRows(), rowsBefore, "nothing was written");
  } finally {
    await close();
  }
});

test("0025s3 c54 in Chromium: Failed in the slot, Retry, values and document stay (criterion 54)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch(WORKER_FAULT);
  try {
    await s.tab("Layers");
    await importImage(s, "blobs.png", blobs, 300, 300);
    const rowsBefore = await s.layerRows();
    const historyBefore = await historyText(s);
    await s.evaluate(`window.__failWorker = true`);
    await openTrace(s);
    await advancedOpen(s); // the counts, which `traceReady` reads after Retry, live in Advanced
    const card = await waitFor("the failure alert", async () =>
      ((await s.evaluate(`(${CARD})?.textContent ?? null`)) as string | null), 30_000);
    console.log("failure alert:", JSON.stringify(card));
    assert.match(card, /^Trace failed: .+\. Your drawing is unchanged\.$/);
    assert.ok(!/at |Error|boom|undefined/.test(card.replace("Trace failed", "")), "no stack, no code");
    const info = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const a = ${CARD}; const ar = a.getBoundingClientRect(); const pill = document.querySelector('${barQ} [data-trace-slot=failed] span'); const cs = getComputedStyle(pill); const probe = document.createElement('i'); probe.style.color = 'var(--field-invalid)'; document.body.appendChild(probe); const invalid = getComputedStyle(probe).color; probe.remove();
        return { hidden: ar.width <= 1 && ar.height <= 1, inBar: document.querySelector('${barQ}').contains(a), shadow: cs.boxShadow, border: cs.borderTopColor + ' ' + cs.borderTopWidth, invalid, word: pill.textContent.trim(), glyph: !!pill.querySelector('svg'), focusInside: document.querySelector('${barQ} [data-trace-slot]').contains(document.activeElement),
          cards: [...document.querySelectorAll('div')].filter((d) => d.getBoundingClientRect().width > 1 && /^Trace failed/.test((d.innerText || '').trim())).length }; })())`)) as string,
    ) as Record<string, unknown>;
    console.log("failed slot:", JSON.stringify(info), JSON.stringify(await slotInfo(s)));
    assert.equal(info.hidden, true, "the alert is visually hidden");
    assert.equal(info.inBar, true);
    assert.equal(info.cards, 0, "the failure card is gone");
    assert.equal(info.word, "Failed");
    assert.equal(info.glyph, true, "red is not the only cue: a glyph");
    assert.equal(info.focusInside, false, "the slot does not take focus");
    assert.ok(String(info.border + String(info.shadow)).includes(String(info.invalid)), `a ring in --field-invalid: ${info.border} / ${info.shadow} vs ${info.invalid}`);
    const failed = await slotInfo(s);
    assert.equal(failed?.kind, "failed");
    assert.ok(failed?.afterSpecks && failed?.afterExpander, "Failed sits in the finish button, after the expander");
    assert.deepEqual(failed?.buttons.map(([n, w, h]) => [n, w >= 28 && h >= 28]), [["Retry", true]], "one Retry, a 28 px target");
    assert.match(await tooltipOf(s, "[data-trace-slot=failed] span"), /^Trace failed: .+\. Your drawing is unchanged\.$/, "the reason is the tooltip");
    // The bar's values stay; the document is exactly as before.
    assert.notEqual(await fieldValue(s, "Smooth"), null);
    assert.deepEqual(await s.layerRows(), rowsBefore);
    // The reason tooltip just read stays open under the pointer. With the analysis row of 0089 slice F the
    // slot is 36 px lower and the tooltip now lies over the History tab: move the pointer away first.
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 700 });
    await sleep(800);
    assert.equal(await historyText(s), historyBefore, "history is untouched");
    // Ctrl+Enter in a field only traces again while failed; it writes nothing.
    await s.evaluate(`(() => { const f = document.querySelector('${field("Smooth")}'); f.focus(); })()`);
    await press(s, "Enter", 13, 2);
    await sleep(500);
    assert.deepEqual(await s.layerRows(), rowsBefore, "Ctrl+Enter (create) on a failed trace writes nothing");
    // Retry with the worker still dying: the card stays (or returns).
    const before = (await s.evaluate(`window.__workers`)) as number;
    await clickText(s, "Retry");
    await sleep(1500);
    assert.match((await s.evaluate(`(${CARD})?.textContent ?? ''`)) as string, /Trace failed/, "a still-dying worker fails again");
    assert.ok(((await s.evaluate(`window.__workers`)) as number) > before, "Retry starts a fresh worker");
    // Now the worker lives: Retry traces and the card goes.
    await s.evaluate(`window.__failWorker = false`);
    await clickText(s, "Retry");
    const ok = await traceReady(s);
    console.log("after Retry:", ok);
    assert.equal(await s.evaluate(`!!(${CARD})`), false, "the alert is gone after a successful trace");
    await atRest(s, "after a successful trace");
    // Enter on the canvas also retries; a changed value clears the card.
    await s.evaluate(`window.__failWorker = true`);
    await typeInto(s, "Smooth", "77");
    await waitFor("card again", async () => ((await s.evaluate(`!!(${CARD})`)) ? true : null), 20_000).catch(() => null);
    // The live worker may be reused; only assert behaviour when a card appeared.
    if (await s.evaluate(`!!(${CARD})`)) {
      await s.evaluate(`window.__failWorker = false`);
      await focusCanvas(s);
      await s.key("Enter", 13);
      await traceReady(s);
      assert.equal(await s.evaluate(`!!(${CARD})`), false, "Enter on the canvas retries");
      assert.equal(await fieldValue(s, "Smooth"), "77", "values stay through failure and retry");
    }
    assert.deepEqual(await s.layerRows(), rowsBefore, "the document never changed");
  } finally {
    await close();
  }
});

const clickRadio = async (s: S, group: string, label: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=radiogroup][aria-label=${group}] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
};
const advancedOpen = async (s: S) => {
  if ((await s.evaluate(`document.querySelector('${barQ} [role=spinbutton][aria-label=Corners]') === null`)) as boolean) {
    await clickAria(s, EXPAND_CLOSED);
    await sleep(400);
  }
};
const switchInfo = async (s: S, label: string) => {
  const info = JSON.parse(
    (await s.evaluate(`(() => { const e = document.querySelector('${barQ} button[aria-label="${label}"]'); return e ? JSON.stringify({ checked: e.getAttribute('aria-pressed'), disabled: e.getAttribute('aria-disabled') }) : 'null'; })()`)) as string,
  ) as { checked: string; disabled: string } | null;
  if (info === null) {
    return "null";
  }
  // The tooltip carries the second line ("Over 4 Mpx"): hover the button for 600 ms.
  const box = JSON.parse((await s.evaluate(`(() => { const r = document.querySelector('${barQ} button[aria-label="${label}"]').getBoundingClientRect(); return JSON.stringify([r.x + r.width / 2, r.y + r.height / 2]); })()`)) as string) as [number, number];
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box[0], y: box[1] });
  await sleep(700);
  const row = (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((t) => t.textContent).join(' ')`)) as string;
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 1, y: 1 });
  return JSON.stringify({ ...info, row });
};

test("0025s3 c52 in Chromium: an image over 4 Mpx, the reduced copy, full-size Create paths with Stop (criterion 52)", { skip: !BROWSER, timeout: 600_000 }, async () => {
  const { s, close } = await launch(WORKER_FAULT);
  try {
    await s.tab("Layers");
    // A small image first: no reduced-copy text; there is no Full-size preview (Frage 74 A).
    await importImage(s, "small.png", blobs, 300, 300);
    await openTrace(s);
    await settle(s);
    assert.ok(!/reduced copy/.test(await bar(s)), "a small image has no reduced-copy line");
    await advancedOpen(s);
    assert.equal(await switchInfo(s, "Full-size preview"), "null", "no Full-size preview control");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);

    // 2400 x 2000 = 4.8 Mpx.
    await importImage(s, "large.png", dense, 2400, 2000);
    const rowsBefore = await s.layerRows();
    await openTrace(s);
    const status = await traceReady(s);
    console.log("large image status:", status, "|", JSON.stringify(await mutedLines(s)));
    assert.match(await bar(s), /Preview from a reduced copy/);
    await advancedOpen(s);
    assert.match(await bar(s), /Preview from a reduced copy/, "the preview is always the reduced copy");

    // Create paths traces at full size: the progress pill at once, document untouched meanwhile.
    // (The real worker needs well under a second here; its answer is held back 5 s.)
    await s.evaluate(`window.__delay = 5000`);
    await createPaths(s);
    const t0 = Date.now();
    let sawStopAt = -1;
    let sawText = "";
    for (let i = 0; i < 100 && sawStopAt < 0; i++) {
      if (await hasButton(s, STOP_X)) {
        sawStopAt = Date.now() - t0;
        sawText = await bar(s);
      }
      await sleep(50);
    }
    console.log("Stop after Create paths (ms):", sawStopAt, "| text:", JSON.stringify(sawText.replace(/\n/g, " / ")));
    assert.ok(sawStopAt >= 0 && sawStopAt < 400, `the pill and its x appear at once (no 400 ms wait): ${sawStopAt}`);
    assert.ok(!/Tracing at full size|Tracing\.\.\./.test(sawText), `no Tracing text: ${sawText}`);
    assert.ok(await hasProgress(s), "the progress pill while the full-size trace runs");
    assert.equal((await slotInfo(s))?.progress?.name, "Tracing");
    assert.equal(await busyAttr(s), null, "no aria-busy");
    assert.deepEqual(await s.layerRows(), rowsBefore, "the document is untouched until the trace finishes");
    // Escape stops it; still untouched.
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(600);
    assert.ok(await trace(s), "first Escape keeps the tool");
    assert.deepEqual(await s.layerRows(), rowsBefore, "a stopped full-size trace changes nothing");
    assert.ok(!/Trace image/.test(await historyText(s)));
    // Let it finish this time.
    await s.evaluate(`window.__delay = 0`);
    assert.ok(!(await hasButton(s, "Trace again")));
    await createPaths(s);
    await settle(s);
    await createPaths(s);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 300_000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
    const rows = await s.layerRows();
    assert.equal(rows.length, rowsBefore.length + 1, `one group appeared: ${JSON.stringify(rows)}`);
    assert.ok(!(await trace(s)), "back in the Select tool");
  } finally {
    await close();
  }
});

test("0025s3 several images in Chromium: label, views, the peek and one step (criteria 46, 51)", { skip: !BROWSER, timeout: 500_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    for (const name of ["a.png", "b.png", "c.png"]) {
      await importImage(s, name, blobs, 300, 300);
    }
    const selectRow = async (name: string, additive: boolean) => {
      await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=row]')].find((e) => (e.getAttribute('aria-label') || '').includes(${JSON.stringify(name)})).setAttribute('data-t25', '1'); })()`);
      const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-t25]').getBoundingClientRect(); return [b.x + 80, b.y + b.height / 2]; })())`)) as string) as [number, number];
      for (const type of ["mousePressed", "mouseReleased"]) {
        await s.send("Input.dispatchMouseEvent", { type, x: c[0], y: c[1], button: "left", clickCount: 1, modifiers: additive ? 2 : 0 });
      }
      await sleep(300);
    };
    await selectRow("a.png", false);
    await selectRow("b.png", true);
    await selectRow("c.png", true);
    await clickText(s, "Trace");
    await settle(s);
    assert.match(await bar(s), /Tracing 1 of 3 images/);
    await clickView(s, "Trace");
    assert.equal(await viewChecked(s), "Trace");
    const traceView = await pixels(s);
    await clickView(s, "Image");
    const imageView = await pixels(s);
    await clickView(s, "Overlay");
    const overlay = await pixels(s);
    console.log("several images, pixels overlay/trace/image:", JSON.stringify({ overlay, traceView, imageView }));
    // The selection chrome of three images is blue too (about 400 pixels); the outline adds to it only in Overlay.
    assert.ok(overlay.blue > traceView.blue + 300 && overlay.blue > imageView.blue + 300, "the outline shows in Overlay only");
    assert.ok(traceView.black > 5000 && imageView.black > 5000, "Trace and Image show black");
    await focusCanvas(s);
    await keyDownOnly(s, "\\", "Backslash", 220);
    await sleep(400);
    assert.equal(await viewChecked(s), "Image");
    await keyUpOnly(s, "\\", "Backslash", 220);
    await sleep(300);
    assert.equal(await viewChecked(s), "Overlay");
    // Stop with several, then trace again.
    await typeInto(s, "Smooth", "55");
    await settle(s);
    await createPaths(s);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 120000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
    const rows = await rowInfo(s);
    assert.equal(rows.filter(([n]) => /Trace of/.test(n)).length, 3, `three groups: ${JSON.stringify(rows)}`);
    const hist = await historyText(s);
    assert.equal(countOf(hist, "Trace image"), 1, `one history step: ${hist}`);
  } finally {
    await close();
  }
});

test("0025s3 c55 in Chromium: over 100,000 nodes the status warns, Enter creates, one group, one step (criterion 55)", { skip: !BROWSER, timeout: 900_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    // Under the limit: no warning. Since 0089 slice D a disc of radius 3 px is fitted with 8 nodes
    // (it was 4), so 800 x 800 px (100 x 100 discs) gives 80,000 nodes and 1000 x 1000 gives 125,000.
    await importImage(s, "discs-under.png", discs, 800, 800);
    await openTrace(s);
    await advancedOpen(s); // the counts, which `settle` reads, live in the Advanced block
    const under = await settle(s);
    assert.equal(nodesOf(under), 80_000);
    assert.ok(!/Slow to edit/.test(await bar(s)), `no warning at 80,000 nodes: ${under}`);
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);
    await importImage(s, "discs.png", discs, 1000, 1000);
    const rowsBefore = await s.layerRows();
    await openTrace(s);
    const status = await waitFor("the settled trace", async () => {
      const t = await barStatus(s);
      return /nodes?|0 paths/.test(t) && !/Tracing/.test(t) ? t : null;
    }, 150_000);
    console.log("status of the dense trace:", JSON.stringify(status), "| bar:", JSON.stringify(await mutedLines(s)));
    const n = nodesOf(status);
    assert.ok(n > 100_000, `the fixture has over 100,000 nodes: ${n}`);
    assert.match(status, /^[\d,]+ nodes\. Slow to edit\.$/, "the exact wording of criterion 55 (comma separators)");
    assert.ok(/\d{1,3}(,\d{3})+ nodes/.test(status), "comma separators");
    const t0 = Date.now();
    await createPaths(s);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 300_000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
    console.log("Create paths took (ms, wall):", Date.now() - t0, "|", "no notice");
    const rows = await s.layerRows();
    assert.equal(rows.length, rowsBefore.length + 1, `the Layers tab shows the result as one row: ${JSON.stringify(rows)}`);
    assert.equal(countOf(await historyText(s), "Trace image"), 1, "one step");
  } finally {
    await close();
  }
});

/** A short description of the focused element: aria-label, else text. */
const focusedName = async (s: S) =>
  (await s.evaluate(`(() => { const e = document.activeElement; if (!e || e === document.body) return 'body'; const n = (e.getAttribute('aria-label') || e.textContent || e.tagName).trim(); return e.tagName.toLowerCase() + ':' + n.slice(0, 40); })()`)) as string;

test("0025s3 c57 in Chromium: Tab order, arrows, focus rings, hit targets, roles, no hint text (criterion 57)", { skip: !BROWSER, timeout: 600_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    // The image is selected after import. Tab from the canvas in the Select tool enters at Trace.
    await focusCanvas(s);
    await s.key("v", 86).catch(() => undefined);
    await focusCanvas(s);
    await press(s, "Tab", 9);
    const first = await focusedName(s);
    console.log("first Tab stop from the canvas in the Select tool:", first);
    const selectStops = [first];
    for (let i = 0; i < 40 && !selectStops[selectStops.length - 1].endsWith(":Trace"); i++) {
      await press(s, "Tab", 9);
      selectStops.push(await focusedName(s));
    }
    console.log("Select bar Tab stops up to Trace:", JSON.stringify(selectStops));
    assert.ok(selectStops[selectStops.length - 1].endsWith(":Trace"), "the Trace button is a Tab stop of the Select bar");
    assert.equal(selectStops.length - 1, selectStops.findIndex((n) => n.endsWith(":Trace")), "Trace is reached once");
    // Press Trace (Enter on the focused button when it is the Trace button).
    await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
    await waitFor("the Trace bar", async () => ((await trace(s)) ? true : null));
    await settle(s);
    // The block is open in this file (the counts live there); the order below starts closed.
    await clickAria(s, "Fewer trace options");
    await sleep(300);
    await focusCanvas(s);
    const stops: string[] = [];
    for (let i = 0; i < 16; i++) {
      await press(s, "Tab", 9);
      const name = await focusedName(s);
      stops.push(name);
      if (name === "body") break;
    }
    console.log("Tab stops in the Trace tool, panel closed:", JSON.stringify(stops));
    const want = ["About Trace", "Preset", "Overlay", "Threshold", "Smooth", "Ignore below", "More trace options"];
    const got = stops.map((n) => n.replace(/^[a-z]+:/, "").replace(/ \d.*$/, ""));
    let at = -1;
    for (const w of want) {
      const idx = got.findIndex((g, i) => i > at && g.startsWith(w));
      assert.ok(idx > at, `${w} comes in order in ${JSON.stringify(got)}`);
      at = idx;
    }
    // The Advanced controls follow the expander when open (UX notes 20.4: Invert, Ignore
    // background, Denoise, Upscale, Corners), Corners ends the bar.
    await advancedOpen(s);
    await sleep(300);
    await s.evaluate(`document.querySelector('${barQ} button[aria-expanded][aria-controls]').focus()`);
    const inside: string[] = [];
    for (let i = 0; i < 14; i++) {
      await press(s, "Tab", 9);
      // A choice is named by its radio group ("Denoise", "Upscale").
      const group = (await s.evaluate(`document.activeElement?.closest('[role=radiogroup]')?.getAttribute('aria-label') ?? ''`)) as string;
      const name = group ? `radiogroup:${group}` : await focusedName(s);
      inside.push(name);
      if (name.includes("Corners")) break;
    }
    console.log("Tab stops from the expander (open) to Corners:", JSON.stringify(inside));
    // "Drop border shapes" since `specs/0089` Frage 88 A (customer).
    const advancedOrder = ["Invert", "Drop border shapes", "Denoise", "Upscale", "Corners"];
    let from = -1;
    for (const w of advancedOrder) {
      const idx = inside.findIndex((g, i) => i > from && g.includes(w));
      assert.ok(idx > from, `${w} comes in order in ${JSON.stringify(inside)}`);
      from = idx;
    }
    // Shift+Tab reverses.
    await press(s, "Tab", 9, 8);
    assert.notEqual(await focusedName(s), inside[inside.length - 1]);
    // Focus ring 2 px --editor-accent on keyboard focus; hit targets >= 28 px.
    const rings: string[] = [];
    await s.evaluate(`document.activeElement?.blur?.()`);
    await focusCanvas(s);
    for (let i = 0; i < 6; i++) {
      await press(s, "Tab", 9);
      rings.push((await s.evaluate(`(() => { const e = document.activeElement; const cs = getComputedStyle(e); const p = document.createElement('i'); p.style.color = 'var(--editor-accent)'; document.body.appendChild(p); const acc = getComputedStyle(p).color; p.remove(); const t = (e.getAttribute('aria-label') || e.textContent).trim().slice(0, 14); const ringOn = (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) >= 2 && cs.outlineColor === acc) || cs.boxShadow.includes(acc); return t + ':' + (ringOn ? 'ring' : 'NO-RING[' + cs.outline + '|' + cs.boxShadow.slice(0, 60) + ']'); })()`)) as string);
    }
    console.log("focus rings:", JSON.stringify(rings));
    const missing = rings.filter((r) => r.includes("NO-RING"));
    assert.deepEqual(missing, [], "every Tab stop has a 2 px --editor-accent ring");
    // Preset menu opens with Enter or Down.
    await s.evaluate(`document.querySelector('button[aria-label=Preset]').focus()`);
    await press(s, "Enter", 13);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=menu]')`), true, "Enter opens the Preset menu");
    await press(s, "Escape", 27);
    assert.equal(await s.evaluate(`document.activeElement?.getAttribute('aria-label')`), "Preset", "focus returns to the button");
    // Operating hint text: every text of the bar, titles, tooltips of the controls.
    const texts = JSON.parse(
      (await s.evaluate(`JSON.stringify({ bar: document.querySelector('${barQ}').innerText, titles: [...document.querySelectorAll('${barQ} [title]')].map((e) => e.getAttribute('title')), describedby: [...document.querySelectorAll('${barQ} [aria-describedby]')].map((e) => document.getElementById(e.getAttribute('aria-describedby'))?.textContent) })`)) as string,
    ) as { bar: string; titles: string[]; describedby: string[] };
    console.log("bar text:", JSON.stringify(texts.bar), "| titles:", JSON.stringify(texts.titles));
    // Roles: no aria-busy and no progress pill at rest (a running job is covered by the Stop test).
    assert.equal(await busyAttr(s), null);
    assert.ok(!(await hasProgress(s)));
  } finally {
    await close();
  }
});

test("0025s3 c57 hit targets in Chromium: the bar's controls are at least 28 x 28 px (criterion 57)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    await openTrace(s);
    await settle(s);
    await advancedOpen(s);
    await sleep(300);
    // A control's target is the box a click on it lands in: the input's field, the toggle's button.
    const small = JSON.parse(
      (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${barQ} button, ${barQ} [role=radio]')].filter((e) => e.offsetParent).map((e) => { const t = e; const b = t.getBoundingClientRect(); return [(e.getAttribute('aria-label') || e.textContent).trim().slice(0, 20), Math.round(b.width), Math.round(b.height)]; }).filter(([, w, h]) => w < 28 || h < 28))`)) as string,
    ) as [string, number, number][];
    console.log("targets under 28 x 28 px:", JSON.stringify(small));
    assert.deepEqual(small, [], "hit targets are at least 28 x 28 px");
  } finally {
    await close();
  }
});

test("0025s3 c13/c55 undo in Chromium: Ctrl+Z reverses a 108,900-node trace in a usable time", { skip: !BROWSER, timeout: 900_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "discs.png", discs, 1320, 1320);
    const rowsBefore = await s.layerRows();
    await openTrace(s);
    await settle(s);
    await createPaths(s);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 300000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
    assert.equal((await s.layerRows()).length, rowsBefore.length + 1);
    await focusCanvas(s);
    const t0 = Date.now();
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
    const gone = await waitFor("the group is gone", async () => ((await s.layerRows()).length === rowsBefore.length ? Date.now() - t0 : null), 120_000).catch(() => null);
    console.log("Ctrl+Z of a 27,225-outline trace took (ms, wall):", gone ?? "more than 120000");
    assert.ok(gone !== null && gone < 10_000, `Ctrl+Z took ${gone ?? ">120000"} ms`);
  } finally {
    await close();
  }
});

// ---- Re-test after the customer's changes of 2026-10-29 (criteria 4, 5, 9, 11, 12, 41, 70, 71) ----

const rectOf = async (s: S, sel: string) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom }; })())`)) as string,
  ) as { x: number; y: number; w: number; h: number; right: number; bottom: number } | null;
const tabLabels = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=tab]')].map((t) => t.getAttribute('aria-label')))`)) as string) as string[];
const activeTab = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => t.getAttribute('aria-selected') === 'true')?.getAttribute('aria-label') ?? null`)) as string | null;
const panelHidden = async (s: S) => (await s.evaluate(`!document.querySelector('[role=tabpanel]')`)) as boolean;
const dblclick = async (s: S, x: number, y: number) => {
  for (const clickCount of [1, 2]) {
    for (const type of ["mousePressed", "mouseReleased"]) {
      await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount });
    }
  }
  await sleep(400);
};

test("0025s3 re c70 in Chromium: the About Trace tab and the i button (criterion 70)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    const before = await tabLabels(s);
    assert.ok(!before.some((t) => /About Trace/.test(t)), `no About Trace tab outside the tool: ${before}`);
    const tabBefore = await activeTab(s);
    await openTrace(s);
    await settle(s);
    const labels = await tabLabels(s);
    assert.match(labels[labels.length - 1] ?? "", /^About Trace/, `About Trace is the last tab: ${labels}`);
    // 0060 snapping (main, 2026-10-30) added a Snapping tab after Fonts; criterion 70 only asks
    // that About Trace is the last tab, which the assertion above checks.
    assert.notEqual(await activeTab(s), "About Trace", "the tool never opens the tab by itself");
    assert.ok(!/About Trace/.test((await s.evaluate(`document.querySelector('[role=tabpanel]')?.innerText ?? ''`)) as string));
    // The i button: right after the Trace label, first Tab stop, 28 px, aria-pressed false.
    const info = await rectOf(s, `${barQ} button[aria-label="About Trace"]`);
    assert.ok(info && info.w >= 28 && info.h >= 28, `the i button is 28 x 28: ${JSON.stringify(info)}`);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-label="About Trace"]').getAttribute('aria-pressed')`), "false");
    const labelBox = await rectOf(s, `${barQ} span`);
    const presetBox = await rectOf(s, `${barQ} button[aria-label=Preset]`);
    assert.ok(info && presetBox && info.x < presetBox.x && Math.abs(info.y - presetBox.y) < 8, "the i button sits before Preset on row 1");
    assert.ok(labelBox === null || (info !== null && labelBox.x < info.x));
    // Press: shows the About Trace page and is pressed; the text is the spec's.
    await clickAria(s, "About Trace");
    await sleep(400);
    assert.equal(await activeTab(s), "About Trace");
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-label="About Trace"]').getAttribute('aria-pressed')`), "true");
    const page = (await s.evaluate(`document.querySelector('[role=tabpanel]')?.innerText ?? ''`)) as string;
    console.log("About Trace page:", JSON.stringify(page));
    // The page is a definition list: a term and its text; "Preset: a named set ..." reads as the pair.
    const flat = page.replace(/\n\n+/g, "\n").split("\n").map((l) => l.trim());
    const joined = flat.map((l, i) => (i > 0 && /^[a-z]/.test(l) ? `${flat[i - 1]}: ${l}` : l)).join("\n");
    assert.ok(await s.evaluate(`/About Trace/.test(document.body.innerText)`), "the header About Trace");
    for (const line of [
      // The intro was reworded by the lead on 2026-10-30 (`specs/0089` criterion 24, 0025 criterion 70).
      "Trace turns the selected image into paths. The method decides which parts become shapes. Press Enter or the Trace button to create the paths.",
      "Preset: a named set of values for what you do with the paths.",
      "Logo / Line art: sharp corners, clean curves.",
      "Photo: many shapes, photos trace roughly.",
      "Silhouette for cutting/carving: closed, simple, few nodes; drops shapes and holes below 2 mm.",
      "Engraving: keeps fine lines, drops shapes below 0.1 mm.",
    ]) {
      assert.ok(joined.includes(line), `the page has: ${line}`);
    }
    assert.equal(await s.evaluate(`!![...document.querySelectorAll('[role=tabpanel] button')].find((b) => /close/i.test(b.getAttribute('aria-label') || ''))`), false, "no close X");
    // A second press restores the stored tab and panel state.
    await clickAria(s, "About Trace");
    await sleep(400);
    assert.equal(await activeTab(s), tabBefore, "back to the stored tab");
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-label="About Trace"]').getAttribute('aria-pressed')`), "false");
    // Choosing the tab in the strip presses the button; leaving the tool returns to the stored tab.
    await s.tab("About Trace");
    await sleep(300);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-label="About Trace"]').getAttribute('aria-pressed')`), "true", "pressed also when chosen in the strip");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(400);
    assert.ok(!(await trace(s)));
    assert.ok(!(await tabLabels(s)).some((t) => /About Trace/.test(t ?? "")), "the tab leaves with the tool");
    assert.notEqual(await activeTab(s), "About Trace");
    // The empty bar has the tab too.
    await s.key("Escape", 27);
    await s.evaluate(`document.activeElement?.blur?.()`);
    await focusCanvas(s);
    await s.key("i", 73);
    await sleep(500);
    assert.ok((await tabLabels(s)).some((t) => /About Trace/.test(t ?? "")), "the tab exists in the Trace tool with the image selected or not");
  } finally {
    await close();
  }
});

test("0025s3 re c4/c71/c41/c11 in Chromium: row 1, the far-right expander, counts only in Advanced, icon toggles (criteria 4, 11, 41, 71)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    await openTrace(s);
    await advancedOpen(s); // the counts, which `settle` reads, live in the Advanced block
    await settle(s);
    // Start closed whatever an earlier run stored.
    if (await s.evaluate(`document.querySelector('${barQ} button[aria-label="Fewer trace options"]') !== null`)) {
      await clickAria(s, "Fewer trace options");
      await sleep(300);
    }
    // Criterion 4 as 0089 criteria 22 and 22a reshape it: the finish button is the blue "Trace" (no
    // "Create paths" button, no state label), and the flow is i, Preset, View, Method, Threshold,
    // Smooth, Tolerance, Ignore below, the expander, the finish button; no Advanced word.
    assert.ok(!(await hasButton(s, "Create paths")), "there is no Create paths button");
    await atRest(s, "with Advanced closed");
    const names = ["button[aria-label=\"About Trace\"]", "button[aria-label=Preset]", "[role=radiogroup][aria-label=View]", "button[aria-label=Method]", "[role=spinbutton][aria-label=Threshold]", "[role=spinbutton][aria-label=Smooth]", "[role=spinbutton][aria-label=Tolerance]", "[role=spinbutton][aria-label=\"Ignore below\"]", "button[aria-expanded][aria-controls]", "[data-trace-slot]"];
    const boxes = [];
    for (const n of names) boxes.push((await rectOf(s, `${barQ} ${n}`)) ?? assert.fail(`missing ${n}`));
    const flow = (a: (typeof boxes)[0], b: (typeof boxes)[0]) => b.y > a.bottom - 4 || b.x >= a.right - 1;
    for (let i = 1; i < boxes.length; i++) assert.ok(flow(boxes[i - 1], boxes[i]), `${names[i]} follows ${names[i - 1]} in reading order`);
    assert.ok(!/Advanced/.test(await bar(s)), "the word Advanced is not drawn");
    const ex = boxes[8];
    const finish = boxes[9];
    const barBox = (await rectOf(s, barQ))!;
    assert.ok(ex.w >= 28 && ex.h >= 28, "the expander is 28 x 28");
    assert.ok(barBox.right - finish.right < 16, `the Trace button is at the far right: gap ${barBox.right - finish.right}`);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-expanded][aria-controls]').getAttribute('aria-label')`), "More trace options");
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-controls]').getAttribute('aria-expanded')`), "false");
    // Criterion 11: with Advanced closed no counts line.
    // Drawn text only: the visually hidden live region announces the counts on purpose (criterion 71, amended 2026-10-31).
    const drawn = (await s.evaluate(`(() => { const c = document.querySelector('${barQ}').cloneNode(true); c.querySelectorAll('[data-trace-hidden-status]').forEach((e) => e.remove()); document.body.appendChild(c); const t = c.innerText; c.remove(); return t; })()`)) as string;
    assert.ok(!/\d+ paths?, [\d,]+ nodes?/.test(drawn), `no counts line while closed: ${JSON.stringify(drawn)}`);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} [data-trace-counts]') === null || document.querySelector('${barQ} [data-trace-counts]').offsetParent === null`), true);
    // Open: the toggles, the controls, the counts as the last row.
    await clickAria(s, "More trace options");
    await sleep(400);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-controls]').getAttribute('aria-label')`), "Fewer trace options");
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-controls]').getAttribute('aria-expanded')`), "true");
    const controls = await s.evaluate(`document.querySelector('${barQ} button[aria-controls]').getAttribute('aria-controls')`);
    assert.ok(await s.evaluate(`!!document.getElementById(${JSON.stringify(controls)})`), "aria-controls names the block");
    const ex2 = (await rectOf(s, `${barQ} button[aria-expanded][aria-controls]`))!;
    assert.ok(Math.abs(ex2.y - ex.y) < 2 && Math.abs(ex2.right - ex.right) < 2, "the expander does not move when the block opens");
    // "Ignore background" is named "Drop border shapes" since `specs/0089` Frage 88 A (customer).
    for (const label of ["Invert", "Drop border shapes"]) {
      const pressed = await s.evaluate(`document.querySelector('${barQ} button[aria-label="${label}"]')?.getAttribute('aria-pressed')`);
      assert.equal(pressed, "false", `${label} is an icon toggle with aria-pressed`);
      const box = (await rectOf(s, `${barQ} button[aria-label="${label}"]`))!;
      assert.ok(box.w >= 28 && box.h >= 28, `${label} is 28 x 28`);
      assert.ok(!(await bar(s)).split("\n").some((l) => l.trim() === label), `${label} is not drawn as text`);
    }
    for (const gone of ["Full-size preview", "Node limit"]) {
      assert.ok(!(await bar(s)).includes(gone) && (await s.evaluate(`document.querySelector('${barQ} [aria-label="${gone}"]') === null`)), `${gone} is removed`);
    }
    const toggleBox = (await rectOf(s, `${barQ} button[aria-label=Invert]`))!;
    const denoise = (await rectOf(s, `${barQ} [role=radiogroup][aria-label=Denoise]`))!;
    assert.ok(toggleBox.y <= denoise.y + 4, "the toggles start Advanced row 1 with Denoise");
    const counts = (await bar(s)).split("\n").map((l) => l.trim()).filter(Boolean);
    const countsLine = counts.findIndex((l) => /^\d+ paths?, [\d,]+ nodes?, [\d.]+ s$/.test(l));
    assert.ok(countsLine >= 0, `the counts line is in Advanced: ${JSON.stringify(counts)}`);
    const countsBox = (await rectOf(s, `${barQ} [data-trace-counts]`))!;
    const corners = (await rectOf(s, `${barQ} [role=spinbutton][aria-label=Corners]`))!;
    // Amended 2026-10-31 (criterion 71, `0089` ux-notes §22.3): the status line is the second row, after the analysis row and above every control.
    const analysisBox = (await rectOf(s, `${barQ} [data-trace-analysis]`))!;
    assert.ok(countsBox.y >= analysisBox.bottom - 1 && countsBox.bottom <= Math.min(corners.y, denoise.y, toggleBox.y) + 1, "the status line is the second row of the block, above the controls");
    // A toggle works: Invert changes the result and the dot/tooltip on the closed expander.
    const rings = await settle(s);
    await clickAria(s, "Invert");
    await sleep(900);
    assert.equal(await s.evaluate(`document.querySelector('${barQ} button[aria-label=Invert]').getAttribute('aria-pressed')`), "true");
    await clickAria(s, "Fewer trace options");
    await sleep(300);
    const exBtn = (await s.evaluate(`document.querySelector('${barQ} button[aria-controls]').outerHTML`)) as string;
    console.log("closed expander html:", exBtn.slice(0, 900));
    let tip = await tooltipOf(s, "button[aria-controls]");
    if (tip === "") {
      await sleep(1000);
      tip = await tooltipOf(s, "button[aria-controls]");
    }
    console.log("closed expander tooltip with Invert on:", JSON.stringify(tip), "| before:", rings);
    assert.match(tip, /Changed: .*Invert/i, "the closed expander's tooltip lists the changed value");
    // Criterion 71: states in the slot and the notice row stay visible with Advanced closed.
    const drawnAgain = (await s.evaluate(`(() => { const c = document.querySelector('${barQ}').cloneNode(true); c.querySelectorAll('[data-trace-hidden-status]').forEach((e) => e.remove()); document.body.appendChild(c); const t = c.innerText; c.remove(); return t; })()`)) as string;
    assert.ok(!/\d+ paths?, [\d,]+ nodes?/.test(drawnAgain), "drawn text only: the hidden live region announces the counts (criterion 71, amended 2026-10-31)");
  } finally {
    await close();
  }
});

test("0025s3 re c5/c9 in Chromium: value fields (click, drag, double-click entry, keys, clamp) and Threshold in percent (criteria 5, 9)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    // Luma 101 left, 200 right: 40 % is luma 102 (the left half is dark), 39 % is luma 99 (it is light).
    await importImage(s, "halves.png", (x) => (x < 100 ? 101 : 200), 200, 200);
    await openTrace(s);
    await settle(s);
    const shown = await fieldValue(s, "Threshold");
    const text = (await s.evaluate(`document.querySelector('${field("Threshold")}').getAttribute('aria-valuetext')`)) as string;
    console.log("Threshold aria-valuetext:", JSON.stringify(text), "min/max/now:", await s.evaluate(`['aria-valuemin','aria-valuemax','aria-valuenow'].map((a) => document.querySelector('${field("Threshold")}').getAttribute(a)).join('/')`));
    const visible = (await s.evaluate(`document.querySelector('${field("Threshold")}').closest('div')?.innerText ?? ''`)) as string;
    console.log("Threshold visible text:", JSON.stringify(visible));
    assert.ok(/%|percent/.test(text) && /%/.test(visible), `the Threshold is shown as percent: ${text} / ${visible}`);
    const pct = Number(shown);
    assert.ok(pct >= 16 && pct <= 78, `the automatic value of a 101/200 picture is between 16 and 78 %: ${pct}`);
    // Typing 40 % dark-ens the left half, 39 % does not (luma 102 against 99).
    const tracedAt = async (v: string) => {
      await typeInto(s, "Threshold", v);
      await sleep(900);
      return (await bar(s)).includes("0 paths") ? 0 : 1;
    };
    // Counts live in Advanced; read the shown status through the notice row or the counts.
    await advancedOpen(s);
    const pathsAt = async (v: string) => {
      await typeInto(s, "Threshold", v);
      await sleep(1000);
      const t = await bar(s);
      const m = /(\d+) paths?, /.exec(t);
      return m ? Number(m[1]) : /0 paths/.test(t) ? 0 : -1;
    };
    void tracedAt;
    assert.ok((await pathsAt("40")) >= 1, "40 % (luma 102) takes the luma-101 half");
    assert.equal(await pathsAt("39"), 0, "39 % (luma 99) does not");
    // Clamp silently, Enter returns focus to the canvas.
    await typeInto(s, "Threshold", "150");
    assert.equal(await fieldValue(s, "Threshold"), "100");
    await typeInto(s, "Threshold", "-5");
    assert.equal(await fieldValue(s, "Threshold"), "0");
    assert.ok(await s.evaluate(`!document.activeElement || !document.querySelector('${barQ}')?.contains(document.activeElement) || document.activeElement.tagName === 'CANVAS'`), "Enter in a field returns focus to the canvas");
    // Keys on the focused field: Up/Down 1, Shift 10, Home/End.
    await typeInto(s, "Smooth", "50");
    await s.evaluate(`document.querySelector('${field("Smooth")}').focus()`);
    await press(s, "ArrowUp", 38);
    assert.equal(await fieldValue(s, "Smooth"), "51");
    await press(s, "ArrowDown", 40);
    await press(s, "ArrowDown", 40);
    assert.equal(await fieldValue(s, "Smooth"), "49");
    await press(s, "ArrowUp", 38, 8);
    assert.equal(await fieldValue(s, "Smooth"), "59", "Shift+Up is 10");
    await press(s, "End", 35);
    assert.equal(await fieldValue(s, "Smooth"), "100");
    await press(s, "Home", 36);
    assert.equal(await fieldValue(s, "Smooth"), "0");
    // Single click changes the value; a drag changes it; a double-click opens number entry.
    const box = (await rectOf(s, field("Smooth")))!;
    await typeInto(s, "Smooth", "50");
    await s.click(box.x + box.w * 0.9, box.y + box.h / 2);
    const afterClick = Number(await fieldValue(s, "Smooth"));
    console.log("Smooth after a click at 90 % of the field:", afterClick);
    assert.notEqual(afterClick, 50, "a single click changes the value");
    await typeInto(s, "Smooth", "50");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box.x + box.w * 0.5, y: box.y + box.h / 2 });
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: box.x + box.w * 0.5, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    for (const f of [0.6, 0.7, 0.8]) {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box.x + box.w * f, y: box.y + box.h / 2, button: "left" });
    }
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: box.x + box.w * 0.8, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    await sleep(400);
    const afterDrag = Number(await fieldValue(s, "Smooth"));
    console.log("Smooth after a drag to 80 %:", afterDrag);
    assert.ok(afterDrag > 50, "a drag to the right raises the value");
    await dblclick(s, box.x + box.w / 2, box.y + box.h / 2);
    assert.ok(await s.evaluate(`document.activeElement?.tagName === 'INPUT' && ${JSON.stringify(barQ)} !== ''`), "a double-click opens number entry");
    await s.send("Input.insertText", { text: "33" });
    await press(s, "Enter", 13);
    assert.equal(await fieldValue(s, "Smooth"), "33");
    // Changing a value is not a history step.
    assert.ok(!/Trace image/.test(await historyText(s)));
  } finally {
    await close();
  }
});

test("0025s3 re c12 in Chromium: Enter, Ctrl+Enter in a field and a double-click create; Enter on a bar button presses it; the hidden description (criterion 12)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    const rowsBefore = (await s.layerRows()).length;
    await openTrace(s);
    await settle(s);
    const tb = await s.evaluate(`(() => { const t = document.querySelector('${barQ}'); const d = t.getAttribute('aria-describedby'); return JSON.stringify({ role: t.getAttribute('role'), name: t.getAttribute('aria-label'), desc: d ? document.getElementById(d)?.textContent : null }); })()`);
    assert.deepEqual(JSON.parse(tb as string), { role: "toolbar", name: "Trace", desc: "Enter creates the paths" });
    // Ctrl+Enter from inside a field creates.
    await s.evaluate(`document.querySelector('${field("Smooth")}').focus()`);
    await press(s, "Enter", 13, 2);
    await waitFor("the group", async () => ((await s.layerRows()).length === rowsBefore + 1 ? true : null), 20_000);
    assert.ok(!(await trace(s)), "back in the Select tool");
    // Enter on a focused bar button presses that button, it does not create.
    await s.key("z", 90, 2).catch(() => undefined);
    await sleep(300);
    await s.evaluate(`document.activeElement?.blur?.()`);
    // Enter in a field only commits (criterion 5).
    const base = (await s.layerRows()).length;
    await openTrace(s).catch(() => undefined);
    if (!(await trace(s))) {
      await clickRow(s, "ring.png");
      await openTrace(s);
    }
    await settle(s);
    await typeInto(s, "Smooth", "70");
    assert.equal((await s.layerRows()).length, base, "Enter in a field commits the value, it does not create");
    await s.evaluate(`document.querySelector('${barQ} button[aria-label=Preset]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
    await sleep(500);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=menu]')`), true, "Enter on the Preset button opens its menu");
    assert.equal((await s.layerRows()).length, base, "and creates nothing");
    await press(s, "Escape", 27);
    // The next Enter on the canvas creates; double-click on the image creates too.
    await focusCanvas(s);
    await s.key("Enter", 13);
    await waitFor("the second group", async () => ((await s.layerRows()).length === base + 1 ? true : null), 20_000);
  } finally {
    await close();
  }
});

const clickRow = async (s: S, name: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=row]')].find((e) => (e.getAttribute('aria-label') || '').includes(${JSON.stringify(name)}) && !/Trace of/.test(e.getAttribute('aria-label') || '')).setAttribute('data-t25', '1'); })()`);
  const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-t25]').getBoundingClientRect(); return [b.x + 80, b.y + b.height / 2]; })())`)) as string) as [number, number];
  await s.click(c[0], c[1]);
};

const PHOTO = path.resolve(FRONTEND, "..", "target", "photo", "iceberg.jpg");

test("0025s3 re photo in Chromium: a 3747 x 3178 px photo (Wikimedia Commons, public domain), Photo preset, every view, Enter, undo, repeated drags (timings)", { skip: !BROWSER || !existsSync(PHOTO), timeout: 1_500_000 }, async () => {
  const { s, close } = await launch(WORKER_FAULT);
  const times: [string, number][] = [];
  const t = async <T>(step: string, work: () => Promise<T>): Promise<T> => {
    const t0 = Date.now();
    const out = await work();
    times.push([step, Date.now() - t0]);
    console.log(`TIMING ${step}: ${Date.now() - t0} ms`);
    return out;
  };
  const frames = (n = 2) => s.evaluate(`new Promise((r) => { let i = 0; const f = () => (++i >= ${n} ? r(performance.now()) : requestAnimationFrame(f)); requestAnimationFrame(f); })`);
  const longtasks = async () => JSON.parse((await s.evaluate(`JSON.stringify(window.__lt.splice(0))`)) as string) as number[];
  try {
    await s.tab("Layers");
    await s.evaluate(`window.__lt = []; new PerformanceObserver((l) => l.getEntries().forEach((e) => window.__lt.push(Math.round(e.duration)))).observe({ entryTypes: ['longtask'] })`);
    const rows = (await s.layerRows()).length;
    await t("import the JPEG (until its Layers row)", async () => {
      await s.choose("[data-import-button]", PHOTO);
      await waitFor("the image row", async () => ((await s.layerRows()).length > rows ? true : null), 120_000);
    });
    console.log("longtasks (ms) during import:", JSON.stringify(await longtasks()));
    await focusCanvas(s);
    await t("key I until the first preview settled (reduced copy)", async () => {
      await s.key("i", 73);
      await settle(s);
    });
    console.log("longtasks during first preview:", JSON.stringify(await longtasks()), "jobs:", JSON.stringify(await s.evaluate(`JSON.stringify(window.__jobs.map((j) => [j.w + 'x' + j.h, Math.round((j.end ?? 0) - j.start)]))`)));
    await t("pick the Photo preset until settled", async () => {
      await s.evaluate(`document.querySelector('${barQ} button[aria-label=Preset]').click()`);
      await waitFor("the menu", async () => ((await s.evaluate(`!!document.querySelector('[role=menu]')`)) ? true : null), 10_000);
      await s.evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitemradio], [role=menu] [role=menuitem]')].find((i) => i.innerText.startsWith('Photo')).click()`);
      await sleep(300);
      await settle(s);
    });
    console.log("preset:", await s.evaluate(`document.querySelector('${barQ} button[aria-label=Preset]').textContent.trim()`), "| notice:", JSON.stringify(await barStatus(s)), "| longtasks:", JSON.stringify(await longtasks()));
    await advancedOpen(s);
    console.log("counts of the reduced preview:", JSON.stringify(await barStatus(s)));
    for (const view of ["Trace", "Image", "Overlay"]) {
      await t(`View ${view}: click until two frames`, async () => {
        await clickView(s, view);
        await frames();
      });
      console.log(`  longtasks in ${view}:`, JSON.stringify(await longtasks()));
    }
    await t("peek (hold backslash) until two frames", async () => {
      await focusCanvas(s);
      await keyDownOnly(s, "\\", "Backslash", 220);
      await frames();
    });
    await keyUpOnly(s, "\\", "Backslash", 220);
    await frames();
    await longtasks();
    await t("zoom with the wheel until two frames", async () => {
      const [cx, cy, cw, ch] = await canvasRect(s);
      await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: cx + cw * 0.6, y: cy + ch * 0.5, deltaX: 0, deltaY: -300, modifiers: 2 });
      await frames();
      await sleep(400);
      await frames();
    });
    console.log("  longtasks in the zoom:", JSON.stringify(await longtasks()));
    // Repeated value drags: 60 moves 16 ms apart on Smooth, then wait for the last job; count the jobs.
    const jobsBefore = (await s.evaluate(`window.__jobs.length`)) as number;
    const box = (await rectOf(s, field("Smooth")))!;
    const dragT0 = Date.now();
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: box.x + box.w * 0.2, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    for (let i = 0; i < 60; i++) {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box.x + box.w * (0.2 + 0.6 * ((i % 30) / 30)), y: box.y + box.h / 2, button: "left" });
      await sleep(16);
    }
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: box.x + box.w * 0.5, y: box.y + box.h / 2, button: "left", clickCount: 1 });
    const dragMs = Date.now() - dragT0;
    await settle(s);
    const jobs = JSON.parse((await s.evaluate(`JSON.stringify(window.__jobs.slice(${jobsBefore}).map((j) => [j.w + 'x' + j.h, Math.round((j.end ?? -1) - j.start)]))`)) as string) as [string, number][];
    times.push(["60-move drag then settle", Date.now() - dragT0]);
    console.log(`TIMING drag of ${dragMs} ms, settled ${Date.now() - dragT0} ms after the press; ${jobs.length} jobs started:`, JSON.stringify(jobs));
    console.log("  longtasks in the drag:", JSON.stringify(await longtasks()));
    // The same with keyboard auto-repeat on Threshold: 40 key presses 30 ms apart.
    const jobs2 = (await s.evaluate(`window.__jobs.length`)) as number;
    const k0 = Date.now();
    await s.evaluate(`document.querySelector('${field("Threshold")}').focus()`);
    for (let i = 0; i < 40; i++) {
      await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "ArrowUp", code: "ArrowUp", windowsVirtualKeyCode: 38, autoRepeat: i > 0 });
      await sleep(30);
    }
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "ArrowUp", code: "ArrowUp", windowsVirtualKeyCode: 38 });
    await settle(s);
    const jobsKey = JSON.parse((await s.evaluate(`JSON.stringify(window.__jobs.slice(${jobs2}).map((j) => [j.w + 'x' + j.h, Math.round((j.end ?? -1) - j.start)]))`)) as string) as [string, number][];
    console.log(`TIMING 40 held-key steps, settled ${Date.now() - k0} ms; ${jobsKey.length} jobs:`, JSON.stringify(jobsKey));
    // Enter: create at full size.
    const before = (await s.layerRows()).length;
    const fullJobs = (await s.evaluate(`window.__jobs.length`)) as number;
    await t("Enter until the group row", async () => {
      await createPaths(s);
      await waitFor("the group", async () => ((await s.layerRows()).length === before + 1 ? true : null), 600_000);
    });
    console.log("jobs of Enter:", JSON.stringify(await s.evaluate(`JSON.stringify(window.__jobs.slice(${fullJobs}).map((j) => [j.w + 'x' + j.h, Math.round((j.end ?? -1) - j.start)]))`)), "| notice:", JSON.stringify(await s.statusText()));
    console.log("  longtasks in Enter:", JSON.stringify(await longtasks()));
    await t("first two frames with the group", async () => {
      await frames();
    });
    await focusCanvas(s);
    await t("Ctrl+Z until the group row is gone", async () => {
      await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
      await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
      await waitFor("undone", async () => ((await s.layerRows()).length === before ? true : null), 600_000);
    });
    console.log("  longtasks in undo:", JSON.stringify(await longtasks()));
    // A dense result: Threshold 16 % (luma 41) makes 130,000 nodes of the full-size trace.
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    await t("Threshold 16 % until the reduced preview settled", async () => {
      await typeInto(s, "Threshold", "16");
      await settle(s);
    });
    console.log("  dense reduced preview:", JSON.stringify(await barStatus(s)), "| notice:", JSON.stringify(await mutedLines(s)));
    const dense = (await s.layerRows()).length;
    await t("dense Enter until the group row", async () => {
      await createPaths(s);
      await waitFor("the group", async () => ((await s.layerRows()).length === dense + 1 ? true : null), 600_000);
    });
    console.log("  notice:", JSON.stringify((await s.statusText()).replace(/[ |]+/g, " ")));
    await t("dense first two frames", async () => {
      await frames();
    });
    await focusCanvas(s);
    await t("dense Ctrl+Z until the group row is gone", async () => {
      await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
      await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "z", code: "KeyZ", windowsVirtualKeyCode: 90, modifiers: 2 });
      await waitFor("undone", async () => ((await s.layerRows()).length === dense ? true : null), 600_000);
    });
    console.log("TIMINGS", JSON.stringify(times));
  } finally {
    await close();
  }
});

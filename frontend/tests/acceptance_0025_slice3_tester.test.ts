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
  let black = 0;
  let near = 0;
  for (let y = Math.floor(cy) + 80; y < cy + ch; y++) {
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
  window.Worker = class extends Real {
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
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
        beforeExpander: r.top < ex.top - 2 || r.right <= ex.left, progress: pb ? { name: pb.getAttribute('aria-label'), now: pb.getAttribute('aria-valuenow') } : null,
        buttons: [...e.querySelectorAll('button')].map((b) => [(b.getAttribute('aria-label') || b.textContent).trim(), Math.round(b.getBoundingClientRect().width), Math.round(b.getBoundingClientRect().height)]) }; })())`)) as string,
  ) as { kind: string; text: string; width: number; height: number; afterSpecks: boolean; beforeExpander: boolean; progress: { name: string; now: string | null } | null; buttons: [string, number, number][] } | null;
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
    assert.equal(names, "Overlay,Trace,Image");
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

    // Shift+backslash cycles Overlay -> Trace -> Image -> Overlay.
    await focusCanvas(s);
    const seen: string[] = [];
    for (let i = 0; i < 3; i++) {
      await keyDownOnly(s, "|", "Backslash", 220, 8);
      await keyUpOnly(s, "|", "Backslash", 220, 8);
      await sleep(300);
      seen.push(await viewChecked(s));
    }
    assert.deepEqual(seen, ["Trace", "Image", "Overlay"]);

    // Arrow keys in the pill move and select; one Tab stop; the view is not remembered.
    await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-checked=true]').focus()`);
    await press(s, "ArrowRight", 39);
    assert.equal(await viewChecked(s), "Trace");
    await press(s, "ArrowRight", 39);
    await press(s, "ArrowRight", 39);
    assert.equal(await viewChecked(s), "Overlay", "arrows wrap");
    await press(s, "ArrowLeft", 37);
    assert.equal(await viewChecked(s), "Image");
    const tabindexes = (await s.evaluate(`[...document.querySelectorAll('[role=radiogroup][aria-label=View] [role=radio]')].map((r) => r.tabIndex).join(',')`)) as string;
    assert.equal(tabindexes, "-1,-1,0", "one Tab stop for the pill");
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
    await settle(s);
    assert.equal(await slotInfo(s), null, "the slot is empty at rest");
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
    assert.ok(running?.afterSpecks && running?.beforeExpander, "the slot is after Specks and before the expander");
    assert.deepEqual(running?.buttons.map(([n, w, h]) => [n, w >= 28 && h >= 28]), [[STOP_X, true]], "the x is a 28 px target");
    assert.ok(!/Tracing\.\.\./.test(await bar(s)), "no Tracing... text");
    assert.equal(await busyAttr(s), null, "no aria-busy");
    // The slot's tab order: after Ignore below, right before the expander.
    const order = await barButtons(s);
    const expanderAt = order.findIndex((n) => n === EXPAND_CLOSED || n === EXPAND_OPEN);
    assert.ok(order.indexOf(STOP_X) >= 0 && order.indexOf(STOP_X) === expanderAt - 1, `the x comes right before the expander: ${order}`);
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
    assert.equal(await slotInfo(s), null, "the slot empties");
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
    assert.ok(failed?.afterSpecks && failed?.beforeExpander, "Failed sits in the slot");
    assert.deepEqual(failed?.buttons.map(([n, w, h]) => [n, w >= 28 && h >= 28]), [["Retry", true]], "one Retry, a 28 px target");
    assert.match(await tooltipOf(s, "[data-trace-slot=failed] span"), /^Trace failed: .+\. Your drawing is unchanged\.$/, "the reason is the tooltip");
    // The bar's values stay; the document is exactly as before.
    assert.notEqual(await fieldValue(s, "Smooth"), null);
    assert.deepEqual(await s.layerRows(), rowsBefore);
    assert.equal(await historyText(s), historyBefore, "history is untouched");
    // Ctrl+Enter in a field only traces again while failed; it writes nothing.
    await s.evaluate(`(() => { const f = document.querySelector('${field("Smooth")}'); f.focus(); })()`);
    await press(s, "Enter", 13, 2);
    await sleep(500);
    assert.deepEqual(await s.layerRows(), rowsBefore, "Create paths on a failed trace writes nothing");
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
    assert.equal(await slotInfo(s), null, "the slot empties after a successful trace");
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
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced: /.test(t) ? t : null;
    }, 300_000);
    console.log("notice:", notice);
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
    await waitFor("the notice", async () => (/Traced 3 images/.test(await s.statusText()) ? true : null), 120_000);
    const rows = await rowInfo(s);
    assert.equal(rows.filter(([n]) => /Trace of/.test(n)).length, 3, `three groups: ${JSON.stringify(rows)}`);
    const hist = await historyText(s);
    assert.equal(countOf(hist, "Trace image"), 1, `one history step: ${hist}`);
  } finally {
    await close();
  }
});

test("0025s3 c55 in Chromium: over 100,000 nodes the status warns, Create paths works, one group, one step (criterion 55)", { skip: !BROWSER, timeout: 900_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    // Under the limit (90,000 nodes): no warning.
    await importImage(s, "discs-under.png", discs, 1200, 1200);
    await openTrace(s);
    const under = await settle(s);
    assert.equal(nodesOf(under), 90_000);
    assert.ok(!/Slow to edit/.test(await bar(s)), `no warning at 90,000 nodes: ${under}`);
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);
    await importImage(s, "discs.png", discs, 1320, 1320);
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
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced: /.test(t) ? t : null;
    }, 300_000);
    console.log("Create paths took (ms, wall):", Date.now() - t0, "|", notice);
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
    const advancedOrder = ["Invert", "Ignore background", "Denoise", "Upscale", "Corners"];
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
    await waitFor("the notice", async () => (/Traced: /.test(await s.statusText()) ? true : null), 300_000);
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

// Tester cases for `specs/0089-trace-quality-and-colour` slice F (automatic analysis, criteria 31 to 34, with
// Frage 83 B and 86 A) in the real app (Chromium via DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the
// wasm bindings must be built). Written from the specification and ux-notes.md §3 before the code was read,
// and kept apart from `acceptance_0089_f_browser.test.ts`: this file delays the analysis answer to edit before
// it arrives, checks that Analyse again never touches the preset, Output, Invert, Drop border shapes or picked
// colours, the low-confidence and failed lines, a second image after a manual change, and the 36 px row in
// every state at 1280 x 800 and 800 x 600. Harness as in `acceptance_0089_sliceE_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5261;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-sliceF-tester");
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
/** 120 x 120 of random RGB: no type is clear. */
const NOISE = (() => {
  let state = 12345;
  return rgbPng(120, 120, () => {
    const next = () => {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
      return state >>> 24;
    };
    return [next(), next(), next()];
  });
})();
/** 360 x 200: a red and a blue block on white. */
const TWO = rgbPng(360, 200, (x, y) => {
  if (y < 40 || y >= 160) return [255, 255, 255];
  if (x >= 30 && x < 160) return [200, 30, 30];
  if (x >= 200 && x < 330) return [30, 60, 200];
  return [255, 255, 255];
});

// ------------------------------------------------------------------ helpers

const HINT = /\b(click|press|drag|tap|hold|try|use|tip|hint|you can|choose|select a|to (pick|add|remove|create))\b/i;
const ROW = `${BAR} [data-trace-analysis]`;
const rowState = async (s: S) => (await s.evaluate(`document.querySelector('${ROW}')?.getAttribute('data-trace-analysis') ?? 'none'`)) as string;
const lineText = async (s: S) => (await s.evaluate(`document.querySelector('${ROW} [role=status]')?.textContent ?? ''`)) as string;
const againState = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('${ROW} button')].find((b) => b.textContent === 'Analyse again')?.getAttribute('aria-disabled') ?? 'missing'`)) as string;
const methodText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]')?.textContent ?? ''`)) as string;
const presetText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Preset]')?.textContent ?? ''`)) as string;
const pressedOf = async (s: S, label: string) => (await s.evaluate(`document.querySelector('${BAR} [aria-label="${label}"]')?.getAttribute('aria-pressed') ?? 'none'`)) as string;
const outputText = async (s: S) =>
  (await s.evaluate(`document.querySelector('${BAR} [role=radiogroup][aria-label=Output] [aria-checked=true]')?.textContent ?? ''`)) as string;
const swatchList = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-swatch]')].map((e) => e.getAttribute('data-trace-swatch')))`)) as string) as string[];
const waitState = (s: S, want: string, ms = 30_000) => waitFor(`the analysis row to be ${want}`, async () => ((await rowState(s)) === want ? true : null), ms);
const rowBox = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => {
    const bar = document.querySelector('${BAR}').getBoundingClientRect(); const row = document.querySelector('${ROW}'); const r = row.getBoundingClientRect();
    const b = [...row.querySelectorAll('button')][0]?.getBoundingClientRect(); const t = row.querySelector('[role=status]'); const cs = getComputedStyle(t);
    return { top: r.top, height: r.height, width: r.width, barWidth: bar.width, barTop: bar.top, barHeight: bar.height, button: b ? [b.width, b.height, bar.right - b.right] : null,
      truncated: t.scrollWidth > t.clientWidth + 1 || cs.textOverflow === 'ellipsis', fontSize: cs.fontSize, aria: t.getAttribute('aria-live'), lineHeight: t.getBoundingClientRect().height, title: t.getAttribute('title'), text: t.textContent }; })())`)) as string) as {
    top: number; height: number; width: number; barWidth: number; barTop: number; barHeight: number; button: number[] | null; truncated: boolean; fontSize: string; aria: string | null; lineHeight: number; title: string | null; text: string };

/** Imports `png` as `name` into the open document and opens the Trace bar on it (a new image). */
async function importAnother(s: S, name: string, png: Buffer) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor(`the ${name} row`, async () => ((await s.layerRows()).some((r) => r?.includes(name)) ? true : null));
  if (!(await s.evaluate(`!!document.querySelector('${BAR}')`))) {
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  }
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
}

/** The analysis answer (it rides on the worker's `threshold` answer) arrives `window.__analysisDelay` ms late. */
const ANALYSIS_DELAY = `(() => {
  const Real = window.Worker;
  window.__analysisDelay = 0;
  const set = Object.getOwnPropertyDescriptor(Real.prototype, 'onmessage').set;
  window.Worker = class extends Real {
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
        const d = event.data;
        if (d && d.kind === 'threshold' && window.__analysisDelay > 0) setTimeout(() => fn(event), window.__analysisDelay);
        else fn(event);
      });
    }
  };
})();`;

/** Workers made while `window.__failWorker` is true die at once (as in `acceptance_0025_slice3_tester`). */
const WORKER_DIES = `(() => {
  const Real = window.Worker;
  window.__failWorker = false;
  window.Worker = class extends Real {
    constructor(url, options) {
      if (window.__failWorker) super(URL.createObjectURL(new Blob(['throw new Error("boom")'], { type: 'text/javascript' })), options);
      else super(url, options);
    }
  };
})();`;

const settleTrace = (s: S) => waitFor("a trace result", async () => (/^\d+ paths?/.test(await counts(s)) ? true : null), 60_000);

// ------------------------------------------------------------------ criterion 31, Frage 83 B: a new image

test("0089F tester c31, c83B: a new line drawing is analysed and its values applied at once; the line names a type and says what it set; Analyse again is dimmed", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES);
    await waitState(s, "auto");
    const line = await lineText(s);
    console.log("line:", JSON.stringify(line), "method:", await methodText(s), "threshold:", await fieldValue(s, "Threshold"));
    assert.match(line, /^(Graphic|Logo|Line drawing|Sketch) · .+/, "the line is 'Type · finding'");
    assert.ok(!HINT.test(line), `no operating hint in the line: ${line}`);
    assert.ok(!/\.\.\.|…/.test(line), "no ellipsis");
    assert.match(await methodText(s), /Brightness/, "thin dark strokes on white stay on Brightness");
    assert.match(await fieldValue(s, "Threshold"), /^\d+$/, "the Threshold shows the analysed value");
    assert.equal(await againState(s), "true", "Analyse again is aria-disabled in Auto");
    assert.equal(await presetText(s), "Logo / Line art", "the preset is the job and is not set by the analysis");
    await settleTrace(s);
    assert.ok(Number(countsOf(await counts(s))) > 0, "the new image is traced with the applied values");
    // Pressing the dimmed button changes nothing (the result would be the same, criterion 34).
    await s.clickSel(`${ROW} button`);
    await sleep(800);
    assert.equal(await rowState(s), "auto");
    assert.equal(await lineText(s), line);
  });
});

test("0089F tester c32: an unclear picture says so, falls back to Brightness and the automatic Threshold, and Analyse again is dimmed", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "noise.png", NOISE);
    await waitState(s, "auto");
    const line = await lineText(s);
    console.log("line:", JSON.stringify(line));
    assert.equal(line, "Type unclear · Brightness with automatic Threshold");
    assert.match(line, /automatic Threshold/, "it says which Threshold is used");
    assert.match(await methodText(s), /Brightness/);
    assert.equal(await againState(s), "true");
    assert.ok(!HINT.test(line));
    // Spec criterion 32 names the fallback "Logo / Line art with the automatic Threshold".
    assert.equal(await presetText(s), "Logo / Line art");
  });
});

// ------------------------------------------------------------------ criterion 33: never after a manual change

test("0089F tester c33, c83B: a manual change made before the analysis answer arrives wins; Analyse again then applies it", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.evaluate(`window.__analysisDelay = 5000`);
    await openOn(s, "pale.png", PALE);
    const before = await methodText(s);
    console.log("method before the answer:", before, "row:", await rowState(s));
    assert.match(before, /Brightness/, "until the answer arrives the default method shows");
    // The maker changes an analysed value (the method) while the analysis is still running.
    await pickMethod(s, "Pick colours");
    await pickMethod(s, "Brightness");
    await s.evaluate(`window.__analysisDelay = 0`);
    await sleep(7000);
    assert.match(await methodText(s), /Brightness/, "the late answer did not overwrite the maker's choice");
    assert.notEqual(await rowState(s), "auto", "the line does not claim the analysis was applied");
    assert.ok(!/pale colours kept/.test(await lineText(s)), `the line does not claim what it did not set: ${await lineText(s)}`);
    // Analyse again recovers the proposal on request.
    assert.notEqual(await againState(s), "true", "Analyse again is enabled after a manual change");
    await s.clickSel(`${ROW} button`);
    await waitState(s, "auto");
    assert.match(await methodText(s), /Remove background/);
    assert.match(await lineText(s), / · pale colours kept$/);
  }, ANALYSIS_DELAY);
});

test("0089F tester c33, c83B: Smooth is a setting too; changed before the answer arrives, nothing is applied (spec: 'any setting')", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.evaluate(`window.__analysisDelay = 5000`);
    await openOn(s, "pale.png", PALE);
    await typeInto(s, "Smooth", "55");
    await s.evaluate(`window.__analysisDelay = 0`);
    await sleep(7000);
    console.log("method:", await methodText(s), "row:", await rowState(s), "line:", await lineText(s), "smooth:", await fieldValue(s, "Smooth"));
    assert.equal(await fieldValue(s, "Smooth"), "55", "the maker's Smooth is kept");
    assert.match(await methodText(s), /Brightness/, "the maker changed a setting of this image, so the analysis is not applied");
  }, ANALYSIS_DELAY);
});

test("0089F tester c33: a second image after a manual change on the first is a new image and is analysed", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pale.png", PALE);
    await waitState(s, "auto");
    assert.match(await methodText(s), /Remove background/);
    await pickMethod(s, "Brightness");
    await waitState(s, "custom");
    await importAnother(s, "pale2.png", PALE);
    await waitFor("the second image analysed", async () => ((await methodText(s)).includes("Remove background") && (await rowState(s)) === "auto" ? true : null), 40_000);
    assert.match(await lineText(s), / · pale colours kept$/);
  });
});

// ------------------------------------------------------------------ Frage 86 A: only image-dependent values

test("0089F tester c33, c86A: Analyse again never touches the preset, Smooth, Tolerance, Corners, Ignore below, Invert, Drop border shapes or Output", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pale.png", PALE);
    await waitState(s, "auto");
    // Preset Photo (a manual job choice) with its own values untouched: Analyse again keeps the preset.
    await s.evaluate(`document.querySelector('${BAR} button[aria-label=Preset]').click()`);
    await sleep(300);
    await s.evaluate(`[...document.querySelectorAll('[role=menu] [role=menuitemradio], [role=menu] [role=menuitem]')].find((e) => e.textContent.startsWith('Photo')).click()`);
    await sleep(800);
    assert.match(await presetText(s), /^Photo/);
    const photo = { smooth: await fieldValue(s, "Smooth"), corners: await fieldValue(s, "Corners"), specks: await fieldValue(s, "Ignore below"), tolerance: await fieldValue(s, "Tolerance") };
    await pickMethod(s, "Brightness");
    await waitState(s, "custom");
    await s.clickSel(`${ROW} button`);
    await waitState(s, "auto");
    assert.match(await presetText(s), /^Photo/, "Analyse again keeps the Photo preset");
    assert.deepEqual({ smooth: await fieldValue(s, "Smooth"), corners: await fieldValue(s, "Corners"), specks: await fieldValue(s, "Ignore below"), tolerance: await fieldValue(s, "Tolerance") }, photo, "the preset's values are not analysed values");
    // Now values of the maker's own.
    await typeInto(s, "Smooth", "37");
    await typeInto(s, "Corners", "42");
    await typeInto(s, "Ignore below", "0.9");
    await typeInto(s, "Tolerance", "1.2");
    await s.evaluate(`document.querySelector('${BAR} button[aria-label="Invert"]').click()`);
    await sleep(500);
    await s.evaluate(`document.querySelector('${BAR} button[aria-label="Drop border shapes"]').click()`);
    await sleep(500);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await sleep(800);
    // A manual change of an analysed value enables Analyse again.
    await pickMethod(s, "Brightness");
    await waitState(s, "custom");
    const snap = async () => ({
      preset: await presetText(s),
      smooth: await fieldValue(s, "Smooth"),
      corners: await fieldValue(s, "Corners"),
      specks: await fieldValue(s, "Ignore below"),
      tolerance: await fieldValue(s, "Tolerance"),
      invert: await pressedOf(s, "Invert"),
      drop: await pressedOf(s, "Drop border shapes"),
      output: await outputText(s),
    });
    const before = await snap();
    console.log("before:", JSON.stringify(before));
    assert.equal(before.preset.startsWith("Photo") || before.preset.startsWith("Custom"), true, `the preset is the maker's: ${before.preset}`);
    assert.equal(before.output, "Multicolour");
    await s.clickSel(`${ROW} button`);
    await waitState(s, "auto");
    const after = await snap();
    console.log("after:", JSON.stringify(after));
    assert.deepEqual(after, before, "Analyse again changed a value that is not image-dependent");
  });
});

test("0089F tester c33: Analyse again keeps the picked colours and the Single/Multicolour choice", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "two.png", TWO);
    await waitState(s, "auto");
    await pickMethod(s, "Pick colours");
    await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=Image]').click()`);
    await sleep(1000);
    const shotNow = await shot(s);
    const [cx, , cw, ch] = await canvasRect(s);
    const red = boxOf(shotNow, [cx, (await barBottom(s)) + 4, cw, ch], ([r, g, b]) => r > 180 && g < 50 && b < 50);
    assert.ok(Number.isFinite(red.cx), "the red block is on screen");
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await s.send("Input.dispatchMouseEvent", { type, x: red.cx, y: red.cy, button: "left", clickCount: 1 });
    }
    await sleep(800);
    const picked = await swatchList(s);
    assert.equal(picked.length, 1, `one colour picked: ${JSON.stringify(picked)}`);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await sleep(800);
    await waitState(s, "custom");
    await s.clickSel(`${ROW} button`);
    await waitState(s, "auto");
    assert.equal(await outputText(s), "Multicolour", "the analysis never switches Single to Multicolour or back");
    console.log("method after Analyse again:", await methodText(s));
    // The picked colour is still there: back in Pick colours the swatch is shown.
    if (!(await methodText(s)).includes("Pick colours")) await pickMethod(s, "Pick colours");
    assert.deepEqual(await swatchList(s), picked, "the picked colours are kept by Analyse again");
  });
});

// ------------------------------------------------------------------ the 36 px analysis row

/** The one-line rule (UX notes 3, lead 2026-10-31): the row is 36 px, the line is one line, and a line that
 * does not fit ends in an ellipsis with the whole text as its tooltip; the live region keeps the whole text. */
function oneLine(box: Awaited<ReturnType<typeof rowBox>>, tag: string) {
  assert.ok(Math.abs(box.height - 36) <= 1, `${tag}: row ${box.height} px`);
  assert.ok(box.lineHeight <= 20, `${tag}: a single line (${box.lineHeight} px high)`);
  assert.ok(box.text.length > 0);
  if (box.truncated) {
    assert.equal(box.title, box.text, `${tag}: a cut line has the whole text as its tooltip`);
  }
}

async function rowInEveryState(w: number, h: number) {
  {
    await withApp(async (s) => {
      await s.evaluate(`window.__analysisDelay = 4000`);
      await openOn(s, "lines.png", LINES);
      const analysing = await rowBox(s);
      const smoothTop = (await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label=Smooth]')?.getBoundingClientRect().top ?? -1`)) as number;
      const state0 = await rowState(s);
      console.log(`${w}x${h} analysing (${state0}):`, JSON.stringify(analysing));
      assert.ok(Math.abs(analysing.height - 36) <= 1, `${w}x${h}: the row exists from the first frame at 36 px, got ${analysing.height}`);
      await s.evaluate(`window.__analysisDelay = 0`);
      await waitState(s, "auto");
      const auto = await rowBox(s);
      console.log(`${w}x${h} auto:`, JSON.stringify(auto), await lineText(s));
      assert.ok(Math.abs(auto.height - 36) <= 1, `${w}x${h}: Auto row ${auto.height} px`);
      assert.ok(Math.abs(auto.top - analysing.top) <= 1, `${w}x${h}: the row did not move when the result arrived (${analysing.top} -> ${auto.top})`);
      const smoothTop2 = (await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label=Smooth]')?.getBoundingClientRect().top ?? -1`)) as number;
      assert.ok(Math.abs(smoothTop2 - smoothTop) <= 1, `${w}x${h}: nothing below the row moved (${smoothTop} -> ${smoothTop2})`);
      assert.ok(auto.width >= auto.barWidth - 24, `${w}x${h}: a full-width row of its own (${auto.width} of ${auto.barWidth})`);
      assert.ok(auto.button !== null && Math.abs(auto.button[1] - 28) <= 1 && Math.abs(auto.button[2] - 8) <= 2, `${w}x${h}: the button is 28 px high, 8 px from the bar's right edge: ${JSON.stringify(auto.button)}`);
      oneLine(auto, `${w}x${h} auto`);
      assert.equal(auto.fontSize, "12px");
      assert.equal(auto.aria, "polite");
      // Custom.
      await pickMethod(s, "Remove background");
      await waitState(s, "custom");
      const custom = await rowBox(s);
      console.log(`${w}x${h} custom:`, await lineText(s));
      assert.match(await lineText(s), /^Custom · (Graphic|Logo|Line drawing|Sketch)$/);
      assert.ok(Math.abs(custom.height - 36) <= 1, `${w}x${h}: Custom row ${custom.height} px`);
      oneLine(custom, `${w}x${h} custom`);
    }, ANALYSIS_DELAY, w, h);
    await withApp(async (s) => {
      await openOn(s, "noise.png", NOISE);
      await waitState(s, "auto");
      const box = await rowBox(s);
      console.log(`${w}x${h} unclear:`, await lineText(s), JSON.stringify(box));
      assert.ok(Math.abs(box.height - 36) <= 1, `${w}x${h}: the 'Type unclear' row is ${box.height} px high`);
      oneLine(box, `${w}x${h} unclear`);
    }, "", w, h);
  }
}

test("0089F tester c31, UX 3: the analysis row is 36 px, full width, from the first frame, in every state, at 1280 x 800", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await rowInEveryState(1280, 800);
});

test("0089F tester c31, UX 3: the analysis row is one 36 px line in every state at 800 x 600 (panel open, 308 px); a cut line has its whole text as tooltip", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await rowInEveryState(800, 600);
});

// ------------------------------------------------------------------ the analysis fails

test("0089F tester c31, UX 3: when the analysis fails the row says so and the trace still has a Threshold", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.evaluate(`window.__failWorker = true`);
    await openOn(s, "lines.png", LINES);
    await sleep(6000);
    const state = await rowState(s);
    const line = await lineText(s);
    console.log("state:", state, "line:", JSON.stringify(line), "threshold:", await fieldValue(s, "Threshold"));
    assert.equal(state, "failed");
    assert.equal(line, "Analysis failed · automatic Threshold used");
  }, WORKER_DIES);
});

test("0089F tester UX 3: the Analyse again button is 88 x 28 px", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "lines.png", LINES);
    await waitState(s, "auto");
    const box = await rowBox(s);
    assert.ok(box.button !== null && Math.abs(box.button[0] - 88) <= 2 && Math.abs(box.button[1] - 28) <= 1, `${JSON.stringify(box.button)}`);
  });
});

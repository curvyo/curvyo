// Tester cases for `specs/0025-raster-trace` slice 2 in the real app (Chromium via DevTools, set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Preset menu, the
// Advanced block, the remembered choice, the trace group tooltip and the dark-theme wash.
// Harness as in `acceptance_0025_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5226;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0025-slice2");
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

async function launch() {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = startVite(PORT, FRONTEND);
  await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
  const s = await session(profile);
  // UX notes 20.4 (2026-10-29): the counts are the Advanced block's last row; the block starts
  // open unless a test closed it (it is remembered).
  await s.send("Page.addScriptToEvaluateOnNewDocument", { source: "if (localStorage.getItem('curvyo.trace-advanced-open') === null) localStorage.setItem('curvyo.trace-advanced-open', '1');" });
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
    return /\d+ paths?, [\d,]+ nodes?, [\d.]+ s|0 paths|covering the whole image/.test(t) ? t : null;
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



/** Black blobs on white, 300 x 300: a shape with many curves. */
const blobs = (x: number, y: number) => (Math.sin(x * 0.05) + Math.cos(y * 0.04) + Math.sin((x + y) * 0.03) > 0.4 ? 0 : 255);

const bar = async (s: S) => (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace]')?.innerText ?? ''`)) as string;
const presetButton = async (s: S) =>
  (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label=Preset]')?.textContent.trim() ?? null`)) as string | null;
const nodesOf = (status: string) => Number(/([\d,]+) nodes?/.exec(status)?.[1].replace(",", ""));
const settle = async (s: S) => {
  await sleep(700);
  return traceReady(s);
};
const openMenu = async (s: S) => {
  await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label=Preset]').setAttribute('data-t25', '1')`);
  await s.clickSel("[data-t25]");
  await waitFor("the menu", async () => ((await s.evaluate(`!!document.querySelector('[role=menu]')`)) ? true : null));
};
const pick = async (s: S, name: string) => {
  await openMenu(s);
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=menu] [role=menuitemradio], [role=menu] [role=menuitem]')].find((i) => i.innerText.startsWith(${JSON.stringify(name)})).setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
  await sleep(500);
};
/** Presses the bar's expander (an icon button, found by its aria-label). */
const clickExpander = async (s: S) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); document.querySelector('[role=toolbar][aria-label=Trace] button[aria-expanded][aria-controls]').setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
};
/** Creates the paths: Enter on the canvas (UX notes 19, no button). */
const createPaths = async (s: S) => {
  await focusCanvas(s);
  await s.key("Enter", 13);
};
const advanced = async (s: S) => {
  if ((await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace]')?.querySelector('[role=spinbutton][aria-label=Corners]') === null`)) as boolean) {
    await clickExpander(s);
    await sleep(400);
  }
};
const radio = async (s: S, group: string, label: string) =>
  (await s.evaluate(`[...document.querySelectorAll('[role=radiogroup][aria-label=${group}] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)})?.getAttribute('aria-checked')`)) as string;
const clickRadio = async (s: S, group: string, label: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=radiogroup][aria-label=${group}] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
  await sleep(500);
};
const toggle = async (s: S, label: string) => {
  await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label="${label}"]').setAttribute('data-t25', '1'); })()`);
  await s.clickSel("[data-t25]");
  await sleep(500);
};
const switchState = async (s: S, label: string) => (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label="${label}"]')?.getAttribute('aria-pressed')`)) as string;
const reload = async (s: S) => {
  const reloading = s.send("Page.reload");
  await sleep(1000);
  await s.send("Page.handleJavaScriptDialog", { accept: true });
  await reloading;
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
};

test("0025s2 in Chromium: the Preset menu, its values, Custom, the remembered choice (criteria 37, 40, 59, 60, 64, 66)", { skip: !BROWSER, timeout: 500_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "blobs.png", blobs, 300, 300);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    // Criteria 37 and 59: a row above Threshold; 132 px; the first use is Logo / Line art.
    assert.equal(await presetButton(s), "Logo / Line art", "the first-ever use starts on Logo / Line art");
    const geo = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('button[aria-label=Preset]').getBoundingClientRect(); const t = document.querySelector('[role=spinbutton][aria-label=Threshold]').getBoundingClientRect(); return { w: Math.round(b.width), presetY: b.y, thresholdY: t.y, presetX: b.x, thresholdX: t.x }; })())`)) as string,
    ) as { w: number; presetY: number; thresholdY: number; presetX: number; thresholdX: number };
    assert.equal(geo.w, 132);
    // The Preset row comes first: Threshold is below it, or, where the bar is wide enough, after it
    // in row 1 (UX notes 20.5: "row 1 + Threshold" with the panel closed).
    assert.ok(geo.presetY < geo.thresholdY || (geo.presetY === geo.thresholdY && geo.presetX < geo.thresholdX), `the Preset row comes before Threshold: ${JSON.stringify(geo)}`);
    assert.equal(await s.evaluate(`document.querySelector('button[aria-label=Preset]').getAttribute('aria-haspopup')`), "menu");
    // Criterion 59: four presets in order with the customer's second lines; criterion 40: no advice.
    await openMenu(s);
    const items = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=menu] [role=menuitemradio], [role=menu] [role=menuitem]')].map((i) => i.innerText.split(String.fromCharCode(10)).map((t) => t.trim()).filter((t) => t && t !== '✓')))`)) as string;
    assert.deepEqual(JSON.parse(items), [
      ["Logo / Line art", "Sharp corners, clean curves."],
      ["Photo", "Many shapes. Photos trace roughly."],
      ["Silhouette for cutting/carving", "Closed, simple, few nodes."],
      ["Engraving", "Keeps fine lines."],
    ]);
    // Escape closes the menu only.
    await press(s, "Escape", 27);
    await sleep(300);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=menu]')`), false, "Escape closes the menu");
    assert.notEqual(await fieldValue(s, "Threshold"), null, "the tool stays");
    // Keyboard: open with Enter/Down, arrows, Enter picks.
    await s.evaluate(`document.querySelector('button[aria-label=Preset]').focus()`);
    await press(s, "ArrowDown", 40);
    await waitFor("the menu by Down", async () => ((await s.evaluate(`!!document.querySelector('[role=menu]')`)) ? true : null));
    await press(s, "ArrowDown", 40);
    await press(s, "Enter", 13);
    await sleep(500);
    const afterKeys = await presetButton(s);
    assert.ok(["Photo", "Silhouette for cutting/carving"].includes(afterKeys ?? ""), `arrows and Enter pick a preset: ${afterKeys}`);

    // Criterion 60: the values, and Threshold is never set.
    await typeInto(s, "Threshold", "100");
    await advanced(s);
    const values = async () => ({
      smooth: await fieldValue(s, "Smooth"),
      specks: await fieldValue(s, "Ignore below"),
      corners: await fieldValue(s, "Corners"),
      denoise: [await radio(s, "Denoise", "Off"), await radio(s, "Denoise", "Low"), await radio(s, "Denoise", "High")].join(),
      upscale: [await radio(s, "Upscale", "Auto"), await radio(s, "Upscale", "Off")].join(),
      threshold: await fieldValue(s, "Threshold"),
    });
    const want: Record<string, ReturnType<typeof values> extends Promise<infer V> ? V : never> = {
      "Logo / Line art": { smooth: "80", specks: "0.50", corners: "70", denoise: "true,false,false", upscale: "false,true", threshold: "100" },
      Photo: { smooth: "50", specks: "0.10", corners: "30", denoise: "true,false,false", upscale: "false,true", threshold: "100" },
      "Silhouette for cutting/carving": { smooth: "100", specks: "2.00", corners: "70", denoise: "false,true,false", upscale: "false,true", threshold: "100" },
      Engraving: { smooth: "80", specks: "0.10", corners: "70", denoise: "true,false,false", upscale: "true,false", threshold: "100" },
    };
    const nodes: Record<string, number> = {};
    for (const name of Object.keys(want)) {
      await pick(s, name);
      assert.equal(await presetButton(s), name);
      assert.deepEqual(await values(), want[name], name);
      nodes[name] = nodesOf(await settle(s));
    }
    console.log("nodes per preset on the blobs fixture:", JSON.stringify(nodes));
    // Criterion 64: the preview updates on a pick (the node order is the Rust test c64, 0089 criterion 9a).

    // Criterion 40: an edit turns the button into Custom; the pick restores.
    await typeInto(s, "Smooth", "33");
    assert.equal(await presetButton(s), "Custom");
    await pick(s, "Logo / Line art");
    assert.equal(await presetButton(s), "Logo / Line art");
    assert.equal((await values()).smooth, "80");
    await typeInto(s, "Corners", "10");
    assert.equal(await presetButton(s), "Custom", "an Advanced edit of a preset value is Custom too");
    await pick(s, "Logo / Line art");
    await toggle(s, "Invert");
    console.log("button after Invert:", await presetButton(s));
    await toggle(s, "Invert");

    // Criterion 66: remembered between uses and across a reload; Custom keeps its values.
    await pick(s, "Engraving");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    assert.equal(await presetButton(s), "Engraving", "the choice is used for the next use");
    await typeInto(s, "Smooth", "33");
    assert.equal(await presetButton(s), "Custom");
    await reload(s);
    await importImage(s, "blobs2.png", blobs, 300, 300);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    assert.equal(await presetButton(s), "Custom", "Custom survives a reload");
    assert.equal(await fieldValue(s, "Smooth"), "33");
    await advanced(s);
    assert.equal(await radio(s, "Upscale", "Auto"), "true", "Custom keeps the Engraving values it started from");
    await pick(s, "Silhouette");
    await reload(s);
    await importImage(s, "blobs3.png", blobs, 300, 300);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    assert.equal(await presetButton(s), "Silhouette for cutting/carving", "a picked preset survives a reload");

    // The notice does not name the preset.
    await createPaths(s);
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced: /.test(t) ? t : null;
    });
    assert.ok(!/Silhouette|Photo|Engraving|Logo|preset/i.test(notice), notice);
  } finally {
    await close();
  }
});

test("0025s2 in Chromium: one preset for all selected images, in one step (criteria 46, 66)", { skip: !BROWSER, timeout: 400_000 }, async () => {
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
    await buttonByText(s, "Trace");
    await s.clickSel("[data-t25]");
    await settle(s);
    await pick(s, "Silhouette");
    const status = await settle(s);
    const one = nodesOf(status);
    await createPaths(s);
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced 3 images/.test(t) ? t : null;
    });
    assert.equal(nodesOf(notice), 3 * one, `identical images: three times the previewed ${one} nodes in "${notice}" (the pick applies to all)`);
    assert.ok(!/Silhouette|preset/i.test(notice));
  } finally {
    await close();
  }
});

test("0025s2 in Chromium: the Advanced block (criteria 41, 44)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    const geometry = async () =>
      JSON.parse(
        (await s.evaluate(`JSON.stringify((() => { const y = (q) => document.querySelector(q).getBoundingClientRect().y; const a = document.querySelector('[role=toolbar][aria-label=Trace] button[aria-expanded][aria-controls]'); return { preset: y('button[aria-label=Preset]'), threshold: y('[role=spinbutton][aria-label=Threshold]'), smooth: y('[role=spinbutton][aria-label=Smooth]'), status: y('[role=toolbar][aria-label=Trace] [role=status]'), advanced: a.getBoundingClientRect().y, disabled: a.disabled || a.getAttribute('aria-disabled'), expanded: a.getAttribute('aria-expanded') }; })())`)) as string,
      ) as Record<string, number | string | boolean | null>;
    // The block starts open in this file (the counts live there); close it first.
    if ((await geometry()).expanded === "true") {
      await clickExpander(s);
      await sleep(400);
    }
    const closed = await geometry();
    assert.equal(closed.expanded, "false");
    assert.ok(!closed.disabled, "Advanced is never disabled or dimmed");
    assert.equal(await s.evaluate(`!!document.querySelector('[role=spinbutton][aria-label=Corners]')`), false, "the block is closed");
    await clickExpander(s);
    await sleep(400);
    const open = await geometry();
    assert.equal(open.expanded, "true");
    for (const k of ["preset", "threshold", "smooth", "status", "advanced"]) {
      assert.equal(open[k], closed[k], `${k} row does not move when the block opens`);
    }
    // Contents.
    const text = await bar(s);
    for (const label of ["Denoise", "Upscale", "Corners"]) {
      assert.ok(text.includes(label), `${label} in the block: ${JSON.stringify(text)}`);
    }
    // Frage 74 A (2026-10-29): Node limit and Full-size preview are gone.
    for (const label of ["Node limit", "Full-size preview"]) {
      assert.ok(!text.includes(label), `${label} is not in the block`);
      assert.equal(await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [aria-label="${label}"]')`), false, `no ${label} control`);
    }
    // "Ignore background" is named "Drop border shapes" since `specs/0089` Frage 88 A (customer).
    for (const label of ["Invert", "Drop border shapes"]) {
      assert.equal(await switchState(s, label), "false", `${label} is an icon toggle (aria-pressed), off by default`);
      assert.ok(!text.includes(label), `${label} is not drawn as a label`);
    }
    assert.deepEqual([await radio(s, "Denoise", "Off"), await radio(s, "Denoise", "Low"), await radio(s, "Denoise", "High")], ["true", "false", "false"]);
    // Two columns: the controls sit at two or more distinct x positions.
    const xs = JSON.parse(
      (await s.evaluate(`JSON.stringify([...new Set([...document.querySelectorAll('[role=radiogroup][aria-label=Denoise],[role=radiogroup][aria-label=Upscale],[role=spinbutton][aria-label=Corners],[role=toolbar][aria-label=Trace] button[aria-pressed]')].map((e) => Math.round(e.getBoundingClientRect().x / 20)))])`)) as string,
    ) as number[];
    assert.ok(xs.length >= 2, `two columns: ${xs}`);
    // Corners clamps.
    await typeInto(s, "Corners", "150");
    assert.equal(await fieldValue(s, "Corners"), "100");
    await typeInto(s, "Corners", "70");
    // Criterion 44: Invert and Ignore background on the ring (outer r 60, hole r 20).
    const status0 = await settle(s);
    assert.match(status0, /^2 paths/);
    await toggle(s, "Invert");
    assert.match(await settle(s), /^3 paths/, "inverted: the page with the ring as a hole, and the inner disc");
    await toggle(s, "Drop border shapes");
    assert.match(await settle(s), /^1 path\b/, "the page touches the border and goes");
    await toggle(s, "Invert");
    assert.match(await settle(s), /^2 paths/, "not inverted, background ignored: the ring touches no border");
    await toggle(s, "Drop border shapes");
    // The open block is remembered.
    await focusCanvas(s);
    await s.key("Escape", 27);
    await sleep(300);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=spinbutton][aria-label=Corners]')`), true, "open stays open");
    await reload(s);
    await importImage(s, "ring2.png", ring);
    await focusCanvas(s);
    await s.key("i", 73);
    await settle(s);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=spinbutton][aria-label=Corners]')`), true, "open survives a reload");
    await clickExpander(s);
    await sleep(300);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=spinbutton][aria-label=Corners]')`), false);
  } finally {
    await close();
  }
});

test("0025s2 in Chromium: the trace group's tooltip and the light wash (criterion 68)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    await focusCanvas(s);
    // (b) the wash on the light theme: the black ring becomes white at 65 % over black = 166.
    const shot = async () => decodePng(Buffer.from(((await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } }).result?.data ?? "", "base64"));
    const [cx, cy, cw, ch] = await canvasRect(s);
    const before = await shot();
    const ringPixels: [number, number][] = [];
    for (let y = Math.floor(cy) + 120; y < cy + ch - 20 && ringPixels.length < 400; y += 3) {
      for (let x = Math.floor(cx) + 420; x < cx + cw - 20 && ringPixels.length < 400; x += 3) {
        const i = (y * before.width + x) * 4;
        if (before.data[i] < 12 && before.data[i + 1] < 12 && before.data[i + 2] < 12) {
          ringPixels.push([x, y]);
        }
      }
    }
    assert.ok(ringPixels.length > 100, `black ring pixels found: ${ringPixels.length}`);
    await s.key("i", 73);
    await settle(s);
    const dim = await shot();
    const reds = ringPixels.map(([x, y]) => dim.data[(y * dim.width + x) * 4]).sort((a, b) => a - b);
    const median = reds[Math.floor(reds.length / 2)];
    console.log("dimmed ring pixel (median red):", median);
    assert.ok(Math.abs(median - 166) <= 10, `the white wash at 65 % over black is about 166, got ${median}`);

    // (a) the Layers row of the group repeats the success notice.
    await press(s, "Enter", 13);
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced: \d+ paths?, [\d,]+ nodes?\./.exec(t)?.[0] ?? null;
    });
    await s.tab("Layers");
    const groupRow = await waitFor("the group row", async () =>
      ((await s.evaluate(`!![...document.querySelectorAll('[role=row]')].find((r) => /Trace of/.test(r.getAttribute('aria-label') || ''))`)) ? true : null),
    );
    assert.ok(groupRow);
    await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); [...document.querySelectorAll('[role=row]')].find((r) => /Trace of/.test(r.getAttribute('aria-label') || '')).setAttribute('data-t25', '1'); })()`);
    const rect = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-t25]').getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: rect[0], y: rect[1] });
    await sleep(900);
    const rowText = (await s.evaluate(`document.querySelector('[data-t25]').innerText`)) as string;
    const tip = (await s.evaluate(`document.querySelector('[role=tooltip]')?.textContent ?? ''`)) as string;
    const title = (await s.evaluate(`document.querySelector('[data-t25]').getAttribute('title') ?? ''`)) as string;
    console.log("group row text:", JSON.stringify(rowText), "| tooltip:", JSON.stringify(tip), "| title:", JSON.stringify(title));
    assert.ok([tip, title].some((t) => t.includes(notice)), `the row's tooltip repeats "${notice}": tooltip=${JSON.stringify(tip)} title=${JSON.stringify(title)}`);
    const imageRow = (await s.evaluate(`[...document.querySelectorAll('[role=row]')].find((r) => /ring.png/.test(r.getAttribute('aria-label') || '') && !/Trace of/.test(r.getAttribute('aria-label') || '')).innerText`)) as string;
    assert.ok(!/Traced/.test(imageRow), "the image row has no trace text");
  } finally {
    await close();
  }
});

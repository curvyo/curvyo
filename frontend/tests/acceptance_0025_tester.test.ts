// Tester cases for `specs/0025-raster-trace` slice 1 in the real app (Chromium via DevTools, set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written from the
// specification: the Trace button of the Select bar, the key I, the bar, the preview overlay,
// Create paths in one undo step, several images, the empty states. Harness as in
// `acceptance_0058_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5225;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0025-tester");
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

async function launch() {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
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
    vite.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
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

test("0025 in Chromium: Trace button, key I, bar, preview, fields, leaving (criteria 1 to 5, 7, 8, 9, 11, 15)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    // Criterion 1: no rail button; a text button "Trace" in the Select bar while an image is selected.
    const rail = (await s.evaluate(`[...document.querySelectorAll('button[aria-label]')].map((b) => b.getAttribute('aria-label')).filter((n) => /trace/i.test(n)).join('|')`)) as string;
    assert.equal(rail, "", "the rail has no Trace button");
    assert.ok(await buttonByText(s, "Trace"), "Select bar shows Trace for an image selection");
    await s.evaluate(`document.querySelector('[data-t25]').scrollIntoView()`);
    const box = JSON.parse((await s.evaluate(`JSON.stringify(document.querySelector('[data-t25]').getBoundingClientRect())`)) as string) as { x: number; y: number; width: number; height: number };
    assert.ok(box.height >= 28 - 0.5 && box.height <= 28.5, `28 px high: ${box.height}`);
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: box.x + box.width / 2, y: box.y + box.height / 2 });
    const tip = await waitFor("the tooltip", async () => {
      const t = (await s.evaluate(`document.querySelector('[role=tooltip]')?.textContent ?? ''`)) as string;
      return t ? t : null;
    });
    assert.equal(tip.trim(), "Trace tool (I)");

    const rowsBefore = await s.layerRows();
    const historyBefore = await historyText(s);
    const base = await pixels(s);
    assert.ok(base.black > 200, `the ring is drawn black: ${base.black}`);

    // Criterion 2: choosing the tool shows the bar, dims the picture, draws the preview, writes nothing.
    await s.clickSel("[data-t25]");
    const settled = await traceReady(s);
    assert.match(settled, /^2 paths, \d+ nodes, [\d.]+ s/, "ring and hole: 2 paths");
    const pressed = (await s.evaluate(`[...document.querySelectorAll('button[aria-pressed=true]')].map((b) => b.getAttribute('aria-label')).join('|')`)) as string;
    assert.equal(pressed, "", "no rail button is pressed in the Trace tool");
    const bar = (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace]')?.innerText ?? ''`)) as string;
    // 0089 criterion 22a (Frage 84): no state label; the first control is the About Trace "i" and
    // the only "Trace" text is the finish button at the end of the flow.
    assert.doesNotMatch(bar, /^Trace\b/, `no state label at the start: ${JSON.stringify(bar.slice(0, 40))}`);
    assert.equal((await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace]').querySelector('button, [role=spinbutton]').getAttribute('aria-label')`)) as string, "About Trace", "the first control is the About Trace button");
    assert.equal((await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-slot]').textContent.trim()`)) as string, "Trace", "the finish button reads Trace");
    const labels = await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=spinbutton]')].filter((i) => i.offsetParent).map((i) => i.getAttribute('aria-label')))`);
    console.log("bar fields:", labels, "| bar text:", JSON.stringify(bar));
    assert.equal(await fieldValue(s, "Threshold"), "50", "criterion 9: automatic value of a 0/255 picture (luma 128) shown as 50 %");
    assert.equal(await fieldValue(s, "Smooth"), "80");
    assert.equal(await fieldValue(s, "Ignore below"), "0.50");
    assert.match(bar, /~\s*\d+ px/, "the muted pixel readout");
    assert.deepEqual(await s.layerRows(), rowsBefore, "criterion 7: no node, no layer row");
    assert.equal(await historyText(s), historyBefore, "criterion 7: no history step");
    const during = await pixels(s);
    console.log("pixels", JSON.stringify({ base, during }));
    assert.ok(during.black < base.black / 4, `the picture is dimmed: black ${base.black} -> ${during.black}`);
    assert.ok(during.blue > 100, `the preview outline is drawn in --preview-new: ${during.blue}`);
    const hintHits = bar.match(/\b(try|tip|hint|press|drag|raise|lower|move)\b/gi);
    assert.equal(hintHits, null, `no operating hint text in the bar: ${bar}`);

    // Criterion 5: the fields.
    await s.evaluate(`document.querySelector('${field("Threshold")}').focus()`);
    await press(s, "ArrowUp", 38);
    assert.equal(await fieldValue(s, "Threshold"), "51");
    await press(s, "ArrowUp", 38, 8);
    assert.equal(await fieldValue(s, "Threshold"), "61");
    await press(s, "ArrowDown", 40, 8);
    assert.equal(await fieldValue(s, "Threshold"), "51");
    await press(s, "Home", 36);
    assert.equal(await fieldValue(s, "Threshold"), "0");
    await press(s, "End", 35);
    assert.equal(await fieldValue(s, "Threshold"), "100");
    await typeInto(s, "Threshold", "999");
    assert.equal(await fieldValue(s, "Threshold"), "100", "clamped silently");
    const focusAfter = (await s.evaluate(`document.activeElement?.tagName`)) as string;
    assert.notEqual(focusAfter, "INPUT", "Enter returns focus to the canvas");
    assert.equal(await s.evaluate(`document.activeElement?.getAttribute('role')`), null, "Enter leaves the field for the canvas");
    await typeInto(s, "Smooth", "150");
    assert.equal(await fieldValue(s, "Smooth"), "100");
    await s.evaluate(`document.querySelector('${field("Ignore below")}').focus()`);
    await press(s, "ArrowUp", 38);
    assert.equal(await fieldValue(s, "Ignore below"), "0.60");
    await press(s, "ArrowUp", 38, 8);
    assert.equal(await fieldValue(s, "Ignore below"), "1.60");
    await press(s, "ArrowUp", 38, 2);
    assert.equal(await fieldValue(s, "Ignore below"), "1.61");
    await press(s, "End", 35);
    assert.equal(await fieldValue(s, "Ignore below"), "10.00");
    await typeInto(s, "Ignore below", "-3");
    assert.equal(await fieldValue(s, "Ignore below"), "0.00");
    assert.equal(await historyText(s), historyBefore, "changing values is not a history step");

    // Criterion 15: 0 paths names the direction; Enter creates nothing (no button to disable).
    await typeInto(s, "Threshold", "0");
    const none = await waitFor("0 paths", async () => {
      const t = await barStatus(s);
      return /^0 paths/.test(t) ? t : null;
    });
    assert.equal(none, "0 paths at this Threshold: too low.");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('button')].some((b) => b.textContent.trim() === 'Create paths')`), false, "there is no Create paths button");
    const rowsNone = await rowInfo(s);
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await sleep(500);
    assert.deepEqual(await rowInfo(s), rowsNone, "Enter creates nothing for 0 paths");
    await typeInto(s, "Threshold", "50");
    await traceReady(s);

    // Criterion 5: remembered as an app setting.
    await typeInto(s, "Smooth", "55");
    await sleep(500);
    // Criterion 1: Escape leaves, the preview goes, the Select bar returns.
    await s.key("Escape", 27);
    await sleep(500);
    assert.equal(await fieldValue(s, "Threshold"), null, "the bar is gone");
    assert.ok(await buttonByText(s, "Trace"));
    const after = await pixels(s);
    assert.ok(Math.abs(after.blue - base.blue) < base.blue * 0.5, `the preview is dropped (only the selection frame stays blue): ${after.blue} vs ${base.blue}`);
    assert.ok(after.black > base.black / 2, "the picture is no longer dimmed");
    // Key I enters the tool, S leaves it.
    await s.key("i", 73);
    await traceReady(s);
    assert.equal(await fieldValue(s, "Smooth"), "55", "Smooth is remembered between uses");
    await s.key("s", 83);
    assert.equal(await fieldValue(s, "Threshold"), null, "S leaves the tool");
    // A rail tool leaves it too.
    await s.key("i", 73);
    await traceReady(s);
    await s.clickSel('button[aria-label^="Rectangle tool"]');
    assert.equal(await fieldValue(s, "Threshold"), null);
    assert.equal(await s.evaluate(`document.querySelector('button[aria-label^="Rectangle tool"]').getAttribute('aria-pressed')`), "true");
    // The value survives a reload (app setting, not project).
    const reloading = s.send("Page.reload");
    await sleep(1000);
    await s.send("Page.handleJavaScriptDialog", { accept: true });
    await reloading;
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    console.log("rows after reload:", JSON.stringify(await s.layerRows()));
    if (!(await s.layerRows()).some((r) => r?.includes("ring.png"))) {
      await importImage(s, "ring2.png", ring);
    }
    await focusCanvas(s);
    await s.key("i", 73);
    await traceReady(s);
    assert.equal(await fieldValue(s, "Smooth"), "55", "Smooth survives a reload");
    assert.deepEqual(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")), [], "no network (criterion 34)");
  } finally {
    await close();
  }
});

test("0025 in Chromium: Create paths in one undo step by Enter, double-click (criteria 12, 13, 30, 31)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "ring.png", ring);
    const rows0 = await rowInfo(s);
    const imageRow = rows0.find(([n]) => n?.includes("ring"))!;
    assert.ok(imageRow, `an image row in ${JSON.stringify(rows0)}`);
    const history0 = countOf(await historyText(s), "Trace image");
    const groupIndex = async () => (await rowInfo(s)).findIndex(([n]) => /Trace of/.test(n ?? ""));
    const check = async (how: string) => {
      // ux-notes section 5: the focus is back on the canvas, where Delete and the tool letters work.
      assert.equal(
        await s.evaluate(`document.activeElement?.tagName + ' ' + document.activeElement?.getAttribute('tabindex') + ' ' + !!document.activeElement?.querySelector?.('canvas')`),
        'DIV 0 true',
        `${how}: the focus is on the canvas after Create paths`,
      );
      const rows = await rowInfo(s);
      const g = rows.findIndex(([n]) => /Trace of/.test(n ?? ""));
      const img = rows.findIndex(([n]) => n?.includes("ring.png") && !/Trace of/.test(n));
      assert.ok(g >= 0 && img > g, `${how}: group row above the image row: ${JSON.stringify(rows)}`);
      assert.ok(rows.slice(g + 1, img).every(([, level]) => level > rows[g][1]), `${how}: directly above (only its own children between)`);
      assert.equal(rows[g][1], rows[img][1], `${how}: same level as the image`);
      assert.ok(/Trace of ring/.test(rows[g][0]!), `${how}: group named after the image: ${rows[g][0]}`);
      const notice = await waitFor("the notice", async () => {
        const t = await s.statusText();
        return /Traced: \d+ paths?, [\d,]+ nodes?\./.test(t) ? t : null;
      });
      assert.match(notice, /Traced: 2 paths, \d+ nodes\./);
      assert.equal(countOf(await historyText(s), "Trace image"), history0 + 1, `${how}: one history step "Trace image"`);
      assert.equal(await fieldValue(s, "Threshold"), null, `${how}: the Trace bar is gone`);
      const focus = (await s.evaluate(`document.activeElement.tagName + ' ' + (document.activeElement.getAttribute('aria-label') || document.activeElement.className).toString().slice(0, 40)`)) as string;
      console.log(`focus after Create paths by ${how}: ${focus}`);
      assert.equal(await s.evaluate(`document.querySelector('button[aria-label^="Select tool"]').getAttribute('aria-pressed')`), "true", `${how}: Select tool afterwards`);
      assert.ok((await selectedRows(s)).includes("Trace of"), `${how}: the new group is selected: ${await selectedRows(s)}`);
      assert.ok(await s.evaluate(`!!document.querySelector('[role=row][aria-label*="ring.png"]')`), `${how}: the image row stays`);
    };
    const undoRedo = async (how: string) => {
      await s.key("z", 90, 2);
      await waitFor("undo", async () => ((await groupIndex()) < 0 ? true : null));
      assert.deepEqual((await rowInfo(s)).map(([n]) => n), rows0.map(([n]) => n), `${how}: Ctrl+Z restores the rows`);
      await s.key("y", 89, 2);
      await waitFor("redo", async () => ((await groupIndex()) >= 0 ? true : null));
      await s.key("z", 90, 2);
      await waitFor("undo again", async () => ((await groupIndex()) < 0 ? true : null));
    };

    // (a) no button; the bar's hidden description says what Enter does (UX notes 19)
    await focusCanvas(s);
    await s.key("i", 73);
    await traceReady(s);
    assert.equal(await buttonByText(s, "Create paths"), false, "there is no Create paths button");
    assert.equal(
      await s.evaluate(`document.getElementById(document.querySelector('[role=toolbar][aria-label=Trace]').getAttribute('aria-describedby')).textContent`),
      "Enter creates the paths",
    );
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await check("first Enter");
    const created = await pixels(s);
    assert.ok(created.black > 200, `the paths are filled black over the visible image: ${created.black}`);
    await undoRedo("first Enter");

    // (b) Enter with focus on the canvas
    await focusCanvas(s);
    await s.key("i", 73);
    await traceReady(s);
    await press(s, "Enter", 13);
    await check("Enter");
    await undoRedo("Enter");

    // (b2) Ctrl+Enter from inside a field
    await focusCanvas(s);
    await s.key("i", 73);
    await traceReady(s);
    await s.evaluate(`document.querySelector('${field("Smooth")}').focus()`);
    await press(s, "Enter", 13, 2);
    await check("Ctrl+Enter");
    await undoRedo("Ctrl+Enter");

    // (c) double-click on the image
    await focusCanvas(s);
    await s.key("i", 73);
    await traceReady(s);
    const [cx, cy, cw, ch] = await canvasRect(s);
    const centre: [number, number] = [cx + cw / 2, cy + ch / 2];
    for (const clickCount of [1, 2]) {
      for (const type of ["mousePressed", "mouseReleased"]) {
        await s.send("Input.dispatchMouseEvent", { type, x: centre[0], y: centre[1], button: "left", clickCount });
      }
    }
    await sleep(800);
    await check("double-click");

    // Criterion 31: the group is independent of the image; deleting the group leaves the image.
    await focusCanvas(s);
    await s.key("Delete", 46);
    await sleep(500);
    const rows1 = await rowInfo(s);
    assert.ok(rows1.some(([n]) => n?.includes("ring.png")) && !rows1.some(([n]) => /Trace of/.test(n ?? "")));
  } finally {
    await close();
  }
});

test("0025 in Chromium: several images, empty states, zero and whole results (criteria 3, 15, 46)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    // Criterion 3: nothing selected.
    await focusCanvas(s);
    await s.key("i", 73);
    await sleep(500);
    const empty = (await s.evaluate(`document.body.innerText`)) as string;
    assert.match(empty, /Click an image to trace it\./);
    assert.ok(await buttonByText(s, "Import image..."), "Import image... button");
    const before = s.chooserCount();
    await s.clickSel("[data-t25]");
    await sleep(800);
    assert.equal(s.chooserCount(), before + 1, "Import image... opens the file chooser");
    writeFileSync(path.join(SCRATCH, "a.png"), pngGrey(200, 200, ring));
    await s.send("DOM.setFileInputFiles", { files: [path.join(SCRATCH, "a.png")], backendNodeId: s.choosers[s.choosers.length - 1] });
    await waitFor("the image row", async () => ((await s.layerRows()).length > 0 ? true : null));
    await focusCanvas(s);
    await s.key("Escape", 27);
    await importImage(s, "b.png", (x, y) => (Math.hypot(x - 100, y - 100) < 50 ? 0 : 255));
    await importImage(s, "c.png", (x, y) => (x > 60 && x < 140 && y > 60 && y < 140 ? 0 : 255));
    // Criterion 3: something else selected.
    await focusCanvas(s);
    await s.key("Escape", 27);
    await focusCanvas(s);
    await s.key("r", 82);
    const [cx, cy, cw, ch] = await canvasRect(s);
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: cx + cw - 160, y: cy + ch - 130 });
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: cx + cw - 160, y: cy + ch - 130, button: "left", clickCount: 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: cx + cw - 100, y: cy + ch - 80 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: cx + cw - 100, y: cy + ch - 80, button: "left", clickCount: 1 });
    await sleep(400);
    assert.ok(!(await buttonByText(s, "Trace")), "no Trace button for a rectangle");
    await focusCanvas(s);
    await s.key("i", 73);
    await sleep(400);
    assert.match((await s.evaluate(`document.body.innerText`)) as string, /The selection is not an image\. Click an image to trace it\./);
    await focusCanvas(s);
    await s.key("Escape", 27);
    await focusCanvas(s);
    await s.key("Escape", 27);

    // Criterion 46: all three images.
    await focusCanvas(s);
    await s.key("a", 65, 2);
    await sleep(400);
    const rowsAll = await rowInfo(s);
    const imagesOnly = rowsAll.filter(([n]) => /\.png/.test(n ?? "")).length === 3 && rowsAll.length === 3;
    console.log("rows", JSON.stringify(rowsAll));
    if (!imagesOnly) {
      // the rectangle exists: select the three images with Shift-clicks on their rows
      await focusCanvas(s);
    await s.key("Escape", 27);
      await s.evaluate(`document.activeElement?.blur?.()`);
    }
    const history0 = countOf(await historyText(s), "Trace image");
    await focusCanvas(s);
    await s.key("Escape", 27);
    // select the three image rows with Ctrl-clicks
    const selectRow = async (name: string, additive: boolean) => {
      await s.evaluate(`(() => { document.querySelectorAll('[data-t25]').forEach((e) => e.removeAttribute('data-t25')); const r = [...document.querySelectorAll('[role=row]')].find((e) => (e.getAttribute('aria-label') || '').includes(${JSON.stringify(name)})); r.setAttribute('data-t25', '1'); })()`);
      const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-t25]').getBoundingClientRect(); return [b.x + 80, b.y + b.height / 2]; })())`)) as string) as [number, number];
      for (const type of ["mousePressed", "mouseReleased"]) {
        await s.send("Input.dispatchMouseEvent", { type, x: c[0], y: c[1], button: "left", clickCount: 1, modifiers: additive ? 2 : 0 });
      }
      await sleep(300);
    };
    await selectRow("a.png", false);
    await selectRow("b.png", true);
    await selectRow("c.png", true);
    assert.equal((await selectedRows(s)).split("|").length, 3, `three images selected: ${await selectedRows(s)}`);
    assert.ok(await buttonByText(s, "Trace"), "Trace shows for several images");
    await s.clickSel("[data-t25]");
    await traceReady(s);
    const bar = (await s.evaluate(`document.body.innerText`)) as string;
    // Criterion 46: the label counts the job (the first image until Create paths moves on).
    assert.match(bar, /Tracing 1 of 3 images/);
    await press(s, "Enter", 13);
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return /Traced 3 images/.test(t) ? t : null;
    });
    assert.match(notice, /Traced 3 images: \d+ paths, [\d,]+ nodes\./);
    const rows = await rowInfo(s);
    const groups = rows.filter(([n]) => /Trace of/.test(n ?? ""));
    assert.equal(groups.length, 3, JSON.stringify(rows));
    for (const name of ["a.png", "b.png", "c.png"]) {
      const g = rows.findIndex(([n]) => n?.includes(`Trace of ${name.replace(".png", "")}`));
      const i = rows.findIndex(([n]) => n?.includes(name) && !/Trace of/.test(n));
      assert.ok(g >= 0 && i > g, `${name}: group above its image: ${JSON.stringify(rows)}`);
    }
    assert.equal((await selectedRows(s)).split("|").filter((n) => /Trace of/.test(n)).length, 3, "the three groups are selected");
    assert.equal(countOf(await historyText(s), "Trace image"), history0 + 1, "one history step");
    await s.key("z", 90, 2);
    await sleep(500);
    assert.equal((await rowInfo(s)).filter(([n]) => /Trace of/.test(n ?? "")).length, 0, "one Ctrl+Z removes all three");
  } finally {
    await close();
  }
});

test("0025 in Chromium: empty and whole-page results (criterion 15)", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const { s, close } = await launch();
  try {
    await s.tab("Layers");
    await importImage(s, "white.png", () => 255);
    await focusCanvas(s);
    await s.key("i", 73);
    const white = await traceReady(s);
    assert.equal(white, "0 paths at this Threshold: too low.");
    await press(s, "Enter", 13);
    await sleep(500);
    assert.ok(!(await rowInfo(s)).some(([n]) => /Trace of/.test(n ?? "")), "Enter creates nothing for 0 paths");
    await focusCanvas(s);
    await s.key("Escape", 27);
    await importImage(s, "black.png", () => 0);
    await focusCanvas(s);
    await s.key("i", 73);
    const black = await traceReady(s);
    assert.equal(black, "1 path covering the whole image.");
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await waitFor("the group", async () => ((await rowInfo(s)).some(([n]) => /Trace of black/.test(n ?? "")) ? true : null));
  } finally {
    await close();
  }
});

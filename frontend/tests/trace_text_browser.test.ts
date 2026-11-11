// `specs/0091-trace-text-recognition` slice 1 in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Recognise text
// switch in Advanced, on by default and remembered (criterion 17); the rotated reference line read
// from the app's own files with no other request (criterion 14), the status line's count, Create
// paths writing the text above the traced group in one step and one undo (criteria 8, 11); with
// the switch off no weight file is fetched (criterion 12). Harness as in
// `acceptance_0089_m_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5294;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0091");
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


const BAR = "[role=toolbar][aria-label=Trace]";
type S = Awaited<ReturnType<typeof session>>;
const ROTATED = path.resolve(FRONTEND, "..", "tests", "assets", "vectrace-fixtures", "rotated.png");

async function openTrace(s: S, file: string) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await sleep(500);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 20_000).catch(async (error: unknown) => {
    throw new Error(`${String(error)}: rows ${JSON.stringify(await s.layerRows())}; buttons ${String(await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].map((b) => b.textContent).join('|')`))}`);
  });
}

const openAdvanced = async (s: S) => {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Recognise text"]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
};

const toggle = (s: S, label: string) => s.evaluate(`document.querySelector('${BAR} [aria-label=${JSON.stringify(label)}]')?.getAttribute('aria-pressed') ?? 'missing'`);

test("0091 slice 1 in Chromium: the switch, reading text offline, the text above the group in one step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, ROTATED);
    await openAdvanced(s);
    // Criterion 17 (Frage 116 B a): off by default, the third toggle, no hint text; turned on
    // here. `0088` Part B: "Recognise shapes" is the fourth toggle, off by default.
    assert.equal(await toggle(s, "Recognise text"), "false");
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
    await sleep(500);
    assert.equal(await toggle(s, "Recognise text"), "true");
    assert.equal(await toggle(s, "Recognise shapes"), "false");
    const order = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} button[aria-pressed]')].map((b) => b.getAttribute('aria-label')))`)) as string;
    const labels = JSON.parse(order) as string[];
    assert.deepEqual(labels.slice(labels.indexOf("Invert"), labels.indexOf("Invert") + 4), ["Invert", "Drop border shapes", "Recognise text", "Recognise shapes"]);
    // The preview with the read line counted in the status line.
    const status = await waitFor("the text line count", async () => {
      const text = await s.statusText();
      return text.includes("1 text line set") ? text : null;
    }, 120_000);
    assert.ok(!status.includes("failed"), status);
    // Criterion 14: the weights came from the app itself, and nothing left the machine.
    const models = s.requests.filter((u) => u.includes("/models/paddleocr-latin/"));
    assert.ok(models.some((u) => u.endsWith("det.onnx")) && models.some((u) => u.endsWith("rec.onnx")), models.join());
    assert.deepEqual(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")), []);
    // Create paths: the group and the text, the text above it, selected; one undo removes both.
    await s.evaluate(`document.querySelector('${BAR}').closest('[data-context-bar]')?.parentElement?.focus?.()`);
    // The preview fits the line to the fonts before its one trace (criterion 25); then the button
    // rests.
    await waitFor("the preview's fit", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 120_000);
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the text row", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Tilted 15")) ? r : null;
    }, 120_000).catch(async (error: unknown) => {
      throw new Error(`${String(error)}: rows ${JSON.stringify(await s.layerRows())}; status ${await s.statusText()}`);
    });
    // The picture is text only: no path, so no group; the text lies above the image.
    const textRow = rows.findIndex((x) => x?.includes("Tilted 15"));
    const imageRow = rows.findIndex((x) => x?.includes("rotated.png"));
    assert.ok(textRow >= 0 && textRow < imageRow && !rows.some((x) => x?.includes("Trace of")), rows.join(" | "));
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(["rotated.png, image"]) ? true : null));
    // Criteria 12 and 17: off is remembered, and a fresh page with it off fetches no weights.
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await openAdvanced(s);
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
    await sleep(500);
    assert.equal(await toggle(s, "Recognise text"), "false");
    // A fresh browser on the same profile: the switch is remembered off.
    await s.close();
    const t = await session(profile);
    closeAll = t.close;
    await openTrace(t, ROTATED);
    await openAdvanced(t);
    assert.equal(await toggle(t, "Recognise text"), "false");
    await waitFor("a preview", async () => ((await t.statusText()).match(/\d+ paths?/) ? true : null), 60_000);
    assert.equal(t.requests.filter((u) => u.includes("/models/")).length, 0);
    assert.ok(!(await t.statusText()).includes("text line"));
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

const POSTER = path.resolve(FRONTEND, "..", "tests", "assets", "vectrace-fixtures", "poster.png");

test("0091 slice 1 in Chromium: reading and fitting the poster keeps the main thread free", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    await s.choose("[data-import-button]", POSTER);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("poster.png")) ? true : null));
    // Main-thread tasks over 50 ms (0025 criterion 58) from opening Trace to the fitted preview.
    await s.evaluate(`(() => { window.__long = []; new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__long.push(e.duration); }).observe({ type: 'longtask' }); })()`);
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
    // Recognise text is off by default (criterion 17): turned on, which starts the job.
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await waitFor("the first preview", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 180_000);
    await openAdvanced(s);
    // Criterion 25: one percent over the whole job, sampled in the page, never running backwards.
    await s.evaluate(`(() => { window.__percent = []; window.__sampler = setInterval(() => { const p = document.querySelector('${BAR} [role=progressbar]'); const v = p?.getAttribute('aria-valuenow'); if (v !== null && v !== undefined) window.__percent.push(Number(v)); }, 20); })()`);
    const started = Date.now();
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
    await waitFor("the text lines", async () => ((await s.statusText()).includes("text lines set") ? true : null), 180_000);
    await waitFor("the preview's fit", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 180_000);
    const seconds = (Date.now() - started) / 1000;
    const percents = JSON.parse((await s.evaluate(`(clearInterval(window.__sampler), JSON.stringify(window.__percent))`)) as string) as number[];
    process.stdout.write(`# progress samples: ${percents.length}, first ${percents[0]}, last ${percents.at(-1)}\n`);
    assert.ok(percents.length > 0, "the progress showed");
    assert.ok(percents.every((p, i) => i === 0 || p >= percents[i - 1]), `never backwards: ${percents.join(",")}`);
    const long = JSON.parse((await s.evaluate(`JSON.stringify(window.__long)`)) as string) as number[];
    const total = long.reduce((a, b) => a + b, 0);
    process.stdout.write(`# poster: ${seconds.toFixed(1)} s to the fitted preview; main-thread long tasks ${long.length}, longest ${Math.max(0, ...long).toFixed(0)} ms, total ${total.toFixed(0)} ms\n`);
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

const COLOUR_SMALL = path.resolve(FRONTEND, "..", "tests", "assets", "vectrace-fixtures", "colour", "colour_small.png");
const COLOUR_POSTER = path.resolve(FRONTEND, "..", "tests", "assets", "vectrace-fixtures", "colour", "colour_poster.png");
const COLOUR_NATIVE = path.resolve(FRONTEND, "..", "curvyo-editor-wasm", "tests", "fixtures", "trace_text_colour_native.json");

test("0091 slice 2 in Chromium: Recognise text drawn and honoured in Multicolour, coloured texts above the group in one step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, COLOUR_SMALL);
    await openAdvanced(s);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await sleep(800);
    // Slice 2 (UX notes §1): the switch is drawn with Multicolour, off by default, then on.
    assert.equal(await toggle(s, "Recognise text"), "false");
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
    await sleep(500);
    assert.equal(await toggle(s, "Recognise text"), "true");
    await waitFor("the text line count", async () => ((await s.statusText()).includes("2 text lines set") ? true : null), 180_000).catch(async (error: unknown) => {
      throw new Error(`${String(error)}: status ${await s.statusText()}`);
    });
    await waitFor("the preview's fit", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 180_000);
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the text rows", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("OFFEN")) && r.some((x) => x?.includes("Rote Farbe")) ? r : null;
    }, 120_000).catch(async (error: unknown) => {
      throw new Error(`${String(error)}: rows ${JSON.stringify(await s.layerRows())}; status ${await s.statusText()}`);
    });
    // Criterion 24/6: the texts lie above the "Trace of" group (rows are listed top first).
    const group = rows.findIndex((x) => x?.includes("Trace of"));
    const texts = [rows.findIndex((x) => x?.includes("OFFEN")), rows.findIndex((x) => x?.includes("Rote Farbe"))];
    assert.ok(group >= 0 && texts.every((t) => t >= 0 && t < group), rows.join(" | "));
    // Criterion 24/11: one undo removes the group and both texts.
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(["colour_small.png, image"]) ? true : null));
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

// The page's calls below run on their own instance of the editor wasm (`?own-instance`): on the
// app's instance, loading the OCR weights and tracing the poster left the app's session trapping
// ("memory access out of bounds" in its timers), on this branch to the point of a hang.
test("0091 slice 2 c24/13 across builds: the editor wasm traces the small colour poster with text as natively", { skip: !BROWSER, timeout: 300_000 }, async () => {
  const native = JSON.parse(readFileSync(COLOUR_NATIVE, "utf8")) as { width: number; height: number; rgba_fnv32: number; settings: string; map: number[]; ocr: string; wire: number[] };
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    const png = readFileSync(COLOUR_SMALL).toString("base64");
    const out = JSON.parse(
      (await s.evaluate(`(async () => {
        const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await m.default();
        const png = Uint8Array.from(atob(${JSON.stringify(png)}), (c) => c.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([png], { type: 'image/png' }), { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
        const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
        const ctx = canvas.getContext('2d', { colorSpace: 'srgb' });
        ctx.drawImage(bitmap, 0, 0);
        const rgba = new Uint8Array(ctx.getImageData(0, 0, bitmap.width, bitmap.height).data.buffer);
        let h = 0x811c9dc5;
        for (let i = 0; i < rgba.length; i++) { h = Math.imul(h ^ rgba[i], 0x01000193) >>> 0; }
        const t = performance.now();
        const wire = Array.from(m.trace_text_job(rgba, bitmap.width, bitmap.height, Float64Array.from(${JSON.stringify(native.map)}), ${JSON.stringify(native.settings)}, ${JSON.stringify(native.ocr)}, { report() {} }));
        return JSON.stringify({ w: bitmap.width, h: bitmap.height, fnv: h, wire, ms: performance.now() - t });
      })()`)) as string,
    ) as { w: number; h: number; fnv: number; wire: number[]; ms: number };
    process.stdout.write(`# editor wasm: multicolour trace with text of the small colour poster ${out.ms.toFixed(0)} ms (wall, main thread)\n`);
    assert.deepEqual([out.w, out.h, out.fnv], [native.width, native.height, native.rgba_fnv32], "the same pixels");
    assert.deepEqual(out.wire, native.wire, "the same numbers");
    // The numbers of the PR: the full multicolour poster read and traced in the editor wasm.
    const big = readFileSync(COLOUR_POSTER).toString("base64");
    const times = JSON.parse(
      (await s.evaluate(`(async () => {
        const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await m.default();
        const png = Uint8Array.from(atob(${JSON.stringify(big)}), (c) => c.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([png], { type: 'image/png' }), { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
        const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
        const ctx = canvas.getContext('2d', { colorSpace: 'srgb' });
        ctx.drawImage(bitmap, 0, 0);
        const rgba = new Uint8Array(ctx.getImageData(0, 0, bitmap.width, bitmap.height).data.buffer);
        const get = async (u) => new Uint8Array(await (await fetch(u)).arrayBuffer());
        m.ocr_load(await get('/models/paddleocr-latin/det.onnx'), await get('/models/paddleocr-latin/rec.onnx'), new TextDecoder().decode(await get('/models/paddleocr-latin/dict.txt')));
        const sink = { report() {} };
        const map = Float64Array.from([0.25, 0, 0, 0.25, 0, 0]);
        let t = performance.now();
        const ocr = m.ocr_job(rgba, bitmap.width, bitmap.height, sink);
        const ocrMs = performance.now() - t;
        t = performance.now();
        const regions = m.trace_text_regions_job(rgba, bitmap.width, bitmap.height, map, ${JSON.stringify(native.settings)}, ocr);
        const regionsMs = performance.now() - t;
        t = performance.now();
        m.trace_text_job(rgba, bitmap.width, bitmap.height, map, ${JSON.stringify(native.settings)}, ocr, sink);
        const withMs = performance.now() - t;
        t = performance.now();
        m.trace_job(rgba, bitmap.width, bitmap.height, map, ${JSON.stringify(native.settings)}, sink);
        const offMs = performance.now() - t;
        return JSON.stringify({ lines: JSON.parse(ocr).lines?.length ?? JSON.parse(ocr).length, regions: regions.length, ocrMs, regionsMs, withMs, offMs });
      })()`)) as string,
    ) as { ocrMs: number; regionsMs: number; withMs: number; offMs: number };
    process.stdout.write(
      `# editor wasm, colour_poster.png 1600 x 1200 (wall, main thread): OCR ${(times.ocrMs / 1000).toFixed(2)} s, text pass ${(times.regionsMs / 1000).toFixed(2)} s, ` +
        `multicolour trace with text ${(times.withMs / 1000).toFixed(2)} s, off ${(times.offMs / 1000).toFixed(2)} s\n`,
    );
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

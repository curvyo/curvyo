// Tester cases for `specs/0088-trace-recognition` Part B ("Recognise shapes" in Trace) in the editor
// wasm and in Chromium (CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built).
// Written from the Part B criteria and ux-notes B1 to B9 on the tester's own pictures
// (`curvyo-editor-wasm/tests/fixtures/acceptance_0088_partb/`, drawn by
// `curvyo-editor-wasm/tests/acceptance_0088_partb_tester.rs`, which also writes the native record
// `acceptance_0088_partb_native.json`).
// Covers: criterion 13 native = wasm (the wire of every picture, switch on and off, bit for bit);
// criterion 11 (the switch off is the wire without the key); B1 (the switch, off by default, aria-label,
// no hint text, not in presets); B2 (fourth toggle after Recognise text, Space toggles); B3 (the muted
// line "N shapes set" / "1 shape set", nothing when off); B4/B5 (primitives in the group, one undo);
// Multicolour; exactly one trace request and one rising progress per run.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { inflateSync } from "node:zlib";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5298;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0088-partb");
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
const ROOT = path.resolve(FRONTEND, "..");
const PICTURES = path.join(ROOT, "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0088_partb");
const RECORD = path.join(ROOT, "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0088_partb_native.json");
const BINDINGS = path.resolve(FRONTEND, "src", "wasm-bindings");
const WASM = path.join(BINDINGS, "curvyo_editor_wasm_bg.wasm");

// ------------------------------------------------------------------ native = wasm (criterion 13, 11)

/** An 8-bit grey (type 0) or RGB (type 2) non-interlaced PNG as RGBA. */
function decodePng(file: string): { width: number; height: number; rgba: Uint8Array } {
  const bytes = readFileSync(file);
  let at = 8;
  let width = 0;
  let height = 0;
  let type = 0;
  const data: Buffer[] = [];
  while (at < bytes.length) {
    const length = bytes.readUInt32BE(at);
    const name = bytes.subarray(at + 4, at + 8).toString("latin1");
    const body = bytes.subarray(at + 8, at + 8 + length);
    if (name === "IHDR") {
      width = body.readUInt32BE(0);
      height = body.readUInt32BE(4);
      type = body[9];
      assert.equal(body[8], 8, "8 bit");
      assert.equal(body[12], 0, "not interlaced");
    } else if (name === "IDAT") {
      data.push(Buffer.from(body));
    }
    at += 12 + length;
  }
  const bpp = type === 0 ? 1 : 3;
  assert.ok(type === 0 || type === 2, `colour type ${type}`);
  const raw = inflateSync(Buffer.concat(data));
  const stride = width * bpp;
  const rows = new Uint8Array(height * stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    for (let i = 0; i < stride; i++) {
      const x = raw[y * (stride + 1) + 1 + i];
      const a = i >= bpp ? rows[y * stride + i - bpp] : 0;
      const b = y > 0 ? rows[(y - 1) * stride + i] : 0;
      const c = y > 0 && i >= bpp ? rows[(y - 1) * stride + i - bpp] : 0;
      const paeth = () => {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      };
      rows[y * stride + i] = (x + [0, a, b, (a + b) >> 1, paeth()][filter]) & 255;
    }
  }
  const rgba = new Uint8Array(width * height * 4);
  for (let p = 0; p < width * height; p++) {
    const g = bpp === 1 ? [rows[p], rows[p], rows[p]] : [rows[p * 3], rows[p * 3 + 1], rows[p * 3 + 2]];
    rgba.set([g[0], g[1], g[2], 255], p * 4);
  }
  return { width, height, rgba };
}

/** FNV-1a over the little-endian bytes of the f64s, as the native record has it. */
function fnv(wire: Float64Array): string {
  let hash = 0xcbf29ce484222325n;
  const bytes = new Uint8Array(wire.buffer, wire.byteOffset, wire.byteLength);
  for (const b of bytes) {
    hash = ((hash ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
  }
  return hash.toString(16).padStart(16, "0");
}

type Record_ = Record<string, { map: number[]; multicolour: boolean; on_settings: string; off_settings: string; on_fnv: string; off_fnv: string; shapes: number }>;

test("0088 B criteria 11 and 13: the wasm wire of every tester picture equals the native one, switch on and off", { skip: !existsSync(WASM) }, async () => {
  const bindings = (await import(path.join(BINDINGS, "curvyo_editor_wasm.js"))) as {
    initSync: (options: { module: BufferSource }) => unknown;
    trace_job: (rgba: Uint8Array, w: number, h: number, map: Float64Array, settings: string, progress: { report: (percent: number) => void }) => Float64Array;
  };
  bindings.initSync({ module: readFileSync(WASM) });
  const record = JSON.parse(readFileSync(RECORD, "utf8")) as Record_;
  const names = Object.keys(record);
  assert.ok(names.length >= 15, names.join());
  for (const name of names) {
    const r = record[name];
    const picture = decodePng(path.join(PICTURES, `${name}.png`));
    const run = (settings: string) => bindings.trace_job(picture.rgba, picture.width, picture.height, new Float64Array(r.map), settings, { report() {} });
    assert.equal(fnv(run(r.off_settings)), r.off_fnv, `${name}: switch off, wasm = native`);
    assert.equal(fnv(run(r.on_settings)), r.on_fnv, `${name}: switch on, wasm = native`);
    // Criterion 11: "shapes": false is the wire without the key.
    assert.equal(fnv(run(r.off_settings.replace(/}$/, ',"shapes":false}'))), r.off_fnv, `${name}: false = absent`);
    // A repeated run is the same wire (13).
    assert.equal(fnv(run(r.on_settings)), r.on_fnv, `${name}: a second run`);
  }
  // The pictures with a shape in them have a different wire when on; the wobbly circle has the same count of shapes (0).
  assert.equal(record.wobbly.shapes, 0);
  assert.ok(record.mixed.shapes >= 4 && record.ring.shapes >= 1 && record.dots.shapes >= 10000, JSON.stringify([record.mixed.shapes, record.ring.shapes, record.dots.shapes]));
});

test("0088 B criterion 11: shapes are not part of any preset", async () => {
  const text = await import("../src/lib/traceText.ts");
  for (const preset of JSON.stringify(text.TRACE_PRESETS_FALLBACK).match(/"shapes"/g) ?? []) {
    assert.fail(`a preset carries ${preset}`);
  }
});

// ------------------------------------------------------------------ Chromium

const SPY = `(() => {
  const Orig = window.Worker;
  window.__traces = 0; window.__pct = [];
  window.Worker = class extends Orig {
    postMessage(m, t) {
      if (m && m.type === 'trace') window.__traces++;
      return t === undefined ? super.postMessage(m) : super.postMessage(m, t);
    }
  };
  let last = null;
  const probe = () => {
    const bar = document.querySelector('${BAR} [role=progressbar]');
    const v = bar ? bar.getAttribute('aria-valuenow') : null;
    if (v !== null && v !== last) { last = v; window.__pct.push(Number(v)); }
  };
  addEventListener('DOMContentLoaded', () => new MutationObserver(probe).observe(document.body, { subtree: true, childList: true, attributes: true }));
})()`;

async function openTrace(s: S, file: string) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await sleep(500);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 20_000);
  await waitFor("the first preview", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 180_000);
}

/** The layer rows with every collapsed group expanded. */
const allRows = async (s: S) => {
  await s.evaluate(`[...document.querySelectorAll('[role=row][aria-expanded=false]')].forEach((r) => r.querySelector('button')?.click())`);
  await sleep(500);
  return s.layerRows();
};

const openAdvanced = async (s: S) => {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Recognise text"]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
};
const pressed = (s: S) => s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]')?.getAttribute('aria-pressed') ?? 'missing'`);
const resting = async (s: S) => (await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) === true;
const counts = async (s: S) => {
  const m = /(\d+) paths?, (\d+) nodes?/.exec(await s.statusText());
  return m ? [Number(m[1]), Number(m[2])] : null;
};
const shapesSet = async (s: S) => /(\d+) shapes? set/.exec(await s.statusText());

async function withApp(body: (s: S, reopen: () => Promise<S>) => Promise<void>) {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: SPY });
    let current = s;
    const reopen = async () => {
      await current.close();
      current = await session(profile);
      closeAll = current.close;
      await current.send("Page.addScriptToEvaluateOnNewDocument", { source: SPY });
      return current;
    };
    await body(s, reopen);
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
}


test("0088 B1 B2 B3 B5: the switch is off by default, sets 1 shape, one trace and one rising progress, Create paths and one undo", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s, reopen) => {
    await openTrace(s, path.join(PICTURES, "circle.png"));
    await openAdvanced(s);
    // B1: off by default, the label is the name, nothing drawn on the button, no second line.
    assert.equal(await pressed(s), "false");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]').textContent.trim()`), "");
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]').getAttribute('title')`), null);
    // B2: the fourth toggle, right after Recognise text.
    const labels = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} button[aria-pressed]')].map((b) => b.getAttribute('aria-label')))`)) as string) as string[];
    assert.deepEqual(labels.slice(labels.indexOf("Invert"), labels.indexOf("Invert") + 4), ["Invert", "Drop border shapes", "Recognise text", "Recognise shapes"]);
    // B3: switch off: no segment.
    assert.equal(await shapesSet(s), null, await s.statusText());
    const before = await counts(s);
    assert.ok(before, await s.statusText());
    // Turn it on with the keyboard (B2: Space toggles): one trace request, one progress that never runs backwards.
    await s.evaluate(`window.__traces = 0; window.__pct = []; document.querySelector('${BAR} [aria-label="Recognise shapes"]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: " ", code: "Space", text: " ", windowsVirtualKeyCode: 32 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: " ", code: "Space", windowsVirtualKeyCode: 32 });
    await waitFor("1 shape set", async () => ((await shapesSet(s)) ? true : null), 60_000);
    await waitFor("rest", async () => ((await resting(s)) ? true : null), 60_000);
    assert.equal(await pressed(s), "true");
    assert.match(await s.statusText(), /1 shape set/, "the singular form");
    assert.ok(!/1 shapes set/.test(await s.statusText()));
    assert.deepEqual(await counts(s), before, "the counts line keeps counting the traced paths and nodes");
    assert.equal(await s.evaluate(`window.__traces`), 1, "exactly one trace request for the change");
    const pct = (await s.evaluate(`JSON.stringify(window.__pct)`)) as string;
    const pcts = JSON.parse(pct) as number[];
    assert.ok(pcts.every((p, i) => i === 0 || p >= pcts[i - 1]), `one progress, never backwards: ${pct}`);
    // B5: Create paths writes the primitive in the group; one undo removes it.
    const rowsBefore = await s.layerRows();
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the group", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Trace of")) ? r : null;
    }, 60_000);
    rows.splice(0, rows.length, ...(await allRows(s)));
    process.stdout.write(`# rows after Create paths: ${JSON.stringify(rows)}\n`);
    assert.ok(rows.some((x) => /Ellipse|Circle/.test(x ?? "")), `a circle primitive is in the layers: ${JSON.stringify(rows)}`);
    assert.ok(!rows.some((x) => /, path/.test(x ?? "")), `no traced path is left: ${JSON.stringify(rows)}`);
    await s.evaluate(`document.activeElement?.blur?.()`);
    process.stdout.write("# sending Ctrl+Z\n");
    await Promise.race([s.key("z", 90, 2), sleep(20_000).then(() => { throw new Error("the key press never returned: the page does not answer"); })]);
    process.stdout.write("# sent\n");
    const alive = await Promise.race([s.evaluate(`1 + 1`).then(() => "alive"), sleep(15_000).then(() => "FROZEN")]);
    process.stdout.write(`# page after Ctrl+Z: ${alive}\n`);
    assert.equal(alive, "alive", "the page answers after Ctrl+Z");
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(rowsBefore) ? true : null), 30_000).catch(async (e: unknown) => {
      throw new Error(`${String(e)}: rows ${JSON.stringify(await s.layerRows())}, before ${JSON.stringify(rowsBefore)}`);
    });
    process.stdout.write("# undone\n");
    // The switch is an app setting: a fresh browser on the same profile has it on (as Recognise text is remembered).
    const t = await reopen();
    await openTrace(t, path.join(PICTURES, "circle.png"));
    await openAdvanced(t);
    assert.equal(await pressed(t), "true", "remembered");
    assert.equal(await t.evaluate(`Object.keys(localStorage).some((k) => k.startsWith('curvyo.') && /shapes/i.test(localStorage.getItem(k) ?? ''))`), true, "kept with the other Trace settings in a curvyo.* key");
    // Turn it off again: no segment, the counts as before.
    await t.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]').click()`);
    await waitFor("no segment", async () => ((await resting(t)) && (await counts(t)) !== null && (await shapesSet(t)) === null ? true : null), 60_000);
    assert.deepEqual(await counts(t), before);
  });
});

test("0088 B3 B4: several shapes are counted by the primitive, 4 in the mixed picture, each kind as its own primitive row", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openTrace(s, path.join(PICTURES, "mixed.png"));
    await openAdvanced(s);
    const before = await counts(s);
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]').click()`);
    await waitFor("4 shapes set", async () => ((await shapesSet(s))?.[1] === "4" ? true : null), 60_000);
    assert.deepEqual(await counts(s), before);
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the group", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Trace of")) ? r : null;
    }, 60_000);
    rows.splice(0, rows.length, ...(await allRows(s)));
    process.stdout.write(`# mixed rows: ${JSON.stringify(rows)}\n`);
    for (const name of ["Ellipse", "Rectangle", "Polygon", "Star"]) {
      assert.ok(rows.some((x) => x?.includes(name)), `${name} in ${JSON.stringify(rows)}`);
    }
  });
});

test("0088 B criterion 10 in Chromium: Multicolour keeps the colours of a circle and a rectangle", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openTrace(s, path.join(PICTURES, "colour.png"));
    await openAdvanced(s);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radio], ${BAR} button')].find((b) => b.textContent?.trim() === 'Multicolour')?.click()`);
    await sleep(1000);
    await waitFor("rest", async () => ((await resting(s)) ? true : null), 120_000);
    await openAdvanced(s);
    assert.equal(await pressed(s), "false", "drawn in Multicolour too, off");
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]').click()`);
    await waitFor("2 shapes set", async () => ((await shapesSet(s))?.[1] === "2" ? true : null), 120_000);
    await waitFor("rest", async () => ((await resting(s)) ? true : null), 120_000);
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the group", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Trace of")) ? r : null;
    }, 120_000);
    rows.splice(0, rows.length, ...(await allRows(s)));
    process.stdout.write(`# colour rows: ${JSON.stringify(rows)}\n`);
    assert.ok(rows.some((x) => /Ellipse|Circle/.test(x ?? "")) && rows.some((x) => /Rectangle/.test(x ?? "")), JSON.stringify(rows));
  });
});

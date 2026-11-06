// Tester cases for `specs/0094-masked-composite-trace` slice 1 in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written
// from the specification and the UX notes before the implementation was read: the Mask item after
// the View pill with Inside on entry (criteria 1, 17), Inside and Outside, Create paths with the
// mask path untouched and one undo (12), the mixed selection row (14), no hint text, and the editor
// wasm against the native record of `curvyo-editor-wasm/tests/acceptance_0094_tester.rs` on two
// own instances (15 across builds and workers). Harness as in `acceptance_0089_tidyup_tester.test.ts`;
// page-side wasm imports use their own instances (`?own-instance`), never the app's.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { ANALYSIS_LINE_JS, ANALYSIS_STATE_JS } from "./browser/traceStatus.ts";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5394;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0094-tester");
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
const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
type Shot = ReturnType<typeof decodePng>;
const at = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};

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

const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);
const settledCounts = (s: S) => waitFor("a trace result", async () => (/(^|\. )(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);

const BRIGHTNESS_BAR = ["About Trace", "Preset", "Pick mask paths", "Mask", "Clear mask", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "More trace options", "Trace"];

// ------------------------------------------------------------------ 0094 fixtures and helpers
/** M1 of the spec at `n` px (the tester's `support_0094` arithmetic, pixel centres in M1's own 1000 px frame). */
function m1Ink(u: number, v: number): "paper" | "disc" | "square" | "bar" | "grey" {
  if (Math.abs(u - 900) <= 50 && Math.abs(v - 100) <= 50) return "grey";
  if ((u - 250) * (u - 250) + (v - 500) * (v - 500) <= 150 * 150) return "disc";
  if (Math.abs(u - 750) <= 150 && Math.abs(v - 500) <= 150) return "square";
  if (v >= 480 && v <= 520) return "bar";
  return "paper";
}
const M1_PNG = rgbPng(400, 400, (x, y) => {
  const ink = m1Ink((x + 0.5) * 2.5, (y + 0.5) * 2.5);
  return ink === "paper" ? [255, 255, 255] : ink === "grey" ? [128, 128, 128] : [0, 0, 0];
});
const MASKED_BAR = ["About Trace", "Preset", "Pick mask paths", "Mask", "Clear mask", "Method", "Threshold", "Smooth", "Tolerance", "Ignore below", "Fewer trace options", "Trace"];
const NATIVE = path.resolve(FRONTEND, "..", "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0094_tester_native.json");

/** The screen box of the picture: the bar of M1 is the longest dark horizontal run on screen and
 * spans the picture's full width at its middle; the picture is square. */
async function pictureBox(s: S) {
  const img = await shot(s);
  const [cx0, , cw] = await canvasRect(s);
  const right = Math.min(img.width, cx0 + cw) - 1;
  let best = { len: 0, x0: 0, x1: 0, y: 0 };
  for (let y = 0; y < img.height; y++) {
    // A run may have gaps of up to 6 px (a selection mark drawn over the picture).
    let start = -1;
    let last = -100;
    for (let x = 0; x < img.width; x++) {
      const [r, g, b] = at(img, x, y);
      if (r < 40 && g < 40 && b < 40) {
        if (x - last > 7) start = x;
        last = x;
        if (x - start + 1 > best.len) best = { len: x - start + 1, x0: start, x1: x, y };
      }
    }
  }
  // The run's end can stop at a mark drawn over the picture; its right edge is the row's last dark pixel.
  for (let x = right; x > best.x1; x--) {
    const [r, g, b] = at(img, x, best.y);
    if (r < 40 && g < 40 && b < 40) {
      best.x1 = x;
      break;
    }
  }
  const w = best.x1 - best.x0;
  return { x0: best.x0, x1: best.x1, y0: best.y - w / 2, y1: best.y + w / 2, cx: (best.x0 + best.x1) / 2, cy: best.y };
}

async function drag(s: S, from: [number, number], to: [number, number]) {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: from[0], y: from[1] });
  await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: from[0], y: from[1], button: "left", buttons: 1, clickCount: 1 });
  for (let i = 1; i <= 8; i++) {
    const x = from[0] + ((to[0] - from[0]) * i) / 8;
    const y = from[1] + ((to[1] - from[1]) * i) / 8;
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, button: "left", buttons: 1 });
  }
  await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: to[0], y: to[1], button: "left", buttons: 0, clickCount: 1 });
  await sleep(500);
}

const shiftClick = async (s: S, x: number, y: number) => {
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, modifiers: 8 });
  }
  await sleep(600);
};

/** Imports M1, draws a rectangle over `part` of it ("left" half, or "outside" right of it), selects both with Ctrl+A and opens Trace. */
async function maskedOn(s: S, part: "left" | "outside" = "left") {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  if (part === "left") {
    await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
  } else {
    await drag(s, [box.x1 + 40, box.cy - 20], [box.x1 + 90, box.cy + 20]);
  }
  await s.key("s", 83);
  await focusCanvas(s);
  await s.key("a", 65, 2);
  await waitFor("two objects selected", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  return box;
}
const maskState = async (s: S) =>
  (await s.evaluate(`(() => { const g = document.querySelector('${BAR} [role=radiogroup][aria-label=Mask]'); if (!g) return 'absent';
    return [...g.querySelectorAll('[role=radio]')].map((r) => r.textContent.trim() + '=' + r.getAttribute('aria-checked')).join(','); })()`)) as string;
const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
// §24.5: the muted "Inside 1 path" segment is announced by the hidden region only.
const advText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
/** The mask paths of the active part, as the hidden region announces them ("Inside 2 paths"); 0 without. */
const maskPaths = async (s: S) => Number((await advText(s)).match(/(?:Inside|Outside) (\d+) paths?/)?.[1] ?? 0);

// ------------------------------------------------------------------ 0094 cases

test("0094 c1, c17 (§24.4): the Mask group follows Preset with Inside on entry; Outside changes the trace; leaving the tool forgets it; an image alone has the same bar with no mask paths", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await maskedOn(s);
    assert.deepEqual(await barControls(s), MASKED_BAR, "the Mask group directly after Preset");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "Inside is selected");
    await settledCounts(s);
    const inside = await advancedStatus(s);
    assert.match(await advText(s), /Inside 1 path/, "the muted status segment");
    await clickMask(s, "Outside");
    await sleep(500);
    assert.equal(await maskState(s), "Inside=false,Outside=true");
    await waitFor("the Outside result", async () => ((await advancedStatus(s)) !== inside && /\d+ paths?/.test(await advancedStatus(s)) ? true : null), 30_000);
    assert.match(await advText(s), /Outside 1 path/);
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
    // c17: the tool left and entered again starts with Inside.
    await focusCanvas(s);
    await s.key("s", 83);
    await waitFor("the Trace bar closes", async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await sleep(800);
    assert.equal(await maskState(s), "Inside=true,Outside=false", "Inside again on entry");
  });
  await withApp(async (s) => {
    await openOn(s, "alone.png", M1_PNG, false);
    assert.equal(await maskPaths(s), 0, "an image alone has no mask paths");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "§24.4: the Mask group is there without mask paths, Inside selected");
    assert.deepEqual(await barControls(s), BRIGHTNESS_BAR, "today's bar");
    await setAdvanced(s, true);
    await settledCounts(s);
    assert.doesNotMatch(await advText(s), /Inside|Outside/);
  });
});

test("0094 c12, c13: Create paths with a mask makes one group, leaves the rectangle, one undo restores; Shift+click on the path removes and adds it", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    const box = await maskedOn(s);
    await settledCounts(s);
    const before = await s.layerRows();
    // c13: Shift+click on the rectangle's edge outside the picture removes it, again adds it.
    const edge: [number, number] = [box.x0 - 30, box.cy];
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item goes", async () => ((await maskPaths(s)) === 0 ? true : null), 10_000);
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item comes back", async () => ((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    const canvas = await canvasRect(s);
    await shiftClick(s, canvas[0] + canvas[2] - 20, canvas[1] + canvas[3] - 20);
    assert.ok((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true"), "Shift+click on empty canvas changes nothing");
    await settledCounts(s);
    await s.clickSel(`${BAR} [data-trace-slot] button, ${BAR} button[data-trace-slot]`);
    await waitFor("the trace group", async () => ((await s.layerRows()).some((r) => r?.startsWith("Trace of")) ? true : null), 30_000);
    const after = await s.layerRows();
    for (const row of before) {
      assert.ok(after.includes(row), `${row} is still there`);
    }
    assert.equal(after.filter((r) => r?.startsWith("Trace of")).length, 1, "one group");
    assert.doesNotMatch(await s.statusText(), /created|done|success/i, "no success notice");
    await focusCanvas(s);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (!(await s.layerRows()).some((r) => r?.startsWith("Trace of")) ? true : null), 10_000);
    assert.deepEqual(await s.layerRows(), before, "one undo restores the rows");
  });
});

test("0094 c7: a mask outside the picture says so, dims Trace and Enter creates nothing", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await maskedOn(s, "outside");
    await waitFor("the outside row", async () => ((await s.evaluate(`document.querySelector('${BAR}').textContent`)) as string).includes("0 paths: the mask is outside the image.") ? true : null, 20_000);
    const button = await traceButton(s);
    assert.equal(button?.disabled, "true", "Trace is dimmed");
    const rows = await s.layerRows();
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await sleep(1500);
    assert.deepEqual(await s.layerRows(), rows, "nothing is created");
  });
});

test("0094 c14: a second image Ctrl+clicked in Layers next to the mask path gives the mixed row and dims Trace", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    // The second picture first, moved away from the centre, then M1 and its mask on top.
    const file = path.join(SCRATCH, "second.png");
    writeFileSync(file, rgbPng(80, 80, () => [40, 40, 40]));
    await s.tab("Layers");
    await s.choose("[data-import-button]", file);
    await waitFor("the second image", async () => ((await s.layerRows()).some((r) => r?.includes("second.png")) ? true : null));
    const canvas = await canvasRect(s);
    const centre: [number, number] = [canvas[0] + canvas[2] / 2, canvas[1] + canvas[3] / 2];
    await drag(s, centre, [canvas[0] + 60, canvas[1] + 60]);
    const file2 = path.join(SCRATCH, "m1.png");
    writeFileSync(file2, M1_PNG);
    await s.choose("[data-import-button]", file2);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
    await sleep(800);
    const box = await pictureBox(s);
    const h = box.y1 - box.y0;
    await focusCanvas(s);
    await press(s, "Escape", 27);
    await s.key("r", 82);
    await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
    await s.key("s", 83);
    // Select M1 and the rectangle only: click the rectangle's edge, Shift+click the picture.
    await s.click(box.x0 - 30, box.cy);
    await shiftClick(s, box.x1 - 5, box.cy);
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await sleep(800);
    assert.ok((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true"), "image and rectangle make a masked trace");
    // Add the second picture with a Ctrl+click on its Layers row; the Trace tool stays.
    const row = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('[role=row][aria-level]')].find((e) => e.getAttribute('aria-label') === 'second.png, image').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })())`)) as string) as [number, number];
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await s.send("Input.dispatchMouseEvent", { type, x: row[0], y: row[1], button: "left", clickCount: 1, modifiers: 2 });
    }
    await sleep(600);
    await waitFor("the mixed row", async () => ((await s.evaluate(`document.querySelector('${BAR}').textContent`)) as string).includes("Use Object to path for text; one image at a time.") ? true : null, 10_000);
    // Criterion 14 as amended (re-test 2026-11-05): the row stands in place of the bar's
    // controls with no Trace button, as the "not an image" row.
    assert.equal(await traceButton(s), null, "no Trace button");
    const controls = await barControls(s);
    for (const name of ["Mask", "Method", "Threshold", "Trace"]) {
      assert.ok(!controls.includes(name), `no ${name} with the row: ${JSON.stringify(controls)}`);
    }
    assert.equal(await maskPaths(s), 0, "no mask paths for a mixed selection");
    const rows = await s.layerRows();
    await focusCanvas(s);
    await press(s, "Enter", 13);
    await sleep(1000);
    assert.deepEqual(await s.layerRows(), rows, "Enter creates nothing");
  });
});

test("0094 c15 across builds and workers: two own editor wasm instances trace the masked M1 as the native record", { skip: !BROWSER, timeout: 300_000 }, async () => {
  const native = JSON.parse(readFileSync(NATIVE, "utf8")) as { n: number; map: number[]; cases: { palette: string; rgba_fnv32: number; settings: string; wire: number[] }[] };
  await withApp(async (s) => {
    const out = JSON.parse(
      (await s.evaluate(`(async () => {
        const ink = ${m1Ink.toString()};
        const draw = (n, palette) => {
          const rgba = new Uint8Array(n * n * 4);
          const scale = 1000 / n;
          const colours = palette === 'colour'
            ? { paper: [255, 255, 255], disc: [200, 30, 30], square: [30, 60, 200], bar: [30, 160, 60], grey: [128, 128, 128] }
            : { paper: [255, 255, 255], disc: [0, 0, 0], square: [0, 0, 0], bar: [0, 0, 0], grey: [128, 128, 128] };
          for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
            const c = colours[ink((x + 0.5) * scale, (y + 0.5) * scale)];
            rgba.set([c[0], c[1], c[2], 255], (y * n + x) * 4);
          }
          return rgba;
        };
        const fnv = (b) => { let h = 0x811c9dc5; for (let i = 0; i < b.length; i++) { h = Math.imul(h ^ b[i], 0x01000193) >>> 0; } return h; };
        const a = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await a.default();
        const b = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance-2');
        await b.default();
        const cases = ${JSON.stringify(native.cases.map((c) => ({ palette: c.palette, settings: c.settings })))};
        const map = Float64Array.from(${JSON.stringify(native.map)});
        return JSON.stringify(cases.map((c) => {
          const rgba = draw(${native.n}, c.palette);
          const one = Array.from(a.trace_job(rgba, ${native.n}, ${native.n}, map, c.settings, { report() {} }));
          const two = Array.from(b.trace_job(rgba, ${native.n}, ${native.n}, map, c.settings, { report() {} }));
          return { fnv: fnv(rgba), one, two, same: a !== b };
        }));
      })()`)) as string,
    ) as { fnv: number; one: number[]; two: number[]; same: boolean }[];
    assert.equal(out.length, native.cases.length);
    out.forEach((got, i) => {
      assert.ok(got.same, "two module instances");
      assert.equal(got.fnv, native.cases[i].rgba_fnv32, `case ${i}: the same pixels`);
      assert.deepEqual(got.one, native.cases[i].wire, `case ${i}: wasm = native`);
      assert.deepEqual(got.two, got.one, `case ${i}: two workers = one`);
    });
  });
});

// ------------------------------------------------------------------ re-test 2026-11-05, item 2

/** M1 with the right half's paper mid grey #A0A0A0 and the disc on the white left half in
 * #787878 (the bar black through it): the whole picture splits between the two papers, the right half between its paper
 * and the black square, so the automatic Thresholds differ widely. */
const GREY_RIGHT_PNG = rgbPng(400, 400, (x, y) => {
  const ink = m1Ink((x + 0.5) * 2.5, (y + 0.5) * 2.5);
  if (ink === "paper") return x >= 200 ? [160, 160, 160] : [255, 255, 255];
  // The bar stays black through the disc, so the picture's box is found by its bar.
  const v = (y + 0.5) * 2.5;
  if (ink === "disc" && (v < 480 || v > 520)) return [120, 120, 120];
  return ink === "grey" ? [128, 128, 128] : [0, 0, 0];
});
const field = (name: string) => `${BAR} [role=spinbutton][aria-label="${name}"]`;
const fieldValue = async (s: S, name: string) =>
  (await s.evaluate(`(() => { const f = document.querySelector('${field(name)}'); return f ? (f.getAttribute('aria-valuetext') ?? '').split(' ')[0] : null; })()`)) as string | null;
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${field(name)}').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const methodNow = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]')?.textContent ?? ''`)) as string;
const analysisLine = async (s: S) => (await s.evaluate(ANALYSIS_LINE_JS)) as string;

/** Imports `png`, draws a rectangle over its `half` reaching 30 px beyond the picture, then
 * selects the image alone (`both` false: a click on the picture's other half) or image and
 * rectangle (Ctrl+A), and opens Trace with Advanced open. The rectangle's outer edge point. */
async function halfMasked(s: S, png: Buffer, half: "left" | "right", both: boolean) {
  const file = path.join(SCRATCH, "half.png");
  writeFileSync(file, png);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("half.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  if (half === "right") {
    await drag(s, [box.cx, box.cy - 0.6 * h], [box.x1 + 30, box.cy + 0.6 * h]);
  } else {
    await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
  }
  await s.key("s", 83);
  const edge: [number, number] = half === "right" ? [box.x1 + 30, box.cy] : [box.x0 - 30, box.cy];
  if (both) {
    await focusCanvas(s);
    await s.key("a", 65, 2);
  } else {
    const w = box.x1 - box.x0;
    await s.click(half === "right" ? box.x0 + 0.1 * w : box.x1 - 0.1 * w, box.y0 + 0.1 * h);
  }
  await waitFor("the Trace button", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  await setAdvanced(s, true);
  await settledCounts(s);
  await sleep(1500);
  return edge;
}

/** Waits until the analysis row is settled (not "Analysing...") and the counts are shown. */
async function settledAnalysis(s: S) {
  await waitFor("the analysis", async () => {
    const kind = (await s.evaluate(ANALYSIS_STATE_JS)) as string;
    return kind !== "analysing" && kind !== "none" ? kind : null;
  }, 30_000);
  await settledCounts(s);
  await sleep(1500);
}

test("0094 item 2: a mask added with Shift+click re-reads the automatic Threshold from the region as a fresh masked trace does; removing it reads the whole picture again", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let fresh = "";
  let freshLine = "";
  await withApp(async (s) => {
    await halfMasked(s, GREY_RIGHT_PNG, "right", true);
    assert.ok((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true"));
    await settledAnalysis(s);
    fresh = (await fieldValue(s, "Threshold")) ?? `(no Threshold, method ${await methodNow(s)})`;
    freshLine = await analysisLine(s);
  });
  await withApp(async (s) => {
    const edge = await halfMasked(s, GREY_RIGHT_PNG, "right", false);
    assert.equal(await maskPaths(s), 0, "the image alone");
    await settledAnalysis(s);
    const whole = (await fieldValue(s, "Threshold")) ?? `(no Threshold, method ${await methodNow(s)})`;
    const wholeLine = await analysisLine(s);
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item", async () => ((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await settledAnalysis(s);
    const masked = (await fieldValue(s, "Threshold")) ?? `(no Threshold, method ${await methodNow(s)})`;
    console.log(JSON.stringify({ whole, wholeLine, masked, maskedLine: await analysisLine(s), fresh, freshLine }));
    assert.notEqual(fresh, whole, "fixture: the right half's automatic Threshold differs from the whole picture's");
    assert.equal(masked, fresh, "the new mask's Threshold is the region's, as a fresh masked trace reads it");
    assert.equal(await analysisLine(s), freshLine, "the analysis is the region's");
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item goes", async () => ((await maskPaths(s)) === 0 ? true : null), 10_000);
    await settledAnalysis(s);
    assert.equal(await fieldValue(s, "Threshold") ?? `(no Threshold, method ${await methodNow(s)})`, whole, "without the mask the whole picture's value again");
  });
});

// Re-test 2026-11-05: FAILS. A new mask forgets the analysis (`frontend/src/hooks/useTrace.ts:547`),
// and the fresh analysis is applied as if the maker had edited nothing (`useTrace.ts:292-306`:
// `edited` sees only a kept Threshold or an edit before the first analysis), so a Method the
// maker chose is replaced by the region's; 0089 (lead 2026-10-31) keeps a maker's edits.
test("0094 item 2: a Threshold the maker set and a Method the maker chose stay when a mask is added", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    const edge = await halfMasked(s, GREY_RIGHT_PNG, "right", false);
    await settledAnalysis(s);
    if ((await methodNow(s)) !== "Brightness") await pickMethod(s, "Brightness");
    await typeInto(s, "Threshold", "33");
    await settledCounts(s);
    assert.equal(await fieldValue(s, "Threshold"), "33");
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item", async () => ((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await settledAnalysis(s);
    assert.equal(await methodNow(s), "Brightness");
    assert.equal(await fieldValue(s, "Threshold"), "33", "the maker's Threshold stays");
  });
  await withApp(async (s) => {
    // The left half is black on white paper; the maker picks a method of their own first.
    const edge = await halfMasked(s, GREY_RIGHT_PNG, "left", false);
    await settledAnalysis(s);
    const before = await methodNow(s);
    const chosen = before === "Remove background" ? "Brightness" : "Remove background";
    await pickMethod(s, chosen);
    await settledCounts(s);
    assert.equal(await methodNow(s), chosen);
    await shiftClick(s, edge[0], edge[1]);
    await waitFor("the Mask item", async () => ((await maskPaths(s)) > 0 && (await maskState(s)).startsWith("Inside=true") ? true : null), 10_000);
    await settledAnalysis(s);
    console.log(JSON.stringify({ before, chosen, after: await methodNow(s), line: await analysisLine(s) }));
    assert.equal(await methodNow(s), chosen, "the maker's Method stays when the region is analysed");
  });
});

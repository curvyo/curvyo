// Chromium cases for `specs/0094-masked-composite-trace` slice 1b, Pick mask paths (criteria 34
// to 41 and 43; UX notes §11): arm, pick two paths, Inside / Outside appears, pick again removes,
// Clear mask, Escape disarms, leaving the tool disarms, a text and a second image are refused with
// the row, the Tab order and the group at 800 x 600. Set CURVYO_E2E_BROWSER=/path/to/chromium; the
// wasm bindings must be built. The harness is the one of `acceptance_0094_tester.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5395;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0094-pick");
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

const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${ADV} [data-trace-counts]')?.textContent ?? ''`)) as string;
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);
const settledCounts = (s: S) => waitFor("a trace result", async () => (/^(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);


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
const maskState = async (s: S) =>
  (await s.evaluate(`(() => { const g = document.querySelector('${BAR} [role=radiogroup][aria-label=Mask]'); if (!g) return 'absent';
    return [...g.querySelectorAll('[role=radio]')].map((r) => r.textContent.trim() + '=' + r.getAttribute('aria-checked')).join(','); })()`)) as string;
const advText = async (s: S) => (await s.evaluate(`document.querySelector('${ADV}')?.textContent ?? ''`)) as string;

// ------------------------------------------------------------------ slice 1b helpers

const PICK = `${BAR} button[aria-label="Pick mask paths"]`;
const CLEAR = `${BAR} button[aria-label="Clear mask"]`;
const REFUSED = "Use Object to path for text; one image at a time.";
const pressedState = async (s: S) => (await s.evaluate(`document.querySelector('${PICK}')?.getAttribute('aria-pressed') ?? 'absent'`)) as string;
const hasClear = async (s: S) => (await s.evaluate(`!!document.querySelector('${CLEAR}')`)) as boolean;
const refusedShown = async (s: S) => (await s.evaluate(`!!document.querySelector('${BAR} [data-trace-mask-refused]')`)) as boolean;
const canvasCursor = async (s: S) =>
  (await s.evaluate(`(() => { const c = [...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0];
    const e = c.closest('[style*="cursor"]'); return e ? e.style.cursor : ''; })()`)) as string;
const focusedName = async (s: S) => (await s.evaluate(`(() => { const e = document.activeElement; return e ? (e.getAttribute('aria-label') || e.textContent.trim()) : ''; })()`)) as string;
const traceRows = async (s: S) => (await s.layerRows()).filter((r) => r?.startsWith("Trace of")).length;
const openTrace = async (s: S) => {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
};

/** Imports M1, draws rectangle A over its left half and a small rectangle B over its bottom right
 * corner (the top is under the bar), selects the picture alone and opens Trace. Returns where a press hits A's and B's edges
 * outside the picture, and a point of the picture outside both. */
async function pickSetup(s: S) {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const w = box.x1 - box.x0;
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  await drag(s, [box.x0 - 30, box.cy - 0.6 * h], [box.cx, box.cy + 0.6 * h]);
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  await drag(s, [box.x1 + 30, box.y1 - 0.25 * h], [box.cx + 0.2 * w, box.y1 - 0.02 * h]);
  await waitFor("two rectangles", async () => ((await s.layerRows()).filter((r) => r?.endsWith("rectangle")).length === 2 ? true : null), 10_000);
  await s.key("s", 83);
  await focusCanvas(s);
  await press(s, "Escape", 27);
  const image: [number, number] = [box.cx + 0.3 * w, box.cy - 0.1 * h];
  await s.click(image[0], image[1]);
  await waitFor("the picture alone selected", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await openTrace(s);
  return { box, a: [box.x0 - 30, box.cy] as [number, number], b: [box.x1 + 30, box.y1 - 0.1 * h] as [number, number], image };
}

// ------------------------------------------------------------------ slice 1b cases

test("0094 c34 to c36, c39 to c41, c43: arm, pick two paths, remove one, Escape disarms, Clear mask, leave the tool, Tab order", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const at = await pickSetup(s);
    const rowsBefore = await s.layerRows();
    // c34: the button after View, no segments, no Clear mask.
    assert.deepEqual((await barControls(s)).slice(0, 5), ["About Trace", "Preset", "View", "Pick mask paths", "Method"]);
    assert.equal(await pressedState(s), "false");
    assert.equal(await maskState(s), "absent");
    assert.equal(await hasClear(s), false);
    // The cursor is read where the pointer is: over the canvas, away from every object.
    const rect = await canvasRect(s);
    const overCanvas = async () => {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: rect[0] + rect[2] - 30, y: rect[1] + rect[3] - 30 });
      await sleep(300);
      return canvasCursor(s);
    };
    const ownCursor = await overCanvas();
    // c35: armed, the pick-mask cursor; pressed again, disarmed, the tool's own cursor.
    await s.clickSel(PICK);
    assert.equal(await pressedState(s), "true");
    assert.match(await overCanvas(), /dasharray[\s\S]*crosshair/, "the pick-mask cursor");
    await s.clickSel(PICK);
    assert.equal(await pressedState(s), "false");
    assert.equal(await overCanvas(), ownCursor);
    assert.equal(await maskState(s), "absent", "the mask is unchanged");
    // c36: two picks, the segments with Inside, the status segment, still armed.
    await s.clickSel(PICK);
    await s.click(at.a[0], at.a[1]);
    await waitFor("the Mask segments", async () => ((await maskState(s)) === "Inside=true,Outside=false" ? true : null), 10_000);
    assert.equal(await hasClear(s), true);
    await s.click(at.b[0], at.b[1]);
    await settledCounts(s);
    await waitFor("two mask paths", async () => (/Inside 2 paths/.test(await advText(s)) ? true : null), 10_000);
    assert.equal(await pressedState(s), "true", "still armed");
    // A second pick on B removes it.
    await s.click(at.b[0], at.b[1]);
    await waitFor("one mask path", async () => (/Inside 1 path(?!s)/.test(await advText(s)) ? true : null), 10_000);
    // c38: the picture and empty canvas change nothing; a double-click on the picture creates nothing.
    await s.click(at.image[0], at.image[1]);
    const canvas = await canvasRect(s);
    await s.click(canvas[0] + canvas[2] - 20, canvas[1] + canvas[3] - 20);
    assert.match(await advText(s), /Inside 1 path(?!s)/);
    for (const clickCount of [1, 2]) {
      await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: at.image[0], y: at.image[1], button: "left", clickCount });
      await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: at.image[0], y: at.image[1], button: "left", clickCount });
    }
    await sleep(1500);
    assert.equal(await traceRows(s), 0, "a double-click on the picture creates no paths while armed");
    // c41: Tab order: the button, the segments, Clear mask; Space and Enter toggle the button.
    await s.evaluate(`document.querySelector('${PICK}').focus()`);
    await press(s, "Tab", 9);
    assert.equal(await focusedName(s), "Inside");
    await press(s, "Tab", 9);
    assert.equal(await focusedName(s), "Clear mask");
    await s.evaluate(`document.querySelector('${PICK}').focus()`);
    await press(s, " ", 32);
    assert.equal(await pressedState(s), "false", "Space disarms");
    // Enter as a typed key (with its text), so the browser activates the focused button.
    await s.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, text: "\r" });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
    await sleep(500);
    assert.equal(await pressedState(s), "true", "Enter arms");
    assert.equal(await traceRows(s), 0, "Enter on the button creates no paths");
    assert.equal(await focusedName(s), "Pick mask paths", "a key keeps the focus on the button");
    // c39: Escape (canvas focused) ends only the mode; selection and mask stay.
    await focusCanvas(s);
    await press(s, "Escape", 27);
    assert.equal(await pressedState(s), "false");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "the mask is kept");
    assert.ok(await s.evaluate(`!!document.querySelector('${BAR}')`), "the tool stays");
    // c40: Clear mask with three paths: only the picture, the paths stay, no notice, armed state unchanged.
    await s.clickSel(PICK);
    await s.click(at.b[0], at.b[1]);
    await waitFor("two mask paths again", async () => (/Inside 2 paths/.test(await advText(s)) ? true : null), 10_000);
    const status = await s.statusText();
    await s.clickSel(CLEAR);
    await waitFor("the segments go", async () => ((await maskState(s)) === "absent" ? true : null), 10_000);
    assert.equal(await hasClear(s), false);
    assert.equal(await pressedState(s), "true", "still armed");
    assert.deepEqual(await s.layerRows(), rowsBefore, "the paths are still there");
    assert.doesNotMatch((await s.statusText()).replace(status, ""), /clear|done|removed/i, "no notice");
    // c43: no hint text anywhere in the bar.
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
    // c39: leaving the tool by S disarms; entering again starts disarmed.
    await focusCanvas(s);
    await s.key("s", 83);
    await waitFor("the Trace bar closes", async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);
    await openTrace(s);
    assert.equal(await pressedState(s), "false", "not armed on entry");
  });
});

test("0094 c38: a text and a second image are refused with the row, the mode stays, the next pick clears the row", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    // The second picture first, moved to the canvas's top left corner (as the tester's c14).
    const second = path.join(SCRATCH, "second.png");
    writeFileSync(second, rgbPng(80, 80, () => [40, 40, 40]));
    await s.tab("Layers");
    await s.choose("[data-import-button]", second);
    await waitFor("the second image", async () => ((await s.layerRows()).some((r) => r?.includes("second.png")) ? true : null));
    const canvas = await canvasRect(s);
    const secondAt: [number, number] = [canvas[0] + 60, canvas[1] + 60];
    await drag(s, [canvas[0] + canvas[2] / 2, canvas[1] + canvas[3] / 2], secondAt);
    const at = await pickSetup(s);
    // A text right of M1, outside it.
    await focusCanvas(s);
    await s.key("s", 83);
    await s.clickSel('button[aria-label="Text tool (T)"]');
    const textAt: [number, number] = [at.box.x1 + 120, at.box.cy + 60];
    await s.click(textAt[0], textAt[1]);
    await sleep(700);
    await s.send("Input.insertText", { text: "Mask" });
    await sleep(300);
    await press(s, "Escape", 27);
    await s.clickSel('button[aria-label^="Select"]');
    await focusCanvas(s);
    await press(s, "Escape", 27);
    await s.click(at.image[0], at.image[1]);
    await openTrace(s);
    assert.equal(await maskState(s), "absent");
    await s.clickSel(PICK);
    await s.click(secondAt[0], secondAt[1]);
    await waitFor("the refused row for the second picture", async () => ((await refusedShown(s)) ? true : null), 10_000);
    assert.equal(await maskState(s), "absent", "the second picture is not added");
    await s.click(at.a[0], at.a[1]);
    await waitFor("the row clears with an accepted pick", async () => (!(await refusedShown(s)) ? true : null), 10_000);
    await s.click(at.a[0], at.a[1]);
    await waitFor("the mask is empty again", async () => ((await maskState(s)) === "absent" ? true : null), 10_000);
    await s.click(textAt[0] + 10, textAt[1] - 4);
    await waitFor("the refused row for the text", async () => ((await refusedShown(s)) ? true : null), 10_000);
    assert.match(await s.evaluate(`document.querySelector('${BAR}').textContent`) as string, new RegExp(REFUSED.replace(/[.;]/g, "\\$&")));
    assert.equal(await pressedState(s), "true", "still armed");
    assert.equal(await maskState(s), "absent", "nothing added");
    assert.ok(await s.evaluate(`!!document.querySelector('${PICK}')`), "the bar keeps its controls");
    // The next accepted pick clears the row.
    await s.click(at.a[0], at.a[1]);
    await waitFor("the Mask segments", async () => ((await maskState(s)) === "Inside=true,Outside=false" ? true : null), 10_000);
    assert.equal(await refusedShown(s), false);
    // Disarming clears the row too.
    await s.click(textAt[0] + 10, textAt[1] - 4);
    await waitFor("the refused row", async () => ((await refusedShown(s)) ? true : null), 10_000);
    await s.clickSel(PICK);
    assert.equal(await refusedShown(s), false);
  });
});

test("0094 c43: at 800 x 600 with the panel open the Mask group wraps as a whole item, targets of 28 px, no horizontal scroll", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(
    async (s) => {
      const at = await pickSetup(s);
      await s.clickSel(PICK);
      await s.click(at.a[0], at.a[1]);
      await waitFor("the Mask segments", async () => ((await maskState(s)) === "Inside=true,Outside=false" ? true : null), 10_000);
      // The window narrows with the mask in place (the picture is set up at 1280 x 800).
      await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
      await sleep(800);
      assert.equal(await maskState(s), "Inside=true,Outside=false");
      const g = JSON.parse(
        (await s.evaluate(`JSON.stringify((() => { const g = document.querySelector('${BAR} [data-trace-mask]');
          const kids = [...g.querySelectorAll('button')].map((b) => b.getBoundingClientRect());
          const tops = new Set(kids.map((r) => Math.round(r.top)));
          return { tops: tops.size, min: Math.min(...kids.map((r) => Math.min(r.width, r.height))),
            scroll: document.documentElement.scrollWidth > document.documentElement.clientWidth }; })())`)) as string,
      ) as { tops: number; min: number; scroll: boolean };
      assert.equal(g.tops, 1, "the group is on one line");
      assert.ok(g.min >= 28, `every target at least 28 px, got ${g.min}`);
      assert.equal(g.scroll, false, "no horizontal scroll");
    },
  );
});

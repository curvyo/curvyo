// Tester cases for the Trace bar fixes of 2026-11-09 (`specs/0089-trace-quality-and-colour/ux-notes.md` §25.4): the
// picked colours as a swatch row beside the pipette (size, order, 8 at most, hover and focus look, tooltip, removal,
// roving Tab stop), the group as wide as its content (customer 2026-11-09), and the palette strip with its detail row
// only for Multicolour with Colours Auto. Real app in Chromium (CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings
// must be built). Harness as in `acceptance_0089_tidyup_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5272;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-colours-row-tester");
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
    ["--headless=new", "--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=4,availablePointerTypes=4", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
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


// ------------------------------------------------------------------ pictures and helpers

/** Nine vertical stripes of 40 x 120, dark enough to stand out from the canvas when washed. */
const STRIPES = ["#C81E1E", "#1EA03C", "#1E3CC8", "#C8A000", "#A01EA0", "#008C96", "#000000", "#E06400", "#646464"];
const stripeRgb = (hex: string): [number, number, number] => [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16)];
const STRIPE_PNG = rgbPng(360, 120, (x) => stripeRgb(STRIPES[Math.floor(x / 40)]));

const GROUP = `${BAR} [role=group][aria-label="Picked colours"]`;
const SWATCH = `${GROUP} [data-trace-picked-swatch]`;
type Rect = { x: number; y: number; w: number; h: number; right: number; bottom: number };
const rectOf = async (s: S, sel: string, nth = 0) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelectorAll(${JSON.stringify(sel)})[${nth}]; if (!e) return null; const r = e.getBoundingClientRect();
    return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom }; })())`)) as string) as Rect | null;
const hexes = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${SWATCH}')].map((e) => e.getAttribute('data-trace-picked-swatch')))`)) as string) as string[];
const names = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${SWATCH}')].map((e) => e.getAttribute('aria-label')))`)) as string) as string[];
const noticeText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-state]')?.textContent ?? ''`)) as string;
const hiddenStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? 'absent'`)) as string;
const barHeight = async (s: S) => (await s.evaluate(`Math.round(document.querySelector('${BAR}').getBoundingClientRect().height)`)) as number;
const visibleBarTexts = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} *')].filter((e) => e.children.length === 0 && !e.closest('.sr-only, [hidden]'))
    .map((e) => e.textContent.trim()).filter((t) => /(\\d+ picked|None picked|no colour picked|\\d+ px$|^[~<] )/i.test(t)))`)) as string) as string[];
const traceState = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot]'); return { disabled: b.getAttribute('aria-disabled'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }; })())`)) as string) as { disabled: string | null; reason: string };
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
const viewport = async (s: S, width: number, height: number) => {
  await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await sleep(600);
};
const historyText = async (s: S) => {
  await s.tab("History");
  await s.evaluate(`[...document.querySelectorAll('[role=tabpanel][data-state=active] [role=radio], [role=tabpanel][data-state=active] button')].find((b) => b.textContent.trim() === 'Document')?.click()`);
  await sleep(300);
  const text = ((await s.evaluate(`document.querySelector('[role=tabpanel][data-state=active]')?.textContent ?? ''`)) as string).replace(/^DocumentObject/, "");
  await s.tab("Layers");
  return text;
};
const click = async (s: S, x: number, y: number, modifiers = 0) => {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y, modifiers });
  for (const type of ["mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, modifiers });
  }
  await sleep(600);
};
/** Where stripe `i` of STRIPE_PNG is on screen: stripes 0 and 7 are found by their washed colour (35 %: c * 0.35 + 255 * 0.65), the others lie at their pitch. */
async function stripeAt(s: S, i: number) {
  await sleep(800);
  const [cx, cy, cw, ch] = await canvasRect(s);
  const img = await shot(s);
  const top = (await barBottom(s)) + 4;
  const find = (n: number) => {
    const [r, g, b] = stripeRgb(STRIPES[n]).map((c) => c * 0.35 + 255 * 0.65);
    const box = boxOf(img, [cx, top, cw, ch - (top - cy)], (rgb) => Math.abs(rgb[0] - r) < 10 && Math.abs(rgb[1] - g) < 10 && Math.abs(rgb[2] - b) < 10);
    assert.ok(Number.isFinite(box.x0), `stripe ${n} is on screen`);
    return box;
  };
  const first = find(0);
  const seventh = find(7);
  const pitch = (seventh.x0 - first.x0) / 7;
  return { x: first.x0 + (i + 0.5) * pitch, y: (first.y0 + first.y1) / 2 };
}
const pickStripe = async (s: S, i: number, add: boolean) => {
  const p = await stripeAt(s, i);
  await click(s, p.x, p.y, add ? 8 : 0);
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 2 });
};
const activeInfo = async (s: S) =>
  (await s.evaluate(`(() => { const e = document.activeElement; if (!e || e === document.body) return 'body';
    if (e.hasAttribute('data-trace-picked-swatch')) return 'swatch ' + e.getAttribute('data-trace-picked-swatch');
    if (e.hasAttribute('data-trace-pipette')) return 'pipette';
    if (!e.closest('${BAR}')) return 'outside';
    return e.getAttribute('aria-label') || e.textContent.trim(); })()`)) as string;
const rovers = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('${SWATCH}')].filter((e) => e.tabIndex === 0).length`)) as number;

/** Opens the stripes in Pick colours and picks the first `n` stripes (Shift+click adds). */
async function withPicks(s: S, n: number) {
  await openOn(s, "stripes.png", STRIPE_PNG, false);
  await pickMethod(s, "Pick colours");
  for (let i = 0; i < n; i++) {
    await pickStripe(s, i, i > 0);
  }
}

// ------------------------------------------------------------------ §25.4: the row, its size, order, maximum and the bar that stays

test("0089 §25.4: the swatch row sits right after the pipette (20 x 20, 4 px radius, cells 24 x 28, 4 px apart), keeps the order of picking, takes 8 at most and ignores the ninth silently", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await withPicks(s, 0);
    assert.deepEqual(await hexes(s), [], "nothing picked: no swatch");
    assert.equal(await s.evaluate(`document.querySelectorAll('${GROUP}').length`), 1, "one group named Picked colours");
    const empty = { height: await barHeight(s), group: await rectOf(s, GROUP) };
    assert.ok(Math.abs(empty.group!.w - 28) <= 0.5, `nothing reserved: the group is the pipette alone with no colour: ${empty.group!.w}`);
    const picked: string[] = [];
    for (let i = 0; i < 8; i++) {
      await pickStripe(s, i, i > 0);
      picked.push(STRIPES[i].toUpperCase());
      assert.deepEqual(await hexes(s), picked, `after ${i + 1} picks: first picked first, the new one at the end`);
      assert.deepEqual(await visibleBarTexts(s), [], "no label, count or notice text");
      assert.equal(await noticeText(s), "", "no notice for a pick");
      // The group grows by the swatches only (§25.4, customer 2026-11-09): 28 + 24 per colour, its swatches on one line;
      // the bar flows by whole groups (one row more at most) and stays inside the window.
      const now = { height: await barHeight(s), group: await rectOf(s, GROUP) };
      assert.ok(Math.abs(now.group!.w - (28 + 24 * (i + 1))) <= 0.5, `${i + 1} colours: the group is ${now.group!.w} px, 28 + 24 per colour`);
      assert.equal(now.group!.h, empty.group!.h, `${i + 1} colours: the swatches stay on one line`);
      assert.ok(now.height <= empty.height + 36, `${i + 1} colours: the bar is ${now.height} px, one row more at most than ${empty.height}`);
      assert.equal(await s.evaluate(`document.querySelector('${BAR}').getBoundingClientRect().right <= innerWidth + 0.5 && document.scrollingElement.scrollWidth <= innerWidth + 1`), true, `${i + 1} colours: the bar fits the window`);
    }
    // Geometry at eight.
    const pipette = (await rectOf(s, `${GROUP} [data-trace-pipette]`))!;
    assert.equal(pipette.w, 28);
    const cells = [];
    for (let i = 0; i < 8; i++) {
      cells.push((await rectOf(s, SWATCH, i))!);
    }
    for (const c of cells) {
      assert.deepEqual([c.w, c.h], [24, 28], "hit target 24 x 28");
    }
    for (let i = 1; i < 8; i++) {
      assert.equal(Math.round(cells[i].x - cells[i - 1].x), 24, "the cells touch: 20 px swatches 4 px apart");
    }
    const inner = JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('${SWATCH}').firstElementChild; const r = e.getBoundingClientRect(); const c = getComputedStyle(e);
      return { x: r.x, w: r.width, h: r.height, radius: c.borderTopLeftRadius, border: c.borderTopWidth, bg: c.backgroundColor, cursor: getComputedStyle(e.parentElement).cursor }; })())`)) as string) as { x: number; w: number; h: number; radius: string; border: string; bg: string; cursor: string };
    assert.deepEqual([inner.w, inner.h, inner.radius, inner.border], [20, 20, "4px", "1px"]);
    assert.equal(inner.bg, `rgb(${stripeRgb(STRIPES[0]).join(", ")})`, "the colour is opaque and exact");
    assert.equal(inner.cursor, "default", "the default arrow, not a hand");
    assert.ok(Math.abs(inner.x - pipette.right - 4) <= 0.5, `the first swatch is 4 px after the pipette: ${inner.x - pipette.right}`);
    assert.ok(Math.abs(cells[7].right - 2 - pipette.x - 220) <= 0.5, `28 + 4 + 8 x 20 + 7 x 4 = 220 px: ${cells[7].right - 2 - pipette.x}`);
    assert.equal((await names(s))[0], "Remove #C81E1E");
    // The ninth pick does nothing, silently; the pipette stays armed.
    await pickStripe(s, 8, true);
    assert.deepEqual(await hexes(s), picked, "a ninth pick adds nothing");
    assert.equal(await noticeText(s), "", "silently: no '8 colours picked' notice");
    assert.deepEqual(await visibleBarTexts(s), []);
    assert.equal(await s.evaluate(`document.querySelector('${GROUP} [data-trace-pipette]').getAttribute('aria-pressed')`), "true", "the pipette stays armed");
    // A plain click replaces the whole set with one swatch.
    await pickStripe(s, 8, false);
    assert.deepEqual(await hexes(s), ["#646464"], "a plain click replaces the set");
  });
});

test("0089 §25.4: at 800 x 600 the bar never scrolls or leaves the window from 0 to 8 colours; with the panel open the group shares Colour tolerance's line up to 5 colours and takes its own line from the sixth; the Advanced block does not wrap", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await withPicks(s, 0);
    const survey = async () => {
      await viewport(s, 800, 600);
      const out: Record<string, { height: number; group: Rect; tol: Rect; fits: boolean }> = {};
      for (const open of [true, false]) {
        await setPanel(s, open);
        out[String(open)] = {
          height: await barHeight(s),
          group: (await rectOf(s, GROUP))!,
          tol: (await rectOf(s, `${BAR} [role=spinbutton][aria-label="Colour tolerance"]`))!,
          fits: (await s.evaluate(`(() => { const b = document.querySelector('${BAR}'); const r = b.getBoundingClientRect(); return b.scrollWidth <= b.clientWidth + 1 && document.scrollingElement.scrollWidth <= innerWidth + 1 && r.right <= innerWidth + 0.5; })()`)) as boolean,
        };
      }
      await setPanel(s, true);
      await viewport(s, 1280, 800);
      return out;
    };
    const sameLine = (a: Rect, b: Rect) => Math.abs(a.y + a.h / 2 - (b.y + b.h / 2)) <= 3;
    const seen: Record<number, Awaited<ReturnType<typeof survey>>> = { 0: await survey() };
    for (let i = 0; i < 8; i++) {
      await pickStripe(s, i, i > 0);
      seen[i + 1] = await survey();
    }
    assert.equal((await hexes(s)).length, 8);
    console.log("800 x 600 by colours:", JSON.stringify(Object.entries(seen).map(([n, v]) => ({ n, open: v.true.height, closed: v.false.height, group: v.true.group.w }))));
    for (const [n, v] of Object.entries(seen)) {
      for (const open of ["true", "false"] as const) {
        assert.ok(v[open].fits, `${n} colours, panel ${open === "true" ? "open" : "closed"}: no sideways scroll, inside the window`);
        assert.ok(Math.abs(v[open].group.w - (28 + 24 * Number(n))) <= 0.5, `${n} colours: the group is as wide as its content`);
        assert.equal(v[open].group.h, seen[0][open].group.h, `${n} colours: the swatches stay on one line`);
      }
      assert.equal(v.false.height, 108, `${n} colours, panel closed: 3 rows`);
      // `0114` UX notes §2.1: the first row is 264 px narrower, so Colour tolerance now sits on line 1 and the
      // group follows it as a whole: on its line while it fits (up to one colour at 800 x 600), else whole on line 2.
      const closedShares = sameLine(v.false.tol, v.false.group);
      assert.ok(closedShares || (v.false.group.y > v.false.tol.y + 20 && v.false.group.x < v.false.tol.x), `${n} colours, panel closed: the group shares Colour tolerance's line or wraps below it as a whole ${JSON.stringify([v.false.tol, v.false.group])}`);
      assert.equal(closedShares, Number(n) <= 1, `${n} colours, panel closed: the group ${Number(n) <= 1 ? "fits" : "wraps"}`);
      const shares = Number(n) <= 5;
      assert.equal(sameLine(v.true.tol, v.true.group), shares, `${n} colours, panel open: the group ${shares ? "shares" : "leaves"} Colour tolerance's line`);
      // `0114` UX notes §2.1: the Mask line left the first row, one row (36 px) less than before.
      assert.equal(v.true.height, shares ? 180 : 216, `${n} colours, panel open: ${shares ? 5 : 6} rows`);
    }
    // The Advanced block keeps its rows at 800 x 600 with eight colours.
    await viewport(s, 800, 600);
    await setAdvanced(s, true);
    for (const open of [true, false]) {
      await setPanel(s, open);
      const wraps = (await s.evaluate(`(() => { const a = document.querySelector('${ADV}'); if (!a) return 'absent';
        return [...a.querySelectorAll('[role=radiogroup], [role=spinbutton], button')].filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && (r.right > a.getBoundingClientRect().right + 0.5 || e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).overflow !== 'visible'); }).map((e) => e.getAttribute('aria-label') || e.textContent.trim()).join(','); })()`)) as string;
      assert.equal(wraps, "", `panel ${open ? "open" : "closed"}: nothing in Advanced runs over or is clipped`);
    }
  });
});

// ------------------------------------------------------------------ §25.4: hover, focus, tooltip, removal, keyboard

test("0089 §25.4: hover fades the swatch to 40 % and shows the x, the tooltip is the colour code, the button is named 'Remove #RRGGBB'", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await withPicks(s, 3);
    const look = async (i: number) =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelectorAll('${SWATCH}')[${i}]; const sw = b.firstElementChild; const x = b.querySelector('svg').getBoundingClientRect();
        return { opacity: getComputedStyle(sw).opacity, xw: x.width, xh: x.height }; })())`)) as string) as { opacity: string; xw: number; xh: number };
    const rest = await look(1);
    assert.deepEqual([rest.opacity, rest.xw], ["1", 0], "at rest: opaque, no x");
    const c = (await rectOf(s, SWATCH, 1))!;
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c.x + 1, y: c.y + 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c.x + c.w / 2, y: c.y + c.h / 2 });
    await sleep(150);
    const hover = await look(1);
    assert.equal(hover.opacity, "0.4", "hover: 40 %");
    assert.ok(Math.abs(hover.xw - 12) <= 1 && Math.abs(hover.xh - 12) <= 1, `hover: the 12 px x: ${hover.xw} x ${hover.xh}`);
    assert.equal((await look(0)).opacity, "1", "the others stay opaque");
    assert.equal(await s.evaluate(`document.querySelectorAll('[role=tooltip]').length`), 0, "no tooltip before the delay");
    const want = (await hexes(s))[1];
    const tip = await waitFor("the tooltip", async () => (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.textContent.trim()).find((t) => t.length > 0) ?? null`)) as string | null, 4000);
    assert.equal(tip, want, "the tooltip is the colour code and nothing else");
    assert.match(tip, /^#[0-9A-F]{6}$/);
    assert.equal((await names(s))[1], `Remove ${want}`, "the name says what the button does");
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 2, y: 2 });
    await sleep(200);
    assert.equal((await look(1)).opacity, "1", "leaving restores it");
  });
});

test("0089 §25.4: the swatches are one Tab stop with roving arrows; keyboard focus shows the x and a ring; Tab order is pipette, the row, Smooth", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await withPicks(s, 4);
    const all = await hexes(s);
    assert.equal(await rovers(s), 1, "one Tab stop");
    await s.evaluate(`document.querySelector('${GROUP} [data-trace-pipette]').focus()`);
    await press(s, "Tab", 9);
    assert.equal(await activeInfo(s), `swatch ${all[0]}`, "Tab from the pipette lands on the row");
    const look = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.activeElement; const x = b.querySelector('svg').getBoundingClientRect();
      return { opacity: getComputedStyle(b.firstElementChild).opacity, xw: x.width, ring: getComputedStyle(b.children[1]).boxShadow }; })())`)) as string) as { opacity: string; xw: number; ring: string };
    assert.equal(look.opacity, "0.4", "focus fades like hover");
    assert.ok(look.xw > 10, "focus shows the x");
    assert.notEqual(look.ring, "none", "focus adds the ring");
    const tip = await waitFor("the tooltip on focus", async () => (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.textContent.trim()).find((t) => t.length > 0) ?? null`)) as string | null, 4000);
    assert.equal(tip, all[0], "the tooltip shows on keyboard focus as well");
    await press(s, "ArrowRight", 39);
    assert.equal(await activeInfo(s), `swatch ${all[1]}`);
    await press(s, "End", 35);
    assert.equal(await activeInfo(s), `swatch ${all[3]}`);
    await press(s, "Home", 36);
    assert.equal(await activeInfo(s), `swatch ${all[0]}`);
    await press(s, "ArrowRight", 39);
    assert.equal(await rovers(s), 1, "still one Tab stop");
    await press(s, "Tab", 9);
    assert.equal(await activeInfo(s), "Smooth", "Tab leaves the row to Smooth");
    await press(s, "Tab", 9, 8);
    assert.equal(await activeInfo(s), `swatch ${all[1]}`, "Shift+Tab returns to the stop where it was");
    await press(s, "Tab", 9, 8);
    assert.equal(await activeInfo(s), "pipette");
  });
});

test("0089 §25.4: removal: a click, Delete or Backspace removes; focus follows the rule; it is no history step and no notice; the last removal leaves the empty state and is announced", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await withPicks(s, 4);
    const all = await hexes(s);
    const history = await historyText(s);
    // Pointer removal: no history step, no notice; the keyboard goes back to the canvas.
    const c = (await rectOf(s, SWATCH, 0))!;
    await click(s, c.x + 12, c.y + 14);
    await waitFor("three swatches", async () => ((await hexes(s)).length === 3 ? true : null));
    assert.deepEqual(await hexes(s), all.slice(1));
    assert.equal(await historyText(s), history, "removal is not a history step");
    assert.equal(await noticeText(s), "", "no notice, no confirm");
    assert.ok(["outside", "body"].includes(await activeInfo(s)) && !(await s.evaluate(`!!document.activeElement?.closest('${BAR}')`)), `after a pointer removal the focus is not in the bar: ${await activeInfo(s)}`);
    assert.match(await hiddenStatus(s), new RegExp(`Removed ${all[0]}`), "the hidden region announces the removal");
    // Delete on the first of three: focus moves to the swatch now at the same place.
    await s.evaluate(`document.querySelector('${SWATCH}').focus()`);
    await press(s, "Delete", 46);
    await waitFor("two swatches", async () => ((await hexes(s)).length === 2 ? true : null));
    await sleep(200);
    assert.deepEqual(await hexes(s), [all[2], all[3]]);
    assert.equal(await activeInfo(s), `swatch ${all[2]}`, "focus: the next swatch at the same place");
    // Backspace on the last: focus goes to the previous one.
    await press(s, "End", 35);
    await press(s, "Backspace", 8);
    await waitFor("one swatch", async () => ((await hexes(s)).length === 1 ? true : null));
    await sleep(200);
    assert.equal(await activeInfo(s), `swatch ${all[2]}`, "focus: the previous swatch");
    // The last one: focus to the pipette, empty state of §25.3.
    await press(s, "Delete", 46);
    await waitFor("none", async () => ((await hexes(s)).length === 0 ? true : null));
    await sleep(300);
    assert.equal(await activeInfo(s), "pipette", "focus: the pipette when none is left");
    assert.equal(await rovers(s), 0, "no colours, no Tab stop");
    const status = await hiddenStatus(s);
    assert.ok(status.includes(`Removed ${all[2]}`) && status.includes("0 paths: no colour picked."), `announced: ${status}`);
    assert.equal(status.split("no colour picked").length - 1, 1, "the empty reason once");
    assert.equal(await noticeText(s), "", "not drawn in the notice row");
    assert.deepEqual(await visibleBarTexts(s), []);
    const trace = await traceState(s);
    assert.equal(trace.disabled, "true", "Trace is aria-disabled");
    assert.equal(trace.reason, "0 paths: no colour picked.");
    assert.equal(await historyText(s), history, "and the whole sequence is no history step");
  });
});

// ------------------------------------------------------------------ §25.4, §26: the palette row only with Multicolour, never with Pick colours

test("0089 §25.4, §26: the palette's swatch row shows only for Multicolour without Pick colours, with no detail row and no actions; Pick colours (Single or Multicolour) uses its own row", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "stripes.png", STRIPE_PNG, true);
    const strip = async () => ({
      swatches: (await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-palette-swatch]').length`)) as number,
      actions: (await s.evaluate(`document.querySelectorAll('${ADV} [data-trace-palette-action]').length`)) as number,
      detail: (await s.evaluate(`!!document.querySelector('${ADV} [data-trace-palette-detail]')`)) as boolean,
    });
    await pickMethod(s, "Brightness");
    await setOutput(s, "Single colour");
    assert.deepEqual(await strip(), { swatches: 0, actions: 0, detail: false }, "Single colour, Brightness: none");
    await setOutput(s, "Multicolour");
    await waitFor("the palette row", async () => (((await strip()).swatches >= 1) ? true : null), 60_000);
    const auto = await strip();
    assert.deepEqual({ actions: auto.actions, detail: auto.detail }, { actions: 0, detail: false }, `Multicolour, Colours Auto: swatches only: ${JSON.stringify(auto)}`);
    assert.equal(await s.evaluate(`document.querySelector('${ADV} [data-trace-pipette="palette"]').parentElement.nextElementSibling === document.querySelector('${ADV} [role=group][aria-label=Palette]')`), true, "the row right after the palette pipette");
    await pickMethod(s, "Pick colours");
    await pickStripe(s, 0, false);
    assert.deepEqual(await strip(), { swatches: 0, actions: 0, detail: false }, "Multicolour, Pick colours: none");
    assert.equal((await hexes(s)).length, 1, "the picked colour is in the swatch row");
    await setOutput(s, "Single colour");
    assert.deepEqual(await strip(), { swatches: 0, actions: 0, detail: false }, "Single colour, Pick colours: none");
    assert.equal((await hexes(s)).length, 1);
    assert.deepEqual(await visibleBarTexts(s), []);
  });
});

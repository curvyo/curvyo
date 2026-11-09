// Tester cases for `specs/0094-masked-composite-trace` slice 2 (Parts and Join) in the real app
// (Chromium via DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be
// built). Written from the specification and the UX notes (§3 to §8) before the implementation
// was read: the Parts row with chip 1 and Add part (criterion 18), a new part with a copy of the
// settings and no paths, chip picks loading each part's own values (19), accessible chip names
// (32), the 8-part cap (18), the inline trash confirm (28), leaving the tool (29), only the changed
// part traced again (25, a count of the worker's trace jobs), one rising progress and a stop with
// no document change (26), the region-only analysis per part (spec Proposal), no hint text at
// 800 x 600 (43), and the editor wasm replaying the native record of
// `curvyo-editor-wasm/tests/acceptance_0094_parts_tester.rs` on two own instances (determinism).
// Harness as in `acceptance_0094_tester.test.ts`; page-side wasm imports use their own
// instances (`?own-instance`), never the app's.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5398;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0094-parts-tester");
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


/** Every user-visible string of the bar and its Advanced block: labels, titles and text. */
const barStrings = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...new Set([...document.querySelector('${BAR}').querySelectorAll('*')].flatMap((e) =>
    [e.getAttribute('aria-label'), e.getAttribute('title'), e.children.length === 0 ? e.textContent.trim() : ''].filter(Boolean)))])`)) as string) as string[];

const advancedStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const settledCounts = async (s: S) => {
  try {
    return await waitFor("a trace result", async () => (/(^|\. )(\d+ colours?, )?\d+ paths?, \d+ nodes?/.test(await advancedStatus(s)) ? true : null), 60_000);
  } catch (error) {
    throw new Error(`${String(error)}; the status reads "${await advancedStatus(s)}", the block "${await advText(s)}", the bar "${String(await s.evaluate(`document.querySelector('${BAR}')?.innerText`)).replace(/\n/g, " / ")}"`);
  }
};


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
const NATIVE = path.resolve(FRONTEND, "..", "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0094_parts_tester_native.json");

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
const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
// §24.5: the muted "Inside 1 path" and "2 parts" segments are announced by the hidden region only.
const advText = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
/** The preview outline's pixels (the Overlay's one colour, 47,111,238, over the washed image) in a box of the picture. */
const isOutline = ([r, g, b]: number[]) => b > 200 && r < 120 && g < 170;
const outlineIn = (img: Shot, box: { x0: number; y0: number; x1: number; y1: number }, fx0: number, fy0: number, fx1: number, fy1: number) => {
  let n = 0;
  for (let y = Math.round(box.y0 + fy0 * (box.y1 - box.y0)); y <= box.y0 + fy1 * (box.y1 - box.y0); y++) {
    for (let x = Math.round(box.x0 + fx0 * (box.x1 - box.x0)); x <= box.x0 + fx1 * (box.x1 - box.x0); x++) {
      if (isOutline(at(img, x, y))) n++;
    }
  }
  return n;
};

// ------------------------------------------------------------------ slice 2 helpers

const PARTS = `${ADV} [role=radiogroup][aria-label=Parts]`;
const ADD = `${BAR} button[aria-label="Add part"]`;
const TALL: [number, number] = [1280, 1100];

/** The chips: name, checked, tabindex. */
const chips = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PARTS} [role=radio]')].map((c) => ({ name: c.getAttribute('aria-label') ?? c.textContent.trim(), text: c.textContent.trim(), checked: c.getAttribute('aria-checked') === 'true', title: c.getAttribute('title') ?? '' })))`)) as string) as { name: string; text: string; checked: boolean; title: string }[];
const clickChip = (s: S, n: number) => s.evaluate(`[...document.querySelectorAll('${PARTS} [role=radio]')][${n - 1}].click()`).then(() => sleep(900));
const addPart = (s: S) => s.evaluate(`document.querySelector('${ADD}').click()`).then(() => sleep(900));
const trash = `${BAR} button[aria-label^="Remove part"], ${BAR} button[aria-label^="Press again to remove part"], ${BAR} button[aria-label="Keep at least one part"]`;
const trashName = async (s: S) => (await s.evaluate(`document.querySelector(${JSON.stringify(trash)})?.getAttribute('aria-label') ?? ''`)) as string;
const spin = async (s: S, name: string) => Number(await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]')?.getAttribute('aria-valuenow')`));
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  const items = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=menu] [role^=menuitem]')].map((e) => e.textContent))`)) as string;
  assert.ok(items.includes(label), `the Method menu offers ${label}: ${items}`);
  await s.evaluate(`[...document.querySelectorAll('[role=menu] [role^=menuitem]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};
const reenter = async (s: S) => {
  await focusCanvas(s);
  await s.key("s", 83);
  await waitFor("the Trace bar closes", async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  await setAdvanced(s, true);
};

/** Imports `png` (default M1), draws a rectangle over the left half (L) and one over the right
 * half (R), each reaching 30 px beyond the picture, selects all and opens Trace; then Shift+click
 * takes R off part 1. Returns the picture box and R's outer edge spot. */
async function twoHalves(s: S, png: Buffer = M1_PNG, name = "m1.png") {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(name)) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  const h = box.y1 - box.y0;
  await focusCanvas(s);
  await press(s, "Escape", 27);
  // The top of the canvas holds the bars: the rectangles start below them.
  const canvas = await canvasRect(s);
  const barBottom = Number(await s.evaluate(`Math.max(0, ...[...document.querySelectorAll('[data-context-bar]')].map((e) => e.getBoundingClientRect().bottom))`));
  const top = Math.max(box.cy - 0.6 * h, canvas[1] + 90, barBottom + 12);
  const bottom = Math.min(box.cy + 0.6 * h, canvas[1] + canvas[3] - 12);
  // The tool palettes sit over the canvas's left edge.
  const palettes = Number(await s.evaluate(`Math.max(0, ...[...document.querySelectorAll('[role=toolbar]')].map((e) => e.getBoundingClientRect()).filter((r) => r.left < 200 && r.height > 200).map((r) => r.right))`));
  const leftEdge = Math.max(box.x0 - 30, palettes + 12);
  await s.key("r", 82);
  await drag(s, [leftEdge, top], [box.cx, bottom]);
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await s.key("r", 82);
  const rightEdge = Math.min(box.x1 + 30, canvas[0] + canvas[2] - 12);
  await drag(s, [rightEdge, top], [box.cx + 6, bottom]);
  await s.key("s", 83);
  const rows = await s.layerRows();
  if (rows.filter((r) => /^Rectangle/i.test(r ?? "")).length !== 2) {
    const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
    writeFileSync(path.join(SCRATCH, "debug.png"), Buffer.from(out.result?.data ?? "", "base64"));
  }
  assert.equal(rows.filter((r) => /^Rectangle/i.test(r ?? "")).length, 2, `two rectangles: ${JSON.stringify(rows)}, box ${JSON.stringify(box)}, canvas ${JSON.stringify(canvas)}`);
  await focusCanvas(s);
  await s.key("a", 65, 2);
  await waitFor("Trace offered", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  await setAdvanced(s, true);
  const left: [number, number] = [leftEdge + 15, box.cy + 0.3 * h];
  // A closed outline is hit by its interior, the traced image skipped (criterion 35's rule).
  const right: [number, number] = [Math.min(box.x1 + 15, rightEdge - 15), box.cy + 0.3 * h];
  await shiftClick(s, right[0], right[1]);
  await waitFor("part 1 has L only", async () => (/Inside 1 path(?!s)/.test(await advText(s)) ? true : null), 20_000);
  return { box, left, right };
}

/** Two parts: 1 = L, 2 = R. */
async function twoParts(s: S, png?: Buffer, name?: string) {
  const scene = await twoHalves(s, png, name);
  await addPart(s);
  await shiftClick(s, scene.right[0], scene.right[1]);
  await waitFor("part 2 has R", async () => ((await chips(s))[1]?.name.includes("1 path") ? true : null), 20_000);
  return scene;
}

// ------------------------------------------------------------------ slice 2 cases

test("0094 c18, c19, c32, c43: chip 1 and Add part, a new part copies the values and has no paths, chips keep their own values, accessible names, 8 parts at most", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const scene = await twoHalves(s);
    await settledCounts(s);
    // c18: the Parts row is the first row of the Advanced block.
    const first = (await s.evaluate(`(() => { const adv = document.querySelector('${ADV}'); const parts = adv.querySelector('[role=radiogroup][aria-label=Parts]'); const analyse = [...adv.querySelectorAll('button')].find((b) => (b.getAttribute('aria-label') ?? b.textContent).trim() === 'Analyse');
      return !!parts && !!analyse && !!(parts.compareDocumentPosition(analyse) & Node.DOCUMENT_POSITION_FOLLOWING); })()`)) as boolean;
    assert.ok(first, "Parts comes before Analyse");
    let c = await chips(s);
    assert.equal(c.length, 1);
    assert.equal(c[0].text.replace(/\s/g, ""), "1");
    assert.ok(c[0].checked, "chip 1 is active");
    assert.equal(c[0].name, "Part 1 of 1, inside, 1 path, active");
    assert.ok(await s.evaluate(`!!document.querySelector('${ADD}')`), "Add part");
    // c32: the Mask segments.
    assert.deepEqual(JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].map((r) => r.getAttribute('aria-label') ?? r.textContent.trim()))`)) as string), ["Inside", "Outside"]);
    // Part 1's own Threshold, then Add part copies it.
    await typeInto(s, "Threshold", "77");
    await settledCounts(s);
    await addPart(s);
    c = await chips(s);
    assert.equal(c.length, 2);
    assert.ok(c[1].checked && !c[0].checked, "chip 2 is active");
    assert.equal(c[0].name, "Part 1 of 2, inside, 1 path");
    assert.match(c[1].name, /^Part 2 of 2, (inside, )?no paths, active$/);
    const tip = async (n: number) => {
      const xy = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('${PARTS} [role=radio]')][${n - 1}].getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })())`)) as string) as number[];
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: xy[0], y: xy[1] });
      await sleep(1200);
      const text = (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((t) => t.textContent.trim()).filter(Boolean).join(' | ')`)) as string;
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 5, y: 5 });
      await sleep(400);
      return text;
    };
    assert.equal(await tip(2), c[1].name.replace(/, active$/, ""), "the tooltip is the name without active");
    assert.equal(await spin(s, "Threshold"), 77, "a copy of the part before");
    assert.equal(await maskState(s), "Inside=true,Outside=false", "§24.4: the Mask group is drawn, Inside, with no paths yet");
    assert.doesNotMatch(await advText(s), /Inside \d|Outside \d/, "no mask paths yet");
    assert.doesNotMatch((await s.evaluate(`document.querySelector('${BAR}').textContent`)) as string, /Part 2 of 2/, "`specs/0089` ux-notes §27: no state label in the bar");
    // c19: Shift+click assigns R to part 2.
    await shiftClick(s, scene.right[0], scene.right[1]);
    await waitFor("part 2 has R", async () => ((await chips(s))[1].name === "Part 2 of 2, inside, 1 path, active" ? true : null), 20_000);
    assert.equal((await chips(s))[0].name, "Part 1 of 2, inside, 1 path", "part 1 keeps L");
    await typeInto(s, "Threshold", "40");
    await sleep(600);
    await clickChip(s, 1);
    assert.ok((await chips(s))[0].checked);
    assert.equal(await spin(s, "Threshold"), 77, "part 1's own value");
    await clickChip(s, 2);
    assert.equal(await spin(s, "Threshold"), 40, "part 2's own value");
    // Outside on part 2 only.
    await clickMask(s, "Outside");
    await sleep(600);
    assert.equal((await chips(s))[1].name, "Part 2 of 2, outside, 1 path, active");
    await clickChip(s, 1);
    assert.equal(await maskState(s), "Inside=true,Outside=false", "part 1 stays Inside");
    await settledCounts(s);
    assert.match(await advText(s), /2 parts/, "the status line's parts segment");
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
    assert.doesNotMatch(await s.statusText(), /created|done|success|added/i, "no success notice");
    // c18: 8 parts at most.
    for (let n = 3; n <= 8; n++) {
      await addPart(s);
    }
    c = await chips(s);
    assert.equal(c.length, 8);
    assert.equal(await s.evaluate(`document.querySelector('${ADD}').getAttribute('aria-disabled')`), "true", "Add part is dimmed at 8");
    await addPart(s);
    assert.equal((await chips(s)).length, 8, "a press at 8 adds nothing");
    assert.match((await chips(s))[7].name, /^Part 8 of 8, (inside, )?no paths, active$/);
    // c43: at 800 x 600 the row wraps as whole items, targets at least 28 px, no horizontal scroll.
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await sleep(1200);
    assert.equal(await s.evaluate(`document.documentElement.scrollWidth <= window.innerWidth`), true, "no horizontal scroll");
    const sizes = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PARTS} [role=radio], ${ADD}, ${trash.replace(/'/g, "\\'")}')].map((e) => { const r = e.getBoundingClientRect(); return [Math.round(r.width), Math.round(r.height)]; }))`)) as string) as number[][];
    assert.equal(sizes.length, 10, `8 chips, Add and the trash: ${JSON.stringify(sizes)}`);
    assert.ok(sizes.every(([w, h]) => w >= 28 && h >= 28), `every target at least 28 px: ${JSON.stringify(sizes)}`);
    const overflow = (await s.evaluate(`(() => { const g = document.querySelector('${PARTS}'); return g.scrollWidth <= g.clientWidth + 1; })()`)) as boolean;
    assert.ok(overflow, "the chips do not overflow their row");
  }, "", TALL[0], TALL[1]);
});

test("0094 c28, c29: the trash arms first and removes on a press after 400 ms, chips renumber; leaving the tool leaves one part", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await twoHalves(s);
    assert.equal(await trashName(s), "Keep at least one part", "one part cannot be removed");
    assert.equal(await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).getAttribute('aria-disabled')`), "true");
    await addPart(s);
    await addPart(s);
    assert.equal((await chips(s)).length, 3);
    await clickChip(s, 2);
    assert.equal(await trashName(s), "Remove part 2");
    // The trash is no Tab stop.
    assert.equal(await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).tabIndex`), -1);
    await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).click()`);
    await sleep(100);
    assert.equal(await trashName(s), "Press again to remove part 2", "armed");
    const armed = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector(${JSON.stringify(trash)}); const cs = getComputedStyle(b); const row = b.parentElement; return [cs.backgroundColor, getComputedStyle(row).backgroundColor]; })())`)) as string) as string[];
    console.log("armed trash and row colours", JSON.stringify(armed));
    assert.ok(armed.some((col) => /rgb/.test(col) && !/rgba\(0, 0, 0, 0\)/.test(col)), `soft red: ${JSON.stringify(armed)}`);
    assert.match(await s.statusText(), /Press again to remove part 2/, "the polite status repeats it");
    // A second press within 400 ms does not remove.
    await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).click()`);
    await sleep(100);
    assert.equal((await chips(s)).length, 3, "no removal within 400 ms");
    await sleep(500);
    await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).click()`);
    await sleep(700);
    const c = await chips(s);
    assert.equal(c.length, 2, "removed");
    assert.deepEqual(c.map((x) => x.text.replace(/\s/g, "")), ["1", "2"], "renumbered");
    assert.match(c[1].name, /^Part 2 of 2/);
    assert.doesNotMatch(await s.statusText(), /removed|deleted|success/i, "no notice after the removal");
    // c29: leaving the tool and entering again: one part.
    await reenter(s);
    const after = await chips(s);
    assert.equal(after.length, 1, "one part after leaving the tool");
    assert.ok(after[0].checked);
  }, "", TALL[0], TALL[1]);
});

// The worker's jobs, counted in the page: every `trace` message posted to a Worker.
const COUNT_JOBS = `(() => { const post = Worker.prototype.postMessage; window.__traceJobs = [];
  Worker.prototype.postMessage = function (message, ...rest) { if (message && message.type === 'trace') window.__traceJobs.push({ settings: message.settings, width: message.width, height: message.height }); return post.call(this, message, ...rest); }; })();`;
const jobsSoFar = async (s: S) => JSON.parse((await s.evaluate(`JSON.stringify(window.__traceJobs)`)) as string) as { settings: string; width: number; height: number }[];

/** Records the Trace progress bar's values in the page. */
const WATCH_PROGRESS = `(() => { window.__progress = []; const seen = () => { const p = document.querySelector('[role=toolbar][aria-label=Trace] [role=progressbar]'); if (p) { const v = Number(p.getAttribute('aria-valuenow')); if (window.__progress[window.__progress.length - 1] !== v) window.__progress.push(v); } };
  new MutationObserver(seen).observe(document.documentElement, { subtree: true, attributes: true, childList: true, attributeFilter: ['aria-valuenow'] }); })();`;

test("0094 c25, c26: a change in one part traces only that part again; one rising progress over several parts", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await twoParts(s);
    await settledCounts(s);
    await sleep(1500);
    console.log("jobs before the change:", (await jobsSoFar(s)).length, "Smooth", await spin(s, "Smooth"));
    await s.evaluate(`window.__traceJobs.length = 0`);
    await typeInto(s, "Smooth", "35");
    await settledCounts(s);
    await sleep(2500);
    const one = await jobsSoFar(s);
    console.log("jobs after one change:", one.length, JSON.stringify(one.map((j) => [j.width, j.height])));
    assert.equal(one.length, 1, "only the changed part is traced again");
    assert.match(one[0].settings, /"smooth":35/, "the changed part's job");
    // The other part's join is still shown: two parts in the status.
    assert.match(await advText(s), /2 parts/);
    // c26: a change of part 2's mode retraces part 1 too (its clip changes): one progress.
    await s.evaluate(WATCH_PROGRESS);
    await s.evaluate(`window.__traceJobs.length = 0`);
    await clickMask(s, "Outside");
    await settledCounts(s);
    await sleep(2500);
    const progress = JSON.parse((await s.evaluate(`JSON.stringify(window.__progress)`)) as string) as number[];
    console.log("progress values:", JSON.stringify(progress), "jobs:", (await jobsSoFar(s)).length);
    for (let i = 1; i < progress.length; i++) {
      assert.ok(progress[i] >= progress[i - 1], `never backwards: ${JSON.stringify(progress)}`);
    }
  }, COUNT_JOBS, TALL[0], TALL[1]);
});

/** M1 drawn at `n` px. */
const m1At = (n: number, rightPaper: [number, number, number] = [255, 255, 255], leftPaper: [number, number, number] = [255, 255, 255]) =>
  rgbPng(n, n, (x, y) => {
    const u = ((x + 0.5) * 1000) / n;
    const ink = m1Ink(u, ((y + 0.5) * 1000) / n);
    return ink === "paper" ? (u >= 500 ? rightPaper : leftPaper) : ink === "grey" ? [128, 128, 128] : [0, 0, 0];
  });

test("0094 c26: a stop while several parts trace leaves the document unchanged, and Trace again finishes the join", { skip: !BROWSER, timeout: 500_000 }, async () => {
  await withApp(async (s) => {
    await twoParts(s, m1At(2400), "big.png");
    await settledCounts(s);
    const rows = await s.layerRows();
    // A change in both parts' clips (part 2's mode), then a stop while it runs.
    await s.evaluate(WATCH_PROGRESS);
    // The Stop button inside the progress slot, pressed by the page as soon as the job is under
    // way (a 2400 px picture traces too fast to stop it from here).
    await s.evaluate(`(() => { window.__stopped = null; const look = () => { const p = document.querySelector('[role=toolbar][aria-label=Trace] [role=progressbar]'); if (!p || window.__stopped !== null) return;
      const v = Number(p.getAttribute('aria-valuenow')); if (v >= 1 && v <= 99) { const b = document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label="Stop (Esc)"]'); if (b) { window.__stopped = v; b.click(); } } };
      new MutationObserver(look).observe(document.documentElement, { subtree: true, attributes: true, childList: true }); })()`);
    await clickMask(s, "Outside");
    await waitFor("the stop", async () => ((await s.evaluate(`window.__stopped`)) !== null ? true : null), 60_000);
    await sleep(800);
    console.log("stopped at", await s.evaluate(`window.__stopped`));
    const progress = JSON.parse((await s.evaluate(`JSON.stringify(window.__progress)`)) as string) as number[];
    console.log("progress before the stop:", JSON.stringify(progress));
    assert.ok(progress.length >= 2, `a progress was shown: ${JSON.stringify(progress)}`);
    for (let i = 1; i < progress.length; i++) {
      assert.ok(progress[i] >= progress[i - 1], `never backwards over the parts: ${JSON.stringify(progress)}`);
    }
    assert.ok(await s.evaluate(`!!document.querySelector('${BAR}')`), "a stop keeps the tool");
    assert.deepEqual(await s.layerRows(), rows, "no document change");
    console.log("after stop:", await advancedStatus(s), "|", await s.statusText());
    assert.match((await s.evaluate(`document.querySelector('${BAR}').innerText`)) as string, /Stopped/, "Stopped, as in 0025");
    // Retry finishes the join.
    const again = (await s.evaluate(`!![...document.querySelectorAll('${BAR} button')].find((b) => /^(Retry|Trace again)/.test((b.getAttribute('aria-label') ?? b.textContent).trim()))`)) as boolean;
    assert.ok(again, "Retry is offered after the stop");
    if (again) {
      await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => /^(Retry|Trace again)/.test((b.getAttribute('aria-label') ?? b.textContent).trim())).click()`);
    }
    await settledCounts(s);
    assert.match(await advText(s), /2 parts/);
  }, "", TALL[0], TALL[1]);
});

// Was FAIL and todo (tester 2026-11-06); since 5f2f56b2 each part reads its own region.
// The grey paper under part 2 (M2), and the mirror: grey under part 1 only, white under part 2. Reading
// the whole picture's background for part 2 passes the first (the grey wins there) and fails the mirror.
for (const [label, left, right] of [["grey", [255, 255, 255], [0xd8, 0xd8, 0xd8]], ["white, grey under part 1", [0xd8, 0xd8, 0xd8], [255, 255, 255]]] as const) {
test(`0094 Proposal (region-only analysis per part): on M2, part 2 over the ${label} paper finds its own background`, { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const scene = await twoParts(s, m1At(400, [...right], [...left]), "m2.png");
    await pickMethod(s, "Remove background");
    await clickChip(s, 1);
    await pickMethod(s, "Remove background");
    await clickChip(s, 2);
    await settledCounts(s);
    // One view only (Overlay, §24.1): content shows as the preview outline. The square is outlined; the grey paper of
    // part 2 is background, so nothing hugs the picture's bottom edge below it (as it would if the paper were content).
    await sleep(2500);
    const img = await shot(s);
    const { box } = scene;
    const squareOutline = outlineIn(img, box, 0.6, 0.3, 0.9, 0.55);
    const bottomEdge = outlineIn(img, box, 0.55, 0.96, 0.95, 1.03);
    console.log("outline pixels: square", squareOutline, "bottom edge below the grey paper", bottomEdge);
    assert.ok(squareOutline > 20, `the square is content: ${squareOutline}`);
    assert.ok(bottomEdge < 5, `the grey paper of part 2 is background, not content: ${bottomEdge}`);
  }, "", TALL[0], TALL[1]);
});
}

test("0094 region-only analysis per part: a Threshold the maker set stays with its part through its region's read, and Add part copies it", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const scene = await twoHalves(s, m1At(400, [0xd8, 0xd8, 0xd8]), "m2.png");
    await settledCounts(s);
    console.log("part 1's automatic Threshold:", await spin(s, "Threshold"));
    await typeInto(s, "Threshold", "77");
    await settledCounts(s);
    await addPart(s);
    assert.equal(await spin(s, "Threshold"), 77, "Add part copies the value");
    // Part 2's region becomes R: its automatic values are read from R, the copied maker value stays.
    await shiftClick(s, scene.right[0], scene.right[1]);
    await waitFor("part 2 has R", async () => ((await chips(s))[1]?.name.includes("1 path") ? true : null), 20_000);
    await settledCounts(s);
    await sleep(1500);
    assert.equal(await spin(s, "Threshold"), 77, "part 2 keeps the copied value the maker set");
    await clickChip(s, 1);
    await settledCounts(s);
    assert.equal(await spin(s, "Threshold"), 77, "part 1 keeps the value the maker set");
    // Part 1 gets R too (its region changes and is read again): the value stays.
    await shiftClick(s, scene.right[0], scene.right[1]);
    await settledCounts(s);
    await sleep(1500);
    assert.equal(await spin(s, "Threshold"), 77, "a new region read leaves the maker's value");
  }, "", TALL[0], TALL[1]);
});

test("0094 region-only analysis per part (control): without a value set, part 2 reads its own automatic Threshold from its region", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    // The right half all grey paper and black square: part 2 over it alone.
    await twoParts(s, m1At(400, [0x60, 0x60, 0x60]), "dark.png");
    await settledCounts(s);
    await sleep(1500);
    const two = await spin(s, "Threshold");
    await clickChip(s, 1);
    await settledCounts(s);
    const one = await spin(s, "Threshold");
    console.log("automatic Threshold: part 1", one, "part 2", two);
    assert.ok(Number.isFinite(one) && Number.isFinite(two));
    assert.notEqual(two, one, "part 2's automatic Threshold is its region's, not a copy of part 1's");
  }, "", TALL[0], TALL[1]);
});

test("0094 Denoise (adrs.md slice 2, Crop): a part with Denoise is traced from the whole picture, a part without from its cut-out", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await twoParts(s);
    await settledCounts(s);
    await sleep(1500);
    const denoise = (choice: string) =>
      s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Denoise] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(choice)}).click()`).then(() => sleep(600));
    // Part 2 (active) with Denoise Low; part 1 without.
    await s.evaluate(`window.__traceJobs.length = 0`);
    await denoise("Low");
    await settledCounts(s);
    await sleep(2500);
    let jobs = await jobsSoFar(s);
    console.log("jobs after Denoise Low on part 2:", JSON.stringify(jobs.map((j) => [j.width, j.height, /"denoise":1/.test(j.settings)])));
    const low = jobs.filter((j) => /"denoise":1/.test(j.settings));
    assert.ok(low.length >= 1, "part 2 is traced again");
    assert.ok(low.every((j) => j.width === 400 && j.height === 400), `the Denoise part gets the whole picture: ${JSON.stringify(low.map((j) => [j.width, j.height]))}`);
    // Part 1 without Denoise: a change traces it from its cut-out.
    await clickChip(s, 1);
    await s.evaluate(`window.__traceJobs.length = 0`);
    await typeInto(s, "Smooth", "35");
    await settledCounts(s);
    await sleep(2500);
    jobs = await jobsSoFar(s);
    console.log("jobs after Smooth on part 1:", JSON.stringify(jobs.map((j) => [j.width, j.height])));
    assert.equal(jobs.length, 1, "only part 1 again");
    assert.ok(jobs[0].width < 400, `part 1 without Denoise is cut out: ${jobs[0].width} px`);
    // Denoise High on part 1: the whole picture too.
    await s.evaluate(`window.__traceJobs.length = 0`);
    await denoise("High");
    await settledCounts(s);
    await sleep(2500);
    jobs = await jobsSoFar(s);
    assert.ok(jobs.length >= 1 && jobs.every((j) => j.width === 400), `Denoise High: the whole picture: ${JSON.stringify(jobs.map((j) => [j.width, j.height]))}`);
  }, COUNT_JOBS, TALL[0], TALL[1]);
});

test("0094 slice 2: in Trace, Delete on a selected mask path's Layers row removes that path alone and keeps the image selected", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const scene = await twoHalves(s);
    // Part 1 takes R again: L and R are its mask paths and selected with the image.
    await shiftClick(s, scene.right[0], scene.right[1]);
    await waitFor("part 1 has L and R", async () => (/Inside 2 paths/.test(await advText(s)) ? true : null), 20_000);
    await settledCounts(s);
    await s.tab("Layers");
    const before = await s.layerRows();
    const selected = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-level]')].map((r) => [r.getAttribute('aria-label'), r.getAttribute('aria-selected') === 'true']))`)) as string) as [string, boolean][];
    const sel = await selected();
    console.log("rows before:", JSON.stringify(sel));
    const rects = sel.filter(([l]) => /^Rectangle/i.test(l));
    assert.equal(rects.length, 2);
    assert.ok(rects.every(([, on]) => on), "both mask paths are selected");
    const imageRow = sel.find(([l]) => l.includes("m1.png"));
    assert.ok(imageRow?.[1], "the image is selected");
    // Focus the second rectangle row (no press: the selection stays) and press Delete.
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].filter((r) => /^Rectangle/i.test(r.getAttribute('aria-label')))[1].focus()`);
    await sleep(300);
    await press(s, "Delete", 46);
    await sleep(1500);
    const after = await selected();
    console.log("rows after:", JSON.stringify(after));
    assert.equal(after.filter(([l]) => /^Rectangle/i.test(l)).length, 1, "that path alone is deleted");
    assert.equal(after.length, before.length - 1, "nothing else is deleted");
    assert.ok(after.find(([l]) => l.includes("m1.png"))?.[1], "the image stays selected");
    assert.ok(await s.evaluate(`!!document.querySelector('${BAR}')`), "the tool stays open");
    await waitFor("part 1 has one path", async () => (/Inside 1 path(?!s)/.test(await advText(s)) ? true : null), 20_000);
    assert.equal((await chips(s))[0].name, "Part 1 of 1, inside, 1 path, active");
    await settledCounts(s);
    assert.doesNotMatch(await s.statusText(), /error|failed/i);
  }, "", TALL[0], TALL[1]);
});

for (const [label, left, right] of [["grey", [255, 255, 255], [0xd8, 0xd8, 0xd8]], ["white, grey beside it", [0xd8, 0xd8, 0xd8], [255, 255, 255]]] as const) {
test(`0094 Proposal control (slice 1, criterion 9): one part over the ${label} paper alone finds its own background`, { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const scene = await twoHalves(s, m1At(400, [...right], [...left]), "m2.png");
    // One part, R only.
    await shiftClick(s, scene.right[0], scene.right[1]);
    await shiftClick(s, scene.left[0], scene.left[1]);
    await waitFor("R alone", async () => (/Inside 1 path(?!s)/.test(await advText(s)) && (await chips(s)).length === 1 ? true : null), 20_000);
    await pickMethod(s, "Remove background");
    await settledCounts(s);
    await sleep(2500);
    const img = await shot(s);
    const { box } = scene;
    const squareOutline = outlineIn(img, box, 0.6, 0.3, 0.9, 0.55);
    const bottomEdge = outlineIn(img, box, 0.55, 0.96, 0.95, 1.03);
    console.log("outline pixels of the one part: square", squareOutline, "bottom edge", bottomEdge);
    assert.ok(squareOutline > 20, "the square is content");
    assert.ok(bottomEdge < 5, "the grey paper is background");
  }, "", TALL[0], TALL[1]);
});
}

test("0094 determinism: the editor wasm replays the native parts record on two own instances", { skip: !BROWSER, timeout: 300_000 }, async () => {
  const native = JSON.parse(readFileSync(NATIVE, "utf8")) as {
    svg: string;
    viewport: [number, number];
    steps: { do: string; x?: number; y?: number }[];
    parts: { settings: string; specks: number; width: number; height: number; map: number[]; rgba_fnv32: number; wire: number[] }[];
    header: number[];
  };
  await withApp(async (s) => {
    const out = JSON.parse(
      (await s.evaluate(`(async () => {
        const fnv = (bytes) => { let h = 0x811c9dc5; for (const b of bytes) { h = Math.imul(h ^ b, 0x01000193) >>> 0; } return h >>> 0; };
        const native = ${JSON.stringify({ svg: native.svg, viewport: native.viewport, steps: native.steps, settings: native.parts.map((p) => p.settings), specks: native.parts.map((p) => p.specks) })};
        const run = async (mod) => {
          const w = new mod.WasmSession(1n);
          w.resize(native.viewport[0], native.viewport[1], 1);
          w.import_svg(new TextEncoder().encode(native.svg), 'parts.svg');
          w.set_tool('trace');
          for (const step of native.steps) {
            if (step.do === 'add_part') { w.add_trace_part(); continue; }
            w.pointer_down(step.x, step.y, true, false, false);
            w.pointer_up(step.x, step.y, true, false, false);
          }
          const id = w.trace_view().shown;
          const chips = JSON.parse(w.trace_parts());
          const parts = native.settings.map((settings, i) => {
            const src = w.trace_part_source(id, i, false, native.specks[i], false);
            const mask = chips[i].mask;
            const wire = Array.from(mod.trace_job(src.rgba, src.width, src.height, Float64Array.from(src.map), settings, { report() {} }));
            return { mask, width: src.width, height: src.height, map: Array.from(src.map), fnv: fnv(src.rgba), wire };
          });
          const flat = Float64Array.from(parts.flatMap((p) => p.wire));
          const header = Array.from(w.set_trace_joined(id, flat, Uint32Array.from(parts.map((p) => p.wire.length)), true));
          return { parts, header, chips: chips.map((c) => c.paths) };
        };
        const a = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await a.default();
        const b = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance-2');
        await b.default();
        return JSON.stringify({ one: await run(a), two: await run(b), distinct: a !== b });
      })()`)) as string,
    ) as { one: { parts: { mask: string; width: number; height: number; map: number[]; fnv: number; wire: number[] }[]; header: number[]; chips: number[] }; two: unknown; distinct: boolean };
    assert.ok(out.distinct, "two module instances");
    assert.deepEqual(out.one.chips, [1, 1, 1], "the steps built the same three parts");
    out.one.parts.forEach((p, i) => {
      const want = native.parts[i];
      assert.ok(want.settings.includes(p.mask), `part ${i + 1}: the same clip: ${p.mask} in ${want.settings}`);
      assert.deepEqual([p.width, p.height, p.fnv], [want.width, want.height, want.rgba_fnv32], `part ${i + 1}: the same cut-out pixels`);
      assert.deepEqual(p.map, want.map, `part ${i + 1}: the same map`);
      assert.deepEqual(p.wire, want.wire, `part ${i + 1}: wasm = native`);
    });
    assert.deepEqual(out.one.header, native.header, "the joined header: wasm = native");
    assert.deepEqual(out.two, out.one, "two workers = one");
  });
});

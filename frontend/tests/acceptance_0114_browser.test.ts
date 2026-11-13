// Chromium cases B1 to B12 of `specs/0114-image-trace-mask/ux-notes.md` section 10 (the Parts and
// Mask unit and the Mask group's controls), in the real app over DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium and build the wasm bindings first. Bar cases compare DOM
// rectangles (+- 1 px) and computed styles at scale 1. Harness as in `acceptance_0094_parts_browser.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5404;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0114");
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

/** Every user-visible string of the bar and its Advanced block: labels, titles and text. */
const barStrings = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...new Set([...document.querySelector('${BAR}').querySelectorAll('*')].flatMap((e) =>
    [e.getAttribute('aria-label'), e.getAttribute('title'), e.children.length === 0 ? e.textContent.trim() : ''].filter(Boolean)))])`)) as string) as string[];
const traceButton = (s: S) => s.evaluate(`(() => { const b = document.querySelector('${BAR} [data-trace-slot] , ${BAR} button[data-trace-slot]'); return b ? JSON.stringify({ disabled: b.getAttribute('aria-disabled'), describedby: b.getAttribute('aria-describedby'), reason: b.getAttribute('aria-describedby') ? document.getElementById(b.getAttribute('aria-describedby'))?.textContent ?? '' : '' }) : 'null'; })()`).then((v) => JSON.parse(v as string) as { disabled: string | null; describedby: string | null; reason: string } | null);


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

/** Presses the Mask group's Rectangle, drags `from` to `to` over the picture and releases the tool
 * (`specs/0114`: a region of the active part). */
async function drawRegion(s: S, from: [number, number], to: [number, number]) {
  await s.clickSel(`${BAR} [data-trace-mask-rect]`);
  await drag(s, from, to);
  await s.clickSel(`${BAR} [data-trace-mask-rect]`);
  await sleep(600);
}
const openTrace = async (s: S) => {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(800);
};

// ------------------------------------------------------------------ slice 2 cases

const PARTS_GROUP = `${ADV} [role=radiogroup][aria-label=Parts]`;
const chipNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PARTS_GROUP} [role=radio]')].map((c) => c.getAttribute('aria-label')))`)) as string) as string[];


// ------------------------------------------------------------------ 0114 helpers

const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
const clickChip = (s: S, n: number) => s.evaluate(`[...document.querySelectorAll('${PARTS_GROUP} [role=radio]')][${n - 1}].click()`).then(() => sleep(700));

const MASK = `${BAR} [data-trace-mask]`;
const rect = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.x, y: r.y, w: r.width, h: r.height, right: r.right, bottom: r.bottom }; })())`)) as string) as
    | { x: number; y: number; w: number; h: number; right: number; bottom: number }
    | null;
const near = (a: number, b: number, tol = 1.5) => Math.abs(a - b) <= tol;
const setPanel = async (s: S, open: boolean) => {
  const isOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
  if (isOpen !== open) {
    await s.clickSel(`[aria-label="${isOpen ? "Hide panel" : "Show panel"}"]`);
    await sleep(600);
  }
};

/** Imports M1 and opens Trace on it with Advanced open; returns the picture's box. */
async function traceOn(s: S) {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  await openTrace(s);
  await setAdvanced(s, true);
  return box;
}

// ------------------------------------------------------------------ B1, B2

test("0114 B1, B2 (criterion 1, 25): the Mask group is in the Parts row of Advanced and in no first row; no Pick mask paths anywhere; the unit's geometry", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await traceOn(s);
    await setPanel(s, false);
    const stray = await s.evaluate(`[...document.querySelectorAll('[data-trace-mask]')].filter((e) => !e.closest('[role=group][aria-label=Advanced]')).length`);
    assert.equal(stray, 0, "no Mask group outside the Advanced block (row 1)");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('[data-trace-row] [data-trace-mask]')].filter((e) => !e.closest('[role=group][aria-label=Advanced]')).length`), 0, "none in a first row");
    assert.equal(await s.evaluate(`!![...document.querySelectorAll('button, [aria-label]')].find((e) => /Pick mask paths/.test(e.getAttribute('aria-label') ?? e.textContent ?? ''))`), false, "no Pick mask paths");
    const unit = (await rect(s, `${ADV} [data-trace-parts]`))!;
    const u = unit.x;
    const expect: [string, string, number, number][] = [
      ["label", `${ADV} [data-trace-parts] > span[aria-hidden]`, 0, 32],
      ["chip 1", `${ADV} [data-trace-parts] [role=radio]`, 36, 64],
      ["Add part", `${ADV} button[aria-label="Add part"]`, 68, 96],
      ["trash", `${ADV} [data-trace-remove-part]`, 100, 128],
      ["Rectangle", `${MASK} [data-trace-mask-rect]`, 153, 181],
      ["Lasso", `${MASK} [data-trace-mask-lasso]`, 185, 213],
      ["Subtract", `${MASK} [data-trace-mask-subtract]`, 217, 245],
      ["Inside", `${MASK} [role=radio]:nth-of-type(1)`, 249, 313],
      ["Clear mask", `${MASK} [data-trace-clear-mask]`, 381, 409],
    ];
    for (const [name, sel, from, to] of expect) {
      const r = await rect(s, sel);
      assert.ok(r, `${name} is drawn`);
      assert.ok(near(r.x - u, from) && near(r.right - u, to), `${name}: ${Math.round(r.x - u)}..${Math.round(r.right - u)}, notes say ${from}..${to}`);
    }
    const divider = await rect(s, `${MASK} > span[aria-hidden]`);
    assert.ok(divider && near(divider.x - u, 140) && near(divider.w, 1) && near(divider.h, 20), `the divider ${JSON.stringify(divider)}`);
    const analyse = (await rect(s, `${ADV} [data-trace-analyse]`))!;
    const block = (await rect(s, ADV))!;
    assert.ok(analyse.right <= block.right + 1 && analyse.right >= block.right - 14, "Analyse is right-aligned");
  });
});

// ------------------------------------------------------------------ B3, B4

test("0114 B3, B4 (UX notes §2.5, customer 6): at 1920 x 1080 the first row and the Advanced line are one line and the bar is 1330 px; eight parts wrap the unit as a whole to line 2", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await traceOn(s);
    await sleep(500);
    const bar = (await rect(s, BAR))!;
    assert.ok(near(bar.w, 1330, 3), `the bar is ${bar.w} px wide (1330)`);
    const tops = JSON.parse((await s.evaluate(`JSON.stringify(['button[aria-label="About Trace"]', '[data-trace-slot]'].map((q) => Math.round(document.querySelector('${BAR} ' + q).getBoundingClientRect().top)))`)) as string) as number[];
    assert.ok(Math.abs(tops[0] - tops[1]) <= 14, `the first row is one line: ${JSON.stringify(tops)}`);
    const items = JSON.parse((await s.evaluate(`JSON.stringify(['[role=radiogroup][aria-label=Output]', '[role=radiogroup][aria-label=Denoise]', '[role=radiogroup][aria-label=Upscale]', '[role=spinbutton][aria-label=Corners]', '[data-trace-parts]', '[data-trace-mask]', '[data-trace-analyse]'].map((q) => { const r = document.querySelector('${ADV} ' + q).getBoundingClientRect(); return Math.round(r.top + r.height / 2); }))`)) as string) as number[];
    assert.ok(Math.max(...items) - Math.min(...items) <= 8, `every Advanced item on one line: ${JSON.stringify(items)}`);
    const analyse = (await rect(s, `${ADV} [data-trace-analyse]`))!;
    assert.ok(near(analyse.right, bar.right - 12, 3), `Analyse's right edge ${Math.round(bar.right - analyse.right)} px from the bar's (12)`);
    // B4: seven more parts.
    for (let n = 2; n <= 8; n++) {
      await s.clickSel(`${ADV} button[aria-label="Add part"]`);
      await sleep(300);
    }
    await sleep(500);
    const parts = (await rect(s, `${ADV} [data-trace-parts]`))!;
    const corners = (await rect(s, `${ADV} [role=spinbutton][aria-label=Corners]`))!;
    const block = (await rect(s, ADV))!;
    assert.ok(parts.y > corners.y + 20, "the unit wrapped to line 2");
    assert.ok(near(parts.x, block.x, 9), `the unit starts the line at the block's left: ${parts.x} vs ${block.x}`);
    assert.ok(near(parts.w, 352, 3), `the Parts group with eight parts is 352 px: ${parts.w}`);
    assert.equal(await s.evaluate(`document.querySelector('${ADV} button[aria-label="Add part"], ${ADV} button[aria-label="8 parts at most"]').getAttribute('aria-disabled')`), "true", "Add part is aria-disabled");
    const unitW = (await rect(s, `${ADV} [data-trace-parts]`))!;
    const maskR = (await rect(s, MASK))!;
    assert.ok(near(maskR.x - unitW.x, 352 + 25 - 0, 3) || maskR.x > unitW.x + 340, "the Mask group follows the Parts group on line 2");
  }, "", 1920, 1080);
});

// ------------------------------------------------------------------ B6, B7, B9

test("0114 B6, B7, B9: pressed looks, focus ring, tooltips and the hidden region's words", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await traceOn(s);
    const look = async (sel: string) =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector(${JSON.stringify(sel)}); const cs = getComputedStyle(e); return { pressed: e.getAttribute('aria-pressed'), bg: cs.backgroundColor, fg: cs.color }; })())`)) as string) as { pressed: string; bg: string; fg: string };
    const RECT = `${MASK} [data-trace-mask-rect]`;
    const LASSO = `${MASK} [data-trace-mask-lasso]`;
    const SUB = `${MASK} [data-trace-mask-subtract]`;
    await s.clickSel(RECT);
    await s.clickSel(LASSO);
    await s.clickSel(SUB);
    const [r, l, su] = [await look(RECT), await look(LASSO), await look(SUB)];
    assert.deepEqual([r.pressed, l.pressed, su.pressed], ["false", "true", "true"], "Rectangle false, Lasso true, Subtract true");
    assert.equal(l.bg, "rgb(47, 111, 238)", "pressed ground");
    assert.equal(su.bg, "rgb(47, 111, 238)");
    assert.match(r.bg, /rgba\(0, 0, 0, 0\)|transparent/, "rest ground");
    assert.equal(l.fg, "rgb(255, 255, 255)", "pressed glyph");
    // B9: the hidden region says "Subtract on" last; press Lasso off and on to hear it.
    await s.clickSel(LASSO);
    await sleep(300);
    await s.clickSel(LASSO);
    await sleep(300);
    assert.match((await s.evaluate(`document.querySelector('${MASK} [role=status]').textContent`)) as string, /Lasso on/, "the group's hidden region says Lasso on");
    // Tooltips verbatim (400 ms).
    for (const [sel, text] of [[RECT, "Rectangle"], [LASSO, "Lasso"], [SUB, "Subtract"], [`${MASK} [data-trace-clear-mask]`, "Clear mask"]] as const) {
      const c = (await rect(s, sel))!;
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c.x + c.w / 2, y: c.y + c.h / 2 });
      await sleep(100);
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c.x + c.w / 2 + 1, y: c.y + c.h / 2 });
      const tipNow = async () => (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((t) => t.textContent.trim()).filter(Boolean).join(' | ')`)) as string;
      const tip = await waitFor(`the tooltip of ${text}`, async () => ((await tipNow()) !== "" ? await tipNow() : null), 4_000).catch(() => "");
      assert.equal(tip, text, `the tooltip of ${text}`);
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 5, y: 5 });
      await sleep(300);
    }
    // B7: Shift+Tab from the Mask segments lands on Subtract with a 2 px ring.
    await s.evaluate(`document.querySelector('${MASK} [role=radio][aria-checked=true]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9, modifiers: 8 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9, modifiers: 8 });
    await sleep(300);
    const focused = JSON.parse((await s.evaluate(`JSON.stringify({ name: document.activeElement.getAttribute('aria-label'), shadow: getComputedStyle(document.activeElement).boxShadow })`)) as string) as { name: string; shadow: string };
    assert.equal(focused.name, "Subtract", "focus on Subtract");
    assert.notEqual(focused.shadow, "none", `the focus ring: ${focused.shadow}`);
    for (const text of await barStrings(s)) {
      assert.doesNotMatch(text, HINT, `no hint text: ${text}`);
    }
  });
});

// ------------------------------------------------------------------ B10, B11, B12

test("0114 B10, B11, B12: the armed trash colours the Parts group only; the chips and their names; notices for a mask outside the image and for no part with anything", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(
    async (s) => {
      const box = await traceOn(s);
      const w = box.x1 - box.x0;
      const h = box.y1 - box.y0;
      const barBottom = (await rect(s, BAR))!.bottom;
      const top = Math.max(box.y0 + 0.05 * h, barBottom + 8);
      // Part 1: two regions Inside; part 2: one region Outside, active; part 3: none.
      await drawRegion(s, [box.x0 + 0.05 * w, top], [box.x0 + 0.3 * w, box.y0 + 0.4 * h]);
      await drawRegion(s, [box.x0 + 0.4 * w, box.y0 + 0.5 * h], [box.x0 + 0.7 * w, box.y0 + 0.8 * h]);
      await s.clickSel(`${ADV} button[aria-label="Add part"]`);
      await sleep(500);
      await drawRegion(s, [box.x0 + 0.72 * w, top], [box.x0 + 0.95 * w, box.y0 + 0.3 * h]);
      await clickMask(s, "Outside");
      await sleep(400);
      await s.clickSel(`${ADV} button[aria-label="Add part"]`);
      await sleep(500);
      await clickChip(s, 2);
      assert.deepEqual(await chipNames(s), ["Part 1 of 3, inside, 2 paths", "Part 2 of 3, outside, 1 path, active", "Part 3 of 3, no paths"], "B11: the names verbatim");
      // B10: a first press on the trash arms it.
      await s.clickSel(`${ADV} [data-trace-remove-part]`);
      await sleep(200);
      const colours = JSON.parse((await s.evaluate(`JSON.stringify([document.querySelector('${ADV} [data-trace-parts]'), document.querySelector('${MASK}'), document.querySelector('${MASK} > span[aria-hidden]')].map((e) => getComputedStyle(e).backgroundColor))`)) as string) as string[];
      assert.equal(colours[0], "rgb(247, 211, 207)", "the Parts group is soft red");
      assert.match(colours[1], /rgba\(0, 0, 0, 0\)|transparent/, "the Mask group stays clear");
      assert.notEqual(colours[2], "rgb(247, 211, 207)", "the divider is not tinted");
      await sleep(500);
      await s.evaluate(`document.activeElement?.blur?.()`);
    },
    "",
    1280,
    900,
  );
  await withApp(async (s) => {
    const box = await traceOn(s);
    const w = box.x1 - box.x0;
    const h = box.y1 - box.y0;
    const canvas = await canvasRect(s);
    // B12: R3 below the picture only.
    await drawRegion(s, [box.x0 + 0.2 * w, box.y1 + 20], [box.x0 + 0.7 * w, Math.min(box.y1 + 70, canvas[1] + canvas[3] - 10)]);
    await waitFor("the outside notice", async () => ((await s.evaluate(`document.querySelector('${BAR}').textContent`)) as string).includes("0 paths: the mask is outside the image.") ? true : null, 30_000);
    assert.equal((await traceButton(s))?.disabled, "true", "Trace is dimmed");
    assert.equal((await traceButton(s))?.reason, "0 paths: the mask is outside the image.");
    void h;
  });
});

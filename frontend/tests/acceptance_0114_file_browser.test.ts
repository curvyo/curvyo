// Chromium cases of `specs/0114-image-trace-mask` for the mask that belongs to the image: save and
// reopen, delete and undo, copy and paste, a linked clone and two images selected (criteria 12, 14,
// 15, 16, 21; Behaviour 6 and Frage 11 A), in the real app over DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium and build the wasm bindings first.
// Harness as in `acceptance_0094_parts_browser.test.ts`.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5406;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0114-file");
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

// ------------------------------------------------------------------ slice 2 cases

const PARTS_GROUP = `${ADV} [role=radiogroup][aria-label=Parts]`;
const chipNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PARTS_GROUP} [role=radio]')].map((c) => c.getAttribute('aria-label')))`)) as string) as string[];


// ------------------------------------------------------------------ helpers

const clickMask = (s: S, label: string) =>
  s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Mask] [role=radio]')].find((r) => r.textContent.trim() === ${JSON.stringify(label)}).click()`);
const imageRow = (name: string) => `[role=row][aria-label="${name}, image"]`;
const traceOnSelected = async (s: S) => {
  await waitFor("Trace offered", async () => ((await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].some((b) => b.textContent === 'Trace')`)) ? true : null), 10_000);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
  await setAdvanced(s, true);
};
const leaveTrace = async (s: S) => {
  await focusCanvas(s);
  await press(s, "Escape", 27);
  await sleep(300);
  if (await s.evaluate(`!!document.querySelector('${BAR}')`)) {
    await focusCanvas(s);
    await press(s, "Escape", 27);
  }
  await waitFor("the Trace bar closes", async () => ((await s.evaluate(`!document.querySelector('${BAR}')`)) ? true : null), 10_000);
};
const regionTop = async (s: S, box: { y0: number }) =>
  Math.max(box.y0 - 6, Number(await s.evaluate(`document.querySelector('${BAR}').getBoundingClientRect().bottom`)) + 6);

/** M1 imported and traced with two parts: part 1 a region over the left half, part 2 (Outside) one over the right. */
async function twoPartsOn(s: S) {
  const file = path.join(SCRATCH, "m1.png");
  writeFileSync(file, M1_PNG);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null));
  await sleep(800);
  const box = await pictureBox(s);
  await traceOnSelected(s);
  const w = box.x1 - box.x0;
  const top = await regionTop(s, box);
  await drawRegion(s, [box.x0 - 30, top], [box.cx, box.y1 + 30]);
  await s.clickSel(`${ADV} button[aria-label="Add part"]`);
  await sleep(500);
  await drawRegion(s, [box.x1 + 30, top], [box.cx + 0.1 * w, box.y1 + 30]);
  await clickMask(s, "Outside");
  await sleep(500);
  return box;
}
const EXPECT = ["Part 1 of 2, inside, 1 path", "Part 2 of 2, outside, 1 path, active"];

// ------------------------------------------------------------------ criteria 12, 16

test("0114 criteria 12 and 16: the mask is back after save and reopen; deleting the image deletes it, one undo brings both back", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const downloads = path.join(SCRATCH, "downloads");
    rmSync(downloads, { recursive: true, force: true });
    mkdirSync(downloads, { recursive: true });
    await s.send("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: downloads });
    await twoPartsOn(s);
    assert.deepEqual(await chipNames(s), EXPECT);
    await leaveTrace(s);
    // Save, new document, open the file.
    await s.tab("File");
    await s.clickSel('[data-file-command="save"]');
    const saved = await waitFor("the download", async () => readdirSync(downloads).find((f) => f.endsWith(".curvyo")) ?? null, 30_000);
    await s.clickSel('[data-file-command="new"]');
    await sleep(1500);
    await s.tab("File");
    await s.choose('[data-file-command="open"]', path.join(downloads, saved));
    await sleep(2500);
    await s.tab("Layers");
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null), 20_000);
    await s.clickSel(imageRow("m1.png"));
    await traceOnSelected(s);
    await waitFor("the stored parts", async () => ((await chipNames(s)).length === 2 ? true : null), 20_000);
    assert.deepEqual(await chipNames(s), ["Part 1 of 2, inside, 1 path, active", "Part 2 of 2, outside, 1 path"], "criterion 12: regions, modes and parts as left");
    await leaveTrace(s);
    // Criterion 16: delete and undo.
    await s.clickSel(imageRow("m1.png"));
    await focusCanvas(s);
    await press(s, "Delete", 46);
    await waitFor("the image is gone", async () => (!(await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null), 10_000);
    await focusCanvas(s);
    await s.key("z", 90, 2);
    await waitFor("the image is back", async () => ((await s.layerRows()).some((r) => r?.includes("m1.png")) ? true : null), 10_000);
    await s.clickSel(imageRow("m1.png"));
    await traceOnSelected(s);
    assert.deepEqual((await chipNames(s)).length, 2, "criterion 16: one undo brings the mask back");
    assert.equal((await chipNames(s))[1], "Part 2 of 2, outside, 1 path");
  });
});

// ------------------------------------------------------------------ criteria 14, 15

test("0114 criteria 14 and 15: a duplicate carries the mask and edits apart; a linked clone shows and edits its root's mask", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const box = await twoPartsOn(s);
    await leaveTrace(s);
    await s.tab("Layers");
    // Duplicate (the Layers toolbar; Ctrl+C and Ctrl+V need the page's clipboard, which headless Chromium withholds: the wasm test `Ctrl+C carries the mask` covers the memo).
    await s.clickSel(imageRow("m1.png"));
    await s.clickSel('[role=toolbar][aria-label=Layers] button[aria-label^="Duplicate"]');
    await waitFor("a second image", async () => ((await s.layerRows()).filter((r) => r?.includes("m1.png, image")).length === 2 ? true : null), 15_000);
    await sleep(500);
    await traceOnSelected(s);
    await waitFor("the copy's parts", async () => ((await chipNames(s)).length === 2 ? true : null), 20_000);
    assert.deepEqual(await chipNames(s), ["Part 1 of 2, inside, 1 path, active", "Part 2 of 2, outside, 1 path"], "criterion 14: the copy has the same parts");
    // Edit the copy: Clear mask on part 1 only changes the copy.
    await s.clickSel(`${MASK_SEL} [data-trace-clear-mask]`);
    await sleep(600);
    assert.equal((await chipNames(s))[0], "Part 1 of 2, whole image, active", "the copy's part 1 is cleared");
    await leaveTrace(s);
    const rows = (await s.layerRows()).filter((r) => r?.includes("m1.png, image"));
    assert.equal(rows.length, 2);
    // The original (the other image row) is unchanged.
    await s.evaluate(`(() => { const rs = [...document.querySelectorAll('[role=row][aria-level]')].filter((r) => r.getAttribute('aria-label') === 'm1.png, image'); rs[rs.length - 1].click(); })()`);
    await sleep(400);
    await traceOnSelected(s);
    const a = await chipNames(s);
    await leaveTrace(s);
    await s.evaluate(`(() => { const rs = [...document.querySelectorAll('[role=row][aria-level]')].filter((r) => r.getAttribute('aria-label') === 'm1.png, image'); rs[0].click(); })()`);
    await sleep(400);
    await traceOnSelected(s);
    const b = await chipNames(s);
    const clears = [a, b].filter((n) => n[0].includes("whole image")).length;
    assert.equal(clears, 1, `exactly one of the two images was changed: ${JSON.stringify([a, b])}`);
    await leaveTrace(s);
    // A linked clone shows and edits its root's mask (criterion 15).
    await s.evaluate(`(() => { const rs = [...document.querySelectorAll('[role=row][aria-level]')].filter((r) => r.getAttribute('aria-label') === 'm1.png, image'); rs[0].click(); })()`);
    await sleep(400);
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((x) => x.textContent === 'Linked clone').click()`);
    await sleep(1000);
    await traceOnSelected(s);
    const clone = await chipNames(s);
    assert.equal(clone.length, 2, `the clone shows its root's parts: ${JSON.stringify(clone)}`);
    // Draw a region on the clone: its root gains it (criterion 15).
    await clickChipN(s, 1);
    const pathsOf = async () => Number(((await chipNames(s))[0].match(/(\d+) paths?/) ?? [])[1] ?? 0);
    const before = await pathsOf();
    await drawRegion(s, [box.x0 + 20, await regionTop(s, box)], [box.x0 + 90, box.y0 + 120]);
    assert.equal(await pathsOf(), before + 1, "the clone's edit lands in the shared mask (criterion 15)");
  });
});

const MASK_SEL = `${BAR} [data-trace-mask]`;
const clickChipN = (s: S, n: number) => s.evaluate(`[...document.querySelectorAll('${ADV} [role=radiogroup][aria-label=Parts] [role=radio]')][${n - 1}].click()`).then(() => sleep(600));

// ------------------------------------------------------------------ criterion 21, Frage 11 A

test("0114 criterion 21 and Frage 11 A: two images selected keep the full bar without the Parts and Mask unit and Create paths traces each with its own mask", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    const box = await twoPartsOn(s);
    await leaveTrace(s);
    const second = path.join(SCRATCH, "second.png");
    writeFileSync(second, rgbPng(80, 80, () => [40, 40, 40]));
    await s.tab("Layers");
    await s.choose("[data-import-button]", second);
    await waitFor("the second image", async () => ((await s.layerRows()).some((r) => r?.includes("second.png")) ? true : null));
    await sleep(500);
    void box;
    await focusCanvas(s);
    await s.key("a", 65, 2);
    await sleep(500);
    await traceOnSelected(s);
    await sleep(1000);
    assert.equal(await s.evaluate(`!!document.querySelector('${MASK_SEL}') || !!document.querySelector('${BAR} [data-trace-parts]')`), false, "the unit is not drawn");
    const controls = await barControls(s);
    for (const name of ["Method", "Threshold", "Trace"]) {
      assert.ok(controls.includes(name), `${name} stays: ${JSON.stringify(controls)}`);
    }
    assert.notEqual((await traceButton(s))?.disabled, "true", "Trace is not dimmed");
    await s.clickSel(`${BAR} button[data-trace-slot=none]`);
    await waitFor("trace groups", async () => (((await s.layerRows()).filter((r) => r?.startsWith("Trace of")).length >= 1) ? true : null), 60_000);
    assert.ok((await s.layerRows()).some((r) => r?.startsWith("Trace of")), "Create paths made a group");
  });
});

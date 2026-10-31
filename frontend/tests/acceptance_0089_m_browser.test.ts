// `specs/0089-trace-quality-and-colour` slice M in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): Output in Advanced
// (criterion 25), the palette with its count and actions (criteria 26, 29), the segmented Mask
// view, Create paths as one group of one filled path per colour in one step, and the bar at
// 800 x 600. Harness as in `acceptance_0089_e_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5209;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089m");
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


const BAR = "[role=toolbar][aria-label=Trace]";

type S = Awaited<ReturnType<typeof session>>;

async function openTrace(s: S, file: string, width: number, height: number) {
  await s.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [aria-label="More trace options"]')`)) ? true : null));
  await sleep(1200);
}

const viewChecked = async (s: S) =>
  (await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [aria-checked=true]')?.getAttribute('aria-label') ?? ''`)) as string;

const clickView = async (s: S, name: string) => {
  await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').click()`);
  await sleep(1200);
};

const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};

const escape = async (s: S) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
  await sleep(400);
};

const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};

type Shot = ReturnType<typeof decodePng>;
const at = (img: Shot, x: number, y: number) => {
  const i = (Math.round(y) * img.width + Math.round(x)) * 4;
  return [img.data[i], img.data[i + 1], img.data[i + 2]];
};

/** The screen box of the pixels of `img` within the canvas whose colour passes `test`. */
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

const canvasRect = async (s: S) =>
  JSON.parse(
    (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string,
  ) as number[];


/** Red left, blue top right, yellow bottom right, adjacent, on white: 240 x 120 px. */
const THREE = rgbPng(240, 120, (x, y) => {
  if (y < 20 || y >= 100 || x < 20 || x >= 220) return [255, 255, 255];
  if (x < 110) return [208, 34, 42];
  return y < 60 ? [35, 78, 184] : [242, 194, 14];
});

const swatches = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=listbox][aria-label=Palette] [role=option]')].map((o) => o.getAttribute('aria-label')))`)) as string) as string[];

const coloursFieldText = async (s: S) =>
  (await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label=Colours]')?.textContent ?? ''`)) as string;

test("0089 M in Chromium: Output, the palette and its actions, the segmented Mask, Create paths per colour", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "three.png");
  writeFileSync(file, THREE);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, file, 1280, 800);
    await pickMethod(s, "Remove background");

    // Criterion 25: Output lives in Advanced (not the main bar), Single colour first.
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [role=radiogroup][aria-label=Output]')`), false, "no Output in the main bar");
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
    const outputs = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=group][aria-label=Advanced] [role=radiogroup][aria-label=Output] [role=radio]')].map((r) => [r.textContent, r.getAttribute('aria-checked')]))`)) as string) as [string, string][];
    assert.deepEqual(outputs, [["Single colour", "true"], ["Multicolour", "false"]]);
    assert.deepEqual(await swatches(s), [], "no palette in Single colour");
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);

    // Criterion 26: the automatic palette finds the three colours; Colours shows Auto and the count.
    const found = await waitFor("three swatches", async () => {
      const list = await swatches(s);
      return list.length === 3 ? list : null;
    }, 30_000);
    console.log("palette:", found.join(" | "));
    assert.ok(found.some((n) => n.startsWith("#D0222A · Red")), "the red swatch names its hex and colour");
    assert.equal((await coloursFieldText(s)).replace(/\s+/g, " ").includes("Auto 3"), true, `Colours reads Auto 3: ${await coloursFieldText(s)}`);

    // Where the blocks are: the Image view, below the bar (whose swatches and button are coloured).
    const whole = await canvasRect(s);
    const barBottom = (await s.evaluate(`document.querySelector('${BAR}').getBoundingClientRect().bottom`)) as number;
    const rect = [whole[0], barBottom + 4, whole[2], whole[1] + whole[3] - barBottom - 4];
    await clickView(s, "Image");
    const image = await shot(s);
    const red = boxOf(image, rect, ([r, g, b]) => r > 180 && g < 70 && b < 70);
    const blue = boxOf(image, rect, ([r, g, b]) => b > 150 && r < 80 && g < 120);
    assert.ok(Number.isFinite(red.cx) && Number.isFinite(blue.cx), "the blocks are on screen");

    // The Mask view shows each colour as found, the background white.
    await clickView(s, "Mask");
    await waitFor("the segmented mask", async () => {
      const m = await shot(s);
      const r = at(m, red.cx, red.cy);
      const b = at(m, blue.cx, blue.cy);
      return r[0] > 180 && r[1] < 70 && b[2] > 150 && b[0] < 80 ? true : null;
    }, 20_000);
    const mask = await shot(s);
    assert.deepEqual(at(mask, red.x0 - 6, red.cy), [255, 255, 255], "the background is white in the Mask");

    // Isolate (view only): the status says so, the other colours are dimmed in the Mask.
    await s.clickSel(`${BAR} [data-trace-palette-swatch="#234EB8"]`);
    const detail = (await s.evaluate(`document.querySelector('${BAR} [data-trace-palette-detail]')?.textContent ?? ''`)) as string;
    assert.ok(detail.startsWith("#234EB8 · Blue"), detail);
    const actions = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-palette-action]')].map((b) => b.getAttribute('aria-label')))`)) as string) as string[];
    assert.deepEqual(actions, ["Isolate", "Recolour", "Merge into", "Exclude"], "an automatic colour has no Remove");
    await s.clickSel(`${BAR} [data-trace-palette-action="Isolate"]`);
    await waitFor("Isolating", async () => ((await s.statusText()).includes("Isolating #234EB8") ? true : null), 10_000);
    await waitFor("the dimmed red", async () => {
      const r = at(await shot(s), red.cx, red.cy);
      return r[1] > 120 ? true : null;
    }, 10_000);
    await s.clickSel(`${BAR} [data-trace-palette-action="Isolate"]`);

    // Recolour by its hex field (UX notes §7): the swatch is then named by its new colour and its
    // old hex; Reset takes it back.
    await s.clickSel(`${BAR} [data-trace-palette-action="Recolour"]`);
    const hexField = `${BAR} [data-trace-palette-picker] input[aria-label="Recolour hex"]`;
    await waitFor("the Recolour hex field", async () => ((await s.evaluate(`!!document.querySelector('${hexField}')`)) ? true : null), 10_000);
    await s.evaluate(`(() => { const i = document.querySelector('${hexField}'); i.focus(); i.select(); })()`);
    await s.send("Input.insertText", { text: "00a0ff" });
    await s.key("Enter", 13);
    await waitFor("the recoloured swatch name", async () => ((await swatches(s)).some((n) => n.startsWith("#00A0FF (was #234EB8) · ")) ? true : null), 10_000);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-palette-picker] button')].find((b) => b.textContent === 'Reset').click()`);
    await waitFor("the reset swatch name", async () => ((await swatches(s)).some((n) => n.startsWith("#234EB8 · Blue")) ? true : null), 10_000);
    await s.clickSel(`${BAR} [data-trace-palette-action="Recolour"]`);

    // Exclude: the colour becomes background; the swatch stays with its mark.
    await s.clickSel(`${BAR} [data-trace-palette-action="Exclude"]`);
    await waitFor("1 colour excluded", async () => ((await s.statusText()).includes("1 colour excluded") ? true : null), 20_000);
    assert.ok((await swatches(s)).some((n) => n.startsWith("#234EB8") && n.includes("excluded")));
    // Sampled near the block's bottom edge: the grown bar's shadow falls on its top.
    await waitFor("the excluded blue is white in the Mask", async () => (at(await shot(s), blue.cx, blue.y1 - 3).every((c) => c > 240) ? true : null), 20_000);

    // Criterion 29: Create paths writes one group with one filled path per traced colour, in one step.
    const before = (await s.layerRows()).length;
    await s.clickSel(`${BAR} [data-trace-slot="none"]`);
    await waitFor("the traced notice", async () => ((await s.statusText()).includes("Traced: 2 colours,") ? true : null), 30_000);
    await s.tab("Layers");
    await sleep(500);
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => r.getAttribute('aria-label').startsWith('Trace of'))?.querySelector('[aria-label=Expand]')?.click()`);
    await sleep(500);
    const rows = await s.layerRows();
    console.log("layer rows:", rows.join(" | "));
    assert.ok(rows.some((r) => r?.includes("Trace of three.png")), "the group is named after the image");
    assert.ok(rows.some((r) => r?.includes("#D0222A")) && rows.some((r) => r?.includes("#F2C20E")), "one path per colour, named by its hex");
    assert.ok(!rows.some((r) => r?.includes("#234EB8")), "the excluded colour is not traced");
    await s.key("z", 90, 2);
    await waitFor("the undo", async () => ((await s.layerRows()).length === before ? true : null), 10_000);
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

test("0089 M in Chromium: Multicolour at 800 x 600 keeps the bar inside the window", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "three.png");
  writeFileSync(file, THREE);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, file, 800, 600);
    await pickMethod(s, "Remove background");
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(400);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await waitFor("three swatches", async () => ((await swatches(s)).length === 3 ? true : null), 30_000);
    await s.clickSel(`${BAR} [data-trace-palette-swatch="#D0222A"]`);
    const sizes = JSON.parse((await s.evaluate(`JSON.stringify((() => { const bar = document.querySelector('${BAR}').getBoundingClientRect();
      const out = [...document.querySelectorAll('${BAR} *')].filter((e) => e.getBoundingClientRect().right > window.innerWidth + 0.5).length;
      return { height: Math.round(bar.height), right: Math.round(bar.right), out }; })())`)) as string) as { height: number; right: number; out: number };
    console.log(`800 x 600, Multicolour, palette and detail row, Advanced open: bar ${sizes.height} px`);
    assert.equal(sizes.out, 0, "nothing past the window's edge");
    assert.ok(sizes.right <= 800);
    // Pick colours + Multicolour: the Colours field is greyed out (Frage 90 A).
    await pickMethod(s, "Pick colours");
    await escape(s);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [data-trace-colours-fixed]')?.getAttribute('aria-disabled')`), "true");
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

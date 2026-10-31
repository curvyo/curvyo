// Tester cases for `specs/0089-trace-quality-and-colour` slice M in the real app (Chromium via
// DevTools; set CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Written
// from the specification and the UX notes (criteria 25 to 29, Frage 81 A, 90 A, 92 B and the
// lead's decision that the Mask view of Multicolour shows the palette colours segmented) before
// the code was read, and kept apart from `acceptance_0089_m_browser.test.ts`. Harness as in
// `acceptance_0089_sliceE_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5251;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-sliceM-tester");
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

async function launch(preload?: string) {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
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
    vite.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
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
const typeInto = async (s: S, name: string, text: string) => {
  await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label="${name}"]').focus()`);
  await press(s, "F2", 113);
  await s.evaluate(`document.activeElement?.select?.()`);
  await s.send("Input.insertText", { text });
  await press(s, "Enter", 13);
};
const clickView = async (s: S, name: string) => {
  await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [role=radio][aria-label=${name}]').click()`);
  await sleep(1000);
};
const shot = async (s: S) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  return decodePng(Buffer.from(out.result?.data ?? "", "base64"));
};
const savePng = async (s: S, name: string) => {
  const out = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
  writeFileSync(path.join(SCRATCH, name), Buffer.from(out.result?.data ?? "", "base64"));
};
type Shot = ReturnType<typeof decodePng>;
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
const isBlack = ([r, g, b]: number[]) => r < 40 && g < 40 && b < 40;

/** Imports `png` as `name` and opens the Trace bar on it. */
async function openOn(s: S, name: string, png: Buffer) {
  const file = path.join(SCRATCH, name);
  writeFileSync(file, png);
  await s.tab("Layers");
  const before = (await s.layerRows()).length;
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).length > before ? true : null));
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
  await sleep(1000);
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

// ------------------------------------------------------------------ criterion 21: Mask, pixel by pixel

/** 64 x 64: a bar on the left border, a big square, a wide shape with a hole, and a 2 x 2 speck. */

// ------------------------------------------------------------------ pictures

/** 320 x 140: red, green and blue blocks, adjacent; two near-greys; a pale accent. White ground. */
const RED = [200, 30, 30];
const GREEN = [30, 160, 60];
const BLUE = [30, 60, 200];
const GREY_A = [0xd9, 0xd9, 0xd9];
const GREY_B = [0xdc, 0xd6, 0xcf];
const PALE = [0xff, 0xf2, 0xa8];
const PICTURE = rgbPng(320, 140, (x, y) => {
  if (x >= 10 && x < 22 && y >= 10 && y < 22) return PALE as [number, number, number];
  if (y >= 40 && y < 130) {
    if (x >= 20 && x < 100) return RED as [number, number, number];
    if (x >= 100 && x < 160) return y < 85 ? (GREEN as [number, number, number]) : (BLUE as [number, number, number]);
    if (x >= 190 && x < 240) return GREY_A as [number, number, number];
    if (x >= 240 && x < 290) return GREY_B as [number, number, number];
  }
  return [255, 255, 255];
});
const hex = (c: number[]) => `#${c.map((v) => v.toString(16).padStart(2, "0")).join("").toUpperCase()}`;
const fromHex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const distance = (a: number[], b: number[]) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) / 255;

const HINT = /\b(click|press|drag|tap|hold|try|use|tip|hint|you can|choose|select a|to (pick|add|remove|create))\b/i;
const OUTPUT = `${BAR} [role=radiogroup][aria-label=Output]`;
const PALETTE = `${BAR} [role=listbox][aria-label=Palette]`;
const swatchNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PALETTE} [role=option]')].map((o) => o.getAttribute('aria-label')))`)) as string) as string[];
const openAdvanced = async (s: S) => {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [role=group][aria-label=Advanced]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
};
const multicolour = async (s: S) => {
  await openAdvanced(s);
  await s.evaluate(`[...document.querySelectorAll('${OUTPUT} [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
  await sleep(1500);
};
/** Multicolour with the Colours field fixed to 6 (the automatic count is a separate todo test). */
const sixColours = async (s: S) => {
  await multicolour(s);
  await typeInto(s, "Colours", "6");
  await waitFor("six swatches", async () => ((await swatchNames(s)).length === 6 ? true : null), 30_000);
};
const swatch = (h: string) => `${PALETTE} [role=option][aria-label^="${h}"]`;
const action = (name: string) => `${BAR} [data-trace-palette-action="${name}"]`;

test("0089M tester c25, c26, c29: Output sits in Advanced, the palette is exactly what is in the picture (pale accent, near-greys), and every swatch is readable without colour", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-picture.png", PICTURE);
    await pickMethod(s, "Remove background");
    // c25: not in the main bar; Single colour is the default; two segments in the Advanced block.
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} > [role=radiogroup][aria-label=Output]')`), false);
    await openAdvanced(s);
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [role=group][aria-label=Advanced] [role=radiogroup][aria-label=Output]')`), true, "Output is in Advanced");
    const segments = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${OUTPUT} [role=radio]')].map((r) => [r.textContent, r.getAttribute('aria-checked')]))`)) as string;
    assert.deepEqual(JSON.parse(segments), [["Single colour", "true"], ["Multicolour", "false"]]);
    assert.deepEqual(await swatchNames(s), [], "Single colour has no palette");
    await sixColours(s);

    // c26: red, green, blue, the two greys 0.043 apart, and the pale accent: six swatches, none
    // for an anti-aliasing seam (the picture has hard edges, so any extra swatch is a seam).
    const names = await waitFor("the palette", async () => {
      const list = await swatchNames(s);
      return list.length >= 6 ? list : null;
    }, 30_000);
    console.log("palette:", names.join(" | "));
    const hexes = names.map((n) => n.slice(0, 7));
    for (const want of [RED, GREEN, BLUE, GREY_A, GREY_B, PALE]) {
      assert.ok(hexes.some((h) => distance(fromHex(h), want) < 0.03), `a swatch near ${hex(want)} in ${hexes.join(", ")}`);
    }
    assert.equal(names.length, 6, `exactly the six colours: ${names.join(" | ")}`);
    // The two greys are separate swatches (merge distance 0.03 is below their 0.043).
    assert.ok(hexes.filter((h) => distance(fromHex(h), GREY_A) < 0.03 || distance(fromHex(h), GREY_B) < 0.03).length === 2);

    // c29, no colour perception: hex and a colour name in every accessible name; text, not hue.
    for (const n of names) {
      assert.match(n, /^#[0-9A-F]{6} · [A-Za-z][A-Za-z ]+/, `hex and name in "${n}"`);
    }
    // Roving tabindex: one Tab stop; the arrow keys move the focus and the selection.
    const stops = (await s.evaluate(`[...document.querySelectorAll('${PALETTE} [role=option]')].filter((o) => o.tabIndex === 0).length`)) as number;
    assert.equal(stops, 1, "one Tab stop in the strip");
    await s.evaluate(`document.querySelector('${PALETTE} [role=option][tabindex="0"]').focus()`);
    const first = (await s.evaluate(`document.activeElement.getAttribute('aria-label')`)) as string;
    await press(s, "ArrowRight", 39);
    const second = (await s.evaluate(`document.activeElement.getAttribute('aria-label')`)) as string;
    assert.notEqual(first, second, "ArrowRight moves to the next swatch");
    assert.equal(second, names[1]);
    // Every state is a word: Exclude it from the keyboard-reachable detail row and read the name.
    if (!(await s.evaluate(`!!document.querySelector('${BAR} [data-trace-palette-detail]')`))) {
      await s.clickSel(swatch(hexes[1]));
    }
    assert.ok(((await s.evaluate(`document.querySelector('${BAR} [data-trace-palette-detail]')?.textContent ?? ''`)) as string).startsWith(hexes[1]), "the arrow-selected swatch has the detail row");
    await s.clickSel(action("Exclude"));
    const excluded = await waitFor("excluded in the name", async () => {
      const list = await swatchNames(s);
      return list.find((n) => n.startsWith(hexes[1]) && /excluded/i.test(n)) ?? null;
    }, 20_000);
    assert.ok(excluded);
    const detail = (await s.evaluate(`document.querySelector('${BAR} [data-trace-palette-detail]')?.textContent ?? ''`)) as string;
    assert.match(detail, /Excluded/);
    await s.clickSel(action("Exclude"));

    // Controls and status of the new UI carry no operating tips.
    const texts = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [role=group][aria-label=Advanced] *, ${PALETTE} *, ${BAR} [data-trace-palette-detail] *')].flatMap((e) => [e.children.length === 0 ? e.textContent : '', e.getAttribute('title') ?? '', e.getAttribute('aria-label') ?? '']).filter((t) => t.trim() !== ''))`)) as string;
    const tips = (JSON.parse(texts) as string[]).concat(await s.statusText()).filter((t) => HINT.test(t));
    console.log("hint-like text:", JSON.stringify(tips));
    assert.deepEqual(tips, [], "no operating tips");
  });
});

test("0089M tester c29: Merge into reassigns, one Create paths is one group of the traced colours in one undo step", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-three.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hexes = (await swatchNames(s)).map((n) => n.slice(0, 7));
    const near = (c: number[]) => hexes.find((h) => distance(fromHex(h), c) < 0.03)!;
    const [red, green] = [near(RED), near(GREEN)];
    // Merge red into green.
    await s.clickSel(swatch(red));
    await s.clickSel(action("Merge into"));
    await waitFor("choosing mode", async () => (/Merging/.test(await s.statusText()) ? true : null), 10_000);
    await s.clickSel(swatch(green));
    const merged = await waitFor("the merged swatch", async () => (await swatchNames(s)).find((n) => n.startsWith(red) && /merged into/i.test(n)) ?? null, 20_000);
    assert.match(merged, new RegExp(green), "the name says what it was merged into");
    // Create paths: five colours remain (the merged one is traced with its target).
    const before = (await s.layerRows()).length;
    await s.clickSel(`${BAR} [data-trace-slot="none"]`);
    await waitFor("the traced notice", async () => (/Traced: 5 colours,/.test(await s.statusText()) ? true : null), 30_000);
    await s.tab("Layers");
    await sleep(500);
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => r.getAttribute('aria-label').startsWith('Trace of'))?.querySelector('[aria-label=Expand]')?.click()`);
    await sleep(500);
    const rows = (await s.layerRows()).filter(Boolean);
    console.log("rows:", rows.join(" | "));
    assert.equal(rows.filter((r) => r.startsWith("Trace of")).length, 1, "one group");
    assert.ok(!rows.some((r) => r.includes(red)), "the merged colour has no path of its own");
    assert.ok(rows.some((r) => r.includes(green)), "its target has");
    const hexRows = rows.filter((r) => /^#[0-9A-F]{6}/.test(r));
    assert.equal(hexRows.length, 5, `five paths named by hex: ${hexRows.join(", ")}`);
    // The largest area is at the bottom: the layer panel lists the top first, so it is last.
    assert.ok(hexRows[hexRows.length - 1].startsWith(green), `the merged green+red is the largest and lowest: ${hexRows.join(", ")}`);
    await s.key("z", 90, 2);
    await waitFor("one undo takes the whole trace back", async () => ((await s.layerRows()).length === before ? true : null), 10_000);
  });
});

test("0089M tester: Multicolour is remembered as an app setting, Single colour needs no palette, and Colours fixes the count", { skip: !BROWSER, timeout: 150_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-count.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    await typeInto(s, "Colours", "3");
    await waitFor("three swatches", async () => ((await swatchNames(s)).length === 3 ? true : null), 30_000);
    await typeInto(s, "Colours", "40");
    const clamped = await waitFor("the clamp", async () => {
      const t = (await s.evaluate(`document.querySelector('${BAR} [role=spinbutton][aria-label=Colours]')?.textContent ?? ''`)) as string;
      return /16/.test(t) ? t : null;
    }, 20_000);
    assert.ok(clamped);
    // Reload: the app setting survives.
    // Remembered as an app setting: the stored settings name Multicolour (a reload would stop at
    // the unsaved-document prompt).
    const stored = (await s.evaluate(`JSON.stringify(Object.fromEntries(Object.entries(localStorage).filter(([k]) => k.startsWith('curvyo.'))))`)) as string;
    console.log("stored:", stored);
    const checked = stored.includes('\\"output\\":1') ? "Multicolour" : "";
    assert.equal(checked, "Multicolour", "Output is remembered");
  });
});

test("0089M tester c29, lead decision: in Multicolour the Mask view shows each palette colour in its place, the background white; Single colour shows black", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-mask.png", PICTURE);
    await pickMethod(s, "Remove background");
    const whole = await canvasRect(s);
    const rectBelowBar = async () => {
      const bottom = await barBottom(s);
      return [whole[0], bottom + 4, whole[2], whole[1] + whole[3] - bottom - 4];
    };
    await clickView(s, "Image");
    const rect = await rectBelowBar();
    const image = await shot(s);
    const near = (c: number[]) => ([r, g, b]: number[]) => distance([r, g, b], c) < 0.06;
    // The greys are found by geometry from the red block (UI chrome has greys too).
    const red = boxOf(image, rect, near(RED));
    const k = (red.x1 - red.x0 + 1) / 80;
    const sub = (x0: number, x1: number, y0: number, y1: number) => {
      const px = red.x0 - 20 * k;
      const py = red.y0 - 40 * k;
      const [a, b, c, d] = [px + x0 * k, px + x1 * k - 1, py + y0 * k, py + y1 * k - 1];
      return { x0: a, y0: c, x1: b, y1: d, cx: (a + b) / 2, cy: (c + d) / 2 };
    };
    const boxes = [red, boxOf(image, rect, near(GREEN)), boxOf(image, rect, near(BLUE)), sub(190, 240, 40, 130), sub(240, 290, 40, 130)];
    for (const b of boxes) {
      assert.ok(Number.isFinite(b.cx), "the blocks are on screen in the Image view");
    }
    // Single colour first: black union.
    await clickView(s, "Mask");
    const single = await shot(s);
    assert.ok(isBlack(at(single, boxes[0].cx, boxes[0].cy)) && isBlack(at(single, boxes[2].cx, boxes[2].cy)), "Single colour: black");
    await sixColours(s);
    await waitFor("the segmented mask", async () => {
      const m = await shot(s);
      return distance(at(m, boxes[0].cx, boxes[0].cy), RED) < 0.06 ? true : null;
    }, 30_000);
    const mask = await shot(s);
    // Every sampled interior pixel of every block shows its own colour (within the screen's rounding).
    const wants = [RED, GREEN, BLUE, GREY_A, GREY_B];
    boxes.forEach((b, i) => {
      for (const fx of [0.25, 0.5, 0.75]) {
        for (const fy of [0.25, 0.5, 0.75]) {
          const px = at(mask, b.x0 + (b.x1 - b.x0) * fx, b.y0 + (b.y1 - b.y0) * fy);
          assert.ok(distance(px, wants[i]) < 0.06, `block ${i} at ${fx},${fy} shows ${hex(px)}, want ${hex(wants[i])}`);
        }
      }
    });
    assert.deepEqual(at(mask, boxes[0].x0 - 8, boxes[0].cy), [255, 255, 255], "the background is white");
    await savePng(s, "mask-multicolour.png");
    // Excluded: the colour turns white in the Mask.
    const names = await swatchNames(s);
    const redHex = names.map((n) => n.slice(0, 7)).find((h) => distance(fromHex(h), RED) < 0.03)!;
    await s.clickSel(swatch(redHex));
    await s.clickSel(action("Exclude"));
    await waitFor("the excluded red is white", async () => (at(await shot(s), boxes[0].cx, boxes[0].y1 - 3).every((c) => c > 240) ? true : null), 20_000);
  });
});

test("0089M tester c29: Recolour changes the fill of the written path and nothing else", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-recolour.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hexes0 = (await swatchNames(s)).map((n) => n.slice(0, 7));
    const blue = hexes0.find((h) => distance(fromHex(h), BLUE) < 0.03)!;
    await s.clickSel(swatch(blue));
    await s.clickSel(action("Recolour"));
    await sleep(500);
    // The picker (UX notes 7 speak of a hex field; the build offers the hex as text with an area
    // and a hue slider): drive the hue slider from the keyboard.
    await s.evaluate(`document.querySelector('${BAR} [data-trace-palette-picker] [role=slider][aria-label="Recolour hue"]').focus()`);
    for (let n = 0; n < 6; n++) {
      await press(s, "ArrowLeft", 37);
    }
    const detail = await waitFor("the detail row names the new colour", async () => {
      const t = (await s.evaluate(`document.querySelector('${BAR} [data-trace-palette-detail]')?.textContent ?? ''`)) as string;
      return new RegExp(`was ${blue}`).test(t) ? t : null;
    }, 20_000);
    const newHex = detail.match(/#[0-9A-F]{6}/)![0];
    assert.notEqual(newHex, blue);
    console.log("recolour:", detail, "| swatch name:", (await swatchNames(s)).find((n) => n.startsWith(blue)));
    const after = await swatchNames(s);
    assert.equal(after.length, 6, "no refit: the palette keeps its six colours");
    await s.clickSel(`${BAR} [data-trace-slot="none"]`);
    await waitFor("the traced notice", async () => (/Traced: 6 colours,/.test(await s.statusText()) ? true : null), 30_000);
    await s.tab("Layers");
    await sleep(500);
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => r.getAttribute('aria-label').startsWith('Trace of'))?.querySelector('[aria-label=Expand]')?.click()`);
    await sleep(500);
    const rows = (await s.layerRows()).filter(Boolean);
    console.log("rows:", rows.join(" | "));
    assert.ok(rows.some((r) => r.includes(newHex)), "the path is filled with the new colour");
    assert.ok(!rows.some((r) => r.includes(blue)), "the old colour is gone from the result");
  });
});

test("0089M tester c26: with the automatic count the palette keeps the pale accent and both near-greys", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "m-auto.png", PICTURE);
    await pickMethod(s, "Remove background");
    await multicolour(s);
    await sleep(6000);
    const hexes = (await swatchNames(s)).map((n) => n.slice(0, 7));
    for (const want of [RED, GREEN, BLUE, GREY_A, GREY_B, PALE]) {
      assert.ok(hexes.some((h) => distance(fromHex(h), want) < 0.03), `a swatch near ${hex(want)} in ${hexes.join(", ")}`);
    }
  });
});

// Tester cases for the Multicolour palette as a swatch row (`specs/0089-trace-quality-and-colour/ux-notes.md` §26), written
// from §26 before the code was read: one row beside the palette pipette, no detail row / Recolour / Merge into, tooltip,
// keys, one line at 800 x 600, the right press, About Trace and the user page. Changed by design with §27 (customer
// 2026-11-09): no Isolate; the x deletes (the result equals a trace with the smaller palette, compared as wires on an own
// wasm instance) down to no colour; Ctrl+click excludes; a plain click elsewhere does nothing; the palette pipette stays
// with Remove background. Real app in Chromium (CURVYO_E2E_BROWSER). Harness as in `acceptance_0089_sliceM_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";


const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5281;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089-palette-row-tester");
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

/** A click on `sel` at (`dx`, `dy`) from its middle (CDP modifiers 2 = Ctrl). */
const clickOn = async (s: S, sel: string, modifiers = 0, dx = 0, dy = 0) => {
  const c = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
  for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
    await s.send("Input.dispatchMouseEvent", { type, x: c[0] + dx, y: c[1] + dy, button: "left", clickCount: 1, modifiers });
  }
  await sleep(300);
};
/** A Ctrl+click on the middle of `sel`: Exclude on or off (§27). */
const shiftClick = (s: S, sel: string) => clickOn(s, sel, 2);
/** A click on the swatch's x, in its middle (shown on hover): delete (§27). */
const clickX = (s: S, sel: string) => clickOn(s, sel);
const press = async (s: S, k: string, vk: number, modifiers = 0) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: k, windowsVirtualKeyCode: vk, modifiers });
  await sleep(350);
};
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
const fromHex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const distance = (a: number[], b: number[]) => Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]) / 255;

const OUTPUT = `${BAR} [role=radiogroup][aria-label=Output]`;
const PALETTE = `${BAR} [role=group][aria-label=Palette]`;
const swatchNames = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${PALETTE} [data-trace-palette-swatch]')].map((o) => o.getAttribute('aria-label')))`)) as string) as string[];
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
const swatch = (h: string) => `${PALETTE} [data-trace-palette-swatch="${h}"]`;

// ------------------------------------------------------------------ helpers of this file

const hiddenStatus = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} [data-trace-hidden-status]')?.textContent ?? ''`)) as string;
const centre = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector(${JSON.stringify(sel)}).getBoundingClientRect(); return [b.x + b.width / 2, b.y + b.height / 2]; })())`)) as string) as [number, number];
const moveTo = async (s: S, sel: string, modifiers = 0) => {
  const c = await centre(s, sel);
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: c[0], y: c[1], modifiers });
  await sleep(300);
};
const away = async (s: S) => {
  await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 700 });
  await sleep(300);
};
const rects = async (s: S, sel: string) =>
  JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll(${JSON.stringify(sel)})].map((e) => { const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; }))`)) as string) as number[][];

/** Records every trace request (with a copy of its pixels) and answer of the trace worker. */
const RECORD = `(() => {
  window.__reqs = []; window.__resps = {};
  const Base = window.Worker;
  window.Worker = class extends Base {
    constructor(...a) { super(...a); this.addEventListener('message', (e) => { if (e.data && e.data.kind === 'trace') window.__resps[e.data.id] = Array.from(e.data.wire); }); }
    postMessage(m, t) { if (m && m.type === 'trace') window.__reqs.push({ id: m.id, settings: m.settings, map: m.map, width: m.width, height: m.height, rgba: m.rgba.slice() }); return super.postMessage(m, t); }
  };
})();`;
type Req = { id: number; settings: string; n: number; done: boolean };
const lastRequest = async (s: S) =>
  JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = window.__reqs[window.__reqs.length - 1]; return r ? { id: r.id, settings: r.settings, n: window.__reqs.length, done: !!window.__resps[r.id] } : null; })())`)) as string) as Req | null;
const settled = async (s: S) =>
  waitFor("the last trace answered", async () => {
    const r = await lastRequest(s);
    return r?.done ? r : null;
  }, 60_000);
/** The app's answer to its last request, and the same pixels traced with `settings` on an own wasm instance. */
const wires = async (s: S, settings: string) =>
  JSON.parse(
    (await s.evaluate(`(async () => {
      const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
      await m.default();
      const r = window.__reqs[window.__reqs.length - 1];
      const mine = Array.from(m.trace_job(r.rgba, r.width, r.height, Float64Array.from(r.map), ${JSON.stringify(settings)}, { report() {} }));
      return JSON.stringify({ app: window.__resps[r.id], mine });
    })()`)) as string,
  ) as { app: number[]; mine: number[] };
const coloursN = async (s: S, n: number) => {
  await multicolour(s);
  await typeInto(s, "Colours", String(n));
  await waitFor(`${n} swatches`, async () => ((await swatchNames(s)).length === n ? true : null), 30_000);
  await sleep(1500);
};
const hexesOf = async (s: S) => (await swatchNames(s)).map((n) => n.slice(0, 7));

test("0089 §26 tester: the palette is one row of 20 x 20 swatches in 24 x 28 cells; no detail row, no Recolour, no Merge into, no action icons", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "row.png", PICTURE);
    await pickMethod(s, "Remove background");
    await coloursN(s, 6);
    const cells = await rects(s, `${PALETTE} [data-trace-palette-swatch]`);
    assert.equal(cells.length, 6);
    assert.equal(new Set(cells.map((c) => Math.round(c[1]))).size, 1, "all swatches share one line");
    for (const c of cells) {
      assert.equal(Math.round(c[2]), 24, `cell width ${c}`);
      assert.equal(Math.round(c[3]), 28, `cell height ${c}`);
    }
    for (let i = 1; i < cells.length; i++) {
      assert.ok(Math.abs(cells[i][0] - (cells[i - 1][0] + 24)) <= 1, "cells touch");
    }
    const chip = JSON.parse((await s.evaluate(`JSON.stringify((() => { const e = document.querySelector('${PALETTE} [data-trace-palette-swatch]'); return [e, ...e.querySelectorAll('*')].map((x) => { const b = x.getBoundingClientRect(); return [Math.round(b.width), Math.round(b.height)]; }); })())`)) as string) as number[][];
    assert.ok(chip.some(([w, h]) => w === 20 && h === 20), `a 20 x 20 swatch inside the cell: ${JSON.stringify(chip)}`);
    assert.equal(await s.evaluate(`document.querySelector('${PALETTE}').getAttribute('role')`), "group");
    const groupWidth = (await rects(s, PALETTE))[0][2];
    assert.ok(Math.abs(groupWidth - 6 * 24) <= 8, `the row is as wide as its colours: ${groupWidth}`);
    const bad = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('*')].flatMap((e) => [e.children.length === 0 ? e.textContent : '', e.getAttribute('aria-label') ?? '', e.getAttribute('title') ?? '']).filter((t) => /recolou?r|merge into|merging|merged|\\(was |enter 6 hex/i.test(t)))`)) as string;
    assert.deepEqual(JSON.parse(bad), [], "no Recolour / Merge into text anywhere");
    assert.equal(await s.evaluate(`document.querySelectorAll('[data-trace-palette-detail], [data-trace-palette-action], [data-trace-palette-picker], ${PALETTE} [role=option], [role=listbox][aria-label=Palette]').length`), 0, "no detail row, actions, picker, listbox");
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} button')].filter((b) => /^(Isolate|Exclude|Remove|Recolour|Reset|Merge into)$/.test(b.getAttribute('aria-label') ?? b.textContent)).length`), 0, "no action buttons");
  });
});

test("0089 §26, §27 tester: the row follows the palette pipette on its line with Brightness and with Remove background; Pick colours has no row", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "pipe.png", PICTURE);
    await pickMethod(s, "Brightness");
    await multicolour(s);
    await waitFor("swatches", async () => ((await swatchNames(s)).length >= 2 ? true : null), 30_000);
    const pipette = `${BAR} [data-trace-pipette][aria-label="Pick palette colour from the image"]`;
    const p = (await rects(s, pipette))[0];
    const first = (await rects(s, `${PALETTE} [data-trace-palette-swatch]`))[0];
    assert.ok(p && first, "the palette pipette and a swatch exist");
    assert.ok(first[0] >= p[0] + p[2] && first[0] - (p[0] + p[2]) <= 8, `the first cell sits right after the pipette: ${first[0] - (p[0] + p[2])}`);
    assert.ok(Math.abs(first[1] + first[3] / 2 - (p[1] + p[3] / 2)) <= 4, "same line as the pipette");
    await pickMethod(s, "Remove background");
    await waitFor("swatches", async () => ((await swatchNames(s)).length >= 2 ? true : null), 30_000);
    // §27 (customer 2026-11-09): the palette pipette stays with Remove background.
    const p2 = (await rects(s, pipette))[0];
    assert.ok(p2, "the palette pipette stays with Remove background");
    const row = (await rects(s, `${PALETTE} [data-trace-palette-swatch]`))[0];
    assert.ok(row[0] >= p2[0] + p2[2] && row[0] - (p2[0] + p2[2]) <= 8, `the row follows the pipette: gap ${row[0] - (p2[0] + p2[2])}`);
    await pickMethod(s, "Pick colours");
    await sleep(800);
    assert.equal(await s.evaluate(`!!document.querySelector('${PALETTE}')`), false, "no palette row with Pick colours");
  });
});

test("0089 §26 tester: tooltip is '#RRGGBB · Name' plus the shortcut line; a plain hover does nothing else", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "tip.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const first = (await swatchNames(s))[0];
    const hexFirst = first.slice(0, 7);
    await moveTo(s, swatch(hexFirst));
    const tip = await waitFor("the tooltip", async () => {
      const t = (await s.evaluate(`[...document.querySelectorAll('[role=tooltip]')].map((e) => e.innerText).join('|')`)) as string;
      return t.includes(hexFirst) ? t : null;
    }, 5_000);
    const lines = tip.split("\n").map((l) => l.trim()).filter(Boolean);
    assert.equal(lines.length, 2, `two lines: ${JSON.stringify(lines)}`);
    assert.equal(lines[0], first, "line 1 is hex · name");
    assert.equal(lines[1], "Ctrl+click: exclude");
    assert.ok(!/excluded|Removed/.test(await hiddenStatus(s)), `hover alone excludes and removes nothing: ${await hiddenStatus(s)}`);
    assert.equal((await swatchNames(s)).length, 6);
    await away(s);
  });
});

test("0089 §27 tester: Ctrl+click excludes and a second Ctrl+click brings the colour back; a plain click beside the x does nothing; there is no Isolate", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "click.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hexes = await hexesOf(s);
    const target = hexes[1];
    await shiftClick(s, swatch(target));
    await waitFor("excluded", async () => ((await swatchNames(s)).find((n) => n.startsWith(target))?.endsWith(", excluded") ? true : null), 20_000);
    await waitFor("status", async () => ((await hiddenStatus(s)).includes("1 colour excluded") ? true : null), 10_000);
    const req = await settled(s);
    assert.ok((JSON.parse(req.settings) as { excluded: string[] }).excluded.includes(target), "the trace carries the exclusion");
    assert.equal((await swatchNames(s)).length, 6, "an excluded colour stays in the row");
    const rowsBefore = (await s.layerRows()).length;
    await shiftClick(s, swatch(target));
    await waitFor("back", async () => (!(await swatchNames(s)).find((n) => n.startsWith(target))?.includes("excluded") ? true : null), 20_000);
    assert.ok(!(await hiddenStatus(s)).includes("excluded"), await hiddenStatus(s));
    assert.equal((await s.layerRows()).length, rowsBefore, "no document change");
    await away(s);
    await settled(s);
    const before = (await lastRequest(s))!.n;
    // A plain click on the swatch's edge (outside the 12 px x) does nothing.
    await clickOn(s, swatch(hexes[2]), 0, -8, -8);
    await sleep(1500);
    assert.equal((await swatchNames(s)).length, 6, "nothing deleted");
    assert.ok(!(await swatchNames(s))[2].endsWith(", excluded"), "nothing excluded");
    assert.equal((await lastRequest(s))!.n, before, "nothing traced again");
    // Ctrl held while pointing changes nothing either (Isolate is gone).
    await moveTo(s, swatch(hexes[2]), 2);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 2 });
    await sleep(800);
    assert.ok(!(await hiddenStatus(s)).includes("Isolating"), await hiddenStatus(s));
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 0 });
    await away(s);
  }, RECORD);
});

test("0089 §26, §27 tester: the x deletes a colour; the result equals a trace with the smaller palette (wires exactly); the others stay; down to no colour, then nothing is traced and Trace is dimmed", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "del.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hexes = await hexesOf(s);
    const gone = hexes.find((h) => distance(fromHex(h), RED) < 0.03)!;
    const keptExcluded = hexes.find((h) => distance(fromHex(h), BLUE) < 0.03)!;
    await shiftClick(s, swatch(keptExcluded));
    await sleep(1500);
    await shiftClick(s, swatch(gone));
    await sleep(2500);
    const before = await settled(s);
    await clickX(s, swatch(gone));
    const left = await waitFor("five swatches", async () => {
      const l = await hexesOf(s);
      return l.length === 5 ? l : null;
    }, 40_000);
    assert.deepEqual([...left].sort(), hexes.filter((h) => h !== gone).sort(), "the shown palette is the others, unchanged");
    await sleep(2500);
    const after = await settled(s);
    assert.ok(after.n > before.n, "the delete traced again");
    const sent = JSON.parse(after.settings) as { fixed: string[]; colours: number; excluded: string[] };
    assert.deepEqual([...sent.fixed].sort(), [...left].sort(), "fixed colours are exactly the kept palette");
    assert.deepEqual(sent.fixed, hexes.filter((h) => h !== gone), "in the order the row showed them");
    assert.equal(sent.colours, 5, "their count");
    assert.ok(!sent.excluded.includes(gone), "a deleted excluded colour is no longer excluded");
    assert.ok(sent.excluded.includes(keptExcluded), "another exclusion stays");
    const ref = JSON.stringify({ ...(JSON.parse(before.settings) as object), fixed: hexes.filter((h) => h !== gone), colours: 5, excluded: [keptExcluded] });
    const w = await wires(s, ref);
    assert.ok(w.app.length > 50, "a real wire");
    assert.deepEqual(w.app, w.mine, "wires identical to a trace with the smaller palette");
    assert.ok((await hiddenStatus(s)).includes(`Removed ${gone}`), await hiddenStatus(s));
    for (const h of left.slice(0, 4)) {
      await clickX(s, swatch(h));
      await sleep(2000);
    }
    const one = await waitFor("one swatch", async () => {
      const l = await hexesOf(s);
      return l.length === 1 ? l : null;
    }, 40_000);
    await sleep(1500);
    const last = await settled(s);
    const sentOne = JSON.parse(last.settings) as { fixed: string[]; colours: number };
    assert.deepEqual([sentOne.fixed, sentOne.colours], [one, 1], "one kept colour is traced alone");
    await clickX(s, swatch(one[0]));
    await waitFor("no swatch", async () => ((await hexesOf(s)).length === 0 ? true : null), 40_000);
    await sleep(2500);
    assert.equal((await lastRequest(s))!.n, last.n, "no colour left: nothing is traced");
    const trace = JSON.parse((await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('${BAR} [data-trace-slot]'); const d = b.getAttribute('aria-describedby'); return { disabled: b.getAttribute('aria-disabled'), reason: d ? document.getElementById(d)?.textContent ?? '' : '' }; })())`)) as string) as { disabled: string | null; reason: string };
    assert.deepEqual(trace, { disabled: "true", reason: "0 paths: no colour in the palette." }, "Trace is dimmed with the reason");
    assert.ok((await hiddenStatus(s)).includes("0 paths: no colour in the palette."), await hiddenStatus(s));
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${BAR} *')].some((e) => e.children.length === 0 && !e.closest('.sr-only, [hidden]') && e.textContent.includes('no colour'))`), false, "no visible message");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR} [data-trace-pipette][aria-label="Pick palette colour from the image"]')`), true, "the palette pipette stays");
    await typeInto(s, "Colours", "4");
    await waitFor("four swatches from a fresh find", async () => ((await swatchNames(s)).length === 4 ? true : null), 30_000);
    await sleep(1500);
    assert.equal((JSON.parse((await lastRequest(s))!.settings) as { fixed: string[] }).fixed.length, 0, "no fixed colours after a new Colours value");
  }, RECORD);
});

test("0089 §26 tester: at 800 x 600 with five colours the Advanced bar keeps the row on one line and inside the window", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(
    async (s) => {
      await openOn(s, "narrow.png", PICTURE);
      await pickMethod(s, "Remove background");
      await coloursN(s, 5);
      const cells = await rects(s, `${PALETTE} [data-trace-palette-swatch]`);
      assert.equal(cells.length, 5);
      assert.equal(new Set(cells.map((c) => Math.round(c[1]))).size, 1, "one line");
      const colours = (await rects(s, `${BAR} [role=spinbutton][aria-label="Colours"]`))[0];
      assert.ok(Math.abs(cells[0][1] + cells[0][3] / 2 - (colours[1] + colours[3] / 2)) <= 16, "on the line of the Colours field");
      const bar = (await rects(s, BAR))[0];
      assert.ok(bar[0] >= 0 && bar[0] + bar[2] <= 800, `the bar stays in the window: ${bar}`);
      assert.ok(cells[4][0] + cells[4][2] <= bar[0] + bar[2] + 1, "the last cell stays in the bar");
      // The common row of the Colours field and the swatches is one 36 px line high (it did not wrap into a second line).
      const rowH = (await s.evaluate(`(() => { let e = document.querySelector('${BAR} [role=spinbutton][aria-label="Colours"]'); const p = document.querySelector('${PALETTE}'); while (e && !e.contains(p)) e = e.parentElement; return Math.round(e.getBoundingClientRect().height); })()`)) as number;
      assert.ok(rowH <= 40, `the Colours row with its swatches is one line high: ${rowH}`);
    },
    "",
    800,
    600,
  );
});

test("0089 §26 tester: keys. One Tab stop; Home/End; Enter and Space exclude; Shift+Enter, Delete and Backspace delete and focus lands on a neighbour", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "keys.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hexes = await hexesOf(s);
    const focusName = async () => (await s.evaluate(`document.activeElement?.getAttribute('aria-label') ?? ''`)) as string;
    await s.evaluate(`document.querySelector('${PALETTE} [data-trace-palette-swatch][tabindex="0"]').focus()`);
    await press(s, "End", 35);
    assert.ok((await focusName()).startsWith(hexes[5]), "End");
    await press(s, "Home", 36);
    assert.ok((await focusName()).startsWith(hexes[0]), "Home");
    await press(s, "ArrowRight", 39);
    await press(s, "Enter", 13);
    await waitFor("Enter excluded", async () => ((await swatchNames(s))[1].endsWith(", excluded") ? true : null), 20_000);
    await s.evaluate(`document.querySelector('${swatch(hexes[1])}').focus()`);
    await press(s, " ", 32);
    await waitFor("Space brings back", async () => (!(await swatchNames(s))[1].endsWith(", excluded") ? true : null), 20_000);
    await s.evaluate(`document.querySelector('${swatch(hexes[1])}').focus()`);
    await press(s, "Enter", 13, 8);
    await waitFor("five", async () => ((await swatchNames(s)).length === 5 ? true : null), 40_000);
    await sleep(1200);
    assert.ok(!(await hexesOf(s)).includes(hexes[1]), "deleted");
    const focusAfter: string[] = [];
    if (!(await focusName()).startsWith(hexes[2])) focusAfter.push(`Shift+Enter: expected the swatch in its place ${hexes[2]}, focus is "${await focusName()}"`);
    await s.evaluate(`document.querySelector('${swatch(hexes[5])}').focus()`);
    await press(s, "Delete", 46);
    await waitFor("four", async () => ((await swatchNames(s)).length === 4 ? true : null), 40_000);
    await sleep(1200);
    if (!(await focusName()).startsWith(hexes[4])) focusAfter.push(`Delete on the last: expected the previous ${hexes[4]}, focus is "${await focusName()}"`);
    await s.evaluate(`document.querySelector('${swatch(hexes[0])}').focus()`);
    await press(s, "Backspace", 8);
    await waitFor("three", async () => ((await swatchNames(s)).length === 3 ? true : null), 40_000);
    assert.equal(await s.evaluate(`[...document.querySelectorAll('${PALETTE} [data-trace-palette-swatch]')].filter((o) => o.tabIndex === 0).length`), 1);
    assert.deepEqual(focusAfter, [], "after a delete the focus goes to the swatch now in its place, else the previous one");
  });
});

test("0089 §26 tester: a right press on the canvas disarms the palette pipette (Brightness + Multicolour) like Esc", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "right.png", PICTURE);
    await pickMethod(s, "Brightness");
    await multicolour(s);
    await waitFor("swatches", async () => ((await swatchNames(s)).length >= 2 ? true : null), 30_000);
    const pipette = `${BAR} [data-trace-pipette][aria-label="Pick palette colour from the image"]`;
    const pressed = async () => (await s.evaluate(`document.querySelector('${pipette}').getAttribute('aria-pressed')`)) as string;
    const canvas = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as number[];
    const on: [number, number] = [canvas[0] + canvas[2] / 2, canvas[1] + canvas[3] - 120];
    await s.clickSel(pipette);
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: on[0], y: on[1] });
    await sleep(300);
    assert.equal(await pressed(), "true", "armed");
    const count = (await swatchNames(s)).length;
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: on[0], y: on[1], button: "right", buttons: 2, clickCount: 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: on[0], y: on[1], button: "right", buttons: 0, clickCount: 1 });
    await sleep(600);
    assert.equal(await pressed(), "false", "a right press disarms");
    assert.equal((await swatchNames(s)).length, count, "and picks nothing");
    assert.equal(await s.evaluate(`!!document.querySelector('${BAR}')`), true, "the bar stays open");
  });
});

test("0089 §26.6 tester: About Trace has one Palette entry, a Parts entry before Analyse, none of the removed entries; the user page says the same and has no View pill, Recolour or Merge into", async () => {
  const { TRACE_INFO_ENTRIES, infoEntryParts } = await import("../src/lib/traceText.ts");
  const entries = TRACE_INFO_ENTRIES.map((e: string) => infoEntryParts(e));
  const terms = entries.map((e: { term: string }) => e.term);
  assert.equal(terms.filter((t: string) => t === "Palette").length, 1, "one Palette entry");
  for (const gone of ["Isolate", "Recolour", "Merge into", "Exclude", "Remove", "View"]) {
    assert.ok(!terms.includes(gone), `no "${gone}" entry`);
  }
  assert.ok(terms.includes("Parts") && terms.indexOf("Parts") < terms.indexOf("Analyse"), `Parts before Analyse: ${terms.join(", ")}`);
  const page = readFileSync(path.resolve(FRONTEND, "..", "docs", "tracing-images.md"), "utf8");
  for (const e of entries as { term: string; text: string }[]) {
    assert.ok(page.includes(`- ${e.term}: ${e.text}`), `the user page has "${e.term}" word for word`);
  }
  assert.ok(!/recolou?r|merge into|\bView pill\b|"View"/i.test(page), "the user page has no Recolour, Merge into or View pill");
  assert.ok(page.includes("## Parts"), "the user page has the Parts section");
  const design = readFileSync(path.resolve(FRONTEND, "..", "docs", "design-system.md"), "utf8");
  assert.ok(design.includes('"Ctrl+click: exclude"'), "the design system names the tooltip line (§27)");
});

test("0089 §27.3 tester: hovering a swatch shows the x; with Ctrl held it shows the Exclude glyph instead", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await openOn(s, "shiftglyph.png", PICTURE);
    await pickMethod(s, "Remove background");
    await sixColours(s);
    const hex = (await swatchNames(s))[0].slice(0, 7);
    const shown = async () =>
      (await s.evaluate(`JSON.stringify([...document.querySelectorAll(${JSON.stringify(swatch(hex))} + ' svg')].filter((v) => getComputedStyle(v.parentElement).display !== 'none').map((v) => v.getAttribute('class').includes('lucide-x') ? 'x' : v.getAttribute('class').includes('lucide-ban') ? 'ban' : '?').filter((v, i, a) => a.indexOf(v) === i))`)) as string;
    await moveTo(s, swatch(hex));
    assert.equal(await shown(), '["x"]', "plain hover: the x");
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Control", code: "ControlLeft", windowsVirtualKeyCode: 17, modifiers: 2 });
    await sleep(300);
    assert.equal(await shown(), '["ban"]', "Ctrl held: the Exclude glyph");
    // The release event may never arrive (the desktop webview drops it): the next pointer move without Ctrl brings the x back.
    await moveTo(s, swatch(hex), 0);
    assert.equal(await shown(), '["x"]', "no Ctrl on the next pointer move: the x again, without a key release");
    await away(s);
  });
});

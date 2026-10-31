// `specs/0089-trace-quality-and-colour` slice F in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the analysis row
// (criteria 31, 33, UX notes §3, Frage 83 B): a new image gets the analysed values at once and the
// line says what was set; an edit of an analysed value makes it Custom; Analyse again sets them
// back; a second image is analysed on its own; Preset and Output are never touched. Harness as in
// `acceptance_0089_e_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5211;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0089f");
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

const pickMethod = async (s: S, label: string) => {
  await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').click()`);
  await sleep(300);
  await s.evaluate(`[...document.querySelectorAll('[role=menu][aria-label=Method] [role=menuitemradio]')].find((e) => e.textContent.includes(${JSON.stringify(label)})).click()`);
  await sleep(800);
};


/** A black bar and a pale yellow (#FFF2A8) panel on white, 240 x 160: Brightness misses the panel. */
const PALE = rgbPng(240, 160, (x, y) => {
  if (x >= 20 && x < 60 && y >= 20 && y < 140) return [0, 0, 0];
  if (x >= 100 && x < 220 && y >= 30 && y < 130) return [0xff, 0xf2, 0xa8];
  return [255, 255, 255];
});

/** Two black blocks on white, 200 x 100: nothing for a colour method. */
const BLOCKS = rgbPng(200, 100, (x, y) => {
  if (y < 20 || y >= 80) return [255, 255, 255];
  if ((x >= 30 && x < 90) || (x >= 110 && x < 170)) return [0, 0, 0];
  return [255, 255, 255];
});

const line = async (s: S) =>
  (await s.evaluate(`document.querySelector('${BAR} [data-trace-analysis] [role=status]')?.textContent ?? ''`)) as string;
const rowState = async (s: S) =>
  (await s.evaluate(`document.querySelector('${BAR} [data-trace-analysis]')?.getAttribute('data-trace-analysis') ?? ''`)) as string;
const againDisabled = async (s: S) =>
  (await s.evaluate(`[...document.querySelectorAll('${BAR} [data-trace-analysis] button')].find((b) => b.textContent === 'Analyse again')?.getAttribute('aria-disabled') ?? 'missing'`)) as string;
const methodLabel = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Method]').textContent`)) as string;
const presetLabel = async (s: S) => (await s.evaluate(`document.querySelector('${BAR} button[aria-label=Preset]').textContent`)) as string;

test("0089 F in Chromium: the analysis applies to a new image, Custom after an edit, Analyse again", { skip: !BROWSER, timeout: 400_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const pale = path.join(SCRATCH, "pale.png");
  const blocks = path.join(SCRATCH, "blocks.png");
  writeFileSync(pale, PALE);
  writeFileSync(blocks, BLOCKS);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, pale, 1280, 800);

    // Criterion 31, Frage 83 B: the new image is analysed and its values applied at once.
    await waitFor("the analysis line", async () => ((await rowState(s)) === "auto" ? true : null));
    const auto = await line(s);
    assert.match(auto, / · pale colours kept$/, auto);
    assert.equal(await methodLabel(s), "Remove background");
    assert.equal(await againDisabled(s), "true", "Analyse again is dimmed in Auto");
    const preset = await presetLabel(s);
    assert.equal(preset, "Logo / Line art", "the preset is never touched");
    const type = auto.split(" · ")[0];

    // UX notes §3: an edit of an analysed value makes the line Custom; the preset stays.
    await pickMethod(s, "Brightness");
    await waitFor("Custom", async () => ((await rowState(s)) === "custom" ? true : null));
    assert.equal(await line(s), `Custom · ${type}`);
    assert.equal(await againDisabled(s), "false");
    assert.equal(await presetLabel(s), preset);

    // Analyse again: the analysed values back in one go, Auto again.
    await s.clickSel(`${BAR} [data-trace-analysis] button`);
    await waitFor("Auto again", async () => ((await rowState(s)) === "auto" ? true : null));
    assert.equal(await line(s), auto);
    assert.equal(await methodLabel(s), "Remove background");

    // Output is the maker's: Multicolour stays when the next image is analysed.
    await s.clickSel(`${BAR} button[aria-label="More trace options"]`);
    await s.evaluate(`[...document.querySelectorAll('${BAR} [role=radiogroup][aria-label=Output] [role=radio]')].find((r) => r.textContent === 'Multicolour').click()`);
    await sleep(800);

    // A second image is new: analysed on its own, back to Brightness, Output untouched.
    await s.tab("Layers");
    await s.choose("[data-import-button]", blocks);
    await waitFor("the second image row", async () => ((await s.layerRows()).some((r) => r?.includes("blocks.png")) ? true : null));
    if (!(await s.evaluate(`!!document.querySelector('${BAR}')`))) {
      await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    }
    await waitFor("the second analysis", async () => ((await methodLabel(s)) === "Brightness" && (await rowState(s)) === "auto" ? true : null));
    assert.match(await line(s), / · automatic Threshold$/);
    const output = (await s.evaluate(`document.querySelector('${BAR} [role=radiogroup][aria-label=Output] [aria-checked=true]')?.textContent ?? ''`)) as string;
    assert.equal(output, "Multicolour", "Output is never set by the analysis");
    assert.equal(await presetLabel(s), preset);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

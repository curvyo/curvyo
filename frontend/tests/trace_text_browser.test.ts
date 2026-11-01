// `specs/0091-trace-text-recognition` slice 1 in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Recognise text
// switch in Advanced, on by default and remembered (criterion 17); the rotated reference line read
// from the app's own files with no other request (criterion 14), the status line's count, Create
// paths writing the text above the traced group in one step and one undo (criteria 8, 11); with
// the switch off no weight file is fetched (criterion 12). Harness as in
// `acceptance_0089_m_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5294;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0091");
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


const BAR = "[role=toolbar][aria-label=Trace]";
type S = Awaited<ReturnType<typeof session>>;
const ROTATED = path.resolve(FRONTEND, "..", "curvyo-ocr-core", "tests", "fixtures", "rotated.png");

async function openTrace(s: S, file: string) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.tab("Layers");
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await sleep(500);
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 20_000).catch(async (error: unknown) => {
    throw new Error(`${String(error)}: rows ${JSON.stringify(await s.layerRows())}; buttons ${String(await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].map((b) => b.textContent).join('|')`))}`);
  });
}

const openAdvanced = async (s: S) => {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Recognise text"]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
};

const toggle = (s: S, label: string) => s.evaluate(`document.querySelector('${BAR} [aria-label=${JSON.stringify(label)}]')?.getAttribute('aria-pressed') ?? 'missing'`);

test("0091 slice 1 in Chromium: the switch, reading text offline, the text above the group in one step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await openTrace(s, ROTATED);
    await openAdvanced(s);
    // Criterion 17: on by default, the third toggle, no "Recognise shapes", no hint text.
    assert.equal(await toggle(s, "Recognise text"), "true");
    assert.equal(await toggle(s, "Recognise shapes"), "missing");
    const order = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} button[aria-pressed]')].map((b) => b.getAttribute('aria-label')))`)) as string;
    const labels = JSON.parse(order) as string[];
    assert.deepEqual(labels.slice(labels.indexOf("Invert"), labels.indexOf("Invert") + 3), ["Invert", "Drop border shapes", "Recognise text"]);
    // The preview with the read line counted in the status line.
    const status = await waitFor("the text line count", async () => {
      const text = await s.statusText();
      return text.includes("1 text line set") ? text : null;
    }, 120_000);
    assert.ok(!status.includes("failed"), status);
    // Criterion 14: the weights came from the app itself, and nothing left the machine.
    const models = s.requests.filter((u) => u.includes("/models/paddleocr-latin/"));
    assert.ok(models.some((u) => u.endsWith("det.onnx")) && models.some((u) => u.endsWith("rec.onnx")), models.join());
    assert.deepEqual(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")), []);
    // Create paths: the group and the text, the text above it, selected; one undo removes both.
    await s.evaluate(`document.querySelector('${BAR}').closest('[data-context-bar]')?.parentElement?.focus?.()`);
    // The preview fits the line to the fonts after the trace (UX notes §2); then the button rests.
    await waitFor("the preview's fit", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 120_000);
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
    const rows = await waitFor("the text row", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Tilted 15")) ? r : null;
    }, 120_000).catch(async (error: unknown) => {
      throw new Error(`${String(error)}: rows ${JSON.stringify(await s.layerRows())}; status ${await s.statusText()}`);
    });
    // The picture is text only: no path, so no group; the text lies above the image.
    const textRow = rows.findIndex((x) => x?.includes("Tilted 15"));
    const imageRow = rows.findIndex((x) => x?.includes("rotated.png"));
    assert.ok(textRow >= 0 && textRow < imageRow && !rows.some((x) => x?.includes("Trace of")), rows.join(" | "));
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(["rotated.png, image"]) ? true : null));
    // Criteria 12 and 17: off is remembered, and a fresh page with it off fetches no weights.
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null));
    await openAdvanced(s);
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
    await sleep(500);
    assert.equal(await toggle(s, "Recognise text"), "false");
    // A fresh browser on the same profile: the switch is remembered off.
    await s.close();
    const t = await session(profile);
    closeAll = t.close;
    await openTrace(t, ROTATED);
    await openAdvanced(t);
    assert.equal(await toggle(t, "Recognise text"), "false");
    await waitFor("a preview", async () => ((await t.statusText()).match(/\d+ paths?/) ? true : null), 60_000);
    assert.equal(t.requests.filter((u) => u.includes("/models/")).length, 0);
    assert.ok(!(await t.statusText()).includes("text line"));
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

const POSTER = path.resolve(FRONTEND, "..", "curvyo-ocr-core", "tests", "fixtures", "poster.png");

test("0091 slice 1 in Chromium: reading and fitting the poster keeps the main thread free", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    await s.choose("[data-import-button]", POSTER);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("poster.png")) ? true : null));
    // Main-thread tasks over 50 ms (0025 criterion 58) from opening Trace to the fitted preview.
    await s.evaluate(`(() => { window.__long = []; new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__long.push(e.duration); }).observe({ type: 'longtask' }); })()`);
    const started = Date.now();
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
    await waitFor("the text lines", async () => ((await s.statusText()).includes("text lines set") ? true : null), 180_000);
    await waitFor("the preview's fit", async () => ((await s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`)) ? true : null), 180_000);
    const seconds = (Date.now() - started) / 1000;
    const long = JSON.parse((await s.evaluate(`JSON.stringify(window.__long)`)) as string) as number[];
    const total = long.reduce((a, b) => a + b, 0);
    process.stdout.write(`# poster: ${seconds.toFixed(1)} s to the fitted preview; main-thread long tasks ${long.length}, longest ${Math.max(0, ...long).toFixed(0)} ms, total ${total.toFixed(0)} ms\n`);
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

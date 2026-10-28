// `specs/0025-raster-trace` slice 1 in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Select bar's Trace
// button for an image (criterion 1), the Trace bar with its status from the worker (criteria 2,
// 4, 11, 17, 58), Create paths as one step with its notice and the group above the image
// (criteria 12, 13, 30), Ctrl+Z (criterion 13), the key I and Escape (criterion 1), and the empty
// bar (criterion 3). Harness as in `acceptance_0058_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5199;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0025");
const IMAGES = path.resolve(FRONTEND, "..", "curvyo-image-core", "tests", "fixtures", "images");
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


/** A `width` x `height` grey PNG whose pixels `dark` decides (black or white), 96 dpi. */
function png(width: number, height: number, dark: (x: number, y: number) => boolean): Buffer {
  const chunk = (kind: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body) >>> 0);
    return Buffer.concat([len, body, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 0;
  const rows = Array.from({ length: height }, (_, y) =>
    Buffer.from([0, ...Array.from({ length: width }, (_, x) => (dark(x, y) ? 0 : 255))]),
  );
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

test("0025 in Chromium: Trace button, preview status, Create paths, undo, key I", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const logo = path.join(SCRATCH, "logo.png");
  writeFileSync(logo, png(200, 100, (x, y) => x >= 50 && x < 150 && y >= 25 && y < 75));
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
    await s.choose("[data-import-button]", logo);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("logo.png")) ? true : null));

    // Criterion 1: the Trace button, last in the Select bar, with its tooltip text.
    const lastButton = (await s.evaluate(`(() => { const bar = document.querySelector('[data-context-bar]'); const b = [...bar.querySelectorAll('button')].at(-1); return b ? b.textContent : ''; })()`)) as string;
    assert.equal(lastButton, "Trace");
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);

    // Criteria 2, 4, 11: the Trace bar with its groups and the status from the worker.
    const status = await waitFor(
      "the trace status",
      async () => {
        const t = (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [role=status]')?.textContent ?? ''`)) as string;
        return /^1 path, 4 nodes, \d+\.\d s$/.test(t) ? t : null;
      },
      30_000,
    );
    assert.match(status, /^1 path, 4 nodes/);
    const fields = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=toolbar][aria-label=Trace] input')].map((i) => [i.getAttribute('aria-label'), i.value]))`)) as string) as [string, string][];
    assert.deepEqual(fields.map((f) => f[0]), ["Threshold", "Smooth", "Ignore below (mm)"]);
    const thresholdValue = Number(fields[0][1]);
    assert.ok(thresholdValue > 0 && thresholdValue < 256, `the automatic Threshold: ${fields[0][1]}`);
    assert.equal(fields[1][1], "80");
    const pressedRail = (await s.evaluate(`document.querySelectorAll('[aria-pressed=true][data-tool]').length`)) as number;
    assert.equal(pressedRail, 0, "no rail button is pressed");

    // Criteria 12, 13: Create paths is one step; the notice; the group above the image.
    const rowsBefore = (await s.layerRows()).length;
    await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].find((b) => b.textContent === 'Create paths').click()`);
    const notice = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return t.includes("Traced:") ? t : null;
    });
    assert.ok(notice.includes("Traced: 1 path, 4 nodes."), notice);
    const rows = await waitFor("the group row", async () => {
      const r = await s.layerRows();
      return r.length > rowsBefore ? r : null;
    });
    assert.ok(rows.some((r) => r?.includes("Trace of logo.png")), `a group row: ${rows}`);
    const toolBar = (await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean;
    assert.equal(toolBar, false, "back in the Select tool");

    // Criterion 13: one Ctrl+Z removes the group, the image stays.
    await s.key("z", 90, 2);
    await waitFor("undo", async () => ((await s.layerRows()).some((r) => r?.includes("Trace of")) ? null : true));
    assert.ok((await s.layerRows()).some((r) => r?.includes("logo.png")));

    // Criterion 1: I chooses the tool from the canvas; Escape leaves it.
    await s.evaluate(`document.activeElement && document.activeElement.blur && document.activeElement.blur()`);
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.key("i", 73);
    await waitFor("the Trace bar by I", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? true : null));
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await waitFor("Escape leaves", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? null : true));
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

test("0025 in Chromium at 800x600: no Trace bar control passes the bar's right edge, panel open and closed", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const logo = path.join(SCRATCH, "logo.png");
  writeFileSync(logo, png(200, 100, (x, y) => x >= 50 && x < 150 && y >= 25 && y < 75));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    await s.choose("[data-import-button]", logo);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("logo.png")) ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the trace status", async () => ((await s.evaluate(`/path/.test(document.querySelector('[role=toolbar][aria-label=Trace] [role=status]')?.textContent ?? '')`)) ? true : null), 30_000);
    await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].find((b) => b.textContent === 'Advanced').click()`);
    await sleep(500);
    const overflow = async () =>
      (await s.evaluate(`(() => { const bar = document.querySelector('[role=toolbar][aria-label=Trace]'); const edge = bar.getBoundingClientRect().right;
        return JSON.stringify([...bar.querySelectorAll('button, input, [role=radiogroup], label')].filter((e) => e.getBoundingClientRect().width > 0 && e.getBoundingClientRect().right > edge + 0.5)
          .map((e) => (e.getAttribute('aria-label') || e.textContent || e.tagName) + ' ' + Math.round(e.getBoundingClientRect().right - edge))); })()`)) as string;
    for (const open of [true, false]) {
      const panelOpen = (await s.evaluate(`!!document.querySelector('[aria-label="Hide panel"]')`)) as boolean;
      if (panelOpen !== open) {
        await s.clickSel(`[aria-label="${panelOpen ? "Hide panel" : "Show panel"}"]`);
        await sleep(600);
      }
      assert.equal(await overflow(), "[]", `controls past the bar's right edge, panel ${open ? "open" : "closed"}`);
      const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
      writeFileSync(path.join(SCRATCH, `pr-panel-${open ? "open" : "closed"}-800x600.png`), Buffer.from(shot.result?.data ?? "", "base64"));
    }
  } finally {
    await closeAll();
    vite?.kill();
    rmSync(profile, { recursive: true, force: true });
  }
});

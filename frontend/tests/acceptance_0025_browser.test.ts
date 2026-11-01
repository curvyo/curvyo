// `specs/0025-raster-trace` slice 1 in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Select bar's Trace
// button for an image (criterion 1), the Trace bar with its status from the worker (criteria 2,
// 4, 11, 17, 58), Create paths (Enter) as one step with no notice and the group above the image
// (criteria 12, 13, 30), Ctrl+Z (criterion 13), the key I and Escape (criterion 1), and the empty
// bar (criterion 3); slice 3: the View pill and the backslash keys (criterion 51). Harness as in `acceptance_0058_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

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


/** The trace worker's answers held back `window.__delay` ms, and a worker that dies at start while
 * `window.__failWorker` is set (as the slice 3 tester does): the slot's states on demand. */
const SLOW_WORKER = `(() => {
  const Real = window.Worker;
  window.__failWorker = false;
  window.__delay = 0;
  const set = Object.getOwnPropertyDescriptor(Real.prototype, 'onmessage').set;
  window.Worker = class extends Real {
    set onmessage(fn) {
      set.call(this, fn === null ? null : (event) => {
        if (window.__delay > 0 && event.data && event.data.kind === 'trace') setTimeout(() => fn(event), window.__delay);
        else fn(event);
      });
    }
    constructor(url, options) {
      if (window.__failWorker) super(URL.createObjectURL(new Blob(['throw new Error("boom")'], { type: 'text/javascript' })), options);
      else super(url, options);
    }
  };
})();`;

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
    vite = startVite(PORT, FRONTEND);
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

    // Criteria 2, 4, 11, 71: the Trace bar with its groups; the counts from the worker are the
    // last row of the Advanced block, never in the state row.
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [aria-label="More trace options"]')`)) ? true : null));
    await sleep(1500);
    assert.equal(
      await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-counts]') === null && !/path/.test(document.querySelector('[role=toolbar][aria-label=Trace] [role=status]')?.textContent ?? '')`),
      true,
      "no counts with Advanced closed",
    );
    await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [aria-label="More trace options"]').click()`);
    const status = await waitFor(
      "the trace counts",
      async () => {
        const t = (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-counts]')?.textContent ?? ''`)) as string;
        return /^1 path, 4 nodes, \d+\.\d s$/.test(t) ? t : null;
      },
      30_000,
    );
    assert.match(status, /^1 path, 4 nodes/);
    await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [aria-label="Fewer trace options"]').click()`);
    const fields = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=toolbar][aria-label=Trace] [role=spinbutton]')].map((i) => [i.getAttribute('aria-label'), i.getAttribute('aria-valuenow'), i.getAttribute('aria-valuemax'), i.getAttribute('aria-valuetext')]))`)) as string) as [string, string, string, string][];
    assert.deepEqual(fields.map((f) => f[0]), ["Threshold", "Smooth", "Tolerance", "Ignore below"]);
    const thresholdValue = Number(fields[0][1]);
    assert.equal(fields[0][2], "100", "the Threshold is shown in percent");
    assert.ok(thresholdValue > 0 && thresholdValue < 100, `the automatic Threshold: ${fields[0][1]} %`);
    assert.match(fields[0][3], /^\d+ percent$/);
    assert.equal(fields[1][1], "80");
    assert.equal(fields[3][3], "0.50 millimetres");
    const pressedRail = (await s.evaluate(`document.querySelectorAll('[aria-pressed=true][data-tool]').length`)) as number;
    assert.equal(pressedRail, 0, "no rail button is pressed");

    // Criteria 12, 13: Create paths is one step with no notice; the group above the image.
    const rowsBefore = (await s.layerRows()).length;
    assert.equal(await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].some((b) => b.textContent === 'Create paths')`), false, "the bar has no Create paths button");
    await s.evaluate(`document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.key("Enter", 13);
    // Decided by the customer (2026-10-31): no notice after creating paths; the new group being selected is the feedback.
    await waitFor("the Trace bar closes", async () => (((await s.evaluate(`!document.querySelector('[role=toolbar][aria-label=Trace]')`)) as boolean) ? true : null), 15000);
    assert.ok(!/Traced/.test(await s.statusText()), "no notice appears after creating paths");
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
    await stopVite(vite, PORT);
    removeProfile(profile);
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
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Emulation.setDeviceMetricsOverride", { width: 800, height: 600, deviceScaleFactor: 1, mobile: false });
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: SLOW_WORKER });
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    await s.choose("[data-import-button]", logo);
    await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes("logo.png")) ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [aria-label="More trace options"]')`)) ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].find((b) => b.getAttribute('aria-label') === 'More trace options').click()`);
    await waitFor("the trace counts", async () => ((await s.evaluate(`/path/.test(document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-counts]')?.textContent ?? '')`)) ? true : null), 30_000);
    await sleep(500);
    const overflow = async () =>
      (await s.evaluate(`(() => { const bar = document.querySelector('[role=toolbar][aria-label=Trace]'); const edge = bar.getBoundingClientRect().right;
        return JSON.stringify([...bar.querySelectorAll('button, input, [role=radiogroup], [role=spinbutton], label')].filter((e) => e.getBoundingClientRect().width > 0 && e.getBoundingClientRect().right > edge + 0.5)
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
    // UX notes 20.5: Advanced row 1 starts with Invert and Drop border shapes (0089 Frage 88 A), the counts row is
    // the block's last; panel open, the bar keeps inside 800 px.
    await s.clickSel('[aria-label="Show panel"]');
    await sleep(600);
    const advanced = JSON.parse((await s.evaluate(`JSON.stringify((() => { const block = document.querySelector('[role=toolbar][aria-label=Trace] [role=group][aria-label=Advanced]');
      const y = (e) => Math.round(e.getBoundingClientRect().top);
      const controls = [...block.querySelectorAll('button[aria-pressed], [role=radiogroup], [role=spinbutton]')];
      const counts = block.querySelector('[data-trace-counts]');
      return { names: controls.map((e) => e.getAttribute('aria-label')), firstRow: controls.filter((e) => y(e) === y(controls[0])).map((e) => e.getAttribute('aria-label')),
        countsFirst: block.children[1] === counts, countsAbove: y(counts) < Math.min(...controls.map(y)) }; })())`)) as string) as { names: string[]; firstRow: string[]; countsFirst: boolean; countsAbove: boolean };
    // `0089` ux-notes §22.3 (2026-10-31, replaces Frage 92 B / UX notes 20.5 here): Output comes first, then the
    // two toggles; the status line (the counts) is the second row, above all controls, after the analysis row.
    // `specs/0091` UX notes §1: Recognise text is the third icon toggle.
    assert.deepEqual(advanced.names, ["Output", "Invert", "Drop border shapes", "Recognise text", "Denoise", "Upscale", "Corners"]);
    assert.deepEqual(advanced.firstRow.slice(0, 1), ["Output"]);
    assert.ok(advanced.countsFirst && advanced.countsAbove, "the status line is the second row, above the controls");
    // 0089 Frage 84: the expander and the finish button share the last line, the button at the
    // far right (at 308 px they wrap below Ignore below, `specs/0089` ux-notes §2).
    const sameRow = (await s.evaluate(`(() => { const bar = document.querySelector('[role=toolbar][aria-label=Trace]');
      const top = (e) => Math.round(e.getBoundingClientRect().top);
      const ex = bar.querySelector('[aria-label="Fewer trace options"]'); const slot = bar.querySelector('[data-trace-slot]');
      return !!ex && !!slot && Math.abs(top(ex) - top(slot)) < 20 && slot.getBoundingClientRect().left >= ex.getBoundingClientRect().right; })()`)) as boolean;
    assert.ok(sameRow, "the expander and the finish button share a line, the button last");
    const advancedShot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
    writeFileSync(path.join(SCRATCH, "r2-advanced.png"), Buffer.from(advancedShot.result?.data ?? "", "base64"));
    // Criterion 70 (round 3): About Trace is the strip's last tab while the tool is active; the
    // "i" button selects it, is pressed while it shows and restores the tab and body state on a
    // second press; the page has no X.
    const heading = async () => (await s.evaluate(`document.querySelector('aside[aria-label=Properties] h2')?.textContent ?? ''`)) as string;
    const pressed = async () => (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] button[aria-label="About Trace"]').getAttribute('aria-pressed')`)) as string;
    const stripTabs = async () => JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=tablist][aria-label=Panel] [role=tab]')].map((t) => t.getAttribute('aria-label')))`)) as string) as string[];
    assert.equal((await stripTabs()).at(-1), "About Trace", `About Trace is the last tab: ${await stripTabs()}`);
    const tabBefore = await heading();
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="About Trace"]');
    await sleep(400);
    const page = JSON.parse((await s.evaluate(`JSON.stringify((() => { const r = document.querySelector('aside[aria-label=Properties] [aria-label="About Trace"]');
      return { text: r ? r.innerText : '', close: !!document.querySelector('aside[aria-label=Properties] button[aria-label=Close]'),
        selected: document.querySelector('[role=tablist][aria-label=Panel] [role=tab][aria-label="About Trace"]').getAttribute('aria-selected') }; })())`)) as string) as { text: string; close: boolean; selected: string };
    assert.equal(await heading(), "About Trace");
    assert.equal(await pressed(), "true");
    assert.equal(page.selected, "true", "the i button selects the tab");
    assert.equal(page.close, false, "the page has no X");
    assert.match(page.text, /^Trace turns the selected image into paths\./);
    assert.match(page.text, /Drop border shapes\s+drops shapes that touch the image border\./);
    const infoShot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
    writeFileSync(path.join(SCRATCH, "r3-about.png"), Buffer.from(infoShot.result?.data ?? "", "base64"));
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="About Trace"]');
    await sleep(400);
    assert.equal(await heading(), tabBefore, "a second press restores the tab");
    assert.equal(await pressed(), "false");
    // Chosen in the strip, the button follows the tab.
    await s.clickSel('[role=tablist][aria-label=Panel] [role=tab][aria-label="About Trace"]');
    await sleep(400);
    assert.equal(await heading(), "About Trace");
    assert.equal(await pressed(), "true", "the button is pressed while the tab chosen in the strip shows");
    await s.clickSel(`[role=tablist][aria-label=Panel] [role=tab][aria-label="${tabBefore}"]`);
    await sleep(400);
    assert.equal(await pressed(), "false");
    // With the body hidden the button shows it, and a second press hides it again.
    await s.clickSel('[aria-label="Hide panel"]');
    await sleep(600);
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="About Trace"]');
    await sleep(600);
    assert.equal(await s.evaluate(`!!document.querySelector('aside[aria-label=Properties]:not([hidden]) [aria-label="About Trace"]')`), true, "the hidden body opens for the page");
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="About Trace"]');
    await sleep(600);
    assert.equal(await s.evaluate(`document.querySelector('aside[aria-label=Properties]').hidden`), true, "the second press hides the body again");
    await s.clickSel('[aria-label="Show panel"]');
    await sleep(600);
    // Round 3 (criteria 4, 10, 53, 54, 71; UX notes 21.2): the slot after Specks, the expander at
    // the far right of the last line in every state; 800 x 600, panel open, Advanced closed.
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="Fewer trace options"]');
    await sleep(400);
    const layout = async () =>
      JSON.parse((await s.evaluate(`JSON.stringify((() => { const bar = document.querySelector('[role=toolbar][aria-label=Trace]');
        const row = bar.querySelector('[data-trace-state]').parentElement;
        const items = [...row.children].filter((e) => !e.matches('[data-trace-state], .basis-full') && e.getBoundingClientRect().width > 0);
        // A line is 36 px; a 28 px control with its 4 px margin starts 4 px lower in the same line.
        const tops = items.map((e) => Math.round(e.getBoundingClientRect().top)).sort((x, y) => x - y).filter((t, i, all) => i === 0 || t - all[i - 1] > 10);
        const ex = bar.querySelector('button[aria-expanded][aria-controls]').getBoundingClientRect();
        const slot = bar.querySelector('[data-trace-slot]');
        return { rows: tops.length, expanderLast: Math.round(ex.top) >= Math.max(...tops) - 4 && Math.abs(ex.right - row.getBoundingClientRect().right) < 1.5,
          lastItem: items.at(-1).contains(bar.querySelector('button[aria-expanded][aria-controls]')), slot: slot ? slot.getAttribute('data-trace-slot') : null,
          specksBeforeSlot: !slot || items.indexOf(slot) === items.findIndex((e) => e.querySelector('[aria-label="Ignore below"]')) + 1 }; })())`)) as string) as
        { rows: number; expanderLast: boolean; lastItem: boolean; slot: string | null; specksBeforeSlot: boolean };
    const shoot = async (name: string) => {
      const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
      writeFileSync(path.join(SCRATCH, name), Buffer.from(shot.result?.data ?? "", "base64"));
    };
    const rest = await layout();
    console.log("at rest, panel open:", JSON.stringify(rest));
    // 0089 Frage 84 and the customer's 2026-10-30 feedback: the blue finish button is always
    // there, after the expander in the last item; the bar does not grow while a job runs.
    assert.deepEqual([rest.lastItem, rest.slot], [true, "none"], "the expander and the finish button last");
    await s.evaluate(`window.__delay = 6000`);
    await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [role=spinbutton][aria-label=Smooth]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "ArrowUp", code: "ArrowUp", windowsVirtualKeyCode: 38 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "ArrowUp", code: "ArrowUp", windowsVirtualKeyCode: 38 });
    await waitFor("the progress pill", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [role=progressbar]')`)) ? true : null), 10_000);
    const running = await layout();
    console.log("tracing, panel open:", JSON.stringify(running));
    assert.deepEqual([running.rows, running.lastItem, running.slot], [rest.rows, true, "running"], "the progress shows in the same button, no new row");
    assert.equal(await overflow(), "[]", "nothing passes the bar's edge while tracing");
    await shoot("r3-tracing.png");
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="Stop (Esc)"]');
    await sleep(600);
    const stopped = await layout();
    assert.deepEqual([stopped.lastItem, stopped.slot], [true, "stopped"], JSON.stringify(stopped));
    assert.equal(await overflow(), "[]", "nothing passes the bar's edge when stopped");
    await shoot("r3-stopped.png");
    await s.evaluate(`window.__delay = 0; window.__failWorker = true`);
    await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].find((b) => b.textContent === 'Retry').click()`);
    await waitFor("Failed", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-slot=failed]')`)) ? true : null), 20_000);
    await sleep(300);
    const failed = await layout();
    assert.deepEqual([failed.lastItem, failed.slot], [true, "failed"], JSON.stringify(failed));
    assert.equal(await overflow(), "[]", "nothing passes the bar's edge when failed");
    await shoot("r3-failed.png");
    await s.evaluate(`window.__failWorker = false`);
    await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Trace] button')].find((b) => b.textContent === 'Retry').click()`);
    await waitFor("the button at rest", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [data-trace-slot=none]')`)) ? true : null), 20_000);
    // Panel closed: 2 rows at rest and while tracing (no growth).
    await s.clickSel('[aria-label="Hide panel"]');
    await sleep(600);
    const closedRest = await layout();
    await s.evaluate(`window.__delay = 6000`);
    await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [role=spinbutton][aria-label=Smooth]').focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "ArrowDown", code: "ArrowDown", windowsVirtualKeyCode: 40 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "ArrowDown", code: "ArrowDown", windowsVirtualKeyCode: 40 });
    await waitFor("the progress pill", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace] [role=progressbar]')`)) ? true : null), 10_000);
    const closedRunning = await layout();
    console.log("panel closed, rest and tracing:", JSON.stringify([closedRest, closedRunning]));
    assert.deepEqual([closedRunning.rows, closedRunning.lastItem], [closedRest.rows, true], "no growth while tracing");
    await shoot("r3-tracing-panel-closed.png");
    await s.evaluate(`window.__delay = 0`);
    await s.clickSel('[role=toolbar][aria-label=Trace] button[aria-label="Stop (Esc)"]');
    await sleep(400);
    await s.clickSel('[aria-label="Show panel"]');
    await sleep(600);
    // Slice 3 (criterion 51): the View pill in row 1; held backslash presses Image, release
    // restores, Shift+backslash cycles.
    const checked = async () =>
      (await s.evaluate(`document.querySelector('[role=radiogroup][aria-label=View] [aria-checked=true]')?.getAttribute('aria-label') ?? ''`)) as string;
    const key = (type: string, k: string, shift = false) =>
      s.evaluate(`window.dispatchEvent(new KeyboardEvent('${type}', { key: ${JSON.stringify(k)}, shiftKey: ${shift} })) && true`);
    assert.equal(await checked(), "Overlay");
    await key("keydown", "\\");
    await sleep(200);
    assert.equal(await checked(), "Image", "the peek presses Image");
    await key("keyup", "\\");
    await sleep(200);
    assert.equal(await checked(), "Overlay");
    await key("keydown", "|", true);
    await sleep(200);
    // Mask follows Overlay since `specs/0089` criterion 21.
    assert.equal(await checked(), "Mask", "Shift+backslash cycles");
    // Leaving the tool with the tab active returns to the stored tab; the tab leaves the strip.
    await s.clickSel('[role=tablist][aria-label=Panel] [role=tab][aria-label="About Trace"]');
    await sleep(400);
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await waitFor("Escape leaves", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? null : true));
    await sleep(400);
    assert.equal(await heading(), tabBefore, "leaving the tool returns to the stored tab");
    assert.ok(!(await stripTabs()).includes("About Trace"), "the tab leaves the strip with the tool");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

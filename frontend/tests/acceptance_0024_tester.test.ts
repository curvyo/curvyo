// Tester cases for `specs/0024-svg-import-export` (import slice) in the real app: with
// CURVYO_E2E_BROWSER=/path/to/chromium and the wasm bindings built, Chromium drives the File tab:
// Import SVG… of a real Inkscape 1.4.4 file through the browser's own file chooser (criteria 3, 5:
// one new layer named after the file, one history step, Ctrl+Z removes it), a refused file
// (criterion 6: the notice text, nothing changed; slice 2 imports live text, so the "nothing
// imported" case of criterion 8 is left to the slice-2 test), Open… of an SVG (criteria 1, 2:
// untitled and dirty), no network request and no file markup in the page (criterion 67). Written from the spec before the implementation was read; the
// selectors come from the rendered page. The browser profile lives under `target/`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, openSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5197;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0024");
const REAL = path.resolve(FRONTEND, "..", "curvyo-svg-core", "tests", "fixtures", "svg_real");
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
  const events: string[] = [];
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as {
      id?: number;
      method?: string;
      params?: { request?: { url: string }; backendNodeId?: number };
    };
    if (msg.method && /^(Page\.frame(Navigated|Started)|Inspector\.|Runtime\.(exceptionThrown|consoleAPICalled)|Page\.(loadEventFired|navigatedWithinDocument))/.test(msg.method)) {
      events.push(`${msg.method} ${JSON.stringify(msg.params).slice(0, 300)}`);
    }
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
  // A call the browser never answers (a reloaded or crashed page) fails after a minute with the
  // page's last events, instead of hanging the file until the test timeout.
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method} in 60 s; page events: ${events.slice(-15).join(" | ")}`)), 60_000);
      pending.set(id, (v) => {
        clearTimeout(timer);
        resolve(v);
      });
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
  await send("Runtime.enable");
  await send("Network.enable");
  await send("Page.setInterceptFileChooserDialog", { enabled: true });
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, click, key, clickSel, choose, tab, statusText, dialogText, layerRows, close, requests };
}

const ENTITY_BOMB = `<?xml version="1.0"?>
<!DOCTYPE svg [
<!ENTITY a "aaaaaaaaaa">
<!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
]>
<svg xmlns="http://www.w3.org/2000/svg"><desc>&b;</desc><rect width="1" height="1"/></svg>`;

test("criteria 1 to 3, 5, 6, 67 in Chromium: Import SVG… and Open… of real files", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const bomb = path.join(SCRATCH, "bomb.svg");
  writeFileSync(bomb, ENTITY_BOMB);
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    const viteLog = openSync(path.join(SCRATCH, "vite.log"), "w");
    vite = startVite(PORT, FRONTEND, ["ignore", viteLog, viteLog]);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[role=tab][aria-label^="File"]')`)) ? true : null));
    await sleep(1500);

    // Criterion 5 as changed by the customer on 2026-10-27 (`specs/0058-bitmap-import`
    // criterion 1): the File tab has no import row; the Import button of the tool rail imports.
    await s.tab("File");
    const commands = JSON.parse(
      (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[data-file-command]')].map((e) => e.getAttribute('data-file-command')))`)) as string,
    ) as string[];
    assert.ok(!commands.includes("import-svg"), `no Import SVG… in the File tab: ${commands}`);
    const rail = (await s.evaluate(`!!document.querySelector('[data-import-button]')`)) as boolean;
    assert.ok(rail, "the Import button is in the tool rail");

    // Criterion 3: a real Inkscape file, one new layer named after the file, one history step.
    const network = s.requests.length;
    await s.choose("[data-import-button]", path.join(REAL, "inkscape", "inkscape_mm_a4.svg"));
    assert.equal(await s.dialogText(), "", "no dialog after an import");
    assert.ok(!/inkscape_mm_a4/.test(await s.statusText()), "criterion 8: no message after a successful import");
    await s.tab("Layers");
    const rows = await s.layerRows();
    assert.ok(rows.some((r) => /inkscape_mm_a4/.test(r ?? "")), `criterion 3: the new layer (${rows})`);
    await s.tab("History");
    const history = (await s.evaluate(`document.body.innerText`)) as string;
    assert.match(history, /Import SVG/, "criterion 3: the step is named Import SVG");
    // Criterion 67: nothing fetched, no markup of the file in the page.
    const fetched = s.requests.slice(network).filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:"));
    assert.deepEqual(fetched, [], "criterion 67: no request");
    const leaked = (await s.evaluate(`!!document.getElementById('p1') || !!document.querySelector('use[href="#p1"], [*|href="#p1"], foreignObject')`)) as boolean;
    assert.equal(leaked, false, "criterion 67: the SVG is never inserted into the page");

    // One Ctrl+Z removes all of it.
    await s.clickSel("canvas");
    await s.key("z", 90, 2);
    await s.tab("Layers");
    assert.ok(!(await s.layerRows()).some((r) => /inkscape_mm_a4/.test(r ?? "")), "Ctrl+Z removes the layer");

    // Criterion 6 on the Import button's route (`0058`, 2026-10-27): a refusal is the canvas
    // notice with the file name and the spec's sentence, and changes nothing.
    await s.tab("Layers");
    const rowsBefore = await s.layerRows().catch(() => []);
    await s.choose("[data-import-button]", bomb);
    const refusal = await waitFor("the refusal", async () => {
      const t = await s.statusText();
      return /XML entities/.test(t) ? t : null;
    }, 10_000);
    assert.match(refusal, /bomb\.svg/);
    assert.match(refusal, /This file defines XML entities, which Curvyo does not read because they can be used to exhaust memory\. Nothing was changed\./);
    assert.equal(await s.dialogText(), "", "no dialog");
    assert.deepEqual(await s.layerRows().catch(() => []), rowsBefore);

    // Criteria 1 and 2: Open… of an SVG gives an untitled, dirty project.
    await s.tab("File");
    // The project is dirty: the unsaved-changes check of `0051` comes first, unchanged.
    await s.clickSel("[data-file-command=open]");
    await sleep(800);
    await s.evaluate(`[...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Discard').setAttribute('data-tester-discard', '')`);
    await s.choose("[data-tester-discard]", path.join(REAL, "wikimedia", "Flag_of_Switzerland.svg"));
    await waitFor("the opened SVG", async () => ((await s.evaluate(`document.title`)) === "*Curvyo" ? true : null), 10_000);
    await s.tab("Layers");
    const opened = await s.layerRows();
    assert.ok(opened.length >= 2, `the flag's two paths (${opened})`);
  } catch (error) {
    console.error(`vite log:\n${existsSync(path.join(SCRATCH, "vite.log")) ? readFileSync(path.join(SCRATCH, "vite.log"), "utf8").slice(-3000) : "(none)"}`);
    throw error;
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
  }
});

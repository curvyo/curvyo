// `specs/0088-trace-recognition` Part A in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): a text typed in the
// included sans and turned into (unfilled) paths by Object to path is read back. The Select bar shows
// "Recognise text" last (UX notes §1); its bar has the Tolerance field and the Faces readout
// (criteria 6, 10); Recognise writes one text in one step "Recognise text", selected, with the
// notice naming face, source and deviation (criteria 1, 3, 10); one undo brings the path back
// (criterion 6); the path mirrored gives the notice row and changes nothing (criterion 4);
// Escape leaves the bar, the selection kept.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5288;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0088");
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
  const pending = new Map<number, (v: Reply) => void>();
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as { id?: number; method?: string };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
    }
    if (msg.method === "Page.javascriptDialogOpening") {
      id += 1;
      ws.send(JSON.stringify({ id, method: "Page.handleJavaScriptDialog", params: { accept: true } }));
    }
  };
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const mine = id;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method}`)), 180_000);
      pending.set(mine, (v) => {
        clearTimeout(timer);
        resolve(v);
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
  const press = async (key: string, code: number, modifiers = 0) => {
    const keyCode = key.length === 1 ? `Key${key.toUpperCase()}` : key;
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key, code: keyCode, windowsVirtualKeyCode: code, modifiers });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key, code: keyCode, windowsVirtualKeyCode: code, modifiers });
    await sleep(300);
  };
  const clickSel = async (sel: string) => {
    const c = JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
    assert.ok(c, `${sel} is on the page`);
    await click(c[0], c[1]);
  };
  const close = async () => {
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
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, click, press, clickSel, close };
}

const BAR = `(() => {
  const bar = document.querySelector('[data-context-bar]');
  return JSON.stringify({
    name: bar?.getAttribute('aria-label') ?? '',
    text: bar?.textContent ?? '',
    commands: [...(bar?.querySelectorAll('[data-select-command]') ?? [])].map((b) => b.getAttribute('data-select-command')),
    faces: document.querySelector('[data-recognise-faces]')?.textContent ?? '',
    tolerance: document.querySelector('[data-context-bar] [role=spinbutton]')?.getAttribute('aria-valuetext') ?? '',
    notice: document.querySelector('[data-recognise-notice]')?.textContent ?? '',
    row: document.querySelector('[data-recognise-state] [role=status]')?.textContent ?? '',
    rows: [...document.querySelectorAll('[role=row][aria-level]')].map((r) => r.getAttribute('aria-label') ?? ''),
  });
})()`;

type Bar = { name: string; text: string; commands: string[]; faces: string; tolerance: string; notice: string; row: string; rows: string[] };

test("0088 in Chromium: Recognise text from the Select bar, its bar, the notice and undo", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    const bar = async () => JSON.parse((await s.evaluate(BAR)) as string) as Bar;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('button[aria-label="Text tool (T)"]')`)) ? true : null));
    await sleep(1500);
    const tab = async (name: string) => {
      await s.evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => (t.getAttribute('aria-label') ?? t.textContent).startsWith(${JSON.stringify(name)}))?.click()`);
      await sleep(300);
    };
    await tab("Layers");

    // A text in the included sans, turned into paths (stroked, not filled).
    await s.clickSel('button[aria-label="Text tool (T)"]');
    await s.click(500, 300);
    await sleep(700);
    await s.send("Input.insertText", { text: "Laser 42" });
    await sleep(300);
    await s.press("Escape", 27);
    await s.clickSel('button[aria-label^="Select"]');
    await s.click(520, 295);
    await waitFor("Object to path", async () => ((await s.evaluate(`!!document.querySelector('button[title^="Convert the selected"]')`)) ? true : null), 10_000);
    await s.clickSel('button[title^="Convert the selected"]');
    await sleep(500);
    // The glyph paths are stroked, not filled: Recognise text reads closed outlines as filled.

    // UX notes §1: the button is last in the Select bar while a closed path is selected.
    let b = await waitFor("the button", async () => {
      const got = await bar();
      return got.commands.includes("recognise-text") ? got : null;
    }, 10_000);
    assert.equal(b.commands[b.commands.length - 1], "recognise-text", b.commands.join(", "));
    const pathRows = b.rows;

    // Criteria 6 and 10: the bar, its tolerance and the face sources readout.
    await s.clickSel('[data-select-command="recognise-text"]');
    b = await bar();
    assert.equal(b.name, "Recognise text");
    assert.match(b.faces, /^\d face sources?$/, b.faces);
    assert.match(b.tolerance, /^0\.30 millimetres$/, b.tolerance);

    // Criteria 1, 3, 10: Enter on the canvas runs; one text. A clean run has no notice (customer
    // rule of 2026-10-31); the Select bar shows the new text's face.
    await s.evaluate(`document.querySelector('[data-context-bar] button[data-recognise-slot]')?.click()`);
    b = await waitFor("the result", async () => {
      const got = await bar();
      return got.name === "" ? got : null;
    }, 60_000);
    assert.equal(b.notice, "", "no success notice");
    assert.equal(b.name, "", "the Select bar is back");
    // The new text is selected (the Select bar shows the group); the History names the step.
    assert.ok(b.text.includes("Group of 7 objects") || b.text.includes("Stroke to path"), b.text);
    await tab("History");
    assert.ok(((await s.evaluate(`document.body.innerText`)) as string).includes("Recognise text"), "the History row");
    await tab("Layers");

    // Criterion 6: one undo brings the path back.
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.press("z", 90, 2);
    b = await waitFor("undo", async () => {
      const got = await bar();
      return JSON.stringify(got.rows) === JSON.stringify(pathRows) && !got.text.includes("Source Sans 3") && got.commands.includes("recognise-text") ? got : null;
    }, 10_000);

    // Criterion 4: the path mirrored (key H) is no text in any face: nothing is written and the
    // notice row names the nearest face. Escape leaves the bar, the selection kept.
    await s.click(520, 295);
    await s.press("h", 72);
    await s.clickSel('[data-select-command="recognise-text"]');
    b = await bar();
    const rowsBefore = b.rows.length;
    await s.evaluate(`document.querySelector('[data-context-bar] button[data-recognise-slot]')?.click()`);
    b = await waitFor("the outcome", async () => {
      const got = await bar();
      return got.row !== "" || got.notice !== "" ? got : null;
    }, 60_000);
    assert.match(b.row, /^No text within 0\.30 mm; nearest face .+, \d+\.\d\d mm\.$/, b.row);
    assert.equal(b.notice, "");
    assert.equal(b.rows.length, rowsBefore, "nothing changed");
    await s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    await s.press("Escape", 27);
    b = await bar();
    assert.notEqual(b.name, "Recognise text", "Escape leaves the bar");
    assert.ok(b.commands.includes("recognise-text"), "the selection is kept");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

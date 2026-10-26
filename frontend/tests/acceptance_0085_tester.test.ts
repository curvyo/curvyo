// Tester cases for `specs/0085-stroke-to-path` on the frontend: the notice sentences and their
// order and lengths (criterion 22), the shortcut chord (criterion 3, UX notes "Keyboard"), and,
// with CURVYO_E2E_BROWSER=/path/to/chromium and the wasm bindings built, the real app in
// Chromium: the button after Object to path and before Linked clone, a press and Ctrl+Alt+C on a
// drawn rectangle, Ctrl+Z, the dimmed button with nothing stroked, and Ctrl+Alt+C ignored while
// typing in a text field (criteria 1, 2, 3, 19, 21, 22). The browser profile lives under
// `target/`, never outside the checkout.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import {
  STROKE_TO_PATH_DIMMED,
  STROKE_TO_PATH_NAME,
  type StrokeToPathResult,
  isStrokeToPathChord,
  strokeToPathNotice,
} from "../src/lib/strokeToPathText.ts";

const result = (over: Partial<StrokeToPathResult>): StrokeToPathResult => ({
  kind: "applied",
  count: 0,
  of: 0,
  strokes: 1,
  unstroked: 0,
  texts: 0,
  unlinked: 0,
  ...over,
});

test("criterion 22: the spec's sentences, in the spec's order", () => {
  const all = strokeToPathNotice(result({ strokes: 3, unstroked: 2, texts: 2, unlinked: 2 }));
  assert.deepEqual(all, {
    text:
      "Stroke to path: 3 strokes became filled outlines. 2 objects have no stroke and were left as they are. " +
      "2 texts were converted to paths. 2 clones were unlinked.",
    alert: false,
  });
  assert.equal(strokeToPathNotice(result({}))?.text, "Stroke to path: 1 stroke became a filled outline.");
  assert.equal(
    strokeToPathNotice(result({ texts: 1 }))?.text,
    "Stroke to path: 1 stroke became a filled outline. 1 text was converted to paths.",
  );
});

test("criterion 22: the first sentence fits 70 characters, a long notice its sentences 140", () => {
  for (const strokes of [1, 9, 999_999]) {
    const text = strokeToPathNotice(result({ strokes, unstroked: 99_999, texts: 9_999, unlinked: 9_999 }))!.text;
    const sentences = text.split(/(?<=\.) /);
    assert.ok(sentences[0].length <= 70, sentences[0]);
    for (const s of sentences) {
      assert.ok(s.length <= 140, s);
    }
    assert.ok(!/[!]/.test(text));
  }
});

test("criterion 26: the 0016 criterion 17a refusal with Stroke to path as the name", () => {
  const one = strokeToPathNotice({ ...result({}), kind: "out_of_range", count: 1, of: 3 });
  assert.deepEqual(one, {
    text: "Stroke to path works only within 10 km of the point 0, 0. 1 of 3 selected objects reaches further. Nothing was changed.",
    alert: true,
  });
  const two = strokeToPathNotice({ ...result({}), kind: "out_of_range", count: 2, of: 3 });
  assert.equal(
    two?.text,
    "Stroke to path works only within 10 km of the point 0, 0. 2 of 3 selected objects reach further. Nothing was changed.",
  );
});

test("criterion 1 and UX notes: the button's name and its dimmed tooltip are state labels", () => {
  assert.equal(STROKE_TO_PATH_NAME, "Stroke to path");
  assert.equal(STROKE_TO_PATH_DIMMED, "Stroke to path: nothing selected has a stroke.");
});

test("criterion 3: Ctrl+Alt+C, Cmd+Alt+C and the composed macOS key, never AltGr, Shift or IME", () => {
  const ev = (over: Partial<Parameters<typeof isStrokeToPathChord>[0]>) => ({
    key: "c",
    code: "KeyC",
    ctrlKey: true,
    metaKey: false,
    altKey: true,
    shiftKey: false,
    ...over,
  });
  assert.equal(isStrokeToPathChord(ev({})), true);
  assert.equal(isStrokeToPathChord(ev({ key: "C" })), true);
  assert.equal(isStrokeToPathChord(ev({ ctrlKey: false, metaKey: true, key: "ç" })), true, "macOS Option+C");
  assert.equal(isStrokeToPathChord(ev({ shiftKey: true })), false);
  assert.equal(isStrokeToPathChord(ev({ altKey: false })), false, "Ctrl+C is copy");
  assert.equal(isStrokeToPathChord(ev({ ctrlKey: false })), false, "Alt+C alone");
  assert.equal(isStrokeToPathChord(ev({ getModifierState: (k: string) => k === "AltGraph" })), false, "AltGr");
  assert.equal(isStrokeToPathChord(ev({ isComposing: true })), false);
  assert.equal(isStrokeToPathChord(ev({ key: "v", code: "KeyV" })), false);
});

// ---------------------------------------------------------------------------------------------
// Criteria 1, 2, 3, 19, 21, 22 in Chromium
// ---------------------------------------------------------------------------------------------

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5197;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0085");
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

async function browser(profile: string) {
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
    const msg = JSON.parse(String(event.data)) as { id?: number };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
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
  const mouse = async (type: string, x: number, y: number) =>
    send("Input.dispatchMouseEvent", { type, x, y, button: "left", buttons: type === "mouseReleased" ? 0 : 1, clickCount: 1 });
  const click = async (x: number, y: number) => {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
    await mouse("mousePressed", x, y);
    await mouse("mouseReleased", x, y);
    await sleep(300);
  };
  const drag = async (a: [number, number], b: [number, number]) => {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x: a[0], y: a[1] });
    await mouse("mousePressed", a[0], a[1]);
    for (let i = 1; i <= 8; i++) {
      await mouse("mouseMoved", a[0] + ((b[0] - a[0]) * i) / 8, a[1] + ((b[1] - a[1]) * i) / 8);
    }
    await mouse("mouseReleased", b[0], b[1]);
    await sleep(400);
  };
  /** A key with modifiers (1 Alt, 2 Ctrl, 8 Shift). */
  const key = async (k: string, code: string, vk: number, modifiers = 0) => {
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code, windowsVirtualKeyCode: vk, modifiers });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code, windowsVirtualKeyCode: vk, modifiers });
    await sleep(300);
  };
  const center = async (sel: string) =>
    JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
  const clickSel = async (sel: string) => {
    const c = await center(sel);
    assert.ok(c, `${sel} is on the page`);
    await click(c[0], c[1]);
  };
  const close = async () => {
    await send("Browser.close").catch(() => undefined);
    ws.close();
  };
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, click, drag, key, center, clickSel, close };
}

/** The Select bar's text buttons in order, the Stroke to path button's state, the notices, the
 * Layers rows, the focused element. */
const STATE = `JSON.stringify((() => {
  const s2p = document.querySelector('[data-select-command="stroke-to-path"]');
  const buttons = [...document.querySelectorAll('button')].map((b) => b.textContent.trim()).filter((t) => ['Object to path', 'Stroke to path', 'Linked clone'].includes(t));
  const notices = [...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent.trim()).filter((t) => t.length > 0);
  const rows = [...document.querySelectorAll('[aria-label="Layers and objects"] [role=row]')].map((r) => r.getAttribute('aria-label') || r.textContent.trim());
  return {
    present: !!s2p,
    disabled: s2p ? s2p.getAttribute('aria-disabled') : null,
    buttons,
    notices,
    rows,
    focus: document.activeElement && document.activeElement.tabIndex === 0 && document.activeElement.querySelector(':scope > * canvas, :scope > canvas') ? 'CANVAS host' : document.activeElement ? document.activeElement.tagName + ':' + (document.activeElement.getAttribute('aria-label') || document.activeElement.textContent.trim().slice(0, 30)) : '',
  };
})())`;

interface State {
  present: boolean;
  disabled: string | null;
  buttons: string[];
  notices: string[];
  rows: string[];
  focus: string;
}

test("criteria 1, 2, 3, 19, 21, 22 in Chromium: button, Ctrl+Alt+C, undo, dimmed, typing", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "run-"));
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const b = await browser(profile);
    await b.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await b.evaluate(`!!document.querySelector('button[aria-label^="Rectangle tool"]')`)) ? true : null));
    await sleep(1500);
    const state = async () => JSON.parse((await b.evaluate(STATE)) as string) as State;
    // Open the Layers tab so its rows can be counted.
    await b.evaluate(`(() => { const t = [...document.querySelectorAll('[role=tab]')].find((t) => (t.getAttribute('aria-label') || t.textContent).trim() === 'Layers'); if (t) t.click(); return !!t; })()`);
    await sleep(500);

    // Criterion 1: nothing selected, no button.
    assert.equal((await state()).present, false, "not in the tree with nothing selected");

    // Draw a rectangle: it has the default stroke.
    // The drawing canvas is the largest one (the rulers are canvases too).
    const canvas = JSON.parse(
      (await b.evaluate(`(() => { const c = [...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0];
        const r = c.getBoundingClientRect(); return JSON.stringify([r.x + r.width / 2, r.y + r.height / 2]); })()`)) as string,
    ) as [number, number];
    await b.clickSel('button[aria-label^="Rectangle tool"]');
    await b.drag([canvas[0] - 120, canvas[1] - 80], [canvas[0] + 60, canvas[1] + 40]);
    await b.clickSel('button[aria-label^="Select tool"]');
    await b.click(canvas[0] - 120, canvas[1] - 80);
    if (process.env.CURVYO_E2E_SHOTS) {
      const shot = (await b.send("Page.captureScreenshot", { format: "png" })) as unknown as { result: { data: string } };
      (await import("node:fs")).writeFileSync(path.join(SCRATCH, "after-draw.png"), Buffer.from(shot.result.data, "base64"));
    }
    let s = await state();
    assert.equal(s.present, true, `the button shows with a selection: ${JSON.stringify(s)}`);
    assert.equal(s.disabled, "false");
    const i = s.buttons.indexOf("Stroke to path");
    assert.ok(i > 0 && s.buttons[i - 1] === "Object to path", `after Object to path: ${s.buttons}`);
    assert.equal(s.buttons[i + 1], "Linked clone", `before Linked clone: ${s.buttons}`);
    const rowsBefore = s.rows.length;

    // Criteria 2, 19, 22: one press, no dialog, a notice, focus back on the canvas.
    await b.clickSel('[data-select-command="stroke-to-path"]');
    s = await state();
    assert.ok(s.notices.some((n) => n === "Stroke to path: 1 stroke became a filled outline."), `notice: ${s.notices}`);
    assert.equal(await b.evaluate(`!!document.querySelector('[role=dialog]')`), false, "no dialog");
    assert.ok(s.focus.startsWith("CANVAS"), `focus after a mouse press: ${s.focus}`);
    const rowsAfter = s.rows.length;
    assert.ok(rowsAfter >= rowsBefore, `rows ${rowsBefore} -> ${rowsAfter}`);

    // Criterion 21: Ctrl+Z restores.
    await b.key("z", "KeyZ", 90, 2);
    s = await state();
    assert.equal(s.rows.length, rowsBefore, "undo restores the objects");
    assert.equal(s.disabled, "false", "the rectangle has its stroke again and is selected");

    // Criterion 3: Ctrl+Alt+C on the canvas runs the same command.
    await b.key("c", "KeyC", 67, 3);
    s = await state();
    assert.ok(s.notices.some((n) => n.startsWith("Stroke to path: 1 stroke")), `Ctrl+Alt+C notice: ${s.notices}`);
    assert.equal(s.rows.length, rowsAfter);

    // Criterion 1: the result has no stroke left (the outline is filled, the fill part, if any,
    // has Stroke off): the button is dimmed, and Ctrl+Alt+C does nothing, no notice.
    await sleep(3500);
    s = await state();
    assert.equal(s.disabled, "true", `dimmed with nothing stroked: ${JSON.stringify(s)}`);
    await b.key("c", "KeyC", 67, 3);
    s = await state();
    assert.equal(s.rows.length, rowsAfter);
    assert.ok(!s.notices.some((n) => n.startsWith("Stroke to path")), `no notice: ${s.notices}`);

    // Criterion 3: never while typing in a text field. Undo to the stroked rectangle, focus a
    // text field of the panel, press the chord.
    await b.key("z", "KeyZ", 90, 2);
    s = await state();
    assert.equal(s.disabled, "false");
    const typed = await b.evaluate(`(() => { const f = [...document.querySelectorAll('input[type=text], input:not([type])')].find((e) => e.offsetParent !== null); if (!f) return false; f.focus(); return document.activeElement === f; })()`);
    assert.equal(typed, true, "a visible text field");
    await b.key("c", "KeyC", 67, 3);
    s = await state();
    assert.equal(s.rows.length, rowsBefore, "nothing converted while typing");
    assert.ok(!s.notices.some((n) => n.startsWith("Stroke to path")), `no notice while typing: ${s.notices}`);

    // Criterion 3 and UX notes: never while text on the canvas is being edited.
    await b.key("Escape", "Escape", 27);
    await b.clickSel('button[aria-label^="Text tool"]');
    await b.click(canvas[0] + 150, canvas[1] + 150);
    for (const ch of ["a", "b"]) {
      await b.send("Input.dispatchKeyEvent", { type: "keyDown", key: ch, code: `Key${ch.toUpperCase()}`, text: ch, windowsVirtualKeyCode: ch.toUpperCase().charCodeAt(0) });
      await b.send("Input.dispatchKeyEvent", { type: "keyUp", key: ch, code: `Key${ch.toUpperCase()}`, windowsVirtualKeyCode: ch.toUpperCase().charCodeAt(0) });
    }
    await sleep(500);
    if (process.env.CURVYO_E2E_SHOTS) {
      const shot = (await b.send("Page.captureScreenshot", { format: "png" })) as unknown as { result: { data: string } };
      (await import("node:fs")).writeFileSync(path.join(SCRATCH, "typing.png"), Buffer.from(shot.result.data, "base64"));
    }
    const rowsTyping = (await state()).rows.length;
    await b.key("c", "KeyC", 67, 3);
    s = await state();
    assert.ok(!s.notices.some((n) => n.startsWith("Stroke to path")), `no notice while editing text: ${s.notices}`);
    assert.equal(s.rows.length, rowsTyping, "nothing converted while editing text");
    await b.send("Input.dispatchKeyEvent", { type: "keyDown", key: "c", code: "KeyC", text: "c", windowsVirtualKeyCode: 67 });
    await b.send("Input.dispatchKeyEvent", { type: "keyUp", key: "c", code: "KeyC", windowsVirtualKeyCode: 67 });
    await sleep(300);
    await b.key("Escape", "Escape", 27);
    const text = (await b.evaluate(`[...document.querySelectorAll('[aria-label="Layers and objects"] [role=row]')].map((r) => r.textContent).join('|')`)) as string;
    assert.ok(/abc/.test(text), `the text keeps its characters, typing went on: ${text}`);
    await b.close();
  } finally {
    chromes.forEach((c) => c.kill("SIGKILL"));
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

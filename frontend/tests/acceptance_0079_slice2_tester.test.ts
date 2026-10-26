// Tester cases for `specs/0079-text-layout-controls` slice 2 on the frontend text rules: the
// Taper field's label, place and notices (criterion 16) and the fold of the text layout block
// (criterion 6b, UX notes §3.1): closed by default, stored as JSON `true` under
// `curvyo.text-layout-open`, unreadable storage means closed, and the "Changed:" description.
// The last case drives the real app in Chromium (only with CURVYO_E2E_BROWSER=/path/to/chromium
// and the wasm bindings built): the fold closed by default, kept across a reload, the dot and
// its description while folded with a changed Taper, and the Taper's clamp notice.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

import {
  type FoldStorage,
  LAYOUT_FIELDS,
  LAYOUT_FIELD_LABELS,
  type LayoutFieldState,
  TEXT_LAYOUT_OPEN_KEY,
  changedLayoutFields,
  foldChangedText,
  layoutNotice,
  layoutUnit,
  readLayoutOpen,
  writeLayoutOpen,
} from "../src/lib/textLayoutText.ts";

const state = (over: Partial<LayoutFieldState>): LayoutFieldState => ({
  text: "0",
  value: 0,
  mixed: false,
  auto: false,
  barFrom: 0,
  barTo: 0,
  zero: null,
  dragBase: 0,
  dragMin: 0,
  dragLimit: 1,
  trackMin: 0,
  trackMax: 1,
  resettable: false,
  ...over,
});

class MemoryStorage implements FoldStorage {
  readonly map = new Map<string, string>();
  getItem(key: string): string | null {
    return this.map.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
}

const throwing: FoldStorage = {
  getItem() {
    throw new Error("blocked");
  },
  setItem() {
    throw new Error("blocked");
  },
};

test("criterion 16: Taper comes directly after Character rotation, labelled Taper, unit %", () => {
  const i = LAYOUT_FIELDS.indexOf("character-rotation");
  assert.equal(LAYOUT_FIELDS[i + 1], "taper");
  assert.equal(LAYOUT_FIELD_LABELS.taper, "Taper");
  assert.equal(layoutUnit("taper"), "%");
});

test("criterion 16: the clamp notices for the Taper", () => {
  assert.equal(layoutNotice("taper", { kind: "max", mm: 50 }), "Taper set to 50 %, the largest taper.");
  assert.equal(layoutNotice("taper", { kind: "min", mm: -50 }), "Taper set to −50 %, the smallest taper.");
  assert.equal(layoutNotice("taper", { kind: "set" }), null);
  const n = layoutNotice("taper", { kind: "max", mm: 50 });
  assert.ok(n !== null && n.length <= 70);
});

test("criterion 6b: a new user starts folded; open is stored as JSON true under the key", () => {
  assert.equal(TEXT_LAYOUT_OPEN_KEY, "curvyo.text-layout-open");
  const s = new MemoryStorage();
  assert.equal(readLayoutOpen(s), false, "nothing stored: folded");
  writeLayoutOpen(true, s);
  assert.equal(s.getItem(TEXT_LAYOUT_OPEN_KEY), "true");
  assert.equal(readLayoutOpen(s), true);
  writeLayoutOpen(false, s);
  assert.equal(readLayoutOpen(s), false);
});

test("criterion 6b: unreadable storage means folded and never throws", () => {
  assert.equal(readLayoutOpen(undefined), false);
  assert.equal(readLayoutOpen(throwing), false);
  assert.doesNotThrow(() => writeLayoutOpen(true, throwing));
  assert.doesNotThrow(() => writeLayoutOpen(true, undefined));
  for (const garbage of ["", "1", "yes", "{", "\"true\"", "null", "TRUE"]) {
    const s = new MemoryStorage();
    s.setItem(TEXT_LAYOUT_OPEN_KEY, garbage);
    assert.equal(readLayoutOpen(s), false, `"${garbage}" is not open`);
  }
});

test("criterion 6b: folded with changed values, the description names them in block order", () => {
  const fields = LAYOUT_FIELDS.map((f) =>
    state({ resettable: f === "letter-spacing" || f === "taper" }),
  );
  assert.deepEqual(changedLayoutFields(fields), ["Letter spacing", "Taper"]);
  assert.equal(foldChangedText(false, fields), "Changed: Letter spacing, Taper");
  assert.equal(foldChangedText(true, fields), null, "open: no dot");
  const none = LAYOUT_FIELDS.map(() => state({}));
  assert.equal(foldChangedText(false, none), null, "all at default: no dot");
  const mixedTaper = LAYOUT_FIELDS.map((f) =>
    f === "taper" ? state({ mixed: true, text: "", resettable: true }) : state({}),
  );
  assert.equal(foldChangedText(false, mixedTaper), "Changed: Taper", "Mixed counts as changed");
});

// ---------------------------------------------------------------------------------------------
// Criterion 6b and 16 in the real app (Chromium over the DevTools protocol)
// ---------------------------------------------------------------------------------------------

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5198;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
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

interface BlockState {
  block: boolean;
  expanded: string | null;
  dot: boolean;
  desc: string | null;
  fields: string[];
  stored: string | null;
  notice: string;
  text: string | null;
}

const STATE = `JSON.stringify((() => {
  const blk = document.querySelector('[data-text-layout-block]');
  const tog = document.querySelector('[data-text-layout-toggle]');
  const d = tog && tog.getAttribute('aria-describedby');
  return {
    block: !!blk,
    expanded: tog ? tog.getAttribute('aria-expanded') : null,
    dot: !!document.querySelector('[data-text-layout-dot]'),
    desc: d ? document.getElementById(d)?.textContent ?? null : null,
    fields: [...document.querySelectorAll('[data-text-layout-block] [role=spinbutton]')]
      .filter((f) => f.offsetParent !== null)
      .map((f) => f.getAttribute('aria-label') + '=' + f.getAttribute('aria-valuetext')),
    stored: localStorage.getItem('curvyo.text-layout-open'),
    notice: [...document.querySelectorAll('[data-text-layout-block] [role=status]')].map((e) => e.textContent).join('|'),
    text: blk ? blk.innerText : null,
  };
})())`;

test("criteria 6b and 16 in Chromium: fold closed by default, kept across reloads, dot while folded", { skip: !BROWSER, timeout: 240_000 }, async () => {
  let vite: ChildProcess | null = null;
  let chrome: ChildProcess | null = null;
  const profile = mkdtempSync(path.join(tmpdir(), "curvyo-0079s2-"));
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const portFile = path.join(profile, "DevToolsActivePort");
    chrome = spawn(
      BROWSER!,
      ["--headless=new", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"],
      { stdio: "ignore" },
    );
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
    const pending = new Map<number, (v: { result?: { result?: { value?: unknown }; exceptionDetails?: unknown } }) => void>();
    ws.onmessage = (event) => {
      const msg = JSON.parse(String(event.data)) as { id?: number; method?: string };
      if (msg.id !== undefined) {
        pending.get(msg.id)?.(msg as never);
      }
      // The app asks before leaving a document with changes; the reload goes on.
      if (msg.method === "Page.javascriptDialogOpening") {
        id += 1;
        ws.send(JSON.stringify({ id, method: "Page.handleJavaScriptDialog", params: { accept: true } }));
      }
    };
    const send = (method: string, params: object = {}) =>
      new Promise<{ result?: { result?: { value?: unknown }; exceptionDetails?: unknown } }>((resolve) => {
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
    const press = async (key: string, code: number) => {
      await send("Input.dispatchKeyEvent", { type: "keyDown", key, code: key, windowsVirtualKeyCode: code });
      await send("Input.dispatchKeyEvent", { type: "keyUp", key, code: key, windowsVirtualKeyCode: code });
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
    const state = async () => JSON.parse((await evaluate(STATE)) as string) as BlockState;
    const fontsTab = () =>
      evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => t.getAttribute('aria-label') === 'Fonts' || t.textContent.trim() === 'Fonts').click()`);
    const open = async () => {
      await send("Page.navigate", { url: `${ORIGIN}/` });
      await waitFor("the app", async () => ((await evaluate(`!!document.querySelector('button[aria-label="Text tool (T)"]')`)) ? true : null));
      await sleep(1000);
      await fontsTab();
      await sleep(300);
    };
    const newText = async (s: string) => {
      await clickSel('button[aria-label="Text tool (T)"]');
      await click(500, 300);
      await sleep(700);
      await send("Input.insertText", { text: s });
      await sleep(300);
      await press("Escape", 27);
      await waitFor("the block", async () => ((await state()).block ? true : null), 10_000);
    };
    await send("Page.enable");
    await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });

    await open();
    assert.equal((await state()).block, false, "criterion 6: no text, no block");
    await newText("Ring");
    let s = await state();
    assert.equal(s.expanded, "false", "6b: a new user starts folded");
    assert.deepEqual(s.fields, [], "folded: no field shown");
    assert.equal(s.dot, false, "every value at default: no dot");
    assert.equal(s.stored, null);

    await clickSel("[data-text-layout-toggle]");
    s = await state();
    assert.equal(s.expanded, "true");
    assert.equal(s.stored, "true", "6b: open is stored as JSON true");
    const labels = s.fields.map((f) => f.split("=")[0]);
    assert.equal(labels[labels.indexOf("Character rotation") + 1], "Taper", "16: Taper after Character rotation");
    assert.ok(s.fields.includes("Taper=0 percent"), s.fields.join(", "));

    // Typed past the range: held at 50 % with its notice.
    await evaluate(`document.querySelector('[data-text-layout-block] [role=spinbutton][aria-label="Taper"]').focus()`);
    await press("Enter", 13);
    await evaluate(`(() => { const a = document.activeElement; if (a && a.select) a.select(); })()`);
    await send("Input.insertText", { text: "80" });
    await press("Enter", 13);
    await sleep(300);
    s = await state();
    assert.ok(s.fields.includes("Taper=50 percent"), s.fields.join(", "));
    assert.equal(s.notice, "Taper set to 50 %, the largest taper.");

    // Folded with a changed Taper: the dot and its description; folding changes no value.
    await clickSel("[data-text-layout-toggle]");
    s = await state();
    assert.equal(s.expanded, "false");
    assert.equal(s.stored, "false");
    assert.equal(s.dot, true, "6b: the dot while folded with a changed value");
    assert.equal(s.desc, "Changed: Taper");
    await clickSel("[data-text-layout-toggle]");
    assert.ok((await state()).fields.includes("Taper=50 percent"), "folding never changes a value");
    await clickSel("[data-text-layout-toggle]");

    // A reload keeps the fold: folded stays folded, open stays open.
    await open();
    await newText("Two");
    assert.equal((await state()).expanded, "false", "6b: folded is remembered");
    await clickSel("[data-text-layout-toggle]");
    await open();
    await newText("Three");
    assert.equal((await state()).expanded, "true", "6b: open is remembered");

    // Unreadable storage means folded.
    await evaluate(`localStorage.setItem('curvyo.text-layout-open', 'garbage')`);
    await open();
    await newText("Four");
    s = await state();
    assert.equal(s.expanded, "false", "6b: an unreadable value means folded");
    await send("Browser.close").catch(() => undefined);
    ws.close();
  } finally {
    chrome?.kill("SIGKILL");
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

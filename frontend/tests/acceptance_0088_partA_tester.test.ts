// Tester cases for `specs/0088-trace-recognition` Part A in the real app (Chromium via DevTools;
// set CURVYO_E2E_BROWSER=/path/to/chromium; run `npm run build:wasm` first). Written from the
// specification and UX notes; the selectors are the ones the app exposes (`data-context-bar`,
// `data-select-command`, `data-recognise-*`, `data-guessed`). Covers what the branch's own browser
// test does not: the Faces readout's tooltip (criterion 10), guessed letters reported in words and
// highlighted in the Glyph table until undo (criterion 7, REC-PARTIAL C, REC-MARK B), the progress
// and Stop of a long run (UX notes §3), the tolerance as a stored preference (criterion 6, UX
// notes §6), and a scan of the bar for operating hint text.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5289;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0088-tester");
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 60_000, every = 200): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch(() => null);
    if (got !== null) {
      return got;
    }
    await sleep(every);
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
  const move = async (x: number, y: number) => {
    await send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
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
  const centre = async (sel: string) =>
    JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
  const clickSel = async (sel: string) => {
    const c = await centre(sel);
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
  return { send, evaluate, move, click, press, centre, clickSel, close };
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
    slot: document.querySelector('[data-recognise-slot]')?.getAttribute('data-recognise-slot') ?? '',
    progress: document.querySelector('[data-context-bar] [role=progressbar]')?.getAttribute('aria-valuenow') ?? null,
    progressText: document.querySelector('[data-context-bar] [role=progressbar]')?.textContent ?? '',
    rows: [...document.querySelectorAll('[role=row][aria-level]')].map((r) => r.getAttribute('aria-label') ?? ''),
    guessed: [...document.querySelectorAll('[data-guessed]')].map((c) => c.getAttribute('aria-label') ?? c.textContent ?? ''),
  });
})()`;

type Bar = {
  name: string;
  text: string;
  commands: string[];
  faces: string;
  tolerance: string;
  notice: string;
  row: string;
  slot: string;
  progress: string | null;
  progressText: string;
  rows: string[];
  guessed: string[];
};

/** Words that tell the maker what to do rather than name a control, a state or a result. */
const HINT = /\b(press|click|drag|try|tip|hint|you can|to start|choose|pick|hold)\b/i;

test("0088 Part A tester in Chromium: guessed letters, readout tooltip, progress and Stop, tolerance preference", { skip: !BROWSER, timeout: 540_000 }, async () => {
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
    const focusCanvas = () => s.evaluate(`[...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus()`);
    const selectAll = async () => {
      await s.clickSel('button[aria-label^="Select"]');
      await focusCanvas();
      await s.press("a", 65, 2);
    };
    const run = () => s.evaluate(`document.querySelector('[data-context-bar] button[data-recognise-slot]')?.click()`);
    const tooltipOf = async (sel: string) => {
      const c = await s.centre(sel);
      assert.ok(c, sel);
      await s.move(c[0], c[1]);
      return waitFor(`the tooltip of ${sel}`, async () => {
        const t = (await s.evaluate(`document.querySelector('[role=tooltip]')?.textContent ?? ''`)) as string;
        return t === "" ? null : t;
      }, 5000);
    };
    const hints: string[] = [];
    const scan = (where: string, text: string) => {
      if (HINT.test(text)) {
        hints.push(`${where}: ${text}`);
      }
    };

    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('button[aria-label="Text tool (T)"]')`)) ? true : null));
    await sleep(1500);
    await s.evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => (t.getAttribute('aria-label') ?? t.textContent).startsWith('Layers'))?.click()`);
    await sleep(300);

    // "Hamburg 1999" in the default face (Source Sans 3 Regular) at 36 mm cap height, turned into
    // paths. In the engine this line reads with two letters guessed (H and a) and the m left.
    await s.clickSel('button[aria-label="Text tool (T)"]');
    await s.click(300, 400);
    await sleep(700);
    await s.send("Input.insertText", { text: "Hamburg 1999" });
    await sleep(300);
    await s.press("Escape", 27);
    await selectAll();
    await waitFor("the Size field", async () => ((await s.evaluate(`!!document.querySelector('input[aria-label="Size"]')`)) ? true : null), 10_000);
    await s.evaluate(`(() => { const i = document.querySelector('input[aria-label="Size"]'); i.focus(); i.select(); })()`);
    await s.send("Input.insertText", { text: "36" });
    await s.press("Enter", 13);
    await sleep(500);
    await selectAll();
    await waitFor("Object to path", async () => ((await s.evaluate(`!!document.querySelector('button[title^="Convert the selected"]')`)) ? true : null), 10_000);
    await s.clickSel('button[title^="Convert the selected"]');
    await sleep(500);
    // Implementer, 2026-10-31 (fixture only): the line read with H and a guessed only through
    // the large-size defect fixed since; H and a are now moved 1 mm up (one nudge) so the laid-out
    // text puts them off and they are guessed. The group is opened in the Layers list, its two
    // back-most rows (H, a) selected and nudged, then the group row selects all again.
    await s.evaluate(`(() => { const r = document.querySelector('[role=row][aria-level]'); r?.focus(); r?.querySelector('[tabindex]')?.focus(); })()`);
    await s.press("ArrowRight", 39);
    await waitFor("the glyph rows", async () => (((await bar()).rows.length > 2) ? true : null), 10_000);
    const rowCentre = async (fromEnd: number) =>
      JSON.parse((await s.evaluate(`(() => { const rows = [...document.querySelectorAll('[role=row][aria-level]')];
        const r = rows[rows.length - ${fromEnd}]; r.scrollIntoView({ block: 'center' });
        const b = r.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string) as [number, number];
    const [hx, hy] = await rowCentre(1);
    await s.click(hx, hy);
    const [ax, ay] = await rowCentre(2);
    for (const type of ["mouseMoved", "mousePressed", "mouseReleased"]) {
      await s.send("Input.dispatchMouseEvent", { type, x: ax, y: ay, button: "left", clickCount: 1, modifiers: 2 });
    }
    await sleep(300);
    await focusCanvas();
    await s.press("ArrowUp", 38);
    // The group row selects every glyph again.
    const [gx, gy] = await rowCentre(12);
    await s.click(gx, gy);
    await focusCanvas();
    let b = await waitFor("the Recognise text button", async () => {
      const got = await bar();
      return got.commands.includes("recognise-text") ? got : null;
    }, 10_000);
    const pathRows = b.rows;

    // Criterion 10: the readout and its tooltip, the fixed order of the sources; no selector.
    await s.clickSel('[data-select-command="recognise-text"]');
    b = await bar();
    assert.equal(b.name, "Recognise text");
    assert.match(b.faces, /^\d face sources?$/, b.faces);
    const facesTip = await tooltipOf("[data-recognise-faces]");
    // Lead decision 2026-10-31: the hint "stops at the first match" left the tooltip.
    assert.equal(facesTip, "Document, favourites, installed, included, system fonts");
    scan("Faces tooltip", facesTip);
    scan("Recognise bar", b.text);
    assert.equal(
      await s.evaluate(`document.querySelectorAll('[data-context-bar] select, [data-context-bar] [role=combobox], [data-context-bar] [role=listbox]').length`),
      0,
      "no face source selector",
    );
    assert.match(b.tolerance, /^0\.30 millimetres$/, "default 0.3 mm");

    // UX notes §3: the first (cold) run is long enough to show its progress after 400 ms with a
    // Stop; Escape stops it, nothing changes, and Retry is offered.
    const rowsIdle = (await bar()).rows;
    await run();
    let progress: Bar | null = null;
    try {
      progress = await waitFor("progress", async () => {
        const got = await bar();
        return got.progress !== null ? got : null;
      }, 3000, 30);
    } catch {
      progress = null;
    }
    if (progress) {
      scan("progress", progress.text);
      assert.match(progress.progressText, /\d+ %/, progress.progressText);
      assert.ok(await s.centre('[data-context-bar] button[aria-label="Stop (Esc)"]'), "Stop (Esc) in the button");
      await focusCanvas();
      await s.press("Escape", 27);
      b = await waitFor("stopped", async () => {
        const got = await bar();
        return got.progress === null && got.text.includes("Stopped") ? got : null;
      }, 10_000);
      assert.ok(b.text.includes("Retry"), b.text);
      assert.deepEqual(b.rows, rowsIdle, "Stop changes nothing");
      await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((e) => e.textContent?.includes('Retry'))?.click()`);
    } else {
      console.log("the cold run showed no progress within 3 s");
      await focusCanvas();
    }

    // Criterion 7: the run reports the guessed letters in words; the new text is selected.
    const started = Date.now();
    if (!progress) {
      await run();
    }
    let firstProgress: string | null = null;
    b = await waitFor("the result", async () => {
      const got = await bar();
      if (got.progress !== null && firstProgress === null) {
        firstProgress = `${Date.now() - started} ms: ${got.progressText}`;
      }
      return got.notice.includes("guessed") || got.row !== "" ? got : null;
    }, 120_000, 30);
    const took = Date.now() - started;
    console.log(`first run: ${took} ms wall in the page; progress ${firstProgress ?? "not shown"}`);
    if (took > 1000) {
      assert.ok(firstProgress, `a ${took} ms run shows its progress after 400 ms`);
    }
    scan("notice", b.notice);
    // The "Recognised N letters as ..." part is gone (customer rule of 2026-10-31); the guessed
    // letters stay as a warning.
    assert.match(b.notice, /^2 letters guessed: H \(1\), a \(2\)\.$/, b.notice);
    // The face used to show in the bar only through the result notice (gone, customer rule of 2026-10-31).
    assert.ok(b.text.includes("Stroke to path"), `the new group is selected: ${b.text}`);

    // The Glyph table highlights the guessed characters (no canvas overlay).
    // The selection is the new text and the leftover path; the text alone (a part of the run's
    // selection, picked in the Layers panel) is the Glyph table's subject.
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level] [aria-label=Expand]')].forEach((e) => e.click())`);
    await sleep(400);
    const textRow = await s.centre('[role=row][aria-level="2"]');
    assert.ok(textRow, "the group's rows");
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => /^Hamburg|^Ha /.test(r.getAttribute('aria-label') ?? '') && r.getAttribute('aria-level') !== '1')?.setAttribute('data-tester-text', '')`);
    await s.clickSel('[data-tester-text]');
    await sleep(400);
    // The Glyphs button is in the text controls' second row.
    if (!(await s.centre('button[aria-label="Glyphs"]'))) {
      await s.evaluate(`document.querySelector('[data-context-bar] button[aria-controls][aria-expanded="false"]')?.click()`);
      await sleep(400);
    }
    await s.clickSel('button[aria-label="Glyphs"]');
    b = await waitFor("the guessed cells", async () => {
      const got = await bar();
      return got.guessed.length > 0 ? got : null;
    }, 10_000);
    assert.equal(b.guessed.length, 2, `two guessed characters: ${b.guessed.join(" | ")}`);
    assert.ok(b.guessed.some((g) => /\bH\b|U\+0048/.test(g)) && b.guessed.some((g) => /\ba\b|U\+0061/.test(g)), b.guessed.join(" | "));
    assert.equal(
      await s.evaluate(`document.querySelectorAll('[data-recognise-overlay], [data-guessed-overlay]').length`),
      0,
      "no overlay element (the canvas itself is not inspected here)",
    );

    // Undo brings the paths back and ends the record.
    await focusCanvas();
    await s.press("z", 90, 2);
    b = await waitFor("undo", async () => {
      const got = await bar();
      return got.rows.length > 0 && got.rows.every((r) => !r.endsWith(", text")) ? got : null;
    }, 10_000);
    assert.equal(b.rows[0], pathRows[0], "the converted group is back");
    assert.equal(b.guessed.length, 0, `no guessed marks after undo: ${b.guessed.join(" | ")}`);

    // UX notes §3: a run longer than 400 ms shows the percentage and Stop; Stop (Escape)
    // changes nothing and offers Retry. The mirrored line searches every source.
    await selectAll();
    await s.press("h", 72);
    await s.clickSel('[data-select-command="recognise-text"]');
    const rowsBefore = (await bar()).rows;
    await run();
    let sawProgress: Bar | null = null;
    try {
      sawProgress = await waitFor("progress", async () => {
        const got = await bar();
        return got.progress !== null ? got : null;
      }, 3000, 30);
    } catch {
      sawProgress = null;
    }
    if (sawProgress) {
      assert.match(sawProgress.progressText, /\d+ %/, sawProgress.progressText);
      await focusCanvas();
      await s.press("Escape", 27);
      b = await waitFor("stopped", async () => {
        const got = await bar();
        return got.slot !== "running" ? got : null;
      }, 10_000);
      assert.ok(b.text.includes("Stopped") && b.text.includes("Retry"), b.text);
      assert.deepEqual(b.rows, rowsBefore, "Stop changes nothing");
    } else {
      b = await waitFor("the outcome", async () => {
        const got = await bar();
        return got.row !== "" || got.notice !== "" ? got : null;
      }, 120_000);
      console.log(`no progress shown; the mirrored run ended as: ${b.row || b.notice}`);
    }

    // Criterion 6: the tolerance is a per-user preference that survives a reload.
    await selectAll();
    if ((await bar()).name !== "Recognise text") {
      await s.clickSel('[data-select-command="recognise-text"]');
    }
    await s.evaluate(`document.querySelector('[data-context-bar] [role=spinbutton][aria-label=Tolerance]').focus()`);
    await s.press("F2", 113);
    await s.evaluate(`document.activeElement?.select?.()`);
    await s.send("Input.insertText", { text: "0.45" });
    await s.press("Enter", 13);
    await sleep(300);
    assert.equal(await s.evaluate(`localStorage.getItem('curvyo.recognise-tolerance')`), "0.45");
    await s.send("Page.reload");
    await waitFor("the app again", async () => ((await s.evaluate(`!!document.querySelector('button[aria-label="Text tool (T)"]')`)) ? true : null));
    await sleep(1500);
    await s.clickSel('button[aria-label="Text tool (T)"]');
    await s.click(300, 400);
    await sleep(500);
    await s.send("Input.insertText", { text: "Lid" });
    await s.press("Escape", 27);
    await selectAll();
    await waitFor("Object to path", async () => ((await s.evaluate(`!!document.querySelector('button[title^="Convert the selected"]')`)) ? true : null), 10_000);
    await s.clickSel('button[title^="Convert the selected"]');
    await sleep(500);
    await s.clickSel('[data-select-command="recognise-text"]');
    b = await bar();
    assert.match(b.tolerance, /^0\.45 millimetres$/, b.tolerance);

    assert.deepEqual(hints, [], `operating hint text: ${hints.join(" / ")}`);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

// Tester cases for `specs/0091-trace-text-recognition` slice 1 in Chromium (CURVYO_E2E_BROWSER=
// /path/to/chromium; the wasm bindings must be built), on the tester's own posters
// (`tests/assets/vectrace-fixtures/tester/`). Written from the specification and the UX notes.
// Covers: the poster end to end with the warning wording (criterion 5), texts above the group
// (6), one undo (11), the weights fetched once from the app itself and nothing else leaving the
// page (14), no second OCR after a setting change (16), the switch as an app setting (17), a stop
// during the OCR (19), no notice for a clean line (5), the OCR, trace and fit in the editor wasm
// against the native record of `curvyo-editor-wasm/tests/acceptance_0091_tester.rs` (13 across
// builds, ADR 0028 §6), and the times: OCR and fit wall time in the worker, main-thread long
// tasks over the whole recognise and trace (20, 21). Criterion 15 as amended: the record and
// `docs/third-party-assets.md` (no in-app licence page in this story).

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5297;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const ROOT = path.resolve(FRONTEND, "..");
const SCRATCH = path.resolve(ROOT, "target", "e2e-0091-tester");
const FIXTURES = path.resolve(ROOT, "tests", "assets", "vectrace-fixtures", "tester");
const POSTER = path.join(FIXTURES, "poster.png");
const CLEAN = path.join(FIXTURES, "clean.png");
const NATIVE = path.resolve(ROOT, "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0091_tester_native.json");
const BAR = "[role=toolbar][aria-label=Trace]";
/** The status texts right after Create paths on the poster (set by the end-to-end case). */
let selectionAfterCreate: string | null = null;
/** The layer rows right after Create paths on the poster (set by the end-to-end case). */
let rowsAfterCreate: string[] | null = null;

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 60_000): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch(() => null);
    if (got !== null) {
      return got;
    }
    await sleep(100);
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
    const msg = JSON.parse(String(event.data)) as { id?: number; method?: string; params?: { request?: { url: string }; backendNodeId?: number } };
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
  const clickSel = async (sel: string) => {
    const c = JSON.parse(
      (await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null';
        e.scrollIntoView({ block: 'center' });
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
    assert.ok(c, `${sel} is on the page`);
    await click(c[0], c[1]);
  };
  const choose = async (sel: string, file: string) => {
    const before = choosers.length;
    await clickSel(sel);
    const node = await waitFor("the file chooser", async () => (choosers.length > before ? choosers[choosers.length - 1] : null), 10_000);
    await send("DOM.setFileInputFiles", { files: [file], backendNodeId: node });
    await sleep(1500);
  };
  const key = async (k: string, code: number, modifiers = 0, codeName = `Key${k.toUpperCase()}`) => {
    await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: codeName, windowsVirtualKeyCode: code, modifiers });
    await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: codeName, windowsVirtualKeyCode: code, modifiers });
    await sleep(400);
  };
  const statusText = async () =>
    (await evaluate(`[...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent).filter((t) => t).join(' | ')`)) as string;
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
  return { send, evaluate, clickSel, choose, key, statusText, layerRows, close, requests };
}

type S = Awaited<ReturnType<typeof session>>;

/** Runs `body` with vite and one Chromium on a fresh profile. */
async function withApp(body: (s: S) => Promise<void>) {
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
  let s: S | null = null;
  try {
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    s = await session(profile);
    await body(s);
  } finally {
    await s?.close();
    vite.kill();
    rmSync(profile, { recursive: true, force: true });
  }
}

/** Opens the app, imports `file`, starts a sampler of the bar's state and long tasks. */
async function importImage(s: S, file: string) {
  await s.send("Page.navigate", { url: `${ORIGIN}/` });
  await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
  await sleep(1500);
  await s.evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => (t.getAttribute('aria-label') ?? '').startsWith('Layers'))?.click()`);
  await sleep(300);
  await s.choose("[data-import-button]", file);
  await waitFor("the image row", async () => ((await s.layerRows()).some((r) => r?.includes(path.basename(file))) ? true : null));
  await sleep(500);
  // Every change of the slot, its progress label and the status texts, with its time; and the
  // main thread's tasks over 50 ms.
  await s.evaluate(`(() => {
    window.__long = []; window.__log = [];
    new PerformanceObserver((l) => { for (const e of l.getEntries()) window.__long.push([e.startTime, e.duration]); }).observe({ type: 'longtask' });
    let last = '';
    const probe = () => {
      const slot = document.querySelector('${BAR} [data-trace-slot]')?.getAttribute('data-trace-slot') ?? '-';
      const label = document.querySelector('${BAR} [role=progressbar]')?.getAttribute('aria-label') ?? '';
      const status = [...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent).filter((t) => t).join(' | ');
      const now = slot + '#' + label + '#' + status;
      if (now !== last) { last = now; window.__log.push([performance.now(), slot, label, status]); }
    };
    new MutationObserver(probe).observe(document.body, { subtree: true, childList: true, attributes: true, characterData: true });
  })()`);
}

async function openAdvanced(s: S) {
  if (!(await s.evaluate(`!!document.querySelector('${BAR} [aria-label="Recognise text"]')`))) {
    await s.evaluate(`document.querySelector('${BAR} [aria-label="More trace options"]').click()`);
    await sleep(500);
  }
}

/** Opens the Trace bar and turns Recognise text on (criterion 17 as amended by Frage 116 B a,
 * 2026-11-01: off by default), which starts the job with text. */
async function openTrace(s: S) {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 20_000);
  await openAdvanced(s);
  const pressed = () => s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]')?.getAttribute('aria-pressed')`);
  assert.equal(await pressed(), "false", "criterion 17: off by default");
  await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
  await waitFor("Recognise text on", async () => ((await pressed()) === "true" ? true : null), 5_000);
}

const restingSlot = (s: S) => s.evaluate(`!!document.querySelector('${BAR} [data-trace-slot=none]')`);
const createPaths = (s: S) => s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Trace')?.click()`);
type Log = [number, string, string, string][];
const log = async (s: S) => JSON.parse((await s.evaluate(`JSON.stringify(window.__log)`)) as string) as Log;
const longTasks = async (s: S) => JSON.parse((await s.evaluate(`JSON.stringify(window.__long)`)) as string) as [number, number][];

test("0091 tester c4 c5 c6 c11 c14 c16 c17 c20 c21: the poster end to end, the warning, one undo, the weights once, the times", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await importImage(s, POSTER);
    const t0 = (await s.evaluate(`performance.now()`)) as number;
    await openTrace(s);
    await waitFor("the text lines", async () => ((await s.statusText()).includes("text lines set") ? true : null), 180_000);
    await waitFor("the preview's fit", async () => ((await restingSlot(s)) ? true : null), 180_000);
    const done = (await s.evaluate(`performance.now()`)) as number;
    const steps = await log(s);
    const reading = steps.filter((e) => e[2] === "Reading text");
    const readStart = reading[0]?.[0];
    const readEnd = steps.find((e) => e[0] > (reading.at(-1)?.[0] ?? 0) && e[2] !== "Reading text")?.[0];
    const linesAt = steps.find((e) => e[3].includes("text lines set"))?.[0];
    assert.ok(readStart !== undefined && readEnd !== undefined && linesAt !== undefined, JSON.stringify(steps.slice(0, 20)));
    const long = (await longTasks(s)).filter(([at]) => at >= t0);
    const busy = long.reduce((a, [, d]) => a + d, 0);
    console.log(
      `# poster in the app: OCR stage ("Reading text", weights load included) ${((readEnd - readStart) / 1000).toFixed(2)} s; ` +
        `trace to the line count ${((linesAt - readEnd) / 1000).toFixed(2)} s; fit until the button rests ${((done - linesAt) / 1000).toFixed(2)} s; ` +
        `all ${((done - t0) / 1000).toFixed(2)} s; main-thread long tasks ${long.length}, longest ${Math.max(0, ...long.map(([, d]) => d)).toFixed(0)} ms, total ${busy.toFixed(0)} ms`,
    );
    // UX notes §4 with Frage 115 B / 117 B: 8 found, every plausible line set, the foreign
    // Liberation Mono line in the nearest face.
    const status = await s.statusText();
    // "8 text lines set" when every line is set, "n of 8 text lines set" otherwise (UX notes §4).
    const setMatch = /(?:(\d+) of )?8 text lines set/.exec(status);
    assert.ok(setMatch, status);
    const set = Number(setMatch[1] ?? 8);
    // Criterion 14: the three weight files once each, from the app; nothing leaves the page's origin.
    const models = s.requests.filter((u) => u.includes("/models/"));
    console.log("# model requests:", JSON.stringify(models.map((u) => u.replace(ORIGIN, ""))));
    for (const f of ["det.onnx", "rec.onnx", "dict.txt"]) {
      assert.equal(models.filter((u) => u.endsWith(`/models/paddleocr-latin/${f}`)).length, 1, f);
    }
    assert.deepEqual(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")), []);
    // Criterion 17: turned on above (off by default), kept with the Trace settings in local storage.
    await openAdvanced(s);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').getAttribute('aria-pressed')`), "true");
    // 0088 Part B: "Recognise shapes" is the fourth toggle, present and off by default.
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise shapes"]')?.getAttribute('aria-pressed') ?? null`), "false");
    // Criterion 17 as amended by slice 2 (ux-notes §1): drawn in Multicolour too, at the same place
    // (right after Drop border shapes), keeping its value; and still on with Single colour.
    const output = (name: string) => s.evaluate(`[...document.querySelectorAll('${BAR} [role=radio], ${BAR} button')].find((b) => b.textContent?.trim() === ${JSON.stringify(name)})?.click()`);
    await output("Multicolour");
    await sleep(800);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]')?.getAttribute('aria-pressed') ?? null`), "true", "drawn and on in Multicolour (slice 2)");
    const toggles = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [aria-pressed]')].map((e) => e.getAttribute('aria-label')))`)) as string) as string[];
    assert.equal(toggles.indexOf("Recognise text"), toggles.indexOf("Drop border shapes") + 1, `the toggle after Drop border shapes: ${JSON.stringify(toggles)}`);
    assert.equal(toggles.indexOf("Recognise shapes"), toggles.indexOf("Recognise text") + 1, `Recognise shapes after Recognise text: ${JSON.stringify(toggles)}`);
    await output("Single colour");
    await sleep(800);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]')?.getAttribute('aria-pressed')`), "true");
    await waitFor("the single-colour preview", async () => ((await restingSlot(s)) && /8 text lines set/.test(await s.statusText()) ? true : null), 180_000);
    const advancedText = (await s.evaluate(`document.querySelector('${BAR}').innerText`)) as string;
    console.log("# Trace bar text with Advanced open:", JSON.stringify(advancedText));
    // Criterion 16: a setting changed: no "Reading text" again, no weight fetched again, a new preview.
    const before = (await log(s)).length;
    const requestsBefore = s.requests.length;
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Drop border shapes"]').click()`);
    await sleep(500);
    await waitFor("the new preview", async () => {
      const l = (await log(s)).slice(before);
      const ran = l.findIndex((e) => e[1] === "running");
      return ran >= 0 && l.slice(ran).some((e) => e[1] === "none") ? true : null;
    }, 120_000);
    await sleep(1000);
    const again = (await log(s)).slice(before);
    assert.ok(!again.some((e) => e[2] === "Reading text"), `no OCR again: ${JSON.stringify(again.map((e) => e[2]))}`);
    assert.equal(s.requests.slice(requestsBefore).filter((u) => u.includes("/models/")).length, 0);
    const back = (await log(s)).length;
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Drop border shapes"]').click()`);
    await sleep(500);
    // The preview of the setting back runs and rests (the fit before the trace makes it longer
    // than the debounce, so a resting slot alone can be the gap before it).
    await waitFor("the preview back", async () => {
      const l = (await log(s)).slice(back);
      const ran = l.findIndex((e) => e[1] === "running");
      return ran >= 0 && l.slice(ran).some((e) => e[1] === "none") && (await restingSlot(s)) ? true : null;
    }, 120_000);
    await sleep(1000);
    const stored = (await s.evaluate(`localStorage.getItem('curvyo.trace-values')`)) as string | null;
    assert.ok(stored !== null && stored.includes('"recogniseTextOn":true'), stored ?? "");
    // Create paths: the texts above the "Trace of" group (criterion 6), one warning (criterion 5).
    const rowsBefore = await s.layerRows();
    await createPaths(s);
    const rows = await waitFor("the text rows", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Größe")) ? r : null;
    }, 180_000).catch(async (e: unknown) => {
      throw new Error(`${String(e)}: rows ${JSON.stringify(await s.layerRows())}; status ${await s.statusText()}`);
    });
    console.log("# layer rows after Create paths:", JSON.stringify(rows));
    const group = rows.findIndex((x) => x?.includes("Trace of"));
    const texts = rows.map((x, i) => [x, i] as const).filter(([x]) => /Größe|Übermaß|Äpfel|Tel\.|Fräsen|Straße|Mono|Brücke/.test(x ?? ""));
    rowsAfterCreate = rows;
    assert.ok(rows.some((x) => /Mono/.test(x ?? "")), `the foreign line is text (115 B): ${JSON.stringify(rows)}`);
    assert.equal(texts.length, set, JSON.stringify(rows));
    assert.ok(group >= 0 && texts.every(([, i]) => i < group), `texts above the group: ${JSON.stringify(rows)}`);
    await sleep(800);
    const notice = await s.statusText();
    console.log("# notices after Create paths:", JSON.stringify(notice));
    selectionAfterCreate = notice;
    // Frage 115 B / 117 B: the two lines no face fits are set in the nearest face and named.
    assert.ok(
      // Criterion 26 as amended (2 px or 5 % tolerance in the fit's own pixel space): fewer guessed.
      notice.includes('3 letters guessed: l (3), 2 (8), l (15). 2 lines set in Source Sans 3, all letters guessed: "Fräsen ab 3 Stück, 12.50…", "Mono 314 Zoll".'),
      `the warning of UX notes §5: ${notice}`,
    );

    // Criterion 11: one undo, the document as before.
    await s.evaluate(`document.activeElement?.blur?.()`);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(rowsBefore) ? true : null), 20_000).catch(async (e: unknown) => {
      throw new Error(`${String(e)}: ${JSON.stringify(await s.layerRows())} vs ${JSON.stringify(rowsBefore)}`);
    });
  });
});

test(
  "0091 tester c9: after Create paths the poster's texts lie on the 210 x 210 mm picture",
  { skip: !BROWSER },
  () => {
    const m = /(\d+(?:\.\d+)?) by (\d+(?:\.\d+)?) mm/.exec(selectionAfterCreate ?? "");
    assert.ok(m, `the selection size after Create paths: ${selectionAfterCreate}`);
    assert.ok(Number(m[1]) <= 211 && Number(m[2]) <= 211, `group and texts within the picture: ${m[0]}`);
  },
);

test(
  "0091 tester c4, Frage 115 B / 117 B: a line with no sure letter (\"Fräsen ab 3 Stück, 12.50 EUR\", 0 of 23 sure natively) becomes text in the nearest face",
  { skip: !BROWSER },
  () => {
    assert.ok(rowsAfterCreate, "the end-to-end case ran");
    assert.ok(rowsAfterCreate.some((x) => /Fräsen/.test(x ?? "")), JSON.stringify(rowsAfterCreate));
  },
);

test("0091 tester c5: a clean line gives no notice at all after Create paths", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await importImage(s, CLEAN);
    await openTrace(s);
    await waitFor("the text line", async () => ((await s.statusText()).includes("1 text line set") ? true : null), 180_000);
    await waitFor("the preview's fit", async () => ((await restingSlot(s)) ? true : null), 180_000);
    const mark = (await log(s)).length;
    await createPaths(s);
    await waitFor("the text row", async () => ((await s.layerRows()).some((x) => x?.includes("Größe")) ? true : null), 120_000);
    await sleep(1500);
    const after = (await log(s)).slice(mark).map((e) => e[3]);
    const words = [...new Set(after.flatMap((t) => t.split(" | ")))];
    console.log("# status texts after Create paths (clean):", JSON.stringify(words));
    assert.ok(!words.some((w) => /guessed|nearest font|created|recognised|done|success/i.test(w)), JSON.stringify(words));
  });
});

test("0091 tester c19: a stop while the text is read leaves the document unchanged and caches nothing", { skip: !BROWSER, timeout: 400_000 }, async () => {
  await withApp(async (s) => {
    await importImage(s, POSTER);
    const rows = await s.layerRows();
    await openTrace(s);
    await waitFor("Reading text", async () => ((await s.evaluate(`document.querySelector('${BAR} [role=progressbar]')?.getAttribute('aria-label')`)) === "Reading text" ? true : null), 60_000);
    await s.evaluate(`document.querySelector('${BAR} [aria-label="Stop (Esc)"]').click()`);
    await waitFor("Stopped", async () => ((await s.evaluate(`document.querySelector('${BAR} [data-trace-slot]')?.getAttribute('data-trace-slot')`)) === "stopped" ? true : null), 20_000);
    assert.deepEqual(await s.layerRows(), rows, "no document change");
    const mark = (await log(s)).length;
    await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Retry').click()`);
    const stopAt = (await log(s)).slice(0, mark);
    await waitFor("Reading text again", async () => ((await log(s)).slice(mark).some((e) => e[2] === "Reading text") ? true : null), 60_000).catch(async (e: unknown) => {
      const fmt = (l: Log) => JSON.stringify(l.map((x) => [Math.round(x[0]), x[1], x[2], x[3].slice(0, 40)]));
      throw new Error(`${String(e)}; before the stop ${fmt(stopAt.slice(-6))}; after ${fmt((await log(s)).slice(mark, mark + 12))}`);
    });
    await waitFor("the text lines", async () => ((await s.statusText()).includes("text lines set") ? true : null), 180_000);
  });
});

/** Recursively compares two JSON values: equal strings and structure, numbers within `tol`; returns
 * the largest numeric difference, or throws at the first structural difference. */
function compare(a: unknown, b: unknown, tol: number, at = "$"): number {
  if (typeof a === "number" && typeof b === "number") {
    const d = Math.abs(a - b);
    assert.ok(d <= tol, `${at}: ${a} vs ${b}`);
    return d;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    assert.equal(a.length, b.length, `${at}: length`);
    return a.reduce<number>((m, x, i) => Math.max(m, compare(x, b[i], tol, `${at}[${i}]`)), 0);
  }
  if (a !== null && b !== null && typeof a === "object" && typeof b === "object") {
    const ka = Object.keys(a).sort();
    assert.deepEqual(ka, Object.keys(b).sort(), `${at}: keys`);
    return ka.reduce((m, k) => Math.max(m, compare((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], tol, `${at}.${k}`)), 0);
  }
  assert.equal(a, b, at);
  return 0;
}

test("0091 tester c13 across builds: the editor wasm reads, traces and fits the poster as natively", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const native = JSON.parse(readFileSync(NATIVE, "utf8")) as {
    width: number; height: number; rgba_fnv32: number; settings: string; map: number[]; ocr: string; wire: number[]; fit_request: string; fit_reply: string;
  };
  await withApp(async (s) => {
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    const png = readFileSync(POSTER).toString("base64");
    const out = JSON.parse(
      (await s.evaluate(`(async () => {
        const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await m.default();
        const png = Uint8Array.from(atob(${JSON.stringify(png)}), (c) => c.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([png], { type: 'image/png' }), { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
        const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
        const ctx = canvas.getContext('2d', { colorSpace: 'srgb' });
        ctx.drawImage(bitmap, 0, 0);
        const rgba = new Uint8Array(ctx.getImageData(0, 0, bitmap.width, bitmap.height).data.buffer);
        let h = 0x811c9dc5;
        for (let i = 0; i < rgba.length; i++) { h = Math.imul(h ^ rgba[i], 0x01000193) >>> 0; }
        const get = async (u) => new Uint8Array(await (await fetch(u)).arrayBuffer());
        const sink = { report() {} };
        let t = performance.now();
        const loaded = m.ocr_load(await get('/models/paddleocr-latin/det.onnx'), await get('/models/paddleocr-latin/rec.onnx'), new TextDecoder().decode(await get('/models/paddleocr-latin/dict.txt')));
        const loadMs = performance.now() - t;
        t = performance.now();
        const ocr = m.ocr_job(rgba, bitmap.width, bitmap.height, sink);
        const ocrMs = performance.now() - t;
        t = performance.now();
        const wire = Array.from(m.trace_text_job(rgba, bitmap.width, bitmap.height, Float64Array.from(${JSON.stringify(native.map)}), ${JSON.stringify(native.settings)}, ocr, sink));
        const passMs = performance.now() - t;
        const request = ${JSON.stringify(native.fit_request)};
        t = performance.now();
        let fetchMs = 0;
        let reply = m.fit_start(request, sink);
        for (let i = 0; i < 64; i++) {
          const v = JSON.parse(reply);
          if (v.kind === 'finished') break;
          for (const [file, index] of v.faces) {
            const f0 = performance.now();
            const bytes = await get('/fonts/' + file);
            fetchMs += performance.now() - f0;
            m.fit_add_face(file, index, bytes);
          }
          reply = m.fit_step(sink);
        }
        const fitMs = performance.now() - t - fetchMs;
        return JSON.stringify({ w: bitmap.width, h: bitmap.height, fnv: h, loaded, loadMs, ocr, ocrMs, wire, passMs, reply, fitMs });
      })()`)) as string,
    ) as { w: number; h: number; fnv: number; loaded: string; loadMs: number; ocr: string; ocrMs: number; wire: number[]; passMs: number; reply: string; fitMs: number };
    console.log(
      `# editor wasm on the page's main thread: weights load ${(out.loadMs / 1000).toFixed(2)} s, OCR ${(out.ocrMs / 1000).toFixed(2)} s, ` +
        `text pass with trace ${(out.passMs / 1000).toFixed(2)} s, font fit ${(out.fitMs / 1000).toFixed(2)} s (wall)`,
    );
    assert.deepEqual([out.w, out.h, out.fnv], [native.width, native.height, native.rgba_fnv32], "the same pixels");
    assert.equal(out.loaded, "");
    const ocrDiff = compare(JSON.parse(out.ocr), JSON.parse(native.ocr), 1e-3);
    const wireDiff = compare(out.wire, native.wire, 1e-6);
    const fitDiff = compare(JSON.parse(out.reply), JSON.parse(native.fit_reply), 1e-6);
    console.log(
      `# wasm vs native: OCR ${out.ocr === native.ocr ? "identical" : `max diff ${ocrDiff}`}, wire ${JSON.stringify(out.wire) === JSON.stringify(native.wire) ? "identical" : `max diff ${wireDiff}`}, fit ${out.reply === native.fit_reply ? "identical" : `max diff ${fitDiff}`}`,
    );
  });
});

test("0091 tester c15: the licence record names the weights' licence and the docs list them with rten's", () => {
  const record = readFileSync(path.join(FRONTEND, "public", "models", "paddleocr-latin", "model.toml"), "utf8");
  assert.match(record, /^licence = "Apache-2\.0"$/m);
  assert.match(record, /^training_data = ".*Frage 112 A/m);
  assert.equal((record.match(/^sha256 = "[0-9a-f]{64}"$/gm) ?? []).length, 3);
  // Amended criterion 15: `docs/third-party-assets.md` lists the weights with the same licence and
  // source as the record.
  const assets = readFileSync(path.join(ROOT, "docs", "third-party-assets.md"), "utf8");
  const section = assets.slice(assets.indexOf("## PaddleOCR"));
  assert.ok(section.startsWith("## PaddleOCR"), "the weights have a section");
  assert.match(section.split("\n## ")[0], /Apache-2\.0/);
  const source = /^source_url = "(.*)"$/m.exec(record)?.[1];
  assert.ok(source && section.split("\n## ")[0].includes(source), `the source ${source} is listed`);
  const adr = readFileSync(path.join(ROOT, "docs", "adr", "0028-ocr-text-in-trace.md"), "utf8");
  assert.match(adr, /`rten` MIT OR Apache-2\.0/);
});

// Round 2 (customer answers of 2026-11-01, Frage 115 B / 116 B / 117 B): one progress and one
// trace per run (criterion 25), the reading and fit pool of min(cores - 1, 4) workers, a stop
// while the pool reads and while it fits (criterion 19), the customer's screenshot, and the wall
// times of the whole job with the main thread's long tasks.

const SCREENSHOT = path.resolve(ROOT, "tests", "assets", "vectrace-fixtures", "customer", "font-manager-screenshot.png");

/** Wraps every `Worker` of the page: logs the requests it is sent (`type`, whether text lines go
 * with a trace), the replies it posts (`kind`, progress left out) and its end; samples the Trace
 * bar's percent. Installed before the page loads. */
const SPY = `(() => {
  const Orig = window.Worker;
  window.__workers = 0; window.__msgs = []; window.__pct = [];
  window.Worker = class extends Orig {
    constructor(...a) {
      super(...a);
      const i = window.__workers++;
      this.__i = i;
      this.addEventListener('message', (e) => {
        const k = e.data && e.data.kind;
        if (k && k !== 'progress') window.__msgs.push([performance.now(), i, 'in', k, '']);
      });
    }
    postMessage(m, t) {
      const ty = (m && m.type) || '';
      const lines = ty === 'trace' && typeof m.lines === 'string' && m.lines.length > 2 ? 'lines' : '';
      window.__msgs.push([performance.now(), this.__i, 'out', ty, lines]);
      return t === undefined ? super.postMessage(m) : super.postMessage(m, t);
    }
    terminate() { window.__msgs.push([performance.now(), this.__i, 'end', '', '']); super.terminate(); }
  };
  let last = null;
  const probe = () => {
    const bar = document.querySelector('${BAR} [role=progressbar]');
    const v = bar ? bar.getAttribute('aria-valuenow') : null;
    const now = bar ? (v ?? '') + '#' + (bar.getAttribute('aria-label') ?? '') : null;
    if (now !== last) { last = now; window.__pct.push([performance.now(), bar ? (v === null ? -1 : Number(v)) : null]); }
  };
  addEventListener('DOMContentLoaded', () => new MutationObserver(probe).observe(document.body, { subtree: true, childList: true, attributes: true }));
})()`;

type Msg = [number, number, string, string, string];
const msgs = async (s: S) => JSON.parse((await s.evaluate(`JSON.stringify(window.__msgs)`)) as string) as Msg[];
const pcts = async (s: S) => JSON.parse((await s.evaluate(`JSON.stringify(window.__pct)`)) as string) as [number, number | null][];
const now = async (s: S) => (await s.evaluate(`performance.now()`)) as number;
const slotOf = (s: S) => s.evaluate(`document.querySelector('${BAR} [data-trace-slot]')?.getAttribute('data-trace-slot') ?? null`) as Promise<string | null>;
const textLinesSet = async (s: S) => /(?:(\d+) of )?(\d+) text lines? set/.exec(await s.statusText());

/** Opens the Trace bar with Recognise text as stored and waits for its first preview to rest. */
async function openTraceBar(s: S) {
  await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace')?.click()`);
  await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('${BAR}')`)) ? true : null), 20_000);
}

async function clickRecogniseText(s: S) {
  await openAdvanced(s);
  await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]').click()`);
}

async function stop(s: S) {
  await s.evaluate(`document.querySelector('${BAR} [aria-label="Stop (Esc)"]').click()`);
  await waitFor("Stopped", async () => ((await slotOf(s)) === "stopped" ? true : null), 20_000);
}

async function retry(s: S) {
  await s.evaluate(`[...document.querySelectorAll('${BAR} button')].find((b) => b.textContent?.trim() === 'Retry').click()`);
}

/** Waits for a run with text to settle: the slot rests and the status names the set lines. */
async function settled(s: S, ms = 240_000) {
  await waitFor("the settled preview", async () => ((await restingSlot(s)) && (await textLinesSet(s)) ? true : null), ms).catch(async (e: unknown) => {
    throw new Error(`${String(e)}: slot ${await slotOf(s)}, status ${await s.statusText()}`);
  });
}

test("0091 tester r2 c25 c19 pool: one progress and one trace per run, a pool of min(cores - 1, 4), stops while reading and while fitting", { skip: !BROWSER, timeout: 900_000 }, async () => {
  await withApp(async (s) => {
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: SPY });
    await importImage(s, SCREENSHOT);
    const rows = await s.layerRows();
    await openTraceBar(s);
    await waitFor("the plain preview", async () => ((await restingSlot(s)) ? true : null), 120_000);
    const cores = (await s.evaluate(`navigator.hardwareConcurrency`)) as number;
    const expected = Math.max(1, Math.min(4, cores - 1));

    // A stop while the pool reads.
    let mark = (await msgs(s)).length;
    await clickRecogniseText(s);
    await waitFor("the pool reading", async () => ((await msgs(s)).slice(mark).some((m) => m[2] === "out" && m[3] === "ocr-read") ? true : null), 120_000);
    await sleep(300);
    let since = (await msgs(s)).slice(mark);
    const readers = [...new Set(since.filter((m) => m[2] === "out" && m[3] === "ocr-read").map((m) => m[1]))];
    assert.ok(!since.some((m) => m[2] === "in" && m[3] === "ocr-read"), "no share was read yet: the stop falls in the reading");
    await stop(s);
    since = (await msgs(s)).slice(mark);
    console.log(`# cores ${cores}, pool readers ${readers.length} (workers ${JSON.stringify(readers)}); after the stop: ${JSON.stringify(since.filter((m) => m[2] === "end").map((m) => m[1]))} ended`);
    assert.equal(readers.length, expected, "the pool reads with min(cores - 1, 4) workers");
    assert.ok(readers.every((w) => since.some((m) => m[2] === "end" && m[1] === w)), "every reader is ended by the stop");
    await sleep(1500);
    assert.deepEqual(await s.layerRows(), rows, "no document change after the stop while reading");
    assert.equal(await textLinesSet(s), null, "no text lines after the stop while reading");

    // Retry, and a stop while the pool fits.
    mark = (await msgs(s)).length;
    await retry(s);
    await waitFor("the pool fitting", async () => ((await msgs(s)).slice(mark).some((m) => m[2] === "out" && m[3] === "fit") ? true : null), 180_000);
    await sleep(300);
    since = (await msgs(s)).slice(mark);
    const fitters = [...new Set(since.filter((m) => m[2] === "out" && m[3] === "fit").map((m) => m[1]))];
    assert.ok(!since.some((m) => m[2] === "out" && m[3] === "fit-finish"), "the fit is not finished: the stop falls in the fit");
    await stop(s);
    console.log(`# pool fitters ${fitters.length}`);
    assert.equal(fitters.length, expected, "the pool fits with min(cores - 1, 4) workers");
    await sleep(1500);
    assert.deepEqual(await s.layerRows(), rows, "no document change after the stop while fitting");
    assert.equal(await textLinesSet(s), null, "no text lines after the stop while fitting");

    // Retry to the end: one percent that never runs backwards, and one main trace with the lines.
    mark = (await msgs(s)).length;
    const pctMark = (await pcts(s)).length;
    const t0 = await now(s);
    await retry(s);
    await settled(s);
    const t1 = await now(s);
    since = (await msgs(s)).slice(mark);
    const run = (await pcts(s)).slice(pctMark).filter((p) => p[1] !== null) as [number, number][];
    const values = run.map((p) => p[1]).filter((v) => v >= 0);
    const traces = since.filter((m) => m[2] === "out" && m[3] === "trace");
    console.log(`# retry run ${Math.round(t1 - t0)} ms; percents ${JSON.stringify(values)}; traces ${JSON.stringify(traces.map((m) => m[4] || "plain"))}; requests ${JSON.stringify(since.filter((m) => m[2] === "out").map((m) => m[3]))}`);
    assert.ok(values.length >= 3, "the percent is shown");
    assert.ok(values.every((v, i) => i === 0 || v >= values[i - 1]), `never backwards: ${JSON.stringify(values)}`);
    // The bar starts low and covers most of the job; the trace's share can pass between two
    // frames, so the last value shown before the result may be well under 100 (reported).
    assert.ok(values[0] <= 20 && values.at(-1)! >= 50, `0 to 100 over the job: ${JSON.stringify(values)}`);
    assert.equal(traces.filter((m) => m[4] === "lines").length, 1, "one main trace with the decided lines");
    assert.ok(traces.length <= 2, `at most the reduced preview and the main trace: ${traces.length}`);

    // The screenshot's texts in the browser after Create paths.
    const status = await s.statusText();
    const t2 = await now(s);
    await createPaths(s);
    const after = await waitFor("the created texts", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Trace of")) ? r : null;
    }, 180_000);
    const t3 = await now(s);
    await sleep(800);
    const textRows = after.filter((x) => !rows.includes(x) && !/Trace of/.test(x ?? ""));
    console.log(`# screenshot: ${status}; Create paths ${Math.round(t3 - t2)} ms; ${textRows.length} new rows: ${JSON.stringify(textRows)}`);
    const notices = await s.statusText();
    console.log(`# screenshot notices after Create paths: ${JSON.stringify(notices)}`);
    const m = /(?:(\d+) of )?(\d+) text lines? set/.exec(status);
    assert.ok(m);
    // The layer list draws only the rows in view; the selection counts the written objects.
    const selected = /(\d+) objects selected/.exec(notices);
    assert.equal(Number(selected?.[1]), Number(m[1] ?? m[2]) + 1, "every set line is a text, beside the group");
  });
});

/** The whole job on `file` with Recognise text stored on: Trace click to the settled preview,
 * Create paths click to the written result, the long tasks over both. */
async function timeJob(file: string) {
  let out = "";
  await withApp(async (s) => {
    await s.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `try { localStorage.setItem('curvyo.trace-values', JSON.stringify({ recogniseTextOn: true })); } catch {}`,
    });
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: SPY });
    await importImage(s, file);
    const t0 = await now(s);
    await openTraceBar(s);
    await settled(s);
    const t1 = await now(s);
    // The longest time the shown percent stays the same while the job runs, and at what value.
    const shown = (await pcts(s)).filter((p) => p[0] >= t0 && p[0] <= t1);
    let flat = [0, -1];
    shown.forEach((p, i) => {
      const end = shown[i + 1]?.[0] ?? t1;
      if (p[1] !== null && end - p[0] > flat[0]) {
        flat = [end - p[0], p[1]];
      }
    });
    const values = shown.map((p) => p[1]).filter((v): v is number => v !== null && v >= 0);
    assert.ok(values.every((v, i) => i === 0 || v >= values[i - 1]), `never backwards: ${JSON.stringify(values)}`);
    await createPaths(s);
    await waitFor("the written result", async () => ((await s.layerRows()).some((x) => x?.includes("Trace of")) ? true : null), 180_000);
    const t2 = await now(s);
    const long = (await longTasks(s)).filter((l) => l[0] >= t0 && l[0] <= t2);
    const total = long.reduce((a, l) => a + l[1], 0);
    const longest = long.reduce((a, l) => Math.max(a, l[1]), 0);
    out = `${path.basename(file)}: Trace click to settled preview ${Math.round(t1 - t0)} ms, Create paths ${Math.round(t2 - t1)} ms; long tasks ${long.length}, longest ${Math.round(longest)} ms, total ${Math.round(total)} ms; longest flat percent ${(flat[0] / 1000).toFixed(1)} s at ${flat[1]} %, last shown ${values.at(-1)} %; ${await s.statusText()}`;
  });
  return out;
}

test("0091 tester r2 times: the whole job on the screenshot and on the poster, and the main thread's long tasks", { skip: !BROWSER, timeout: 900_000 }, async () => {
  for (const file of [SCREENSHOT, POSTER]) {
    console.log(`# ${await timeJob(file)}`);
  }
});

// ------------------------------------------------------------------ slice 2: Multicolour (criteria 17 as amended, 23, 24)

const COLOUR_POSTER = path.join(FIXTURES, "colour_poster.png");
const NATIVE_S2 = path.resolve(ROOT, "curvyo-editor-wasm", "tests", "fixtures", "acceptance_0091_slice2_tester_native.json");

test("0091 tester slice 2 c24 with c13 across builds: the editor wasm reads, fits and traces the colour poster in Multicolour as natively", { skip: !BROWSER, timeout: 400_000 }, async () => {
  const native = JSON.parse(readFileSync(NATIVE_S2, "utf8")) as {
    width: number; height: number; rgba_fnv32: number; settings: string; map: number[]; ocr: string; fit_request: string; fit_reply: string; decided: string; wire: number[];
  };
  await withApp(async (s) => {
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    const png = readFileSync(COLOUR_POSTER).toString("base64");
    const out = JSON.parse(
      (await s.evaluate(`(async () => {
        const m = await import('/src/wasm-bindings/curvyo_editor_wasm.js?own-instance');
        await m.default();
        const png = Uint8Array.from(atob(${JSON.stringify(png)}), (c) => c.charCodeAt(0));
        const bitmap = await createImageBitmap(new Blob([png], { type: 'image/png' }), { colorSpaceConversion: 'none', premultiplyAlpha: 'none' });
        const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
        const ctx = canvas.getContext('2d', { colorSpace: 'srgb' });
        ctx.drawImage(bitmap, 0, 0);
        const rgba = new Uint8Array(ctx.getImageData(0, 0, bitmap.width, bitmap.height).data.buffer);
        let h = 0x811c9dc5;
        for (let i = 0; i < rgba.length; i++) { h = Math.imul(h ^ rgba[i], 0x01000193) >>> 0; }
        const get = async (u) => new Uint8Array(await (await fetch(u)).arrayBuffer());
        const sink = { report() {} };
        const loaded = m.ocr_load(await get('/models/paddleocr-latin/det.onnx'), await get('/models/paddleocr-latin/rec.onnx'), new TextDecoder().decode(await get('/models/paddleocr-latin/dict.txt')));
        const ocr = m.ocr_job(rgba, bitmap.width, bitmap.height, sink);
        let reply = m.fit_start(${JSON.stringify(native.fit_request)}, sink);
        for (let i = 0; i < 64; i++) {
          const v = JSON.parse(reply);
          if (v.kind === 'finished') break;
          for (const [file, index] of v.faces) { m.fit_add_face(file, index, await get('/fonts/' + file)); }
          reply = m.fit_step(sink);
        }
        const wire = Array.from(m.trace_text_job(rgba, bitmap.width, bitmap.height, Float64Array.from(${JSON.stringify(native.map)}), ${JSON.stringify(native.settings)}, ${JSON.stringify(native.decided)}, sink));
        return JSON.stringify({ w: bitmap.width, h: bitmap.height, fnv: h, loaded, ocr, reply, wire });
      })()`)) as string,
    ) as { w: number; h: number; fnv: number; loaded: string; ocr: string; reply: string; wire: number[] };
    assert.deepEqual([out.w, out.h, out.fnv], [native.width, native.height, native.rgba_fnv32], "the same pixels");
    assert.equal(out.loaded, "");
    const ocrDiff = compare(JSON.parse(out.ocr), JSON.parse(native.ocr), 1e-3);
    console.log(
      `# slice 2 wasm vs native: OCR ${out.ocr === native.ocr ? "identical" : `max diff ${ocrDiff}`}, fit ${out.reply === native.fit_reply ? "identical" : "differs"}, multicolour wire ${JSON.stringify(out.wire) === JSON.stringify(native.wire) ? "identical" : "differs"}`,
    );
    // ADR 0028 §6: the trace and the fit from the same lines are bit-identical across builds.
    assert.equal(out.reply, native.fit_reply, "the fit");
    assert.deepEqual(out.wire, native.wire, "the multicolour wire with its text section");
  });
});

test("0091 tester slice 2 c17 c23 c24 (c6 c11): the colour poster in Multicolour end to end", { skip: !BROWSER, timeout: 600_000 }, async () => {
  await withApp(async (s) => {
    await importImage(s, COLOUR_POSTER);
    await openTrace(s);
    const output = (name: string) => s.evaluate(`[...document.querySelectorAll('${BAR} [role=radio], ${BAR} button')].find((b) => b.textContent?.trim() === ${JSON.stringify(name)})?.click()`);
    await waitFor("the single-colour text lines", async () => ((await s.statusText()).includes("text lines set") && (await restingSlot(s)) ? true : null), 180_000);
    await output("Multicolour");
    await sleep(800);
    assert.equal(await s.evaluate(`document.querySelector('${BAR} [aria-label="Recognise text"]')?.getAttribute('aria-pressed') ?? null`), "true", "criterion 17 as amended: drawn and on");
    await waitFor("the multicolour preview", async () => {
      const swatches = (await s.evaluate(`document.querySelectorAll('${BAR} [data-trace-palette-swatch]').length`)) as number;
      return swatches > 0 && (await restingSlot(s)) && (await s.statusText()).includes("text lines set") ? true : null;
    }, 180_000);
    await sleep(1500);
    const swatches = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('${BAR} [data-trace-palette-swatch]')].map((e) => e.getAttribute('aria-label')))`)) as string) as string[];
    const barText = (await s.evaluate(`document.querySelector('${BAR}').innerText`)) as string;
    console.log("# Multicolour with text: swatches", JSON.stringify(swatches));
    console.log("# Multicolour with text: bar text", JSON.stringify(barText));
    // Criterion 23 as amended (lead 2026-11-02): the count line counts the colours only texts use
    // too, so it equals the strip (before the amendment: "2 colours" beside 4 swatches).
    const counted = /(\d+) colours?,/.exec(barText);
    assert.ok(counted, barText);
    assert.equal(Number(counted[1]), swatches.length, `the count line equals the strip: ${JSON.stringify(swatches)}`);
    assert.equal(swatches.length, 4, "blue, red and the text-only black and green");
    // No hint text (design-system, ux-notes §9): nothing like a tip in the bar.
    assert.ok(!/\b(tip|hint|try|click to|drag to)\b/i.test(barText), `no hint text: ${barText}`);
    const rowsBefore = await s.layerRows();
    await createPaths(s);
    const rows = await waitFor("the text rows", async () => {
      const r = await s.layerRows();
      return r.some((x) => x?.includes("Trace of")) && r.some((x) => x?.includes("Straße")) ? r : null;
    }, 180_000);
    console.log("# layer rows after Create paths (Multicolour):", JSON.stringify(rows));
    const group = rows.findIndex((x) => x?.includes("Trace of"));
    const texts = rows.map((x, i) => [x, i] as const).filter(([x]) => /Größe|Übermaß|Äpfel|Straße|Süße|Brücke/.test(x ?? ""));
    assert.ok(texts.length >= 5, JSON.stringify(rows));
    assert.ok(group >= 0 && texts.every(([, i]) => i < group), `texts above the group (criterion 6): ${JSON.stringify(rows)}`);
    await sleep(800);
    console.log("# notices after Create paths (Multicolour):", JSON.stringify(await s.statusText()));
    // Criterion 11: one undo, the document as before.
    await s.evaluate(`document.activeElement?.blur?.()`);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => (JSON.stringify(await s.layerRows()) === JSON.stringify(rowsBefore) ? true : null), 20_000).catch(async (e: unknown) => {
      throw new Error(`${String(e)}: ${JSON.stringify(await s.layerRows())} vs ${JSON.stringify(rowsBefore)}`);
    });
  });
});

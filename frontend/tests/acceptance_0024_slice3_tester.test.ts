// Tester cases for `specs/0024-svg-import-export` slice 3 (Inkscape shapes) in the real app: with
// CURVYO_E2E_BROWSER=/path/to/chromium and the wasm bindings built, Chromium imports the real
// Inkscape 1.4.4 save `curvyo-ui-core/tests/fixtures/svg/inkscape_shapes.svg` through the Import
// button: arcs come in as ellipses named "Arc" (a label wins: "My pie"), the polygons and stars as
// their kinds, the rx != ry rectangle and the rounded, randomized, twisted and stretched shapes as
// paths, the two spirals as paths named "Spiral" (criteria 112 to 121), the 3D box as a group "3D
// box" of six faces "Face 1" to "Face 6" with the labelled "Lid" (120, 121); one history step that
// one Ctrl+Z removes, and no hint text is added. Written from the spec; selectors come from the
// rendered page. Harness as in
// `acceptance_0024_slice2_tester.test.ts`; the browser profile lives under `target/`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5198;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0024s3");
const FIXTURE = path.resolve(FRONTEND, "..", "curvyo-ui-core", "tests", "fixtures", "svg", "inkscape_shapes.svg");
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



test("0024 slice 3 in Chromium: real Inkscape shapes as primitives, a box group, one undo step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "inkscape_shapes.svg");
  writeFileSync(file, readFileSync(FIXTURE, "utf8"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    const textBefore = (await s.evaluate(`document.body.innerText`)) as string;
    await s.tab("History");
    const stepsBefore = (await s.evaluate(`document.body.innerText.split('Import SVG').length`)) as number;
    await s.tab("Layers");
    await s.choose("[data-import-button]", file);
    await waitFor("the import layer", async () => ((await s.layerRows()).some((x) => /inkscape_shapes/.test(x ?? "")) ? true : null), 30_000);
    for (let i = 0; i < 5; i += 1) {
      await s.evaluate(`[...document.querySelectorAll('[role=row][aria-expanded=false]')].forEach((r) => r.querySelector('button')?.click())`);
      await sleep(300);
    }
    const rows = await s.layerRows();
    const count = (re: RegExp) => rows.filter((r) => re.test(r ?? "")).length;
    // The Layers tab lists the top of the z-order first.
    const box = rows.indexOf("3D box, group");
    assert.ok(box >= 0, `a group "3D box": ${rows.join(" / ")}`);
    assert.deepEqual(rows.slice(box + 1, box + 7), ["Face 6, path", "Face 5, path", "Face 4, path", "Lid, path", "Face 2, path", "Face 1, path"]);
    assert.equal(count(/^Arc, ellipse$/), 9, "nine arcs named Arc");
    assert.equal(count(/^My pie, ellipse$/), 1, "the label wins");
    assert.equal(count(/^Ellipse \S+, ellipse$/), 2, "two whole ellipses with the default name");
    assert.equal(count(/, polygon$/), 4, "four polygons");
    assert.equal(count(/, star$/), 4, "four stars");
    assert.equal(count(/^Spiral, path$/), 2, "two spirals");
    // Paths: rx != ry rectangle, rounded, randomized and twisted star, skewed arc, stretched polygon, six faces, two spirals.
    assert.equal(count(/, path$/), 14, `paths: ${rows.join(" / ")}`);
    assert.equal(await s.dialogText(), "", "no dialog");
    await s.tab("History");
    const stepsAfter = (await s.evaluate(`document.body.innerText.split('Import SVG').length`)) as number;
    assert.equal(stepsAfter, stepsBefore + 1, "one history step Import SVG");
    await s.tab("Layers");
    await s.evaluate(`[...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0].closest('[tabindex]').focus()`);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => ((await s.layerRows()).some((r) => /inkscape_shapes/.test(r ?? "")) ? null : true), 10_000);
    const textAfter = (await s.evaluate(`document.body.innerText`)) as string;
    const added = textAfter.split("\n").filter((l) => l.trim() && !textBefore.includes(l.trim()));
    const hints = added.filter((l) => /\b(tip|hint|press|click|drag|try)\b/i.test(l));
    assert.deepEqual(hints, [], `no hint text: ${hints}`);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

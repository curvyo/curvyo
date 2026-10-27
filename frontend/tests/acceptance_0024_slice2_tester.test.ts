// Tester cases for `specs/0024-svg-import-export` slice 2 (text and images) in the real app:
// with CURVYO_E2E_BROWSER=/path/to/chromium and the wasm bindings built, Chromium imports a real
// Inkscape 1.4.4 file (`curvyo-svg-core/tests/fixtures/svg_real/inkscape_text/text_ink.svg` with
// the embedded PNG of `images_ink.svg` added) through the Import button: the texts arrive live
// (Enter on a selected text starts edit mode in its face, criterion 78), the image appears (100),
// only the text in "Nonexistent Sans" shows the "Font missing" tag (82), one history step "Import
// SVG" that one Ctrl+Z removes (104, 106), no network request (67), and no hint text is added to
// the page. Written from the spec; selectors come from the rendered page. Harness as in
// `acceptance_0058_tester.test.ts`; the browser profile lives under `target/`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5199;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0024s2");
const INK = path.resolve(FRONTEND, "..", "curvyo-svg-core", "tests", "fixtures", "svg_real", "inkscape_text");
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



test("0024 slice 2 in Chromium: an Inkscape file with live text and an image, one undo step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const file = path.join(SCRATCH, "ink_text_image.svg");
  const images = readFileSync(path.join(INK, "images_ink.svg"), "utf8");
  const pngImage = images.slice(images.indexOf("<image"), images.indexOf("/>", images.indexOf("<image")) + 2);
  assert.match(pngImage, /img_png/);
  writeFileSync(file, readFileSync(path.join(INK, "text_ink.svg"), "utf8").replace("</svg>", `${pngImage}</svg>`));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
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
    const network = s.requests.length;
    await s.choose("[data-import-button]", file);
    const rows = await waitFor("the import layer", async () => {
      const r = await s.layerRows();
      return r.some((x) => /ink_text_image/.test(x ?? "")) ? r : null;
    }, 30_000);
    assert.equal(await s.dialogText(), "", "no dialog");
    assert.ok(!/ink_text_image/.test(await s.statusText()), "no notice after a successful import (criterion 8)");
    // Expand every collapsed row so the texts and the image are listed.
    for (let i = 0; i < 4; i += 1) {
      await s.evaluate(`[...document.querySelectorAll('[role=row][aria-expanded=false]')].forEach((r) => r.querySelector('button')?.click())`);
      await sleep(300);
    }
    const all = await s.layerRows();
    for (const name of ["t_one", "t_multi", "t_missing", "tp_circle", "img_png"]) {
      assert.ok(all.some((r) => (r ?? "").includes(name)), `${name} in the Layers tab: ${all.slice(0, 60).join(" / ")} (first rows ${rows.slice(0, 5)})`);
    }
    // Criterion 82: one "Font missing" tag, for the text in Nonexistent Sans only.
    const tags = (await s.evaluate(`[...document.querySelectorAll('div')].filter((d) => d.textContent?.trim() === 'Font missing' && d.children.length === 1).length`)) as number;
    assert.equal(tags, 1, "only the missing font is tagged");
    // Criterion 106: one step "Import SVG".
    await s.tab("History");
    const stepsAfter = (await s.evaluate(`document.body.innerText.split('Import SVG').length`)) as number;
    assert.equal(stepsAfter, stepsBefore + 1, "one history step Import SVG");
    // Criterion 78: the text is live: select its row, Enter on the canvas starts edit mode in its face.
    await s.tab("Layers");
    await s.evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => (r.getAttribute('aria-label') || '').includes('t_one')).click()`);
    await sleep(400);
    await s.evaluate(`[...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0].closest('[tabindex]').focus()`);
    await s.key("Enter", 13);
    const editing = await waitFor("edit mode", async () => {
      const a = (await s.evaluate(`document.activeElement?.getAttribute('aria-label') ?? ''`)) as string;
      return a === "Text" ? a : null;
    }, 5_000);
    assert.equal(editing, "Text");
    const bar = (await s.evaluate(`[...document.querySelectorAll('[role=toolbar][aria-label=Text], [role=toolbar][aria-label=Text] *')].map((e) => [e.getAttribute('aria-label'), e.getAttribute('title'), e.children.length ? '' : e.textContent].join(' ')).join(' | ')`)) as string;
    assert.match(bar, /Source Sans 3/, `the Text bar names the face: ${bar}`);
    await s.key("Escape", 27);
    // Criterion 104/106: one Ctrl+Z removes the whole import.
    await s.evaluate(`[...document.querySelectorAll('canvas')].sort((a, b) => b.clientWidth * b.clientHeight - a.clientWidth * a.clientHeight)[0].closest('[tabindex]').focus()`);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => ((await s.layerRows()).some((r) => /ink_text_image/.test(r ?? "")) ? null : true), 10_000);
    const fetched = s.requests.slice(network).filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:"));
    assert.deepEqual(fetched, [], "criterion 67: nothing fetched");
    // Project rule: no operating hint text appeared in the page.
    const textAfter = (await s.evaluate(`document.body.innerText`)) as string;
    const added = textAfter.split("\n").filter((l) => l.trim() && !textBefore.includes(l.trim()));
    const hints = added.filter((l) => /\b(tip|hint|press|click|drag|try)\b/i.test(l));
    assert.deepEqual(hints, [], `no hint text: ${hints}`);
  } finally {
    await closeAll();
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

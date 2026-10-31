// `specs/0087-image-clipboard-and-drop` slice 1 in the real app (Chromium via DevTools; set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): a synthetic paste event
// with a PNG file imports "Pasted image" as one step "Paste image" (criteria 1, 2, 10); text only
// says there is nothing to paste (criteria 4, 5); a text field keeps its paste (criterion 3);
// Ctrl+C puts the stored pixels on the clipboard as image/png, read back and decoded
// (criteria 11, 12); Ctrl+V from that clipboard pastes it again; Ctrl+C with no image selected
// does nothing (criterion 13). Slice 2: synthetic drops of a named image file, of nameless image
// data and of a link only (no request), a mixed drop, and fonts dropped on the Fonts tab
// (criteria 20 to 23, `0075` criterion 24). Harness as in `acceptance_0025_browser.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5287;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0087");
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
  // The clipboard API needs a focused page and the permission a click on "Allow" would give.
  await send("Emulation.setFocusEmulationEnabled", { enabled: true });
  await send("Browser.grantPermissions", { origin: ORIGIN, permissions: ["clipboardReadWrite", "clipboardSanitizedWrite"] });
  const chooserCount = () => choosers.length;
  return { send, evaluate, click, key, clickSel, choose, tab, statusText, dialogText, layerRows, close, requests, chooserCount, choosers };
}


/** A `width` x `height` RGBA PNG of `pixel`, without a density chunk. */
function png(width: number, height: number, pixel: (x: number, y: number) => [number, number, number, number]): Buffer {
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
  ihdr[9] = 6;
  const rows = Array.from({ length: height }, (_, y) =>
    Buffer.from([0, ...Array.from({ length: width }, (_, x) => pixel(x, y)).flat()]),
  );
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(Buffer.concat(rows))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** 40 x 20 px, a red 5 x 5 marker top-left, white elsewhere. */
const MARKER = png(40, 20, (x, y) => (x < 5 && y < 5 ? [255, 0, 0, 255] : [255, 255, 255, 255]));

/** Dispatches a paste event on the focused element with `files` (base64 PNGs) and `text`. */
const pasteEvent = (files: string[], text: string) => `(() => {
  const dt = new DataTransfer();
  for (const b64 of ${JSON.stringify(files)}) {
    const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    dt.items.add(new File([bytes], "image.png", { type: "image/png" }));
  }
  if (${JSON.stringify(text)}) dt.setData("text/plain", ${JSON.stringify(text)});
  const target = document.activeElement || document.body;
  return target.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
})()`;

const focusCanvas = `(() => { document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus(); return true; })()`;

test("0087 in Chromium: paste event, nothing to paste, text field, Ctrl+C read back, Ctrl+V", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
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
    const pasted = async () => (await s.layerRows()).filter((r) => r?.includes("Pasted image")).length;

    // Criteria 1, 2, 10: the pointer over the canvas, a paste event with a PNG file and text.
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 420 });
    await s.evaluate(focusCanvas);
    assert.equal(await s.evaluate(pasteEvent([MARKER.toString("base64")], "some text")), false, "the paste is taken");
    await waitFor("the pasted image row", async () => ((await pasted()) === 1 ? true : null));
    const toolBar = (await s.evaluate(`document.querySelector('[data-context-bar]')?.textContent ?? ''`)) as string;
    assert.ok(toolBar.length > 0, "the Select bar shows the selection");

    // Criterion 1: one step "Paste image"; Ctrl+Z removes it, Ctrl+Shift+Z brings it back.
    await s.tab("History");
    const history = (await s.evaluate(`document.body.innerText`)) as string;
    assert.ok(history.includes("Paste image"), "the History names the step");
    await s.evaluate(focusCanvas);
    await s.key("z", 90, 2);
    await s.tab("Layers");
    await waitFor("undo", async () => ((await pasted()) === 0 ? true : null));
    await s.evaluate(focusCanvas);
    await s.key("z", 90, 2 | 8);
    await waitFor("redo", async () => ((await pasted()) === 1 ? true : null));

    // Criteria 4, 5: text only is nothing to paste; nothing changes.
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEvent([], "only text"));
    const nothing = await waitFor("the notice", async () => {
      const t = await s.statusText();
      return t.includes("Nothing to paste") ? t : null;
    });
    assert.ok(nothing.includes("Nothing to paste: the clipboard has no image."), nothing);
    assert.equal(await pasted(), 1);

    // Criterion 3: a text field keeps its paste.
    await s.tab("Document");
    const field = (await s.evaluate(`(() => { const i = [...document.querySelectorAll('input')].find((e) => e.offsetParent && !e.disabled && (e.type === 'text' || e.type === '')); if (!i) return false; i.focus(); return document.activeElement === i; })()`)) as boolean;
    assert.ok(field, "a text field has focus");
    assert.equal(await s.evaluate(pasteEvent([MARKER.toString("base64")], "")), true, "the field's paste is not taken");
    await sleep(800);
    await s.tab("Layers");
    assert.equal(await pasted(), 1);

    // Criteria 11, 12: Ctrl+C with the image selected writes image/png; read back, it is the
    // 40 x 20 px picture with its red marker top-left.
    await s.clickSel('[role=row][aria-label*="Pasted image"]');
    await s.evaluate(focusCanvas);
    await s.evaluate(`(() => {
      const log = (window.__clipLog = []);
      const t0 = performance.now();
      const at = () => Math.round(performance.now() - t0);
      const write = navigator.clipboard.write.bind(navigator.clipboard);
      navigator.clipboard.write = (items) => {
        log.push('write ' + at());
        const p = write(items);
        p.then(() => log.push('resolved ' + at()), (e) => log.push('rejected ' + at() + ' ' + e));
        return p;
      };
      new MutationObserver((records) => {
        for (const r of records) {
          const added = [...r.addedNodes].map((n) => n.textContent).join('/');
          const removed = [...r.removedNodes].map((n) => n.textContent).join('/');
          if (/cop|Cop|Nothing/.test(added + removed)) log.push('dom ' + at() + ' +' + added.slice(0, 40) + ' -' + removed.slice(0, 40));
        }
      })
        .observe(document.body, { subtree: true, childList: true, characterData: true });
      return true;
    })()`);
    await s.key("c", 67, 2);
    const copied = await waitFor("the copy notice", async () => {
      const t = await s.statusText();
      return t.includes("Image copied.") || t.includes("Could not copy") ? t : null;
    }, 15_000).catch(async (error: Error) => {
      const bar = await s.evaluate(`document.querySelector('[data-context-bar]')?.textContent ?? ''`);
      const rows = await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')))`);
      const clip = await s.evaluate(`navigator.clipboard.read().then((i) => JSON.stringify(i.map((x) => x.types)), (e) => String(e))`);
      const clipLog = await s.evaluate(`JSON.stringify(window.__clipLog.slice(0, 30))`);
      throw new Error(`${error.message}; log ${clipLog}; bar ${bar}; selected ${rows}; clipboard ${clip}; focus ${await s.evaluate("document.hasFocus()")}`);
    });
    assert.ok(copied.includes("Image copied."), copied);
    const readBack = JSON.parse((await s.evaluate(`(async () => {
      const items = await navigator.clipboard.read();
      const item = items.find((i) => i.types.includes("image/png"));
      if (!item) return JSON.stringify({ types: items.map((i) => i.types) });
      const blob = await item.getType("image/png");
      const bitmap = await createImageBitmap(blob);
      const c = new OffscreenCanvas(bitmap.width, bitmap.height);
      const g = c.getContext("2d");
      g.drawImage(bitmap, 0, 0);
      return JSON.stringify({ w: bitmap.width, h: bitmap.height, tl: [...g.getImageData(0, 0, 1, 1).data], br: [...g.getImageData(39, 19, 1, 1).data] });
    })()`)) as string) as { w: number; h: number; tl: number[]; br: number[] };
    assert.deepEqual(readBack, { w: 40, h: 20, tl: [255, 0, 0, 255], br: [255, 255, 255, 255] });

    // Ctrl+V from the clipboard the copy wrote: a real paste command.
    await s.evaluate(focusCanvas);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "v", code: "KeyV", windowsVirtualKeyCode: 86, modifiers: 2, commands: ["paste"] });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "v", code: "KeyV", windowsVirtualKeyCode: 86, modifiers: 2 });
    await waitFor("the second pasted image", async () => ((await pasted()) === 2 ? true : null));

    // Criterion 13: with nothing selected Ctrl+C does nothing and says nothing.
    await s.evaluate(focusCanvas);
    await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 });
    await sleep(3500);
    await s.key("c", 67, 2);
    await sleep(800);
    assert.ok(!(await s.statusText()).includes("copied"), "no copy notice");
    assert.equal(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")).length, 0, "nothing left the machine");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

/** Drags `files` ([name, type, base64]) and `strings` ([type, value]) onto the element `sel` (for
 * "canvas" the drawing canvas, the largest one) at its centre: dragenter, dragover, drop. Returns how dragover was taken: "copy" by a drop target (it stops the event there), "none" by the
 * page's refusal (it reaches the window), "refused" by nobody. A synthetic event's `dropEffect`
 * cannot be read back, so the propagation tells them apart. */
const dropOn = (sel: string, files: [string, string, string][], strings: [string, string][]) => `(() => {
  const target = ${JSON.stringify(sel)} === 'canvas' ? [...document.querySelectorAll('canvas')].sort((a, b) => b.width * b.height - a.width * a.height)[0] : document.querySelector(${JSON.stringify(sel)});
  const box = target.getBoundingClientRect();
  const at = { clientX: box.x + box.width / 2, clientY: box.y + box.height / 2 };
  const dt = new DataTransfer();
  for (const [name, type, b64] of ${JSON.stringify(files)}) {
    dt.items.add(new File([Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))], name, { type }));
  }
  for (const [type, value] of ${JSON.stringify(strings)}) dt.setData(type, value);
  const fire = (type) => { const e = new DragEvent(type, { ...at, dataTransfer: dt, bubbles: true, cancelable: true }); target.dispatchEvent(e); return e; };
  fire("dragenter");
  let reached = false;
  const seen = () => { reached = true; };
  window.addEventListener("dragover", seen);
  const over = fire("dragover");
  window.removeEventListener("dragover", seen);
  const effect = over.defaultPrevented ? (reached ? "none" : "copy") : "refused";
  fire("drop");
  return effect;
})()`;

test("0087 slice 2 in Chromium: dropped image files, image data, a link, a mixed drop, fonts", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
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
    const canvas = "canvas";
    const b64 = MARKER.toString("base64");
    const rows = async (name: string) => (await s.layerRows()).filter((r) => r?.includes(name)).length;

    // Criterion 19 (as 0058 criterion 4): a named image file lands, one step.
    assert.equal(await s.evaluate(dropOn(canvas, [["logo.png", "image/png", b64]], [])), "copy");
    await waitFor("logo.png", async () => ((await rows("logo.png")) === 1 ? true : null));

    // Criterion 20: image data a browser hands over without a file name is "Dropped image".
    await s.evaluate(dropOn(canvas, [["", "image/png", b64]], [["text/html", '<img src="https://example.com/x.png">']]));
    await waitFor("Dropped image", async () => ((await rows("Dropped image")) === 1 ? true : null));

    // Criterion 21: a drop of only a link fetches nothing and says so.
    const before = s.requests.length;
    assert.equal(await s.evaluate(dropOn(canvas, [], [["text/uri-list", "https://example.com/photo.jpg"], ["text/plain", "https://example.com/photo.jpg"]])), "copy");
    const link = await waitFor("the link notice", async () => {
      const t = await s.statusText();
      return t.includes("That drop is a link") ? t : null;
    });
    assert.ok(link.includes("That drop is a link, not an image. Nothing was downloaded."), link);
    await sleep(500);
    assert.deepEqual(s.requests.slice(before).filter((u) => !u.startsWith(ORIGIN)), [], "no request left the machine");

    // Criterion 23: plain text is no drop for the canvas; a link elsewhere is refused.
    assert.equal(await s.evaluate(dropOn(canvas, [], [["text/plain", "hello"]])), "refused");
    assert.equal(await s.evaluate(dropOn("[data-import-button]", [], [["text/uri-list", "https://example.com/a.png"]])), "none");

    // Criterion 22: images and another file: the image comes in, the other is named.
    await s.evaluate(dropOn(canvas, [["b.png", "image/png", b64], ["notes.txt", "text/plain", Buffer.from("hi").toString("base64")]], []));
    await waitFor("b.png", async () => ((await rows("b.png")) === 1 ? true : null));
    const mixed = await waitFor("the mixed notice", async () => {
      const t = await s.statusText();
      return t.includes("could not be imported") ? t : null;
    });
    assert.ok(mixed.includes("1 file could not be imported."), mixed);

    // `0075` criterion 24: fonts dropped on the Fonts tab are added.
    await s.tab("Fonts");
    const font = readFileSync(path.join(FRONTEND, "public", "fonts", "great-vibes", "GreatVibes-Regular.ttf")).toString("base64");
    assert.equal(await s.evaluate(dropOn('[role=tree][aria-label="Fonts"]', [["GreatVibes-Regular.ttf", "font/ttf", font]], [])), "copy");
    const added = await waitFor("the font notice", async () => {
      const t = (await s.evaluate(`document.body.innerText`)) as string;
      return /added\.|already there\./.test(t) ? t : null;
    });
    assert.match(added, /1 font added\.|already there\./);
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

/** A desktop host for the page (`window.__TAURI_INTERNALS__`, installed before the app loads):
 * every command answers as an empty host would, `read_local_files` records the arguments it gets
 * and answers `window.__localAnswer` (the real host reads the clipboard's or the drag's list). */
const FAKE_HOST = `(() => {
  let next = 1;
  window.__localCalls = [];
  window.__localAnswer = [];
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
    transformCallback: () => next++,
    unregisterCallback: () => undefined,
    convertFileSrc: (p) => p,
    invoke: async (cmd, args) => {
      switch (cmd) {
        case "read_local_files":
          window.__localCalls.push({ keys: Object.keys(args), from: args.from });
          return window.__localAnswer;
        case "font_folder_list":
          return { faces: [], skipped: [] };
        case "system_font_list":
        case "pick_import_files":
          return [];
        case "plugin:event|listen":
          return next++;
        default:
          return null;
      }
    },
  };
})()`;

test("0087 local files in Chromium with a faked desktop host: file URIs and paths pasted and dropped", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = startVite(PORT, FRONTEND);
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.addScriptToEvaluateOnNewDocument", { source: FAKE_HOST });
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.tab("Layers");
    const bytes = [...MARKER];
    const answer = (files: object[]) => s.evaluate(`(window.__localAnswer = ${JSON.stringify(files)}, true)`);
    const calls = async () => JSON.parse((await s.evaluate(`JSON.stringify(window.__localCalls)`)) as string) as { keys: string[]; from: string }[];
    const rows = async (name: string) => (await s.layerRows()).filter((r) => r?.includes(name)).length;
    const textPaste = (strings: [string, string][]) => `(() => {
      const dt = new DataTransfer();
      for (const [type, value] of ${JSON.stringify(strings)}) dt.setData(type, value);
      return (document.activeElement || document.body).dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
    })()`;

    // Criterion 6: a file manager's copy of two files, as a uri list and GNOME's copied files,
    // no File objects: the host reads both, one step under the pointer, named by the files.
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 640, y: 420 });
    await s.evaluate(focusCanvas);
    await answer([{ kind: "bytes", name: "a b.png", bytes }, { kind: "bytes", name: "c.png", bytes }]);
    const uriList = "file:///home/maker/a%20b.png\r\nfile:///home/maker/c.png\r\n";
    const gnome = "copy\nfile:///home/maker/a%20b.png\nfile:///home/maker/c.png";
    assert.equal(await s.evaluate(textPaste([["text/uri-list", uriList], ["x-special/gnome-copied-files", gnome]])), false, "the paste is taken");
    await waitFor("both pasted files", async () => ((await rows("a b.png")) === 1 && (await rows("c.png")) === 1 ? true : null));
    assert.deepEqual((await calls())[0], { keys: ["from"], from: "paste" }, "the page names no file");

    // WebKitGTK names nothing readable: the host reads its clipboard all the same.
    await answer([{ kind: "bytes", name: "native.png", bytes }]);
    await s.evaluate(focusCanvas);
    await s.evaluate(textPaste([]));
    await waitFor("native.png", async () => ((await rows("native.png")) === 1 ? true : null));
    assert.deepEqual((await calls()).at(-1), { keys: ["from"], from: "paste" });

    // One file: one step "Paste image" with the file's name.
    await answer([{ kind: "bytes", name: "one.png", bytes }]);
    await s.evaluate(focusCanvas);
    await s.evaluate(textPaste([["text/plain", "/home/maker/one.png"]]));
    await waitFor("one.png", async () => ((await rows("one.png")) === 1 ? true : null));
    await s.tab("History");
    const history = (await s.evaluate(`document.body.innerText`)) as string;
    assert.ok(history.includes("Paste image"), history);
    await s.tab("Layers");

    // A text that names no local file: nothing to paste, nothing read.
    await answer([]);
    await s.evaluate(focusCanvas);
    await s.evaluate(textPaste([["text/plain", "https://example.com/a.png"]]));
    const nothing = await waitFor("nothing to paste", async () => {
      const t = await s.statusText();
      return t.includes("Nothing to paste") ? t : null;
    });
    assert.ok(nothing.includes("Nothing to paste: the clipboard has no image."), nothing);

    // Criterion 21 on the desktop: a drop of a file manager's uri list, no File objects.
    await answer([{ kind: "bytes", name: "d.png", bytes }]);
    assert.equal(await s.evaluate(dropOn("canvas", [], [["text/uri-list", "file:///home/maker/d.png"]])), "copy");
    await waitFor("d.png", async () => ((await rows("d.png")) === 1 ? true : null));
    assert.deepEqual((await calls()).at(-1), { keys: ["from"], from: "drop" });

    // A plain absolute path dropped as text is taken on the desktop.
    await answer([{ kind: "bytes", name: "path.png", bytes }]);
    assert.equal(await s.evaluate(dropOn("canvas", [], [["text/plain", "/home/maker/path.png"]])), "copy");
    await waitFor("path.png", async () => ((await rows("path.png")) === 1 ? true : null));

    // A file over the limit is refused with the import's notice, from its length.
    await answer([{ kind: "too_large", name: "huge.png", bytes: 140 * 1024 * 1024 }]);
    await s.evaluate(dropOn("canvas", [], [["text/uri-list", "file:///home/maker/huge.png"]]));
    const large = await waitFor("the limit notice", async () => {
      const t = await s.statusText();
      return t.includes("File too large") ? t : null;
    });
    assert.ok(large.includes("File too large: 140 MB. The limit is 100 MB."), large);

    // A remote link is still never fetched: the host names no file, the link notice shows.
    await answer([]);
    const before = s.requests.length;
    await s.evaluate(dropOn("canvas", [], [["text/uri-list", "https://example.com/photo.jpg"]]));
    const link = await waitFor("the link notice", async () => {
      const t = await s.statusText();
      return t.includes("That drop is a link") ? t : null;
    });
    assert.ok(link.includes("That drop is a link, not an image. Nothing was downloaded."), link);
    await sleep(500);
    assert.deepEqual(s.requests.slice(before).filter((u) => !u.startsWith(ORIGIN)), [], "no request left the machine");
  } finally {
    await closeAll();
    await stopVite(vite, PORT);
    removeProfile(profile);
  }
});

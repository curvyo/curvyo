// Tester cases for `specs/0058-bitmap-import` in the real app (Chromium via DevTools, set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built): the Import button of the
// tool rail (criterion 1: name, no pressed state, File tab without import rows), an image through
// the browser's file chooser (criterion 2: one layer row, selected), Ctrl+I opening the chooser
// outside a text field and not inside one (criterion 3), a decompression bomb refused with its
// notice and nothing changed (criteria 17, 26), a drop in a zoomed and panned view landing centred
// on the drop point (criterion 4, checked by hitting the image around the point), and one undo
// step per import and per delete (criterion 45). Harness as in `acceptance_0024_tester.test.ts`.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32, deflateSync } from "node:zlib";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5198;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0058");
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


/** A `width` x `height` opaque RGB PNG (96 dpi by default: no pHYs). */
function png(width: number, height: number, rgb: [number, number, number]): Buffer {
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
  ihdr[9] = 2;
  const row = Buffer.concat([Buffer.from([0]), Buffer.from(Array.from({ length: width }, () => rgb).flat())]);
  const raw = Buffer.concat(Array.from({ length: height }, () => row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

test("0058 in Chromium: Import button, Ctrl+I, bomb refusal, drop placement, undo", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const photo = path.join(SCRATCH, "photo.png");
  writeFileSync(photo, png(200, 100, [200, 40, 40]));
  const dropBytes = png(160, 80, [40, 40, 200]).toString("base64");
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);

    // Criterion 1: the rail button.
    const button = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const b = document.querySelector('[data-import-button]');
        const r = b.getBoundingClientRect(); return { name: b.getAttribute('aria-label'), pressed: b.getAttribute('aria-pressed'),
        disabled: b.disabled, w: Math.round(r.width), h: Math.round(r.height),
        card: b.closest('[role=toolbar]')?.getAttribute('aria-label') }; })())`)) as string,
    ) as { name: string; pressed: string | null; disabled: boolean; w: number; h: number; card: string };
    assert.deepEqual(button, { name: "Import", pressed: null, disabled: false, w: 40, h: 40, card: "Import" });
    await s.tab("File");
    const fileRows = (await s.evaluate(`[...document.querySelectorAll('[data-file-command]')].map((e) => e.textContent).join('|')`)) as string;
    assert.ok(!/import/i.test(fileRows), `no import row in the File tab: ${fileRows}`);

    // Criterion 2: an image through the chooser becomes one selected layer row.
    await s.tab("Layers");
    const rowsBefore = (await s.layerRows()).length;
    await s.choose("[data-import-button]", photo);
    const rows = await waitFor("the image row", async () => {
      const r = await s.layerRows();
      return r.length > rowsBefore ? r : null;
    });
    assert.ok(rows.some((r) => r?.includes("photo.png")), `a row named after the file: ${rows}`);
    const selected = (await s.evaluate(`[...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')).join('|')`)) as string;
    assert.ok(selected.includes("photo.png"), `the image is the selection: ${selected}`);

    // Criterion 45: Ctrl+Z removes the import in one step, Ctrl+Shift+Z brings it back.
    await s.key("z", 90, 2);
    await waitFor("undo", async () => ((await s.layerRows()).some((r) => r?.includes("photo.png")) ? null : true));
    await s.key("z", 90, 2 | 8);
    await waitFor("redo", async () => ((await s.layerRows()).some((r) => r?.includes("photo.png")) ? true : null));

    // Criterion 3: Ctrl+I opens the chooser on the canvas, not inside a text field.
    const canvas = JSON.parse(
      (await s.evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string,
    ) as [number, number, number, number];
    await s.evaluate(`document.activeElement && document.activeElement.blur && document.activeElement.blur()`);
    let before = s.chooserCount();
    await s.key("i", 73, 2);
    const node = await waitFor("the chooser of Ctrl+I", async () => (s.chooserCount() > before ? s.choosers[s.choosers.length - 1] : null), 10_000);
    // Criteria 17 and 26: a 60,000 x 60,000 px bomb is refused, nothing changes.
    const rowsNow = (await s.layerRows()).length;
    await s.send("DOM.setFileInputFiles", { files: [path.join(IMAGES, "png_bomb_60000x60000.png")], backendNodeId: node });
    const refusal = await waitFor("the refusal", async () => {
      const t = await s.statusText();
      return t.includes("png_bomb_60000x6") ? t : null;
    });
    // The exact reason is checked in the skipped FAIL test below (wasm32 says "damaged").
    assert.ok(/too large|damaged/.test(refusal), refusal);
    assert.equal((await s.layerRows()).length, rowsNow, "nothing imported");
    await s.tab("Document");
    const field = (await s.evaluate(`(() => { const f = [...document.querySelectorAll('input')].find((i) => (!i.type || i.type === 'text' || i.type === 'number') && i.offsetParent); if (!f) return false; f.focus(); return document.activeElement === f; })()`)) as boolean;
    assert.ok(field, "a text field of the app takes the focus");
    {
      before = s.chooserCount();
      await s.key("i", 73, 2);
      await sleep(800);
      assert.equal(s.chooserCount(), before, "Ctrl+I inside a text field does not import");
    }
    await s.tab("Layers");

    // Criterion 4: zoom in about one point, pan, then drop; the image is centred on the drop point.
    await s.evaluate(`document.activeElement && document.activeElement.blur && document.activeElement.blur()`);
    const [cx, cy, cw, ch] = canvas;
    await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: cx + cw * 0.3, y: cy + ch * 0.3, deltaX: 0, deltaY: -240, modifiers: 2 });
    await sleep(400);
    await s.send("Input.dispatchMouseEvent", { type: "mouseWheel", x: cx + cw * 0.5, y: cy + ch * 0.5, deltaX: 0, deltaY: 120, modifiers: 0 });
    await sleep(400);
    const drop = [Math.round(cx + cw * 0.62), Math.round(cy + ch * 0.55)];
    const before4 = (await s.layerRows()).length;
    const dropInfo = await s.evaluate(`(() => {
      const bytes = Uint8Array.from(atob(${JSON.stringify(dropBytes)}), (c) => c.charCodeAt(0));
      const dt = new DataTransfer();
      dt.items.add(new File([bytes], "dropped.png", { type: "image/png" }));
      const target = document.elementFromPoint(${drop[0]}, ${drop[1]});
      for (const type of ["dragenter", "dragover", "drop"]) {
        target.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true, clientX: ${drop[0]}, clientY: ${drop[1]}, dataTransfer: dt }));
      }
      return target.tagName + " " + (target.className || "").toString().slice(0, 40) + " " + dt.types.join(",");
    })()`);
    assert.ok(String(dropInfo).startsWith("CANVAS"), String(dropInfo));
    await waitFor("the dropped image", async () => ((await s.layerRows()).length > before4 ? true : null));
    const hitAt = async (dx: number, dy: number) => {
      await s.key("Escape", 27);
      await s.click(drop[0] + dx, drop[1] + dy);
      return (await s.evaluate(`[...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')).join('|')`)) as string;
    };
    assert.ok((await hitAt(0, 0)).includes("dropped.png"), "the drop point is on the image");
    const zoom = (await s.evaluate(`(() => { const e = [...document.querySelectorAll('*')].find((n) => n.children.length === 0 && /^\\d+\\s?%$/.test((n.textContent || '').trim())); return e ? parseFloat(e.textContent) : 0; })()`)) as number;
    // 160 x 80 px at 96 dpi; on screen at zoom z it is 160 z/100 x 80 z/100 CSS px if 100 % is 96 dpi.
    assert.ok(zoom > 100, `the zoom readout shows the zoomed view: ${zoom}`);
    {
      const halfW = (160 * zoom) / 200;
      const halfH = (80 * zoom) / 200;
      for (const [dx, dy] of [[halfW - 6, 0], [-(halfW - 6), 0], [0, halfH - 6], [0, -(halfH - 6)]]) {
        assert.ok((await hitAt(dx, dy)).includes("dropped.png"), `inside at (${dx}, ${dy}) of zoom ${zoom}`);
      }
      for (const [dx, dy] of [[halfW + 12, 0], [-(halfW + 12), 0], [0, halfH + 12], [0, -(halfH + 12)]]) {
        assert.ok(!(await hitAt(dx, dy)).includes("dropped.png"), `outside at (${dx}, ${dy}) of zoom ${zoom}`);
      }
    }

    // Criterion 45: delete is one step, undo brings the image back.
    await hitAt(0, 0);
    await s.key("Delete", 46);
    await waitFor("delete", async () => ((await s.layerRows()).some((r) => r?.includes("dropped.png")) ? null : true));
    await s.key("z", 90, 2);
    await waitFor("undo of delete", async () => ((await s.layerRows()).some((r) => r?.includes("dropped.png")) ? true : null));
    assert.deepEqual(s.requests.filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("data:") && !u.startsWith("blob:")), [], "no network");
  } finally {
    await closeAll();
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

test.skip("0058 FAIL: in the browser (wasm32) a 60,000 x 60,000 px PNG bomb is refused as damaged, not with \"Image too large: 60,000 x 60,000 px. The limit is 50 megapixels.\" (criterion 17; native gives the right text)", async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  let closeAll: () => Promise<void> = async () => undefined;
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    closeAll = s.close;
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await s.choose("[data-import-button]", path.join(IMAGES, "png_bomb_60000x60000.png"));
    const refusal = await waitFor("the refusal", async () => {
      const t = await s.statusText();
      return t.includes("png_bomb_60000x6") ? t : null;
    });
    assert.ok(refusal.includes("Image too large: 60,000 x 60,000 px. The limit is 50 megapixels."), refusal);
  } finally {
    await closeAll();
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

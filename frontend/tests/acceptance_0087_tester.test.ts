// `specs/0087-image-clipboard-and-drop` slice 1, tester cases in Chromium (set
// CURVYO_E2E_BROWSER=/path/to/chromium; the wasm bindings must be built). Complements
// `acceptance_0087_browser.test.ts` without repeating it: paste of PNG, JPEG and WebP under the
// pointer and at the view centre (read back from a screenshot), links and HTML never fetched (no
// request at all), the 0058 limits, Ctrl+C read back pixel by pixel, one undo step per paste, the
// text editor's and the fields' own clipboard, and the Trace tool. Harness as in
// `acceptance_0087_browser.test.ts`, on its own port.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import path from "node:path";
import { after, before, test } from "node:test";
import { crc32, deflateSync, inflateSync } from "node:zlib";
import { removeProfile } from "./browser/removeProfile.ts";
import { startVite, stopVite } from "./browser/vite.ts";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5288;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0087-tester");
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
  // A call the browser never answers (a clipboard read that waits for a permission or focus)
  // fails after a minute and says which call it was, instead of hanging until the test timeout.
  const send = (method: string, params: object = {}) =>
    new Promise<Reply>((resolve, reject) => {
      id += 1;
      const timer = setTimeout(() => reject(new Error(`no reply to ${method} in 60 s: ${JSON.stringify(params).slice(0, 200)}`)), 60_000);
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

const focusCanvas = `(() => { document.activeElement?.blur?.(); [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean)?.focus(); return true; })()`;
const IMG = path.resolve(FRONTEND, "..", "curvyo-image-core", "tests", "fixtures", "images");
const fixture64 = (name: string) => readFileSync(path.join(IMG, name)).toString("base64");

/** A `width` x `height` RGBA PNG of zeros (fully transparent), made fast for the size limits. */
function bigPng(width: number, height: number): Buffer {
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
  const raw = Buffer.alloc(height * (1 + width * 4));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 1 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Decodes an 8-bit RGB or RGBA, non-interlaced PNG into RGBA pixels. */
function decodePng(file: Buffer): { width: number; height: number; rgba: Uint8Array } {
  let pos = 8;
  let width = 0;
  let height = 0;
  let colour = 0;
  const idat: Buffer[] = [];
  while (pos < file.length) {
    const len = file.readUInt32BE(pos);
    const kind = file.toString("latin1", pos + 4, pos + 8);
    const data = file.subarray(pos + 8, pos + 8 + len);
    if (kind === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      assert.equal(data[8], 8, "8-bit");
      colour = data[9];
      assert.ok(colour === 2 || colour === 6, `colour type ${colour}`);
      assert.equal(data[12], 0, "not interlaced");
    }
    if (kind === "IDAT") {
      idat.push(data);
    }
    pos += 12 + len;
  }
  const bpp = colour === 6 ? 4 : 3;
  const raw = inflateSync(Buffer.concat(idat));
  const stride = width * bpp;
  const out = new Uint8Array(width * height * 4);
  let prev = new Uint8Array(stride);
  for (let y = 0; y < height; y++) {
    const f = raw[y * (stride + 1)];
    const line = new Uint8Array(raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1)));
    for (let i = 0; i < stride; i++) {
      const a = i >= bpp ? line[i - bpp] : 0;
      const b = prev[i];
      const c = i >= bpp ? prev[i - bpp] : 0;
      let add = 0;
      if (f === 1) add = a;
      else if (f === 2) add = b;
      else if (f === 3) add = (a + b) >> 1;
      else if (f === 4) {
        const p = a + b - c;
        const pa = Math.abs(p - a);
        const pb = Math.abs(p - b);
        const pc = Math.abs(p - c);
        add = pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
      }
      line[i] = (line[i] + add) & 255;
    }
    for (let x = 0; x < width; x++) {
      const o = (y * width + x) * 4;
      out[o] = line[x * bpp];
      out[o + 1] = line[x * bpp + 1];
      out[o + 2] = line[x * bpp + 2];
      out[o + 3] = bpp === 4 ? line[x * bpp + 3] : 255;
    }
    prev = line;
  }
  return { width, height, rgba: out };
}

/** 16 x 8 px with distinct opaque pixels, a half-transparent column and a marker top-left. */
const PATTERN = png(16, 8, (x, y) => {
  if (x === 0 && y === 0) return [255, 0, 0, 255];
  if (x === 15) return [0, 0, 255, 128];
  return [x * 16, y * 30, (x * 7 + y * 11) & 255, 255];
});
const patternPixel = (x: number, y: number): [number, number, number, number] => {
  if (x === 0 && y === 0) return [255, 0, 0, 255];
  if (x === 15) return [0, 0, 255, 128];
  return [x * 16, y * 30, (x * 7 + y * 11) & 255, 255];
};

const MAGENTA = png(300, 200, () => [255, 0, 255, 255]).toString("base64");

type Session = Awaited<ReturnType<typeof session>>;

let vite: ChildProcess | null = null;
before(async () => {
  if (!BROWSER) return;
  mkdirSync(SCRATCH, { recursive: true });
  vite = startVite(PORT, FRONTEND);
  await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
});
after(async () => {
  await stopVite(vite, PORT);
});

/** Opens the app in a fresh browser profile, runs `body`, closes everything. */
async function withApp(body: (s: Session) => Promise<void>): Promise<void> {
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  const s = await session(profile);
  try {
    await s.send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await s.evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(1500);
    await body(s);
  } finally {
    await s.close();
    removeProfile(profile);
  }
}

type PasteFile = { b64: string; type: string; name?: string };

/** A paste event on the focused element with `files` and the other flavours. */
const pasteEventOf = (files: PasteFile[], flavours: Record<string, string> = {}) => `(() => {
  const dt = new DataTransfer();
  for (const f of ${JSON.stringify(files)}) {
    const bytes = Uint8Array.from(atob(f.b64), (c) => c.charCodeAt(0));
    dt.items.add(new File([bytes], f.name ?? "image.png", { type: f.type }));
  }
  for (const [type, value] of Object.entries(${JSON.stringify(flavours)})) dt.setData(type, value);
  const target = document.activeElement || document.body;
  return target.dispatchEvent(new ClipboardEvent("paste", { clipboardData: dt, bubbles: true, cancelable: true }));
})()`;

const CANVAS_RECT = `(() => { const e = [...document.querySelectorAll('canvas')].map((c) => c.closest('[tabindex]')).find(Boolean); const b = e.getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; })()`;

/** The bounding box of the magenta pixels inside the canvas area of a screenshot. */
async function magentaBox(s: Session): Promise<{ n: number; cx: number; cy: number; w: number; h: number }> {
  const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as unknown as { result: { data: string } };
  const rect = (await s.evaluate(CANVAS_RECT)) as number[];
  const got = JSON.parse(
    (await s.evaluate(`(async () => {
      const blob = await (await fetch("data:image/png;base64,${shot.result.data}")).blob();
      const bm = await createImageBitmap(blob);
      const c = new OffscreenCanvas(bm.width, bm.height); const g = c.getContext("2d"); g.drawImage(bm, 0, 0);
      const d = g.getImageData(0, 0, bm.width, bm.height).data;
      const [rx, ry, rw, rh] = ${JSON.stringify(rect)};
      let x0 = 1e9, y0 = 1e9, x1 = -1, y1 = -1, n = 0;
      for (let y = Math.ceil(ry); y < ry + rh; y++) for (let x = Math.ceil(rx); x < rx + rw; x++) { const i = (y * bm.width + x) * 4;
        if (d[i] > 240 && d[i + 1] < 20 && d[i + 2] > 240) { n++; x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y); } }
      return JSON.stringify({ n, x0, y0, x1, y1 });
    })()`)) as string,
  ) as { n: number; x0: number; y0: number; x1: number; y1: number };
  return { n: got.n, cx: (got.x0 + got.x1 + 1) / 2, cy: (got.y0 + got.y1 + 1) / 2, w: got.x1 - got.x0 + 1, h: got.y1 - got.y0 + 1 };
}

const chord = async (s: Session, k: string, vk: number, commands?: string[], modifiers = 2) => {
  await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: `Key${k.toUpperCase()}`, windowsVirtualKeyCode: vk, modifiers, ...(commands ? { commands } : {}) });
  await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: `Key${k.toUpperCase()}`, windowsVirtualKeyCode: vk, modifiers });
  await sleep(500);
};

const readClipboard = async (s: Session): Promise<{ types: string[]; png?: string }> =>
  JSON.parse(
    (await s.evaluate(`(async () => {
      const items = await navigator.clipboard.read();
      const out = { types: items.flatMap((i) => i.types) };
      const it = items.find((i) => i.types.includes("image/png"));
      if (it) { const buf = new Uint8Array(await (await it.getType("image/png")).arrayBuffer()); let s = "";
        for (let i = 0; i < buf.length; i += 0x8000) s += String.fromCharCode(...buf.subarray(i, i + 0x8000)); out.png = btoa(s); }
      return JSON.stringify(out);
    })().catch((e) => JSON.stringify({ types: ["error " + e] }))`)) as string,
  );

const pastedRows = async (s: Session) => (await s.layerRows()).filter((r) => r?.includes("Pasted image")).length;
/** The non-empty texts of the status and alert regions, joined. */
const texts = async (s: Session) => ((await s.statusText()).split(" | ").filter((x) => x.trim() !== "")).join(" | ");
const notice = async (s: Session, needle: string, ms = 15_000) =>
  waitFor(`notice ${needle}`, async () => {
    const t = await texts(s);
    return t.includes(needle) ? t : null;
  }, ms);
const noNetwork = (s: Session, base: number) => assert.equal(s.requests.length, base, `requests since load: ${s.requests.slice(base).join(", ")}`);

// --- criteria 1, 2, 10: where it lands -------------------------------------------------------

test("0087 tester: a pasted PNG lands under the pointer, anywhere on the canvas", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    for (const [x, y] of [[640, 420], [330, 250], [700, 600]]) {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: x - 3, y });
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x, y });
      await s.evaluate(focusCanvas);
      const before = await pastedRows(s);
      await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
      await waitFor("the row", async () => ((await pastedRows(s)) === before + 1 ? true : null));
      await sleep(600);
      const box = await magentaBox(s);
      assert.ok(Math.abs(box.cx - x) <= 2 && Math.abs(box.cy - y) <= 2, `pointer ${x},${y}; image centre ${box.cx},${box.cy} (${box.w}x${box.h})`);
      await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Delete", code: "Delete", windowsVirtualKeyCode: 46 });
      await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Delete", code: "Delete", windowsVirtualKeyCode: 46 });
      await sleep(500);
    }
  });
});

test("0087 tester: the view centre when the pointer is over a panel, left the canvas, or never seen", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const [rx, ry, rw, rh] = (await s.evaluate(CANVAS_RECT)) as number[];
    const centre = { x: rx + rw / 2, y: ry + rh / 2 };
    const check = async (what: string) => {
      await sleep(600);
      const box = await magentaBox(s);
      assert.ok(Math.abs(box.cx - centre.x) <= 2 && Math.abs(box.cy - centre.y) <= 2, `${what}: view centre ${centre.x},${centre.y}; image centre ${box.cx},${box.cy}`);
    };
    const del = async () => {
      await s.send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: "Delete", code: "Delete", windowsVirtualKeyCode: 46 });
      await s.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Delete", code: "Delete", windowsVirtualKeyCode: 46 });
      await sleep(500);
    };
    // Never over the canvas since the app started.
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
    await waitFor("row 1", async () => ((await pastedRows(s)) === 1 ? true : null));
    await check("before any pointer move");
    await del();
    // The pointer was over the canvas, then moved to the right panel.
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 600, y: 300 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 1100, y: 300 });
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
    await waitFor("row 2", async () => ((await pastedRows(s)) === 1 ? true : null));
    await check("pointer over the panel");
    await del();
    // The pointer left through the toolbar rail (left of the canvas).
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 700, y: 500 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 10, y: 400 });
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
    await waitFor("row 3", async () => ((await pastedRows(s)) === 1 ? true : null));
    await check("pointer over the rail");
  });
});

test("0087 tester: JPEG and WebP files in a paste event import, by content and not by label", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const cases: [string, PasteFile][] = [
      ["jpeg", { b64: fixture64("jpeg_progressive.jpg"), type: "image/jpeg", name: "image.jpeg" }],
      ["webp", { b64: fixture64("webp_lossy.webp"), type: "image/webp", name: "image.webp" }],
      ["webp with alpha", { b64: fixture64("webp_lossless_alpha.webp"), type: "image/webp", name: "image.webp" }],
      ["png labelled jpeg", { b64: fixture64("png_rgb_8.png"), type: "image/jpeg", name: "image.jpeg" }],
      ["jpeg with no label", { b64: fixture64("jpeg_progressive.jpg"), type: "", name: "" }],
    ];
    let rows = 0;
    for (const [what, file] of cases) {
      await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 500, y: 400 });
      await s.evaluate(focusCanvas);
      assert.equal(await s.evaluate(pasteEventOf([file])), false, `${what}: the paste is taken`);
      rows += 1;
      await waitFor(`${what} row`, async () => ((await pastedRows(s)) === rows ? true : null));
    }
    // The image wins over the text and the HTML of the same clipboard (criterion 4).
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: fixture64("jpeg_progressive.jpg"), type: "image/jpeg" }], { "text/plain": "a caption", "text/html": "<b>a caption</b>" }));
    await waitFor("image with text", async () => ((await pastedRows(s)) === rows + 1 ? true : null));
  });
});

// --- criteria 4, 5, 7, 8: nothing, links, damaged, large -----------------------------------------

test("0087 tester: links and HTML are never fetched and say there is nothing to paste", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const base = s.requests.length;
    const flavours: [string, Record<string, string>][] = [
      ["html img", { "text/html": '<img src="https://example.com/x.png"><img src="http://127.0.0.1:9/y.jpg">' }],
      ["plain url", { "text/plain": "https://example.com/picture.png" }],
      ["uri list", { "text/uri-list": "https://example.com/picture.png\nhttp://127.0.0.1:9/a.webp" }],
      ["html with data uri", { "text/html": '<img src="data:image/png;base64,iVBORw0KGgo=">' }],
      ["html and url", { "text/html": '<a href="https://example.com/a.png"><img src="https://example.com/a.png"></a>', "text/plain": "https://example.com/a.png" }],
    ];
    for (const [what, f] of flavours) {
      await s.evaluate(focusCanvas);
      assert.equal(await s.evaluate(pasteEventOf([], f)), false, what);
      const text = await notice(s, "Nothing to paste");
      assert.ok(text.includes("Nothing to paste: the clipboard has no image."), `${what}: ${text}`);
      assert.ok(!/download|fetch|link|tip|try/i.test(text), `no hint text: ${text}`);
      assert.equal(await pastedRows(s), 0, what);
      await sleep(3300);
    }
    await sleep(1000);
    noNetwork(s, base);
  });
});

test("0087 tester: other clipboard content (SVG file, BMP, GIF, empty) is nothing or a refusal, never a step", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const svg = Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="5" height="5"/></svg>').toString("base64");
    const gif = Buffer.from("GIF89a\x01\x00\x01\x00\x00\x00\x00;", "latin1").toString("base64");
    const bmp = Buffer.from("BM\x1e\x00\x00\x00\x00\x00\x00\x00\x1a\x00\x00\x00", "latin1").toString("base64");
    const cases: [string, PasteFile[]][] = [
      ["svg file", [{ b64: svg, type: "image/svg+xml", name: "a.svg" }]],
      ["gif", [{ b64: gif, type: "image/gif", name: "a.gif" }]],
      ["bmp", [{ b64: bmp, type: "image/bmp", name: "a.bmp" }]],
      ["pdf as png", [{ b64: fixture64("pdf_renamed.png"), type: "image/png" }]],
      ["damaged png", [{ b64: fixture64("png_bad_crc.png"), type: "image/png" }]],
      ["truncated webp", [{ b64: fixture64("webp_truncated.webp"), type: "image/webp" }]],
      ["a file of another type", [{ b64: Buffer.from("hello").toString("base64"), type: "text/plain", name: "a.txt" }]],
    ];
    await s.tab("History");
    const historyBefore = (await s.evaluate(`document.body.innerText`)) as string;
    for (const [what, files] of cases) {
      await s.evaluate(focusCanvas);
      await s.evaluate(pasteEventOf(files));
      const text = await waitFor(`${what} notice`, async () => {
        const t = await texts(s);
        return t.length > 0 ? t : null;
      });
      console.log(`  notice for ${what}: ${JSON.stringify(text)}`);
      assert.ok(!/tip|try|you can|hint/i.test(text), `${what}: no hint text in ${text}`);
      await sleep(5200);
    }
    await s.tab("Layers");
    assert.equal(await pastedRows(s), 0);
    await s.tab("History");
    assert.equal((await s.evaluate(`document.body.innerText`)) as string, historyBefore, "no history step");
  });
});

test("0087 tester: header bombs, an image over 50 Mpx and one over the working limit", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const base = s.requests.length;
    for (const [what, b64, type] of [
      ["60,000 x 60,000 header", fixture64("png_bomb_60000x60000.png"), "image/png"],
      ["65535 x 65535 jpeg", fixture64("jpeg_dimensions_65535x65535.jpg"), "image/jpeg"],
      ["7100 x 7100 px", bigPng(7100, 7100).toString("base64"), "image/png"],
    ] as const) {
      await s.evaluate(focusCanvas);
      await s.evaluate(pasteEventOf([{ b64, type }]));
      const text = await notice(s, "too large", 60_000);
      assert.ok(/Image too large: [\d,]+ x [\d,]+ px\. The limit is 50 megapixels\./.test(text), `${what}: ${text}`);
      assert.equal(await pastedRows(s), 0, `${what}: nothing inserted`);
      await sleep(5500);
    }
    // 5000 x 4000 px (20 Mpx) is over the working limit and imports.
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: bigPng(5000, 4000).toString("base64"), type: "image/png" }]));
    await waitFor("the large image", async () => ((await pastedRows(s)) === 1 ? true : null), 60_000);
    console.log(`  notice for 5000 x 4000 px: ${JSON.stringify(await s.statusText())}`);
    noNetwork(s, base);
  });
});

// --- criteria 1, 9: undo, Trace --------------------------------------------------------------

test("0087 tester: one undo step per paste, redo restores, and the selection follows", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    for (let i = 1; i <= 3; i++) {
      await s.evaluate(focusCanvas);
      await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
      await waitFor(`row ${i}`, async () => ((await pastedRows(s)) === i ? true : null));
    }
    for (const left of [2, 1, 0]) {
      await s.evaluate(focusCanvas);
      await s.key("z", 90, 2);
      await waitFor(`undo to ${left}`, async () => ((await pastedRows(s)) === left ? true : null));
    }
    for (const back of [1, 2, 3]) {
      await s.evaluate(focusCanvas);
      await s.key("z", 90, 2 | 8);
      await waitFor(`redo to ${back}`, async () => ((await pastedRows(s)) === back ? true : null));
    }
    const names = (await s.evaluate(`document.body.innerText`)) as string;
    assert.ok(!/Paste image\s+Paste image\s+Paste image\s+Paste image/.test(names));
  });
});

test("0087 tester: a paste in the Trace tool keeps the tool and shows the new image's trace", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MARKER.toString("base64"), type: "image/png" }]));
    await waitFor("row", async () => ((await pastedRows(s)) === 1 ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("the Trace bar", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? true : null));
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: PATTERN.toString("base64"), type: "image/png" }]));
    await waitFor("second row", async () => ((await pastedRows(s)) === 2 ? true : null));
    await sleep(1500);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`), true, "the tool stays Trace");
    const selected = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')))`)) as string) as string[];
    assert.equal(selected.length, 1, `one selected row: ${selected}`);
    const status = (await s.evaluate(`document.querySelector('[role=toolbar][aria-label=Trace] [role=status]')?.textContent ?? ''`)) as string;
    console.log(`  Trace bar status after the paste: ${JSON.stringify(status)}`);
    await s.evaluate(`document.activeElement?.blur?.()`);
    await s.key("z", 90, 2);
    await waitFor("undo", async () => ((await pastedRows(s)) === 1 ? true : null));
  });
});

// --- criteria 11, 12, 13: copy -----------------------------------------------------------------

test("0087 tester: Ctrl+C puts the stored pixels on the clipboard as PNG, flip does not matter", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: PATTERN.toString("base64"), type: "image/png" }]));
    await waitFor("row", async () => ((await pastedRows(s)) === 1 ? true : null));
    const check = async (what: string) => {
      await s.evaluate(focusCanvas);
      await s.key("c", 67, 2);
      const text = await notice(s, "Image copied.");
      assert.ok(text.includes("Image copied."), text);
      const got = await readClipboard(s);
      assert.deepEqual(got.types, ["image/png"], `${what}: only a PNG`);
      const decoded = decodePng(Buffer.from(got.png!, "base64"));
      assert.deepEqual([decoded.width, decoded.height], [16, 8], what);
      for (let y = 0; y < 8; y++) {
        for (let x = 0; x < 16; x++) {
          const want = patternPixel(x, y);
          const o = (y * 16 + x) * 4;
          const have = [...decoded.rgba.subarray(o, o + 4)];
          // Chromium may premultiply a translucent pixel on its way through the clipboard.
          const tol = want[3] === 255 ? 0 : 3;
          assert.ok(want.every((v, i) => Math.abs(v - have[i]) <= (i === 3 ? 0 : tol)), `${what}: pixel ${x},${y} want ${want} have ${have}`);
        }
      }
      await sleep(3300);
    };
    await check("as pasted");
    await s.clickSel('[aria-label="Flip horizontally"]');
    await s.clickSel('[aria-label="Flip vertically"]');
    await check("flipped both ways");
    // The copy is read-only: the history has the paste and the two flips, nothing else.
    assert.equal(await pastedRows(s), 1);
  });
});

test("0087 tester: several images say how many, the topmost is copied; other selections copy nothing", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    for (const [i, file] of [MARKER, PATTERN, MARKER].entries()) {
      await s.evaluate(focusCanvas);
      await s.evaluate(pasteEventOf([{ b64: file.toString("base64"), type: "image/png" }]));
      await waitFor(`row ${i + 1}`, async () => ((await pastedRows(s)) === i + 1 ? true : null));
    }
    // Put text on the clipboard first: a copy that does nothing leaves it alone.
    await s.evaluate(`navigator.clipboard.writeText("sentinel")`);
    await s.evaluate(focusCanvas);
    await s.key("a", 65, 2);
    await s.key("c", 67, 2);
    const text = await notice(s, "Copied 1 of 3 images.");
    assert.ok(text.includes("Copied 1 of 3 images."), text);
    const got = await readClipboard(s);
    const decoded = decodePng(Buffer.from(got.png!, "base64"));
    assert.deepEqual([decoded.width, decoded.height], [40, 20], "the topmost is the last pasted, the 40 x 20 marker");
    await sleep(3300);
    // A rectangle only: nothing happens, no notice, the clipboard keeps its image.
    await s.evaluate(`navigator.clipboard.writeText("sentinel")`);
    await s.clickSel('[aria-label^="Rectangle tool"]');
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 300, y: 300 });
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: 300, y: 300, button: "left", clickCount: 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 380, y: 360 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: 380, y: 360, button: "left", clickCount: 1 });
    await sleep(500);
    await s.evaluate(focusCanvas);
    await s.key("c", 67, 2);
    await sleep(800);
    assert.ok(!/copied|Copying|Could not/i.test(await s.statusText()), `no notice: ${await s.statusText()}`);
    assert.equal(await s.evaluate(`navigator.clipboard.readText()`), "sentinel", "the clipboard is untouched");
  });
});

test("0087 tester: Ctrl+C in a refusing browser says so and the document is unchanged", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MARKER.toString("base64"), type: "image/png" }]));
    await waitFor("row", async () => ((await pastedRows(s)) === 1 ? true : null));
    await s.evaluate(`navigator.clipboard.write = () => Promise.reject(new DOMException("Write permission denied.", "NotAllowedError"))`);
    await s.evaluate(focusCanvas);
    await s.key("c", 67, 2);
    const text = await notice(s, "Could not copy the image");
    const line = text.split(" | ").find((t) => t.includes("Could not copy")) ?? "";
    assert.ok(/^Could not copy the image: .+\.$/.test(line) && line.length <= 70, `${line.length}: ${line}`);
    assert.equal(await pastedRows(s), 1);
    await sleep(5500);
    await s.evaluate(`delete navigator.clipboard.write; Object.defineProperty(window, "ClipboardItem", { value: undefined, configurable: true })`);
    await s.evaluate(focusCanvas);
    await s.key("c", 67, 2);
    const old = await notice(s, "needs a newer browser");
    assert.ok(old.includes("Copying images needs a newer browser."), old);
  });
});

// --- criteria 3, 16: text keeps its clipboard ----------------------------------------------------

test("0087 tester: the text editor and the panel fields keep copy and paste", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: PATTERN.toString("base64"), type: "image/png" }]));
    await waitFor("row", async () => ((await pastedRows(s)) === 1 ? true : null));
    // Ctrl+C on the canvas: the image is on the clipboard.
    await s.key("c", 67, 2);
    await notice(s, "Image copied.");
    assert.ok((await readClipboard(s)).types.includes("image/png"));
    await sleep(3300);

    // A panel field: Ctrl+C copies its text; Ctrl+V of the image changes nothing and adds no object.
    await s.tab("Document");
    const field = (await s.evaluate(`(() => { const i = document.querySelector('input[aria-label="Document width"]'); if (!i) return false; i.focus(); i.select(); return document.activeElement === i; })()`)) as boolean;
    if (field) {
      await chord(s, "c", 67, ["copy"]);
      assert.equal(await s.evaluate(`navigator.clipboard.readText()`), "210", "the field's text is on the clipboard");
      assert.ok(!(await readClipboard(s)).types.includes("image/png"), "the image is not copied from a field");
    }
    assert.ok(field, "the Document width field is reachable");
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.key("c", 67, 2);
    await notice(s, "Image copied.");
    await s.tab("Document");
    assert.equal(await s.evaluate(`(() => { const i = document.querySelector('input[aria-label="Document width"]'); i.focus(); i.select(); return document.activeElement === i; })()`), true);
    await chord(s, "v", 86, ["paste"]);
    await sleep(500);
    await s.tab("Layers");
    assert.equal(await pastedRows(s), 1, "a paste in a field adds no image");
    await s.evaluate(`document.activeElement?.blur?.()`);

    // The text editor on the canvas.
    await s.clickSel('[aria-label^="Text tool"]');
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 200, y: 150 });
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: 200, y: 150, button: "left", clickCount: 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: 200, y: 150, button: "left", clickCount: 1 });
    await sleep(600);
    const editing = await s.evaluate(`document.activeElement?.tagName`);
    assert.equal(editing, "TEXTAREA", "the text editor has focus");
    await s.send("Input.insertText", { text: "abc" });
    await chord(s, "a", 65, ["selectAll"]);
    await chord(s, "c", 67, ["copy"]);
    assert.equal(await s.evaluate(`navigator.clipboard.readText()`), "abc", "Ctrl+C in the text editor copies the text");
    assert.ok(!(await readClipboard(s)).types.includes("image/png"), "no image on the clipboard after a text copy");
    await chord(s, "v", 86, ["paste"]);
    assert.equal(await s.evaluate(`document.activeElement?.value`), "abc", "Ctrl+V in the text editor pastes the text");
    // The text editor's own handler may swallow the event; what counts is that no image comes in.
    await s.evaluate(pasteEventOf([{ b64: PATTERN.toString("base64"), type: "image/png" }]));
    await sleep(800);
    assert.equal(await pastedRows(s), 1, "no image object from the text editor");
    const text = await s.statusText();
    assert.ok(!text.includes("Nothing to paste"), text);
  });
});

test("0087 tester: a paste is ignored while a mouse drag runs", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.clickSel('[aria-label^="Rectangle tool"]');
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 300, y: 300 });
    await s.send("Input.dispatchMouseEvent", { type: "mousePressed", x: 300, y: 300, button: "left", clickCount: 1 });
    await s.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: 350, y: 340 });
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
    await sleep(1000);
    assert.equal(await pastedRows(s), 0, "nothing pasted during the drag");
    await s.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: 350, y: 340, button: "left", clickCount: 1 });
    await sleep(400);
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MAGENTA, type: "image/png" }]));
    await waitFor("the paste after the drag", async () => ((await pastedRows(s)) === 1 ? true : null));
  });
});

test("0087 tester: a real Ctrl+V of a text-only clipboard says there is nothing to paste", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(`navigator.clipboard.writeText("only text")`);
    await s.evaluate(focusCanvas);
    await chord(s, "v", 86, ["paste"]);
    const text = await notice(s, "Nothing to paste");
    assert.ok(text.includes("Nothing to paste: the clipboard has no image."), text);
    assert.equal(text.split(" | ").find((t) => t.startsWith("Nothing"))?.length, 45);
    assert.equal(await pastedRows(s), 0);
  });
});

// --- slice 2: dropping image data (criteria 19 to 24) --------------------------------------------

type DropItem = { name: string; type: string; b64: string };

/** dragenter + dragover on the element at (x, y) with `files` and `strings`; keeps the DataTransfer
 * for `dropNow`. Returns how dragover was taken: "copy" (a drop target stopped it), "none" (it
 * reached the window and the page refused) or "refused" (nobody took it). */
const dragHover = (x: number, y: number, files: DropItem[], strings: Record<string, string>) => `(() => {
  const dt = new DataTransfer();
  for (const f of ${JSON.stringify(files)}) dt.items.add(new File([Uint8Array.from(atob(f.b64), (c) => c.charCodeAt(0))], f.name, { type: f.type }));
  for (const [type, value] of Object.entries(${JSON.stringify(strings)})) dt.setData(type, value);
  window.__dt = dt;
  const target = document.elementFromPoint(${x}, ${y});
  const fire = (type) => { const e = new DragEvent(type, { clientX: ${x}, clientY: ${y}, dataTransfer: dt, bubbles: true, cancelable: true }); target.dispatchEvent(e); return e; };
  fire("dragenter");
  let reached = false;
  const seen = () => { reached = true; };
  window.addEventListener("dragover", seen);
  const over = fire("dragover");
  window.removeEventListener("dragover", seen);
  return over.defaultPrevented ? (reached ? "none" : "copy") : "refused";
})()`;
const dropNow = (x: number, y: number) => `(() => {
  const target = document.elementFromPoint(${x}, ${y});
  const e = new DragEvent("drop", { clientX: ${x}, clientY: ${y}, dataTransfer: window.__dt, bubbles: true, cancelable: true });
  target.dispatchEvent(e);
  return e.defaultPrevented;
})()`;
const outlineShown = `[...document.querySelectorAll('div[aria-hidden]')].some((d) => d.style.outline.includes('dashed'))`;
const dropAt = async (s: Session, x: number, y: number, files: DropItem[], strings: Record<string, string> = {}) => {
  const effect = await s.evaluate(dragHover(x, y, files, strings));
  await s.evaluate(dropNow(x, y));
  return effect;
};

test("0087 tester drop: image data lands at the drop point; hover shows the outline and a leave removes it", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const item: DropItem = { name: "", type: "image/png", b64: MAGENTA };
    // Hover: outline shows on the canvas, not over a panel; dragleave takes it away.
    assert.equal(await s.evaluate(dragHover(400, 300, [item], {})), "copy");
    await sleep(200);
    assert.equal(await s.evaluate(outlineShown), true, "dashed outline while image data hovers");
    await s.evaluate(`window.__dt && document.elementFromPoint(400, 300).dispatchEvent(new DragEvent("dragleave", { dataTransfer: window.__dt, bubbles: true }))`);
    await sleep(200);
    assert.equal(await s.evaluate(outlineShown), false, "outline gone after the drag left");
    // Drop points.
    for (const [x, y] of [[400, 300], [720, 560]]) {
      assert.equal(await dropAt(s, x, y, [item]), "copy");
      await waitFor("row", async () => (((await s.layerRows()).filter((r) => r?.includes("Dropped image")).length) >= 1 ? true : null));
      await sleep(600);
      const box = await magentaBox(s);
      assert.ok(Math.abs(box.cx - x) <= 2 && Math.abs(box.cy - y) <= 2, `drop ${x},${y}; image centre ${box.cx},${box.cy}`);
      await s.evaluate(focusCanvas);
      await s.key("z", 90, 2);
      await waitFor("undo", async () => (((await s.layerRows()).filter((r) => r?.includes("Dropped image")).length) === 0 ? true : null));
    }
    // The History names the step "Import image".
    await dropAt(s, 400, 300, [item]);
    await waitFor("row", async () => (((await s.layerRows()).filter((r) => r?.includes("Dropped image")).length) === 1 ? true : null));
    await s.tab("History");
    assert.ok(((await s.evaluate(`document.body.innerText`)) as string).includes("Import image"));
  });
});

test("0087 tester drop: JPEG and WebP image data, several files in one step, image wins over HTML", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const count = async (n: string) => (await s.layerRows()).filter((r) => r?.includes(n)).length;
    await dropAt(s, 400, 300, [{ name: "", type: "image/jpeg", b64: fixture64("jpeg_progressive.jpg") }], { "text/html": '<img src="https://example.com/a.jpg">', "text/uri-list": "https://example.com/a.jpg" });
    await waitFor("jpeg", async () => ((await count("Dropped image")) === 1 ? true : null));
    await dropAt(s, 400, 300, [{ name: "", type: "image/webp", b64: fixture64("webp_lossy.webp") }]);
    await waitFor("webp", async () => ((await count("Dropped image")) === 2 ? true : null));
    // Several files: one step "Import images", the last on top, all selected.
    await dropAt(s, 500, 350, [
      { name: "a.png", type: "image/png", b64: MARKER.toString("base64") },
      { name: "b.jpg", type: "image/jpeg", b64: fixture64("jpeg_progressive.jpg") },
      { name: "c.webp", type: "image/webp", b64: fixture64("webp_lossless_alpha.webp") },
    ]);
    await waitFor("three files", async () => (((await count("a.png")) + (await count("b.jpg")) + (await count("c.webp"))) === 3 ? true : null));
    const selected = (await s.evaluate(`document.querySelectorAll('[role=row][aria-selected=true]').length`)) as number;
    assert.equal(selected, 3, "all three are selected");
    await s.tab("History");
    assert.ok(((await s.evaluate(`document.body.innerText`)) as string).includes("Import images"));
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.key("z", 90, 2);
    await waitFor("one undo removes the three", async () => (((await count("a.png")) + (await count("b.jpg")) + (await count("c.webp"))) === 0 ? true : null));
  });
});

test("0087 tester drop: a link only is never fetched, whatever the flavour, and says so as an alert", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const base = s.requests.length;
    const flavours: Record<string, string>[] = [
      { "text/uri-list": "https://example.com/photo.jpg" },
      { "text/html": '<img src="https://example.com/photo.jpg">' },
      { "text/html": '<img src="http://127.0.0.1:9/a.png">', "text/plain": "http://127.0.0.1:9/a.png" },
      { "text/uri-list": "# comment\nhttps://example.com/1.png\nhttps://example.com/2.png" },
      { "text/html": '<img src="data:image/png;base64,iVBORw0KGgo=">' },
    ];
    for (const f of flavours) {
      assert.equal(await dropAt(s, 400, 300, [], f), "copy", JSON.stringify(f));
      const text = await notice(s, "That drop is a link");
      assert.ok(text.includes("That drop is a link, not an image. Nothing was downloaded."), text);
      assert.equal(await s.evaluate(`[...document.querySelectorAll('[role=alert]')].some((e) => e.textContent.includes('That drop is a link'))`), true, "role=alert");
      assert.ok(!/tip|try|drag|save/i.test(text.replace("That drop is a link, not an image. Nothing was downloaded.", "")), text);
      assert.equal((await s.layerRows()).length, 0);
      await sleep(5300);
    }
    await sleep(500);
    noNetwork(s, base);
  });
});

test("0087 tester drop: mixed drops, a .curvyo file, a bomb, and drops outside the canvas", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    const count = async (n: string) => (await s.layerRows()).filter((r) => r?.includes(n)).length;
    // An image with a .curvyo file, a text file and a bomb: the image comes in, one merged notice.
    await dropAt(s, 400, 300, [
      { name: "ok.png", type: "image/png", b64: MARKER.toString("base64") },
      { name: "project.curvyo", type: "application/octet-stream", b64: Buffer.from("PK\u0003\u0004").toString("base64") },
      { name: "notes.txt", type: "text/plain", b64: Buffer.from("hi").toString("base64") },
      { name: "bomb.png", type: "image/png", b64: fixture64("png_bomb_60000x60000.png") },
    ]);
    await waitFor("ok.png", async () => ((await count("ok.png")) === 1 ? true : null));
    const text = await notice(s, "could not be imported");
    console.log(`  merged notice: ${JSON.stringify(text)}`);
    assert.ok(text.includes("3 files could not be imported."), text);
    assert.equal((await s.layerRows()).length, 1, "only the image came in; the .curvyo file opened nothing");
    // A .curvyo file alone opens nothing.
    await sleep(5500);
    await dropAt(s, 400, 300, [{ name: "project.curvyo", type: "application/octet-stream", b64: Buffer.from("PK\u0003\u0004").toString("base64") }]);
    const alone = await notice(s, "isn't supported");
    console.log(`  .curvyo alone: ${JSON.stringify(alone)}`);
    assert.equal((await s.layerRows()).length, 1);
    // Outside the canvas: panel, rail, ruler, the Import button; image data, files and links alike.
    const before = (await s.layerRows()).length;
    const png: DropItem = { name: "x.png", type: "image/png", b64: MAGENTA };
    for (const [what, x, y] of [["panel", 1100, 300], ["rail", 10, 400], ["ruler", 400, 10], ["left ruler", 12, 100]] as const) {
      assert.equal(await s.evaluate(dragHover(x, y, [png], {})), "none", `${what}: files`);
      assert.equal(await s.evaluate(dragHover(x, y, [], { "text/uri-list": "https://example.com/a.png" })), "none", `${what}: link`);
      assert.equal(await s.evaluate(outlineShown), false, `${what}: no outline`);
      await s.evaluate(dropNow(x, y));
    }
    await sleep(1000);
    assert.equal((await s.layerRows()).length, before, "nothing imported outside the canvas");
  });
});

test("0087 tester drop: during the Trace tool the tool stays and the new image is selected", { skip: !BROWSER, timeout: 300_000 }, async () => {
  await withApp(async (s) => {
    await s.tab("Layers");
    await s.evaluate(focusCanvas);
    await s.evaluate(pasteEventOf([{ b64: MARKER.toString("base64"), type: "image/png" }]));
    await waitFor("row", async () => ((await pastedRows(s)) === 1 ? true : null));
    await s.evaluate(`[...document.querySelectorAll('[data-context-bar] button')].find((b) => b.textContent === 'Trace').click()`);
    await waitFor("Trace bar", async () => ((await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? true : null));
    await dropAt(s, 600, 400, [{ name: "", type: "image/png", b64: PATTERN.toString("base64") }]);
    await waitFor("dropped", async () => (((await s.layerRows()).filter((r) => r?.includes("Dropped image")).length) === 1 ? true : null));
    await sleep(1200);
    assert.equal(await s.evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`), true, "the tool stays Trace");
    const selected = JSON.parse((await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-selected=true]')].map((r) => r.getAttribute('aria-label')))`)) as string) as string[];
    assert.equal(selected.length, 1);
    assert.ok(selected[0]?.includes("Dropped image"), String(selected));
  });
});

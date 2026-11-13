// Re-capture of the 0114 target pictures 06 to 10 from the real build (not a test file), slice 4. Run from frontend/:
// node --experimental-strip-types <this file> <out-dir> <frontend-dir> <picture.png>
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { crc32, deflateSync } from "node:zlib";

const OUT = process.argv[2];
const FRONTEND = process.argv[3];
const PORT = 5412;
const ORIGIN = `http://localhost:${PORT}`;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
mkdirSync(OUT, { recursive: true });
const DOWNLOADS = path.join(OUT, "downloads");
mkdirSync(DOWNLOADS, { recursive: true });

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 90_000): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch(() => null);
    if (got !== null) return got;
    await sleep(250);
  }
  throw new Error(`timed out waiting for ${what}`);
}

function png(width: number, height: number, colour: (x: number, y: number) => [number, number, number]): Buffer {
  const chunk = (kind: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const all = Buffer.concat([Buffer.from(kind, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(all) >>> 0);
    return Buffer.concat([len, all, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const rows = Array.from({ length: height }, (_, y) => Buffer.from([0, ...Array.from({ length: width }, (_, x) => colour(x, y)).flat()]));
  return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk("IHDR", ihdr), chunk("IDAT", deflateSync(Buffer.concat(rows))), chunk("IEND", Buffer.alloc(0))]);
}

// M1-like: a disc on the left, a square on the right, on white.
const picture = png(300, 300, (x, y) => {
  const disc = (x - 75) ** 2 + (y - 150) ** 2 < 45 ** 2;
  const square = x > 180 && x < 270 && y > 100 && y < 200;
  return disc || square ? [0, 0, 0] : [255, 255, 255];
});

const vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore", detached: true });
const stop = () => {
  try {
    if (vite.pid) process.kill(-vite.pid, "SIGKILL");
  } catch {}
};
process.on("exit", stop);

const profile = mkdtempSync(path.join(OUT, "profile-"));
const chrome = spawn("/usr/bin/chromium", ["--headless=new", "--no-first-run", "--no-default-browser-check", "--force-device-scale-factor=1", "--window-size=1600,1000", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"], { stdio: "ignore" });
process.on("exit", () => chrome.kill("SIGKILL"));

type Reply = { result?: { result?: { value?: unknown }; exceptionDetails?: unknown; data?: string } };
const portFile = path.join(profile, "DevToolsActivePort");
const port = await waitFor("devtools", async () => (existsSync(portFile) && readFileSync(portFile, "utf8").includes("\n") ? readFileSync(portFile, "utf8").split("\n")[0] : null));
const target = await waitFor("page", async () => ((await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as { type: string; webSocketDebuggerUrl: string }[]).find((t) => t.type === "page") ?? null);
const ws = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((res, rej) => {
  ws.onopen = res;
  ws.onerror = rej;
});
let id = 0;
const pending = new Map<number, (v: Reply) => void>();
const choosers: number[] = [];
ws.onmessage = (e) => {
  const m = JSON.parse(String(e.data)) as { id?: number; method?: string; params?: { backendNodeId?: number } };
  if (m.id !== undefined) pending.get(m.id)?.(m as Reply);
  if (m.method === "Page.fileChooserOpened" && m.params?.backendNodeId !== undefined) choosers.push(m.params.backendNodeId);
  if (m.method === "Page.javascriptDialogOpening") void send("Page.handleJavaScriptDialog", { accept: true });
};
const send = (method: string, params: object = {}) =>
  new Promise<Reply>((resolve) => {
    id += 1;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
const evaluate = async (expression: string): Promise<unknown> => {
  const out = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (out.result?.exceptionDetails) throw new Error(JSON.stringify(out.result.exceptionDetails));
  return out.result?.result?.value;
};
const shotN = { n: 0 };
const shot = async (name: string) => {
  const out = await send("Page.captureScreenshot", { format: "png" });
  shotN.n += 1;
  const file = path.join(OUT, `${String(shotN.n).padStart(2, "0")}-${name}.png`);
  writeFileSync(file, Buffer.from(out.result?.data ?? "", "base64"));
  console.log("shot", file);
  return out.result?.data ?? "";
};
const mouse = async (type: string, x: number, y: number, buttons = 0) =>
  send("Input.dispatchMouseEvent", { type, x, y, button: type === "mouseMoved" && buttons === 0 ? "none" : "left", buttons, clickCount: 1 });
const click = async (x: number, y: number) => {
  await mouse("mouseMoved", x, y);
  await mouse("mousePressed", x, y, 1);
  await mouse("mouseReleased", x, y);
  await sleep(400);
};
const drag = async (a: [number, number], b: [number, number]) => {
  await mouse("mouseMoved", a[0], a[1]);
  await mouse("mousePressed", a[0], a[1], 1);
  for (let i = 1; i <= 10; i++) await mouse("mouseMoved", a[0] + ((b[0] - a[0]) * i) / 10, a[1] + ((b[1] - a[1]) * i) / 10, 1);
  await mouse("mouseReleased", b[0], b[1]);
  await sleep(600);
};
const key = async (k: string, code: number, modifiers = 0, keyCode = `Key${k.toUpperCase()}`) => {
  await send("Input.dispatchKeyEvent", { type: "rawKeyDown", key: k, code: keyCode, windowsVirtualKeyCode: code, modifiers });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key: k, code: keyCode, windowsVirtualKeyCode: code, modifiers });
  await sleep(500);
};
const centre = async (sel: string): Promise<[number, number]> => {
  const c = JSON.parse((await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return 'null'; const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string) as [number, number] | null;
  if (!c) throw new Error(`${sel} is not on the page`);
  return c;
};
const clickSel = async (sel: string) => click(...(await centre(sel)));
const choose = async (sel: string, file: string) => {
  const before = choosers.length;
  await clickSel(sel);
  const node = await waitFor("chooser", async () => (choosers.length > before ? choosers[choosers.length - 1] : null), 10_000);
  await send("DOM.setFileInputFiles", { files: [file], backendNodeId: node });
  await sleep(2000);
};
const layerRows = async () => JSON.parse((await evaluate(`JSON.stringify([...document.querySelectorAll('[role=row][aria-level]')].map((r) => r.getAttribute('aria-label')))`)) as string) as string[];
const canvasRect = async () => JSON.parse((await evaluate(`JSON.stringify((() => { const r = [...document.querySelectorAll('canvas')].map((c) => c.getBoundingClientRect()).sort((a, b) => b.width * b.height - a.width * a.height)[0]; return [r.x, r.y, r.width, r.height]; })())`)) as string) as [number, number, number, number];
const pressed = async (sel: string) => evaluate(`document.querySelector(${JSON.stringify(sel)})?.getAttribute('aria-pressed') ?? 'missing'`);
const historyText = async () => (await evaluate(`[...document.querySelectorAll('[data-history-row], [role=listitem], [role=option]')].map((e) => e.textContent).join(' | ')`)) as string;
const tab = async (name: string) => {
  await clickSel(`[role=tab][aria-label^="${name}"]`);
  await sleep(400);
};

// ---- the real build at the sizes of the UX notes (section 0) ----
const LOGO = process.argv[4];
const barRect = async () => JSON.parse((await evaluate(`JSON.stringify((() => { const b = document.querySelector('[role=toolbar][aria-label=Trace]').getBoundingClientRect(); return [b.x, b.y, b.width, b.height]; })())`)) as string) as number[];
const clipShot = async (name: string, pad = 8) => {
  const [x, y, w, h] = await barRect();
  const clip = { x: Math.max(0, x - pad), y: Math.max(0, y - pad), width: w + 2 * pad, height: h + 2 * pad, scale: 1 };
  const out = await send("Page.captureScreenshot", { format: "png", clip });
  const file = path.join(OUT, `${name}.png`);
  writeFileSync(file, Buffer.from(out.result?.data ?? "", "base64"));
  console.log("shot", file, JSON.stringify(clip));
};
/** [window width, height, panel open, extra parts, file name] */
const CASES: [number, number, boolean, number, string][] = [
  [1280, 800, true, 0, "06-target-1280-advanced-open"],
  [1280, 800, false, 0, "07-target-1280-panel-closed-advanced-open"],
  [1920, 1080, true, 0, "08-target-1920-advanced-open-grown"],
  [1920, 1080, true, 7, "09-target-1920-advanced-open"],
  [800, 600, true, 0, "10-target-800-advanced-open"],
];
try {
  await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
  await send("Page.enable");
  await send("Page.setInterceptFileChooserDialog", { enabled: true });
  for (const [W, H, panel, extraParts, name] of CASES) {
    await send("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
    await send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await evaluate(`!!document.querySelector('[data-import-button]')`)) ? true : null));
    await sleep(2000);
    await tab("Layers");
    await choose("[data-import-button]", LOGO);
    await waitFor("the image row", async () => ((await layerRows()).some((r) => r?.startsWith("input-logo.png")) ? true : null));
    await sleep(800);
    await evaluate(`[...document.querySelectorAll('[role=row][aria-level]')].find((r) => r.getAttribute('aria-label')?.startsWith('input-logo.png')).click()`);
    await sleep(400);
    const [cx, cy, cw, ch] = await canvasRect();
    await mouse("mouseMoved", cx + cw - 40, cy + ch - 40);
    await key("i", 73);
    await waitFor("the Trace bar", async () => ((await evaluate(`!!document.querySelector('[role=toolbar][aria-label=Trace]')`)) ? true : null), 20_000);
    await sleep(2500);
    if (!panel) {
      await clickSel('[aria-label="Hide panel"]');
      await sleep(800);
    }
    await clickSel('[role=toolbar][aria-label=Trace] button[aria-label="Method"]');
    await sleep(400);
    await evaluate(`[...document.querySelectorAll('[role=menuitemradio]')].find((e) => e.textContent.startsWith('Brightness'))?.click()`);
    await sleep(1500);
    if (await evaluate(`!document.querySelector('[aria-label="Fewer trace options"]')`)) await clickSel('[aria-label="More trace options"]');
    for (let n = 0; n < extraParts; n++) {
      await clickSel('[role=toolbar][aria-label=Trace] button[aria-label="Add part"]');
      await sleep(300);
    }
    await mouse("mouseMoved", cx + cw - 40, cy + ch - 40);
    await sleep(1500);
    await clipShot(name);
  }
} catch (error) {
  console.error("ERROR", error);
  await shot("error").catch(() => undefined);
} finally {
  ws.close();
  chrome.kill("SIGKILL");
  stop();
  rmSync(profile, { recursive: true, force: true });
  process.exit(0);
}

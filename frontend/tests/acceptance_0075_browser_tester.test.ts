// Independent tester case for `specs/0075-font-library` criteria 29 and 31 in a real browser:
// a font added in the browser build is listed again, with its bytes, after the browser is
// closed and started again with the same profile; the font library text survives the same way;
// a fresh profile lists no Curvyo font (the browser sees nothing of the desktop's folder).
//
// Needs Chromium and the wasm bindings (`npm run build:wasm`); runs only with
// CURVYO_E2E_BROWSER=/path/to/chromium, e.g.
//   CURVYO_E2E_BROWSER=/usr/bin/chromium npm --prefix frontend test
// Written from the specification and the public names before the implementation was read.
import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5199;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

async function waitFor<T>(what: string, probe: () => Promise<T | null>, ms = 60_000): Promise<T> {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    const got = await probe().catch((e) => { if (process.env.DEBUG_E2E) console.error(what, e); return null; });
    if (got !== null) {
      return got;
    }
    await sleep(200);
  }
  throw new Error(`timed out waiting for ${what}`);
}

/** Starts Chromium on `profile`, opens the app's origin and evaluates `expression` there. */
async function inBrowser(profile: string, expression: string): Promise<unknown> {
  const portFile = path.join(profile, "DevToolsActivePort");
  // A port file of the previous run would point at a closed browser.
  rmSync(portFile, { force: true });
  const chrome = spawn(
    BROWSER!,
    [
      "--headless=new",
      "--no-first-run",
      "--no-default-browser-check",
      "--remote-debugging-port=0",
      `--user-data-dir=${profile}`,
      "about:blank",
    ],
    { stdio: process.env.DEBUG_E2E ? "inherit" : "ignore" },
  );
  try {
    const port = await waitFor("DevTools port", async () =>
      existsSync(portFile) && readFileSync(portFile, "utf8").includes("\n")
        ? readFileSync(portFile, "utf8").split("\n")[0]
        : null,
    );
    const targets = await waitFor("page target", async () => {
      const list = (await (await fetch(`http://127.0.0.1:${port}/json`)).json()) as {
        type: string;
        webSocketDebuggerUrl: string;
      }[];
      return list.find((t) => t.type === "page") ?? null;
    });
    const ws = new WebSocket(targets.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = reject;
    });
    let id = 0;
    const pending = new Map<number, (v: { result?: unknown; error?: unknown }) => void>();
    ws.onmessage = (event) => {
      const msg = JSON.parse(String(event.data)) as { id?: number; result?: unknown; error?: unknown };
      if (msg.id !== undefined) {
        pending.get(msg.id)?.(msg);
      }
    };
    const send = (method: string, params: object) =>
      new Promise<{ result?: { result?: { value?: unknown }; exceptionDetails?: unknown }; error?: unknown }>(
        (resolve) => {
          id += 1;
          pending.set(id, resolve as never);
          ws.send(JSON.stringify({ id, method, params }));
        },
      );
    await send("Page.enable", {});
    await send("Page.navigate", { url: `${ORIGIN}/` });
    await sleep(1500);
    const out = await send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (out.result?.exceptionDetails) {
      throw new Error(JSON.stringify(out.result.exceptionDetails));
    }
    // Let the browser flush its storage before it is closed.
    await sleep(500);
    await send("Browser.close", {}).catch(() => undefined);
    ws.close();
    return out.result?.result?.value;
  } finally {
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
  }
}

const PRELUDE = `
  const w = await import('/src/wasm-bindings/curvyo_editor_wasm.js');
  await w.default();
  const fonts = await import('/src/lib/browserFonts.ts');
  const host = await import('/src/platform/host.ts');
`;

test("ac29 ac31: browser fonts and library survive a browser restart; a fresh profile has none", { skip: !BROWSER, timeout: 180_000 }, async () => {
  let vite: ChildProcess | null = null;
  const profile = mkdtempSync(path.join(tmpdir(), "curvyo-0075-"));
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));

    const first = (await inBrowser(
      profile,
      `(async () => {${PRELUDE}
        const before = await host.invoke('font_folder_list');
        const bytes = await (await fetch('/fonts/great-vibes/GreatVibes-Regular.ttf')).arrayBuffer();
        const notFont = new File([new Uint8Array([1, 2, 3])], 'notes.ttf');
        const added = await fonts.addBrowserFontFiles([new File([bytes], 'GreatVibes.ttf'), notFont]);
        const again = await fonts.addBrowserFontFiles([new File([bytes], 'Copy.ttf')]);
        await host.invoke('write_font_library', { text: 'format = 1\\nfavourites = ["great vibes"]\\n' });
        const after = await host.invoke('font_folder_list');
        return JSON.stringify({ before, added, again, after, size: bytes.byteLength });
      })()`,
    )) as string;
    const a = JSON.parse(first);
    assert.deepEqual(a.before.faces, [], "ac31: a fresh profile has no Curvyo font");
    assert.deepEqual(a.added.added, ["GreatVibes.ttf"]);
    assert.deepEqual(a.added.notFonts, ["notes.ttf"]);
    assert.deepEqual(a.again.present, ["Copy.ttf"], "the same bytes again are already there");
    assert.equal(a.after.faces.length, 1);
    assert.equal(a.after.faces[0].family, "Great Vibes");

    // The browser is closed and started again with the same profile.
    const second = (await inBrowser(
      profile,
      `(async () => {${PRELUDE}
        const listed = await host.invoke('font_folder_list');
        const bytes = await host.invoke('font_bytes', { id: listed.faces[0]?.id });
        const library = await host.invoke('read_font_library');
        const system = await host.invoke('system_font_list');
        return JSON.stringify({ listed, size: bytes.byteLength, library, system });
      })()`,
    )) as string;
    const b = JSON.parse(second);
    assert.equal(b.listed.faces.length, 1, "ac29: listed again after a restart");
    assert.equal(b.listed.faces[0].family, "Great Vibes");
    assert.equal(b.size, a.size, "ac29: its bytes are kept, so texts can draw with it");
    assert.match(b.library, /great vibes/, "ac29: the library text survives");
    assert.deepEqual(b.system, [], "ac31: the browser lists no system fonts");
  } finally {
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

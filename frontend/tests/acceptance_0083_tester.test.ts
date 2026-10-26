// Tester cases for `specs/0083-font-info-and-zip` on the frontend: the words of the Info mode
// and the archive notices that the implementer's tests do not pin (criteria 2, 4, 5, 9, 13, 14),
// and, with CURVYO_E2E_BROWSER=/path/to/chromium and the wasm bindings built, the real app in
// Chromium: a zip dropped on the Fonts tab, the notice, IndexedDB version 3 with its documents
// store, the Info mode with its documents (plain text, the 1 MiB cut, a PDF in a blob tab, no
// network request), a reload, Remove with the documents sentence, and a quota failure that keeps
// nothing (criteria 1, 4, 6, 7, 13, 14, 18). Written from the spec before the implementation was
// read. The browser profile lives under `target/`, never outside the checkout.

import assert from "node:assert/strict";
import { spawn, type ChildProcess } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import { crc32 } from "node:zlib";

import { addNoticeWithArchives, freeFolderName } from "../src/lib/fontArchiveText.ts";
import {
  EMPTY_DOCUMENT,
  NOT_STATED,
  SHOWN_FIRST_MIB,
  countText,
  documentsGoing,
  embeddingText,
  infoSubjectLine,
  isPdfName,
  missingInfoText,
  sortedDocuments,
} from "../src/lib/fontInfoText.ts";
import { type ArchiveResult, STORAGE_FULL_NOTICE, armedRemoveLabel } from "../src/lib/fontLibraryText.ts";

const archive = (over: Partial<ArchiveResult>): ArchiveResult => ({
  name: "Lobster.zip",
  added: 0,
  present: 0,
  documents: 0,
  unsafeEntries: 0,
  refused: null,
  ...over,
});

test("criterion 2: Embedding words for combined and reserved bits", () => {
  assert.equal(embeddingText(0x0006, false), "Preview and print", "the least restrictive of two usage bits");
  assert.equal(embeddingText(0x000a, false), "Editable");
  assert.equal(embeddingText(0x0200, false), "Installable, bitmap only");
  assert.equal(embeddingText(0x0100, false), "Installable, no subsetting");
  assert.equal(embeddingText(0x0302, false), "Restricted: may not be embedded, no subsetting, bitmap only");
  assert.equal(embeddingText(0x0001, false), "Installable", "reserved bit 0 is no restriction");
  assert.equal(embeddingText(0, true), "Not stated (SVG font)", "an SVG font never states a permission");
  assert.equal(NOT_STATED, "Not stated");
});

test("criterion 2: Glyphs and Characters counts, criterion 1 subject line, criterion 5 missing", () => {
  assert.equal(countText(412, "character"), "412 characters");
  assert.equal(countText(1, "glyph"), "1 glyph");
  assert.equal(countText(1234, "glyph"), "1,234 glyphs");
  assert.equal(infoSubjectLine("Great Vibes Regular", "Included"), "Great Vibes Regular · Included");
  assert.equal(missingInfoText("Lobster"), "Lobster is missing.");
});

test("criterion 4: documents A to Z ignoring case, PDFs by extension, the cut line", () => {
  const docs = sortedDocuments([{ name: "README.md" }, { name: "b.txt" }, { name: "OFL.txt" }, { name: "a.PDF" }, { name: "Z" }]);
  assert.deepEqual(docs.map((d) => d.name), ["a.PDF", "b.txt", "OFL.txt", "README.md", "Z"]);
  assert.equal(isPdfName("Licence.PDF"), true);
  assert.equal(isPdfName("pdf.txt"), false);
  assert.equal(isPdfName("README"), false);
  assert.equal(SHOWN_FIRST_MIB, "Shown: the first 1 MiB.");
  assert.equal(EMPTY_DOCUMENT, "This file is empty.");
});

test("criterion 9: the folder -N rule matches the Rust table of the tester", () => {
  assert.equal(freeFolderName("Lobster", []), "Lobster");
  assert.equal(freeFolderName("Lobster", ["LOBSTER", "lobster-2"]), "Lobster-3");
  assert.equal(freeFolderName("Ärger", ["ärger"]), "Ärger-2", "Unicode lower case");
});

test("criterion 13: singular forms, every part at most 70 characters, a long archive name", () => {
  const only = (a: ArchiveResult) => addNoticeWithArchives({ added: [], present: [], notFonts: [], archives: [a] }, null).text;
  assert.equal(only(archive({ present: 1 })), "Lobster.zip: 1 font was already there.");
  assert.equal(only(archive({ added: 1, unsafeEntries: 1 })), "Lobster.zip: 1 font added. 1 file was skipped as unsafe.");
  const long = `${"A-very-long-foundry-download-name-".repeat(4)}.zip`;
  for (const a of [archive({ name: long, added: 12, documents: 30 }), archive({ name: long, refused: "over_limit" }), archive({ name: long })]) {
    const text = only(a);
    for (const part of text.split(/(?<=\.) /)) {
      assert.ok(part.length <= 70, `${part.length}: ${part}`);
    }
    assert.ok(!/[!]/.test(text), "no exclamation");
  }
  const refused = only(archive({ name: long, refused: "unreadable" }));
  assert.match(refused, /could not be read\. Nothing was added\.$/, "the reason survives a long name");
});

test("criterion 13: archives and loose files combined", () => {
  const notice = addNoticeWithArchives(
    { added: ["Loose.ttf"], present: [], notFonts: [], archives: [archive({ added: 4, documents: 4, unsafeEntries: 2 })] },
    null,
  );
  assert.equal(notice.text, "5 fonts added. 4 documents kept. 2 files were skipped as unsafe.");
});

test("criterion 14: a family spread over two emptied folders takes both folders' documents", () => {
  const faces = [
    { family: "Lobster", folder: "Lobster" },
    { family: "Lobster", folder: "Lobster-2" },
  ];
  const folders = [
    { name: "Lobster", documents: [1, 2] },
    { name: "Lobster-2", documents: [1] },
    { name: "Empty", documents: [1] },
  ];
  assert.equal(documentsGoing("Lobster", faces, folders), 3);
  assert.equal(armedRemoveLabel("Lobster", 1, 3), "Press again to remove Lobster from Curvyo fonts. Its 3 documents go too. 1 text here uses it.");
});

// ---------------------------------------------------------------------------------------------
// Criteria 1, 4, 6, 7, 13, 14, 18 in Chromium
// ---------------------------------------------------------------------------------------------

const BROWSER = process.env.CURVYO_E2E_BROWSER;
const PORT = 5196;
const ORIGIN = `http://localhost:${PORT}`;
const FRONTEND = path.resolve(import.meta.dirname, "..");
const SCRATCH = path.resolve(FRONTEND, "..", "target", "e2e-0083");
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

/** A stored (uncompressed) zip of `entries`. */
function storedZip(entries: { name: string; bytes: Uint8Array }[]): Uint8Array {
  const parts: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const e of entries) {
    const name = Buffer.from(e.name, "utf8");
    const crc = crc32(e.bytes);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt16LE(0x0800, 6);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(e.bytes.length, 18);
    local.writeUInt32LE(e.bytes.length, 22);
    local.writeUInt16LE(name.length, 26);
    parts.push(local, name, Buffer.from(e.bytes));
    const c = Buffer.alloc(46);
    c.writeUInt32LE(0x02014b50, 0);
    c.writeUInt16LE(0x0314, 4);
    c.writeUInt16LE(20, 6);
    c.writeUInt16LE(0x0800, 8);
    c.writeUInt32LE(crc, 16);
    c.writeUInt32LE(e.bytes.length, 20);
    c.writeUInt32LE(e.bytes.length, 24);
    c.writeUInt16LE(name.length, 28);
    c.writeUInt32LE((0o100644 << 16) >>> 0, 38);
    c.writeUInt32LE(offset, 42);
    central.push(c, name);
    offset += 30 + name.length + e.bytes.length;
  }
  const cd = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(cd.length, 12);
  end.writeUInt32LE(offset, 16);
  return new Uint8Array(Buffer.concat([...parts, cd, end]));
}

const NOTO = readFileSync(path.resolve(FRONTEND, "..", "curvyo-text-core/tests/fixtures/vertical/NotoSansJP-Subset.otf"));
const OFL_TEXT = "SIL OPEN FONT LICENSE\n<b>not bold</b><img src=x onerror=\"window.__pwned=1\">\n# not a heading\nlast line";
const text = (s: string) => new Uint8Array(Buffer.from(s, "utf8"));

function goodZip(): Uint8Array {
  return storedZip([
    { name: "NotoSansJP/static/NotoSansJP-Subset.otf", bytes: new Uint8Array(NOTO) },
    { name: "NotoSansJP/OFL.txt", bytes: text(OFL_TEXT) },
    { name: "README.md", bytes: text("# Readme\n*stars*") },
    { name: "Specimen.pdf", bytes: text("%PDF-1.4\n%%EOF\n") },
    { name: "docs/big.txt", bytes: new Uint8Array(1024 * 1024 + 10).fill(0x78) },
    { name: "../evil.txt", bytes: text("evil") },
    { name: "__MACOSX/NotoSansJP/._OFL.txt", bytes: text("junk") },
  ]);
}

/** Every IndexedDB store of the font database: version, store names, keys. */
const IDB_STATE = `(async () => {
  const dbs = await indexedDB.databases();
  const info = dbs.find((d) => d.name === 'curvyo-fonts');
  if (!info) return JSON.stringify({ version: null });
  const db = await new Promise((res, rej) => { const r = indexedDB.open('curvyo-fonts'); r.onsuccess = () => res(r.result); r.onerror = () => rej(r.error); });
  const stores = [...db.objectStoreNames];
  const keys = {};
  for (const s of stores) {
    keys[s] = await new Promise((res) => { const r = db.transaction(s).objectStore(s).getAllKeys(); r.onsuccess = () => res(r.result.map(String).sort()); });
  }
  db.close();
  return JSON.stringify({ version: info.version, stores: stores.sort(), keys });
})()`;

interface Idb {
  version: number | null;
  stores?: string[];
  keys?: Record<string, string[]>;
}

const chromes: ChildProcess[] = [];
process.on("exit", () => chromes.forEach((c) => c.kill("SIGKILL")));

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
  const pending = new Map<number, (v: { result?: { result?: { value?: unknown }; exceptionDetails?: unknown } }) => void>();
  ws.onmessage = (event) => {
    const msg = JSON.parse(String(event.data)) as { id?: number; method?: string; params?: { request?: { url: string } } };
    if (msg.id !== undefined) {
      pending.get(msg.id)?.(msg as never);
    }
    if (msg.method === "Network.requestWillBeSent" && msg.params?.request) {
      requests.push(msg.params.request.url);
    }
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
        e.scrollIntoView({ block: 'center' });
        const b = e.getBoundingClientRect(); return JSON.stringify([b.x + b.width / 2, b.y + b.height / 2]); })()`)) as string,
    ) as [number, number] | null;
    assert.ok(c, `${sel} is on the page`);
    await click(c[0], c[1]);
  };
  const open = async () => {
    await send("Page.navigate", { url: `${ORIGIN}/` });
    await waitFor("the app", async () => ((await evaluate(`!!document.querySelector('button[aria-label="Text tool (T)"]')`)) ? true : null));
    await sleep(1000);
    await evaluate(`[...document.querySelectorAll('[role=tab]')].find((t) => t.getAttribute('aria-label') === 'Fonts' || t.textContent.trim() === 'Fonts').click()`);
    await sleep(500);
  };
  /** Drops `bytes` as `name` on the Fonts tab's tree (the page's own drop path). */
  const drop = async (name: string, bytes: Uint8Array) => {
    const b64 = Buffer.from(bytes).toString("base64");
    await evaluate(`(() => {
      const raw = atob(${JSON.stringify(b64)});
      const data = new Uint8Array(raw.length);
      for (let i = 0; i < raw.length; i++) data[i] = raw.charCodeAt(i);
      const dt = new DataTransfer();
      dt.items.add(new File([data], ${JSON.stringify(name)}, { type: 'application/zip' }));
      const target = document.querySelector('[role=tree]');
      target.dispatchEvent(new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: dt }));
      target.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: dt }));
      return true;
    })()`);
  };
  const statusText = async () =>
    (await evaluate(`[...document.querySelectorAll('[role=status], [role=alert]')].map((e) => e.textContent).join(' | ')`)) as string;
  const idb = async () => JSON.parse((await evaluate(IDB_STATE)) as string) as Idb;
  const close = async () => {
    await sleep(500);
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
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  return { send, evaluate, click, press, clickSel, open, drop, statusText, idb, close, requests, port };
}

const INFO = `JSON.stringify((() => {
  const region = document.querySelector('[role=region][aria-label="Font info"]');
  const pane = document.querySelector('[aria-label="Font preview"]');
  const radios = [...document.querySelectorAll('[role=radiogroup][aria-label="Preview mode"] [role=radio]')];
  const shown = region ? [...region.querySelectorAll('[role=region]')].map((r) => ({ name: r.getAttribute('aria-label'), text: r.textContent, elements: r.querySelectorAll('b, img, h1').length })) : [];
  const dts = region ? [...region.querySelectorAll('dt')].map((d) => d.textContent) : [];
  const values = region ? Object.fromEntries([...region.querySelectorAll('dt')].map((d) => [d.textContent, d.nextElementSibling ? d.nextElementSibling.innerText : null])) : {};
  const docs = region ? [...region.querySelectorAll('dd ul li button')].map((b) => b.innerText.split('\\n')[0]) : [];
  return {
    pane: !!pane,
    paneText: pane ? pane.innerText : '',
    radios: radios.map((r) => r.textContent + '=' + r.getAttribute('aria-checked')),
    region: !!region,
    dts, values, docs, shown,
    pwned: !!window.__pwned,
  };
})())`;

interface InfoState {
  pane: boolean;
  paneText: string;
  radios: string[];
  region: boolean;
  dts: string[];
  values: Record<string, string | null>;
  docs: string[];
  shown: { name: string; text: string; elements: number }[];
  pwned: boolean;
}

test("criteria 1, 4, 6, 7, 13, 14, 18 in Chromium: a zip dropped, Info mode, documents, reload, Remove", { skip: !BROWSER, timeout: 300_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "profile-"));
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    let s = await session(profile);
    await s.open();

    // A text in the default face, selected, so the family row applies to it.
    await s.clickSel('button[aria-label="Text tool (T)"]');
    await s.click(500, 300);
    await sleep(700);
    await s.send("Input.insertText", { text: "Ab" });
    await s.press("Escape", 27);

    const network = s.requests.length;
    await s.drop("NotoSans.zip", goodZip());
    const notice = await waitFor("the add notice", async () => {
      const t = await s.statusText();
      return /NotoSans\.zip/.test(t) ? t : null;
    }, 20_000);
    assert.match(notice, /NotoSans\.zip: 1 font added, 4 documents kept\. 1 file was skipped as unsafe\./, "criterion 13");

    let db = await s.idb();
    assert.equal(db.version, 3, "criterion 18: IndexedDB version 3");
    assert.ok(db.stores?.includes("documents"), `criterion 18: a documents store (${db.stores})`);
    assert.deepEqual(db.keys?.documents, ["NotoSans/OFL.txt", "NotoSans/README.md", "NotoSans/Specimen.pdf", "NotoSans/big.txt"].sort());
    assert.deepEqual(db.keys?.files, ["NotoSans/NotoSansJP-Subset.otf"]);

    // The family row applies the face to the selected text; then the pane in Info mode.
    const labels = (await s.evaluate(`JSON.stringify([...document.querySelectorAll('[role=treeitem]')].map((t) => t.getAttribute('aria-label')))`)) as string;
    assert.ok(labels.includes('"Noto Sans JP, curvyo fonts'), `criterion 13: the new family is listed (${labels})`);
    await s.clickSel('[role=treeitem][aria-label^=\"Noto Sans JP,\"]');
    await s.clickSel('button[aria-label="Font preview"]');
    await s.clickSel('[role=radiogroup][aria-label="Preview mode"] [role=radio]:nth-child(3)');
    let info = await waitFor("Info mode", async () => {
      const st = JSON.parse((await s.evaluate(INFO)) as string) as InfoState;
      return st.region && st.dts.length > 0 ? st : null;
    }, 15_000);
    if (process.env.CURVYO_0083_SHOT) {
      const shot = (await s.send("Page.captureScreenshot", { format: "png" })) as { result?: { data?: string } };
      writeFileSync(path.join(SCRATCH, "info-mode.png"), Buffer.from(shot.result?.data ?? "", "base64"));
    }
    assert.deepEqual(info.radios, ["Fonts=false", "Glyphs=false", "Info=true"], "criterion 1: the third cell");
    assert.match(info.paneText, /Noto Sans JP Regular · Browser storage/, "criterion 1: the subject line");
    assert.deepEqual(
      info.dts,
      ["Full name", "Format", "File", "Version", "Designer", "Vendor", "Copyright", "Trademark", "Licence", "Embedding", "Glyphs", "Characters", "Documents"],
      "criterion 2: the order",
    );
    assert.equal(info.values.Format, "OpenType CFF (OTF)");
    assert.match(info.values.File ?? "", /^NotoSansJP-Subset\.otf · 3 KB\n+Browser storage$/, "criterion 6: no path in the browser");
    assert.ok(!Object.values(info.values).some((v) => v === ""), "never an empty row");
    assert.deepEqual(info.docs, ["big.txt", "OFL.txt", "README.md", "Specimen.pdf"], "criterion 4: A to Z ignoring case");

    // A text document in place, as plain text; its markup is never interpreted.
    await s.clickSel('[role=region][aria-label="Font info"] dd ul li:nth-child(2) button');
    info = JSON.parse((await s.evaluate(INFO)) as string) as InfoState;
    const ofl = info.shown.find((r) => r.name === "OFL.txt");
    assert.ok(ofl, "criterion 4: OFL.txt shown in place");
    assert.ok(ofl.text.includes("<b>not bold</b>") && ofl.text.includes("# not a heading"), "plain text, not HTML or Markdown");
    assert.equal(ofl.elements, 0);
    assert.equal(info.pwned, false);

    // The 1 MiB cut.
    await s.clickSel('[role=region][aria-label="Font info"] dd ul li:nth-child(1) button');
    info = JSON.parse((await s.evaluate(INFO)) as string) as InfoState;
    const big = info.shown.find((r) => r.name === "big.txt");
    assert.ok(big && big.text.endsWith("Shown: the first 1 MiB."), "criterion 4: the cut line");
    assert.equal(big.text.length, 1024 * 1024 + "Shown: the first 1 MiB.".length);

    // Escape closes the shown document first, the pane stays.
    await s.evaluate(`document.querySelector('[role=region][aria-label="big.txt"]').focus()`);
    await s.press("Escape", 27);
    info = JSON.parse((await s.evaluate(INFO)) as string) as InfoState;
    assert.deepEqual(info.shown, [], "criterion 4: Escape closes the document");
    assert.equal(info.pane, true, "the pane stays open");

    // The PDF in a new tab from the stored bytes.
    await s.clickSel('[role=region][aria-label="Font info"] dd ul li:nth-child(4) button');
    await sleep(800);
    const tabs = (await (await fetch(`http://127.0.0.1:${s.port}/json`)).json()) as { url: string }[];
    assert.ok(tabs.some((t) => t.url.startsWith("blob:")), `criterion 4: a blob tab (${tabs.map((t) => t.url)})`);
    const outside = s.requests.slice(network).filter((u) => !u.startsWith(ORIGIN) && !u.startsWith("blob:") && !u.startsWith("data:"));
    assert.deepEqual(outside, [], "criterion 18: no network request");

    // Hint text check (project rule): no operating tip in the pane.
    assert.doesNotMatch(info.paneText, /\b(Tip|Hint|Click|Drag|Drop|Press)\b/, "no hint text in the Info pane");

    // Reload: fonts and documents survive (criterion 18).
    await s.close();
    s = await session(profile);
    await s.open();
    db = await s.idb();
    assert.equal(db.keys?.documents?.length, 4, "criterion 18: documents survive a browser restart");
    const listed = JSON.parse(
      (await s.evaluate(`(async () => { const h = await import('/src/platform/host.ts'); return JSON.stringify(await h.invoke('font_folder_list')); })()`)) as string,
    ) as { faces: { family: string; folder?: string }[]; folders: { name: string; documents: { name: string }[] }[] };
    assert.equal(listed.faces.find((f) => f.family === "Noto Sans JP")?.folder, "NotoSans");
    assert.equal(listed.folders.find((f) => f.name === "NotoSans")?.documents.length, 4);

    // Remove: the armed trash names the documents; afterwards nothing is stored.
    const trash = '[role=treeitem][aria-label^=\"Noto Sans JP,\"] button[data-arm-trash]';
    await s.clickSel(trash);
    const armed = (await s.evaluate(`document.querySelector(${JSON.stringify(trash)}).getAttribute('aria-label')`)) as string;
    assert.match(armed, /^Press again to remove Noto Sans JP from Curvyo fonts\. Its 4 documents go too\./, "criterion 14");
    await sleep(500);
    await s.clickSel(trash);
    await sleep(800);
    db = await s.idb();
    assert.deepEqual(db.keys?.files, [], "criterion 14: the font is gone");
    assert.deepEqual(db.keys?.documents, [], "criterion 14: the folder's documents go with its last font");
    await s.close();
  } finally {
    chromes.forEach((c) => c.kill("SIGKILL"));
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

test("criterion 18 in Chromium: a quota error keeps nothing of the archive and shows the 0075 notice", { skip: !BROWSER, timeout: 180_000 }, async () => {
  let vite: ChildProcess | null = null;
  mkdirSync(SCRATCH, { recursive: true });
  const profile = mkdtempSync(path.join(SCRATCH, "quota-"));
  try {
    vite = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], { cwd: FRONTEND, stdio: "ignore" });
    await waitFor("vite", async () => ((await fetch(ORIGIN)).ok ? true : null));
    const s = await session(profile);
    await s.open();
    // The storage is full when the documents are put: the transaction aborts with a
    // QuotaExceededError, as a real full quota does (the earlier puts of the same transaction
    // are rolled back by IndexedDB itself, or not, if they were committed separately).
    await s.evaluate(`(() => {
      const original = IDBDatabase.prototype.transaction;
      IDBDatabase.prototype.transaction = function (...args) {
        const tx = original.apply(this, args);
        const store = tx.objectStore.bind(tx);
        tx.objectStore = (name) => {
          const os = store(name);
          if (name === 'documents' && tx.mode === 'readwrite') {
            const put = os.put.bind(os);
            os.put = (value) => {
              const req = put(value);
              Object.defineProperty(tx, 'error', { value: new DOMException('full', 'QuotaExceededError') });
              tx.abort();
              return req;
            };
          }
          return os;
        };
        return tx;
      };
      return true;
    })()`);
    const zip = storedZip([
      { name: "NotoSansJP-Subset.otf", bytes: new Uint8Array(NOTO) },
      { name: "Huge.txt", bytes: new Uint8Array(4 * 1024 * 1024).fill(0x61) },
    ]);
    await s.drop("Full.zip", zip);
    const notice = await waitFor("a notice", async () => {
      const t = await s.statusText();
      return /room|Full\.zip/.test(t) ? t : null;
    }, 20_000);
    assert.match(notice, new RegExp(STORAGE_FULL_NOTICE.replace(/\./g, "\\.")), "the 0075 criterion 30 notice");
    const db = await s.idb();
    for (const store of ["files", "faces", "documents"]) {
      assert.deepEqual(db.keys?.[store] ?? [], [], `${store}: nothing kept`);
    }
    await s.close();
  } finally {
    chromes.forEach((c) => c.kill("SIGKILL"));
    vite?.kill("SIGTERM");
    rmSync(profile, { recursive: true, force: true });
  }
});

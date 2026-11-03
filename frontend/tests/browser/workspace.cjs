// Browser journey of `specs/0094-workspace` slice 1 (plan task 27): a local `curvyo-sync-server`
// on a temporary data directory and two browser contexts, A and B, each its own device with a
// faked desktop host (`window.__TAURI_INTERNALS__`, as `acceptance_0087_browser.test.ts`) whose
// workspace commands run on real folders under target/0094-browser/. A opens an empty folder as a
// workspace, creates documents, shares one of them on its own (`0092`) and then the whole
// workspace: the Share confirm names the document taken in (criteria 25, 42). B joins by the `2.`
// link into an empty folder (26 to 28); A's create, rename, move and delete reach B's disk with
// notices naming Anna, B restores the deleted document for both (35 to 40); a live edit in one
// open document reaches the other side (30); A removes B, whose workspace turns local (43, 44).
//
// The fake host mirrors `curvyo-app`'s workspace commands closely enough for the page; their
// Rust side is covered by `curvyo-storage-io` and `curvyo-app` tests, not here.
//
// Needs `target/debug/curvyo-sync-server` (`cargo build -p curvyo-sync-server`) and a production
// preview (`npm run build`, `npx vite preview --host 127.0.0.1 --port 1421`); starts the preview
// itself unless CURVYO_BROWSER_URL is set. Screenshots: target/0094-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn, execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0094-browser');
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
const children = [];
process.on('exit', () => {
  for (const child of children) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* gone */ }
  }
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const started = Date.now();
const step = (text) => console.log(`[${((Date.now() - started) / 1000).toFixed(1)} s] ${text}`);

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then((r) => r.ok, () => false)) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

/** Starts the relay on a free port of 127.0.0.1 with a fresh data directory; its address. */
async function startRelay() {
  const data = path.join(out, 'relay-data');
  fs.mkdirSync(data);
  const binary = path.join(root, 'target/debug/curvyo-sync-server');
  const relay = spawn(binary, ['--listen', '127.0.0.1:0', '--data', data], { detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
  children.push(relay);
  let log = '';
  const address = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`relay did not start: ${log}`)), 15000);
    const read = (chunk) => {
      log += chunk;
      const match = /listening on (\S+)/.exec(log);
      if (match) { clearTimeout(timer); resolve(match[1]); }
    };
    relay.stdout.on('data', read);
    relay.stderr.on('data', read);
  });
  await waitFor(`http://${address}/healthz`);
  return { address, data, log: () => log };
}

// ---- the fake desktop host ----------------------------------------------------------------------

/** Zip reads the host does without opening a document (`curvyo-storage-io`'s probe and sharing
 * members): `probe` prints the thumbnail as hex or an error code, `members` the share.json text
 * and whether a keyring is there, `json` the document's objects. */
const ZIP_PY = `
import json, sys, zipfile
op, path = sys.argv[1], sys.argv[2]
try:
    z = zipfile.ZipFile(path)
except Exception:
    print(json.dumps({"error": "not-curvyo"})); sys.exit(0)
names = z.namelist()
if op == "probe":
    if "manifest.json" not in names:
        print(json.dumps({"error": "damaged"}))
    else:
        print(json.dumps({"thumbnail": z.read("thumbnail.png").hex() if "thumbnail.png" in names else ""}))
elif op == "members":
    share = z.read("share.json").decode() if "share.json" in names else None
    print(json.dumps({"share": share, "keyring": "keyring.log" in names}))
elif op == "json":
    print(z.read("document.json").decode())
`;
const zip = (op, file) => JSON.parse(execFileSync('python3', ['-c', ZIP_PY, op, file], { encoding: 'utf8' }));

const MARKER = '.curvyo-workspace';
const hex = (bytes) => Buffer.from(bytes).toString('hex');

/** One device's host: its workspace root, project path, keys and folders on disk. */
function makeHost(label) {
  const home = path.join(out, label);
  const trash = path.join(home, 'trash');
  fs.mkdirSync(trash, { recursive: true });
  const state = { root: null, project: null, keys: new Map(), identity: crypto.randomBytes(64).toString('hex'), joinFolder: null, saves: 0, calls: [] };
  const at = (rel) => {
    for (const part of rel) {
      if (!part || part === '.' || part === '..' || part.includes('/') || part.startsWith('.')) throw 'outside';
    }
    if (!state.root) throw 'not-found';
    return path.join(state.root, ...rel);
  };
  const marker = (name) => path.join(state.root, MARKER, name);
  const scan = () => {
    const found = [];
    const walk = (dir, rel) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        if (entry.name.startsWith('.') || entry.isSymbolicLink()) continue;
        const full = path.join(dir, entry.name);
        const stat = fs.statSync(full);
        if (entry.isDirectory()) {
          found.push({ path: [...rel, entry.name], kind: 'folder', size: 0, modified: Math.floor(stat.mtimeMs / 1000) });
          walk(full, [...rel, entry.name]);
        } else if (entry.name.toLowerCase().endsWith('.curvyo')) {
          found.push({ path: [...rel, entry.name], kind: 'document', size: stat.size, modified: Math.floor(stat.mtimeMs / 1000) });
        }
      }
    };
    walk(state.root, []);
    return found;
  };
  const writeMarker = (id) => {
    fs.mkdirSync(path.join(state.root, MARKER), { recursive: true });
    fs.writeFileSync(marker('workspace.toml'), `format_version = 1\nid = "${id}"\n`);
  };
  const open = (folder, id) => {
    state.root = folder;
    if (!fs.existsSync(path.join(folder, MARKER, 'workspace.toml'))) writeMarker(id ?? crypto.randomBytes(16).toString('hex'));
    return { kind: 'opened', name: path.basename(folder), entries: scan() };
  };
  const readIndex = () => {
    try {
      return JSON.parse(fs.readFileSync(marker('index.json'), 'utf8')).documents;
    } catch {
      return {};
    }
  };
  const create = (file, bytes) => {
    if (fs.existsSync(file)) throw 'exists';
    fs.writeFileSync(file, bytes);
  };
  const toTrash = (file) => fs.renameSync(file, path.join(trash, `${Date.now()}-${crypto.randomBytes(3).toString('hex')}-${path.basename(file)}`));
  const kept = (document) => path.join(state.root, MARKER, 'deleted', `${document}.curvyo`);
  const commands = {
    take_pending_open: () => null,
    font_folder_list: () => ({ faces: [], skipped: [] }),
    system_font_list: () => [],
    read_device_keys: (a) => ({ status: 'ok', identity: state.identity, keys: state.keys.get(a.room) ?? [] }),
    write_document_keys: (a) => { state.keys.set(a.room, a.keys); return null; },
    recent_workspaces: () => [],
    remove_recent_workspace: () => [],
    pick_workspace: () => {
      const folder = path.join(home, 'Box job');
      fs.mkdirSync(folder, { recursive: true });
      return open(folder);
    },
    close_workspace: () => { state.root = null; return null; },
    scan_open_workspace: () => scan(),
    apply_workspace_op: (a, bytes) => {
      const op = a.op;
      if (op.kind === 'create-document') create(at(op.path), bytes ?? Buffer.alloc(0));
      else if (op.kind === 'create-folder') fs.mkdirSync(at(op.path));
      else if (op.kind === 'move') {
        const to = at(op.to);
        if (fs.existsSync(to)) throw 'exists';
        fs.renameSync(at(op.from), to);
        if (state.project && path.join(...state.project) === path.join(...op.from)) state.project = op.to;
      } else if (op.kind === 'trash') toTrash(at(op.path));
      return null;
    },
    read_workspace_document: (a) => {
      const file = at(a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      return { raw: fs.readFileSync(file) };
    },
    probe_workspace_document: (a) => {
      const answer = zip('probe', at(a.path));
      if (answer.error) throw answer.error;
      return { raw: Buffer.from(answer.thumbnail, 'hex') };
    },
    confirm_workspace_document_opened: (a) => { state.project = a.path; return null; },
    reset_project: () => { state.project = null; return null; },
    save_project_bytes: (a) => {
      const bytes = Buffer.from(a.bytes);
      if (state.project && state.root && !a.saveAs) {
        fs.writeFileSync(at(state.project), bytes);
        return { kind: 'saved', file_name: state.project[state.project.length - 1], path: at(state.project) };
      }
      state.saves += 1;
      const file = path.join(home, `save-${state.saves}.curvyo`);
      fs.writeFileSync(file, bytes);
      return { kind: 'saved', file_name: a.fileName, path: file };
    },
    workspace_marker_share: () => {
      if (!state.root || !fs.existsSync(marker('share.json'))) return null;
      const read = (name) => Array.from(fs.readFileSync(marker(name)));
      return { share_json: read('share.json'), keyring_log: read('keyring.log'), list: read('list.loro') };
    },
    write_workspace_marker_share: (a) => {
      fs.writeFileSync(marker('share.json'), Buffer.from(a.marker.share_json));
      fs.writeFileSync(marker('keyring.log'), Buffer.from(a.marker.keyring_log));
      fs.writeFileSync(marker('list.loro'), Buffer.from(a.marker.list));
      return null;
    },
    write_workspace_id: (a) => { writeMarker(a.id); return null; },
    replace_workspace_document: (a, bytes) => {
      const file = at(a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      fs.writeFileSync(file, bytes);
      return null;
    },
    write_workspace_index: (a) => {
      const documents = {};
      for (const { document, path: rel } of a.documents) {
        const stat = fs.statSync(at(rel), { throwIfNoEntry: false });
        documents[document] = { path: rel, size: stat?.size ?? 0, modified: stat ? Math.floor(stat.mtimeMs / 1000) : 0 };
      }
      fs.writeFileSync(marker('index.json'), JSON.stringify({ format_version: 1, documents }));
      return null;
    },
    pick_join_folder: () => {
      state.joinFolder = path.join(home, 'Joined box');
      fs.mkdirSync(state.joinFolder, { recursive: true });
      return { kind: 'picked', path: state.joinFolder };
    },
    start_joined_workspace: (a) => open(state.joinFolder, a.id),
    scan_shared_workspace: () => {
      const entries = scan();
      const documents = entries.filter((e) => e.kind === 'document').map((e) => {
        const members = zip('members', at(e.path));
        return { path: e.path, share: members.share ?? null, keyring: !!members.keyring };
      });
      const known = Object.entries(readIndex()).map(([document, entry]) => ({ document, path: entry.path }));
      return { entries, folders: entries.filter((e) => e.kind === 'folder').map((e) => e.path), documents, known };
    },
    keep_workspace_document: (a) => {
      fs.mkdirSync(path.dirname(kept(a.document)), { recursive: true });
      fs.copyFileSync(at(a.path), kept(a.document));
      if (a.trash) toTrash(at(a.path));
      return null;
    },
    restore_workspace_document: (a) => {
      if (!fs.existsSync(kept(a.document))) throw 'not-found';
      create(at(a.path), fs.readFileSync(kept(a.document)));
      return null;
    },
    read_deleted_document: (a) => {
      if (!fs.existsSync(kept(a.document))) throw 'not-found';
      return { raw: fs.readFileSync(kept(a.document)) };
    },
    write_deleted_document: (a, bytes) => {
      fs.mkdirSync(path.dirname(kept(a.document)), { recursive: true });
      fs.writeFileSync(kept(a.document), bytes);
      return null;
    },
    clear_workspace_marker_share: () => {
      for (const name of ['share.json', 'keyring.log', 'list.loro', 'index.json']) fs.rmSync(marker(name), { force: true });
      return null;
    },
  };
  return {
    state,
    home,
    /** One command from the page: `{value}`, `{raw: hex}` or `{error}`. */
    handle(cmd, argsJson, bytesHex) {
      state.calls.push(cmd);
      const command = commands[cmd];
      if (!command) return { value: null };
      try {
        const answer = command(JSON.parse(argsJson), bytesHex == null ? null : Buffer.from(bytesHex, 'hex'));
        if (answer && answer.raw) return { raw: answer.raw.toString('hex') };
        return { value: answer ?? null };
      } catch (error) {
        return { error: typeof error === 'string' ? error : String(error) };
      }
    },
  };
}

/** The page side of the fake host: Tauri's internals, forwarding every command to `__host`. Raw
 * bodies travel as hex with their arguments from the `curvyo-args` header (`raw_ipc`); a raw
 * answer comes back as an `ArrayBuffer`. */
const FAKE_HOST = `(() => {
  let next = 1;
  const toHex = (bytes) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  const fromHex = (text) => {
    const bytes = new Uint8Array(text.length / 2);
    for (let i = 0; i < bytes.length; i += 1) bytes[i] = parseInt(text.slice(2 * i, 2 * i + 2), 16);
    return bytes.buffer;
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
    transformCallback: () => next++,
    unregisterCallback: () => undefined,
    convertFileSrc: (p) => p,
    invoke: async (cmd, args, options) => {
      if (cmd.startsWith('plugin:')) return cmd === 'plugin:event|listen' ? next++ : null;
      let plain = args ?? {};
      let bytes = null;
      if (args instanceof Uint8Array || args instanceof ArrayBuffer) {
        bytes = toHex(new Uint8Array(args));
        const headers = options && options.headers;
        const text = headers instanceof Headers ? headers.get('curvyo-args') : headers && headers['curvyo-args'];
        plain = JSON.parse(text || '{}');
      }
      const answer = await window.__host(cmd, JSON.stringify(plain), bytes);
      if (answer.error !== undefined) throw answer.error;
      if (answer.raw !== undefined) return fromHex(answer.raw);
      return answer.value;
    },
  };
})()`;

// ---- the journey -------------------------------------------------------------------------------

const settle = (page, ms = 120) => page.waitForTimeout(ms);
/** Polls `check` until it holds; returns the milliseconds it took. */
async function until(what, check, timeout = 10000) {
  const start = Date.now();
  for (;;) {
    if (await check()) return Date.now() - start;
    if (Date.now() - start > timeout) throw new Error(`timed out after ${timeout} ms: ${what}`);
    await new Promise((r) => setTimeout(r, 100));
  }
}

async function device(browser, url, label) {
  const host = makeHost(label);
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(`${label}: ${e.stack || e.message}`));
  await page.exposeFunction('__host', (cmd, args, bytes) => host.handle(cmd, args, bytes));
  await page.addInitScript(FAKE_HOST);
  await page.goto(url, { waitUntil: 'networkidle' });
  const canvas = await page.locator('canvas').first().boundingBox();
  return { context, page, errors, label, canvas, host };
}

const panel = (d) => d.page.locator('[role=tabpanel]');
const tab = async (d, name) => { await d.page.getByRole('tab', { name: new RegExp(`^${name}`) }).click(); await settle(d.page); };
const tool = async (d, name) => { await d.page.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(d.page); };
const at = (d, x, y) => ({ x: d.canvas.x + x, y: d.canvas.y + y });
const tree = (d) => d.page.getByRole('treegrid', { name: 'Workspace documents' });
const row = (d, name) => tree(d).getByRole('row', { name: new RegExp(`^${name}\\b`) });
const participants = (d) => d.page.getByRole('list', { name: 'Participants' }).locator('li');
const shot = (d, name) => d.page.screenshot({ path: path.join(out, `${name}.png`) });
const exists = (d, ...rel) => fs.existsSync(path.join(d.host.state.root, ...rel));
const objects = (d, ...rel) => {
  const flat = (nodes) => nodes.flatMap((n) => (n.shape === 'layer' || n.shape === 'group' ? flat(n.objects || []) : [n]));
  return flat(zip('json', path.join(d.host.state.root, ...rel)).objects);
};
const noticeShown = (d, text) => d.page.getByText(text, { exact: true }).count().then((n) => n > 0);

async function rect(d, x, y, w, h) {
  await tool(d, 'Rectangle');
  const p = at(d, x, y);
  const q = at(d, x + w, y + h);
  await d.page.mouse.move(p.x, p.y);
  await d.page.mouse.down();
  await d.page.mouse.move(q.x, q.y, { steps: 5 });
  await d.page.mouse.up();
  await settle(d.page);
}

async function save(d) {
  await tab(d, 'File');
  await panel(d).getByRole('button', { name: 'Save', exact: true }).click();
  await settle(d.page, 300);
}

/** New document or New folder in the open workspace's header, named in the inline field. */
async function newEntry(d, kind, name) {
  await tab(d, 'File');
  // The header's icon button (an empty workspace also shows a text button of the same name).
  await panel(d).getByRole('region', { name: 'Workspace' }).getByLabel(kind, { exact: true }).click();
  const field = panel(d).getByRole('textbox', { name: kind === 'New folder' ? 'Name of the new folder' : 'Name of the new document' });
  await field.fill(name);
  await field.press('Enter');
  await settle(d.page, 400);
}

(async () => {
  const relay = await startRelay();
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const preview = spawn('npx', ['vite', 'preview', '--host', '127.0.0.1', '--port', '1421', '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    children.push(preview);
    url = 'http://127.0.0.1:1421/';
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const timings = {};
  let A = null;
  let B = null;
  try {
    A = await device(browser, url, 'A');
    B = await device(browser, url, 'B');

    step('Criteria 1 and 13: A opens an empty folder and creates documents');
    await tab(A, 'File');
    await panel(A).getByRole('button', { name: 'Open workspace…', exact: true }).click();
    await tree(A).waitFor();
    assert(exists(A, MARKER, 'workspace.toml'), 'the marker is written');
    await newEntry(A, 'New document', 'Lid');
    await rect(A, 300, 200, 120, 80);
    await save(A);
    await newEntry(A, 'New document', 'Base');
    await rect(A, 300, 200, 60, 60);
    await save(A);
    await newEntry(A, 'New document', 'Solo');
    await rect(A, 400, 300, 50, 50);
    await save(A);
    assert.equal(objects(A, 'Lid.curvyo').length, 1);
    assert.equal(objects(A, 'Base.curvyo').length, 1);

    step('Criterion 42 setup: Solo is shared on its own');
    await tab(A, 'Share');
    const server = panel(A).getByLabel('Server', { exact: true });
    await server.fill(`ws://${relay.address}`);
    await server.press('Enter');
    const name = panel(A).getByLabel('Name shown to others', { exact: true });
    await name.fill('Anna');
    await name.press('Enter');
    await panel(A).getByText('Reachable', { exact: true }).first().waitFor();
    await panel(A).getByRole('button', { name: 'Share', exact: true }).click();
    await panel(A).getByRole('group', { name: /^Upload this document to/ }).getByRole('button', { name: 'Share', exact: true }).click();
    await panel(A).getByText(/^Shared on /).first().waitFor({ timeout: 10000 });
    await save(A);
    await until('Solo carries its own keyring', async () => zip('members', path.join(A.host.state.root, 'Solo.curvyo')).keyring);

    step('Criteria 25 and 42: Share workspace names Solo beforehand and takes it in');
    await tab(A, 'Share');
    await panel(A).getByRole('button', { name: 'Share workspace', exact: true }).click();
    const confirm = panel(A).getByRole('group', { name: /^Upload the workspace Box job \(3 documents\)/ });
    await confirm.waitFor();
    assert.match(await confirm.innerText(), /Solo is shared on its own\. It becomes part of the workspace and its own link stops working\./);
    await shot(A, '01-a-share-confirm');
    await confirm.getByRole('button', { name: 'Share', exact: true }).click();
    await until('A shared the workspace', async () => {
      await tab(A, 'Share');
      return (await panel(A).getByText(/^Shared on /).count()) >= 1;
    }, 30000);
    await until('Solo is a workspace document', async () => {
      const members = zip('members', path.join(A.host.state.root, 'Solo.curvyo'));
      return !members.keyring && members.share && JSON.parse(members.share).format_version === 2;
    }, 30000);
    await shot(A, '02-a-shared');

    step('Criterion 26: a 2. invite');
    await panel(A).getByRole('button', { name: 'Invite', exact: true }).click();
    await panel(A).getByRole('button', { name: 'Create link', exact: true }).click();
    const linkField = panel(A).getByLabel('Invite link', { exact: true });
    await linkField.waitFor();
    const link = await linkField.inputValue();
    assert.match(link, /^curvyo:\/\/join\?server=[^#]+#2\.[A-Za-z0-9_-]+$/, link);
    assert.match(await panel(A).innerText(), /Anyone with this link can edit every document of this workspace once, until it expires\./);

    step('Criteria 27 and 28: B joins into an empty folder and receives every document');
    await tab(B, 'Share');
    await panel(B).getByRole('button', { name: /^Join shared workspace/ }).click();
    await panel(B).getByLabel('Invite link', { exact: true }).fill(link);
    const bName = panel(B).getByLabel('Name shown to others', { exact: true }).last();
    await bName.fill('Ben');
    await bName.press('Enter');
    await panel(B).getByRole('button', { name: /^Choose folder/ }).click();
    const joinStart = Date.now();
    await panel(B).getByRole('button', { name: 'Join', exact: true }).click();
    await until('B has every file', async () => B.host.state.root !== null
      && ['Lid.curvyo', 'Base.curvyo', 'Solo.curvyo'].every((f) => exists(B, f)), 30000);
    timings.join = Date.now() - joinStart;
    await until('B lists two participants', async () => {
      await tab(B, 'Share');
      return (await participants(B).count()) === 2;
    }, 15000);
    await until('A lists B', async () => (await participants(A).count()) === 2, 15000);
    assert.equal(objects(B, 'Lid.curvyo').length, 1, 'B holds A\'s Lid');
    await tab(B, 'File');
    await until('B\'s tree shows the documents', async () => (await row(B, 'Solo').count()) === 1);
    await shot(B, '03-b-joined');

    step('Criterion 35: a document A creates reaches B');
    let start = Date.now();
    await newEntry(A, 'New document', 'Cover');
    await until('Cover on B\'s disk', async () => exists(B, 'Cover.curvyo'), 15000);
    timings.create = Date.now() - start;

    step('Criterion 36: a rename reaches B with a notice naming Anna');
    await tab(A, 'File');
    await row(A, 'Base').click();
    await settle(A.page, 400);
    await row(A, 'Base').press('F2');
    const renameField = panel(A).getByRole('textbox').first();
    await renameField.fill('Bottom');
    await renameField.press('Enter');
    start = Date.now();
    await until('Bottom on B\'s disk', async () => exists(B, 'Bottom.curvyo') && !exists(B, 'Base.curvyo'), 15000);
    timings.rename = Date.now() - start;
    await until('B is told by name', () => noticeShown(B, 'Anna renamed Base to Bottom.'), 8000);

    step('Criterion 37: a move into a new folder reaches B');
    await newEntry(A, 'New folder', 'Sides');
    await until('Sides on B\'s disk', async () => exists(B, 'Sides'), 15000);
    await row(A, 'Lid').click();
    await settle(A.page, 300);
    await row(A, 'Lid').press('Control+x');
    await row(A, 'Sides').press('Control+v');
    start = Date.now();
    await until('Sides/Lid on B\'s disk', async () => exists(B, 'Sides', 'Lid.curvyo') && !exists(B, 'Lid.curvyo'), 15000);
    timings.move = Date.now() - start;
    await until('B is told of the move', () => noticeShown(B, 'Anna moved Lid to Sides.'), 8000);
    await shot(B, '04-b-after-move');

    step('Criterion 40: a deletion goes to B\'s trash, B restores it for both');
    await row(A, 'Cover').click();
    await settle(A.page, 300);
    await row(A, 'Cover').press('Delete');
    const ask = panel(A).getByRole('group', { name: /Cover/ });
    await ask.waitFor();
    await settle(A.page, 600);
    await ask.getByRole('button', { name: 'Delete', exact: true }).click();
    await until('Cover gone from B\'s disk', async () => !exists(B, 'Cover.curvyo'), 15000);
    assert(fs.readdirSync(path.join(B.host.home, 'trash')).some((f) => f.endsWith('Cover.curvyo')), 'B\'s file went to its trash');
    await until('B is told who deleted', () => noticeShown(B, 'Anna deleted Cover.'), 8000);
    await tab(B, 'File');
    await panel(B).getByRole('button', { name: /^Deleted/ }).click();
    await panel(B).getByRole('button', { name: 'Restore Cover', exact: true }).click();
    await until('Cover back on A\'s disk', async () => exists(A, 'Cover.curvyo'), 15000);
    await until('Cover back on B\'s disk', async () => exists(B, 'Cover.curvyo'), 15000);
    await shot(B, '05-b-restored');

    step('Criterion 30: a live edit in an open document');
    await tab(A, 'File');
    await row(A, 'Bottom').click();
    await settle(A.page, 800);
    await tab(B, 'File');
    await row(B, 'Bottom').click();
    await settle(B.page, 800);
    await rect(A, 600, 400, 80, 40);
    start = Date.now();
    await until('B holds A\'s new rectangle', async () => {
      await save(B);
      return objects(B, 'Bottom.curvyo').length === 2;
    }, 15000);
    timings.liveEdit = Date.now() - start;
    await rect(B, 200, 500, 40, 40);
    await until('A holds B\'s rectangle', async () => {
      await save(A);
      return objects(A, 'Bottom.curvyo').length === 3;
    }, 15000);
    await shot(B, '06-b-live-edit');

    step('Criteria 43 and 44: A removes B; B\'s workspace turns local');
    await tab(A, 'Share');
    await panel(A).getByRole('button', { name: 'Remove Ben', exact: true }).click();
    const removeAsk = panel(A).getByRole('group', { name: /^Remove Ben/ });
    await removeAsk.waitFor();
    await settle(A.page, 600);
    await removeAsk.getByRole('button', { name: 'Remove', exact: true }).click();
    await until('A lists only itself', async () => (await participants(A).count()) === 1, 15000);
    await until('B told', async () => (await B.page.getByText('You were removed from this workspace. Your copies stay on this computer.').count()) >= 1, 15000);
    await until('B\'s marker share files are gone', async () => !exists(B, MARKER, 'share.json'), 8000);
    await tab(B, 'Share');
    await until('B\'s Share tab says Not shared', async () => (await panel(B).getByText('Not shared', { exact: true }).count()) >= 1, 8000);
    assert.equal(await panel(B).getByRole('button', { name: 'Share workspace', exact: true }).count(), 1, 'Share workspace is available again');
    await shot(B, '07-b-removed');
    for (const f of ['Bottom.curvyo', 'Solo.curvyo', 'Cover.curvyo']) assert(exists(B, f), `B keeps ${f}`);

    assert.deepEqual([...A.errors, ...B.errors], []);
    console.log(`PASS browser 0094: open, create, share with take-in, invite 2., join, create/rename/move/delete/restore both ways, live edit, removal; timings ${JSON.stringify(timings)}`);
  } catch (error) {
    for (const d of [A, B]) {
      if (d) {
        await shot(d, `failed-${d.label}`).catch(() => undefined);
        console.error(`${d.label} panel: ${await panel(d).innerText().catch(() => '?')}`);
        console.error(`${d.label} errors: ${d.errors.join(' | ')}`);
        console.error(`${d.label} host calls: ${d.host.state.calls.filter((c) => !c.startsWith('scan_')).slice(-40).join(', ')}`);
      }
    }
    console.error(`relay log: ${relay.log()}`);
    throw error;
  } finally {
    await browser.close();
  }
  // Criterion 34 in the journey's own run: no document or folder name and no label in the
  // relay's data directory or log.
  const files = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)]));
  const stored = [...files(relay.data).map((f) => fs.readFileSync(f)), Buffer.from(relay.log())];
  for (const needle of ['Anna', 'Bottom', 'Sides', 'Solo', '"objects"']) {
    assert(stored.every((bytes) => !bytes.includes(Buffer.from(needle))), `the relay holds "${needle}"`);
  }
  console.log(`relay data directory ${relay.data}: ${stored.length - 1} files, no plaintext found`);
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

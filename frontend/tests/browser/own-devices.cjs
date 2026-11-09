// Browser journey of `specs/0096-own-devices-sync` slice 1 (plan task 19, M10): a local
// `curvyo-sync-server` on a temporary data directory and two browser contexts, A and B, each a
// desktop device with a faked desktop host (`window.__TAURI_INTERNALS__`, as `workspace.cjs`)
// whose workspace and device-group commands run on real folders under target/0096-browser/.
//
// A opens a workspace with a document; Add a device shows a code, B types it in Join my devices,
// both show the same four check words and A presses Pair (criteria 1 to 3). A keeps the open
// workspace checked and presses Sync these: it arrives on B under the Synced workspaces folder
// without any input there (8 to 10). B opens it; an edit on either side reaches the other (18).
// A switches the workspace off (armed, B is told, 13); B edits its copy and closes
// it; A switches on again and B's Connect this folder sends B's edit to A without B opening the
// document (14). A removes B: B is told and keeps its files (23, 24), and A, now the only device
// of a synced workspace, shows the one-device warning (28). The server's data holds none of the
// names (30). PR #281 feedback: the removed B pairs again with the same identity and is listed
// again on both; the server stops before Sync these of two closed workspaces, one alert names
// both, and it goes once the restarted server lets them in; the Share tab fits its panel in every
// state shown (`support/panel-overflow.cjs`); the run prints how long each change took. Share tab
// feedback (2026-11-09): the Server row's Try again; a switch-on whose server never answers turns
// off again at once, fails with Try again after its bound and switches on once the server is up;
// Remove, Leave my devices and the switch-off are armed; the last device may leave and every
// workspace it synced reopens local, never refused as "not a participant". Browser pairing and
// the passkey backup are `key-backup.cjs`.
//
// The fake host mirrors `curvyo-app`'s commands closely enough for the page; their Rust side is
// covered by `curvyo-app` and `curvyo-storage-io` tests, not here.
//
// Needs `target/debug/curvyo-sync-server` (`cargo build -p curvyo-sync-server`) and a production
// preview (`npm run build`, `npx vite preview --host localhost --port 1421`); starts the preview
// itself unless CURVYO_BROWSER_URL is set. Playwright as in README.md. Screenshots:
// target/0096-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn, execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const { assertPanelFits } = require('./support/panel-overflow.cjs');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0096-browser');
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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then((r) => r.ok, () => false)) return;
    await sleep(200);
  }
  throw new Error(`no server at ${url}`);
}

/** A server on `address` that accepts the WebSocket upgrade and then never answers: reachable,
 * but a share or a sync there never gets anywhere (customer feedback on PR #281). */
function silentRelay(address) {
  const http = require('node:http');
  const [host, port] = address.split(':');
  const server = http.createServer();
  const sockets = [];
  server.on('upgrade', (request, socket) => {
    const accept = crypto.createHash('sha1').update(`${request.headers['sec-websocket-key']}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    sockets.push(socket);
  });
  return new Promise((resolve) => server.listen(Number(port), host, () => resolve({
    stop: () => new Promise((done) => { for (const socket of sockets) socket.destroy(); server.close(() => done()); }),
  })));
}

/** The relay on a free port with a fresh data directory, or again on `again`'s address and data. */
async function startRelay(again) {
  const data = again ? again.data : path.join(out, 'relay-data');
  if (!again) fs.mkdirSync(data);
  const listen = again ? again.address : '127.0.0.1:0';
  const relay = spawn(path.join(root, 'target/debug/curvyo-sync-server'), ['--listen', listen, '--data', data], {
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
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
  return { address, data, log: () => log, stop: () => process.kill(-relay.pid, 'SIGKILL') };
}

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
const markerId = (folder) => {
  try {
    return /id = "([0-9a-f]{32})"/.exec(fs.readFileSync(path.join(folder, MARKER, 'workspace.toml'), 'utf8'))[1];
  } catch {
    return null;
  }
};

/** One desktop device's host: the open workspace, the synced workspaces by id, recents, the
 * device group files and the credential store, all under target/0096-browser/<label>/. */
function makeHost(label) {
  const home = path.join(out, label);
  const trash = path.join(home, 'trash');
  fs.mkdirSync(trash, { recursive: true });
  const state = {
    root: null,
    project: null,
    synced: new Map(),
    recents: [],
    keys: new Map(),
    identity: crypto.randomBytes(64).toString('hex'),
    group: null,
    saves: 0,
  };
  const rootOf = (a) => {
    const found = a && a.workspace ? state.synced.get(a.workspace) : state.root;
    if (!found) throw 'not-found';
    return found;
  };
  const at = (a, rel) => {
    for (const part of rel) {
      if (!part || part === '.' || part === '..' || part.includes('/') || part.startsWith('.')) throw 'outside';
    }
    return path.join(rootOf(a), ...rel);
  };
  const marker = (a, name) => path.join(rootOf(a), MARKER, name);
  const scan = (base) => {
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
    walk(base, []);
    return found;
  };
  const writeMarker = (folder, id) => {
    fs.mkdirSync(path.join(folder, MARKER), { recursive: true });
    fs.writeFileSync(path.join(folder, MARKER, 'workspace.toml'), `format_version = 1\nid = "${id}"\n`);
  };
  const remember = (folder) => {
    state.recents = [folder, ...state.recents.filter((f) => f !== folder)];
  };
  const recentRows = () => state.recents.map((folder) => ({ name: path.basename(folder), path: folder, found: fs.existsSync(folder) }));
  const open = (folder) => {
    state.root = folder;
    if (!markerId(folder)) writeMarker(folder, crypto.randomBytes(16).toString('hex'));
    remember(folder);
    return { kind: 'opened', name: path.basename(folder), entries: scan(folder) };
  };
  const readIndex = (a) => {
    try {
      return JSON.parse(fs.readFileSync(marker(a, 'index.json'), 'utf8')).documents;
    } catch {
      return {};
    }
  };
  const create = (file, bytes) => {
    if (fs.existsSync(file)) throw 'exists';
    fs.writeFileSync(file, bytes);
  };
  const toTrash = (file) => fs.renameSync(file, path.join(trash, `${Date.now()}-${crypto.randomBytes(3).toString('hex')}-${path.basename(file)}`));
  const kept = (a, document) => path.join(rootOf(a), MARKER, 'deleted', `${document}.curvyo`);
  const syncedFolder = path.join(home, 'Curvyo');
  const commands = {
    take_pending_open: () => null,
    font_folder_list: () => ({ faces: [], skipped: [] }),
    system_font_list: () => [],
    read_device_keys: (a) => ({ status: 'ok', identity: state.identity, keys: state.keys.get(a.room) ?? [] }),
    write_document_keys: (a) => { state.keys.set(a.room, a.keys); return null; },
    device_host_name: () => `ZQX-PC-${label}`,
    read_device_group: () => state.group,
    write_device_group: (a) => { state.group = a.group; return null; },
    remove_device_group: () => { state.group = null; return null; },
    prepare_synced_folder: (a) => {
      const folder = a.path || syncedFolder;
      fs.mkdirSync(folder, { recursive: true });
      return { kind: 'picked', path: folder };
    },
    pick_synced_folder: () => ({ kind: 'cancelled' }),
    register_synced_workspace: (a) => {
      const folder = a.path || state.root;
      if (!folder || markerId(folder) !== a.id) throw 'not-found';
      state.synced.set(a.id, folder);
      return folder;
    },
    create_synced_workspace: (a) => {
      fs.mkdirSync(a.synced, { recursive: true });
      let folder = path.join(a.synced, a.name);
      for (let n = 2; fs.existsSync(folder) && markerId(folder) !== a.id && fs.readdirSync(folder).length > 0; n += 1) {
        folder = path.join(a.synced, `${a.name} ${n}`);
      }
      fs.mkdirSync(folder, { recursive: true });
      if (!markerId(folder)) writeMarker(folder, a.id);
      remember(folder);
      state.synced.set(a.id, folder);
      return { kind: 'picked', path: folder };
    },
    find_workspace_copy: (a) => {
      const below = fs.existsSync(a.synced) ? fs.readdirSync(a.synced).map((n) => path.join(a.synced, n)) : [];
      return [...state.recents, ...below].find((folder) => markerId(folder) === a.id) ?? null;
    },
    workspace_summary: (a) => {
      const id = markerId(a.path);
      if (!id) return null;
      const share = path.join(a.path, MARKER, 'share.json');
      const shared = fs.existsSync(share);
      return {
        id,
        documents: scan(a.path).filter((e) => e.kind === 'document').length,
        shared,
        keyring: shared ? fs.readFileSync(path.join(a.path, MARKER, 'keyring.log')).toString('hex') : '',
      };
    },
    recent_workspaces: () => recentRows(),
    remove_recent_workspace: (a) => { state.recents.splice(a.index, 1); return recentRows(); },
    pick_workspace: () => {
      const folder = path.join(home, state.pickName || 'ZQX-WS-7731');
      fs.mkdirSync(folder, { recursive: true });
      return open(folder);
    },
    open_recent_workspace: (a) => open(state.recents[a.index]),
    close_workspace: () => { state.root = null; return null; },
    scan_open_workspace: (a) => scan(rootOf(a)),
    apply_workspace_op: (a, bytes) => {
      const op = a.op;
      if (op.kind === 'create-document') create(at(a, op.path), bytes ?? Buffer.alloc(0));
      else if (op.kind === 'create-folder') {
        if (fs.existsSync(at(a, op.path))) throw 'exists';
        fs.mkdirSync(at(a, op.path));
      } else if (op.kind === 'move') {
        const to = at(a, op.to);
        if (fs.existsSync(to)) throw 'exists';
        fs.renameSync(at(a, op.from), to);
      } else if (op.kind === 'trash') toTrash(at(a, op.path));
      return null;
    },
    read_workspace_document: (a) => {
      const file = at(a, a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      return { raw: fs.readFileSync(file) };
    },
    probe_workspace_document: (a) => {
      const answer = zip('probe', at(a, a.path));
      if (answer.error) throw answer.error;
      return { raw: Buffer.from(answer.thumbnail, 'hex') };
    },
    confirm_workspace_document_opened: (a) => { state.project = a.path; return null; },
    reset_project: () => { state.project = null; return null; },
    save_project_bytes: (a) => {
      const bytes = Buffer.from(a.bytes);
      if (state.project && state.root && !a.saveAs) {
        fs.writeFileSync(at({}, state.project), bytes);
        return { kind: 'saved', file_name: state.project[state.project.length - 1], path: at({}, state.project) };
      }
      state.saves += 1;
      const file = path.join(home, `save-${state.saves}.curvyo`);
      fs.writeFileSync(file, bytes);
      return { kind: 'saved', file_name: a.fileName, path: file };
    },
    workspace_marker_share: (a) => {
      if (!fs.existsSync(marker(a, 'share.json'))) return null;
      const read = (name) => Array.from(fs.readFileSync(marker(a, name)));
      return { share_json: read('share.json'), keyring_log: read('keyring.log'), list: read('list.loro') };
    },
    write_workspace_marker_share: (a) => {
      fs.writeFileSync(marker(a, 'share.json'), Buffer.from(a.marker.share_json));
      fs.writeFileSync(marker(a, 'keyring.log'), Buffer.from(a.marker.keyring_log));
      fs.writeFileSync(marker(a, 'list.loro'), Buffer.from(a.marker.list));
      fs.rmSync(marker(a, 'share.ended.json'), { force: true });
      return null;
    },
    write_workspace_id: (a) => { writeMarker(rootOf(a), a.id); return null; },
    replace_workspace_document: (a, bytes) => {
      const file = at(a, a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      fs.writeFileSync(file, bytes);
      return null;
    },
    write_workspace_index: (a) => {
      const documents = {};
      for (const { document, path: rel } of a.documents) {
        const stat = fs.statSync(at(a, rel), { throwIfNoEntry: false });
        documents[document] = { path: rel, size: stat?.size ?? 0, modified: stat ? Math.floor(stat.mtimeMs / 1000) : 0 };
      }
      fs.writeFileSync(marker(a, 'index.json'), JSON.stringify({ format_version: 1, documents }));
      return null;
    },
    scan_shared_workspace: (a) => {
      const entries = scan(rootOf(a));
      const documents = entries.filter((e) => e.kind === 'document').map((e) => {
        const members = zip('members', at(a, e.path));
        return { path: e.path, share: members.share ?? null, keyring: !!members.keyring };
      });
      const known = Object.entries(readIndex(a)).map(([document, entry]) => ({ document, path: entry.path }));
      return { entries, folders: entries.filter((e) => e.kind === 'folder').map((e) => e.path), documents, known };
    },
    keep_workspace_document: (a) => {
      fs.mkdirSync(path.dirname(kept(a, a.document)), { recursive: true });
      fs.copyFileSync(at(a, a.path), kept(a, a.document));
      if (a.trash) toTrash(at(a, a.path));
      return null;
    },
    restore_workspace_document: (a) => {
      if (!fs.existsSync(kept(a, a.document))) throw 'not-found';
      create(at(a, a.path), fs.readFileSync(kept(a, a.document)));
      return null;
    },
    read_deleted_document: (a) => {
      if (!fs.existsSync(kept(a, a.document))) throw 'not-found';
      return { raw: fs.readFileSync(kept(a, a.document)) };
    },
    write_deleted_document: (a, bytes) => {
      fs.mkdirSync(path.dirname(kept(a, a.document)), { recursive: true });
      fs.writeFileSync(kept(a, a.document), bytes);
      return null;
    },
    clear_workspace_marker_share: (a) => {
      for (const name of ['share.json', 'keyring.log', 'list.loro', 'index.json', 'share.ended.json']) fs.rmSync(marker(a, name), { force: true });
      return null;
    },
    // `specs/0097` (ADR 0041 decision 6): a removal keeps share.ended.json in place of share.json
    // and keyring.log; complete share files win over it.
    workspace_marker_ended: (a) => {
      if (!fs.existsSync(marker(a, 'share.ended.json'))) return null;
      if (['share.json', 'keyring.log', 'list.loro'].every((n) => fs.existsSync(marker(a, n)))) return null;
      const list = fs.existsSync(marker(a, 'list.loro')) ? Array.from(fs.readFileSync(marker(a, 'list.loro'))) : [];
      return { ended_json: Array.from(fs.readFileSync(marker(a, 'share.ended.json'))), list };
    },
    end_workspace_marker_share: (a) => {
      fs.writeFileSync(marker(a, 'share.ended.json'), Buffer.from(a.ended_json));
      for (const name of ['share.json', 'keyring.log']) fs.rmSync(marker(a, name), { force: true });
      return null;
    },
  };
  return {
    state,
    home,
    syncedFolder,
    /** One command from the page: `{value}`, `{raw: hex}` or `{error}`. */
    handle(cmd, argsJson, bytesHex) {
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

/** The page side of the fake host (as `workspace.cjs`). */
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
async function until(what, check, timeout = 15000) {
  const start = Date.now();
  for (;;) {
    if (await check().catch(() => false)) return Date.now() - start;
    if (Date.now() - start > timeout) throw new Error(`timed out after ${timeout} ms: ${what}`);
    await sleep(150);
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
const devicesList = (d) => panel(d).getByRole('list', { name: 'My devices' }).locator(':scope > li');
const shown = (d, text) => d.page.getByText(text, { exact: true }).count().then((n) => n > 0);
const shot = (d, name) => d.page.screenshot({ path: path.join(out, `${name}.png`) });
const objects = (file) => {
  const flat = (nodes) => nodes.flatMap((n) => (n.shape === 'layer' || n.shape === 'group' ? flat(n.objects || []) : [n]));
  return flat(zip('json', file).objects);
};

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

async function setServer(d, address, name) {
  await tab(d, 'Share');
  const server = panel(d).getByLabel('Server', { exact: true });
  await server.fill(address);
  await server.press('Enter');
  const person = panel(d).getByLabel('Name shown to others', { exact: true });
  await person.fill(name);
  await person.press('Enter');
  await panel(d).getByText('Connected', { exact: true }).first().waitFor();
}

/** The four check words of the compare group on `d`'s screen. */
async function checkWords(d) {
  const group = panel(d).getByRole('group', { name: /Check words: / });
  await group.waitFor({ timeout: 20000 });
  const name = await group.getAttribute('aria-label');
  return /Check words: (.*)\.$/.exec(name)[1].split(', ');
}

(async () => {
  const relay = await startRelay();
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const preview = spawn('npx', ['vite', 'preview', '--host', 'localhost', '--port', '1421', '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    children.push(preview);
    url = 'http://localhost:1421/';
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const address = `ws://${relay.address}`;
  let A = null;
  let B = null;
  try {
    A = await device(browser, url, 'A');
    B = await device(browser, url, 'B');

    step('A opens a workspace with one document');
    await tab(A, 'File');
    await panel(A).getByRole('button', { name: 'Open workspace…', exact: true }).click();
    await tree(A).waitFor();
    await panel(A).getByRole('region', { name: 'Workspace' }).getByLabel('New document', { exact: true }).click();
    const field = panel(A).getByRole('textbox', { name: 'Name of the new document' });
    await field.fill('ZQX-DOC-7731');
    await field.press('Enter');
    await settle(A.page, 400);
    await rect(A, 300, 200, 120, 80);
    await save(A);
    const aFile = path.join(A.host.state.root, 'ZQX-DOC-7731.curvyo');
    assert.equal(objects(aFile).length, 1);

    step('Criteria 1 and 2: A shows a code, B joins with it; both show the same four words');
    await setServer(A, address, 'Anna');
    await setServer(B, address, 'Anna');
    assert.equal(await devicesList(A).count(), 1, 'only this device before pairing');
    await assertPanelFits(A.page, 'A before pairing');
    await panel(A).getByRole('button', { name: 'Add a device', exact: true }).click();
    const codeGroup = panel(A).getByRole('group', { name: /^[0-9A-Z](, [0-9A-Z]){7}$/ });
    await codeGroup.waitFor({ timeout: 15000 });
    const code = (await codeGroup.innerText()).trim();
    assert.match(code, /^[0-9A-Z]{4}-[0-9A-Z]{4}$/);
    assert.ok(!(await shown(A, 'The device that enters this code can sync the workspaces you choose, and you can remove it again. The server can tell which devices belong together, not their names.')), 'no hint sentence');
    await shot(A, '01-a-code');
    await assertPanelFits(A.page, 'A shows its code');
    await panel(B).getByRole('button', { name: 'Join my devices', exact: true }).click();
    await panel(B).getByLabel('Pairing code', { exact: true }).fill(code.toLowerCase().replace('-', ' '));
    await panel(B).getByRole('button', { name: 'Connect', exact: true }).click();
    const wordsA = await checkWords(A);
    const wordsB = await checkWords(B);
    assert.equal(wordsA.length, 4);
    assert.deepEqual(wordsA, wordsB, 'the same check words on both screens');
    assert.ok(await shown(B, 'Waiting for ZQX-PC-A to confirm'), 'B waits for A');
    assert.equal(fs.existsSync(B.host.syncedFolder) ? fs.readdirSync(B.host.syncedFolder).length : 0, 0, 'B holds nothing yet');
    await shot(A, '02-a-compare');
    await shot(B, '02-b-compare');
    await assertPanelFits(A.page, 'A compares the words');
    await assertPanelFits(B.page, 'B compares the words');

    step('Criteria 3 and 8: A pairs; both list both devices; A offers its workspace, checked');
    await panel(A).getByRole('button', { name: 'Pair', exact: true }).click();
    await until('A lists two devices', async () => (await devicesList(A).count()) === 2, 20000);
    await until('B lists two devices', async () => (await devicesList(B).count()) === 2, 20000);
    const pick = panel(A).getByRole('list', { name: 'Workspaces to sync' });
    await pick.waitFor({ timeout: 15000 });
    assert.ok(await pick.getByRole('checkbox').first().isChecked(), 'the open workspace is checked');
    await shot(A, '03-a-pick');
    await assertPanelFits(A.page, 'A picks workspaces');

    step('Criteria 9 and 10: Sync these; the workspace arrives on B without input there');
    await panel(A).getByRole('button', { name: 'Sync these', exact: true }).click();
    const bFolder = path.join(B.host.syncedFolder, 'ZQX-WS-7731');
    const bFile = path.join(bFolder, 'ZQX-DOC-7731.curvyo');
    const arrivalMs = await until('the document is on B\'s disk', async () => fs.existsSync(bFile) && objects(bFile).length === 1, 60000);
    await shot(B, '04-b-arrived');

    step('Criterion 18: B opens it; edits reach the other side both ways');
    await tab(B, 'File');
    await panel(B).getByRole('button', { name: /^ZQX-WS-7731/ }).first().click();
    await tree(B).waitFor({ timeout: 15000 });
    await row(B, 'ZQX-DOC-7731').click();
    await settle(B.page, 800);
    await tab(A, 'File');
    await row(A, 'ZQX-DOC-7731').click();
    await settle(A.page, 800);
    await rect(A, 600, 400, 80, 40);
    await until('B holds A\'s new rectangle', async () => {
      await save(B);
      return objects(bFile).length === 2;
    }, 20000);
    await rect(B, 200, 500, 40, 40);
    await until('A holds B\'s rectangle', async () => {
      await save(A);
      return objects(aFile).length === 3;
    }, 20000);
    await tab(A, 'Share');
    assert.ok(await shown(A, 'Synced, 2 devices'), 'the Workspace block says it is synced to my devices');
    await shot(A, '05-a-synced');

    step('Criterion 13: A switches the workspace off (armed, no sentence); B keeps a local copy');
    const syncSwitch = panel(A).getByRole('switch', { name: 'Sync to my devices' });
    // Off is armed (customer feedback on PR #281): the first press arms the switch, a second press
    // after the guard turns it off; no sentence (customer 2026-11-09).
    await syncSwitch.click();
    await settle(A.page, 200);
    assert.equal(await panel(A).getByText(/keeps its copy as local files/).count(), 0, 'no sentence');
    assert.equal(await syncSwitch.getAttribute('aria-checked'), 'true', 'the first press only arms');
    await sleep(450);
    await syncSwitch.click();
    const switchOffMs = await until('A\'s switch is off', async () => (await syncSwitch.getAttribute('aria-checked')) === 'false', 30000);
    await until('B is told', () => shown(B, 'ZQX-WS-7731 is no longer synced. Your copy stays here.'), 30000);

    step('Criterion 14: B edits offline from the room and closes the workspace; A switches on again; Connect this folder sends B\'s edit without B opening the document');
    await rect(B, 500, 600, 30, 30);
    await save(B);
    assert.equal(objects(bFile).length, 4);
    await tab(B, 'File');
    await panel(B).getByRole('region', { name: 'Workspace' }).getByLabel('Close workspace', { exact: true }).click();
    await settle(B.page, 500);
    await tab(A, 'Share');
    // On asks nothing (customer 2026-11-09).
    await syncSwitch.click();
    const switchOnMs = await until('A\'s switch is on', async () => (await syncSwitch.getAttribute('aria-checked')) === 'true', 30000);
    await tab(B, 'Share');
    const connect = panel(B).getByRole('button', { name: 'Connect this folder', exact: true });
    await connect.waitFor({ timeout: 40000 });
    await shot(B, '06-b-decision');
    await assertPanelFits(B.page, 'B decides about its copy');
    await connect.click();
    await until('A holds B\'s offline rectangle', async () => {
      await save(A);
      return objects(aFile).length === 4;
    }, 40000);
    await tab(A, 'Share');

    step('Criteria 23 and 24: A removes B; B is told and keeps its files');
    const removeB = panel(A).getByRole('button', { name: 'Remove ZQX-PC-B', exact: true });
    await removeB.click();
    await sleep(450);
    await removeB.click();
    const removeMs = await until('A lists only itself', async () => (await devicesList(A).count()) === 1, 30000);
    await tab(B, 'Share');
    await until('B is told', () => shown(B, 'This device was removed from your devices. Your copies stay here as local files.'), 30000);
    assert.ok(objects(bFile).length >= 4, 'B keeps its copy');
    await shot(B, '07-b-removed');
    await assertPanelFits(B.page, 'B was removed');

    step('Customer 2026-11-09: no one-device warning');
    await settle(A.page, 1500);
    assert.ok(!(await shown(A, 'Only this device is paired. If you lose it, the copy on the server cannot be opened by anyone. Pair a second device as a backup.')));
    await shot(A, '08-a-one-device');

    step('Criterion 30: the server\'s data holds none of the names or the code');
    const needles = ['ZQX-WS-7731', 'ZQX-DOC-7731', 'ZQX-PC-A', 'ZQX-PC-B', code, code.replace('-', '')];
    const files = [];
    const walk = (dir) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) walk(full);
        else files.push(full);
      }
    };
    walk(relay.data);
    for (const file of files) {
      const bytes = fs.readFileSync(file);
      for (const needle of needles) assert.equal(bytes.indexOf(Buffer.from(needle)), -1, `${file} holds ${needle}`);
    }
    for (const needle of needles) assert.equal(relay.log().indexOf(needle), -1, `the log holds ${needle}`);

    step('PR #281 review: the removed B pairs again with the same identity and is listed again');
    const closed = ['ZQX-WS-OUT1', 'ZQX-WS-OUT2'].map((name) => {
      const folder = path.join(out, 'A', name);
      fs.mkdirSync(path.join(folder, MARKER), { recursive: true });
      fs.writeFileSync(path.join(folder, MARKER, 'workspace.toml'), `format_version = 1\nid = "${crypto.randomBytes(16).toString('hex')}"\n`);
      A.host.state.recents.push(folder);
      return name;
    });
    const identityB = B.host.state.identity;
    await panel(B).getByRole('region', { name: 'My devices' }).getByRole('button', { name: 'OK', exact: true }).click();
    await panel(B).getByRole('button', { name: 'Join my devices', exact: true }).waitFor({ timeout: 15000 });
    await assertPanelFits(B.page, 'B removed, Add a device and Join my devices');
    await shot(B, '09-b-unpaired');
    await tab(A, 'Share');
    let pairedAt = Date.now();
    await panel(A).getByRole('button', { name: 'Add a device', exact: true }).click();
    const again = panel(A).getByRole('group', { name: /^[0-9A-Z](, [0-9A-Z]){7}$/ });
    await again.waitFor({ timeout: 15000 });
    const codeShownMs = Date.now() - pairedAt;
    const secondCode = (await again.innerText()).trim();
    await panel(B).getByRole('button', { name: 'Join my devices', exact: true }).click();
    await assertPanelFits(B.page, 'B joining again');
    await panel(B).getByLabel('Pairing code', { exact: true }).fill(secondCode);
    pairedAt = Date.now();
    await panel(B).getByRole('button', { name: 'Connect', exact: true }).click();
    await checkWords(A);
    const wordsMs = Date.now() - pairedAt;
    pairedAt = Date.now();
    await panel(A).getByRole('button', { name: 'Pair', exact: true }).click();
    await until('A lists B again', async () => (await devicesList(A).count()) === 2, 20000);
    const pairMs = Date.now() - pairedAt;
    await until('B lists both again', async () => (await devicesList(B).count()) === 2, 20000);
    assert.ok(!(await shown(A, 'Pairing ZQX-PC-B...')), 'A is not left at "Pairing"');
    assert.equal(B.host.state.identity, identityB, 'B paired again with the same identity');
    await shot(A, '09-a-b-again');

    step('PR #281 feedback: the server goes down before Sync these of two closed workspaces: one alert for both, gone once the server answers again, and both arrive on B');
    const pickAgain = panel(A).getByRole('list', { name: 'Workspaces to sync' });
    await pickAgain.waitFor({ timeout: 15000 });
    for (const name of closed) {
      const box = pickAgain.getByRole('checkbox', { name: new RegExp(`^${name}`) });
      if (!(await box.isChecked())) await box.check();
    }
    await assertPanelFits(A.page, 'A picks two closed workspaces');
    relay.stop();
    // The group's socket sees the close at once; a second covers the page's next look.
    await sleep(1000);
    await panel(A).getByRole('button', { name: 'Sync these', exact: true }).click();
    const outage = 'Could not sync 2 workspaces: cannot reach the server.';
    const alertMs = await until('one alert for both workspaces', () => shown(A, outage), 40000);
    const alerts = await panel(A).getByRole('alert').allInnerTexts();
    assert.deepEqual(alerts.filter((t) => t.includes('cannot reach the server')), [outage], 'one line per outage, not one per workspace');
    await assertPanelFits(A.page, 'A shows the outage alert');
    await shot(A, '10-a-outage');
    const restarted = await startRelay(relay);
    const backAt = Date.now();
    await until('the alert goes once the server answers', async () => !(await shown(A, outage)), 60000);
    const clearMs = Date.now() - backAt;
    for (const name of closed) {
      await until(`${name} arrives on B`, async () => fs.existsSync(path.join(B.host.syncedFolder, name)), 60000);
    }
    const arriveMs = Date.now() - backAt;
    await shot(A, '11-a-back');
    let live = restarted;

    step('Share tab feedback 2: the server row says it cannot reach the server and Try again checks again');
    B.host.state.pickName = 'ZQX-WS-NEW';
    await tab(B, 'File');
    await panel(B).getByRole('button', { name: 'Open workspace…', exact: true }).click();
    await tree(B).waitFor();
    await panel(B).getByRole('region', { name: 'Workspace' }).getByLabel('New document', { exact: true }).click();
    const newDoc = panel(B).getByRole('textbox', { name: 'Name of the new document' });
    await newDoc.fill('ZQX-DOC-NEW');
    await newDoc.press('Enter');
    await settle(B.page, 400);
    await rect(B, 300, 200, 60, 60);
    await save(B);
    const newRow = row(B, 'ZQX-DOC-NEW');
    console.log(`the new document's row after New document: aria-selected=${await newRow.getAttribute('aria-selected')}`);
    live.stop();
    await sleep(1000);
    await tab(B, 'Share');
    const serverField = panel(B).getByLabel('Server', { exact: true });
    for (const text of ['ws://127.0.0.1:9', address]) {
      await serverField.fill(text);
      await serverField.press('Enter');
      await settle(B.page, 300);
    }
    const tryServer = panel(B).getByRole('button', { name: 'Try again', exact: true });
    await until('the server row offers Try again', async () => (await tryServer.count()) === 1, 10000);
    assert.equal(await tryServer.getAttribute('aria-label'), 'Try again');
    await shot(B, '12-b-server-try-again');
    live = await startRelay(live);
    await tryServer.click();
    await panel(B).getByText('Connected', { exact: true }).first().waitFor({ timeout: 10000 });

    step('0105 criterion 21: the line follows the server every 5 s without a press');
    live.stop();
    const stoppedAt = Date.now();
    await until('the line says the server cannot be reached', async () => (await panel(B).getByText(/^Cannot reach /).count()) > 0, 10000);
    const downMs = Date.now() - stoppedAt;
    assert.ok(!(await shown(B, 'Connected')), 'no other line says Connected');
    live = await startRelay(live);
    const upAt = Date.now();
    await until('the line says Connected', () => shown(B, 'Connected'), 10000);
    console.log(`reachability: unreachable after ${downMs} ms, Connected ${Date.now() - upAt} ms after the restart`);

    step('Share tab feedback 1: a switch-on whose server never answers turns off again at once');
    live.stop();
    await sleep(500);
    const silent = await silentRelay(live.address);
    await tab(B, 'File');
    await newRow.click();
    await settle(B.page, 500);
    assert.equal(await newRow.getAttribute('aria-selected'), 'true', 'the document is open before the switch');
    await tab(B, 'Share');
    const newSwitch = panel(B).getByRole('switch', { name: 'Sync to my devices' });
    const workspaceBlock = panel(B).getByRole('region', { name: 'Workspace ZQX-WS-NEW' });
    await newSwitch.click();
    // Share closes the document while it prepares (as Share workspace): back to the Share tab.
    await tab(B, 'Share');
    await until('the switch runs', async () => (await newSwitch.getAttribute('aria-checked')) === 'true', 5000);
    await until('the header says Sharing', async () => (await workspaceBlock.innerText()).includes('Sharing'), 5000);
    assert.ok(!(await workspaceBlock.innerText()).includes('Not shared'), 'never "Not shared" while it shares');
    await shot(B, '13-b-sharing');
    const cancelAt = Date.now();
    await newSwitch.click();
    await until('the switch is off again', async () => (await newSwitch.getAttribute('aria-checked')) === 'false', 2000);
    const cancelMs = Date.now() - cancelAt;
    // The attempt's session is replaced (the document comes back, or an empty one): Share again.
    await settle(B.page, 1500);
    await tab(B, 'Share');
    await until('the header says Not shared again', async () => (await workspaceBlock.innerText()).includes('Not shared'), 5000);
    await tab(B, 'File');
    await until('the document is open again after the cancel', async () => (await newRow.getAttribute('aria-selected')) === 'true', 10000);
    assert.equal(await B.page.getByText(/did not let this device in/).count(), 0);
    await tab(B, 'Share');

    step('Share tab feedback 1: a switch-on that never gets an answer fails with Try again, which works once the server is up');
    await until('the server reads connected', () => shown(B, 'Connected'), 10000);
    await newSwitch.click();
    await tab(B, 'Share');
    const tryAgain = workspaceBlock.getByRole('button', { name: 'Try again', exact: true });
    // The attempt's session is replaced when it fails, which shows another tab: Share again.
    const failMs = await until('the switch fails with Try again', async () => {
      if ((await newSwitch.count()) === 0) await tab(B, 'Share');
      return (await tryAgain.count()) === 1;
    }, 45000);
    assert.equal(await newSwitch.getAttribute('aria-checked'), 'false');
    assert.ok((await workspaceBlock.getByRole('alert').allInnerTexts()).some((t) => t.startsWith('Could not ')), 'the alert says why');
    assert.ok(!(await workspaceBlock.innerText()).includes('Uploading...'), 'no endless Uploading');
    await tab(B, 'File');
    await until('the document is open again after the failure', async () => (await newRow.getAttribute('aria-selected')) === 'true', 10000);
    await tab(B, 'Share');
    await shot(B, '14-b-switch-failed');
    await assertPanelFits(B.page, 'B shows the failed switch');
    await silent.stop();
    live = await startRelay(live);
    await tryAgain.click();
    await tab(B, 'Share');
    const retryMs = await until('the switch is on after Try again', async () => {
      if ((await newSwitch.count()) === 0) await tab(B, 'Share');
      return (await newSwitch.getAttribute('aria-checked')) === 'true' && (await shown(B, 'In sync with 1 device'));
    }, 60000);
    await until('the new workspace arrives on A', async () => fs.existsSync(path.join(A.host.syncedFolder, 'ZQX-WS-NEW')), 60000);

    step('Share tab feedback 3 and 4: B leaves its devices (armed); its workspace reopens local, never refused');
    const leaveB = panel(B).getByRole('button', { name: 'Leave my devices', exact: true });
    await leaveB.click();
    await sleep(450);
    await leaveB.click();
    await panel(B).getByRole('button', { name: 'Join my devices', exact: true }).waitFor({ timeout: 15000 });
    await until('B\'s marker has no share files', async () => !fs.existsSync(path.join(B.host.home, 'ZQX-WS-NEW', MARKER, 'share.json')), 10000);
    await tab(B, 'File');
    await panel(B).getByRole('region', { name: 'Workspace' }).getByLabel('Close workspace', { exact: true }).click();
    await settle(B.page, 500);
    await panel(B).getByRole('button', { name: /^ZQX-WS-NEW/ }).first().click();
    await tree(B).waitFor({ timeout: 15000 });
    await row(B, 'ZQX-DOC-NEW').click();
    await settle(B.page, 3000);
    await tab(B, 'Share');
    assert.ok((await panel(B).getByRole('region', { name: 'Workspace ZQX-WS-NEW' }).innerText()).includes('Not shared'));
    assert.equal(await B.page.getByText(/did not let this device in/).count(), 0, 'no refusal after the reopen');

    step('Share tab feedback 3: A, now the last device, may leave; the group ends and its workspaces stay local');
    await tab(A, 'Share');
    await until('A is the only device', async () => (await devicesList(A).count()) === 1, 30000);
    const leaveA = panel(A).getByRole('button', { name: 'Leave my devices', exact: true });
    await leaveA.click();
    await shot(A, '15-a-leave-last');
    await sleep(450);
    await leaveA.click();
    await panel(A).getByRole('button', { name: 'Join my devices', exact: true }).waitFor({ timeout: 15000 });
    assert.ok(!(await shown(A, 'This device is the only one that can manage your devices. Pair another device first.')));
    for (const name of ['ZQX-WS-7731', 'ZQX-WS-NEW', ...closed]) {
      const where = name === 'ZQX-WS-7731' ? path.join(A.host.home, name) : name === 'ZQX-WS-NEW' ? path.join(A.host.syncedFolder, name) : path.join(out, 'A', name);
      await until(`${name} is local on A`, async () => !fs.existsSync(path.join(where, MARKER, 'share.json')), 15000);
    }
    await tab(A, 'File');
    await panel(A).getByRole('region', { name: 'Workspace' }).getByLabel('Close workspace', { exact: true }).click();
    await settle(A.page, 500);
    await panel(A).getByRole('button', { name: /^ZQX-WS-7731/ }).first().click();
    await tree(A).waitFor({ timeout: 15000 });
    await row(A, 'ZQX-DOC-7731').click();
    await settle(A.page, 3000);
    await tab(A, 'Share');
    assert.ok((await panel(A).getByRole('region', { name: 'Workspace ZQX-WS-7731' }).innerText()).includes('Not shared'));
    assert.equal(await A.page.getByText(/did not let this device in/).count(), 0, 'no refusal after the reopen');
    await shot(A, '16-a-local-again');

    console.log(`timings: Sync these to the file on B ${arrivalMs} ms, switch off ${switchOffMs} ms, switch on ${switchOnMs} ms, remove ${removeMs} ms`);
    console.log(`timings: code shown ${codeShownMs} ms, words ${wordsMs} ms, paired ${pairMs} ms, outage alert ${alertMs} ms, alert gone ${clearMs} ms after the restart, both arrived ${arriveMs} ms after it`);
    console.log(`timings: switch turned back ${cancelMs} ms, failed switch shown ${failMs} ms, on after Try again ${retryMs} ms`);
    live.stop();

    assert.deepEqual([...A.errors, ...B.errors], []);
    console.log('PASS browser 0096: pair with code and words, sync these, arrival, edits both ways, switch off and on, connect this folder, remove, one-device warning, server data blind, a removed device pairs again');
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

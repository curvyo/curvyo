// Browser journey of `specs/0095-workspace` slice 1 (plan task 27): a local `curvyo-sync-server`
// on a temporary data directory and two browser contexts, A and B, each its own device with a
// faked desktop host (`window.__TAURI_INTERNALS__`, as `acceptance_0087_browser.test.ts`) whose
// workspace commands run on real folders under target/0095-browser/. A opens an empty folder as a
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
// itself unless CURVYO_BROWSER_URL is set. Screenshots: target/0095-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0095-browser');
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

// ---- the fake desktop host (shared with `membership.cjs`) --------------------------------------

const { FAKE_HOST, MARKER, makeHost: hostIn, zip } = require('./support/fake-workspace-host.cjs');
const { watchTabs, tabsSeen } = require('./support/tab-watch.cjs');
/** Every tab selected since `watchTabs` was the Share tab (`specs/0105` criterion 20). */
async function stayedOnShare(d, what) {
  const seen = await tabsSeen(d.page);
  assert(seen.length > 0 && seen.every((t) => /^Share/.test(t)), `${what}: the tabs seen were ${JSON.stringify(seen)}`);
}
const makeHost = (label) => hostIn(out, label);

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
    await panel(A).getByText('Connected', { exact: true }).first().waitFor();
    await panel(A).getByRole('button', { name: 'Share', exact: true }).click();
    await panel(A).getByText(/^Shared on /).first().waitFor({ timeout: 10000 });
    await save(A);
    await until('Solo carries its own keyring', async () => zip('members', path.join(A.host.state.root, 'Solo.curvyo')).keyring);

    step('Criteria 25 and 42: Share workspace takes Solo in (no confirm, customer 2026-11-09)');
    await tab(A, 'Share');
    await watchTabs(A.page);
    await panel(A).getByRole('button', { name: 'Share workspace', exact: true }).click();
    await until('A shared the workspace', async () => (await panel(A).getByText(/^Shared on /).count()) >= 1, 30000);
    await settle(A.page, 500);
    await stayedOnShare(A, 'Share workspace');
    assert.equal(await panel(A).getByRole('button', { name: 'Invite', exact: true }).count(), 1, 'Invite is right there');
    await until('Solo is a workspace document', async () => {
      const members = zip('members', path.join(A.host.state.root, 'Solo.curvyo'));
      return !members.keyring && members.share && JSON.parse(members.share).format_version === 2;
    }, 30000);
    await shot(A, '02-a-shared');

    step('Criterion 26: a 2. invite');
    await panel(A).getByRole('button', { name: 'Invite', exact: true }).click();
    const linkField = panel(A).getByLabel('Invite link', { exact: true });
    await linkField.waitFor();
    const link = await linkField.inputValue();
    assert.match(link, /^curvyo:\/\/join\?server=[^#]+#2\.[A-Za-z0-9_-]+$/, link);
    // The invite shows its expiry without a sentence (customer 2026-11-09).
    assert.doesNotMatch(await panel(A).innerText(), /Anyone with this link/);
    assert.match(await panel(A).innerText(), /Expires /);

    step('Criteria 27 and 28: B joins into an empty folder and receives every document');
    await tab(B, 'Share');
    await panel(B).getByRole('button', { name: /^Join shared workspace/ }).click();
    await panel(B).getByLabel('Invite link', { exact: true }).fill(link);
    const bName = panel(B).getByLabel('Name shown to others', { exact: true }).last();
    await bName.fill('Ben');
    await bName.press('Enter');
    await panel(B).getByRole('button', { name: /^Choose folder/ }).click();
    const joinStart = Date.now();
    await watchTabs(B.page);
    await panel(B).getByRole('button', { name: 'Join', exact: true }).click();
    await until('B has every file', async () => B.host.state.root !== null
      && ['Lid.curvyo', 'Base.curvyo', 'Solo.curvyo'].every((f) => exists(B, f)), 30000);
    timings.join = Date.now() - joinStart;
    await until('B lists two participants', async () => {
      await tab(B, 'Share');
      return (await participants(B).count()) === 2;
    }, 15000);
    await until('A lists B', async () => (await participants(A).count()) === 2, 15000);
    await stayedOnShare(B, 'Join');
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
    const removeBen = panel(A).getByRole('button', { name: 'Remove Ben', exact: true });
    await removeBen.click();
    await settle(A.page, 600);
    await removeBen.click();
    await until('A lists only itself', async () => (await participants(A).count()) === 1, 15000);
    await until('B told', async () => (await B.page.getByText('You were removed from this workspace. Your copies stay on this computer.').count()) >= 1, 15000);
    await until('B\'s marker share files are gone', async () => !exists(B, MARKER, 'share.json'), 8000);
    // Since 0097 (ADR 0041 decision 6) the marker keeps the ended record for the way back, and
    // the Share tab shows the Removed block instead of the plain not-shared state.
    assert(exists(B, MARKER, 'share.ended.json'), 'B keeps the ended record');
    await tab(B, 'Share');
    // The Removed block is its heading (add back dropped, customer 2026-11-09).
    await until('B\'s Share tab shows the Removed block', async () => (await panel(B).getByText(/^You were removed from /).count()) >= 1, 8000);
    await until('B\'s Share tab says Not shared', async () => (await panel(B).getByText('Not shared', { exact: true }).count()) >= 1, 8000);
    assert.equal(await panel(B).getByRole('button', { name: 'Share workspace', exact: true }).count(), 1, 'Share workspace is available again');
    await shot(B, '07-b-removed');
    for (const f of ['Bottom.curvyo', 'Solo.curvyo', 'Cover.curvyo']) assert(exists(B, f), `B keeps ${f}`);

    assert.deepEqual([...A.errors, ...B.errors], []);
    console.log(`PASS browser 0095: open, create, share with take-in, invite 2., join, create/rename/move/delete/restore both ways, live edit, removal; timings ${JSON.stringify(timings)}`);
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

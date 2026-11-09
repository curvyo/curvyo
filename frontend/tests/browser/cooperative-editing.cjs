// Browser journey of `specs/0092-cooperative-editing` on a shared workspace (`specs/0105`
// criterion 23: the workspace is the only unit of sharing): a local `curvyo-sync-server` on a
// temporary data directory and two browser contexts, A and B, each its own device with a faked
// desktop host (`support/fake-workspace-host.cjs`) on real folders under target/0092-browser/. A
// shares a workspace with one document by Invite person, B joins by the link and opens the same
// document; both draw and see the other's rectangles (6) and pointer, selection and "is editing
// this" marker (14, 15); a drag shows live in B and ends as one step (7); B goes offline (17),
// both edit, B comes back and both hold every edit (18); A removes B (32), whose copy stays and
// who is told so.
//
// Needs `target/debug/curvyo-sync-server` (`cargo build -p curvyo-sync-server`) and a production
// preview (`npm run build`, `npx vite preview --host 127.0.0.1 --port 1421`); starts the preview
// itself unless CURVYO_BROWSER_URL is set. Screenshots and saved files: target/0092-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0092-browser');
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
  fs.rmSync(data, { recursive: true, force: true });
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

const settle = (page, ms = 120) => page.waitForTimeout(ms);
/** Polls `check` until it holds; returns the milliseconds it took. */
async function until(what, check, timeout = 10000) {
  const start = Date.now();
  for (;;) {
    if (await check()) return Date.now() - start;
    if (Date.now() - start > timeout) throw new Error(`timed out after ${timeout} ms: ${what}`);
    await new Promise((r) => setTimeout(r, 50));
  }
}

const { FAKE_HOST, makeHost, zip } = require('./support/fake-workspace-host.cjs');

async function device(browser, url, label) {
  const host = makeHost(out, label);
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

const tab = async (d, name) => { await d.page.getByRole('tab', { name: new RegExp(`^${name}`) }).click(); await settle(d.page); };
const tool = async (d, name) => { await d.page.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(d.page); };
const at = (d, x, y) => ({ x: d.canvas.x + x, y: d.canvas.y + y });

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

/** Saves the open workspace document through the File tab and returns its objects. */
async function saved(d) {
  await tab(d, 'File');
  await panel(d).getByRole('button', { name: 'Save', exact: true }).click();
  await settle(d.page, 300);
  const file = path.join(d.host.state.root, 'Poster.curvyo');
  const flat = (nodes) => nodes.flatMap((n) => (n.shape === 'layer' || n.shape === 'group' ? flat(n.objects || []) : [n]));
  return { objects: flat(zip('json', file).objects), file };
}

const panel = (d) => d.page.locator('[role=tabpanel]');
const participants = (d) => d.page.getByRole('list', { name: 'Participants' }).locator('li[data-member]');
const tree = (d) => d.page.getByRole('treegrid', { name: 'Workspace documents' });
const row = (d, name) => tree(d).getByRole('row', { name: new RegExp(`^${name}\\b`) });
const pill = (d) => d.page.locator('button[aria-label^="Collaboration"]');
/** The other participant's chips on the canvas (the presence overlay is `aria-hidden`). */
const chip = (d, text) => d.page.locator('div.pointer-events-none.absolute.inset-0 span', { hasText: text });
const shot = (d, name) => d.page.screenshot({ path: path.join(out, `${name}.png`) });

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

    step('0105 criterion 23 setup: A shares a workspace of one document by Invite person');
    await tab(A, 'File');
    await panel(A).getByRole('button', { name: 'Open workspace…', exact: true }).click();
    await tree(A).waitFor();
    await panel(A).getByRole('region', { name: 'Workspace' }).getByLabel('New document', { exact: true }).click();
    const newName = panel(A).getByRole('textbox', { name: 'Name of the new document' });
    await newName.fill('Poster');
    await newName.press('Enter');
    await settle(A.page, 400);
    await rect(A, 300, 200, 120, 80);
    const first = await saved(A);
    assert.equal(first.objects.length, 1);
    await tab(A, 'Share');
    const server = panel(A).getByLabel('Server', { exact: true });
    await server.fill(`ws://${relay.address}`);
    await server.press('Enter');
    const name = panel(A).getByLabel('Name shown to others', { exact: true });
    await name.fill('Anna');
    await name.press('Enter');
    await panel(A).getByText('Connected', { exact: true }).first().waitFor();
    await panel(A).getByRole('button', { name: 'Invite person', exact: true }).click();
    await panel(A).getByRole('group', { name: /leave this device encrypted/ }).getByRole('button', { name: 'Invite person', exact: true }).click();
    const linkField = panel(A).getByLabel('Invite link', { exact: true });
    await linkField.waitFor({ timeout: 30000 });
    const link = await linkField.inputValue();
    assert.match(link, /^curvyo:\/\/join\?server=[^#]+#2\.[A-Za-z0-9_-]+$/, link);
    assert.equal(await participants(A).count(), 1);
    const own = await participants(A).first().innerText();
    assert.match(own, /Anna\s*this device/);
    assert.match(own, /[A-Z2-7]{4} [A-Z2-7]{4}/, 'the own row shows a fingerprint');
    assert.equal(await pill(A).count(), 1, 'the collaboration pill shows');
    await shot(A, '01-a-shared');

    step('B joins by the link and opens the same document');
    await tab(B, 'Share');
    await panel(B).getByRole('button', { name: 'Join…', exact: true }).click();
    await panel(B).getByLabel('Link or code', { exact: true }).fill(link);
    const bName = panel(B).getByLabel('Name shown to others', { exact: true }).last();
    await bName.fill('Ben');
    await bName.press('Enter');
    await panel(B).getByRole('button', { name: /^Choose folder/ }).click();
    const joinStart = Date.now();
    await panel(B).getByRole('button', { name: 'Join', exact: true }).click();
    await until('B has the file', async () => B.host.state.root !== null && fs.existsSync(path.join(B.host.state.root, 'Poster.curvyo')), 30000);
    timings.join = Date.now() - joinStart;
    await tab(B, 'File');
    await until('B\'s tree shows Poster', async () => (await row(B, 'Poster').count()) === 1);
    await row(B, 'Poster').click();
    await settle(B.page, 800);
    await tab(B, 'Share');
    await until('B lists both', async () => (await participants(B).count()) === 2, 15000);
    await until('A lists B', async () => (await participants(A).count()) === 2);
    // Labels travel with the first presence message (heartbeat 2 s).
    await until('B names both', async () => {
      const list = await participants(B).allInnerTexts();
      return list.some((t) => /Ben\s*this device/.test(t)) && list.some((t) => /Anna/.test(t));
    }, 5000);
    await until('A names B', async () => (await participants(A).allInnerTexts()).some((t) => /Ben/.test(t)), 5000);
    await shot(B, '03-b-joined');
    assert.equal((await saved(B)).objects.length, 1, 'B holds A\'s rectangle');
    await tab(A, 'File');
    await row(A, 'Poster').click();
    await settle(A.page, 800);

    step('Criterion 6');
    // Criterion 6: a rectangle each way.
    await rect(A, 500, 200, 80, 60);
    timings.aToB = await until('A\'s rectangle in B', async () => (await saved(B)).objects.length === 2);
    await rect(B, 300, 420, 60, 60);
    timings.bToA = await until('B\'s rectangle in A', async () => (await saved(A)).objects.length === 3);

    step('Criterion 14');
    // Criterion 14: A's pointer and selection appear in B with A's label.
    await tool(A, 'Select');
    // The rectangles have no fill: a click selects on the outline.
    let p = at(A, 300, 240);
    await A.page.mouse.click(p.x, p.y);
    await A.page.mouse.move(p.x + 40, p.y + 120, { steps: 4 });
    timings.presence = await until('A\'s chip in B', async () => (await chip(B, 'Anna').count()) >= 1, 5000);
    await shot(B, '04-b-sees-anna');

    step('Criterion 15');
    // Criterion 15: B selects A's first rectangle; A hovering it sees "Ben is editing this".
    await tool(B, 'Select');
    p = at(B, 300, 240);
    await B.page.mouse.click(p.x, p.y);
    await settle(B.page, 300);
    await A.page.mouse.move(at(A, 300, 250).x, at(A, 300, 250).y, { steps: 3 });
    await until('marker in A', async () => (await chip(A, 'Ben is editing this').count()) >= 1, 5000);
    await shot(A, '05-a-sees-marker');
    await B.page.keyboard.press('Escape');

    step('Criterion 7');
    // Criterion 7: A drags its second rectangle; B sees it move during the drag.
    const beforeDrag = (await saved(B)).objects.map((o) => JSON.stringify(o));
    const clip = { x: B.canvas.x + 500, y: B.canvas.y + 200, width: 25, height: 60 };
    // Pressed on the top outline, right of the clip, so A's pointer is never inside it.
    p = at(A, 560, 200);
    await A.page.mouse.move(p.x, p.y);
    await A.page.mouse.down();
    await settle(A.page, 600);
    const before = await B.page.screenshot({ clip });
    await A.page.mouse.move(p.x + 200, p.y + 100, { steps: 12 });
    await until('live drag in B', async () => !(await B.page.screenshot({ clip })).equals(before), 3000);
    await shot(B, '06-b-sees-drag');
    await A.page.mouse.up();
    await until('the drag lands in B as one changed object', async () => {
      const after = (await saved(B)).objects.map((o) => JSON.stringify(o));
      return after.length === beforeDrag.length && after.filter((o) => !beforeDrag.includes(o)).length === 1;
    });
    await tab(B, 'History');
    assert.equal((await panel(B).innerText()).match(/\bMove\b/g)?.length ?? 0, 1, 'one history step for the drag');

    step('Criterion 17');
    // Criterion 17: B offline; the pill says so within 5 s, A loses B's pointer.
    await B.page.mouse.move(at(B, 700, 500).x, at(B, 700, 500).y, { steps: 3 });
    await until('B\'s chip in A', async () => (await chip(A, 'Ben').count()) >= 1, 5000);
    await B.context.setOffline(true);
    const offStart = Date.now();
    await until('B offline', async () => (await pill(B).innerText()).includes('Offline, changes are kept'), 5000);
    timings.offline = Date.now() - offStart;
    await until('B gone in A', async () => (await chip(A, 'Ben').count()) === 0, 8000);
    await shot(B, '07-b-offline');

    step('Criterion 18');
    // Criterion 18: edits on both sides while B is offline, then all of them on both.
    const countBefore = (await saved(A)).objects.length;
    for (let i = 0; i < 3; i += 1) await rect(B, 100 + 70 * i, 650, 40, 40);
    for (let i = 0; i < 3; i += 1) await rect(A, 100 + 70 * i, 760, 40, 40);
    assert.equal((await saved(B)).objects.length, countBefore + 3, 'B keeps editing offline');
    await B.context.setOffline(false);
    await B.page.evaluate(() => window.dispatchEvent(new Event('online')));
    const backStart = Date.now();
    await until('merged', async () => (await saved(A)).objects.length === countBefore + 6 && (await saved(B)).objects.length === countBefore + 6, 10000);
    timings.merge = Date.now() - backStart;
    await shot(B, '08-b-merged');

    step('Criterion 32');
    // Criterion 32: A removes B; Remove ignores the first 400 ms; epoch 1; B is told.
    await tab(A, 'Share');
    // Remove is armed (customer feedback on PR #281): the first press arms it (no sentence), a
    // second press inside 400 ms does nothing, one after it removes.
    const removeBen = panel(A).getByRole('button', { name: 'Remove Ben', exact: true });
    await removeBen.click();
    await removeBen.click();
    await settle(A.page, 100);
    assert.equal(await participants(A).count(), 2, 'a press in the first 400 ms does not remove');
    await settle(A.page, 450);
    await removeBen.click();
    await until('A lists only itself', async () => (await participants(A).count()) === 1);
    await until('B told', async () => (await B.page.getByText('You were removed from this workspace. Your copies stay on this device.').count()) >= 1, 8000);
    await shot(A, '09-a-removed-b');
    await shot(B, '10-b-removed');
    const kept = await saved(B);
    assert.equal(kept.objects.length, countBefore + 6, 'B keeps its copy');
    await rect(A, 600, 650, 40, 40);
    await settle(A.page, 1500);
    assert.equal((await saved(B)).objects.length, countBefore + 6, 'B gets no later change');

    assert.deepEqual([...A.errors, ...B.errors], []);
    console.log(`PASS browser 0092 on a shared workspace: invite person, join, live edits both ways, presence and marker, live drag, offline and merge, removal; timings ${JSON.stringify(timings)}`);
  } catch (error) {
    for (const d of [A, B]) {
      if (d) {
        await shot(d, `failed-${d.label}`).catch(() => undefined);
        console.error(`${d.label} panel: ${await panel(d).innerText().catch(() => '?')}`);
        console.error(`${d.label} errors: ${d.errors.join(' | ')}`);
      }
    }
    console.error(`relay log: ${relay.log()}`);
    throw error;
  } finally {
    await browser.close();
  }
  // Criterion 24 in the journey's own run: neither a label nor document JSON in the relay's data
  // directory or log.
  const files = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)]));
  const stored = [...files(relay.data).map((f) => fs.readFileSync(f)), Buffer.from(relay.log())];
  for (const needle of ['Anna', '"objects"', 'rect']) {
    assert(stored.every((bytes) => !bytes.includes(Buffer.from(needle))), `the relay holds "${needle}"`);
  }
  console.log(`relay data directory ${relay.data}: ${stored.length - 1} files, no plaintext found`);
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

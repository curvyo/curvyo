// Browser journey of `specs/0097-membership-lifecycle` slice 1 (plan task 19, criterion 36): a
// local `curvyo-sync-server` on a temporary data directory and three browser contexts A (Anna),
// B (Ben) and C (Cleo), each its own device.
//
// Document: A shares, B and C join; there is no second-admin line (customer 2026-11-09); A makes
// Ben an admin (criterion 2, armed, no sentence) and a member again (3); A, the only admin, finds
// Leave and Step down greyed (14); A removes Ben, whose Removed block is its heading only, with
// no Check and no "Removed or left" list on A (add back dropped, customer 2026-11-09); A makes a
// new invite and Ben joins again and syncs; Ben leaves and A changes the keys (11, 12).
// Workspace: a faked desktop host (`support/fake-workspace-host.cjs`) for A and B; A removes Ben,
// makes a new invite and Ben joins again into a new folder and syncs. The relay's data holds no
// label, name or content (31).
//
// Needs `target/debug/curvyo-sync-server` (`cargo build -p curvyo-sync-server`) and a production
// preview (`npm run build`, `npx vite preview --host localhost --port 1421`); starts the preview
// itself unless CURVYO_BROWSER_URL is set. Playwright as in README.md. Screenshots:
// target/0097-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const { FAKE_HOST, MARKER, makeHost } = require('./support/fake-workspace-host.cjs');
const { assertPanelFits } = require('./support/panel-overflow.cjs');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0097-browser');
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

/** One device; with `host`, a faked desktop host on real folders. */
async function device(browser, url, label, host = false) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(`${label}: ${e.stack || e.message}`));
  const d = { context, page, errors, label, host: null, saves: 0 };
  if (host) {
    d.host = makeHost(out, label);
    await page.exposeFunction('__host', (cmd, args, bytes) => d.host.handle(cmd, args, bytes));
    await page.addInitScript(FAKE_HOST);
  }
  await page.goto(url, { waitUntil: 'networkidle' });
  d.canvas = await page.locator('canvas').first().boundingBox();
  return d;
}

const panel = (d) => d.page.locator('[role=tabpanel]');
const tab = async (d, name) => { await d.page.getByRole('tab', { name: new RegExp(`^${name}`) }).click(); await settle(d.page); };
const tool = async (d, name) => { await d.page.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(d.page); };
const at = (d, x, y) => ({ x: d.canvas.x + x, y: d.canvas.y + y });
const participants = (d) => panel(d).getByRole('list', { name: 'Participants' }).locator('li');
const row = (d, label) => participants(d).filter({ hasText: label });
const shot = (d, name) => d.page.screenshot({ path: path.join(out, `${name}.png`) });
const shown = (d, text) => d.page.getByText(text, { exact: true }).count().then((n) => n > 0);
const exists = (d, ...rel) => fs.existsSync(path.join(d.host.state.root, ...rel));


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

/** Saves a browser document through the File tab (a download). */
async function download(d) {
  await tab(d, 'File');
  const file = d.page.waitForEvent('download');
  await panel(d).getByRole('button', { name: 'Save', exact: true }).click();
  d.saves += 1;
  await (await file).saveAs(path.join(out, `${d.label}-${d.saves}.curvyo`));
}

/** The server and the name shown to others, then waits for "Connected". */
async function setUp(d, address, name) {
  await tab(d, 'Share');
  const server = panel(d).getByLabel('Server', { exact: true });
  await server.fill(`ws://${address}`);
  await server.press('Enter');
  const field = panel(d).getByLabel('Name shown to others', { exact: true }).first();
  await field.fill(name);
  await field.press('Enter');
}

/** A creates an invite link in its Share tab. */
async function invite(d) {
  await tab(d, 'Share');
  const done = panel(d).getByRole('button', { name: 'Done', exact: true });
  if ((await done.count()) > 0) await done.click();
  await panel(d).getByRole('button', { name: 'Invite', exact: true }).click();
  const field = panel(d).getByLabel('Invite link', { exact: true });
  await field.waitFor();
  return field.inputValue();
}

/** The armed act `name` (customer feedback on PR #281): the first press arms it and nothing
 * else shows, a second press inside the 400 ms guard does nothing, one after it acts. */
async function armAct(d, name) {
  const button = panel(d).getByRole('button', { name, exact: true });
  await button.click();
  await settle(d.page, 450);
  await button.click();
  return button;
}

const role = async (d, label) => (await row(d, label).innerText()).split('\n').map((l) => l.trim()).find((l) => /^(Admin|Member)/.test(l)) ?? '';

/** `d` opens the invite link `link` of a document as `name`. */
async function joinDocument(d, link, name) {
  await tab(d, 'Share');
  await panel(d).getByRole('button', { name: /^Open shared document/ }).click();
  await panel(d).getByLabel('Invite link', { exact: true }).fill(link);
  const field = panel(d).getByLabel('Name shown to others', { exact: true }).last();
  await field.fill(name);
  await field.press('Enter');
  await panel(d).getByRole('button', { name: 'Join', exact: true }).click();
  // A removed copy with unsaved changes asks first (the File tab's check): it is not needed.
  const discard = panel(d).getByRole('button', { name: 'Discard', exact: true });
  if (await discard.waitFor({ timeout: 1500 }).then(() => true, () => false)) {
    await settle(d.page, 450);
    await discard.click();
  }
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
  const all = [];
  try {
    const A = await device(browser, url, 'A');
    const B = await device(browser, url, 'B');
    const C = await device(browser, url, 'C');
    all.push(A, B, C);

    /** How long the UI took to show each change after the press (ms), for the lead's report. */
    const timings = {};
    step('Share, join twice; no second-admin line (customer 2026-11-09)');
    await rect(A, 300, 200, 120, 80);
    await download(A);
    await setUp(A, relay.address, 'Anna');
    await panel(A).getByText('Connected', { exact: true }).waitFor();
    // Share asks nothing (customer 2026-11-09: no confirm sentences in the Share tab).
    await panel(A).getByRole('button', { name: 'Share', exact: true }).click();
    timings.shareConnected = await until('A connected', async () => (await panel(A).getByText('Connected', { exact: true }).count()) === 1, 10000);
    for (const [d, name] of [[B, 'Ben'], [C, 'Cleo']]) {
      await joinDocument(d, await invite(A), name);
      timings[`${name}Joined`] = await until(`${name} joined`, async () => (await row(A, name).count()) === 1, 15000);
    }
    await until('A names Ben and Cleo', async () => (await row(A, 'Ben').count()) === 1 && (await row(A, 'Cleo').count()) === 1, 8000);
    await tab(A, 'Share');
    assert.equal(await panel(A).getByText(/^Only this device is an admin/).count(), 0, 'no second-admin line');
    assert.equal(await role(A, 'Ben'), 'Member');
    await shot(A, '01-a-one-admin');
    await assertPanelFits(A.page, '01-a-one-admin');

    step('Criterion 2: A makes Ben an admin (armed, no sentence); Cleo is told');
    const makeAdmin = panel(A).getByRole('button', { name: 'Make Ben an admin', exact: true });
    await makeAdmin.click();
    await settle(A.page, 200);
    assert.equal(await panel(A).getByText(/^Make Ben an admin\?/).count(), 0, 'no sentence under the armed button');
    assert.ok((await role(A, 'Ben')).startsWith('Member'), 'the first press only arms');
    await settle(A.page, 300);
    await makeAdmin.click();
    timings.makeAdmin = await until('Ben is an admin in A', async () => (await role(A, 'Ben')).startsWith('Admin'), 8000);
    await tab(C, 'Share');
    timings.cleoSeesRole = timings.makeAdmin + (await until('Cleo sees the change', async () => (await panel(C).getByText('Ben is now an admin').count()) >= 1, 8000));
    await shot(C, '02-c-ben-admin');
    await assertPanelFits(C.page, '02-c-ben-admin');

    step('Criterion 3: A makes Ben a member again');
    await armAct(A, 'Make Ben a member');
    timings.makeMember = await until('Ben is a member in A', async () => (await role(A, 'Ben')).startsWith('Member'), 8000);

    step('Criterion 14: the only admin cannot leave or step down; both are greyed with the reason');
    const leave = row(A, 'Anna').getByRole('button', { name: /^Leave / });
    const stepDown = row(A, 'Anna').locator('[data-slot="role"]');
    for (const greyed of [leave, stepDown]) {
      assert.equal(await greyed.getAttribute('aria-disabled'), 'true');
      assert.equal(await greyed.getAttribute('title'), 'You are the only admin');
      assert.equal(await greyed.getAttribute('aria-description'), 'You are the only admin');
      // A press does nothing: no sentence, no refusal text (customer 2026-11-09).
      await greyed.click({ force: true });
      await settle(A.page, 500);
      await greyed.click({ force: true });
    }
    assert.equal(await panel(A).getByText(/Make someone else an admin first/).count(), 0, 'no refusal sentence');
    assert.equal(await panel(A).getByText(/^Step down as admin\?/).count(), 0, 'nothing armed');
    assert.equal(await participants(A).count(), 3);
    assert.ok((await role(A, 'Anna')).startsWith('Admin'));

    step('Criterion 13: A removes Ben; Ben\'s Removed block is its heading; no Removed or left list');
    await armAct(A, 'Remove Ben');
    timings.remove = await until('A lists two', async () => (await participants(A).count()) === 2, 8000);
    timings.benTold = timings.remove + (await until('Ben is told', async () => shown(B, 'You were removed from this document. Your copy stays on this device.'), 8000));
    await tab(B, 'Share');
    await until('Ben sees the Removed block', async () => (await panel(B).getByText(/^You were removed from /).count()) >= 1, 8000);
    assert.equal(await panel(B).getByRole('button', { name: /^Check whether I was added back/ }).count(), 0, 'no Check');
    assert.equal(await panel(A).getByRole('button', { name: /^Removed or left/ }).count(), 0, 'no Removed or left list');
    await shot(B, '03-b-removed');
    await assertPanelFits(B.page, '03-b-removed');

    step('Share tab feedback 3: A makes a new invite; the removed Ben joins again and syncs');
    await joinDocument(B, await invite(A), 'Ben');
    timings.rejoin = await until('A lists Ben again', async () => (await row(A, 'Ben').count()) === 1, 15000);
    await until('Ben lists three', async () => {
      await tab(B, 'Share');
      return (await participants(B).count()) === 3;
    }, 15000);
    await until('Ben is connected', async () => (await panel(B).getByText('Connected', { exact: true }).count()) >= 1, 15000);
    assert.equal(await panel(B).getByText(/^You were removed from /).count(), 0, 'the old removal no longer blocks');
    await shot(B, '04-b-joined-again');

    step('Criteria 11 and 12: Ben leaves (armed); A sees the rotation pending and changes the keys');
    await tab(B, 'Share');
    const benLeave = row(B, 'Ben').locator('[data-slot="action"]');
    await benLeave.click();
    await settle(B.page, 450);
    await benLeave.click();
    timings.leave = await until('Ben left', async () => (await panel(B).getByText(/^You left /).count()) >= 1, 15000);
    await tab(A, 'Share');
    await until('A lists two', async () => (await participants(A).count()) === 2, 8000);
    await until('A sees the rotation pending', async () => (await panel(A).getByText('Ben left. The keys have not been changed yet.').count()) >= 1, 8000);
    await shot(A, '05-a-rotation-pending');
    await assertPanelFits(A.page, '05-a-rotation-pending');
    await panel(A).getByRole('button', { name: 'Change keys now', exact: true }).click();
    await until('the keys changed', () => shown(A, 'Keys changed.'), 8000);
    await until('the notice is gone', async () => (await panel(A).getByText('Ben left. The keys have not been changed yet.').count()) === 0, 8000);

    step('Share tab feedback 3: a workspace; A removes Ben, Ben joins again by a new invite and syncs');
    const WA = await device(browser, url, 'WA', true);
    const WB = await device(browser, url, 'WB', true);
    all.push(WA, WB);
    await tab(WA, 'File');
    await panel(WA).getByRole('button', { name: 'Open workspace…', exact: true }).click();
    await panel(WA).getByRole('treegrid', { name: 'Workspace documents' }).waitFor();
    await panel(WA).getByRole('region', { name: 'Workspace' }).getByLabel('New document', { exact: true }).click();
    const docName = panel(WA).getByRole('textbox', { name: 'Name of the new document' });
    await docName.fill('Lid');
    await docName.press('Enter');
    await settle(WA.page, 400);
    await rect(WA, 300, 200, 80, 60);
    await tab(WA, 'File');
    await panel(WA).getByRole('button', { name: 'Save', exact: true }).click();
    await settle(WA.page, 300);
    await setUp(WA, relay.address, 'Anna');
    await panel(WA).getByText('Connected', { exact: true }).first().waitFor();
    await panel(WA).getByRole('button', { name: 'Share workspace', exact: true }).click();
    timings.workspaceShared = await until('A shared the workspace', async () => {
      await tab(WA, 'Share');
      return (await panel(WA).getByText(/^Shared on /).count()) >= 1;
    }, 30000);
    const wsLink = await invite(WA);
    await tab(WB, 'Share');
    await panel(WB).getByRole('button', { name: /^Join shared workspace/ }).click();
    await panel(WB).getByLabel('Invite link', { exact: true }).fill(wsLink);
    const wbName = panel(WB).getByLabel('Name shown to others', { exact: true }).last();
    await wbName.fill('Ben');
    await wbName.press('Enter');
    await panel(WB).getByRole('button', { name: /^Choose folder/ }).click();
    await panel(WB).getByRole('button', { name: 'Join', exact: true }).click();
    timings.workspaceJoined = await until('Ben has the file', async () => WB.host.state.root !== null && exists(WB, 'Lid.curvyo'), 30000);
    await until('A lists Ben', async () => {
      await tab(WA, 'Share');
      return (await row(WA, 'Ben').count()) === 1;
    }, 15000);
    await armAct(WA, 'Remove Ben');
    timings.workspaceRemove = await until('Ben keeps the ended record', async () => exists(WB, MARKER, 'share.ended.json') && !exists(WB, MARKER, 'share.json'), 15000);
    await until('Ben is told', async () => (await WB.page.getByText('You were removed from this workspace. Your copies stay on this computer.').count()) >= 1, 15000);
    const wsAgain = await invite(WA);
    WB.host.state.joinName = 'Joined box again';
    await tab(WB, 'Share');
    await panel(WB).getByRole('button', { name: /^Join shared workspace/ }).click();
    await panel(WB).getByLabel('Invite link', { exact: true }).fill(wsAgain);
    await panel(WB).getByRole('button', { name: /^Choose folder/ }).click();
    await panel(WB).getByRole('button', { name: 'Join', exact: true }).click();
    timings.workspaceRejoin = await until('Ben has the file again', async () => WB.host.state.root !== null && WB.host.state.root.endsWith('Joined box again') && exists(WB, 'Lid.curvyo'), 30000);
    await until('A lists Ben again', async () => {
      await tab(WA, 'Share');
      return (await row(WA, 'Ben').count()) === 1;
    }, 15000);
    await until('Ben lists two', async () => {
      await tab(WB, 'Share');
      return (await participants(WB).count()) === 2;
    }, 15000);
    await shot(WB, '07-wb-joined-again');
    await assertPanelFits(WB.page, '07-wb-joined-again');

    step('Share tab feedback round 4: Anna makes Ben admin, steps down and leaves; Ben, the only admin, keeps the workspace block, invites and can stop sharing');
    await tab(WA, 'Share');
    await armAct(WA, 'Make Ben an admin');
    await until('Ben is an admin in A', async () => (await role(WA, 'Ben')).startsWith('Admin'), 8000);
    const stepDownA = row(WA, 'Anna').locator('[data-slot="role"]');
    await stepDownA.click();
    await settle(WA.page, 450);
    await stepDownA.click();
    await until('Anna is a member', async () => (await role(WA, 'Anna')).startsWith('Member'), 8000);
    const leaveA = row(WA, 'Anna').locator('[data-slot="action"]');
    await leaveA.click();
    await settle(WA.page, 450);
    await leaveA.click();
    await until('Ben lists only himself', async () => {
      await tab(WB, 'Share');
      return (await participants(WB).count()) === 1;
    }, 15000);
    await shot(WB, '08-wb-only-admin');
    assert.equal(await panel(WB).getByRole('region', { name: /^Workspace / }).count(), 1, 'the workspace block shows');
    assert.equal(await panel(WB).getByRole('button', { name: 'Invite', exact: true }).count(), 1, 'Ben can invite');
    await assertPanelFits(WB.page, '08-wb-only-admin');
    // Stop sharing, armed, for the only participant left: the workspace is local again.
    const stop = panel(WB).getByRole('button', { name: 'Stop sharing', exact: true });
    await stop.click();
    await settle(WB.page, 450);
    assert.ok(exists(WB, MARKER, 'share.json'), 'the first press only arms');
    await stop.click();
    timings.stopSharing = await until('the marker has no share files', async () => !exists(WB, MARKER, 'share.json'), 10000);
    await until('Ben\'s workspace is not shared', async () => {
      await tab(WB, 'Share');
      return (await panel(WB).getByText('Not shared', { exact: true }).count()) >= 1;
    }, 10000);
    await shot(WB, '09-wb-stopped');

    assert.deepEqual(all.flatMap((d) => d.errors), []);
    console.log(`timings (ms): ${JSON.stringify(timings)}`);
    console.log('PASS browser 0097: roles armed without sentences, last admin greyed, removal without add back, a removed device joins again by a new invite (document and workspace), leave and change keys');
  } catch (error) {
    for (const d of all) {
      await shot(d, `failed-${d.label}`).catch(() => undefined);
      console.error(`${d.label} panel: ${await panel(d).innerText().catch(() => '?')}`);
      console.error(`${d.label} errors: ${d.errors.join(' | ')}`);
    }
    console.error(`relay log: ${relay.log()}`);
    throw error;
  } finally {
    await browser.close();
  }
  // Criterion 31: no label, document or workspace name, and no content in the relay's data
  // directory or log.
  const files = (dir) => fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => (e.isDirectory() ? files(path.join(dir, e.name)) : [path.join(dir, e.name)]));
  const stored = [...files(relay.data).map((f) => fs.readFileSync(f)), Buffer.from(relay.log())];
  for (const needle of ['Anna', 'Ben', 'Cleo', 'Box job', 'Lid', '"objects"']) {
    assert(stored.every((bytes) => !bytes.includes(Buffer.from(needle))), `the relay holds "${needle}"`);
  }
  console.log(`relay data directory ${relay.data}: ${stored.length - 1} files, no plaintext found`);
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

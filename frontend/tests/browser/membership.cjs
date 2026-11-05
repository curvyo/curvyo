// Browser journey of `specs/0097-membership-lifecycle` slice 1 (plan task 19, criterion 36): a
// local `curvyo-sync-server` on a temporary data directory and three browser contexts A (Anna),
// B (Ben) and C (Cleo), each its own device.
//
// Document: A shares, B and C join; the second-admin line shows and goes when A makes Ben an admin
// (criteria 2, 7); A makes Ben a member again (3); A, the only admin, cannot leave (14); A removes
// Ben, whose Removed block checks "not yet" (13, 20); A adds Ben back with Full history and Ben
// syncs again (17, 18, 21); A removes and adds Ben back once more and Ben keeps it as it is: Ben
// left, A sees the rotation pending and changes the keys (11, 12, 21). The first check after A
// lists Ben again says "added back" (26): A's list waits for the relay's confirmation (ADR 0041
// decision 11). Through one relay a mutual removal removes one admin; the frozen room of criteria
// 27 to 29 comes from crossed demotions, which `curvyo-sync-core` tests (`sim_confirmation`)
// cover. Workspace: a faked desktop host (`support/fake-workspace-host.cjs`)
// for A and B; A removes Ben and adds Ben back with no history choice, Ben syncs again (23). The
// relay's data holds no label, name or content (31).
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

const ONE_ADMIN = 'Only this device is an admin. If you lose it, nobody can add or remove people. Make someone else an admin.';

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

/** The server and the name shown to others, then waits for "Reachable". */
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
  await panel(d).getByRole('button', { name: 'Create link', exact: true }).click();
  const field = panel(d).getByLabel('Invite link', { exact: true });
  await field.waitFor();
  return field.inputValue();
}

/** A confirm opened by `opener` (accessible name), confirmed by `action` after the guard. */
async function confirmAct(d, opener, group, action) {
  await panel(d).getByRole('button', { name: opener, exact: true }).click();
  const ask = panel(d).getByRole('group', { name: group, exact: true });
  await ask.waitFor();
  await settle(d.page, 450);
  await ask.getByRole('button', { name: action, exact: true }).click();
  return ask;
}

const role = async (d, label) => (await row(d, label).innerText()).split('\n').map((l) => l.trim()).find((l) => /^(Admin|Member)/.test(l)) ?? '';

/** The admin opens Removed or left and adds `label` back; `full` checks the history choice. */
async function addBack(d, label, history) {
  await tab(d, 'Share');
  const disclosure = panel(d).getByRole('button', { name: /^Removed or left \(\d+\)$/ });
  if ((await disclosure.getAttribute('aria-expanded')) !== 'true') await disclosure.click();
  await panel(d).getByRole('button', { name: `Add back ${label}`, exact: true }).click();
  const dialog = panel(d).getByRole('group', { name: `Add back ${label}`, exact: true });
  await dialog.waitFor();
  const radios = dialog.getByRole('radiogroup', { name: 'History' });
  if (history) {
    assert.equal(await dialog.getByRole('radio', { name: /^Full history/ }).getAttribute('aria-checked'), 'true', 'Full history is preselected');
  } else {
    assert.equal(await radios.count(), 0, 'a workspace has no history choice');
  }
  await dialog.getByRole('button', { name: 'Add back', exact: true }).click();
  return dialog;
}

/** The removed or left device presses Check; returns once an answer shows. */
async function check(d) {
  await tab(d, 'Share');
  await panel(d).getByRole('button', { name: 'Check whether I was added back', exact: true }).click();
  // The answer of an earlier check goes while this one runs ("Checking..." or the prompt).
  await until(`${d.label} checks`, async () => (await panel(d).getByRole('button', { name: 'Checking...', exact: true }).count()) > 0
    || (await panel(d).getByText(/added (you )?back to .*Sync again\?$/).count()) > 0, 5000).catch(() => undefined);
  await until(`${d.label} has an answer`, async () => (await panel(d).getByRole('button', { name: 'Checking...', exact: true }).count()) === 0 &&
    (await shown(d, 'Not added back yet.')) || (await panel(d).getByText(/added (you )?back to .*Sync again\?$/).count()) > 0, 15000);
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
    step('Share, join twice; the second-admin line on the only admin (criterion 7)');
    await rect(A, 300, 200, 120, 80);
    await download(A);
    await setUp(A, relay.address, 'Anna');
    await panel(A).getByText('Reachable', { exact: true }).waitFor();
    await panel(A).getByRole('button', { name: 'Share', exact: true }).click();
    await panel(A).getByRole('group', { name: /^Upload this document to/ }).getByRole('button', { name: 'Share', exact: true }).click();
    timings.shareConnected = await until('A connected', async () => (await panel(A).getByText('Connected', { exact: true }).count()) === 1, 10000);
    for (const [d, name] of [[B, 'Ben'], [C, 'Cleo']]) {
      const link = await invite(A);
      await tab(d, 'Share');
      await panel(d).getByRole('button', { name: /^Open shared document/ }).click();
      await panel(d).getByLabel('Invite link', { exact: true }).fill(link);
      const field = panel(d).getByLabel('Name shown to others', { exact: true }).last();
      await field.fill(name);
      await field.press('Enter');
      await panel(d).getByRole('button', { name: 'Join', exact: true }).click();
      timings[`${name}Joined`] = await until(`${name} joined`, async () => (await row(A, name).count()) === 1, 15000);
    }
    await until('A names Ben and Cleo', async () => (await row(A, 'Ben').count()) === 1 && (await row(A, 'Cleo').count()) === 1, 8000);
    await tab(A, 'Share');
    assert.ok(await shown(A, ONE_ADMIN), 'the only admin sees the second-admin line');
    assert.equal(await role(A, 'Ben'), 'Member');
    await shot(A, '01-a-one-admin');
    await assertPanelFits(A.page, '01-a-one-admin');

    step('Criteria 2 and 7: A makes Ben an admin; the line goes; Cleo is told');
    await confirmAct(A, 'Make Ben an admin', 'Make Ben an admin', 'Make admin');
    timings.makeAdmin = await until('Ben is an admin in A', async () => (await role(A, 'Ben')).startsWith('Admin'), 8000);
    await until('the nudge is gone', async () => !(await shown(A, ONE_ADMIN)), 5000);
    await tab(C, 'Share');
    timings.cleoSeesRole = timings.makeAdmin + (await until('Cleo sees the change', async () => (await panel(C).getByText('Ben is now an admin').count()) >= 1, 8000));
    await shot(C, '02-c-ben-admin');
    await assertPanelFits(C.page, '02-c-ben-admin');

    step('Criterion 3: A makes Ben a member again');
    await confirmAct(A, 'Make Ben a member', 'Make Ben a member', 'Make member');
    timings.makeMember = await until('Ben is a member in A', async () => (await role(A, 'Ben')).startsWith('Member'), 8000);

    step('Criterion 14: the only admin cannot leave');
    const leave = row(A, 'Anna').getByRole('button', { name: /^Leave / });
    assert.equal(await leave.getAttribute('aria-disabled'), 'true');
    assert.equal(await leave.getAttribute('title'), 'You are the only admin');
    // aria-disabled keeps it focusable and pressable: the press says why (ux-notes 3.3).
    await leave.click({ force: true });
    await until('A is told why', async () => shown(A, 'You are the only admin. Make someone else an admin first.'), 3000);
    assert.equal(await panel(A).getByRole('group', { name: /^Leave / }).count(), 0, 'no leave confirm opens');
    assert.equal(await participants(A).count(), 3);

    step('Criteria 13 and 20: A removes Ben; Ben checks: not yet');
    await confirmAct(A, 'Remove Ben', 'Remove Ben', 'Remove');
    timings.remove = await until('A lists two', async () => (await participants(A).count()) === 2, 8000);
    timings.benTold = timings.remove + (await until('Ben is told', async () => shown(B, 'You were removed from this document. Your copy stays on this device.'), 8000));
    await tab(B, 'Share');
    await until('Ben sees the Removed block', async () => (await panel(B).getByText(/^You were removed from /).count()) >= 1, 8000);
    assert.ok(await shown(B, `Contacts ${relay.address}.`), 'the block says what Check contacts');
    await check(B);
    assert.ok(await shown(B, 'Not added back yet.'));
    await shot(B, '03-b-not-yet');
    await assertPanelFits(B.page, '03-b-not-yet');

    step('Criteria 17, 18 and 21: A adds Ben back with Full history; Ben syncs again');
    await addBack(A, 'Ben', true);
    timings.addBack = await until('A lists Ben again', async () => (await row(A, 'Ben').count()) === 1, 8000);
    // Criterion 26: A lists Ben only once the relay holds the add (ADR 0041 decision 11), so
    // the first check finds it.
    await check(B);
    assert.ok(await shown(B, 'Anna added you back to this document. Sync again?'), 'the first check after "Added" says added back');
    await shot(B, '04-b-added-back');
    await assertPanelFits(B.page, '04-b-added-back');
    await panel(B).getByRole('button', { name: 'Sync again', exact: true }).click();
    timings.syncAgain = await until('Ben is a participant again', async () => (await participants(B).count()) === 3, 15000);
    // PR #281 review: it took 4.6 s while the first save held the page and the connection timed out.
    assert.ok(timings.syncAgain < 1000, `Sync again took ${timings.syncAgain} ms`);

    step('Criteria 11, 12 and 21: removed and added back again, Ben keeps it as it is');
    await tab(A, 'Share');
    await confirmAct(A, 'Remove Ben', 'Remove Ben', 'Remove');
    await until('A lists two', async () => (await participants(A).count()) === 2, 8000);
    await until('Ben sees the Removed block', async () => {
      await tab(B, 'Share');
      return (await panel(B).getByRole('button', { name: 'Check whether I was added back', exact: true }).count()) === 1;
    }, 8000);
    await addBack(A, 'Ben', true);
    await until('A lists Ben again', async () => (await row(A, 'Ben').count()) === 1, 8000);
    await check(B);
    await panel(B).getByRole('button', { name: 'Keep as it is', exact: true }).click();
    timings.keepAsItIs = await until('Ben left', async () => (await panel(B).getByText(/^You left /).count()) >= 1, 15000);
    await until('A lists two', async () => (await participants(A).count()) === 2, 8000);
    await until('A sees the rotation pending', async () => (await panel(A).getByText('Ben left. The keys have not been changed yet.').count()) >= 1, 8000);
    await shot(A, '05-a-rotation-pending');
    await assertPanelFits(A.page, '05-a-rotation-pending');
    const epochBefore = await panel(A).getByText(/^Key epoch \d+$/).innerText();
    await panel(A).getByRole('button', { name: 'Change keys now', exact: true }).click();
    await until('the keys changed', async () => (await panel(A).getByText(/^Key epoch \d+$/).innerText()) !== epochBefore, 8000);
    await until('the notice is gone', async () => (await panel(A).getByText('Ben left. The keys have not been changed yet.').count()) === 0, 8000);

    step('Criterion 23: a workspace; A removes Ben and adds Ben back with no history choice');
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
    await panel(WA).getByText('Reachable', { exact: true }).first().waitFor();
    await panel(WA).getByRole('button', { name: 'Share workspace', exact: true }).click();
    await panel(WA).getByRole('group', { name: /^Upload the workspace / }).getByRole('button', { name: 'Share', exact: true }).click();
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
    await confirmAct(WA, 'Remove Ben', 'Remove Ben', 'Remove');
    timings.workspaceRemove = await until('Ben keeps the ended record', async () => exists(WB, MARKER, 'share.ended.json') && !exists(WB, MARKER, 'share.json'), 15000);
    await check(WB);
    assert.ok(await shown(WB, 'Not added back yet.'));
    const dialog = await addBack(WA, 'Ben', false);
    assert.equal(await dialog.count(), 0, 'the dialog closed');
    await until('A lists Ben again', async () => (await row(WA, 'Ben').count()) === 1, 8000);
    await check(WB);
    await shot(WB, '07-wb-added-back');
    await assertPanelFits(WB.page, '07-wb-added-back');
    await panel(WB).getByRole('button', { name: 'Sync again', exact: true }).click();
    timings.workspaceSyncAgain = await until('Ben syncs the workspace again', async () => exists(WB, MARKER, 'share.json') && !exists(WB, MARKER, 'share.ended.json'), 15000);
    await until('Ben lists two', async () => {
      await tab(WB, 'Share');
      return (await participants(WB).count()) === 2;
    }, 15000);
    assert.ok(exists(WB, 'Lid.curvyo'), 'Ben keeps the document');

    assert.deepEqual(all.flatMap((d) => d.errors), []);
    console.log(`timings (ms): ${JSON.stringify(timings)}`);
    console.log('PASS browser 0097: roles and the nudge, last admin, removal and check, add back with Full history, keep as it is and change keys, workspace add back without a history choice');
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

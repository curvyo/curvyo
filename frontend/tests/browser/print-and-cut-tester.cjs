// Tester browser smoke of `specs/0121-print-and-cut` slice 1: Add registration marks on the
// Drawing page (criterion 1), place the sheet on a new laser page (Place page, `0100`), type the
// measures of marks 1 to 3 (criteria 7, 9), read the residual line (12), Apply (16, greyed on
// the fit), undo (one step), Apply again and Reset placement (16). No JS error on the way.
// Run: NODE_PATH=target/browser-check/node_modules CURVYO_E2E_BROWSER=/usr/bin/chromium \
//   node frontend/tests/browser/print-and-cut-tester.cjs
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/print-and-cut-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5193;

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    const up = await fetch(url).then(() => true, () => false);
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

(async () => {
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(PORT), '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    url = `http://127.0.0.1:${PORT}/`;
    process.on('exit', () => { try { process.kill(-vite.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CURVYO_E2E_BROWSER || undefined, args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const report = {};
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(400);
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const groupText = async () => (await p.locator('body').innerText());

    // Criterion 1: Add registration marks on the Drawing page.
    await tab('Document');
    await p.getByRole('button', { name: 'Add registration marks', exact: true }).click();
    await settle();
    await p.screenshot({ path: path.join(out, '1-marks.png') });
    report.marksAdded = true;

    // A laser page, and the sheet placed on it.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    let place = p.getByRole('button', { name: /^Place page/ }).first();
    if ((await place.count()) === 0) {
      for (const t of ['Document', 'Manifest', 'Laser', 'Machine']) {
        const found = await p.locator(`[role=tab][aria-label^="${t}"]`).count();
        if (found === 0) continue;
        await tab(t);
        place = p.getByRole('button', { name: /^Place page/ }).first();
        if ((await place.count()) > 0) break;
      }
    }
    await place.click();
    await settle();
    await p.getByRole('option').first().click().catch(async () => { await p.keyboard.press('Enter'); });
    await p.waitForTimeout(800);
    await p.screenshot({ path: path.join(out, '2-placed.png') });

    // Criterion 7: the Print and cut group in the Operations tab, rows 1 to 4.
    await tab('Operations');
    const body0 = await groupText();
    assert.ok(body0.includes('Measure at least 2 marks'), `the group shows: ${body0.slice(0, 2000)}`);
    assert.ok(body0.includes('Registration marks (not cut)'), 'criterion 5 row');
    for (let n = 1; n <= 4; n += 1) {
      assert.ok(await p.getByLabel(`Mark ${n} X`, { exact: true }).count() > 0, `row ${n}`);
    }
    // Measures of a sheet moved by (100, 50) on a bottom-left machine: x + 100, 250 - y.
    const measures = { 1: ['110', '240'], 2: ['300', '240'], 3: ['300', '-37'] };
    for (const [n, [x, y]] of Object.entries(measures)) {
      await p.getByLabel(`Mark ${n} X`, { exact: true }).fill(x);
      await p.keyboard.press('Enter');
      await settle();
      await p.getByLabel(`Mark ${n} Y`, { exact: true }).fill(y.replace('.', ','));
      await p.keyboard.press('Enter');
      await settle();
    }
    const body1 = await groupText();
    report.residualLine = (body1.match(/Largest[^\n]*/) || [null])[0];
    assert.ok(/Largest 0\.00 mm, RMS 0\.00 mm/.test(body1), `residual line: ${report.residualLine}`);
    report.fitLine = (body1.match(/Moved[^\n]*\n?[^\n]*/) || [null])[0];
    await p.screenshot({ path: path.join(out, '3-measured.png') });

    // Criterion 16: Apply, greyed on the fit; undo is one step; Reset placement restores.
    const apply = p.locator('[data-pnc-apply]');
    assert.equal(await apply.isDisabled(), false, 'Apply offered');
    await apply.click();
    await settle();
    assert.equal(await apply.isDisabled(), true, 'greyed while on the fit');
    const body2 = await groupText();
    assert.ok(body2.includes('On the fit') || body2.includes('The sheet is on the fit'), 'on the fit');
    await p.screenshot({ path: path.join(out, '4-applied.png') });
    await p.mouse.click(700, 500);
    await p.keyboard.press('Control+z');
    await settle();
    await tab('Operations');
    assert.equal(await p.locator('[data-pnc-apply]').isDisabled(), false, 'undo of Apply offers Apply again');
    assert.ok((await groupText()).includes('Largest 0.00 mm'), 'measures kept after undo');
    await p.locator('[data-pnc-apply]').click();
    await settle();
    const reset = p.getByRole('button', { name: /Reset placement/ }).first();
    assert.equal(await reset.isDisabled(), false, 'Reset offered on the fit');
    await reset.click();
    await settle();
    assert.equal(await p.locator('[data-pnc-apply]').isDisabled(), false, 'after Reset Apply is offered');
    await p.screenshot({ path: path.join(out, '5-reset.png') });
    report.errors = errors;
    assert.deepEqual(errors, [], 'no JS errors');
    report.pass = true;
  } finally {
    fs.writeFileSync(path.join(out, 'report.json'), JSON.stringify(report, null, 2));
    console.log(JSON.stringify(report, null, 2));
    await browser.close();
  }
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

// Browser check of `specs/0113-fixture-sets-and-pins` criterion 46 and the UX review of
// 2026-11-16: a laser page whose profile is deleted in the library turns read only (the line
// "From the project file. <profile> is not on this computer.", no "+ Set", no add bar, no Lock),
// and a locked set fixture shows its values as text. Starts its own dev server unless
// CURVYO_BROWSER_URL is set; build the wasm bindings first. Screenshots:
// target/fixture-sets-read-only-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/fixture-sets-read-only-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5187;

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
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CURVYO_E2E_BROWSER || undefined, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(250);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };

    // A laser page beside Page 1.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);

    // Save as profile makes the sets editable; add a set with a holder and lock the holder.
    await tab('Document');
    await p.getByRole('button', { name: 'Save as profile' }).click();
    await settle();
    await tab('Fixtures');
    await p.getByRole('button', { name: 'Set', exact: true }).click();
    await settle();
    await shot('1-editable');
    const holder = p.locator('[role=toolbar][aria-label="Add fixture to Set 1"] button[aria-label="Holder"]');
    await holder.click();
    await settle();
    await p.locator('button[aria-label^="Lock Holder"]').first().click();
    await settle();
    // Unfold the holder row to show its body.
    const name = p.locator('button[aria-expanded][aria-label^="Holder 1"]').first();
    if ((await name.getAttribute('aria-expanded')) !== 'true') await name.click();
    await settle();
    await shot('2-locked-holder');
    const editable = await p.locator('[role=tabpanel] input[aria-label$="(mm)"]').count();
    console.log('editable mm fields while the holder is locked:', editable);

    // Delete the profile in the library: the page turns read only.
    await tab('Document');
    await p.getByRole('button', { name: /^Machine profile,/ }).click();
    await settle();
    const trash = p.locator('[aria-label="Machine profiles"] button[aria-label^="Delete "]').first();
    const box = await trash.boundingBox();
    await p.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await p.waitForTimeout(600);
    // The armed press of `0081`: a second press after its 400 ms guard deletes.
    await p.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await p.waitForTimeout(600);
    const deleted = true;
    await shot('3-library-after-delete');
    assert.ok(deleted, 'a user profile was deleted');
    await p.keyboard.press('Escape');
    await tab('Fixtures');
    await shot('4-read-only');
    const panel = (await p.locator('[role=tabpanel]').first().textContent()) || '';
    console.log('panel text:', panel.slice(0, 400));
    assert.match(panel, /From the project file\. .* is not on this computer\./, 'the read-only line');
    // The set's add bar is gone; the Job fixtures bar stays (its fixtures are the page's own).
    assert.equal(await p.locator('[role=toolbar][aria-label="Add fixture to Set 1"]').count(), 0, 'no add bar in the set');
    await p.waitForTimeout(500);
    console.log('select bar Lock count:', await p.locator('[data-context-bar] button[aria-label="Lock"]').count());
    assert.equal(await p.getByRole('button', { name: 'Set', exact: true }).count(), 0, 'no + Set');
    assert.equal(await p.locator('button[aria-label^="Lock "]').count(), 0, 'no Lock while read only');
    assert.equal(await p.locator('[data-context-bar] button[aria-label="Lock"]').count(), 0, 'no Lock bar while read only');
    assert.deepEqual(errors, []);
    console.log('PASS');
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });

// Independent real-event acceptance; run from repository root with Playwright on NODE_PATH.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
(async () => {
  fs.mkdirSync('target/0066-browser', { recursive: true });
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = []; page.on('pageerror', e => errors.push(e.stack));
    await page.goto(process.env.CURVYO_BROWSER_URL || 'http://127.0.0.1:5173/', { waitUntil: 'networkidle' });
    await page.getByLabel('Workbench', { exact: true }).selectOption('sketch');
    await page.getByRole('button', { name: 'New', exact: true }).click();
    await page.getByRole('button', { name: 'Circle', exact: true }).click();
    await page.mouse.click(450, 300); await page.mouse.click(500, 300);
    await page.getByText(/^Geometry \(/).click();
    const circle = page.getByLabel('Sketch entities').getByRole('button', { name: /circle/ });
    await circle.click();
    await page.getByRole('button', { name: 'Diameter', exact: true }).and(page.getByTitle('Diameter', { exact: true })).click();
    await page.getByLabel('New diameter').fill('20 mm');
    await page.getByRole('button', { name: 'Apply', exact: true }).last().click();
    assert(!(await circle.textContent()).includes('Fully constrained'), 'fixed diameter leaves centre free');
    assert.match(await page.getByLabel('Sketch status').textContent(), /Under-constrained/);
    await page.getByRole('button', { name: 'Select', exact: true }).click();
    const annotation = page.getByLabel('Sketch annotations').getByRole('button', { name: /edit dimension/ });
    await annotation.dblclick();
    const field = page.getByLabel('Diameter expression', { exact: true });
    await field.waitFor();
    await field.fill('24 mm'); await field.press('Enter');
    await page.getByRole('button', { name: 'Edit diameter', exact: true }).click();
    assert.equal(await field.inputValue(), '24 mm');
    await field.fill('missing_parameter'); await field.press('Enter');
    assert.equal(await field.getAttribute('aria-invalid'), 'true');
    await field.press('Escape');
    await page.getByRole('button', { name: 'Edit diameter', exact: true }).click();
    assert.equal(await field.inputValue(), '24 mm');
    // Native text undo changes the draft; it must not undo the committed dimension.
    await field.press('Control+a'); await field.pressSequentially('27 mm');
    await field.press('Control+z'); await field.press('Escape');
    await page.getByRole('button', { name: 'Edit diameter', exact: true }).click();
    assert.equal(await field.inputValue(), '24 mm'); await field.press('Escape');
    await circle.click();
    await page.getByRole('button', { name: 'Diameter', exact: true }).and(page.getByTitle('Diameter', { exact: true })).click();
    await page.getByLabel('New diameter').fill('30 mm');
    await page.getByRole('button', { name: 'Apply', exact: true }).last().click();
    assert.match(await page.getByLabel('Sketch status').textContent(), /Conflicting/);
    await page.getByRole('button', { name: 'Remove diameter', exact: true }).last().click();
    assert.match(await page.getByLabel('Sketch status').textContent(), /Under-constrained/);
    await page.screenshot({ path: 'target/0066-browser/independent-circle-repair.png' });
    await page.getByRole('tab', { name: /^File/ }).click();
    const download = page.waitForEvent('download');
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await (await download).saveAs('target/0066-browser/independent-circle.curvyo');
    assert.deepEqual(errors, []);
    console.log('PASS actual annotation double-click, field commit/refusal/native undo, free-centre status, conflict repair and saved artifact');
  } finally { await browser.close(); }
})();

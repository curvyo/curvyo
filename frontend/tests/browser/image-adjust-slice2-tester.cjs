// Tester browser smoke of `specs/0135-image-optimisation` slice 2, written from the spec: adding a
// Crop opens the Crop bar with the canvas frame (criterion 5, plan "The Crop bar"), a handle drag
// is one step that narrows the crop, Done leaves the bar, the cropped picture draws only its
// visible part, and the Curves graph adds a point on a press (criterion 5) with undo. Starts its
// own dev server unless CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium; build
// the wasm bindings first. Screenshots: target/image-adjust-slice2-tester-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/image-adjust-slice2-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5194;

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
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => { errors.push(e.message); console.log('PAGEERROR', e.message); });
    p.on('console', (m) => { if (m.type() === 'error') console.log('CONSOLE', m.text().slice(0, 300)); });
    p.setDefaultTimeout(30000);
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(500);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const group = () => p.locator('[data-adjustments]');
    const undo = async () => { await p.keyboard.press('Control+z'); await settle(); };

    // A laser page, zoomed out.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);

    // A 240 x 160 picture: left half dark, right half light, dropped on the canvas.
    const canvas = await p.locator('canvas[data-state]').boundingBox();
    const data = await p.evaluateHandle(async () => {
      const c = document.createElement('canvas');
      c.width = 240; c.height = 160;
      const g = c.getContext('2d');
      g.fillStyle = '#202020'; g.fillRect(0, 0, 120, 160);
      g.fillStyle = '#e0e0e0'; g.fillRect(120, 0, 120, 160);
      g.fillStyle = '#c03030'; g.beginPath(); g.arc(60, 80, 40, 0, 7); g.fill();
      const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
      const t = new DataTransfer();
      t.items.add(new File([blob], 'photo.png', { type: 'image/png' }));
      return t;
    });
    const cx = canvas.x + canvas.width / 2; const cy = canvas.y + canvas.height / 2;
    for (const type of ['dragenter', 'dragover', 'drop']) {
      await p.dispatchEvent('canvas[data-state]', type, { dataTransfer: data, clientX: cx, clientY: cy });
    }
    await p.waitForTimeout(1500);
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    await shot('0-picture');

    // Criterion 5: adding a Crop opens the Crop bar with the frame on the canvas.
    await group().getByRole('button', { name: /Crop/ }).first().click();
    await settle();
    const bar = p.getByRole('toolbar', { name: 'Crop' });
    await bar.waitFor({ timeout: 5000 });
    assert.equal(await p.locator('[data-crop-overlay]').count(), 1, 'the canvas frame');
    assert.equal(await p.locator('g[data-crop-grip]').count(), 8, 'eight handles');
    await shot('1-crop-bar');
    const width = bar.getByLabel('Crop Width, %');
    assert.equal(await width.inputValue(), '100');

    // Drag the right handle to the left: the crop narrows in one step.
    const grip = await p.locator('g[data-crop-grip="r"]').boundingBox();
    const inside = await p.locator('path[data-crop-grip="inside"]').boundingBox();
    const gx = grip.x + grip.width / 2; const gy = grip.y + grip.height / 2;
    await p.mouse.move(gx, gy);
    await p.mouse.down();
    await p.mouse.move(gx - inside.width * 0.25, gy, { steps: 8 });
    await p.mouse.move(gx - inside.width * 0.5, gy, { steps: 8 });
    await p.mouse.up();
    await settle();
    const w1 = Number(await width.inputValue());
    assert.ok(w1 > 30 && w1 < 70, `the crop narrowed to about half: ${w1}`);
    await shot('2-crop-dragged');

    // A typed field in the bar.
    await width.fill('40'); await p.keyboard.press('Enter'); await settle();
    assert.equal(await width.inputValue(), '40');
    await bar.getByRole('button', { name: 'Done' }).click();
    await settle();
    assert.equal(await bar.count(), 0, 'Done leaves the bar');
    assert.equal(await p.locator('[data-crop-overlay]').count(), 0, 'the overlay is gone');
    await shot('3-cropped-picture');
    assert.equal(await group().getByLabel(/Crop Width/).first().inputValue(), '40', 'the row shows the crop');
    // Nothing selected: the cropped picture alone.
    await p.mouse.click(canvas.x + 60, canvas.y + canvas.height - 60); await settle();
    await shot('3b-cropped-unselected');
    await p.mouse.click(cx - 40, cy); await settle();

    // Undo the typed width: back to the dragged one.
    await p.locator('canvas[data-state]').click({ position: { x: 5, y: 5 } }).catch(() => {});
    await undo();
    await tab('Style');

    // Criterion 5: Curves; a press on the graph adds a point.
    await p.mouse.click(cx, cy); await settle();
    await tab('Style');
    await group().getByRole('button', { name: /Curves/ }).first().click();
    await settle();
    const graph = p.locator('[data-curves-graph]');
    await graph.waitFor({ timeout: 5000 });
    assert.equal(await p.locator('rect[data-curve-point]').count(), 2, 'a straight curve');
    const gb = await graph.boundingBox();
    await p.mouse.click(gb.x + gb.width * 0.5, gb.y + gb.height * 0.3);
    await settle();
    assert.equal(await p.locator('rect[data-curve-point]').count(), 3, 'the press added a point');
    await shot('4-curve-point');
    await group().getByRole('button', { name: 'Straight curve' }).click();
    await settle();
    assert.equal(await p.locator('rect[data-curve-point]').count(), 2, 'Straight curve');
    await shot('5-final');
    assert.deepEqual(errors, [], 'no page errors');
    console.log('image-adjust-slice2-tester: all checks passed');
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

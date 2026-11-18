// Tester browser check for the gradient fill (`specs/0124-gradient-fill`), independent of
// gradient-fill.cjs: no rail tool (1), the Gradient item makes the canvas pixels inside the
// rectangle a shade, not one colour, left darker or lighter than right (2, 9), the handles are
// drawn on the line ends (6), an End-handle drag changes the pixels and is one undo step (6, 18),
// a double-click on the End handle opens the entry box with the Angle field and Enter writes
// 90 degrees so the shade runs top to bottom (7), Escape closes a box and writes nothing (7),
// and no page errors. Starts its own dev server on port 5187 unless CURVYO_BROWSER_URL is set;
// CURVYO_E2E_BROWSER names a Chromium. Screenshots: target/0124-browser-tester/.
const { chromium } = require('playwright');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0124-browser-tester');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5187;
const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok: Boolean(ok), detail });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `: ${detail}` : ''}`);
};

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then(() => true, () => false)) return;
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
  const errors = [];
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const canvasEl = p.locator('canvas[data-state]');
    await canvasEl.waitFor();
    const canvas = await canvasEl.boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
    const drag = async (a, b) => { await move(a[0], a[1]); await p.mouse.down(); await move(b[0], b[1]); await p.mouse.up(); };
    const settle = (ms = 200) => p.waitForTimeout(ms);
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    // Pixel read of the canvas at canvas-relative CSS points, from a screenshot (no GL readback).
    const pixels = async (pts) => {
      const png = await p.screenshot({ clip: canvas });
      return p.evaluate(async ({ data, pts }) => {
        const img = new Image();
        img.src = `data:image/png;base64,${data}`;
        await img.decode();
        const c = document.createElement('canvas');
        c.width = img.width; c.height = img.height;
        const g = c.getContext('2d');
        g.drawImage(img, 0, 0);
        return pts.map(([x, y]) => Array.from(g.getImageData(Math.round(x), Math.round(y), 1, 1).data).slice(0, 3));
      }, { data: png.toString('base64'), pts });
    };
    const grey = (c) => (c[0] + c[1] + c[2]) / 3;

    check('criterion 1: no rail tool for gradients', (await p.getByRole('button', { name: /gradient tool/i }).count()) === 0);
    await tool('Rectangle');
    await drag([300, 250], [800, 550]); await settle();
    await tool('Select');
    await move(550, 250); await p.mouse.down(); await p.mouse.up(); await settle();
    const kinds = p.getByRole('radiogroup', { name: 'Fill paint' });
    // The rectangle needs a fill: Colour first, then Gradient.
    await kinds.getByRole('radio', { name: 'Solid fill' }).click().catch(() => {});
    await settle();
    await kinds.getByRole('radio', { name: 'Gradient fill' }).click(); await settle(400);
    await p.screenshot({ path: path.join(shots, 'gradient.png') });
    const row = await pixels([[330, 330], [450, 330], [550, 330], [650, 330], [770, 330]]);
    const g = row.map(grey);
    const distinct = new Set(row.map((c) => c.join(','))).size;
    check('criterion 2, 9: the pixels across the shape are a shade, not uniform', distinct >= 4, JSON.stringify(row));
    check('criterion 2, 9: monotone from Start to End (towards white)', g.every((v, i) => i === 0 || v >= g[i - 1] - 2) && g[4] - g[0] > 60, JSON.stringify(g));
    const handles = p.locator('[data-gradient-handle]');
    check('criterion 6: two handles drawn', (await handles.count()) === 2, `${await handles.count()}`);
    const endBox = await p.locator('[data-gradient-handle="end"]').boundingBox();
    const startBox = await p.locator('[data-gradient-handle="start"]').boundingBox();
    const ex = endBox.x + endBox.width / 2 - canvas.x;
    const ey = endBox.y + endBox.height / 2 - canvas.y;
    const sx = startBox.x + startBox.width / 2 - canvas.x;
    check('criterion 4, 6: handles at the middles of the left and right edges', Math.abs(sx - 300) < 4 && Math.abs(ex - 800) < 4 && Math.abs(ey - 400) < 4, `${sx},${ex},${ey}`);
    // Drag the End handle to the middle: the right half becomes all End colour.
    await drag([ex, ey], [550, 400]); await settle(400);
    const after = (await pixels([[650, 330], [770, 330]])).map(grey);
    check('criterion 6: the End drag changes the picture (pad beyond the end)', Math.abs(after[0] - after[1]) < 3 && after[0] > 200, JSON.stringify(after));
    await p.keyboard.press('Control+z'); await settle(400);
    const undone = (await pixels([[650, 330], [770, 330]])).map(grey);
    check('criterion 18: one undo brings the shade back', undone[1] - undone[0] > 10, JSON.stringify(undone));
    // Double-click the End handle: the entry box with the Angle field.
    const endBox2 = await p.locator('[data-gradient-handle="end"]').boundingBox();
    const q = { x: endBox2.x + endBox2.width / 2, y: endBox2.y + endBox2.height / 2 };
    await p.mouse.dblclick(q.x, q.y); await settle(400);
    const box = p.locator('[data-gradient-entry]');
    check('criterion 7: the entry box opens', (await box.count()) === 1);
    const field = p.locator('[data-gradient-field]');
    check('criterion 7: it has the Angle field', /angle/i.test((await field.getAttribute('aria-label')) || ''), await field.getAttribute('aria-label'));
    await field.fill('90'); await p.keyboard.press('Enter'); await settle(500);
    await p.screenshot({ path: path.join(shots, 'angle90.png') });
    const col = (await pixels([[450, 270], [450, 360], [450, 530]])).map(grey);
    const rowAfter = (await pixels([[330, 330], [770, 330]])).map(grey);
    check('criterion 7: Angle 90 shades top to bottom', Math.abs(col[0] - col[2]) > 60 && Math.abs(rowAfter[0] - rowAfter[1]) < 4, `col ${JSON.stringify(col)} row ${JSON.stringify(rowAfter)}`);
    check('criterion 3: 90 points up the page (End at the top is white)', col[0] > col[2], JSON.stringify(col));
    // Escape closes a box and writes nothing.
    const endBox3 = await p.locator('[data-gradient-handle="end"]').boundingBox();
    await p.mouse.dblclick(endBox3.x + endBox3.width / 2, endBox3.y + endBox3.height / 2); await settle(300);
    await p.locator('[data-gradient-field]').fill('0');
    await p.keyboard.press('Escape'); await settle(400);
    const col2 = (await pixels([[450, 270], [450, 530]])).map(grey);
    check('criterion 7: Escape writes nothing', (await p.locator('[data-gradient-entry]').count()) === 0 && Math.abs(col2[0] - col[0]) < 3, JSON.stringify(col2));
    // Colour switches the handles away.
    await kinds.getByRole('radio', { name: 'Solid fill' }).click(); await settle(300);
    check('criterion 6: the handles appear only while Gradient is chosen', (await p.locator('[data-gradient-handle]').count()) === 0);
    check('no page errors', errors.length === 0, errors.join(' | '));
  } catch (e) {
    check('script ran to the end', false, String(e && e.stack || e));
  } finally {
    await browser.close();
  }
  const failed = results.filter((r) => !r.ok);
  console.log(`${results.length - failed.length}/${results.length} passed`);
  process.exit(failed.length ? 1 : 0);
})();

// Browser check of the Measure tool and Vector dimensions (`specs/0070-measure-and-dimension`):
// a measurement with its readout, a dimension of each kind left with Shift, a dimension following
// its geometry, unlinked after a delete, converted to paths with the Guide button, and Undo.
// Starts its own dev server unless CURVYO_BROWSER_URL is set. Screenshots: target/0070-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0070-browser');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5179;

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    const up = await fetch(url).then(() => true, () => false);
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

(async () => {
  let vite = null;
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(PORT), '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    url = `http://127.0.0.1:${PORT}/`;
    const server = vite;
    process.on('exit', () => { try { process.kill(-server.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const canvas = await p.locator('canvas').first().boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
    const click = async (x, y) => { await move(x, y); await p.mouse.down(); await p.mouse.up(); };
    const settle = () => p.waitForTimeout(120);
    const labels = () => p.locator('[data-measure-dimension]');
    const shot = (name) => p.screenshot({ path: path.join(shots, `${name}.png`) });
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };

    // Geometry: an L path with the Pen, a rectangle, a circle (Ctrl).
    await tool('Pen');
    await click(500, 300); await click(700, 300); await click(700, 450);
    await p.keyboard.press('Enter'); await settle();
    await tool('Rectangle');
    await move(800, 250); await p.mouse.down(); await move(950, 350); await p.mouse.up(); await settle();
    await tool('Ellipse');
    await move(500, 550); await p.keyboard.down('Control'); await p.mouse.down(); await move(600, 650); await p.mouse.up(); await p.keyboard.up('Control'); await settle();

    // Criterion 2: M selects the tool; the rail button is pressed.
    await p.mouse.move(canvas.x + 300, canvas.y + 800);
    await p.keyboard.press('Escape'); await p.keyboard.press('Escape');
    await p.keyboard.press('m'); await settle();
    const rail = p.getByRole('button', { name: /^Measure tool/ });
    assert.equal(await rail.getAttribute('aria-pressed'), 'true');
    assert.equal(await p.getByRole('toolbar', { name: 'More tools' }).count(), 1);
    // Customer feedback on PR #243: a plain crosshair and no chip before the first pick.
    await move(300, 780); await settle();
    assert.equal(await p.locator('canvas[data-state]').evaluate((e) => getComputedStyle(e).cursor), 'crosshair');
    assert.equal(await p.locator('[data-measure-chip]').count(), 0, 'no chip before a pick');

    // Criterion 1: a measurement writes nothing and shows the readout.
    await click(500, 300); await move(650, 300); await click(700, 300); await settle();
    const readout = p.locator('[data-measure-chip="readout"]');
    await readout.waitFor();
    assert.match(await readout.innerText(), /mm/);
    assert.match(await p.locator('[data-measure-speech]').innerText(), /millimetres/);
    assert.equal(await labels().count(), 0, 'a measurement stores nothing');
    await shot('01-measurement');
    await p.keyboard.press('Escape'); await settle();

    // Criterion 4: Shift at the second pick leaves a dimension of each kind.
    const leave = async (a, b, place) => {
      await click(a[0], a[1]);
      await move(b[0], b[1]);
      await p.keyboard.down('Shift'); await settle();
      await click(b[0], b[1]);
      await p.keyboard.up('Shift');
      await move(place[0], place[1]); await settle();
      await click(place[0], place[1]); await settle();
    };
    await leave([800, 250], [950, 350], [970, 230]); // aligned (corner quadrant)
    await leave([800, 250], [950, 250], [875, 200]); // horizontal (above)
    await leave([800, 250], [800, 350], [760, 300]); // vertical (left)
    assert.equal(await labels().count(), 3);
    // Radius on the circle, Alt switches to diameter.
    await move(600, 600); await p.keyboard.down('Shift'); await click(600, 600); await p.keyboard.up('Shift');
    await move(660, 520); await settle(); await shot('02-radius-placing');
    await click(660, 520); await settle();
    await move(550, 650); await p.keyboard.down('Shift'); await click(550, 650); await p.keyboard.up('Shift');
    await p.keyboard.press('Alt'); await move(420, 680); await settle();
    await click(420, 680); await settle();
    // Angle: the two segments of the L path.
    // Shift is read at the second segment pick (criterion 4).
    await move(560, 300); await click(560, 300);
    await move(700, 420); await p.keyboard.down('Shift'); await click(700, 420); await p.keyboard.up('Shift');
    await move(640, 340); await settle(); await click(640, 340); await settle();
    const texts = await labels().allInnerTexts();
    assert.equal(texts.length, 6, texts.join(' | '));
    assert.ok(texts.some((t) => t.startsWith('R ')), texts.join(' | '));
    assert.ok(texts.some((t) => t.startsWith('⌀ ')), texts.join(' | '));
    assert.ok(texts.some((t) => t.endsWith('°')), texts.join(' | '));
    assert.ok(texts.some((t) => t.startsWith('↔ ')), texts.join(' | '));
    await shot('03-dimensions');

    // Criterion 11: moving the rectangle moves its dimensions.
    await tool('Select');
    const before = await labels().first().boundingBox();
    await move(850, 350); await p.mouse.down(); await move(850, 430); await p.mouse.up(); await settle();
    const after = await labels().first().boundingBox();
    assert.ok(Math.abs(after.y - before.y - 80) < 3, `${before.y} -> ${after.y}`);
    await shot('04-followed');

    // Criterion 12: deleting the L path unlinks the angle.
    await click(500, 300); await p.keyboard.press('Delete'); await settle();
    assert.equal(await p.locator('[data-measure-dimension="unlinked"]').count(), 1);
    await shot('05-unlinked');
    await p.keyboard.press('Control+z'); await settle();
    assert.equal(await p.locator('[data-measure-dimension="unlinked"]').count(), 0, 'undo links it again');

    // Criterion 10: the Guide button converts a selected dimension; Undo brings it back.
    const first = await labels().first().boundingBox();
    await p.mouse.click(first.x + first.width / 2, first.y + first.height / 2); await settle();
    const guide = p.getByRole('button', { name: 'Guide', exact: true });
    assert.equal(await guide.getAttribute('aria-pressed'), 'true');
    await guide.click(); await settle();
    assert.equal(await labels().count(), 5);
    await shot('06-converted');
    await p.keyboard.press('Control+z'); await settle();
    assert.equal(await labels().count(), 6);
    await shot('07-undone');

    // Customer feedback on PR #243: primitives measure by their edges, vertices, centres and
    // axis ends; a right click does nothing.
    await tool('Ellipse');
    await move(750, 560); await p.mouse.down(); await move(950, 640); await p.mouse.up(); await settle();
    await tool('Polygon/star');
    await move(300, 700); await p.mouse.down(); await move(380, 780); await p.mouse.up(); await settle();
    await tool('Measure');
    const chip = async () => ((await readout.count()) ? (await readout.innerText()).split('\n')[0].trim() : '');
    await click(820, 430); await move(830, 470); await settle(); // the moved rectangle's top edge
    const side = await chip();
    assert.match(side, /^39\.6\d* mm, (0|180)°$/, side);
    await p.keyboard.press('Escape'); await settle();
    await click(950, 600); await move(750, 600); await click(750, 600); await settle(); // ellipse ends
    const across = await chip();
    assert.match(across, /^52\.9\d* mm, 180°$/, across);
    await p.keyboard.press('Escape'); await settle();
    await click(850, 560); await p.keyboard.down('Shift'); await click(850, 640); await p.keyboard.up('Shift');
    // Placed right of the ellipse, its text over the east handle (950, 600) of the ellipse's box.
    await move(953, 600); await settle(); await click(953, 600); await settle();
    assert.equal(await labels().count(), 7, 'a dimension between two axis ends');
    await click(300, 740); await move(330, 760); await settle(); // centre of the polygon
    const centre = await chip();
    await p.keyboard.press('Escape'); await settle();
    assert.match(centre, /^\d/, centre);
    const count = await labels().count();
    await move(600, 800); await p.mouse.down({ button: 'right' }); await p.mouse.up({ button: 'right' });
    await move(650, 830); await settle();
    assert.equal(await p.locator('[data-measure-chip]').count(), 0, 'a right click starts nothing');
    await p.keyboard.down('Shift'); await move(820, 430);
    await p.mouse.down({ button: 'right' }); await p.mouse.up({ button: 'right' }); await p.keyboard.up('Shift');
    await move(850, 470); await settle();
    assert.equal(await p.locator('[data-measure-chip]').count(), 0, 'a right click on an edge picks nothing');
    assert.equal(await labels().count(), count, 'a right click leaves no dimension');
    await shot('08-primitives');

    // Customer feedback on PR #243: a resize handle wins over a dimension on it.
    await tool('Select');
    await click(750, 600); await settle(); // the ellipse, by its west end
    const textAt = async () => (await labels().last().boundingBox()).x;
    const before7 = await textAt();
    await move(950, 600); await p.mouse.down(); await move(1010, 600); await p.mouse.up(); await settle();
    assert.equal(await p.getByRole('button', { name: 'Guide', exact: true }).getAttribute('aria-pressed'), 'false', 'the ellipse stays selected');
    const moved = (await textAt()) - before7;
    assert.ok(Math.abs(moved - 30) < 4, `the dimension follows the scaled ellipse's centre: ${moved}`);
    await shot('09-handle-wins');
    assert.deepEqual(errors, []);
    console.log('PASS browser 0070: M and rail; measurement and readout; aligned, horizontal, vertical, radius, diameter, angle dimensions; follow; unlink and relink; convert and undo; crosshair and no chip before a pick; rectangle edge, ellipse axis ends, polygon centre; right click does nothing; resize handle wins over a dimension; no page errors');
  } finally {
    await browser.close();
    if (vite?.pid) { try { process.kill(-vite.pid, 'SIGKILL'); } catch { /* gone */ } }
  }
})().catch((e) => { console.error(e); process.exit(1); });

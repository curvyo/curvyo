// Browser check of the pen modes and width editing (`specs/0110-pen-modes-variable-stroke`):
// each mode draws a visible curve while placing points and after finishing (criteria 1 to 3),
// width editing on the canvas with a live sideways drag (criterion 4, frame intervals reported),
// and Spiro and B-spline curves on a laser page in their look and as drawn; at 800 x 600 and at
// desktop size. Also the UX review of 2026-11-10: U1 an unsolvable Spiro commits straight lines
// and its notice ends, U4 the Select bar keeps its width beside the Width chip, U6 Pen bar buttons
// of 32 px with tooltips below, U7 the Pen bar clears the page label, U8 refusals beside the
// pointer, U9 the active profile is marked. Starts its own dev server unless CURVYO_BROWSER_URL is
// set.
// Screenshots: target/0110-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0110-browser');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5181;

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    const up = await fetch(url).then(() => true, () => false);
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

/** How many pixels in the CSS box differ from the box's corner colour (the page paper). */
async function inkIn(p, box) {
  const png = await p.screenshot({ clip: box });
  return p.evaluate(async (data) => {
    const image = new Image();
    image.src = `data:image/png;base64,${data}`;
    await image.decode();
    const c = new OffscreenCanvas(image.width, image.height);
    const g = c.getContext('2d');
    g.drawImage(image, 0, 0);
    const d = g.getImageData(0, 0, image.width, image.height).data;
    const [r0, g0, b0] = [d[0], d[1], d[2]];
    let n = 0;
    for (let i = 0; i < d.length; i += 4) {
      if (Math.abs(d[i] - r0) + Math.abs(d[i + 1] - g0) + Math.abs(d[i + 2] - b0) > 60) n += 1;
    }
    return n;
  }, png.toString('base64'));
}

async function journey(browser, url, viewport, tag) {
  const p = await browser.newPage({ viewport });
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
  console.log(tag, 'goto'); await p.goto(url, { waitUntil: 'load' }); await p.locator('canvas[data-state]').waitFor(); await p.waitForTimeout(1500); console.log(tag, 'loaded');
  const canvas = await p.locator('canvas[data-state]').boundingBox();
  const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
  const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
  const click = async (x, y) => { await move(x, y); await p.mouse.down(); await p.mouse.up(); };
  const settle = () => p.waitForTimeout(250);
  const shot = (name) => p.screenshot({ path: path.join(shots, `${tag}-${name}.png`) });
  const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
  // Ink between two clicked points, away from the nodes: the curve itself, not its markers.
  const band = (x0, x1, y0, y1) => ({ x: canvas.x + x0, y: canvas.y + y0, width: x1 - x0, height: y1 - y0 });
  const w = canvas.width;
  const h = canvas.height;
  const results = {};
  const overlaps = (a, b) => !!a && !!b && a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
  const near = p.locator('[data-near-pointer]');
  await tool('Pen');
  // U6 and U7: the Pen bar's buttons, its tooltips and the page label.
  const penBar = await p.getByRole('toolbar', { name: 'Pen options' }).boundingBox();
  const radioSizes = await p.getByRole('radiogroup', { name: 'Pen mode' }).getByRole('radio')
    .evaluateAll((els) => els.map((e) => { const r = e.getBoundingClientRect(); return Math.min(r.width, r.height); }));
  await p.getByRole('radio', { name: 'Spiro', exact: true }).hover(); await p.waitForTimeout(800);
  const tip = await p.locator('[data-radix-popper-content-wrapper]').first().boundingBox();
  await shot('pen-tooltip');
  await move(w * 0.5, h * 0.5);
  results.penBar = {
    minButton: Math.round(Math.min(...radioSizes)),
    tipBelow: !!tip && tip.y >= penBar.y + penBar.height - 1,
    overLabel: overlaps(penBar, await p.locator('[data-page-label]').first().boundingBox()),
  };
  for (const [i, mode] of ['Lines', 'Spiro', 'B-spline', 'Bézier'].entries()) {
    await p.getByRole('radio', { name: mode, exact: true }).click(); await settle();
    const y = h * (0.2 + 0.18 * i);
    const xs = [w * 0.35, w * 0.5, w * 0.65];
    const before = await inkIn(p, band(xs[0] + 10, xs[2] - 10, y - 60, y + 20));
    await click(xs[0], y); await click(xs[1], y - 40); await click(xs[2], y);
    await move(w * 0.8, y - 30); await settle();
    const live = await inkIn(p, band(xs[0] + 10, xs[2] - 10, y - 60, y + 20)) - before;
    await shot(`${mode}-live`);
    await p.keyboard.press('Enter'); await settle(); await settle();
    const done = await inkIn(p, band(xs[0] + 10, xs[2] - 10, y - 60, y + 20)) - before;
    await shot(`${mode}-done`);
    results[mode] = { live, done };
  }
  // U1: a Spiro whose live curve has no solution (the pointer as a sixth point of this zigzag)
  // shows the notice beside the pointer; Enter commits straight lines and the notice ends. Spiro
  // curves are similar under scaling, so the document's zigzag works at any zoom.
  await p.getByRole('radio', { name: 'Spiro', exact: true }).click(); await settle();
  const k = Math.min(w, h) / 320;
  const zig = [[10, 60], [50, 30], [85, 65], [120, 35], [143, 70]].map(([x, y]) => [w * 0.12 + x * k, h * 0.05 + y * k]);
  for (const [x, y] of zig) await click(x, y);
  await move(w * 0.12 + 160 * k, h * 0.05 + 98 * k); await settle(); await settle();
  const pointerAt = at(w * 0.12 + 160 * k, h * 0.05 + 98 * k);
  const nearBox = await near.boundingBox();
  const spiroNotice = { shown: (await near.count()) > 0 && /straight lines/.test(await near.innerText()), distance: nearBox ? Math.round(Math.hypot(nearBox.x - pointerAt.x, nearBox.y - pointerAt.y)) : -1 };
  await shot('spiro-unsolved-live');
  await p.keyboard.press('Enter'); await settle(); await settle();
  spiroNotice.afterEnter = await near.count();
  await shot('spiro-unsolved-done');
  await p.keyboard.press('Control+z'); await settle();
  results.spiroNotice = spiroNotice;
  // U8: a mode change while placing is refused beside the pointer.
  await p.getByRole('radio', { name: 'Bézier', exact: true }).click(); await settle();
  await click(w * 0.2, h * 0.9); await click(w * 0.3, h * 0.9);
  const spiroRadio = await p.getByRole('radio', { name: 'Spiro', exact: true }).boundingBox();
  await p.getByRole('radio', { name: 'Spiro', exact: true }).click(); await settle();
  const refusal = await near.boundingBox();
  results.modeRefusal = refusal ? Math.round(Math.hypot(refusal.x - (spiroRadio.x + spiroRadio.width / 2), refusal.y - (spiroRadio.y + spiroRadio.height / 2))) : -1;
  await shot('mode-refusal');
  await p.keyboard.press('Escape'); await settle();
  // Criterion 5: a profile button on the last (straight-segment) path changes its outline as
  // one undo step.
  await tool('Select');
  const y0 = h * 0.74;
  const xs0 = [w * 0.35, w * 0.5, w * 0.65];
  await click(xs0[0], y0); await settle();
  await p.getByRole('tab', { name: /^Style/ }).click(); await settle();
  const lineBand = band(xs0[0] - 15, xs0[0] + 25, y0 - 20, y0 + 20);
  const plain = await inkIn(p, lineBand);
  await p.getByRole('button', { name: 'Taper both', exact: true }).click(); await settle(); await settle();
  const tapered = await inkIn(p, lineBand);
  // U9: the profile the path has is marked; the row is named Edit.
  results.marked = {
    taper: await p.getByRole('button', { name: 'Taper both', exact: true }).getAttribute('aria-pressed'),
    editRow: await p.getByText('Edit', { exact: true }).count(),
  };
  await shot('taper-both');
  await p.keyboard.press('Control+z'); await settle(); await settle();
  const undone = await inkIn(p, lineBand);
  results.preset = { plain, tapered, undone };
  // Criterion 4: diamonds on the centre line, a ghost on hover, a live sideways drag, Done.
  console.log(tag, 'width');
  // U4: the Width chip does not squeeze the Select bar, and does not wrap.
  const selectBar = p.locator('[data-bar-area] > *').first();
  const barBefore = await selectBar.boundingBox();
  await p.getByRole('button', { name: 'Edit width', exact: true }).click(); await settle();
  const barAfter = await selectBar.boundingBox();
  const chip = await p.locator('[data-width-done]').boundingBox();
  results.selectBar = { widthBefore: Math.round(barBefore.width), widthAfter: Math.round(barAfter.width), chipHeight: chip ? Math.round(chip.height) : -1, dones: await p.getByRole('button', { name: 'Done', exact: true }).count() };
  const [ax, ay, bx, by] = [xs0[0], y0, xs0[1], y0 - 40];
  const mid = [(ax + bx) / 2, (ay + by) / 2];
  const len = Math.hypot(bx - ax, by - ay);
  const normal = [-(by - ay) / len, (bx - ax) / len];
  await move(mid[0], mid[1]); await settle();
  await shot('width-ghost');
  const spot = band(mid[0] - 30, mid[0] + 30, mid[1] - 30, mid[1] + 30);
  const thin = await inkIn(p, spot);
  await click(mid[0], mid[1]); await settle();
  await p.evaluate(() => {
    window.__frames = []; window.__rec = true;
    let last = performance.now();
    const tick = (t) => { window.__frames.push(t - last); last = t; if (window.__rec) requestAnimationFrame(tick); };
    requestAnimationFrame(tick);
  });
  const start = at(mid[0], mid[1]);
  await p.mouse.move(start.x, start.y); await p.mouse.down();
  for (let k = 1; k <= 24; k += 1) {
    const q = at(mid[0] + normal[0] * k, mid[1] + normal[1] * k);
    await p.mouse.move(q.x, q.y);
    await p.waitForTimeout(16);
  }
  const frames = await p.evaluate(() => { window.__rec = false; return window.__frames.slice(2); });
  await shot('width-drag-live');
  await p.mouse.up(); await settle();
  const wide = await inkIn(p, spot);
  await shot('width-dragged');
  const doneBar = p.locator('[data-width-done]').getByRole('button', { name: 'Done', exact: true });
  await doneBar.click(); await settle();
  const sorted = [...frames].sort((a, b) => a - b);
  results.width = {
    thin, wide,
    doneGone: await p.locator('[data-width-done]').count(),
    frameMedian: Math.round(sorted[Math.floor(sorted.length / 2)] ?? 0),
    frameMax: Math.round(sorted[sorted.length - 1] ?? 0),
  };
  // U8 from the panel: a profile on a rectangle (not a path) is refused beside the button.
  await tool('Rectangle');
  const r0 = at(w * 0.7, h * 0.85);
  const r1 = at(w * 0.8, h * 0.92);
  await p.mouse.move(r0.x, r0.y); await p.mouse.down(); await p.mouse.move(r1.x, r1.y, { steps: 4 }); await p.mouse.up(); await settle();
  await tool('Select');
  await click(w * 0.7, h * 0.85); await settle();
  await p.getByRole('tab', { name: /^Style/ }).click(); await settle();
  const taper = p.getByRole('button', { name: 'Taper start', exact: true });
  const taperBox = await taper.boundingBox();
  await taper.click(); await settle(); await settle();
  const panelRefusal = await near.boundingBox();
  // Beside the button: under it (kept inside the window, so measured by its span and its gap).
  const middle = taperBox.x + taperBox.width / 2;
  results.panelRefusal = panelRefusal && panelRefusal.x <= middle && middle <= panelRefusal.x + panelRefusal.width ? Math.round(Math.abs(panelRefusal.y - (taperBox.y + taperBox.height))) : -1;
  await shot('panel-refusal');
  await p.keyboard.press('Control+z'); await settle();
  // U5 and U10 (screenshots): the B-spline's control polygon and round smooth points in the Node
  // tool.
  await tool('Node');
  await click(w * 0.35, h * 0.56); await settle();
  await shot('node-bspline');
  await tool('Select');
  // Criterion 2 on a laser page: Spiro and B-spline curves in the operation look and as drawn.
  console.log(tag, 'laser');
  await p.getByRole('tab', { name: /^Document/ }).click(); await settle();
  await p.getByRole('button', { name: 'Add page', exact: true }).click(); await settle();
  const laserKind = p.getByRole('radio', { name: /Laser/ });
  if (await laserKind.count()) {
    await laserKind.first().click(); await settle();
    await p.getByRole('button', { name: 'Add', exact: true }).click(); await settle(); await settle();
  }
  await tool('Pen');
  const laser = {};
  for (const [i, mode] of ['Spiro', 'B-spline'].entries()) {
    await p.getByRole('radio', { name: mode, exact: true }).click(); await settle();
    const y = h * (0.5 + 0.22 * i);
    const xs = [w * 0.35, w * 0.5, w * 0.65];
    const area = band(xs[0] + 10, xs[2] - 10, y - 60, y + 20);
    const before = await inkIn(p, area);
    await click(xs[0], y); await click(xs[1], y - 40); await click(xs[2], y);
    await p.keyboard.press('Enter'); await settle(); await settle();
    laser[mode] = { looked: (await inkIn(p, area)) - before, area, before };
  }
  await shot('laser-looks');
  console.log(tag, 'view');
  await tool('Select');
  await p.getByRole('tab', { name: /^Operations/ }).click(); await settle();
  const drawn = p.getByRole('radio', { name: 'Drawn', exact: true });
  if (await drawn.count()) {
    await drawn.click(); await settle(); await settle();
    for (const mode of Object.keys(laser)) laser[mode].drawn = (await inkIn(p, laser[mode].area)) - laser[mode].before;
    await shot('laser-drawn');
  }
  for (const mode of Object.keys(laser)) { delete laser[mode].area; delete laser[mode].before; }
  results.laser = laser;
  await p.close();
  return { results, errors };
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
    for (const [viewport, tag] of [[{ width: 1440, height: 1000 }, 'desktop'], [{ width: 800, height: 600 }, 'small']]) {
      const { results, errors } = await journey(browser, url, viewport, tag);
      console.log(tag, JSON.stringify(results), errors.length ? `errors: ${errors.join(' | ')}` : '');
      const { preset, width, laser, penBar, spiroNotice, modeRefusal, marked, selectBar, panelRefusal, ...modes } = results;
      assert.ok(panelRefusal >= 0 && panelRefusal < 80, `${tag}: a panel refusal shows beside the button (${panelRefusal})`);
      assert.ok(penBar.minButton >= 32, `${tag}: Pen bar buttons are at least 32 px (${penBar.minButton})`);
      assert.ok(penBar.tipBelow, `${tag}: Pen bar tooltips open below the bar`);
      assert.ok(!penBar.overLabel, `${tag}: the Pen bar clears the page label`);
      assert.ok(spiroNotice.shown, `${tag}: an unsolvable Spiro says so`);
      assert.ok(spiroNotice.distance >= 0 && spiroNotice.distance < 80, `${tag}: beside the pointer (${spiroNotice.distance})`);
      assert.equal(spiroNotice.afterEnter, 0, `${tag}: Enter ends the Spiro notice`);
      assert.ok(modeRefusal >= 0 && modeRefusal < 80, `${tag}: a refused mode change shows beside the pointer (${modeRefusal})`);
      assert.equal(marked.taper, 'true', `${tag}: the applied profile is marked`);
      assert.ok(marked.editRow >= 1, `${tag}: the width row is named Edit`);
      assert.equal(selectBar.widthAfter, selectBar.widthBefore, `${tag}: the Width chip leaves the Select bar's width`);
      assert.ok(selectBar.chipHeight > 0 && selectBar.chipHeight <= 40, `${tag}: the Width chip is one line (${selectBar.chipHeight})`);
      assert.equal(selectBar.dones, 1, `${tag}: one Done`);
      assert.ok(width.wide > width.thin + 50, `${tag}: a sideways drag widens the stroke (${width.thin} -> ${width.wide})`);
      assert.equal(width.doneGone, 0, `${tag}: Done leaves width editing`);
      for (const [mode, ink] of Object.entries(laser)) {
        assert.ok(ink.looked > 20, `${tag}: ${mode} visible in its laser look (${ink.looked})`);
        assert.ok(ink.drawn > 20, `${tag}: ${mode} visible under View: Drawn (${ink.drawn})`);
      }
      assert.notEqual(preset.tapered, preset.plain, `${tag}: Taper both changes the outline`);
      assert.equal(preset.undone, preset.plain, `${tag}: one undo takes the profile back`);
      for (const [mode, ink] of Object.entries(modes)) {
        assert.ok(ink.live > 20, `${tag}: ${mode} curve visible while placing points (${ink.live})`);
        assert.ok(ink.done > 20, `${tag}: ${mode} curve visible after finishing (${ink.done})`);
      }
      assert.deepEqual(errors, [], `${tag}: no page errors`);
    }
  } finally {
    await browser.close();
  }
  console.log('pen-modes: ok');
})().catch((e) => { console.error(e); process.exit(1); });

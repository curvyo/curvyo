// Browser check of the pen modes and width editing (`specs/0110-pen-modes-variable-stroke`):
// each mode draws a visible curve while placing points and after finishing (criteria 1 to 3),
// width editing on the canvas with a live sideways drag (criterion 4, frame intervals reported),
// and Spiro and B-spline curves on a laser page in their look and as drawn; at 800 x 600 and at
// desktop size. Also the UX review of 2026-11-10: U1 an unsolvable Spiro commits straight lines
// and its notice ends, U4 the Select bar keeps its width beside the Width chip, U6 Pen bar buttons
// of 32 px with tooltips below, U7 the Pen bar clears the page label, U8 refusals beside the
// pointer, U9 the active profile is marked. Since `specs/0114`: no Spiro notice (the point
// follows the pointer at every event), Edit width in the tool bar and its Done, bigger diamonds, a double-click types a
// point's width, the wheel over a point sets its width without zooming; UX review 2026-11-12: the
// field and a wheel readout beside the diamond inside the canvas, the short refusal, the toggle's
// look. Starts its own dev server
// unless CURVYO_BROWSER_URL is set.
// Screenshots: target/0110-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0114-browser');
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
  // 0114 (customer 2026-11-12): circling the last of three Spiro points, the live end is at the
  // pointer at every event (ink right at the pointer): the curve is a function of the points.
  await p.getByRole('radio', { name: 'Spiro', exact: true }).click(); await settle();
  const R = { x0: w * 0.2, x1: w * 0.75, y0: h * 0.25, y1: h * 0.8 };
  const P = [[0, 1], [0.25, 0.2], [0.55, 0.8], [0.8, 0.15]].map(([a, b]) => [R.x0 + (R.x1 - R.x0) * a, R.y0 + (R.y1 - R.y0) * b]);
  for (let i = 0; i < 3; i += 1) { await click(...P[i]); await settle(); }
  const rad = Math.min(w, h) / 9;
  const circling = [];
  for (let i = 0; i <= 10; i += 1) { const t = i / 10; circling.push([P[3][0] + (P[2][0] + rad - P[3][0]) * t, P[3][1] + (P[2][1] - P[3][1]) * t]); }
  for (let i = 1; i <= 48; i += 1) { const a = (i / 48) * 2 * Math.PI; circling.push([P[2][0] + Math.cos(a) * rad, P[2][1] + Math.sin(a) * rad]); }
  let atPointer = 0; let trail = "";
  for (const [x, y] of circling) {
    const q = at(x, y); await p.mouse.move(q.x, q.y); await p.waitForTimeout(40);
    if ((await inkIn(p, { x: q.x - 4, y: q.y - 4, width: 8, height: 8 })) > 0) { atPointer += 1; trail += "o"; } else trail += ".";
  }
  await shot('spiro-circling');
  results.spiroCircling = { frames: circling.length, atPointer, trail };
  await p.keyboard.press('Escape'); await settle();
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
  // 0114: the Spiro zigzag with the pointer as a sixth point shows its curve and no notice.
  await p.getByRole('radio', { name: 'Spiro', exact: true }).click(); await settle();
  const k = Math.min(w, h) / 320;
  const zig = [[10, 60], [50, 30], [85, 65], [120, 35], [143, 70]].map(([x, y]) => [w * 0.12 + x * k, h * 0.05 + y * k]);
  for (const [x, y] of zig) await click(x, y);
  await move(w * 0.12 + 160 * k, h * 0.05 + 98 * k); await settle(); await settle();
  const spiroNotice = { shown: (await near.count()) > 0 && /straight lines|solved/.test(await near.innerText()) };
  await shot('spiro-zigzag-live');
  await p.keyboard.press('Enter'); await settle(); await settle();
  await shot('spiro-zigzag-done');
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
  // U9: the profile the path has is marked; 0114: Edit width is in the tool bar, not the panel.
  results.marked = {
    taper: await p.getByRole('button', { name: 'Taper both', exact: true }).getAttribute('aria-pressed'),
    panelEdit: await p.getByRole('tabpanel').getByRole('button', { name: 'Edit width', exact: true }).count(),
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
  const toggle = p.locator('[data-width-edit]');
  const toggleBefore = await toggle.boundingBox();
  await toggle.getByRole('button', { name: 'Edit width', exact: true }).click(); await settle();
  const barAfter = await selectBar.boundingBox();
  const chip = await toggle.boundingBox();
  const inside = (a, b) => !!a && !!b && a.x >= b.x - 1 && a.y >= b.y - 1 && a.x + a.width <= b.x + b.width + 1 && a.y + a.height <= b.y + b.height + 1;
  results.toggle = { shown: !!toggleBefore, inBar: inside(toggleBefore, barBefore), insideWindow: !!chip && chip.x + chip.width <= viewport.width };
  // UX review 2026-11-12: Edit width is a 14 px bordered text button like its neighbours, on
  // their line in the Select bar's second row.
  const toggleButton = toggle.getByRole('button').first();
  const strokeToPath = await p.getByRole('button', { name: 'Stroke to path', exact: true }).boundingBox();
  const toggleBox = await toggleButton.boundingBox();
  results.toggleLook = await toggleButton.evaluate((e) => {
    const c = getComputedStyle(e);
    return { font: parseFloat(c.fontSize), border: parseFloat(c.borderTopWidth) };
  });
  results.toggleLook.centreOff = strokeToPath && toggleBox ? Math.round(Math.abs((strokeToPath.y + strokeToPath.height / 2) - (toggleBox.y + toggleBox.height / 2))) : -1;
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
  // 0114 criterion 6: a double-click on the diamond types its width; Enter sets it.
  const dia = at(mid[0], mid[1]);
  await p.mouse.dblclick(dia.x, dia.y); await settle();
  const entry = p.locator('[data-width-entry] input');
  const entryShown = (await entry.count()) > 0;
  await shot('width-entry');
  const canvasBox = { x: canvas.x, y: canvas.y, width: canvas.width, height: canvas.height };
  // UX review 2026-11-12: the field opens above or below the diamond, inside the canvas; a
  // refused value says so on one short line under the field.
  const diamondBox = { x: dia.x - 9, y: dia.y - 9, width: 18, height: 18 };
  const chipPlace = async (locator) => placeOf(await locator.boundingBox());
  const placeOf = (box) => {
    return {
      inside: inside(box, canvasBox),
      clearOfDiamond: !overlaps(box, diamondBox),
      beside: !!box && (box.y >= dia.y + 8 || box.y + box.height <= dia.y - 8) && box.x <= dia.x + 20 && box.x + box.width >= dia.x - 20,
    };
  };
  const entryPlace = entryShown ? await chipPlace(p.locator('[data-width-entry]')) : null;
  let entryRefusal = null;
  if (entryShown) {
    await entry.fill('-1'); await entry.press('Enter'); await settle();
    const alert = p.locator('[data-width-entry] [role="alert"]');
    const alertBox = await alert.boundingBox();
    const inputBox = await entry.boundingBox();
    entryRefusal = {
      text: (await alert.count()) ? await alert.innerText() : '',
      oneLine: !!alertBox && alertBox.height <= 20,
      under: !!alertBox && !!inputBox && alertBox.y >= inputBox.y + inputBox.height - 1,
    };
    await shot('width-entry-refused');
    await entry.fill('1'); await entry.press('Enter'); await settle(); await settle();
  }
  const typed = await inkIn(p, spot);
  // Criterion 7: the wheel over the diamond widens it and does not zoom (the page label stays).
  const labelBefore = await p.locator('[data-page-label]').first().boundingBox();
  await p.mouse.move(dia.x, dia.y); await settle();
  for (let n = 0; n < 6; n += 1) { await p.mouse.wheel(0, -100); await p.waitForTimeout(30); }
  // UX review 2026-11-12: a live readout beside the diamond while the wheel turns, gone soon
  // after the last notch.
  const readout = p.locator('[data-width-wheel-readout]');
  // Read at once: the readout is meant to go 800 ms after the last notch.
  const seen = await p.evaluate(() => {
    const e = document.querySelector('[data-width-wheel-readout]');
    if (!e) return null;
    const r = e.getBoundingClientRect();
    return { text: e.textContent, box: { x: r.x, y: r.y, width: r.width, height: r.height } };
  });
  await shot('width-wheel-readout');
  const wheelReadout = { shown: !!seen, text: seen?.text ?? '', place: seen ? placeOf(seen.box) : null };
  await p.waitForTimeout(1500);
  wheelReadout.gone = (await readout.count()) === 0;
  await settle();
  const wheeled = await inkIn(p, spot);
  const labelAfter = await p.locator('[data-page-label]').first().boundingBox();
  await shot('width-wheeled');
  await p.keyboard.press('Control+z'); await settle(); await settle();
  const wheelUndone = await inkIn(p, spot);
  // Ctrl+wheel over the diamond also sets the width and never zooms (lead decision F4).
  await p.mouse.move(dia.x, dia.y); await settle();
  await p.keyboard.down('Control');
  for (let n = 0; n < 4; n += 1) { await p.mouse.wheel(0, -100); await p.waitForTimeout(30); }
  await p.keyboard.up('Control');
  await p.waitForTimeout(700); await settle();
  const ctrlWheeled = await inkIn(p, spot);
  const labelCtrl = await p.locator('[data-page-label]').first().boundingBox();
  await p.keyboard.press('Control+z'); await settle(); await settle();
  // UX review 2026-11-12: a point near the canvas's right edge opens its field inside the canvas.
  const [cx, cy] = [xs0[1] + (xs0[2] - xs0[1]) * 0.85, y0 - 40 + 40 * 0.85];
  await click(cx, cy); await settle();
  const edge = at(cx, cy);
  await p.mouse.dblclick(edge.x, edge.y); await settle();
  const edgeEntry = p.locator('[data-width-entry]');
  const edgeBox = (await edgeEntry.count()) ? await edgeEntry.boundingBox() : null;
  const edgePlace = { shown: !!edgeBox, inside: inside(edgeBox, canvasBox) };
  await shot('width-entry-edge');
  if (edgeBox) { await p.keyboard.press('Escape'); await settle(); }
  await p.keyboard.press('Control+z'); await settle();
  const doneBar = toggle.getByRole('button', { name: 'Done', exact: true });
  await doneBar.click(); await settle();
  const sorted = [...frames].sort((a, b) => a - b);
  results.width = {
    thin, wide, entryShown, typed, wheeled, wheelUndone, entryPlace, refusal: entryRefusal, wheelReadout, edgePlace,
    zoomed: !labelBefore || !labelAfter || Math.abs(labelBefore.x - labelAfter.x) + Math.abs(labelBefore.y - labelAfter.y) > 1,
    ctrlWheeled,
    ctrlZoomed: !labelBefore || !labelCtrl || Math.abs(labelBefore.x - labelCtrl.x) + Math.abs(labelBefore.y - labelCtrl.y) > 1,
    doneGone: await toggle.getByRole('button', { name: 'Done', exact: true }).count(),
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
      const { preset, width, laser, penBar, spiroNotice, modeRefusal, marked, selectBar, panelRefusal, toggle, toggleLook, spiroCircling, ...modes } = results;
      // Before 0114 the end froze after 4 of 27 events (desktop); the hold of the first 0114
      // build let it rejoin at 15 of 27; without the hold it is at the pointer at every event.
      assert.ok(spiroCircling.atPointer === spiroCircling.frames, `${tag}: a Spiro end circling its last point follows the pointer (${JSON.stringify(spiroCircling)})`);
      assert.ok(panelRefusal >= 0 && panelRefusal < 80, `${tag}: a panel refusal shows beside the button (${panelRefusal})`);
      // 28 px since the Pen bar took the app's tool-bar look (`0110` plan task 8).
      assert.ok(penBar.minButton >= 28, `${tag}: Pen bar buttons are at least 28 px (${penBar.minButton})`);
      assert.ok(penBar.tipBelow, `${tag}: Pen bar tooltips open below the bar`);
      assert.ok(!penBar.overLabel, `${tag}: the Pen bar clears the page label`);
      assert.ok(!spiroNotice.shown, `${tag}: a Spiro shows no notice`);
      assert.ok(modeRefusal >= 0 && modeRefusal < 80, `${tag}: a refused mode change shows beside the pointer (${modeRefusal})`);
      assert.equal(marked.taper, 'true', `${tag}: the applied profile is marked`);
      assert.equal(marked.panelEdit, 0, `${tag}: Edit width is no longer in the Style panel`);
      assert.ok(toggle.shown && toggle.inBar && toggle.insideWindow, `${tag}: Edit width sits in the tool bar row (${JSON.stringify(toggle)})`);
      assert.equal(selectBar.widthAfter, selectBar.widthBefore, `${tag}: the Width chip leaves the Select bar's width`);
      assert.ok(selectBar.chipHeight > 0 && selectBar.chipHeight <= 40, `${tag}: the Width chip is one line (${selectBar.chipHeight})`);
      assert.equal(selectBar.dones, 1, `${tag}: one Done`);
      assert.ok(width.wide > width.thin + 50, `${tag}: a sideways drag widens the stroke (${width.thin} -> ${width.wide})`);
      assert.ok(width.entryShown, `${tag}: a double-click on a width point opens its typed width`);
      assert.ok(width.typed < width.wide, `${tag}: a typed 1 mm narrows the point (${width.wide} -> ${width.typed})`);
      assert.ok(width.wheeled > width.typed + 20, `${tag}: the wheel widens the point (${width.typed} -> ${width.wheeled})`);
      assert.ok(!width.zoomed, `${tag}: the wheel over a width point does not zoom`);
      assert.equal(width.wheelUndone, width.typed, `${tag}: one undo takes the wheel burst back`);
      assert.ok(width.ctrlWheeled > width.typed + 10, `${tag}: Ctrl+wheel over a width point widens it (${width.typed} -> ${width.ctrlWheeled})`);
      assert.ok(!width.ctrlZoomed, `${tag}: Ctrl+wheel over a width point does not zoom`);
      assert.equal(width.doneGone, 0, `${tag}: Done leaves width editing`);
      assert.ok(toggleLook.font === 14 && toggleLook.border >= 1, `${tag}: Edit width is a 14 px bordered button (${JSON.stringify(toggleLook)})`);
      // On the same row (at 800 × 600 the toggle wraps to a row of its own).
      assert.ok(toggleLook.centreOff >= 0 && (toggleLook.centreOff <= 1 || toggleLook.centreOff >= 30), `${tag}: Edit width sits on the line of Stroke to path (${toggleLook.centreOff})`);
      const placed = (place) => !!place && place.inside && place.clearOfDiamond && place.beside;
      assert.ok(placed(width.entryPlace), `${tag}: the typed width opens above or below its diamond, inside the canvas (${JSON.stringify(width.entryPlace)})`);
      assert.ok(width.refusal && width.refusal.text === 'Must be 0 or more.' && width.refusal.oneLine && width.refusal.under, `${tag}: a refused width says so on one line under the field (${JSON.stringify(width.refusal)})`);
      assert.ok(width.wheelReadout.shown && /^Width [\d.]+ mm$/.test(width.wheelReadout.text) && placed(width.wheelReadout.place), `${tag}: the wheel shows its width beside the diamond (${JSON.stringify(width.wheelReadout)})`);
      assert.ok(width.wheelReadout.gone, `${tag}: the wheel readout goes after the last notch`);
      assert.ok(width.edgePlace.shown && width.edgePlace.inside, `${tag}: a field near the right edge stays inside the canvas (${JSON.stringify(width.edgePlace)})`);
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

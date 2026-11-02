// Browser check of the pattern fill (`specs/0071-pattern-fill`): a rectangle with an elliptical
// hole gets each of the ten generators (one screenshot each), a parameter edit and its undo,
// Lines and Shapes, a margin, the SVG export with the pieces as paths and no `pattern` element,
// and Convert to paths with its undo, and Whole pieces only on Honeycomb (criterion 29, with and
// without, in target/0071-browser/whole-pieces/). Starts its own dev server unless CURVYO_BROWSER_URL is set.
// Screenshots and the exported SVG: target/0071-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0071-browser');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5181;

const GENERATORS = [
  ['hatch', 'Hatch'],
  ['crosshatch', 'Crosshatch'],
  ['grid', 'Grid'],
  ['dots', 'Dots'],
  ['honeycomb', 'Honeycomb'],
  ['brick_wall', 'Brick wall'],
  ['leaf_forest', 'Leaf forest'],
  ['sea_waves', 'Sea waves'],
  ['waves', 'Waves'],
  ['interpolation_waves', 'Interpolation waves'],
];

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
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const canvasEl = p.locator('canvas[data-state]');
    const canvas = await canvasEl.boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
    const drag = async (a, b) => { await move(a[0], a[1]); await p.mouse.down(); await move(b[0], b[1]); await p.mouse.up(); };
    const settle = () => p.waitForTimeout(150);
    const shot = (name) => p.screenshot({ path: path.join(shots, `${name}.png`) });
    const canvasShot = () => p.screenshot({ clip: canvas });
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    const block = p.locator('[data-pattern-block]');
    const field = (name) => p.getByRole('spinbutton', { name });

    // A rectangle with an elliptical hole (Difference).
    await tool('Rectangle');
    await drag([250, 200], [650, 600]); await settle();
    await tool('Ellipse');
    await drag([390, 340], [510, 460]); await settle();
    await tool('Select');
    await move(900, 800); await p.keyboard.press('Control+a'); await settle();
    await p.getByRole('button', { name: /^Difference/ }).first().click(); await settle();
    const plain = await canvasShot();
    fs.writeFileSync(path.join(shots, 'plain.png'), plain);

    // Criterion 1: Pattern is the third fill kind; choosing it shows the block.
    const kinds = p.getByRole('radiogroup', { name: 'Fill paint' });
    await kinds.getByRole('radio', { name: 'Pattern fill' }).click(); await settle();
    await block.waitFor();
    const tiles = p.getByRole('radiogroup', { name: 'Pattern generator' });
    assert.equal(await tiles.getByRole('radio').count(), 10, 'criterion 4: ten generators');
    await shot('00-pattern-chosen');
    assert.ok(!(await canvasShot()).equals(plain), 'the pattern is drawn');

    // Each generator on the shape with a hole, one screenshot each.
    for (const [id, label] of GENERATORS) {
      await tiles.getByRole('radio', { name: label, exact: true }).click(); await settle();
      assert.equal(await tiles.getByRole('radio', { name: label, exact: true }).getAttribute('aria-checked'), 'true');
      await shot(`gen-${id}`);
    }

    // Criterion 2: a parameter edit is one step, undo restores it.
    await tiles.getByRole('radio', { name: 'Hatch', exact: true }).click(); await settle();
    const spacing = field('Pattern spacing');
    const before = await spacing.getAttribute('aria-valuenow');
    await spacing.focus();
    await p.keyboard.down('Shift'); await p.keyboard.press('ArrowUp'); await p.keyboard.up('Shift'); await settle();
    const after = await spacing.getAttribute('aria-valuenow');
    assert.notEqual(after, before, `spacing ${before} -> ${after}`);
    await shot('param-edited');
    await move(900, 800); await p.mouse.down(); await p.mouse.up(); await settle();
    // The click on empty canvas deselected; undo the parameter only, then reselect.
    await p.keyboard.press('Control+a'); await settle();
    await p.keyboard.press('Control+z'); await settle();
    await p.keyboard.press('Control+a'); await settle();
    assert.equal(await field('Pattern spacing').getAttribute('aria-valuenow'), before, 'undo restores the spacing');

    // Criterion 16: Shapes fills closed pieces and shows the colour rows; back to Lines.
    const drawAs = p.getByRole('radiogroup', { name: 'Draw as' });
    await tiles.getByRole('radio', { name: 'Dots', exact: true }).click(); await settle();
    await drawAs.getByRole('radio', { name: 'Shapes' }).click(); await settle();
    await shot('dots-shapes');
    await tiles.getByRole('radio', { name: 'Grid', exact: true }).click(); await settle();
    await shot('grid-shapes');
    await drawAs.getByRole('radio', { name: 'Lines' }).click(); await settle();

    // Criterion 29: Whole pieces only, shown for the generators of shapes only, one undo step.
    const wholeShots = path.join(shots, 'whole-pieces');
    fs.mkdirSync(wholeShots, { recursive: true });
    const whole = p.getByRole('switch', { name: 'Whole pieces only' });
    assert.equal(await whole.count(), 0, 'Grid has no whole pieces');
    await tiles.getByRole('radio', { name: 'Honeycomb', exact: true }).click(); await settle();
    assert.equal(await whole.getAttribute('aria-checked'), 'false', 'off by default');
    const cutCells = await canvasShot();
    fs.writeFileSync(path.join(wholeShots, 'honeycomb-cut.png'), cutCells);
    await whole.click(); await settle();
    assert.equal(await whole.getAttribute('aria-checked'), 'true');
    const wholeCells = await canvasShot();
    fs.writeFileSync(path.join(wholeShots, 'honeycomb-whole.png'), wholeCells);
    assert.ok(!wholeCells.equals(cutCells), 'cut cells are left out');
    await p.screenshot({ path: path.join(wholeShots, 'panel-whole.png') });
    await p.getByRole('tab', { name: /^History/ }).click(); await settle();
    assert.match(await p.locator('aside').first().innerText(), /Pattern: whole pieces/, 'history name');
    await p.getByRole('tab', { name: /^Style/ }).click(); await settle();
    await p.keyboard.press('Control+z'); await settle();
    await p.keyboard.press('Control+a'); await settle();
    assert.equal(await whole.getAttribute('aria-checked'), 'false', 'undo turns it off');
    {
      // Buffer.equals, never deepEqual: a failing deepEqual on two screenshots builds a diff of
      // several MB per byte run and once took the whole machine's memory.
      // The undo notice sits on the canvas; compare once it is gone.
      await p.getByText(/^Undid:/).waitFor({ state: 'hidden', timeout: 15000 }).catch(() => {});
      const restored = await canvasShot();
      if (!restored.equals(cutCells)) fs.writeFileSync(path.join(wholeShots, 'undo-restored.png'), restored);
      assert.ok(restored.equals(cutCells), 'undo restores the cut cells (see undo-restored.png)');
    }
    await tiles.getByRole('radio', { name: 'Grid', exact: true }).click(); await settle();

    // Criterion 14: a margin, typed.
    const margin = field('Pattern margin');
    await margin.dblclick(); await settle();
    await p.keyboard.press('Control+a');
    await p.keyboard.type('4');
    await p.keyboard.press('Enter'); await settle();
    assert.equal(await field('Pattern margin').getAttribute('aria-valuenow'), '4');
    await p.keyboard.press('Control+a'); await settle();
    await shot('grid-margin');

    // Criterion 26: the SVG export holds the pieces as paths and no `pattern` element.
    await p.getByRole('tab', { name: /^File/ }).click(); await settle();
    const [download] = await Promise.all([
      p.waitForEvent('download'),
      p.getByRole('button', { name: /^Export SVG/ }).click(),
    ]);
    const svgPath = path.join(shots, 'export.svg');
    await download.saveAs(svgPath);
    const svg = fs.readFileSync(svgPath, 'utf8');
    assert.ok(!svg.includes('<pattern'), 'no pattern element');
    const paths = (svg.match(/<path/g) ?? []).length;
    assert.ok(paths > 20, `${paths} paths`);
    assert.ok(!svg.includes('grid'), 'no generator name');

    // Criterion 24: Convert to paths, then undo.
    await p.getByRole('tab', { name: /^Style/ }).click().catch(() => {});
    await settle();
    await p.keyboard.press('Control+a'); await settle();
    await p.locator('[data-pattern-convert]').click(); await settle();
    assert.equal(await block.count(), 0, 'the converted group is selected, no pattern block');
    // UI review 2026-11-02 item 2: the new group is selected (panel subject, handles), and the
    // Style tab stays usable after a History visit.
    assert.match(await p.locator('aside').first().innerText(), /Group of \d+ objects/, 'the group is selected');
    await p.getByRole('tab', { name: /^History/ }).click(); await settle();
    await p.getByRole('tab', { name: /^Style/ }).click(); await settle();
    assert.equal(await p.getByRole('tab', { name: /^Style/ }).getAttribute('aria-disabled'), null, 'Style tab enabled');
    await shot('converted');
    await p.keyboard.press('Control+z'); await settle();
    await p.keyboard.press('Control+a'); await settle();
    await block.waitFor();
    await shot('convert-undone');

    // Criterion 12 (customer: like Inkscape's Interpolate): a wavy top over a straight bottom.
    await p.keyboard.press('Escape'); await move(1000, 850); await p.mouse.down(); await p.mouse.up(); await settle();
    await tool('Pen');
    const node = async (x, y) => { await move(x, y); await p.mouse.down(); await p.mouse.up(); await settle(); };
    const smooth = async (x, y, dx) => { await move(x, y); await p.mouse.down(); await move(x + dx, y); await p.mouse.up(); await settle(); };
    await node(700, 820);
    await node(1000, 820);
    await node(1000, 560);
    await smooth(925, 520, -40);
    await smooth(850, 560, -40);
    await smooth(775, 600, -40);
    await node(700, 560);
    await node(700, 820);
    await settle();
    await tool('Select');
    // An unfilled shape is picked on its outline: the straight bottom.
    await move(850, 820); await p.mouse.down(); await p.mouse.up(); await settle();
    await kinds.getByRole('radio', { name: 'Pattern fill' }).click(); await settle();
    await tiles.getByRole('radio', { name: 'Interpolation waves', exact: true }).click(); await settle();
    await shot('gen-interpolation_waves-wavy');

    assert.deepEqual(errors, []);
    console.log(`pattern-fill: ok (${paths} paths in the export)`);
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((error) => { console.error(error); process.exit(1); });

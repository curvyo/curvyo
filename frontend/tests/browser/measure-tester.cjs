// Independent tester journey for `specs/0070-measure-and-dimension` (written from the spec and the
// UX notes, not from measure.cjs's gestures): rail name and tooltips, no bar while measuring, the
// readout format, the mode label with and without Shift, Shift read live, the pinned measurement
// ending on Undo, an angle left with Shift at the second segment pick, selection first with Enter,
// the Layers row and the live region. Every check is recorded; the script fails if any check
// fails and prints them all. Screenshots: target/0070-browser/tester-*.png.
const { chromium } = require('playwright');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0070-browser');
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
  const results = [];
  const check = (name, ok, detail = '') => results.push({ name, ok: Boolean(ok), detail: String(detail) });
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
    const settle = () => p.waitForTimeout(150);
    const shot = (name) => p.screenshot({ path: path.join(shots, `tester-${name}.png`) });
    const labels = () => p.locator('[data-measure-dimension]');
    const readout = p.locator('[data-measure-chip="readout"]');
    const modeChip = p.locator('[data-measure-chip="mode"]');
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    const toolbars = async () => p.getByRole('toolbar').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));

    // Geometry: a V path with the Pen (two segments at an angle) and a circle.
    await tool('Pen');
    await click(400, 500); await click(600, 500); await click(600, 300);
    await p.keyboard.press('Enter'); await settle();
    await tool('Ellipse');
    await move(800, 400); await p.keyboard.down('Control'); await p.mouse.down(); await move(900, 500); await p.mouse.up(); await p.keyboard.up('Control'); await settle();
    await tool('Select');
    await p.keyboard.press('Escape'); await settle();

    // Criterion 2: names and tooltips.
    const rail = p.getByRole('button', { name: /^Measure tool/ });
    check('c2 rail accessible name without selection', (await rail.getAttribute('aria-label') ?? await rail.innerText()).includes('Measure tool (M)'), await rail.getAttribute('aria-label'));
    check('c2 card named "More tools"', await p.getByRole('toolbar', { name: 'More tools' }).count() === 1, (await toolbars()).join(' | '));
    await click(850, 400); await settle(); // select the circle
    const nameWithSel = await rail.getAttribute('aria-label');
    check('c2 rail name with a Select selection reads "Measure tool (Esc, M)"', /Measure tool \(Esc, M\)/.test(nameWithSel ?? ''), nameWithSel);
    await p.keyboard.press('m'); await settle();
    check('c2 M with a selection keeps the move entry (tool stays Select)', (await rail.getAttribute('aria-pressed')) !== 'true');
    await p.keyboard.press('Escape'); await settle();
    const barsSelect = await toolbars();

    // Criterion 5: selection first + Enter (the circle is still selected), via the rail button.
    await rail.click(); await settle();
    const barsMeasure = await toolbars();
    check('c2 no top bar while Measure is active', barsMeasure.length <= barsSelect.length, `select: ${barsSelect.join(' | ')}; measure: ${barsMeasure.join(' | ')}`);
    await move(1000, 300); await settle();
    const previewCount = await labels().count();
    check('c5 preview follows the pointer with an eligible selection', previewCount === 1, previewCount);
    await p.keyboard.press('Enter'); await settle();
    const afterEnter = await labels().allInnerTexts();
    check('c5 Enter places a radius at the default position', afterEnter.some((t) => t.startsWith('R ')), afterEnter.join(' | '));
    await shot('01-enter');

    // Criterion 1 / 2: readout format and mode label.
    const ensureMeasure = async () => { if ((await rail.getAttribute('aria-pressed')) !== 'true') { await rail.click(); await settle(); } };
    // Clear the selection so the next clicks pick instead of placing selection-first.
    await tool('Select'); await click(1300, 900); await p.keyboard.press('Escape'); await settle();
    await ensureMeasure();
    await move(200, 200); await settle();
    check('c2 mode label "Measure" before a pick', (await modeChip.innerText().catch(() => '')).trim() === 'Measure', await modeChip.innerText().catch(() => 'none'));
    await p.keyboard.down('Shift'); await move(201, 201); await settle();
    check('c2 mode label "Dimension" with Shift before a pick', (await modeChip.innerText().catch(() => '')).trim() === 'Dimension', await modeChip.innerText().catch(() => 'none'));
    await p.keyboard.up('Shift'); await settle();
    await click(200, 200); await move(320, 360); await settle();
    const live = (await readout.innerText().catch(() => '')).split('\n').map((s) => s.trim()).filter(Boolean);
    check('c1 readout has two lines: length, angle / Δ x, y', live.length === 2 && /^[\d.]+ mm, −?[\d.]+°$/.test(live[0]) && /^Δ −?[\d.]+, −?[\d.]+ mm$/.test(live[1]), live.join(' / '));
    // Shift read live: swap to the green preview and back.
    await p.keyboard.down('Shift'); await settle();
    const shiftReadout = await readout.count();
    const shiftLabels = await labels().count();
    const shiftMode = (await modeChip.innerText().catch(() => '')).trim();
    await shot('02-shift-live');
    await p.keyboard.up('Shift'); await settle();
    check('c4 Shift swaps readout for the dimension preview', shiftReadout === 0 && shiftLabels === 2 && shiftMode.startsWith('Dimension, '), `readout ${shiftReadout}, labels ${shiftLabels}, mode ${shiftMode}`);
    check('c4 releasing Shift brings the readout back', await readout.count() === 1 && await labels().count() === 1);
    await click(320, 360); await settle();
    const speech = await p.locator('[data-measure-speech]').innerText().catch(() => '');
    check('UX §3 live region speaks the pinned value', /millimetres, .*degrees/.test(speech), speech);
    // Criterion 1: Ctrl+Z ends the pinned measurement (it also undoes the Enter dimension).
    const beforeUndo = await readout.count();
    await p.keyboard.press('Control+z'); await settle();
    check('c1 Undo ends the pinned measurement', beforeUndo === 1 && await readout.count() === 0, `before ${beforeUndo}, after ${await readout.count()}`);
    await shot('03-after-undo');
    await p.keyboard.press('Control+Shift+z'); await settle();
    await tool('Select'); await click(1300, 900); await p.keyboard.press('Escape'); await settle();
    await ensureMeasure();

    // Criterion 3/4: angle from two segment picks, Shift held at the SECOND pick.
    const n0 = await labels().count();
    await click(460, 500); await settle();
    await move(600, 360); await p.keyboard.down('Shift'); await settle();
    await click(600, 360); await p.keyboard.up('Shift');
    await move(560, 460); await settle();
    const angleMode = (await modeChip.innerText().catch(() => '')).trim();
    await click(560, 460); await settle();
    const texts = await labels().allInnerTexts();
    check('c4 angle with Shift at the second segment pick leaves a dimension', texts.length === n0 + 1 && texts.some((t) => t.endsWith('°')), `mode while placing: ${angleMode}; labels: ${texts.join(' | ')}`);
    await shot('04-angle');

    // Criterion 17: Layers row.
    await p.keyboard.press('Escape'); await p.keyboard.press('Escape'); await settle();
    const layersTab = p.getByRole('tab', { name: /Layers/ });
    if (await layersTab.count()) { await layersTab.first().click(); await settle(); }
    const rows = await p.getByRole('row').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label') || e.textContent)).catch(() => []);
    check('c17 Layers has a "Dimension R …" row', rows.some((r) => /Dimension R [\d.]+ mm/.test(r)), rows.join(' | '));
    await shot('05-layers');
    check('no page errors', errors.length === 0, errors.join(' | '));
  } finally {
    await browser.close();
    if (vite?.pid) { try { process.kill(-vite.pid, 'SIGKILL'); } catch { /* gone */ } }
  }
  for (const r of results) console.log(`${r.ok ? 'PASS' : 'FAIL'} ${r.name}${r.detail ? ` [${r.detail}]` : ''}`);
  const failed = results.filter((r) => !r.ok).length;
  console.log(`${results.length - failed}/${results.length} checks passed`);
  process.exit(failed ? 1 : 0);
})().catch((e) => { console.error(e); process.exit(1); });

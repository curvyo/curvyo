// Browser check of `specs/0123-raster-engraving` (Image operation kind): a picture dropped on a
// laser page and assigned Image with the Select bar's Image button shows the Image settings group
// in its unfolded row (Dither as an always-visible two-row strip, Brightness, Contrast, Gamma,
// Invert, the mode's own field last); the strip by mouse and keyboard (no wrap, one undo step per
// selection, focus stays); every setting is changed and undone; Grey on an M3 machine is a check
// that focuses the strip; a linked clone of the image burns its own dots and the file holds M4
// runs; an image in Engrave hatches its frame rectangle; the Image test grid bar (kind Image,
// six-segment Dither, no Fill); the Materials tab holds four sets. Starts its own dev server
// unless CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium; build the wasm
// bindings first. Screenshots: target/raster-engraving-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/raster-engraving-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5190;

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
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(350);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const drag = async (x1, y1, x2, y2) => { await p.mouse.move(x1, y1); await p.mouse.down(); await p.mouse.move(x2, y2, { steps: 5 }); await p.mouse.up(); await settle(); };
    const body = () => p.locator('[data-row-body]');
    const group = () => p.locator('[data-image-settings]');
    const field = (name) => p.getByLabel(name, { exact: true });
    const commit = async (name, text) => { await field(name).fill(text); await p.keyboard.press('Enter'); await settle(); };
    const undo = async () => { await p.mouse.click(700, 950); await p.keyboard.press('Control+z'); await settle(); };
    const pressed = (name) => p.getByRole('button', { name, exact: true }).getAttribute('aria-pressed');

    // A laser page, zoomed out.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);
    for (let i = 0; i < 4; i += 1) { await p.keyboard.down('Control'); await p.mouse.wheel(0, 200); await p.keyboard.up('Control'); await settle(); }

    // A picture dropped on the canvas: a 120 x 80 grey ramp PNG.
    const canvas = await p.locator('canvas[data-state]').boundingBox();
    const data = await p.evaluateHandle(async () => {
      const c = document.createElement('canvas');
      c.width = 120; c.height = 80;
      const g = c.getContext('2d');
      const ramp = g.createLinearGradient(0, 0, 120, 0);
      ramp.addColorStop(0, '#000'); ramp.addColorStop(1, '#fff');
      g.fillStyle = ramp; g.fillRect(0, 0, 120, 80);
      const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
      const t = new DataTransfer();
      t.items.add(new File([blob], 'ramp.png', { type: 'image/png' }));
      return t;
    });
    const cx = canvas.x + canvas.width / 2; const cy = canvas.y + canvas.height / 2;
    await p.dispatchEvent('canvas[data-state]', 'dragenter', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'dragover', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'drop', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.waitForTimeout(1500);
    await shot('0-dropped');

    // The Select bar's Image button (last, symbol only) assigns the picture to a new Image 1.
    const kinds = p.locator('[role=group][aria-label=Operation] button');
    assert.deepEqual(await kinds.evaluateAll((els) => els.map((e) => e.getAttribute('aria-label'))), ['Cut', 'Score', 'Engrave', 'Engrave + Outline', 'Cut-out', 'Image'], 'six kind buttons, Image last');
    assert.equal(await kinds.last().locator('svg').count(), 1, 'symbol only: the dot-ramp glyph');
    await shot('0b-select-bar');
    await kinds.last().click(); await settle();
    await tab('Operations');
    if ((await p.locator('[data-row-body]').count()) === 0) { await p.getByRole('button', { name: /^Image 1/ }).first().click(); await settle(); }
    await commit('Power, percent', '40');
    await commit('Speed, millimetres per minute', '3000');
    await group().waitFor();
    await shot('1-image-settings');
    assert.equal(await group().getAttribute('aria-label'), 'Image settings of Image 1');
    assert.match(await p.locator('li[data-row-key]').first().innerText(), /Image 1[\s\S]*Image · 1 item/);
    assert.match(await p.locator('li[data-row-key]').first().innerText(), /· Floyd/, 'numbers line ends with the dither');
    assert.equal(await group().locator('h3, h4, [role=heading]').count(), 0, 'no group heading (P19)');
    for (const name of ['Brightness, percent', 'Contrast, percent', 'Gamma']) assert.equal(await field(name).count(), 1, name);
    assert.equal(await field('Threshold, percent').count(), 0, 'no Threshold with Floyd-Steinberg');
    assert.equal(await field('Lowest power, percent').count(), 0);
    assert.equal(await p.getByRole('button', { name: 'Crosshatch', exact: true }).count(), 0, 'no Crosshatch on Image');
    assert.equal(await p.getByRole('button', { name: 'Bidirectional fill', exact: true }).count(), 1);
    assert.equal(await p.getByRole('button', { name: 'Invert', exact: true }).getAttribute('aria-pressed'), 'false');
    assert.equal(await field('Gamma').inputValue(), '1');

    // The Dither strip: always visible, two fixed rows, short name Floyd with the full name as label.
    const strip = p.getByRole('radiogroup', { name: 'Dither' });
    const cells = strip.getByRole('radio');
    assert.deepEqual(await cells.evaluateAll((els) => els.map((e) => [e.getAttribute('aria-label'), e.textContent.trim()])), [['Threshold', 'Threshold'], ['Ordered', 'Ordered'], ['Floyd-Steinberg', 'Floyd'], ['Jarvis', 'Jarvis'], ['Atkinson', 'Atkinson'], ['Stucki', 'Stucki'], ['Grey', 'Grey']]);
    const rowsOf = async () => cells.evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    const tops0 = await rowsOf();
    assert.ok(tops0[0] === tops0[1] && tops0[1] === tops0[2] && tops0[3] === tops0[6] && tops0[2] < tops0[3], `rows 3 + 4: ${tops0}`);
    const checkedName = () => strip.locator('[role=radio][aria-checked=true]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.deepEqual(await checkedName(), ['Floyd-Steinberg']);
    await p.getByRole('radio', { name: 'Floyd-Steinberg', exact: true }).hover();
    await p.waitForTimeout(700);
    assert.match(await p.locator('[role=tooltip]').first().innerText(), /Floyd-Steinberg/);
    await shot('1b-strip-tooltip');
    // Mouse: Threshold. Its field is the LAST row; the strip does not move.
    const stripTop = () => strip.evaluate((e) => Math.round(e.getBoundingClientRect().top));
    const top0 = await stripTop();
    await p.getByRole('radio', { name: 'Threshold', exact: true }).click(); await settle();
    assert.deepEqual(await checkedName(), ['Threshold']);
    assert.equal(await stripTop(), top0, 'the strip does not move');
    assert.equal(await field('Threshold, percent').inputValue(), '50');
    assert.equal(await field('Lowest power, percent').count(), 0);
    const tops = await p.evaluate(() => ['[data-image-dither]', '[data-image-field=brightness]', '[data-image-field=gamma]', '[data-image-field=threshold]'].map((q) => Math.round(document.querySelector(q).getBoundingClientRect().top)));
    assert.ok(tops[0] < tops[1] && tops[1] < tops[2] && tops[2] < tops[3], `Threshold is the last row ${tops}`);
    await commit('Threshold, percent', '35');
    assert.equal(await field('Threshold, percent').inputValue(), '35');
    await field('Threshold, percent').fill('150'); await p.keyboard.press('Enter'); await settle();
    assert.match(await p.locator('[data-image-field=threshold]').innerText(), /Threshold is 0 to 100 %\./);
    assert.equal(await field('Threshold, percent').getAttribute('aria-invalid'), 'true');
    await p.keyboard.press('Escape'); await settle();
    assert.equal(await field('Threshold, percent').inputValue(), '35', 'Escape reverts');
    await shot('2-threshold');

    // Keyboard: the arrows move and select (no wrap), the focus stays on the strip, each one undo step.
    // Radix selects on the focus an arrow key causes, while the key is still down: hold it 80 ms.
    const active = () => p.evaluate(() => document.activeElement?.getAttribute('aria-label'));
    await p.getByRole('radio', { name: 'Threshold', exact: true }).focus();
    await p.keyboard.press('ArrowLeft', { delay: 80 }); await settle();
    assert.deepEqual(await checkedName(), ['Threshold'], 'no wrap at the start');
    await p.keyboard.press('ArrowRight', { delay: 80 }); await settle();
    assert.deepEqual(await checkedName(), ['Ordered']);
    assert.equal(await active(), 'Ordered', 'the focus stays on the strip');
    assert.equal(await field('Threshold, percent').count(), 0, 'Ordered has no mode field');
    await p.keyboard.press('ArrowDown', { delay: 80 }); await settle();
    assert.deepEqual(await checkedName(), ['Floyd-Steinberg']);
    await p.keyboard.press('End', { delay: 80 }); await settle();
    assert.deepEqual(await checkedName(), ['Grey']);
    await p.keyboard.press('ArrowRight', { delay: 80 }); await settle();
    assert.deepEqual(await checkedName(), ['Grey'], 'no wrap at the end');
    assert.equal(await field('Lowest power, percent').inputValue(), '0');
    assert.equal(await stripTop(), top0, 'the strip never moves');
    assert.match(await p.locator('li[data-row-key]').first().innerText(), /· Grey/);
    // One undo step per selection: Grey (End), Floyd, Ordered, back to Threshold.
    const undoStep = async () => { await p.locator('canvas[data-state]').focus(); await p.keyboard.press('Control+z'); await settle(); };
    await undoStep(); assert.deepEqual(await checkedName(), ['Floyd-Steinberg'], 'undo Grey');
    await undoStep(); assert.deepEqual(await checkedName(), ['Ordered'], 'undo Floyd');
    await undoStep(); assert.deepEqual(await checkedName(), ['Threshold'], 'undo Ordered');
    await p.keyboard.press('Control+Shift+z'); await settle();
    assert.deepEqual(await checkedName(), ['Ordered'], 'redo');
    await p.getByRole('radio', { name: 'Grey', exact: true }).click(); await settle();
    assert.deepEqual(await checkedName(), ['Grey']);
    await commit('Lowest power, percent', '60');
    assert.match(await p.locator('[role=list][aria-label=Checks]').innerText(), /Image 1: Lowest power is above Power\./);
    await p.locator('[role=list][aria-label=Checks] button', { hasText: 'Image 1' }).click(); await settle();
    assert.equal(await p.evaluate(() => document.activeElement?.getAttribute('aria-label')), 'Lowest power, percent', 'focus on Lowest power');
    await commit('Lowest power, percent', '10');
    assert.doesNotMatch(await p.locator('[role=list][aria-label=Checks]').innerText(), /Lowest power is above/);

    // Grey on an M3 machine: a block that focuses the strip's pressed cell; back to M4 clears it.
    await tab('Document');
    await p.getByRole('switch', { name: 'Laser mode' }).click(); await settle();
    await tab('Operations');
    assert.match(await p.locator('[role=list][aria-label=Checks]').innerText(), /Image 1 needs laser mode M4 for Grey\./);
    await shot('3-grey-on-m3');
    await p.locator('[role=list][aria-label=Checks] button', { hasText: 'Image 1' }).first().click(); await settle();
    assert.equal(await p.evaluate(() => document.activeElement?.closest('[data-image-dither]') !== null), true, 'focus on the Dither strip');
    await tab('Document');
    await p.getByRole('switch', { name: 'Laser mode' }).click(); await settle();
    await tab('Operations');
    assert.doesNotMatch(await p.locator('[role=list][aria-label=Checks]').innerText(), /needs laser mode M4/);

    // Every number and Invert: set, then undone one by one.
    await commit('Brightness, percent', '-10');
    await commit('Contrast, percent', '20');
    await commit('Gamma', '1.5');
    await p.getByRole('button', { name: 'Invert', exact: true }).click(); await settle();
    assert.equal(await p.getByRole('button', { name: 'Invert', exact: true }).getAttribute('aria-pressed'), 'true');
    await shot('4-all-set');
    await undoStep();
    assert.equal(await p.getByRole('button', { name: 'Invert', exact: true }).getAttribute('aria-pressed'), 'false', 'undo Invert');
    await undoStep();
    assert.equal(await field('Gamma').inputValue(), '1', 'undo Gamma');
    await undoStep();
    assert.equal(await field('Contrast, percent').inputValue(), '0', 'undo Contrast');
    await undoStep();
    assert.equal(await field('Brightness, percent').inputValue(), '0', 'undo Brightness');
    await commit('Brightness, percent', '-10'); await commit('Contrast, percent', '20'); await commit('Gamma', '1.5');

    // A linked clone of the image, assigned to Image 1, shows its own dots; the file holds M4 runs.
    await tab('Document');
    await p.mouse.click(cx, cy); await settle();
    const clone = p.getByRole('button', { name: 'Linked clone', exact: true });
    await clone.first().click(); await settle();
    await p.keyboard.press('ArrowRight'); await p.keyboard.press('ArrowRight'); await settle();
    await kinds.last().click(); await settle();
    await tab('Operations');
    assert.match(await p.locator('li[data-row-key]').first().innerText(), /Image · 2 items/);
    await shot('5-linked-clone');
    const dots = await p.evaluate(() => { const c = document.querySelector('canvas[data-state]'); return c ? c.width : 0; });
    assert.ok(dots > 0);
    await p.getByRole('button', { name: 'Toolpath view', exact: true }).click(); await settle();
    await p.waitForTimeout(800);
    assert.equal(await p.locator('[data-play-bar]').count(), 1);
    await shot('6-toolpath');
    await p.getByRole('button', { name: 'Operations view', exact: true }).click(); await settle();
    const download = p.waitForEvent('download');
    await p.locator('[data-export-job]').click();
    const file = await download;
    const target = path.join(out, file.suggestedFilename());
    await file.saveAs(target);
    const gcode = fs.readFileSync(target, 'utf8').split('\n');
    assert.ok(gcode.some((l) => /^M4 S0/.test(l)), 'M4 header');
    const runs = gcode.filter((l) => /^G1 .*S\d+/.test(l) && !/S0\b/.test(l));
    assert.ok(runs.length > 20, `burned runs with S words: ${runs.length}`);
    assert.ok(runs.some((l) => /S(?!400\b)\d+/.test(l)), 'Grey runs vary in S');

    // The same image in Engrave hatches its frame rectangle (no Image settings group there).
    await tab('Document');
    await p.mouse.click(cx, cy); await settle();
    await p.getByRole('button', { name: 'Engrave', exact: true }).last().click(); await settle();
    await tab('Operations');
    await p.getByRole('button', { name: /^Engrave 1/ }).first().click(); await settle();
    assert.equal(await group().count(), 0, 'Engrave never shows the Image settings group');
    assert.equal(await p.getByRole('button', { name: 'Crosshatch', exact: true }).count(), 1, 'Engrave keeps Crosshatch');
    assert.match(await p.locator('[role=list][aria-label=Checks]').innerText().catch(() => ''), /^((?!not a picture)[\s\S])*$/, 'an image in Engrave never warns');
    await shot('7-image-in-engrave');

    // The test grid bar: four kinds; Image with the six-segment Dither strip, no Fill, no menu.
    await tab('Document');
    await p.getByRole('button', { name: /^Test grid tool/ }).click(); await settle();
    await p.locator('canvas[data-state]').focus();
    await p.keyboard.press('Enter'); await settle();
    const bar = p.getByRole('toolbar', { name: /^Test grid/ });
    await bar.waitFor();
    assert.deepEqual(await bar.getByRole('radiogroup', { name: 'Kind' }).getByRole('radio').evaluateAll((els) => els.map((e) => e.textContent.trim())), ['Cut', 'Score', 'Engrave', 'Image']);
    await bar.getByRole('radio', { name: 'Engrave' }).click(); await settle();
    assert.equal(await bar.getByRole('radiogroup', { name: 'Fill' }).count() + await bar.getByRole('radiogroup', { name: 'Dither' }).count(), 0, 'the Engrave grid has neither Fill nor Dither');
    await bar.getByRole('radio', { name: 'Image' }).click(); await settle();
    const gridDither = bar.getByRole('radiogroup', { name: 'Dither' });
    assert.deepEqual(await gridDither.getByRole('radio').evaluateAll((els) => els.map((e) => [e.getAttribute('aria-label') ?? e.textContent.trim(), e.getAttribute('aria-checked')])), [['Threshold', 'false'], ['Ordered', 'false'], ['Floyd-Steinberg', 'true'], ['Jarvis', 'false'], ['Atkinson', 'false'], ['Stucki', 'false']]);
    assert.equal(await bar.getByRole('radiogroup', { name: 'Fill' }).count(), 0, 'no Fill');
    assert.equal(await bar.getByRole('button', { name: /^Dither/ }).count(), 0, 'no menu');
    assert.equal(await bar.getByRole('button', { name: 'Crosshatch' }).count(), 0, 'no Crosshatch on Image');
    await gridDither.getByRole('radio', { name: 'Ordered' }).click(); await settle();
    assert.equal(await gridDither.getByRole('radio', { name: 'Ordered' }).getAttribute('aria-checked'), 'true');
    await p.keyboard.press('ArrowRight'); await settle();
    assert.equal(await gridDither.locator('[aria-checked=true]').getAttribute('aria-label'), 'Floyd-Steinberg', 'arrows pick');
    await shot('8-grid-image');
    await gridDither.locator('[aria-checked=true]').focus();
    await p.keyboard.press('End'); await p.keyboard.press('ArrowRight'); await settle();

    // Materials: four sets per laser, Image last with its glyph and Interval.
    await tab('Materials');
    await p.getByRole('button', { name: 'New material', exact: true }).click(); await settle();
    await p.getByLabel('Name', { exact: true }).fill('Birch 3 mm');
    await p.getByLabel('Thickness, millimetres', { exact: true }).fill('3');
    await p.keyboard.press('Enter'); await settle();
    const birch = p.getByRole('button', { name: /^Birch 3 mm/ }).first();
    if ((await birch.getAttribute('aria-expanded')) !== 'true') { await birch.click(); await settle(); }
    await shot('9a-material');
    for (const word of ['Cut', 'Score', 'Engrave', 'Image']) {
      await p.getByRole('button', { name: `${word} set`, exact: true }).click(); await settle();
    }
    const setHeads = await p.locator('[role=group][aria-label$=" sets"] [aria-label*=" set of "]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.ok(setHeads.some((n) => /^Interval, Image set of /.test(n)), `Image set has Interval: ${setHeads}`);
    const powers = await p.locator('input[aria-label^="Power, "]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label').split(',')[1].trim().split(' ')[0]));
    assert.deepEqual(powers, ['Cut', 'Score', 'Engrave', 'Image'], 'four sets, Image last');
    await shot('9-materials-four-sets');

    assert.deepEqual(errors, [], `page errors: ${errors.join('\n')}`);
    console.log('raster engraving browser check: ok');
    process.exitCode = 0;
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((error) => { console.error(error); process.exit(1); });

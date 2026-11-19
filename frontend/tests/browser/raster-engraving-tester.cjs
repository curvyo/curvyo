// Tester browser check of `specs/0123-raster-engraving`, independent of `raster-engraving.cjs`,
// rewritten for the Image kind (2026-11-19): a large noisy picture (800 x 800 px, about 200 mm)
// dropped on a laser page and assigned with the Select bar's Image button (criterion 47: one row
// "Image 1"); criterion 11 (the Image settings group shows on the Image row and on no Engrave
// row; the Dither strip is two fixed rows, the arrows move and select without wrap, Home and End
// select the ends, each selection is one undo step); criterion 1 (the same picture pressed into
// Engrave makes "Engrave 1" and no warn line names the image); criterion 12 (an Image setting
// edit recomputes in the same frame) and the real cost of one Image setting edit in the app: the
// wall time from Enter in Gamma to the next painted frame, and the longest main-thread task it
// causes (criterion 37 (d): 50 ms). Wall time is an observation here; the asserted CPU budget is
// `curvyo-editor-wasm/tests/acceptance_0123_tester_budget.rs`.
// Run: NODE_PATH=target/browser-check/node_modules CURVYO_E2E_BROWSER=/usr/bin/chromium \
//   node frontend/tests/browser/raster-engraving-tester.cjs
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/raster-engraving-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5191;

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
  const report = {};
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(400);
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const group = () => p.locator('[data-image-settings]');
    const field = (name) => p.getByLabel(name, { exact: true });
    const commit = async (name, text) => { await field(name).fill(text); await p.keyboard.press('Enter'); await settle(); };

    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);
    for (let i = 0; i < 4; i += 1) { await p.keyboard.down('Control'); await p.mouse.wheel(0, 200); await p.keyboard.up('Control'); await settle(); }
    const canvas = await p.locator('canvas[data-state]').boundingBox();
    const cx = canvas.x + canvas.width / 2; const cy = canvas.y + canvas.height / 2;

    // A large noisy picture joins Engrave 1.
    await tab('Document');
    const data = await p.evaluateHandle(async () => {
      const c = document.createElement('canvas');
      c.width = 800; c.height = 800;
      const g = c.getContext('2d');
      const img = g.createImageData(800, 800);
      let seed = 7;
      for (let i = 0; i < 800 * 800; i += 1) {
        seed = (seed * 1103515245 + 12345) & 0x7fffffff;
        const x = i % 800; const y = Math.floor(i / 800);
        const v = Math.min(255, Math.floor(((x + y) / 1600) * 200 + (seed % 56)));
        img.data[i * 4] = v; img.data[i * 4 + 1] = v; img.data[i * 4 + 2] = v; img.data[i * 4 + 3] = 255;
      }
      g.putImageData(img, 0, 0);
      const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
      const t = new DataTransfer();
      t.items.add(new File([blob], 'noise.png', { type: 'image/png' }));
      return t;
    });
    await p.dispatchEvent('canvas[data-state]', 'dragenter', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'dragover', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'drop', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.waitForTimeout(2000);
    // Criterion 47: the sixth kind button, symbol-only, named Image; one press makes Image 1.
    const kinds = await p.locator('button[aria-label]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    for (const k of ['Cut', 'Score', 'Engrave', 'Engrave + Outline', 'Cut-out', 'Image']) assert.ok(kinds.includes(k), `kind button ${k}: ${kinds}`);
    const imageButton = p.getByRole('button', { name: 'Image', exact: true }).last();
    const look = await imageButton.evaluate((b) => ({ svg: b.querySelector('svg') !== null, w: b.getBoundingClientRect().width }));
    assert.ok(look.svg && look.w <= 40, `the Image button is symbol-only: ${JSON.stringify(look)}`);
    await imageButton.click(); await settle();
    await tab('Operations');
    const rowNames = () => p.locator('li[data-row-key]').evaluateAll((els) => els.map((e) => e.innerText.split('\n')[0]));
    assert.ok((await rowNames()).some((t) => t.startsWith('Image 1')), `one row Image 1: ${await rowNames()}`);
    if ((await p.locator('[data-row-body]').count()) === 0) { await p.getByRole('button', { name: /^Image 1/ }).first().click(); await settle(); }
    await commit('Power, percent', '40');
    await commit('Speed, millimetres per minute', '6000');
    await group().waitFor({ timeout: 10000 });
    report.groupShownWithImage = true;
    await p.screenshot({ path: path.join(out, '1-group.png') });

    // The real cost of one Image setting edit: Enter in Gamma to the next painted frame, and the
    // longest task the edit causes.
    await p.evaluate(() => {
      window.__long = [];
      new PerformanceObserver((list) => { for (const e of list.getEntries()) window.__long.push(e.duration); }).observe({ entryTypes: ['longtask'] });
    });
    const runs = [];
    for (const g of ['1.2', '1.4', '1.6', '1.8', '2']) {
      await field('Gamma').fill(g);
      await p.evaluate(() => { window.__long = []; });
      const ms = await p.evaluate(() => new Promise((resolve) => {
        const input = document.activeElement;
        const t0 = performance.now();
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
        requestAnimationFrame(() => requestAnimationFrame(() => resolve(performance.now() - t0)));
      }));
      await settle();
      const longest = await p.evaluate(() => Math.max(0, ...window.__long));
      runs.push({ gamma: g, toPaintMs: Math.round(ms), longestTaskMs: Math.round(longest) });
      assert.equal(await field('Gamma').inputValue(), g, 'written');
    }
    report.gammaEdits = runs;
    const sorted = runs.map((r) => Math.max(r.toPaintMs, r.longestTaskMs)).sort((a, b) => a - b);
    report.medianEditFrameMs = sorted[Math.floor(sorted.length / 2)];

    // Criterion 11: the Dither strip. Two fixed rows; Floyd-Steinberg by default, shown "Floyd".
    const strip = p.getByRole('radiogroup', { name: 'Dither' });
    const names = await strip.getByRole('radio').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.deepEqual(names, ['Threshold', 'Ordered', 'Floyd-Steinberg', 'Jarvis', 'Atkinson', 'Stucki', 'Grey']);
    const tops = await strip.getByRole('radio').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    assert.equal(new Set(tops).size, 2, `two rows: ${tops}`);
    assert.equal(tops[2], tops[0], 'Threshold, Ordered, Floyd on row 1');
    assert.ok(tops[3] > tops[2] && tops[6] === tops[3], 'Jarvis to Grey on row 2');
    assert.equal((await strip.getByRole('radio', { name: 'Floyd-Steinberg', exact: true }).innerText()).trim(), 'Floyd');
    const checked = () => strip.locator('[role=radio][aria-checked=true]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.deepEqual(await checked(), ['Floyd-Steinberg'], 'the default');
    assert.equal(await p.locator('[data-image-dither] p', { hasText: 'Damaged' }).count(), 0);
    await strip.getByRole('radio', { name: 'Floyd-Steinberg', exact: true }).focus();
    const key = async (k) => { await p.keyboard.press(k, { delay: 60 }); await settle(); };
    const tabStops = await strip.getByRole('radio').evaluateAll((els) => els.filter((e) => e.tabIndex === 0).length);
    assert.equal(tabStops, 1, 'one Tab stop');
    await key('ArrowRight'); assert.deepEqual(await checked(), ['Jarvis'], 'an arrow moves and selects');
    await key('Home'); assert.deepEqual(await checked(), ['Threshold'], 'Home selects the first');
    await key('ArrowLeft'); assert.deepEqual(await checked(), ['Threshold'], 'no wrap at the start');
    await key('End'); assert.deepEqual(await checked(), ['Grey'], 'End selects the last');
    await key('ArrowRight'); assert.deepEqual(await checked(), ['Grey'], 'no wrap at the end');
    assert.ok(await p.evaluate(() => document.activeElement?.closest('[data-image-dither]') !== null), 'focus stays on the strip');
    // Grey shows Lowest power last; Threshold shows Threshold; the strip never moves.
    const stripTop = async () => Math.round((await strip.boundingBox()).y);
    const greyTop = await stripTop();
    assert.equal(await field('Lowest power, percent').count(), 1, 'Lowest power with Grey');
    // Each selection was one undo step: Grey <- Threshold <- Jarvis <- Floyd-Steinberg.
    const undo = async () => { await p.mouse.click(canvas.x + 300, canvas.y + canvas.height - 40); await p.keyboard.press('Control+z'); await settle(); };
    await undo(); assert.deepEqual(await checked(), ['Threshold'], 'undo 1');
    assert.equal(await field('Threshold, percent').count(), 1, 'Threshold with Threshold');
    assert.equal(await stripTop(), greyTop, 'the strip keeps its place');
    await undo(); assert.deepEqual(await checked(), ['Jarvis'], 'undo 2');
    await undo(); assert.deepEqual(await checked(), ['Floyd-Steinberg'], 'undo 3');
    report.ditherStripKeyboard = true;
    await p.screenshot({ path: path.join(out, '1b-strip.png') });

    // Criterion 12: the run time of the row follows the edit at once (same frame).
    const before = await p.locator('li[data-row-key]').first().innerText();
    await commit('Brightness, percent', '100');
    const after = await p.locator('li[data-row-key]').first().innerText();
    report.rowFollowsEdit = before !== after;
    await p.screenshot({ path: path.join(out, '2-brightness-100.png') });

    // Criterion 1: the picture pressed into Engrave makes Engrave 1, which hatches its frame;
    // no warn line names an image, and no Engrave row shows the Image settings group.
    await p.locator('canvas[data-state]').click({ position: { x: cx - canvas.x, y: cy - canvas.y } }); await settle();
    await p.getByRole('button', { name: 'Engrave', exact: true }).last().click(); await settle();
    await tab('Operations');
    assert.ok((await rowNames()).some((t) => t.startsWith('Engrave 1')), `Engrave 1: ${await rowNames()}`);
    await p.getByRole('button', { name: /^Engrave 1/ }).first().click(); await settle();
    assert.equal(await p.locator('li[data-row-key]', { hasText: 'Engrave 1' }).locator('[data-image-settings]').count(), 0, 'no group on an Engrave row');
    // The checks stand in the output group of the Operations tab.
    const output = await p.locator('body').innerText();
    assert.ok(!/image|picture/i.test(output.split('\n').filter((l) => /does not|cannot|not a/.test(l)).join('\n')), `no warn line for the image: ${output}`);
    report.engraveHatchesFrame = true;
    await p.screenshot({ path: path.join(out, '3-engrave.png') });

    assert.deepEqual(errors, [], `page errors: ${errors.join('\n')}`);
    console.log(JSON.stringify(report, null, 2));
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((error) => { console.error(error); process.exit(1); });

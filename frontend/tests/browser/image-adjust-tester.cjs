// Tester browser check of `specs/0135-image-optimisation` slice 1, written from the spec: "Not
// engraved yet" for a picture in no operation (criterion 60), the preset tooltips and short cell
// (25), the strip's keyboard (25), the Strength slider's keys, field and hiding with Expert (26),
// the one slider (57), Restore original greyed not hidden and its undo (59). Starts its own dev
// server unless CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium; build the wasm
// bindings first. Screenshots: target/image-adjust-tester-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/image-adjust-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5193;

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
    p.on('pageerror', (e) => { errors.push(e.message); console.log('PAGEERROR', e.message); });
    p.on('console', (m) => { if (m.type() === 'error') console.log('CONSOLE', m.text().slice(0, 300)); });
    p.setDefaultTimeout(60000);
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(400);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const group = () => p.locator('[data-adjustments]');
    const preset = (name) => group().getByRole('radiogroup', { name: 'Preset' }).getByRole('radio', { name });
    const undo = async () => { await p.locator('canvas[data-state]').click({ position: { x: 5, y: 5 } }).catch(() => {}); await p.keyboard.press('Control+z'); await settle(); };

    // A laser page, zoomed out.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);
    for (let i = 0; i < 4; i += 1) { await p.keyboard.down('Control'); await p.mouse.wheel(0, 200); await p.keyboard.up('Control'); await settle(); }

    // A "photo": a lit sphere on a grey wall with some grain, 240 x 160, dropped on the canvas.
    const canvas = await p.locator('canvas[data-state]').boundingBox();
    const data = await p.evaluateHandle(async () => {
      const c = document.createElement('canvas');
      c.width = 240; c.height = 160;
      const g = c.getContext('2d');
      const wall = g.createLinearGradient(0, 0, 240, 160);
      wall.addColorStop(0, '#5a5a5a'); wall.addColorStop(1, '#c8c8c8');
      g.fillStyle = wall; g.fillRect(0, 0, 240, 160);
      const ball = g.createRadialGradient(95, 60, 6, 120, 80, 60);
      ball.addColorStop(0, '#fff'); ball.addColorStop(1, '#202020');
      g.fillStyle = ball; g.beginPath(); g.arc(120, 80, 60, 0, 7); g.fill();
      let seed = 7;
      for (let i = 0; i < 4000; i += 1) {
        seed = (seed * 1103515245 + 12345) & 0x7fffffff;
        g.fillStyle = `rgba(${seed % 255},${(seed >> 8) % 255},${(seed >> 16) % 255},0.12)`;
        g.fillRect(seed % 240, (seed >> 9) % 160, 1, 1);
      }
      const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
      const t = new DataTransfer();
      t.items.add(new File([blob], 'photo.png', { type: 'image/png' }));
      return t;
    });
    const cx = canvas.x + canvas.width / 2; const cy = canvas.y + canvas.height / 2;
    await p.dispatchEvent('canvas[data-state]', 'dragenter', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'dragover', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.dispatchEvent('canvas[data-state]', 'drop', { dataTransfer: data, clientX: cx, clientY: cy });
    await p.waitForTimeout(1500);


    // Criterion 60: a picture in no Image operation states "Not engraved yet" in its group.
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    await preset('Photo (detailed)').click(); await p.waitForTimeout(600);
    assert.match(await group().innerText(), /Not engraved yet/, 'no operation: Not engraved yet');
    await undo();
    assert.equal(await preset('Expert').getAttribute('aria-checked'), 'true', 'undo of a pick: Expert again');

    // Assign the picture to an Image operation.
    const kinds = p.locator('[role=group][aria-label=Operation] button');
    await kinds.last().click(); await settle();
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    assert.doesNotMatch(await group().innerText(), /Not engraved yet/);

    // Criterion 25: tooltips are "<name> · Dither: <mode>", nothing else; no description lines.
    const radios = group().getByRole('radiogroup', { name: 'Preset' }).getByRole('radio');
    const tips = [];
    for (let i = 0; i < 6; i += 1) {
      await radios.nth(i).hover(); await p.waitForTimeout(700);
      tips.push((await p.locator('[role=tooltip]').first().innerText().catch(() => '')).trim());
      await p.mouse.move(5, 5); await p.waitForTimeout(150);
    }
    const want = ['Photo (detailed) · Dither: Jarvis', 'Graphics (flat areas) · Dither: Ordered', 'Black and white (hard edges) · Dither: Threshold', 'Line art (outlines and handwriting) · Dither: Threshold', 'Dot raster (round dots) · Dither: Halftone', 'Newspaper (retro screen) · Dither: Lines'];
    for (let i = 0; i < want.length; i += 1) assert.ok(tips[i].startsWith(want[i]), `tooltip ${i}: ${tips[i]}`);
    const cellText = await radios.evaluateAll((els) => els.map((e) => e.textContent.trim()));
    assert.equal(cellText[4], 'Dots', 'short cell Dots');
    const text = await group().innerText();
    for (const sentence of ['stays close to the original', 'Picks the dither', 'Takes every improvement off']) {
      assert.ok(!text.includes(sentence), `no tip sentence: ${sentence}`);
    }

    // Criterion 25: the strip is keyboard operable; arrows move and select; each is one undo step.
    // Spec findings are collected and asserted at the end so every check runs.
    const findings = [];
    await preset('Expert').focus();
    await p.keyboard.press('ArrowLeft'); await p.waitForTimeout(500);
    if (await preset('Newspaper (retro screen)').getAttribute('aria-checked') !== 'true') {
      findings.push('criterion 25: ArrowLeft moves the focus but does not select (arrows move and select)');
      await p.keyboard.press('Space'); await p.waitForTimeout(500);
    }
    assert.equal(await preset('Newspaper (retro screen)').getAttribute('aria-checked'), 'true', 'selected');
    await p.keyboard.press('ArrowLeft'); await p.waitForTimeout(500);
    if (await preset('Dot raster (round dots)').getAttribute('aria-checked') !== 'true') { await p.keyboard.press('Space'); await p.waitForTimeout(500); }
    assert.equal(await preset('Dot raster (round dots)').getAttribute('aria-checked'), 'true');
    await undo();
    assert.equal(await preset('Newspaper (retro screen)').getAttribute('aria-checked'), 'true', 'one undo step per pick');

    // Criterion 26: the one slider; keyboard steps; field and slider agree; hidden with Expert.
    await preset('Photo (detailed)').click(); await p.waitForTimeout(600);
    assert.equal(await p.getByRole('slider').count(), 1, 'the Strength slider is the only slider');
    const slider = group().getByRole('slider', { name: 'Strength' });
    const field = group().getByLabel('Strength, percent');
    await slider.focus();
    await p.keyboard.press('Home'); await p.waitForTimeout(400);
    await p.keyboard.press('ArrowRight'); await p.waitForTimeout(400);
    assert.equal(await slider.getAttribute('aria-valuenow'), '1', 'Right +1');
    await p.keyboard.press('ArrowUp'); await p.waitForTimeout(400);
    assert.equal(await slider.getAttribute('aria-valuenow'), '2', 'Up +1');
    await p.keyboard.press('PageUp'); await p.waitForTimeout(400);
    assert.equal(await slider.getAttribute('aria-valuenow'), '12', 'PageUp +10');
    await p.keyboard.press('PageDown'); await p.waitForTimeout(400);
    await p.keyboard.press('ArrowDown'); await p.waitForTimeout(400);
    await p.keyboard.press('ArrowLeft'); await p.waitForTimeout(600);
    assert.equal(await slider.getAttribute('aria-valuenow'), '0', 'PageDown -10, Down and Left -1');
    assert.equal(await field.inputValue(), '0', 'the field shows the slider value');
    await p.keyboard.press('End'); await p.waitForTimeout(600);
    assert.equal(await field.inputValue(), '100');
    await field.fill('-1'); await p.keyboard.press('Enter'); await settle();
    assert.match(await group().innerText(), /0 and 100 %|0 to 100 %/, 'out of range names the range');
    await p.keyboard.press('Escape');
    await preset('Expert').click(); await p.waitForTimeout(600);
    assert.equal(await group().getByRole('slider', { name: 'Strength' }).count(), 0, 'hidden with Expert');
    assert.equal(await group().getByLabel('Strength, percent').count(), 0, 'field hidden too');
    await preset('Graphics (flat areas)').click(); await p.waitForTimeout(600);
    assert.equal(await group().getByRole('slider', { name: 'Strength' }).getAttribute('aria-valuenow'), '100', 'value kept');
    await shot('1-strength-keyboard');

    // Criterion 59: Restore original greyed, not hidden; enabled with a preset; one click and undo.
    const restore = group().getByRole('button', { name: 'Restore original' });
    assert.notEqual(await restore.getAttribute('aria-disabled'), 'true');
    await restore.hover(); await p.waitForTimeout(700);
    assert.equal((await p.locator('[role=tooltip]').first().innerText()).trim(), 'Restore original', 'tooltip is the name only');
    await p.mouse.move(5, 5); await p.waitForTimeout(150);
    await restore.click(); await p.waitForTimeout(600);
    assert.equal(await restore.count(), 1, 'still shown');
    assert.equal(await restore.getAttribute('aria-disabled'), 'true', 'greyed');
    assert.equal(await preset('Expert').getAttribute('aria-checked'), 'true');
    await undo();
    assert.equal(await preset('Graphics (flat areas)').getAttribute('aria-checked'), 'true', 'undo of Restore original');

    // Criterion 1, 57: the group is in the Style tab; no extra keyboard shortcut, no Add button.
    assert.equal(await group().getByRole('button', { name: /^Add$/ }).count(), 0, 'no Add button');
    assert.equal(await group().getByRole('combobox').count(), 0, 'no drop-down in the group');
    await shot('2-final');
    assert.deepEqual(errors, [], 'no page errors');
    assert.deepEqual(findings, [], 'spec findings');
    console.log('image-adjust-tester: all checks passed');
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });

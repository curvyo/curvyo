// Browser check of `specs/0135-image-optimisation` slice 1 (Part D): a photo on a laser page,
// assigned Image; the selected picture's Style tab shows the Adjustments group (Preset strip in two
// fixed rows, Strength, derived rows, Make editable, add toolbar, step list); every preset changes
// the dots in the Operations view and the Before and after pane; a Strength drag, Make editable, a
// step edit, Restore original and its undo; a linked clone with own adjustments while the
// original stays; Follow original; the picture's Dither strip with Halftone and Lines; two
// pictures with two presets in one Image operation keep different dots; the job file holds M4
// runs. Starts its own dev server unless CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a
// Chromium; build the wasm bindings first. Screenshots: target/image-adjust-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/image-adjust-browser');
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
    const pane = () => p.getByRole('region', { name: 'Before and after' });
    const undo = async () => { await p.locator('canvas[data-state]').click({ position: { x: 5, y: 5 } }).catch(() => {}); await p.keyboard.press('Control+z'); await settle(); };
    const steps = () => group().locator('[data-adjust-row]');

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

    // The Select bar's Image button assigns the picture; the Operations tab shows Image 1.
    const kinds = p.locator('[role=group][aria-label=Operation] button');
    await kinds.last().click(); await settle();
    await tab('Operations');
    if ((await p.locator('[data-row-body]').count()) === 0) { await p.getByRole('button', { name: /^Image 1/ }).first().click(); await settle(); }
    const commit = async (name, text) => { await p.getByLabel(name, { exact: true }).fill(text); await p.keyboard.press('Enter'); await settle(); };
    await commit('Power, percent', '40');
    await commit('Speed, millimetres per minute', '3000');
    // The picture is selected: its Style tab holds the group.
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    await shot('1-group-expert');
    assert.equal(await group().getByRole('heading', { name: 'Adjustments' }).count(), 1);
    assert.equal(await p.getByText('Stroke', { exact: true }).count(), 0, 'no Stroke and Fill for a picture');
    const names = await group().getByRole('radiogroup', { name: 'Preset' }).getByRole('radio').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.deepEqual(names, ['Photo (detailed)', 'Graphics (flat areas)', 'Black and white (hard edges)', 'Line art (outlines and handwriting)', 'Dot raster (round dots)', 'Newspaper (retro screen)', 'Expert']);
    const tops = await group().getByRole('radiogroup', { name: 'Preset' }).getByRole('radio').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    assert.ok(tops[0] === tops[2] && tops[3] === tops[6] && tops[2] < tops[3], `two fixed rows ${tops}`);
    assert.equal(await group().getByRole('slider', { name: 'Strength' }).count(), 0, 'no Strength with Expert');
    assert.equal(await group().getByRole('button', { name: 'Restore original' }).getAttribute('aria-disabled'), 'true', 'greyed, not hidden');
    assert.equal(await group().getByRole('toolbar', { name: 'Add adjustment' }).getByRole('button').count(), 9, 'slice 2: seven tone kinds and Crop, Resize');

    // Every preset changes the dots (canvas) and the pane's After.
    const lookAt = async () => p.screenshot({ clip: { x: 520, y: 470, width: 110, height: 90 } }).then((b) => require('node:crypto').createHash('md5').update(b).digest('hex'));
    const afterHash = () => pane().locator('[data-pane-half=after]').first().evaluate((c) => c.toDataURL().length + ':' + c.toDataURL().slice(-60));
    await group().getByRole('button', { name: 'Before and after' }).click(); await p.waitForTimeout(1500);
    await pane().waitFor();
    assert.equal(await group().getByRole('button', { name: 'Before and after' }).getAttribute('aria-pressed'), 'true');
    assert.equal(await pane().getByRole('radiogroup', { name: 'View' }).getByRole('radio', { checked: true }).getAttribute('aria-label'), 'Split');
    const paneBox = await pane().boundingBox(); const areaBox = await p.locator('canvas[data-state]').boundingBox();
    assert.ok(Math.abs(paneBox.width - areaBox.width / 2) < areaBox.width * 0.12, `pane about half: ${paneBox.width} of ${areaBox.width}`);
    await shot('2-pane-split');
    const seenDots = new Set(); const seenPane = new Set();
    for (const name of ['Photo (detailed)', 'Graphics (flat areas)', 'Black and white (hard edges)', 'Line art (outlines and handwriting)', 'Dot raster (round dots)', 'Newspaper (retro screen)']) {
      console.log('preset', name);
      await preset(name).click(); await p.waitForTimeout(1800);
      assert.equal(await preset(name).getAttribute('aria-checked'), 'true', name);
      seenDots.add(await lookAt()); seenPane.add(await afterHash());
      await shot(`3-preset-${name.split(' ')[0].toLowerCase()}`);
      assert.equal(await group().getByRole('slider', { name: 'Strength' }).count(), 1, 'Strength with a preset');
    }
    assert.ok(seenDots.size >= 4, `the canvas dots differ per preset: ${seenDots.size}`);
    assert.ok(seenPane.size >= 4, `the pane dots differ per preset: ${seenPane.size}`);
    await pane().getByRole('radio', { name: 'Side by side' }).click(); await settle();
    assert.equal(await pane().locator('[data-pane-half]').count(), 2);
    await shot('4-pane-side-by-side');
    await pane().getByRole('radio', { name: 'Split' }).click(); await settle();

    // Strength: a drag on the slider is held and written once at the release; one undo takes it back.
    await preset('Photo (detailed)').click(); await p.waitForTimeout(800);
    const slider = group().getByRole('slider', { name: 'Strength' });
    assert.equal(await slider.getAttribute('aria-valuenow'), '50');
    const sb = await slider.boundingBox();
    await p.mouse.move(sb.x + sb.width / 2, sb.y + sb.height / 2); await p.mouse.down();
    await p.mouse.move(sb.x + sb.width - 8, sb.y + sb.height / 2, { steps: 8 });
    assert.ok(Number(await slider.getAttribute('aria-valuenow')) > 85, 'the slider follows the drag');
    await p.mouse.up(); await p.waitForTimeout(800);
    const strong = await slider.getAttribute('aria-valuenow');
    assert.ok(Number(strong) > 85);
    assert.equal(await group().getByLabel('Strength, percent').inputValue(), strong, 'the field shows the same value');
    assert.match(await group().innerText(), new RegExp(`From preset Photo, Strength ${strong} %`));
    await slider.focus(); await p.keyboard.press('Home'); await p.waitForTimeout(600);
    assert.equal(await slider.getAttribute('aria-valuenow'), '0');
    await slider.press('End'); await p.waitForTimeout(600);
    assert.equal(await slider.getAttribute('aria-valuenow'), '100');
    await group().getByLabel('Strength, percent').fill('120'); await p.keyboard.press('Enter'); await settle();
    assert.match(await group().innerText(), /Between 0 and 100 %/);
    await p.keyboard.press('Escape');
    await group().getByLabel('Strength, percent').fill('30'); await p.keyboard.press('Enter'); await settle();
    assert.equal(await slider.getAttribute('aria-valuenow'), '30');
    await shot('5-strength');
    assert.equal(await group().locator('[data-derived-row]').count() >= 3, true, 'derived rows');

    // Make editable: the derived steps become manual rows and the preset reads Expert.
    const dotsBefore = await afterHash();
    await group().getByRole('button', { name: 'Make editable', exact: true }).click(); await p.waitForTimeout(1500);
    assert.equal(await preset('Expert').getAttribute('aria-checked'), 'true');
    const rows = await steps().count();
    assert.ok(rows >= 3, `manual rows ${rows}`);
    assert.equal(await group().getByRole('button', { name: /^Remove / }).count(), rows);
    assert.equal(await afterHash(), dotsBefore, 'the dots stay identical');
    await shot('6-manual-stack');
    // Edit a step: unfold the first row, change a field.
    const first = group().locator('[data-adjust-row]').first();
    await first.locator('[data-adjust-name]').click(); await settle();
    const lineBefore = await first.innerText();
    const inputs = first.locator('[data-adjust-body] input');
    assert.ok((await inputs.count()) >= 2, 'fields in the body');
    await inputs.first().fill('7'); await p.keyboard.press('Enter'); await p.waitForTimeout(900);
    await first.locator('[data-adjust-name]').click(); await settle();
    assert.notEqual(await first.innerText(), lineBefore, 'line 2 shows the new number');
    // Add a step at its pipeline place, unfolded with focus in its first field.
    const before = await steps().count();
    await group().getByRole('toolbar', { name: 'Add adjustment' }).getByRole('button', { name: 'Add Contours' }).click(); await p.waitForTimeout(900);
    assert.equal(await steps().count(), before + 1);
    assert.ok(await p.evaluate(() => document.activeElement?.closest('[data-adjust-body]') !== null), 'focus in the new body');
    const kindsNow = () => steps().evaluateAll((els) => els.map((e) => e.getAttribute('data-adjust-kind')));
    // Criteria 5, 16: the Levels body shows its input histogram; Stretch writes Black and White as
    // one step.
    const levelsRow = group().locator('[data-adjust-row][data-adjust-kind="levels"]').first();
    await levelsRow.locator('[data-adjust-name]').click(); await settle();
    assert.equal(await levelsRow.locator('[data-levels-graph] [data-levels-mark]').count(), 3, 'Black, Gamma and White marks');
    const barsPath = await levelsRow.locator('[data-levels-graph] > path').first().getAttribute('d');
    assert.ok(barsPath && barsPath.length > 50, 'the histogram bars are drawn');
    const blackField = levelsRow.locator('[data-adjust-field="black_pct"] input');
    await blackField.fill('40'); await p.keyboard.press('Enter'); await p.waitForTimeout(900);
    await shot('6b-levels-histogram');
    await levelsRow.getByRole('button', { name: 'Stretch', exact: true }).click(); await p.waitForTimeout(900);
    assert.notEqual(await blackField.inputValue(), '40', 'Stretch sets Black from the histogram');
    await undo();
    if ((await blackField.count()) === 0) { await levelsRow.locator('[data-adjust-name]').click(); await settle(); }
    assert.equal(await blackField.inputValue(), '40', 'one undo takes Stretch back');
    await levelsRow.locator('[data-adjust-name]').click(); await settle();
    // Criterion 2: a drag of a row over several places is one step.
    const orderDrag = await kindsNow();
    const source = await steps().last().locator('[data-adjust-name]').boundingBox();
    const firstBox = await steps().first().boundingBox();
    await p.mouse.move(source.x + 10, source.y + source.height / 2); await p.mouse.down();
    await p.mouse.move(source.x + 10, firstBox.y + 3, { steps: 12 }); await p.mouse.up(); await p.waitForTimeout(900);
    const dragged = await kindsNow();
    assert.equal(dragged[0], orderDrag[orderDrag.length - 1], `the last row lands first: ${dragged}`);
    await undo();
    assert.deepEqual(await kindsNow(), orderDrag, 'one undo takes the whole drag back');
    // Arrows move, the armed trash removes on the second press.
    const order0 = await kindsNow();
    await group().getByRole('button', { name: /^Move .* down$/ }).first().click(); await settle();
    assert.notDeepEqual(await kindsNow(), order0, 'a row moved');
    const trash = group().getByRole('button', { name: /^Remove / }).last();
    await trash.click(); await settle();
    assert.equal(await steps().count(), before + 1, 'armed, not removed');
    await p.waitForTimeout(500);
    await group().getByRole('button', { name: /^Press again to remove/ }).click(); await p.waitForTimeout(800);
    assert.equal(await steps().count(), before, 'second press removes');
    // A refused value shows its range.
    await group().locator('[data-adjust-row] [data-adjust-name]').first().click(); await settle();
    // Restore original: one step; undo brings the stack back.
    const rowsNow = await steps().count();
    await group().getByRole('button', { name: 'Restore original' }).click(); await p.waitForTimeout(900);
    assert.equal(await steps().count(), 0, 'no steps');
    assert.equal(await group().getByRole('button', { name: 'Restore original' }).getAttribute('aria-disabled'), 'true');
    await undo();
    assert.equal(await steps().count(), rowsNow, 'undo restores the stack');
    await shot('7-restore-undone');

    // Back to a preset for the clone steps.
    await preset('Photo (detailed)').click(); await p.waitForTimeout(800);
    await group().getByRole('button', { name: 'Before and after' }).click(); await settle();
    assert.equal(await pane().count(), 0, 'the pane closes with its button');

    // A linked clone: own adjustments leave the original unchanged; Original's follows again.
    await p.locator('canvas[data-state]').click({ position: { x: cx - canvas.x, y: cy - canvas.y } }); await settle();
    await p.getByRole('button', { name: 'Linked clone', exact: true }).first().click(); await settle();
    await p.keyboard.press('ArrowRight'); await p.keyboard.press('ArrowRight'); await settle();
    await tab('Style');
    const cloneRow = p.getByRole('radiogroup', { name: 'Adjustments' });
    await cloneRow.waitFor();
    assert.equal(await cloneRow.getByRole('radio', { name: "Original's" }).getAttribute('aria-checked'), 'true');
    assert.equal(await preset('Photo (detailed)').getAttribute('aria-checked'), 'true', "shows the original's");
    await preset('Graphics (flat areas)').click(); await p.waitForTimeout(900);
    assert.equal(await cloneRow.getByRole('radio', { name: 'Own' }).getAttribute('aria-checked'), 'true', 'the first edit makes it Own');
    await shot('8-clone-own');
    await p.getByRole('button', { name: 'Select original', exact: true }).click(); await settle();
    assert.equal(await preset('Photo (detailed)').getAttribute('aria-checked'), 'true', 'the original is unchanged');
    await p.getByRole('button', { name: 'Select clone', exact: true }).click(); await settle();
    await cloneRow.getByRole('radio', { name: "Original's" }).click(); await settle();
    assert.equal(await preset('Photo (detailed)').getAttribute('aria-checked'), 'true', 'Follow original');
    assert.equal(await cloneRow.getByRole('radio', { name: "Original's" }).getAttribute('aria-checked'), 'true');

    // The picture's Dither strip (Frage 283 B): nine modes in three fixed rows, the screens' fields
    // after Strength; the Image row of a picture-only operation shows no Dither of its own.
    await p.getByRole('button', { name: 'Select original', exact: true }).click(); await settle();
    const strip = group().getByRole('radiogroup', { name: 'Dither' });
    const modes = await strip.getByRole('radio').evaluateAll((els) => els.map((e) => [e.getAttribute('aria-label'), e.textContent.trim()]));
    assert.deepEqual(modes.slice(6), [['Grey', 'Grey'], ['Halftone dots', 'Halftone'], ['Line screen', 'Lines']]);
    assert.equal(await strip.getByRole('radio', { name: 'Jarvis' }).getAttribute('aria-checked'), 'true', 'Photo picked Jarvis');
    const dtops = await strip.getByRole('radio').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    assert.ok(dtops[0] === dtops[2] && dtops[3] === dtops[6] && dtops[7] === dtops[8] && dtops[2] < dtops[3] && dtops[6] < dtops[7], `three fixed rows ${dtops}`);
    const presetBottom = await group().getByRole('radiogroup', { name: 'Preset' }).evaluate((e) => e.getBoundingClientRect().bottom);
    const stripTop = await strip.evaluate((e) => e.getBoundingClientRect().top);
    assert.ok(Math.abs(stripTop - presetBottom - 8) <= 1, `8 px between the strips: ${stripTop - presetBottom}`);
    await strip.getByRole('radio', { name: 'Halftone dots' }).click(); await settle();
    assert.equal(await preset('Photo (detailed)').getAttribute('aria-checked'), 'true', 'the Preset is kept');
    assert.equal(await group().getByLabel('Screen angle, degrees').count(), 1);
    assert.equal(await group().getByLabel('Cell size or line pitch, millimetres').count(), 1);
    await shot('9-halftone');
    await strip.getByRole('radio', { name: 'Line screen' }).click(); await settle();
    assert.equal(await group().getByLabel('Screen angle, degrees').count(), 1);
    await shot('10-lines');
    await strip.getByRole('radio', { name: 'Jarvis' }).click(); await settle();
    await tab('Operations');
    if ((await p.locator('[data-image-settings]').count()) === 0) { await p.getByRole('button', { name: /^Image 1/ }).first().click(); await settle(); }
    assert.equal(await p.locator('[data-image-settings]').getByRole('radiogroup', { name: 'Dither' }).count(), 0, 'no Dither on a picture-only Image row');
    assert.equal(await p.getByText('Dither set in', { exact: false }).count(), 0, 'no notice');

    // Criterion 67: two pictures with different presets in ONE Image operation keep different dots.
    await tab('Style');
    const second = { x: cx + 120, y: cy + 60 };
    await p.dispatchEvent('canvas[data-state]', 'dragenter', { dataTransfer: data, clientX: second.x, clientY: second.y });
    await p.dispatchEvent('canvas[data-state]', 'dragover', { dataTransfer: data, clientX: second.x, clientY: second.y });
    await p.dispatchEvent('canvas[data-state]', 'drop', { dataTransfer: data, clientX: second.x, clientY: second.y });
    await p.waitForTimeout(1500);
    await tab('Operations');
    await p.getByRole('checkbox', { name: 'Image 1' }).click(); await settle();
    await tab('Style');
    await group().waitFor();
    await group().getByRole('button', { name: 'Before and after' }).click(); await p.waitForTimeout(1500);
    await preset('Photo (detailed)').click(); await p.waitForTimeout(1800);
    const secondPhoto = await afterHash();
    await preset('Dot raster (round dots)').click(); await p.waitForTimeout(1800);
    assert.equal(await strip.getByRole('radio', { name: 'Halftone dots' }).getAttribute('aria-checked'), 'true');
    const secondDots = await afterHash();
    assert.notEqual(secondDots, secondPhoto, 'the second picture burns Halftone dots');
    await shot('11-two-pictures-one-operation');
    await group().getByRole('button', { name: 'Before and after' }).click(); await settle();
    await p.locator('canvas[data-state]').click({ position: { x: cx - canvas.x, y: cy - canvas.y } }); await settle();
    await p.getByRole('button', { name: 'Select original', exact: true }).click().catch(() => {}); await settle();
    assert.equal(await strip.getByRole('radio', { name: 'Jarvis' }).getAttribute('aria-checked'), 'true', 'the first keeps Jarvis');
    await group().getByRole('button', { name: 'Before and after' }).click(); await p.waitForTimeout(1800);
    const firstDots = await afterHash();
    assert.notEqual(firstDots, secondDots, 'two pictures in one operation, two dot sets');
    await group().getByRole('button', { name: 'Before and after' }).click(); await settle();

    // The job file holds M4 runs.
    await tab('Operations');
    const download = p.waitForEvent('download');
    await p.locator('[data-export-job]').click();
    const file = await download;
    const target = path.join(out, file.suggestedFilename());
    await file.saveAs(target);
    const gcode = fs.readFileSync(target, 'utf8').split('\n');
    assert.ok(gcode.some((l) => /^M4 S0/.test(l)), 'M4 header');
    assert.ok(gcode.filter((l) => /^G1 .*S\d+/.test(l) && !/S0\b/.test(l)).length > 20, 'burned runs');

    // The group at 280 px (the panel body), alone.
    await tab('Style');
    await p.getByRole('button', { name: 'Select original', exact: true }).click().catch(() => {});
    await settle();
    await group().screenshot({ path: path.join(out, 'group-280.png') });
    assert.deepEqual(errors, [], 'no page errors');
    console.log('image-adjust: all checks passed');
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });

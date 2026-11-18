// Browser check of the gradient fill (`specs/0124-gradient-fill`, Part D): a rectangle gets the
// Gradient paint (the pixels are not uniform), its stops are edited in the panel, the handles are
// dragged (Shift snaps the angle, Ctrl moves the whole line), the entry boxes and the double-clicks
// work, Radial has its Radius, undo restores, a laser page draws the gradient grey while the panel
// keeps the colours, and an SVG with a reflected gradient opens with the import notice.
// Starts its own dev server unless CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium.
// Screenshots: target/0124-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0124-browser');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5184;
const REFLECT_SVG = path.join(root, 'curvyo-svg-core/tests/fixtures/svg_real/inkscape_multistop_chain.svg');

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
    const canvasEl = p.locator('canvas[data-state]');
    const canvas = await canvasEl.boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
    const drag = async (a, b) => { await move(a[0], a[1]); await p.mouse.down(); await move(b[0], b[1]); await p.mouse.up(); };
    const settle = () => p.waitForTimeout(250);
    const shot = (name) => p.screenshot({ path: path.join(shots, `${name}.png`) });
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    const kinds = p.getByRole('radiogroup', { name: 'Fill paint' });
    const block = p.locator('[data-gradient-block]');
    const rows = () => block.locator('[data-gradient-stop]');
    const field = (name) => p.getByRole('textbox', { name });

    // Screen positions of the handles drawn on the canvas, by kind and stop.
    const handle = async (kind, stop) => {
      const sel = stop === undefined ? `[data-gradient-handle="${kind}"]` : `[data-gradient-handle="${kind}"][data-stop="${stop}"]`;
      const box = await p.locator(sel).first().evaluate((g) => {
        const r = g.querySelector('circle:last-child, rect:last-child').getBoundingClientRect();
        return { x: r.x + r.width / 2, y: r.y + r.height / 2 };
      });
      return box;
    };
    const handleKinds = () => p.locator('[data-gradient-handle]').evaluateAll((els) => els.map((e) => `${e.getAttribute('data-gradient-handle')}:${e.getAttribute('data-stop')}`));
    // The mean colour of a 6x6 block of the canvas, read from a screenshot.
    const sample = async (x, y) => {
      const png = await p.screenshot({ clip: { x: canvas.x + x - 3, y: canvas.y + y - 3, width: 6, height: 6 } });
      return p.evaluate(async (b64) => {
        const img = new Image();
        img.src = `data:image/png;base64,${b64}`;
        await img.decode();
        const c = document.createElement('canvas');
        c.width = img.width; c.height = img.height;
        const ctx = c.getContext('2d');
        ctx.drawImage(img, 0, 0);
        const d = ctx.getImageData(0, 0, c.width, c.height).data;
        const n = d.length / 4;
        let r = 0, g = 0, b = 0;
        for (let i = 0; i < d.length; i += 4) { r += d[i]; g += d[i + 1]; b += d[i + 2]; }
        return [r / n, g / n, b / n];
      }, png.toString('base64'));
    };
    const spread = async (xs, y) => { const out = []; for (const x of xs) out.push(await sample(x, y)); return out; };

    // A rectangle, selected. The default gradient runs Start (the Solid colour) to End (white).
    await tool('Rectangle');
    await drag([250, 250], [650, 550]); await settle();
    await tool('Select');
    await move(900, 800); await p.keyboard.press('Control+a'); await settle();
    // Criterion 1: Gradient is the fourth paint item, a symbol with a tooltip, no text.
    assert.equal(await kinds.getByRole('radio').count(), 4, 'None, Solid, Pattern, Gradient');
    const item = kinds.getByRole('radio', { name: 'Gradient fill' });
    assert.equal((await item.innerText()).trim(), '', 'symbol only');
    await item.click(); await settle();
    await block.waitFor();
    assert.equal(await rows().count(), 2, 'two stops');
    assert.equal(await field('Gradient angle, degrees').inputValue(), '0.0');
    assert.equal(await p.getByRole('radiogroup', { name: 'Gradient kind' }).getByRole('radio', { name: 'Linear' }).getAttribute('aria-checked'), 'true');
    assert.equal(await p.locator('[data-style-section]').filter({ has: block }).getByText('Opacity', { exact: true }).count(), 0, 'no Opacity field in the Fill section with a gradient');
    assert.deepEqual((await handleKinds()).sort(), ['end:1', 'start:0'], 'two handles on a linear gradient');
    await shot('01-gradient-chosen');

    // The gradient is really drawn: the pixels along the box are not uniform and run to white.
    const line1 = await spread([270, 350, 450, 550, 630], 340);
    const lum = (c) => 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    assert.ok(Math.abs(lum(line1[0]) - lum(line1[4])) > 40, `gradient pixels vary: ${JSON.stringify(line1)}`);
    assert.ok(line1[4].every((v) => v > 205), `the End is white: ${line1[4]}`);

    // Criterion 3: the stop colours are typed in the hex, one step each.
    const hex0 = field('Stop 1 colour');
    await hex0.fill('#D81E1EFF'); await hex0.press('Enter'); await settle();
    assert.equal(await field('Stop 1 colour').inputValue(), '#D81E1EFF');
    await shot('01b-start-red');
    const red = await sample(270, 340);
    assert.ok(red[0] > 150 && red[1] < 90, `Start is red: ${red}`);
    await field('Stop 2 colour').fill('#1E3CD8FF'); await field('Stop 2 colour').press('Enter'); await settle();
    const blue = await sample(630, 340);
    assert.ok(blue[2] > 150 && blue[0] < 90, `End is blue: ${blue}`);

    // Add stop: a third row (the middle), inner, with Remove; Position typed; its colour typed.
    await p.getByRole('button', { name: 'Add stop', exact: true }).click(); await settle();
    assert.equal(await rows().count(), 3);
    assert.equal(await block.locator('[data-gradient-remove]').count(), 1, 'only the inner stop has Remove');
    assert.equal(await block.locator('[data-gradient-stop][data-active]').getAttribute('data-gradient-stop'), '1', 'the new stop is active');
    await field('Stop 2 colour').fill('#F2C21AFF'); await field('Stop 2 colour').press('Enter'); await settle();
    await field('Stop 2 position, percent').fill('25'); await field('Stop 2 position, percent').press('Enter'); await settle();
    assert.equal(await field('Stop 2 position, percent').inputValue(), '25');
    assert.deepEqual((await handleKinds()).sort(), ['end:2', 'start:0', 'stop:1']);
    await shot('02-three-stops');
    // Criterion 3: every row has its eyedropper after the swatch (UX review U1); it toggles picking.
    assert.equal(await block.locator('[data-eyedropper]').count(), 3, 'one eyedropper per stop');
    const pick2 = p.getByRole('button', { name: 'Pick colour of stop 2 from the drawing', exact: true });
    await pick2.focus(); await p.keyboard.press('Enter'); await settle();
    assert.equal(await pick2.getAttribute('aria-pressed'), 'true', 'picking for stop 2');
    await p.keyboard.press('Enter'); await settle();
    assert.equal(await pick2.getAttribute('aria-pressed'), 'false', 'a second press ends picking');
    // Criterion 7: the Tab order of a row is eyedropper, hex, Position, Remove, then the next row.
    const order = [];
    await p.getByRole('button', { name: 'Pick colour of stop 1 from the drawing', exact: true }).focus();
    for (let i = 0; i < 7; i += 1) {
      order.push(await p.evaluate(() => document.activeElement?.getAttribute('aria-label')));
      await p.keyboard.press('Tab');
    }
    assert.deepEqual(order, [
      'Pick colour of stop 1 from the drawing', 'Stop 1 colour',
      'Pick colour of stop 2 from the drawing', 'Stop 2 colour', 'Stop 2 position, percent', 'Remove stop 2',
      'Pick colour of stop 3 from the drawing',
    ], `Tab order: ${JSON.stringify(order)}`);
    await p.keyboard.press('Escape'); await settle();
    // UX review U3: the widest hex, eight digits of D, fits its field.
    const wide = field('Stop 1 colour');
    await wide.fill('#DDDDDDDD');
    const fit = await wide.evaluate((el) => {
      const row = el.closest('[data-gradient-stop]');
      return { scroll: el.scrollWidth, client: el.clientWidth, row: row?.scrollWidth, room: row?.parentElement?.clientWidth, block: el.closest('[data-gradient-block]')?.clientWidth };
    });
    assert.ok(fit.scroll <= fit.client, `#DDDDDDDD fits: ${JSON.stringify(fit)}`);
    await wide.press('Escape'); await settle();
    // Reverse: the order flips, the handles stay.
    await p.getByRole('button', { name: 'Reverse', exact: true }).click(); await settle();
    assert.equal(await field('Stop 1 colour').inputValue(), '#1E3CD8FF');
    assert.equal(await field('Stop 2 position, percent').inputValue(), '75');
    await p.getByRole('button', { name: 'Reverse', exact: true }).click(); await settle();
    assert.equal(await field('Stop 1 colour').inputValue(), '#D81E1EFF');
    // Remove the inner stop (its row active first, so the picker does not move under the press).
    await field('Stop 2 colour').focus(); await settle();
    await p.getByRole('button', { name: 'Remove stop 2', exact: true }).click(); await settle();
    assert.equal(await rows().count(), 2);
    // Invalid text keeps the field and shows the message.
    await field('Stop 1 colour').fill('#12'); await field('Stop 1 colour').press('Tab'); await settle();
    assert.equal(await p.getByText('Enter 3, 4, 6 or 8 hex digits').count(), 1);
    await field('Stop 1 colour').press('Escape'); await settle();

    // Criterion 6: dragging the End handle. Shift snaps the angle to the 15 degree stops.
    await tool('Select');
    await move(900, 800); await p.keyboard.press('Control+a'); await settle();
    let end = await handle('end', 1);
    await p.mouse.move(end.x, end.y, { steps: 3 });
    await p.mouse.down();
    await p.keyboard.down('Shift');
    await p.mouse.move(end.x - 60, end.y - 80, { steps: 6 });
    const readout = await p.locator('[data-gradient-readout]').innerText();
    assert.match(readout, /°$/, `a readout of the angle while dragging: ${readout}`);
    await shot('03-end-drag-shift');
    await p.mouse.up();
    await p.keyboard.up('Shift');
    await settle();
    const angle = Number.parseFloat(await field('Gradient angle, degrees').inputValue());
    assert.notEqual(angle, 0);
    assert.ok(Math.abs(angle / 15 - Math.round(angle / 15)) < 0.01 || Math.abs(angle / 22.5 - Math.round(angle / 22.5)) < 0.01, `Shift snapped: ${angle}`);

    // Ctrl moves the whole line: both ends move by the same vector.
    const s1 = await handle('start'); const e1 = await handle('end');
    await p.mouse.move(e1.x, e1.y, { steps: 3 });
    await p.mouse.down();
    await p.keyboard.down('Control');
    await p.mouse.move(e1.x + 30, e1.y + 25, { steps: 6 });
    await p.mouse.up();
    await p.keyboard.up('Control');
    await settle();
    const s2 = await handle('start'); const e2 = await handle('end');
    assert.ok(Math.abs((s2.x - s1.x) - (e2.x - e1.x)) < 2 && Math.abs((s2.y - s1.y) - (e2.y - e1.y)) < 2, 'both ends moved together');
    assert.ok(Math.abs(s2.x - s1.x) > 10, 'and they moved');
    assert.equal(Number.parseFloat(await field('Gradient angle, degrees').inputValue()), angle, 'the angle is kept');

    // A press on a handle activates its row: the panel tab is not switched, focus stays on the canvas.
    // Double-click on the line adds a stop there.
    const s3 = await handle('start'); const e3 = await handle('end');
    await p.mouse.dblclick((s3.x + e3.x) / 2, (s3.y + e3.y) / 2);
    await settle();
    assert.equal(await rows().count(), 3, 'a double-click on the line adds a stop');
    await shot('04-stop-added-on-canvas');
    // A press on the inner stop handle makes its row active and keeps the focus on the canvas.
    await field('Stop 1 colour').focus();
    const innerAt = await handle('stop', 1);
    await p.mouse.click(innerAt.x, innerAt.y); await settle();
    assert.equal(await block.locator('[data-gradient-stop][data-active]').getAttribute('data-gradient-stop'), '1');
    assert.equal(await p.evaluate(() => document.activeElement?.tagName), 'DIV', 'focus is on the canvas container');

    // Criterion 7: a double-click on an inner stop opens its entry box; Position typed, Enter writes.
    await p.mouse.dblclick(innerAt.x, innerAt.y); await settle();
    const entryBox = p.locator('[data-gradient-entry]');
    await entryBox.waitFor();
    assert.equal(await entryBox.getAttribute('aria-label'), 'Gradient stop');
    await shot('05-stop-entry-box');
    // The swatch button opens the picker inside the box; a second press closes it.
    await entryBox.getByRole('button', { name: 'Edit colour' }).click(); await settle();
    assert.equal(await entryBox.locator('[data-gradient-entry-picker]').count(), 1);
    await shot('06-stop-entry-picker');
    await entryBox.getByRole('button', { name: 'Edit colour' }).click(); await settle();
    assert.equal(await entryBox.locator('[data-gradient-entry-picker]').count(), 0);
    const pos = entryBox.getByRole('textbox', { name: 'Stop 2 position, percent' });
    await pos.fill('40');
    await pos.press('Enter'); await settle();
    assert.equal(await entryBox.count(), 0, 'Enter closes the box');
    assert.equal(await field('Stop 2 position, percent').inputValue(), '40');
    // Escape closes without a write.
    await p.mouse.dblclick((await handle('stop', 1)).x, (await handle('stop', 1)).y); await settle();
    await entryBox.getByRole('textbox', { name: 'Stop 2 position, percent' }).fill('90');
    await p.keyboard.press('Escape'); await settle();
    assert.equal(await entryBox.count(), 0);
    assert.equal(await field('Stop 2 position, percent').inputValue(), '40', 'Escape wrote nothing');
    // UX review U8: a press outside the box only closes it: the shape stays selected.
    await p.mouse.dblclick((await handle('stop', 1)).x, (await handle('stop', 1)).y); await settle();
    await entryBox.waitFor();
    await p.mouse.click(canvas.x + 1100, canvas.y + 750); await settle();
    assert.equal(await entryBox.count(), 0, 'the press outside closed the box');
    assert.equal(await block.count(), 1, 'and did not deselect the shape');

    // The end entry box: Angle with its swatch and hex; Enter writes the whole box in one step.
    const endAt = await handle('end');
    await p.mouse.dblclick(endAt.x, endAt.y); await settle();
    await entryBox.waitFor();
    assert.equal(await entryBox.getAttribute('aria-label'), 'Gradient end');
    await entryBox.getByRole('textbox', { name: 'Gradient angle, degrees' }).fill('45');
    await entryBox.getByRole('textbox', { name: 'Stop 3 colour' }).fill('#00AA55FF');
    await shot('07-end-entry-box');
    await entryBox.getByRole('textbox', { name: 'Stop 3 colour' }).press('Enter'); await settle();
    assert.equal(await field('Gradient angle, degrees').inputValue(), '45.0');
    assert.equal(await field('Stop 3 colour').inputValue(), '#00AA55FF');
    // Undo is one step: both come back together.
    await p.keyboard.press('Control+z'); await settle();
    assert.notEqual(await field('Gradient angle, degrees').inputValue(), '45.0', 'undo restores the angle');
    assert.equal(await field('Stop 3 colour').inputValue(), '#1E3CD8FF', 'and the colour');
    await p.keyboard.press('Control+Shift+z'); await settle();
    assert.equal(await field('Gradient angle, degrees').inputValue(), '45.0');

    // Radial: the Kind switch keeps the stops; the Radius field replaces Angle; centre and radius handles.
    await p.getByRole('radiogroup', { name: 'Gradient kind' }).getByRole('radio', { name: 'Radial' }).click(); await settle();
    assert.equal(await field('Gradient angle, degrees').count(), 0);
    const radius = await field('Gradient radius, millimetres').inputValue();
    assert.ok(Number.parseFloat(radius) > 0, `a radius: ${radius}`);
    assert.equal(await rows().count(), 3, 'the stops are kept');
    const kindsNow = (await handleKinds()).map((k) => k.split(':')[0]).sort();
    assert.deepEqual(kindsNow, ['centre', 'radius', 'stop'], 'centre, radius and the inner stop');
    assert.equal(await p.locator('[data-gradient-handles] ellipse, [data-gradient-handles] g[transform] circle').count() > 0, true, 'the circle is drawn');
    await shot('08-radial');
    const rad = await handle('radius');
    await p.mouse.move(rad.x, rad.y, { steps: 3 }); await p.mouse.down();
    await p.mouse.move(rad.x - 40, rad.y, { steps: 5 });
    assert.match(await p.locator('[data-gradient-readout]').innerText(), /^r [\d.]+ mm$/);
    await p.mouse.up(); await settle();
    assert.notEqual(await field('Gradient radius, millimetres').inputValue(), radius);
    const rad2 = await handle('radius');
    await p.mouse.dblclick(rad2.x, rad2.y); await settle();
    await entryBox.getByRole('textbox', { name: 'Gradient radius, millimetres' }).fill('40');
    await entryBox.getByRole('textbox', { name: 'Gradient radius, millimetres' }).press('Enter'); await settle();
    assert.equal(await field('Gradient radius, millimetres').inputValue(), '40');
    // The radial gradient is drawn: the centre differs from the rim.
    const centre = await handle('centre');
    const cc = await sample(centre.x - canvas.x + 2, centre.y - canvas.y + 30);
    const rim = await sample(centre.x - canvas.x + 190, centre.y - canvas.y + 100);
    assert.ok(Math.abs(lum(cc) - lum(rim)) > 15, `radial pixels vary: ${cc} ${rim}`);
    await shot('09-radial-radius-40');
    // Undo the kind switch: Linear returns.
    await p.keyboard.press('Control+z'); await settle(); await p.keyboard.press('Control+z'); await settle(); await p.keyboard.press('Control+z'); await settle();
    // UX review U10: the arrow keys switch the Kind, Radial and back to Linear.
    const kindGroup = p.getByRole('radiogroup', { name: 'Gradient kind' });
    await kindGroup.getByRole('radio', { name: 'Linear' }).focus();
    await p.keyboard.press('ArrowRight'); await settle();
    const after = await p.evaluate(() => ({ active: document.activeElement?.getAttribute('aria-label'), checked: [...document.querySelectorAll('[aria-label="Gradient kind"] [role=radio]')].map((r) => `${r.getAttribute('aria-label')}:${r.getAttribute('aria-checked')}`) }));
    assert.equal(await kindGroup.getByRole('radio', { name: 'Radial' }).getAttribute('aria-checked'), 'true', `ArrowRight picks Radial: ${JSON.stringify(after)}`);
    assert.equal(await field('Gradient radius, millimetres').count(), 1);
    await p.keyboard.press('ArrowLeft'); await settle();
    assert.equal(await kindGroup.getByRole('radio', { name: 'Linear' }).getAttribute('aria-checked'), 'true', 'ArrowLeft picks Linear again');
    assert.equal(await field('Gradient angle, degrees').count(), 1);
    await p.keyboard.press('Escape'); await settle();

    // Criterion 10: a laser page draws gradients grey, the panel keeps the colours.
    await p.getByRole('tab', { name: /^Document/ }).click(); await settle();
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click(); await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    // Zoomed out so the laser page lies beside Page 1 in the view; the rectangle goes on it.
    await p.mouse.click(600, 500);
    for (let i = 0; i < 4; i += 1) { await p.keyboard.down('Control'); await p.mouse.wheel(0, 200); await p.keyboard.up('Control'); await settle(); }
    await tool('Rectangle');
    await p.mouse.move(430, 400); await p.mouse.down(); await p.mouse.move(540, 470, { steps: 5 }); await p.mouse.up(); await settle();
    await tool('Select');
    await p.getByRole('tab', { name: /^Style/ }).click(); await settle();
    await kinds.getByRole('radio', { name: 'Gradient fill' }).click(); await settle();
    await field('Stop 1 colour').fill('#D81E1EFF'); await field('Stop 1 colour').press('Enter'); await settle();
    await field('Stop 2 colour').fill('#F2E01AFF'); await field('Stop 2 colour').press('Enter'); await settle();
    // The Operations view (on by default on a laser page) draws every fill as its kind look; off, the canvas shows the artwork.
    await p.locator('button[aria-pressed="true"][aria-label*="Operations"]').click(); await settle();
    await p.mouse.move(900, 800); await settle();
    await shot('10-laser-page-grey');
    const greys = await spread([436 - canvas.x, 470 - canvas.x, 500 - canvas.x, 534 - canvas.x], 415 - canvas.y);
    assert.ok(greys.every((c) => Math.abs(c[0] - c[1]) < 4 && Math.abs(c[1] - c[2]) < 4), `grey on a laser page: ${JSON.stringify(greys)}`);
    assert.ok(Math.abs(lum(greys[0]) - lum(greys[3])) > 8, `and still a gradient: ${JSON.stringify(greys)}`);
    assert.equal(await field('Stop 1 colour').inputValue(), '#D81E1EFF', 'the panel keeps the colour');
    // UX review U4: above 40 stops the list scrolls and renders only the rows in view.
    for (let i = 0; i < 42; i += 1) {
      await p.getByRole('button', { name: 'Add stop', exact: true }).click();
    }
    await settle();
    await block.locator('[data-gradient-stop-window]').waitFor();
    const drawnRows = await rows().count();
    assert.ok(drawnRows > 0 && drawnRows < 30, `44 stops, ${drawnRows} rows rendered`);
    await shot('10b-long-stop-list');

    // Criterion 14: an SVG with a reflected gradient opens (in a fresh editor, with nothing unsaved)
    // with the import notice; the gradient is drawn, not left out.
    const q = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    q.on('pageerror', (e) => errors.push(e.message));
    await q.goto(url, { waitUntil: 'networkidle' });
    await q.getByRole('tab', { name: /^File/ }).click();
    const chooser = q.waitForEvent('filechooser');
    await q.getByRole('button', { name: 'Open…', exact: true }).click();
    await (await chooser).setFiles(REFLECT_SVG);
    await q.getByText('1 gradient repeat was shown as a plain shade.').waitFor({ timeout: 8000 });
    await q.screenshot({ path: path.join(shots, '11-import-notice.png') });
    await q.close();

    assert.deepEqual(errors, [], `page errors: ${errors.join('\n')}`);
    console.log('gradient fill browser check: ok');
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });

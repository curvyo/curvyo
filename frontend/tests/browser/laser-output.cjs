// Browser check of `specs/0029-laser-job-preview-and-output` (Part D and the rework R4): draw two
// shapes on a laser page, assign Cut, read the Operations tab with its output group (blocked, then
// with numbers on the rows), the Route toggle (exclusive with the Operations view), the play bar
// (keys, Play, time only), the Start pick (mouse and keys), Air assist pressed twice at one point
// and Export job file (a download whose first lines are the GRBL header). Starts its own dev server unless
// CURVYO_BROWSER_URL is set; build the wasm bindings first. Screenshots: target/laser-output-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/laser-output-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5188;

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
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CURVYO_E2E_BROWSER || undefined, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(300);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const drag = async (x1, y1, x2, y2) => { await p.mouse.move(x1, y1); await p.mouse.down(); await p.mouse.move(x2, y2, { steps: 5 }); await p.mouse.up(); await settle(); };
    const readout = () => p.locator('[data-play-readout]').innerText();
    const pressed = (name) => p.getByRole('button', { name, exact: true }).getAttribute('aria-pressed');

    // A laser page beside Page 1, zoomed out so both pages show.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);
    for (let i = 0; i < 4; i += 1) { await p.keyboard.down('Control'); await p.mouse.wheel(0, 200); await p.keyboard.up('Control'); await settle(); }

    // Nothing runs: the output group of the Operations tab says so, no Output tab exists.
    await tab('Operations');
    const tabs0 = await p.locator('[role=tab]').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label')));
    assert.ok(!tabs0.includes('Output'), `no Output tab: ${tabs0}`);
    const group = p.locator('[data-output-group]');
    assert.match(await group.locator('[role=list][aria-label=Checks]').innerText(), /Nothing runs\. Assign items to an operation\./);
    await shot('0-nothing-runs');
    await tab('Document');

    // Two shapes on the laser page, both assigned to Cut.
    await p.getByRole('button', { name: /^Rectangle tool/ }).click();
    await drag(430, 400, 540, 470);
    await p.getByRole('button', { name: /^Ellipse tool/ }).click();
    await drag(580, 420, 700, 560);
    await p.getByRole('button', { name: /^Select tool/ }).click();
    await p.keyboard.press('Control+a');
    await settle();
    await p.locator('button[aria-label="Cut"]').last().click();
    await settle();

    // Blocked: no power and speed. Export is aria-disabled, still focusable, and presses do nothing.
    await tab('Operations');
    await shot('1-blocked');
    const exportButton = p.locator('[data-export-job]');
    assert.equal(await exportButton.getAttribute('aria-disabled'), 'true');
    assert.equal(await exportButton.getAttribute('disabled'), null, 'a Tab stop, not disabled');
    assert.match(await p.locator('#output-export-reason').innerText(), /Cut 1 has no power and speed\./);
    assert.equal(await p.getByRole('button', { name: 'Export job file' }).count(), 1);
    assert.match(await p.locator('[role=list][aria-label=Checks]').innerText(), /Cut 1 has no power and speed\./);
    assert.match(await p.locator('li[data-row-key]').first().innerText(), /No settings/, 'a row without numbers says so on line 3');
    // The disabled Export's tooltip opens above it.
    await exportButton.hover();
    const tip = p.locator('[data-radix-popper-content-wrapper] [data-side]').first();
    await tip.waitFor();
    assert.equal(await tip.getAttribute('data-side'), 'top');
    await p.mouse.move(10, 990); await settle();

    // The check's name unfolds its row in the same tab, with the focus on its first field.
    await p.locator('[role=list][aria-label=Checks] button', { hasText: 'Cut 1' }).click();
    await settle();
    assert.equal(await p.locator('[role=tab][aria-label^="Operations"]').getAttribute('aria-selected'), 'true');
    assert.equal(await p.evaluate(() => document.activeElement?.getAttribute('aria-label')), 'Power, percent', 'focus on the first field');
    await shot('1b-check-opened');
    await p.getByLabel('Power, percent').fill('60'); await p.keyboard.press('Enter');
    await p.getByLabel('Speed, millimetres per minute').fill('600'); await p.keyboard.press('Enter');
    await p.getByLabel('Passes, passes').fill('2'); await p.keyboard.press('Enter');
    await settle();

    // Ready: the row has its run ring, time and numbers; the group a total and an enabled button.
    assert.equal(await exportButton.getAttribute('aria-disabled'), null);
    const row = p.locator('li[data-row-key]').first();
    assert.equal(await row.locator('[data-run-ring]').innerText(), '1');
    assert.match(await row.locator('[data-run-time]').innerText(), /^\d+ (s|min)/);
    assert.match(await row.innerText(), /Cut · 2 items[\s\S]*60 %, 600 mm\/min, 2 ×/);
    assert.match(await row.innerText(), /Own numbers/, 'the unfolded row names its material pick');
    assert.match(await p.getByRole('button', { name: /^Cut 1, run 1, / }).getAttribute('aria-label'), /^Cut 1, run 1, \d+ (s|min)/);
    assert.match(await p.locator('[aria-label=Total]').innerText(), /About [^\n]*\n+Runs 1 operation, 2 items\.\n+Burn .* · travel /);
    // "2 items" of the work line is a button that selects the running items (0101 criterion 17).
    await p.locator('[data-total-runs]').getByRole('button', { name: '2 items', exact: true }).click();
    await settle();
    assert.equal(await p.locator('[data-output-divider]').count() >= 2, true, 'dividers before the checks or the total');
    assert.match(await p.locator('[data-output-machine]').innerText(), /^Generic laser · GRBL$/);
    assert.match(await exportButton.innerText(), /Export job file/);
    // Folded, the row shows numbers only: Own numbers stays in the unfolded body.
    await p.getByRole('button', { name: /^Cut 1, run 1, / }).click(); await settle();
    assert.doesNotMatch(await p.locator('li[data-row-key]').first().innerText(), /Own numbers|Material/);
    await shot('1c-ready');

    // Route: pressed, exclusive with the Operations view, a bar and the ring number.
    assert.equal(await pressed('Operations view'), 'true');
    await p.getByRole('button', { name: 'Toolpath view', exact: true }).click();
    await settle();
    assert.equal(await pressed('Toolpath view'), 'true');
    assert.equal(await pressed('Operations view'), 'false');
    assert.equal(await p.locator('[data-ring-label="1"]').count(), 1);
    assert.equal(await p.locator('[data-play-bar]').count(), 1);
    const end = await readout();
    assert.match(end, /^(.+) of \1$/, `the bar starts at the end, time only: ${end}`);
    assert.doesNotMatch(end, /\d\s*m\b(?!in)/, 'no metres in the readout');
    assert.equal(await p.locator('[data-play-bar]').evaluate((bar) => bar.getBoundingClientRect().height), 40, 'one line');
    await shot('2-route');
    // The bar keeps clear of the tool columns, and sticks 12 px above the canvas bottom once the
    // frame's bottom has left the view upwards.
    const barBox = async () => p.locator('[data-play-bar]').evaluate((bar) => {
      const parent = bar.parentElement.getBoundingClientRect();
      const box = bar.getBoundingClientRect();
      const rail = parseFloat(getComputedStyle(bar.parentElement).getPropertyValue('--rail-right'));
      return { left: box.left - parent.left, gap: parent.bottom - box.bottom, rail };
    });
    const placed = await barBox();
    assert.ok(placed.rail > 0 && placed.left >= placed.rail + 12, `clear of the rail: ${JSON.stringify(placed)}`);
    for (let i = 0; i < 12; i += 1) { await p.mouse.move(900, 500); await p.mouse.wheel(0, 300); }
    await settle();
    const stuck = await barBox();
    assert.ok(Math.abs(stuck.gap - 12) < 1, `stuck 12 px above the bottom: ${JSON.stringify(stuck)}`);
    await shot('2b-route-stuck');
    for (let i = 0; i < 12; i += 1) { await p.mouse.move(900, 500); await p.mouse.wheel(0, -300); }
    await settle();

    // The slider keys: Home, then Right moves 1 %, End returns.
    const slider = p.getByRole('slider', { name: 'Run position' });
    await slider.focus();
    await p.keyboard.press('Home'); await settle();
    assert.match(await readout(), /^0 s of /);
    await p.keyboard.press('PageUp'); await settle();
    assert.notEqual(await slider.getAttribute('aria-valuenow'), '0');
    assert.equal(await slider.getAttribute('aria-valuetext'), await readout());
    await p.keyboard.press('End'); await settle();
    assert.equal(await readout(), end);

    // Play from the start: the head moves and Pause is offered; Pause stops it.
    await p.keyboard.press('Home'); await settle();
    await p.getByRole('button', { name: 'Play', exact: true }).click();
    await p.waitForTimeout(1200);
    await p.getByRole('button', { name: 'Pause', exact: true }).waitFor();
    const playing = await readout();
    assert.doesNotMatch(playing, /^0 s of /, `moved: ${playing}`);
    await shot('3-playing');
    await p.getByRole('button', { name: 'Pause', exact: true }).click();
    await settle();
    const paused = await readout();
    await p.waitForTimeout(400);
    assert.equal(await readout(), paused, 'Pause holds the head');

    // The pair is exclusive: the Operations view releases Route and the bar goes.
    await p.getByRole('button', { name: 'Operations view', exact: true }).click();
    await settle();
    assert.equal(await pressed('Toolpath view'), 'false');
    assert.equal(await pressed('Operations view'), 'true');
    assert.equal(await p.locator('[data-play-bar]').count(), 0);
    await p.getByRole('button', { name: 'Toolpath view', exact: true }).click();
    await settle();

    // The Start pick: three plain options, none greyed; a press writes one step; a check can open it.
    await tab('Operations');
    const startPick = p.locator('[data-start-pick]');
    assert.match(await startPick.innerText(), /Start\s*Machine origin/);
    await startPick.click(); await settle();
    const options = await p.getByRole('radiogroup', { name: 'Start' }).getByRole('radio').evaluateAll((els) => els.map((e) => [e.innerText, e.getAttribute('aria-disabled')]));
    assert.deepEqual(options, [['Machine origin', null], ['Head position', null], ['Home, then machine origin', null]]);
    await p.getByRole('radio', { name: 'Head position', exact: true }).click(); await settle();
    assert.match(await startPick.innerText(), /Head position/);
    assert.doesNotMatch(await p.locator('[role=list][aria-label=Checks]').innerText(), /head stands/, 'no Head position warning');
    // Keyboard: Enter opens on the picked row, arrows move, Enter picks, focus returns to the button.
    await startPick.focus(); await p.keyboard.press('Enter'); await settle();
    assert.equal(await p.evaluate(() => document.activeElement?.textContent), 'Head position');
    await p.keyboard.press('ArrowDown'); await p.keyboard.press('Enter'); await settle();
    assert.match(await startPick.innerText(), /Home, then machine origin/);
    assert.equal(await p.evaluate(() => document.activeElement?.hasAttribute('data-start-pick')), true);
    await startPick.focus(); await p.keyboard.press('Enter'); await p.keyboard.press('Escape'); await settle();
    assert.equal(await p.getByRole('radiogroup', { name: 'Start' }).count(), 0, 'Escape closes');
    await shot('4-start');
    await startPick.click(); await settle();
    await p.getByRole('radio', { name: 'Machine origin', exact: true }).click(); await settle();

    // Fixture jobs (UX 6.2, P32): a Pin's Centre mark with Output on shows the pick, one run line and
    // the header time at the top of the section; the check's "Fixture jobs" opens the section and
    // focuses the pick; the pick takes the keys of the Start pick.
    await tab('Fixtures');
    await p.getByRole('button', { name: 'Pin', exact: true }).click(); await settle();
    await p.mouse.click(450, 600); await settle();
    await tab('Operations');
    const fixtureHeader = p.getByRole('button', { name: /^Fixture jobs \(/ });
    if ((await fixtureHeader.getAttribute('aria-expanded')) !== 'true') await fixtureHeader.click();
    await settle();
    assert.equal(await p.locator('[data-fixture-pick]').count(), 0, 'no pick while no fixture job runs');
    await p.getByRole('button', { name: /^Output Centre mark/ }).click(); await settle();
    assert.equal(await p.locator('[data-fixture-pick]').count(), 1, 'the pick shows while a fixture job runs');
    assert.match(await p.locator('[data-fixture-run="score"]').innerText(), /Score\s*\S+\s*No settings/);
    // No pick, no set: no run has a time, so the header shows "–" (UX review U4).
    assert.equal(await p.locator('[data-fixture-time]').innerText(), '–');
    assert.equal(await p.locator('[data-fixture-ring]').count(), 1, 'the run line has the fixture ring');
    await shot('4c-fixture-jobs');
    // Collapse the section, then press the check: it opens the section and focuses the pick.
    await fixtureHeader.click(); await settle();
    assert.equal(await p.locator('[data-fixture-pick]').count(), 0);
    await p.locator('[role=list][aria-label=Checks] button', { hasText: 'Fixture jobs' }).click(); await settle();
    assert.equal(await p.evaluate(() => document.activeElement?.hasAttribute('data-fixture-pick')), true, 'focus on the pick');
    await p.keyboard.press('Enter'); await settle();
    assert.equal(await p.getByRole('radiogroup', { name: 'Fixture jobs run with' }).getByRole('radio').first().getAttribute('aria-checked'), 'true', 'None is picked');
    await p.keyboard.press('Escape'); await settle();
    assert.equal(await p.getByRole('radiogroup', { name: 'Fixture jobs run with' }).count(), 0, 'Escape closes');
    assert.equal(await p.evaluate(() => document.activeElement?.hasAttribute('data-fixture-pick')), true, 'focus back on the pick');
    await p.getByRole('button', { name: /^Output Centre mark/ }).click(); await settle();
    assert.equal(await p.locator('[data-fixture-pick]').count(), 0, 'the pick leaves with the last running fixture job');

    // Air assist: the machine line follows. In the Document tab the switch sits at the row's right
    // edge and a second press at the same screen point turns it off again (nothing moves under the
    // pointer, P31).
    await tab('Document');
    const air = p.getByRole('switch', { name: 'Air assist' });
    const motorised = p.getByRole('switch', { name: 'Motorised Z axis' });
    const rowBox = await air.evaluate((el) => {
      const row = el.closest('div').getBoundingClientRect();
      const box = el.getBoundingClientRect();
      return { rowRight: row.right, switchRight: box.right };
    });
    assert.ok(Math.abs(rowBox.rowRight - rowBox.switchRight) < 2, `switch at the row's right edge: ${JSON.stringify(rowBox)}`);
    assert.equal(await p.locator('[data-machine-state-row]').evaluate((el) => el.getBoundingClientRect().height), 28, 'the state row keeps its place');
    const at = await air.boundingBox();
    const point = { x: at.x + at.width / 2, y: at.y + at.height / 2 };
    const zBefore = await motorised.getAttribute('aria-checked');
    // Tester (criterion 40): no row's top edge moves when Air assist changes.
    const tops = () => p.locator('[role=switch]').evaluateAll((els) => els.map((e) => Math.round(e.getBoundingClientRect().top)));
    const topsBefore = await tops();
    await p.mouse.click(point.x, point.y); await settle();
    assert.deepEqual(await tops(), topsBefore, 'no switch row moved after the first press');
    assert.equal(await p.getByRole('switch', { name: 'Air assist' }).getAttribute('aria-checked'), 'true', 'first press: on');
    assert.match(await p.locator('[data-machine-state-row]').innerText(), /Differs from profile/);
    await p.mouse.click(point.x, point.y); await settle();
    assert.equal(await p.getByRole('switch', { name: 'Air assist' }).getAttribute('aria-checked'), 'false', 'second press at the same point: off');
    assert.equal(await motorised.getAttribute('aria-checked'), zBefore, 'Motorised Z axis untouched');
    await shot('4b-air-assist-twice');
    await p.mouse.click(point.x, point.y); await settle();
    await tab('Operations');
    assert.match(await p.locator('[data-output-machine]').innerText(), /^Generic laser · GRBL · air assist$/);
    await tab('Document');
    await p.mouse.click(point.x, point.y); await settle();
    await tab('Operations');

    // Export job file: a download named by project and page, GRBL header first, one notice.
    const download = p.waitForEvent('download');
    await exportButton.click();
    const file = await download;
    assert.equal(file.suggestedFilename(), 'Untitled - Laser 1.gcode');
    const target = path.join(out, file.suggestedFilename());
    await file.saveAs(target);
    const lines = fs.readFileSync(target, 'utf8').split('\n');
    assert.equal(lines[0], '; Curvyo job: Laser 1');
    assert.ok(lines.includes('G21') && lines.includes('G90'), 'units and absolute mode');
    assert.ok(lines.some((l) => /^M4 S0/.test(l)), 'dynamic power on, laser off');
    await p.getByText('Downloaded Untitled - Laser 1.gcode.').waitFor();
    await shot('5-exported');

    assert.deepEqual(errors, []);
    console.log('laser output browser check: ok');
  } finally {
    await browser.close();
  }
})().catch((error) => { console.error(error); process.exit(1); });

// Tester journey for the pattern fill (`specs/0071-pattern-fill`), independent of
// pattern-fill.cjs: placement (no rail tool, no top bar), Escape during a field drag, a drag
// release as one undo step, the count note and the over-limit refusal, a node drag that
// regenerates the pattern, Convert to paths from the panel only (criterion 24),
// copy and paste keeping the seed, and undo of the fill-kind switch. Every check is recorded and
// printed; the script exits 1 if any failed. Starts its own dev server on port 5183 unless
// CURVYO_BROWSER_URL is set. Screenshots: target/0071-browser-tester/.
const { chromium } = require('playwright');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const shots = path.join(root, 'target/0071-browser-tester');
fs.mkdirSync(shots, { recursive: true });
const PORT = 5183;
const results = [];
const check = (name, ok, detail = '') => {
  results.push({ name, ok: Boolean(ok), detail });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${detail ? `: ${detail}` : ''}`);
};

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then(() => true, () => false)) return;
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
  const errors = [];
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const canvasEl = p.locator('canvas[data-state]');
    const canvas = await canvasEl.boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps: 4 }); };
    const drag = async (a, b) => { await move(a[0], a[1]); await p.mouse.down(); await move(b[0], b[1]); await p.mouse.up(); };
    const settle = (ms = 150) => p.waitForTimeout(ms);
    const shot = (name) => p.screenshot({ path: path.join(shots, `${name}.png`) });
    const canvasShot = () => p.screenshot({ clip: canvas });
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    const block = p.locator('[data-pattern-block]');
    const field = (name) => p.getByRole('spinbutton', { name });
    const value = async (name) => field(name).getAttribute('aria-valuenow');
    const kinds = p.getByRole('radiogroup', { name: 'Fill paint' });
    const tiles = p.getByRole('radiogroup', { name: 'Pattern generator' });
    // Double click opens typing; its first click may set the value on the bar, so the value
    // before the typed text is returned.
    const typeInto = async (name, text) => {
      await field(name).dblclick(); await settle();
      const prior = await p.locator(':focus').inputValue().catch(() => null);
      await p.keyboard.press('Control+a');
      await p.keyboard.type(text);
      await p.keyboard.press('Enter'); await settle(300);
      return prior;
    };

    // Placement: a filled rectangle, Pattern from the Fill paint group only.
    check('no rail tool for patterns', (await p.getByRole('button', { name: /pattern/i }).count()) === 0,
      `${await p.getByRole('button', { name: /pattern/i }).count()} buttons named Pattern before any selection`);
    await tool('Rectangle');
    await drag([250, 200], [650, 600]); await settle();
    await tool('Select');
    await move(250, 400); await p.mouse.down(); await p.mouse.up(); await settle();
    const plain = await canvasShot();
    await kinds.getByRole('radio', { name: 'Pattern fill' }).click(); await settle();
    await block.waitFor();
    check('criterion 1: the block appears with ten tiles', (await tiles.getByRole('radio').count()) === 10);
    const styleTab = p.getByRole('tab', { name: /^Style/ });
    check('criterion 1: in the Style tab', (await styleTab.getAttribute('aria-selected')) === 'true');
    const patterned = await canvasShot();
    check('criterion 1: the canvas shows the pattern', !plain.equals(patterned));
    await shot('01-hatch');

    // Criterion 2: Escape during a field drag restores the value and the canvas.
    const spacing0 = await value('Pattern spacing');
    const box = await field('Pattern spacing').boundingBox();
    await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await p.mouse.down();
    await p.mouse.move(box.x + box.width / 2 + 80, box.y + box.height / 2, { steps: 8 }); await settle();
    const dragged = await value('Pattern spacing');
    const preview = await canvasShot();
    check('criterion 2: dragging previews a new value', dragged !== spacing0, `${spacing0} -> ${dragged}`);
    check('criterion 2: the canvas updates while dragging', !preview.equals(patterned));
    await p.keyboard.press('Escape'); await settle();
    await p.mouse.up(); await settle();
    check('criterion 2: Escape restores the value', (await value('Pattern spacing')) === spacing0, `now ${await value('Pattern spacing')}`);
    const escaped = await canvasShot();
    fs.writeFileSync(path.join(shots, 'escape-before.png'), patterned);
    fs.writeFileSync(path.join(shots, 'escape-after.png'), escaped);
    check('criterion 2: Escape restores the canvas', escaped.equals(patterned), 'see escape-before/after.png');

    // A drag and release is one step; Ctrl+Z restores.
    await p.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await p.mouse.down();
    await p.mouse.move(box.x + box.width / 2 + 60, box.y + box.height / 2, { steps: 8 });
    await p.mouse.up(); await settle();
    const released = await value('Pattern spacing');
    check('criterion 2: release commits', released !== spacing0, released);
    await canvasEl.focus().catch(() => {});
    await move(250, 400); await p.mouse.down(); await p.mouse.up(); await settle();
    await p.keyboard.press('Control+z'); await settle(300);
    check('criterion 2: one undo restores the spacing', (await value('Pattern spacing')) === spacing0, `after undo ${await value('Pattern spacing')}`);
    check('criterion 2: and the pattern is still chosen', (await block.count()) === 1);

    // Criterion 18: over the limit the value is refused with the reason; between, a quiet count.
    await tiles.getByRole('radio', { name: 'Dots', exact: true }).click(); await settle();
    const dotSpacing = await value('Pattern spacing');
    await typeInto('Pattern diameter', '0.05');
    const prior = await typeInto('Pattern spacing', '0.05');
    const alert = p.locator('[data-pattern-block] [role="alert"]');
    const alertText = (await alert.count()) ? await alert.first().innerText() : '';
    const blockText = await block.innerText();
    check('criterion 18: over the limit the section shows the reason and the count',
      /500,000/.test(blockText) && /Not generated/.test(blockText), alertText || blockText.split('\n').slice(-3).join(' | '));
    check('criterion 18: the spacing stays at its previous value', (await value('Pattern spacing')) === prior, `${await value('Pattern spacing')} vs ${prior} (Dots started at ${dotSpacing})`);
    await shot('02-over-limit');
    await typeInto('Pattern spacing', '0.4');
    await settle(8500); // the refusal text stays 8 s (UX notes section 7)
    const note = await block.innerText();
    const count = note.match(/([\d,]+) pieces/);
    check('criterion 18: between the limits a quiet count', count !== null, count ? count[0] : note.split('\n').slice(-2).join(' | '));
    await shot('03-note');
    await typeInto('Pattern spacing', '3');
    await typeInto('Pattern diameter', '1');

    // Criterion 19: a resize regenerates the pattern into the new area (no extra step).
    const clipAt = (x, y) => p.screenshot({ clip: { x: canvas.x + x, y: canvas.y + y, width: 40, height: 40 } });
    const blank = await clipAt(880, 850);
    check('criterion 19: before the resize the new area is empty', (await clipAt(740, 650)).equals(blank));
    await drag([650, 600], [800, 700]); await settle(500);
    check('criterion 19: after the resize the pattern fills the new area', !(await clipAt(740, 650)).equals(blank));
    await shot('04-resized');
    await move(1000, 850); await p.mouse.down(); await p.mouse.up(); await settle(600);

    // Criterion 24 (panel-only): no Object-menu or context-menu entry; the panel button works.
    await move(250, 400); await p.mouse.down(); await p.mouse.up(); await settle(600);
    const objectMenu = p.getByRole('button', { name: /^Object/ }).or(p.getByRole('menuitem', { name: /^Object/ }));
    let menuEntry = false;
    if (await objectMenu.count()) {
      await objectMenu.first().click(); await settle();
      menuEntry = (await p.getByRole('menuitem', { name: /Convert.*pattern|pattern.*path/i }).count()) > 0;
      await p.keyboard.press('Escape'); await settle();
    }
    check('criterion 24: no Convert pattern entry in the Object menu', !menuEntry);
    await move(880, 850); await p.mouse.down(); await p.mouse.up(); await settle(600);
    await tool('Select');
    await move(450, 300); await p.mouse.click(at(450, 300).x, at(450, 300).y, { button: 'right' }); await settle();
    const ctx = await p.getByRole('menuitem', { name: /Convert.*pattern|pattern.*path/i }).count();
    check('criterion 24: no Convert pattern entry in the context menu', ctx === 0, `${ctx} pattern items`);
    await p.keyboard.press('Escape'); await settle();
    await move(250, 400); await p.mouse.down(); await p.mouse.up(); await settle(600);
    const convertBtn = p.getByRole('button', { name: /Convert.*path/i });
    check('criterion 24: the panel has a Convert to paths button', (await convertBtn.count()) > 0);
    if (await convertBtn.count()) {
      await convertBtn.first().click(); await settle(600);
      check('criterion 24: the panel button converts and selects the new group', /Group of \d+ objects/.test(await p.locator('body').innerText()));
      await shot('05-after-convert');
      await p.keyboard.press('Control+z'); await settle(600);
    }

    // Criterion 20: copy and paste keep the seed.
    await settle(600);
    await move(250, 400); await p.mouse.down(); await p.mouse.up(); await settle();
    await tiles.getByRole('radio', { name: 'Leaf forest', exact: true }).click(); await settle();
    const seedBox = p.locator('[data-pattern-block]').getByRole('textbox', { name: /seed/i });
    const seed = (await seedBox.count()) ? await seedBox.inputValue() : null;
    await p.keyboard.press('Control+c'); await settle();
    await p.keyboard.press('Control+v'); await settle(400);
    const pastedSeed = (await seedBox.count()) ? await seedBox.inputValue() : null;
    check('criterion 20: the pasted copy has the same seed', seed !== null && seed === pastedSeed, `${seed} / ${pastedSeed}`);
    await shot('06-pasted');

    // Criterion 3: None hides the pattern; undo brings it back.
    console.log('fill kinds:', await kinds.getByRole('radio').evaluateAll((els) => els.map((e) => e.getAttribute('aria-label') || e.textContent)));
    await kinds.getByRole('radio').first().click(); await settle();
    check('criterion 3: None hides the block', (await block.count()) === 0);
    await move(1100, 850); await p.mouse.down(); await p.mouse.up(); await settle();
    await p.keyboard.press('Control+z'); await settle(300);
    await p.keyboard.press('Control+a'); await settle();
    check('criterion 3: undo brings the pattern back', (await block.count()) === 1);
  } catch (e) {
    await browser.contexts()[0]?.pages()[0]?.screenshot({ path: path.join(shots, 'zz-failure.png') }).catch(() => {});
    check('journey ran to the end', false, String(e).split('\n').slice(0, 4).join(' / '));
  } finally {
    await browser.close();
  }
  check('no page errors', errors.length === 0, errors.join(' | '));
  const failed = results.filter((r) => !r.ok).length;
  console.log(`pattern-fill-tester: ${results.length - failed} passed, ${failed} failed`);
  process.exit(failed ? 1 : 0);
})().catch((error) => { console.error(error); process.exit(1); });

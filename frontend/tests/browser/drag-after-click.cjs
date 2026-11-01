// Browser check of a drag right after a selecting click (fix/drag-after-click, found in the 0070
// UI review): a press that moves beyond the drag threshold within the double-click interval is a
// drag, never a double-click. A real double-click still enters node editing on a path and text
// editing on a text. Starts its own dev server unless CURVYO_BROWSER_URL is set. Screenshots and
// saved projects: target/drag-after-click-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { execFileSync, spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/drag-after-click-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5181;

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    const up = await fetch(url).then(() => true, () => false);
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

const shapes = (nodes) => nodes.flatMap((n) => (n.shape === 'layer' || n.shape === 'group' ? shapes(n.objects || []) : [n]));

(async () => {
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(PORT), '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    url = `http://127.0.0.1:${PORT}/`;
    process.on('exit', () => { try { process.kill(-vite.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const p = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
    const errors = [];
    p.on('pageerror', (e) => errors.push(e.message));
    await p.goto(url, { waitUntil: 'networkidle' });
    const canvas = await p.locator('canvas').first().boundingBox();
    const at = (x, y) => ({ x: canvas.x + x, y: canvas.y + y });
    const move = async (x, y, steps = 4) => { const q = at(x, y); await p.mouse.move(q.x, q.y, { steps }); };
    const settle = () => p.waitForTimeout(150);
    const tool = async (name) => { await p.getByRole('button', { name: new RegExp(`^${name} tool`) }).click(); await settle(); };
    const pressed = (name) => p.getByRole('button', { name: new RegExp(`^${name} tool`) }).getAttribute('aria-pressed');
    const deselect = async () => { await move(1100, 850); await p.keyboard.press('Escape'); await p.keyboard.press('Escape'); await settle(); };
    let saves = 0;
    const boxes = async () => {
      await p.getByRole('tab', { name: /^File/ }).click();
      const download = p.waitForEvent('download');
      await p.getByRole('button', { name: 'Save', exact: true }).click();
      const file = path.join(out, `state-${++saves}.curvyo`);
      await (await download).saveAs(file);
      const json = execFileSync('python3', ['-c', 'import zipfile,sys; print(zipfile.ZipFile(sys.argv[1]).read("document.json").decode())', file], { encoding: 'utf8' });
      return shapes(JSON.parse(json).objects).map((o) => ({ x: o.bounds.origin.x, y: o.bounds.origin.y }));
    };

    // A rectangle; nothing selected.
    await tool('Rectangle');
    await move(400, 300); await p.mouse.down(); await move(550, 400); await p.mouse.up(); await settle();
    await tool('Select');
    await deselect();
    const [before] = await boxes();

    // A click on its edge selects it; a press at once at the same place, dragged 100 px right,
    // moves it (no wait: the press lands within the double-click interval).
    // The click and the next press are sent in one burst: awaited one by one, a slow software
    // renderer can stretch them past the double-click interval and hide the bug.
    await p.evaluate(() => { window.__downs = []; window.addEventListener('pointerdown', (e) => window.__downs.push(e.timeStamp), true); });
    await move(400, 325);
    const cdp = await p.context().newCDPSession(p);
    const q = at(400, 325);
    const mouse = (type, buttons) => cdp.send('Input.dispatchMouseEvent', { type, x: q.x, y: q.y, button: 'left', buttons, clickCount: 1 });
    await Promise.all([mouse('mousePressed', 1), mouse('mouseReleased', 0), mouse('mousePressed', 1)]);
    const [d1, d2] = await p.evaluate(() => window.__downs);
    assert(d2 - d1 < 400, `the second press came ${d2 - d1} ms after the first, within the interval`);
    for (let i = 1; i <= 6; i += 1) {
      await cdp.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: q.x + (100 * i) / 6, y: q.y, button: 'left', buttons: 1 });
    }
    await cdp.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: q.x + 100, y: q.y, button: 'left', buttons: 0, clickCount: 1 });
    await settle();
    await p.screenshot({ path: path.join(out, '01-drag-after-click.png') });
    const [after] = await boxes();
    assert(after.x - before.x > 10, `the drag after the click moved the rectangle: ${before.x} -> ${after.x} mm`);
    assert(Math.abs(after.y - before.y) < 0.5, `only sideways: ${before.y} -> ${after.y} mm`);
    console.log(`PASS drag right after a selecting click moves the rectangle ${(after.x - before.x).toFixed(2)} mm`);

    // A real double-click on a path still enters node editing.
    await tool('Pen');
    await move(400, 600); await p.mouse.down(); await p.mouse.up(); await settle();
    await move(600, 600); await p.mouse.down(); await p.mouse.up(); await settle();
    await p.keyboard.press('Enter'); await settle();
    await tool('Select');
    await deselect();
    const seg = at(500, 600);
    await p.mouse.dblclick(seg.x, seg.y); await settle();
    assert.equal(await pressed('Node'), 'true', 'a double-click on a path enters node editing');
    console.log('PASS double-click on a path enters node editing');

    // A real double-click on a text still enters text editing.
    await tool('Text');
    await move(800, 600); await p.mouse.down(); await p.mouse.up(); await settle();
    await p.keyboard.type('Hello'); await settle();
    await p.keyboard.press('Escape'); await settle();
    await tool('Select');
    await deselect();
    // The press at (800, 600) is the left end of the baseline; (860, 585) is inside the word.
    const word = at(860, 585);
    await p.mouse.dblclick(word.x, word.y); await settle();
    await p.screenshot({ path: path.join(out, '02-text-double-click.png') });
    assert.equal(await pressed('Text'), 'true', 'a double-click on a text enters text editing');
    console.log('PASS double-click on a text enters text editing');

    assert.deepEqual(errors, []);
  } finally {
    await browser.close();
  }
  process.exit(0);
})().catch((e) => { console.error(e); process.exit(1); });

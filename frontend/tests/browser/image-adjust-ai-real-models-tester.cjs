// Tester browser run of `specs/0135-image-optimisation` slice 3 with the real pinned models and of
// `specs/0143-ai-tile-pool`, written from the specs: the model host is a Playwright route that
// serves the converted files from CURVYO_MODELS_DIR (layout `<name>-<version>/<sha256>.onnx`) in
// place of models.curvyo.org. Upscale and AI denoise run end to end on the real models (download
// once, verified by the real pin, progress, a stored result); the AI steps use a pool of extra
// workers that are gone after the step (0143 criteria 3, 6); Cancel ends every pool worker and
// returns the row to idle within 1 s (criterion 8); the page is not cross-origin isolated
// (criterion 11). Then the job file is saved, the document reopened in a fresh browser profile
// without any model (the host refuses every request), and the job's burn lines are byte-identical
// (0135 criterion 48). Skips without CURVYO_MODELS_DIR. Starts its own dev server unless
// CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium; build the wasm bindings first.
// Screenshots: target/image-adjust-ai-real-models-tester-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/image-adjust-ai-real-models-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5197;
const MODELS = process.env.CURVYO_MODELS_DIR;

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    const up = await fetch(url).then(() => true, () => false);
    if (up) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

const t0 = Date.now();
const log = (s) => console.log(`[${((Date.now() - t0) / 1000).toFixed(1)} s] ${s}`);

async function setup(context, url, served) {
  await context.route('https://models.curvyo.org/**', async (route) => {
    const u = new URL(route.request().url());
    served.push(u.pathname);
    const m = /^\/([^/]+)\/([^/]+)\/([0-9a-f]{64})\.onnx$/.exec(u.pathname);
    const file = served.refuse || !m ? null : path.join(MODELS, `${m[1]}-${m[2]}`, `${m[3]}.onnx`);
    if (!file || !fs.existsSync(file)) {
      await route.fulfill({ status: 404, headers: { 'Access-Control-Allow-Origin': '*' }, body: '' });
      return;
    }
    await route.fulfill({ status: 200, headers: { 'Access-Control-Allow-Origin': '*', 'Content-Type': 'application/octet-stream' }, body: fs.readFileSync(file) });
  });
  const p = await context.newPage();
  const errors = [];
  p.on('pageerror', (e) => { errors.push(e.message); console.log('PAGEERROR', e.message); });
  p.setDefaultTimeout(60000);
  await p.goto(url, { waitUntil: 'networkidle' });
  return { p, errors };
}

(async () => {
  if (!MODELS) {
    console.log('image-adjust-ai-real-models-tester: SKIP (set CURVYO_MODELS_DIR)');
    return;
  }
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const vite = spawn('npx', ['vite', '--host', '127.0.0.1', '--port', String(PORT), '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    url = `http://127.0.0.1:${PORT}/`;
    process.on('exit', () => { try { process.kill(-vite.pid, 'SIGKILL'); } catch { /* gone */ } });
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CURVYO_E2E_BROWSER || undefined, args: ['--no-sandbox', '--enable-unsafe-webgpu', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  try {
    const served = [];
    const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
    const { p, errors } = await setup(context, url, served);
    const settle = () => p.waitForTimeout(500);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const group = () => p.locator('[data-adjustments]');
    assert.equal(await p.evaluate(() => globalThis.crossOriginIsolated), false, '0143 criterion 11: not cross-origin isolated');
    const cores = await p.evaluate(() => navigator.hardwareConcurrency);
    log(`hardwareConcurrency ${cores}`);

    // A laser page with a 480 x 300 photo (6 tiles) in an Image operation.
    await tab('Document');
    await p.getByRole('button', { name: 'Add page', exact: true }).first().click();
    await settle();
    await p.locator('[role=group][aria-label="Add page"] [role=radio]', { hasText: 'Laser' }).click();
    await p.locator('[role=group][aria-label="Add page"] button', { hasText: /^Add$/ }).click();
    await p.waitForTimeout(800);
    await p.mouse.click(600, 500);
    const canvas = await p.locator('canvas[data-state]').boundingBox();
    const data = await p.evaluateHandle(async () => {
      const c = document.createElement('canvas');
      c.width = 480; c.height = 300;
      const g = c.getContext('2d');
      const wall = g.createLinearGradient(0, 0, 480, 300);
      wall.addColorStop(0, '#4a4a4a'); wall.addColorStop(1, '#d0d0d0');
      g.fillStyle = wall; g.fillRect(0, 0, 480, 300);
      const ball = g.createRadialGradient(200, 120, 8, 240, 150, 110);
      ball.addColorStop(0, '#fff'); ball.addColorStop(1, '#202020');
      g.fillStyle = ball; g.beginPath(); g.arc(240, 150, 110, 0, 7); g.fill();
      let seed = 11;
      for (let i = 0; i < 20000; i += 1) {
        seed = (seed * 1103515245 + 12345) & 0x7fffffff;
        g.fillStyle = `rgba(${seed % 255},${(seed >> 8) % 255},${(seed >> 16) % 255},0.2)`;
        g.fillRect(seed % 480, (seed >> 9) % 300, 1, 1);
      }
      const blob = await new Promise((r) => c.toBlob(r, 'image/png'));
      const t = new DataTransfer();
      t.items.add(new File([blob], 'photo.png', { type: 'image/png' }));
      return t;
    });
    const cx = canvas.x + canvas.width / 2; const cy = canvas.y + canvas.height / 2;
    for (const type of ['dragenter', 'dragover', 'drop']) {
      await p.dispatchEvent('canvas[data-state]', type, { dataTransfer: data, clientX: cx, clientY: cy });
    }
    await p.waitForTimeout(1500);
    await p.locator('[role=group][aria-label=Operation] button').last().click(); await settle();
    await tab('Operations');
    if ((await p.locator('[data-row-body]').count()) === 0) { await p.getByRole('button', { name: /^Image 1/ }).first().click(); await settle(); }
    for (const [name, text] of [['Power, percent', '40'], ['Speed, millimetres per minute', '3000']]) {
      await p.getByLabel(name, { exact: true }).fill(text); await p.keyboard.press('Enter'); await settle();
    }
    await p.locator('canvas[data-state]').click({ position: { x: cx - canvas.x, y: cy - canvas.y } }); await settle();
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    const baseWorkers = p.workers().length;
    log(`workers before any AI step: ${baseWorkers}`);

    /** Run, consent, Download, then wait for the stored result; returns what was seen. */
    const runStep = async (kind, addName, { cancelFirst = false } = {}) => {
      await group().getByRole('button', { name: addName, exact: true }).first().click();
      await p.waitForTimeout(1000);
      const body = p.locator(`[data-ai-body="${kind}"]`).last();
      await body.waitFor({ timeout: 5000 });
      assert.match(await body.innerText(), /Model not downloaded yet/);
      await body.getByRole('button', { name: 'Run', exact: true }).click();
      await settle();
      assert.match(await body.innerText(), /once from models\.curvyo\.org/);
      const before = served.length;
      await body.getByRole('button', { name: 'Download', exact: true }).click();
      const seen = { lines: new Set(), maxWorkers: 0, oneCore: false };
      const start = Date.now();
      if (cancelFirst) {
        // Wait for the Run to show progress with its pool, then Cancel.
        while (!/Running/.test(await body.innerText())) {
          if (Date.now() - start > 120000) throw new Error(`${kind}: no Running line`);
          await p.waitForTimeout(100);
        }
        await p.waitForTimeout(1500);
        const during = p.workers().length;
        log(`${kind}: ${during} workers while running`);
        assert.ok(during > baseWorkers + 1 || cores <= 2, `${kind}: a pool of more than one AI worker (${during} vs ${baseWorkers})`);
        const c0 = Date.now();
        await body.getByRole('button', { name: 'Cancel', exact: true }).click();
        while (/Running/.test(await body.innerText())) {
          if (Date.now() - c0 > 3000) break;
          await p.waitForTimeout(20);
        }
        const idle = Date.now() - c0;
        log(`${kind}: Cancel to idle row ${idle} ms`);
        assert.ok(idle <= 1000, `0143 criterion 8: the row idle within 1 s (${idle} ms)`);
        // Criterion 9: no worker stays behind (polled for up to 5 s: the browser reports a
        // terminated worker's end asynchronously).
        let after = p.workers().length;
        for (let i = 0; i < 25 && after > baseWorkers; i += 1) {
          await p.waitForTimeout(200);
          after = p.workers().length;
        }
        log(`${kind}: worker URLs after Cancel: ${p.workers().map((w) => w.url().split('/').pop()).join(', ')}`);
        log(`${kind}: ${after} workers after Cancel`);
        assert.ok(after <= baseWorkers, `0143 criteria 8, 9: no AI worker after Cancel (${after} vs ${baseWorkers})`);
        assert.doesNotMatch(await body.innerText(), /Stored result/, 'no partial result stored');
        await shot(`${kind}-cancelled`);
        await body.getByRole('button', { name: 'Run', exact: true }).click();
      }
      for (;;) {
        const text = await body.innerText();
        const running = /Running[^\n]*/.exec(text);
        if (running) seen.lines.add(running[0]);
        if (/one core/.test(text)) seen.oneCore = true;
        seen.maxWorkers = Math.max(seen.maxWorkers, p.workers().length);
        if (/Stored result \d/.test(text)) break;
        if (/did not match|not reachable|failed|error/i.test(text)) throw new Error(`${kind}: ${text}`);
        if (Date.now() - start > 300000) throw new Error(`${kind}: no stored result after 300 s: ${text}`);
        await p.waitForTimeout(250);
      }
      const secs = (Date.now() - start) / 1000;
      log(`${kind}: stored after ${secs.toFixed(1)} s; peak ${seen.maxWorkers} workers; ${seen.lines.size} progress lines; one core ${seen.oneCore}`);
      assert.equal(served.length - before, 1, `${kind}: one model request: ${served.slice(before)}`);
      await p.waitForTimeout(1000);
      assert.ok(p.workers().length <= baseWorkers + 1, `0143 criterion 6: only worker 0 after the step (${p.workers().length})`);
      await shot(`${kind}-stored`);
      return { secs, ...seen };
    };

    const up = await runStep('upscale', 'Add Upscale');
    assert.ok(up.lines.size >= 1, 'a progress line');
    if (cores > 2) assert.equal(up.oneCore, false, 'more than two cores: no one-core line');
    if (cores > 2) assert.ok(up.maxWorkers > baseWorkers + 1, `0143 criterion 3: a pool (${up.maxWorkers})`);
    const dn = await runStep('denoise', 'Add AI denoise', { cancelFirst: true });
    assert.ok(dn.lines.size >= 1);

    // The job's burn lines and the saved document.
    const job = async (page) => {
      await page.locator('[role=tab][aria-label^="Operations"]').first().click(); await page.waitForTimeout(800);
      await page.screenshot({ path: path.join(out, `job-${Date.now()}.png`) });
      const download = page.waitForEvent('download', { timeout: 120000 });
      await page.locator('[data-export-job]').click();
      const file = await download;
      const target = path.join(out, `job-${Date.now()}-${file.suggestedFilename()}`);
      await file.saveAs(target);
      return fs.readFileSync(target, 'utf8');
    };
    const first = await job(p);
    const burns = first.split('\n').filter((l) => /^G1 .*S\d+/.test(l) && !/S0\b/.test(l));
    assert.ok(burns.length > 50, `burned runs: ${burns.length}`);
    await p.locator('[role=tab][aria-label^="File"]').first().click(); await settle();
    const saving = p.waitForEvent('download');
    await p.getByRole('button', { name: 'Save', exact: true }).click();
    const saved = path.join(out, 'with-ai-results.curvyo');
    await (await saving).saveAs(saved);
    assert.deepEqual(errors, [], 'no page errors in the first session');
    await context.close();

    // A fresh profile: no model on this device, the host refuses everything.
    const served2 = [];
    served2.refuse = true;
    const context2 = await browser.newContext({ viewport: { width: 1440, height: 1000 }, acceptDownloads: true });
    const second = await setup(context2, url, served2);
    await second.p.locator('[role=tab][aria-label^="File"]').first().click(); await second.p.waitForTimeout(500);
    const chooser = second.p.waitForEvent('filechooser');
    await second.p.getByRole('button', { name: 'Open…', exact: true }).click();
    await (await chooser).setFiles(saved);
    await second.p.waitForTimeout(2000);
    // The file opens on its first page; the picture is on the laser page.
    await second.p.locator('[role=tab][aria-label^="Document"]').first().click(); await second.p.waitForTimeout(500);
    await second.p.locator('[role=tabpanel]').getByText('Laser 1', { exact: true }).first().click(); await second.p.waitForTimeout(1500);
    await second.p.screenshot({ path: path.join(out, 'reopened.png') });
    const reopened = await job(second.p);
    const burns2 = reopened.split('\n').filter((l) => /^G1 .*S\d+/.test(l) && !/S0\b/.test(l));
    assert.equal(burns2.length, burns.length, 'the same number of burn lines');
    assert.ok(burns2.every((l, i) => l === burns[i]), '0135 criterion 48: the burn lines are identical after reopen without the model');
    assert.deepEqual([...served2], [], 'reopening and exporting requests no model');
    assert.deepEqual(second.errors, [], 'no page errors after reopen');
    log(`Upscale ${up.secs.toFixed(1)} s, AI denoise ${dn.secs.toFixed(1)} s (after Cancel and a second Run), ${burns.length} burn lines equal after reopen`);
    console.log('image-adjust-ai-real-models-tester: PASS');
  } finally {
    await browser.close();
  }
})().then(() => process.exit(0), (e) => { console.error(e); process.exit(1); });

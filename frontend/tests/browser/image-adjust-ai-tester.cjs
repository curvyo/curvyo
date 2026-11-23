// Tester browser smoke of `specs/0135-image-optimisation` slice 3, written from the spec: starting
// the app, adding a picture and a non-AI step make no request to the model host (criterion 63);
// an AI step shows "Model not downloaded yet" (criterion 65); Run shows the consent line and
// still requests nothing; Download makes exactly one GET of a fixed URL on models.curvyo.org with
// no query, body, cookie or referrer (served here by a fake host); the pin (a 1 KiB file is never the
// pinned one) refuses the file with the checksum line (criterion 64). Starts its own dev server unless
// CURVYO_BROWSER_URL is set; CURVYO_E2E_BROWSER names a Chromium; build the wasm bindings first.
// Screenshots: target/image-adjust-ai-tester-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/image-adjust-ai-tester-browser');
fs.mkdirSync(out, { recursive: true });
const PORT = 5196;

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
    const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    const p = await context.newPage();
    const modelRequests = [];
    const external = [];
    p.on('request', (r) => {
      const u = new URL(r.url());
      if (u.hostname === 'models.curvyo.org') modelRequests.push(r);
      else if (!['127.0.0.1', 'localhost'].includes(u.hostname) && !u.protocol.startsWith('data') && !u.protocol.startsWith('blob')) external.push(r.url());
    });
    // The fake host: 1 KiB of bytes that cannot match any pin.
    await context.route('https://models.curvyo.org/**', (route) => route.fulfill({
      status: 200,
      headers: { 'Access-Control-Allow-Origin': '*', 'Content-Type': 'application/octet-stream' },
      body: Buffer.alloc(1024, 7),
    }));
    const errors = [];
    p.on('pageerror', (e) => { errors.push(e.message); console.log('PAGEERROR', e.message); });
    p.setDefaultTimeout(30000);
    await p.goto(url, { waitUntil: 'networkidle' });
    const settle = () => p.waitForTimeout(500);
    const shot = (name) => p.screenshot({ path: path.join(out, `${name}.png`) });
    const tab = async (name) => { await p.locator(`[role=tab][aria-label^="${name}"]`).first().click(); await settle(); };
    const group = () => p.locator('[data-adjustments]');
    assert.equal(modelRequests.length, 0, 'no model request at start');

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
      c.width = 200; c.height = 120;
      const g = c.getContext('2d');
      g.fillStyle = '#e0e0e0'; g.fillRect(0, 0, 200, 120);
      g.fillStyle = '#202020'; g.beginPath(); g.arc(100, 60, 40, 0, 7); g.fill();
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
    await tab('Style');
    await group().waitFor({ timeout: 5000 });
    // A non-AI step: still no model request.
    await group().getByRole('button', { name: 'Add Levels', exact: true }).first().click();
    await settle();
    assert.equal(modelRequests.length, 0, 'no model request for a non-AI step');

    // AI denoise: the not-downloaded line, no request.
    await group().getByRole('button', { name: /AI denoise/ }).first().click();
    await p.waitForTimeout(1000);
    const body = p.locator('[data-ai-body="denoise"]');
    await body.waitFor({ timeout: 5000 });
    await shot('1-ai-step');
    assert.match(await body.innerText(), /Model not downloaded yet \(3\.3 MB\)\./);
    assert.equal(modelRequests.length, 0, 'showing the AI step requests nothing');

    // Run: the consent line first, nothing fetched.
    await body.getByRole('button', { name: 'Run', exact: true }).click();
    await settle();
    await shot('2-consent');
    const consent = await body.innerText();
    assert.match(consent, /Downloads the AI denoise model \(ffdnet-color, 3\.3 MB\) once from models\.curvyo\.org\. Your picture does not leave this computer\./);
    assert.equal(modelRequests.length, 0, 'consent before any download');

    // Download: one GET, fixed URL, then the pin refuses it (not the pinned file).
    await body.getByRole('button', { name: 'Download', exact: true }).click();
    await p.waitForTimeout(2500);
    await shot('3-checksum');
    assert.equal(modelRequests.length, 1, `one model request: ${modelRequests.map((r) => r.url())}`);
    const r = modelRequests[0];
    const u = new URL(r.url());
    assert.equal(r.method(), 'GET');
    assert.equal(u.search, '', 'no query');
    assert.match(u.pathname, /^\/ffdnet-color\/1\/[0-9a-f]{64}\.onnx$/);
    assert.equal(r.postData(), null, 'no body');
    const headers = await r.allHeaders();
    assert.equal(headers.cookie, undefined, 'no cookie');
    assert.equal(headers.referer, undefined, 'no referrer');
    assert.equal(headers.authorization, undefined);
    assert.match(await body.innerText(), /The downloaded model did not match its checksum and was removed\./);
    assert.match(await body.innerText(), /Try again/);
    assert.deepEqual(external, [], 'no other external request');
    assert.deepEqual(errors, [], 'no page errors');
    console.log('image-adjust-ai-tester: PASS');
  } finally {
    await browser.close();
  }
})().catch((e) => { console.error(e); process.exit(1); });

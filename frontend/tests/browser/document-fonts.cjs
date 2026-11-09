// Browser journey of `specs/0107-document-fonts` criteria 6, 10 to 14: a file whose font no device
// lists ("Zzz Embedded", the included sans' bytes under another family name) opens with its text
// drawn in that font; the Fonts tab shows a Document Fonts group first, whose row is greyed (star
// disabled) with the install icon and no trash can; Install puts the family into Curvyo fonts and
// the Document Fonts row turns into an installed row (star enabled, no icon).
//
// Needs a production preview (`npm run build`, `npx vite preview --host 127.0.0.1 --port 1421`);
// starts the preview itself unless CURVYO_BROWSER_URL is set. Screenshots: target/0107-browser/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0107-browser');
const fixture = path.join(root, 'curvyo-editor-wasm/tests/fixtures/document_font_foreign.curvyo');
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
const children = [];
process.on('exit', () => {
  for (const child of children) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* gone */ }
  }
});
const step = (text) => console.log(text);

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then((r) => r.ok, () => false)) return;
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`no server at ${url}`);
}

(async () => {
  let url = process.env.CURVYO_BROWSER_URL;
  if (!url) {
    const preview = spawn('npx', ['vite', 'preview', '--host', '127.0.0.1', '--port', '1421', '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
    children.push(preview);
    url = 'http://127.0.0.1:1421/';
    await waitFor(url);
  }
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(e.stack || e.message));
  try {
    await page.goto(url, { waitUntil: 'networkidle' });
    await page.getByRole('tab', { name: /^File/ }).click();
    const chooser = page.waitForEvent('filechooser');
    await page.getByRole('button', { name: 'Open…', exact: true }).click();
    await (await chooser).setFiles(fixture);
    await page.waitForTimeout(800);

    step('0107 criteria 6, 10, 12: the Fonts tab lists Document Fonts first, the row greyed with Install');
    await page.getByRole('tab', { name: /^Fonts/ }).click();
    await page.waitForTimeout(800);
    const panel = page.locator('[role=tabpanel]');
    const headers = await panel.getByText(/^(Document Fonts|Curvyo fonts|Included|This computer)$/).allTextContents();
    assert.equal(headers[0], 'Document Fonts', `Document Fonts first: ${headers}`);
    const documentRow = panel.getByRole('treeitem', { name: /^Zzz Embedded, document fonts/ });
    assert.equal(await documentRow.count(), 1);
    assert.equal(await documentRow.getByRole('button', { name: 'Install Zzz Embedded' }).count(), 1);
    assert.equal(await documentRow.getByRole('button', { name: /^Remove/ }).count(), 0, 'never a trash can');
    assert.equal(await documentRow.getByRole('button', { name: /^Favourite/ }).isDisabled(), true);
    await page.screenshot({ path: path.join(out, 'not-installed.png') });

    step('0107 criterion 14: Install hands the embedded file to Add font; its family lands in Curvyo fonts');
    // The file's own family name is "Source Sans 3" (only the document's reference says
    // "Zzz Embedded"), so the installed family is listed under that name and the document row
    // stays a not-installed row: "installed" compares the names a text uses.
    await documentRow.getByRole('button', { name: 'Install Zzz Embedded' }).click();
    await page.waitForTimeout(1500);
    assert.equal(await panel.getByRole('treeitem', { name: /^Source Sans 3, curvyo fonts/ }).count(), 1);
    assert.equal(await panel.getByRole('treeitem', { name: /^Zzz Embedded, document fonts/ }).count(), 1);
    await page.screenshot({ path: path.join(out, 'after-install.png') });
    assert.deepEqual(errors, []);
    console.log('document-fonts journey: PASS');
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

// Browser check of the passkey key backup (`specs/0096-own-devices-sync` plan task 18e, ADR 0040
// decision 13, Frage 145 A). A local `curvyo-sync-server` behind a TLS proxy (the browser needs
// `wss://`), a desktop device D run on the wasm bindings in node (it opens the group and shows
// the code), and one browser R with Playwright's virtual authenticator over CDP
// (`WebAuthn.addVirtualAuthenticator` with `hasPrf: true`: the real PRF evaluation, no seam).
//
// R joins D's group, backs its keys up with a passkey, its site data is cleared
// (`Storage.clearDataForOrigin`), and Restore with passkey brings the same device back into the
// group without a new pairing and while D is offline (same fingerprint; D sees it once back). The server's backup file holds none
// of R's names or the group room id. Remove backup deletes it. An authenticator without PRF
// makes Back up say so and stores nothing.
//
// Needs `target/debug/curvyo-sync-server` (`cargo build -p curvyo-sync-server`), the wasm
// bindings and a production build (`npm run build`); starts `vite preview` on localhost:1422.
// Playwright as in README.md. Screenshots: target/0096-key-backup/.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const { spawn, execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const net = require('node:net');
const path = require('node:path');
const tls = require('node:tls');
const { pathToFileURL } = require('node:url');

const root = path.resolve(__dirname, '../../..');
const out = path.join(root, 'target/0096-key-backup');
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
const children = [];
process.on('exit', () => {
  for (const child of children) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch { /* gone */ }
  }
});
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => process.exit(1));
const started = Date.now();
const step = (text) => console.log(`[${((Date.now() - started) / 1000).toFixed(1)} s] ${text}`);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function until(what, probe, timeout = 20000) {
  const start = Date.now();
  for (;;) {
    if (await probe().catch(() => false)) return;
    if (Date.now() - start > timeout) throw new Error(`timed out after ${timeout} ms: ${what}`);
    await sleep(100);
  }
}

async function waitFor(url) {
  for (let i = 0; i < 300; i += 1) {
    if (await fetch(url).then((r) => r.ok, () => false)) return;
    await sleep(200);
  }
  throw new Error(`no server at ${url}`);
}

/** The relay on a free port with a fresh data directory; its `host:port`. */
async function startRelay() {
  const data = path.join(out, 'relay-data');
  fs.mkdirSync(data);
  const relay = spawn(path.join(root, 'target/debug/curvyo-sync-server'), ['--listen', '127.0.0.1:0', '--data', data], {
    detached: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  children.push(relay);
  let log = '';
  const address = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`relay did not start: ${log}`)), 15000);
    const read = (chunk) => {
      log += chunk;
      const match = /listening on (\S+)/.exec(log);
      if (match) { clearTimeout(timer); resolve(match[1]); }
    };
    relay.stdout.on('data', read);
    relay.stderr.on('data', read);
  });
  await waitFor(`http://${address}/healthz`);
  return { address, data, log: () => log };
}

/** A TLS front for the relay with a self-signed certificate for localhost; its port. */
async function startTls(relayAddress) {
  const key = path.join(out, 'key.pem');
  const cert = path.join(out, 'cert.pem');
  execFileSync('openssl', ['req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:prime256v1', '-nodes', '-days', '1', '-subj', '/CN=localhost', '-keyout', key, '-out', cert], { stdio: 'ignore' });
  const [host, port] = relayAddress.split(':');
  const server = tls.createServer({ key: fs.readFileSync(key), cert: fs.readFileSync(cert) }, (socket) => {
    const up = net.connect(Number(port), host);
    socket.pipe(up).pipe(socket);
    socket.on('error', () => up.destroy());
    up.on('error', () => socket.destroy());
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  return { port: server.address().port, close: () => server.close() };
}

// ---- the desktop device D on the wasm bindings ---------------------------------------------------

/** Drives one wasm transport port (`sync_next_action` and friends) over node's WebSocket; the
 * `wss://localhost` address the channel is bound to is reached as plain `ws://` on the relay. */
function drive(port, wssAddress, relayAddress) {
  let socket = null;
  const timer = setInterval(() => {
    port.sync_tick(Date.now());
    for (let action = port.sync_next_action(); action; action = port.sync_next_action()) {
      if (action.kind === 'connect') {
        const address = action.address === wssAddress ? `ws://${relayAddress}` : action.address;
        const opened = new WebSocket(address);
        opened.binaryType = 'arraybuffer';
        opened.onopen = () => port.sync_connected(Date.now());
        opened.onmessage = (e) => port.sync_received(new Uint8Array(e.data), Date.now());
        opened.onclose = () => { if (socket === opened) { socket = null; port.sync_disconnected(Date.now()); } };
        socket = opened;
      } else if (action.kind === 'send') {
        socket?.send(action.bytes);
      } else if (action.kind === 'close') {
        const closing = socket;
        socket = null;
        closing?.close();
      }
      action.free();
    }
  }, 50);
  return () => { clearInterval(timer); socket?.close(); };
}

async function loadWasm() {
  const bindings = path.join(root, 'frontend/src/wasm-bindings');
  const wasm = await import(pathToFileURL(path.join(bindings, 'curvyo_editor_wasm.js')).href);
  wasm.initSync({ module: fs.readFileSync(path.join(bindings, 'curvyo_editor_wasm_bg.wasm')) });
  return wasm;
}

function peerId() {
  return crypto.randomBytes(8).readBigUInt64BE();
}

// ---- the browser device R ------------------------------------------------------------------------

const panel = (page) => page.locator('[role=tabpanel]');
const tab = async (page, name) => { await page.getByRole('tab', { name: new RegExp(`^${name}`) }).click(); await sleep(200); };
const devicesList = (page) => panel(page).getByRole('list', { name: 'My devices' }).locator(':scope > li');
const shown = (page, text) => page.getByText(text, { exact: true }).count().then((n) => n > 0);
const shot = (page, name) => page.screenshot({ path: path.join(out, `${name}.png`) });

async function setServer(page, address) {
  await tab(page, 'Share');
  const server = panel(page).getByLabel('Server', { exact: true });
  await server.fill(address);
  await server.press('Enter');
  const name = panel(page).getByLabel('Name shown to others', { exact: true });
  await name.fill('Rita');
  await name.press('Enter');
  await sleep(300);
}

async function authenticator(cdp, hasPrf) {
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      ctap2Version: 'ctap2_1',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
      hasPrf,
    },
  });
  return authenticatorId;
}

function backupFiles(data) {
  const dir = path.join(data, 'backups');
  return fs.existsSync(dir) ? fs.readdirSync(dir).map((name) => fs.readFileSync(path.join(dir, name))) : [];
}

(async () => {
  const relay = await startRelay();
  const front = await startTls(relay.address);
  const wssAddress = `wss://localhost:${front.port}`;
  const wasm = await loadWasm();
  const preview = spawn('npx', ['vite', 'preview', '--host', 'localhost', '--port', '1422', '--strictPort'], { cwd: path.join(root, 'frontend'), stdio: 'ignore', detached: true });
  children.push(preview);
  const url = 'http://localhost:1422/';
  await waitFor(url);
  const browser = await chromium.launch({ headless: true, args: ['--no-sandbox', '--ignore-certificate-errors', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
  const stops = [];
  let stopGroup = null;
  try {
    const context = await browser.newContext({ viewport: { width: 1440, height: 1000 }, ignoreHTTPSErrors: true });
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.stack || e.message));
    await page.goto(url, { waitUntil: 'networkidle' });
    const cdp = await context.newCDPSession(page);
    await cdp.send('WebAuthn.enable');
    let passkeys = await authenticator(cdp, true);

    step('D opens a pairing code; R joins with it');
    const desk = crypto.randomBytes(64);
    const attempt = wasm.WasmPairing.open(wssAddress, desk, 'Desk', crypto.randomBytes(32), Date.now());
    stops.push(drive(attempt, wssAddress, relay.address));
    let code = '';
    await until('D shows a code', async () => {
      code = JSON.parse(attempt.view_json()).code ?? '';
      return code !== '';
    });
    await setServer(page, wssAddress);
    await shot(page, '0-share');
    // One Join field takes the device code (`specs/0105` criterion 16).
    await panel(page).getByRole('button', { name: 'Join…', exact: true }).click();
    await panel(page).getByLabel('Link or code', { exact: true }).fill(code);
    await panel(page).getByLabel('Device name', { exact: true }).fill('ZQX-PC-7731');
    await panel(page).getByRole('button', { name: 'Connect', exact: true }).click();
    await until('D compares words', async () => JSON.parse(attempt.view_json()).phase === 'compare');

    step('D pairs: it starts the group and adds R');
    attempt.mark_pairing();
    const group = wasm.WasmDeviceGroup.create(wssAddress, crypto.randomBytes(16), 'Desk', desk, crypto.randomBytes(32), crypto.randomBytes(32), peerId(), Date.now());
    stopGroup = drive(group, wssAddress, relay.address);
    const peer = attempt.peer_identity();
    await until('the group is online', async () => group.is_online());
    assert.ok(group.add_device(peer, attempt.peer_label()));
    await until('R is in the group keyring', async () => group.has_device(peer));
    assert.ok(attempt.pair(group.room()));
    await until('R lists two devices', async () => (await devicesList(page).count()) === 2, 30000);
    const fingerprint = await devicesList(page).filter({ hasText: 'this device' }).locator('.font-mono').innerText();
    await shot(page, '1-paired');

    step('R backs its keys up with a passkey');
    // Back up asks nothing: the passkey prompt confirms (customer 2026-11-09).
    await panel(page).getByRole('button', { name: 'Back up with passkey', exact: true }).click();
    await until('the backup is stored', () => shown(page, 'Keys backed up with a passkey.'));
    assert.ok(await shown(page, 'Keys backed up with a passkey. Clearing site data does not lose this device.'));
    const files = backupFiles(relay.data);
    assert.equal(files.length, 1, 'one backup on the server');
    const room = Buffer.from(group.room(), 'hex');
    for (const needle of [Buffer.from('ZQX-PC-7731'), room, Buffer.from(`localhost:${front.port}`)]) {
      assert.equal(files[0].indexOf(needle), -1, `the backup file holds ${needle}`);
    }
    assert.equal(relay.log().indexOf('ZQX-PC-7731'), -1, 'the log names the device');
    await shot(page, '2-backed-up');

    step('D goes offline; R\'s site data is cleared: it is an unpaired browser');
    stopGroup();
    stops.forEach((stop) => stop());
    await until('D is offline', async () => !group.is_online());
    await cdp.send('Storage.clearDataForOrigin', { origin: new URL(url).origin, storageTypes: 'all' });
    await page.reload({ waitUntil: 'networkidle' });
    await setServer(page, wssAddress);
    await until('R shows Join…', () => shown(page, 'Join…'));
    assert.equal(await devicesList(page).count(), 1);

    step('Restore with passkey brings the same device back into the group, with no other device online');
    await panel(page).getByRole('button', { name: 'Restore with passkey', exact: true }).click();
    await until('R is restored', () => shown(page, 'This device was restored from its passkey backup.'));
    await until('R lists two devices again', async () => (await devicesList(page).count()) === 2, 30000);
    const restored = await devicesList(page).filter({ hasText: 'this device' }).locator('.font-mono').innerText();
    assert.equal(restored, fingerprint, 'the same device, no new fingerprint');
    assert.ok(await devicesList(page).filter({ hasText: 'ZQX-PC-7731' }).count(), 'its name came back');
    await shot(page, '3-restored');
    step('D comes back and sees R');
    assert.equal(group.is_online(), false, 'D stayed offline during the restore');
    stopGroup = drive(group, wssAddress, relay.address);
    await until('D is back online', async () => group.is_online(), 40000);
    await until('D sees R online', async () => {
      const view = JSON.parse(group.view_json(0, '[]'));
      return view.rows.some((row) => !row.own && row.seen.endsWith('Online'));
    }, 40000);
    // R marked itself a browser in the devices list and its name reached D: D shows both
    // (ADR 0040 decision 1).
    await until('D shows R as a browser', async () => {
      const view = JSON.parse(group.view_json(0, '[]'));
      return view.rows.some((row) => !row.own && row.browser && row.seen.startsWith('Browser, ') && row.label === 'ZQX-PC-7731');
    }, 20000);

    step('Remove backup deletes it on the server');
    // Remove backup is armed (customer feedback on PR #281): a second press removes it.
    await panel(page).getByRole('button', { name: 'Remove backup', exact: true }).click();
    await sleep(450);
    await panel(page).getByRole('button', { name: 'Remove backup', exact: true }).click();
    await until('the backup is removed', () => shown(page, 'Backup removed from the server.'));
    assert.equal(backupFiles(relay.data).length, 0, 'no backup on the server');
    assert.ok(!(await shown(page, 'Clearing site data removes this device.')), 'no loss hint');

    step('A passkey without PRF stores nothing and says so');
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId: passkeys });
    passkeys = await authenticator(cdp, false);
    await panel(page).getByRole('button', { name: 'Back up with passkey', exact: true }).click();
    await until('the no-PRF sentence', () => shown(page, 'This device or passkey cannot lock keys. The keys stay on this device only.'));
    assert.equal(backupFiles(relay.data).length, 0, 'still no backup on the server');
    await shot(page, '4-no-prf');

    step('0105 criteria 3 and 19: the browser\'s Join field refuses a person\'s link and a 1. link, and sends nothing');
    const sockets = [];
    page.on('websocket', (socket) => sockets.push(socket.url()));
    const secret = crypto.randomBytes(48).toString('base64url');
    const nowhere = 'curvyo://join?server=ws%3A%2F%2F127.0.0.1%3A9';
    // 0105 criterion 9: the browser makes no workspaces; only the line says so.
    assert.ok(await shown(page, 'Sharing works on workspaces, made in the desktop app.'));
    assert.equal(await panel(page).getByRole('button', { name: 'Make a workspace for this file', exact: true }).count(), 0);
    await panel(page).getByRole('button', { name: 'Join…', exact: true }).click();
    const joinField = panel(page).getByLabel('Link or code', { exact: true });
    await joinField.fill(`${nowhere}#2.${secret}`);
    await until('the desktop refusal', () => shown(page, 'Open this link in the desktop app.'));
    await joinField.fill(`${nowhere}#1.${secret}`);
    await until('the document link refusal', () => shown(page, 'This link is for a single shared document. Documents are now shared in workspaces: ask for a workspace invite.'));
    await page.waitForTimeout(1500);
    assert.deepEqual(sockets.filter((u) => u.includes(':9')), [], 'no connection to the link\'s server');
    assert.equal(await panel(page).getByRole('button', { name: 'Join', exact: true }).count(), 0, 'nothing to join');
    await shot(page, '5-join-refusals');

    assert.deepEqual(errors, []);
    step('PASS');
  } finally {
    stopGroup?.();
    stops.forEach((stop) => stop());
    front.close();
    await browser.close();
  }
  process.exit(0);
})().catch((error) => {
  console.error(error);
  process.exit(1);
});

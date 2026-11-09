// The fake desktop host of the workspace journeys (`workspace.cjs`, `membership.cjs`): the
// workspace commands of `curvyo-app` on real folders, closely enough for the page, and the page
// side that forwards Tauri's `invoke` to it. Their Rust side is covered by `curvyo-storage-io`
// and `curvyo-app` tests, not here.
const { execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

// ---- the fake desktop host ----------------------------------------------------------------------

/** Zip reads the host does without opening a document (`curvyo-storage-io`'s probe and sharing
 * members): `probe` prints the thumbnail as hex or an error code, `members` the share.json text
 * and whether a keyring is there, `json` the document's objects. */
const ZIP_PY = `
import json, sys, zipfile
op, path = sys.argv[1], sys.argv[2]
try:
    z = zipfile.ZipFile(path)
except Exception:
    print(json.dumps({"error": "not-curvyo"})); sys.exit(0)
names = z.namelist()
if op == "probe":
    if "manifest.json" not in names:
        print(json.dumps({"error": "damaged"}))
    else:
        print(json.dumps({"thumbnail": z.read("thumbnail.png").hex() if "thumbnail.png" in names else ""}))
elif op == "members":
    share = z.read("share.json").decode() if "share.json" in names else None
    print(json.dumps({"share": share, "keyring": "keyring.log" in names}))
elif op == "json":
    print(z.read("document.json").decode())
`;
const zip = (op, file) => JSON.parse(execFileSync('python3', ['-c', ZIP_PY, op, file], { encoding: 'utf8' }));

const MARKER = '.curvyo-workspace';
const hex = (bytes) => Buffer.from(bytes).toString('hex');

/** One device's host under `out`: its workspace root, project path, keys and folders on disk. */
function makeHost(out, label) {
  const home = path.join(out, label);
  const trash = path.join(home, 'trash');
  fs.mkdirSync(trash, { recursive: true });
  const state = { root: null, project: null, keys: new Map(), identity: crypto.randomBytes(64).toString('hex'), joinFolder: null, saves: 0, calls: [] };
  const at = (rel) => {
    for (const part of rel) {
      if (!part || part === '.' || part === '..' || part.includes('/') || part.startsWith('.')) throw 'outside';
    }
    if (!state.root) throw 'not-found';
    return path.join(state.root, ...rel);
  };
  const marker = (name) => path.join(state.root, MARKER, name);
  /** The folder Make a workspace for this file makes: named after the file, " 2", " 3" on a clash. */
  const freeFolder = (file) => {
    const stem = path.basename(file).replace(/\.curvyo$/i, '');
    for (let n = 1; ; n += 1) {
      const folder = path.join(path.dirname(file), n === 1 ? stem : `${stem} ${n}`);
      if (!fs.existsSync(folder)) return folder;
    }
  };
  const scan = () => {
    const found = [];
    const walk = (dir, rel) => {
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        if (entry.name.startsWith('.') || entry.isSymbolicLink()) continue;
        const full = path.join(dir, entry.name);
        const stat = fs.statSync(full);
        if (entry.isDirectory()) {
          found.push({ path: [...rel, entry.name], kind: 'folder', size: 0, modified: Math.floor(stat.mtimeMs / 1000) });
          walk(full, [...rel, entry.name]);
        } else if (entry.name.toLowerCase().endsWith('.curvyo')) {
          found.push({ path: [...rel, entry.name], kind: 'document', size: stat.size, modified: Math.floor(stat.mtimeMs / 1000) });
        }
      }
    };
    walk(state.root, []);
    return found;
  };
  const writeMarker = (id) => {
    fs.mkdirSync(path.join(state.root, MARKER), { recursive: true });
    fs.writeFileSync(marker('workspace.toml'), `format_version = 1\nid = "${id}"\n`);
  };
  const open = (folder, id) => {
    state.root = folder;
    if (!fs.existsSync(path.join(folder, MARKER, 'workspace.toml'))) writeMarker(id ?? crypto.randomBytes(16).toString('hex'));
    return { kind: 'opened', name: path.basename(folder), entries: scan() };
  };
  const readIndex = () => {
    try {
      return JSON.parse(fs.readFileSync(marker('index.json'), 'utf8')).documents;
    } catch {
      return {};
    }
  };
  const create = (file, bytes) => {
    if (fs.existsSync(file)) throw 'exists';
    fs.writeFileSync(file, bytes);
  };
  const toTrash = (file) => fs.renameSync(file, path.join(trash, `${Date.now()}-${crypto.randomBytes(3).toString('hex')}-${path.basename(file)}`));
  const kept = (document) => path.join(state.root, MARKER, 'deleted', `${document}.curvyo`);
  const commands = {
    take_pending_open: () => null,
    font_folder_list: () => ({ faces: [], skipped: [] }),
    system_font_list: () => [],
    read_device_keys: (a) => ({ status: 'ok', identity: state.identity, keys: state.keys.get(a.room) ?? [] }),
    write_document_keys: (a) => { state.keys.set(a.room, a.keys); return null; },
    recent_workspaces: () => [],
    remove_recent_workspace: () => [],
    pick_workspace: () => {
      const folder = path.join(home, 'Box job');
      fs.mkdirSync(folder, { recursive: true });
      return open(folder);
    },
    close_workspace: () => { state.root = null; return null; },
    scan_open_workspace: () => scan(),
    apply_workspace_op: (a, bytes) => {
      const op = a.op;
      if (op.kind === 'create-document') create(at(op.path), bytes ?? Buffer.alloc(0));
      else if (op.kind === 'create-folder') fs.mkdirSync(at(op.path));
      else if (op.kind === 'move') {
        const to = at(op.to);
        if (fs.existsSync(to)) throw 'exists';
        fs.renameSync(at(op.from), to);
        if (state.project && path.join(...state.project) === path.join(...op.from)) state.project = op.to;
      } else if (op.kind === 'trash') toTrash(at(op.path));
      return null;
    },
    read_workspace_document: (a) => {
      const file = at(a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      return { raw: fs.readFileSync(file) };
    },
    probe_workspace_document: (a) => {
      const answer = zip('probe', at(a.path));
      if (answer.error) throw answer.error;
      return { raw: Buffer.from(answer.thumbnail, 'hex') };
    },
    confirm_workspace_document_opened: (a) => { state.project = a.path; state.outside = null; return null; },
    reset_project: () => { state.project = null; state.outside = null; return null; },
    save_project_bytes: (a) => {
      const bytes = Buffer.from(a.bytes);
      if (state.project && state.root && !a.saveAs) {
        fs.writeFileSync(at(state.project), bytes);
        return { kind: 'saved', file_name: state.project[state.project.length - 1], path: at(state.project) };
      }
      if (state.outside && !a.saveAs) {
        fs.writeFileSync(state.outside, bytes);
        return { kind: 'saved', file_name: path.basename(state.outside), path: state.outside };
      }
      state.saves += 1;
      const file = path.join(home, `save ${state.saves}.curvyo`);
      fs.writeFileSync(file, bytes);
      state.outside = file;
      return { kind: 'saved', file_name: path.basename(file), path: file };
    },
    workspace_marker_share: () => {
      if (!state.root || !fs.existsSync(marker('share.json'))) return null;
      const read = (name) => Array.from(fs.readFileSync(marker(name)));
      return { share_json: read('share.json'), keyring_log: read('keyring.log'), list: read('list.loro') };
    },
    write_workspace_marker_share: (a) => {
      fs.writeFileSync(marker('share.json'), Buffer.from(a.marker.share_json));
      fs.writeFileSync(marker('keyring.log'), Buffer.from(a.marker.keyring_log));
      fs.writeFileSync(marker('list.loro'), Buffer.from(a.marker.list));
      fs.rmSync(marker('share.ended.json'), { force: true });
      return null;
    },
    write_workspace_id: (a) => { writeMarker(a.id); return null; },
    replace_workspace_document: (a, bytes) => {
      const file = at(a.path);
      if (!fs.existsSync(file)) throw 'not-found';
      fs.writeFileSync(file, bytes);
      return null;
    },
    write_workspace_index: (a) => {
      const documents = {};
      for (const { document, path: rel } of a.documents) {
        const stat = fs.statSync(at(rel), { throwIfNoEntry: false });
        documents[document] = { path: rel, size: stat?.size ?? 0, modified: stat ? Math.floor(stat.mtimeMs / 1000) : 0 };
      }
      fs.writeFileSync(marker('index.json'), JSON.stringify({ format_version: 1, documents }));
      return null;
    },
    // `specs/0105` criterion 8: Make a workspace for this file, as `curvyo-app` does it.
    workspace_folder_for_file: () => {
      if (!state.outside) throw 'not-found';
      return `${freeFolder(state.outside)}${path.sep}`;
    },
    make_workspace_for_file: () => {
      const file = state.outside;
      if (!file || !fs.existsSync(file)) throw 'not-found';
      const folder = freeFolder(file);
      fs.mkdirSync(folder);
      const name = path.basename(file);
      fs.renameSync(file, path.join(folder, name));
      state.outside = null;
      const answer = open(folder);
      state.project = [name];
      return { answer, path: [name] };
    },
    pick_join_folder: () => {
      state.joinFolder = path.join(home, state.joinName || 'Joined box');
      fs.mkdirSync(state.joinFolder, { recursive: true });
      return { kind: 'picked', path: state.joinFolder };
    },
    start_joined_workspace: (a) => open(state.joinFolder, a.id),
    scan_shared_workspace: () => {
      const entries = scan();
      const documents = entries.filter((e) => e.kind === 'document').map((e) => {
        const members = zip('members', at(e.path));
        return { path: e.path, share: members.share ?? null };
      });
      const known = Object.entries(readIndex()).map(([document, entry]) => ({ document, path: entry.path }));
      return { entries, folders: entries.filter((e) => e.kind === 'folder').map((e) => e.path), documents, known };
    },
    keep_workspace_document: (a) => {
      fs.mkdirSync(path.dirname(kept(a.document)), { recursive: true });
      fs.copyFileSync(at(a.path), kept(a.document));
      if (a.trash) toTrash(at(a.path));
      return null;
    },
    restore_workspace_document: (a) => {
      if (!fs.existsSync(kept(a.document))) throw 'not-found';
      create(at(a.path), fs.readFileSync(kept(a.document)));
      return null;
    },
    read_deleted_document: (a) => {
      if (!fs.existsSync(kept(a.document))) throw 'not-found';
      return { raw: fs.readFileSync(kept(a.document)) };
    },
    write_deleted_document: (a, bytes) => {
      fs.mkdirSync(path.dirname(kept(a.document)), { recursive: true });
      fs.writeFileSync(kept(a.document), bytes);
      return null;
    },
    clear_workspace_marker_share: () => {
      for (const name of ['share.json', 'keyring.log', 'list.loro', 'index.json', 'share.ended.json']) fs.rmSync(marker(name), { force: true });
      return null;
    },
    // `specs/0097` (ADR 0041 decision 6): a removal keeps share.ended.json in place of share.json
    // and keyring.log; complete share files win over it.
    workspace_marker_ended: () => {
      if (!fs.existsSync(marker('share.ended.json'))) return null;
      if (['share.json', 'keyring.log', 'list.loro'].every((n) => fs.existsSync(marker(n)))) return null;
      const list = fs.existsSync(marker('list.loro')) ? Array.from(fs.readFileSync(marker('list.loro'))) : [];
      return { ended_json: Array.from(fs.readFileSync(marker('share.ended.json'))), list };
    },
    end_workspace_marker_share: (a) => {
      fs.writeFileSync(marker('share.ended.json'), Buffer.from(a.ended_json));
      for (const name of ['share.json', 'keyring.log']) fs.rmSync(marker(name), { force: true });
      return null;
    },
  };
  return {
    state,
    home,
    /** One command from the page: `{value}`, `{raw: hex}` or `{error}`. */
    /** `state.delays[cmd]` holds a command back by that many ms (a slow disk, for the race of a
     * Save during Share's upload). */
    async handle(cmd, argsJson, bytesHex) {
      state.calls.push(cmd);
      const command = commands[cmd];
      if (!command) return { value: null };
      const delay = state.delays?.[cmd];
      if (delay) await new Promise((resolve) => setTimeout(resolve, delay));
      try {
        const answer = command(JSON.parse(argsJson), bytesHex == null ? null : Buffer.from(bytesHex, 'hex'));
        if (answer && answer.raw) return { raw: answer.raw.toString('hex') };
        return { value: answer ?? null };
      } catch (error) {
        return { error: typeof error === 'string' ? error : String(error) };
      }
    },
  };
}

/** The page side of the fake host: Tauri's internals, forwarding every command to `__host`. Raw
 * bodies travel as hex with their arguments from the `curvyo-args` header (`raw_ipc`); a raw
 * answer comes back as an `ArrayBuffer`. */
const FAKE_HOST = `(() => {
  let next = 1;
  const toHex = (bytes) => Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  const fromHex = (text) => {
    const bytes = new Uint8Array(text.length / 2);
    for (let i = 0; i < bytes.length; i += 1) bytes[i] = parseInt(text.slice(2 * i, 2 * i + 2), 16);
    return bytes.buffer;
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
    transformCallback: () => next++,
    unregisterCallback: () => undefined,
    convertFileSrc: (p) => p,
    invoke: async (cmd, args, options) => {
      if (cmd.startsWith('plugin:')) return cmd === 'plugin:event|listen' ? next++ : null;
      let plain = args ?? {};
      let bytes = null;
      if (args instanceof Uint8Array || args instanceof ArrayBuffer) {
        bytes = toHex(new Uint8Array(args));
        const headers = options && options.headers;
        const text = headers instanceof Headers ? headers.get('curvyo-args') : headers && headers['curvyo-args'];
        plain = JSON.parse(text || '{}');
      }
      const answer = await window.__host(cmd, JSON.stringify(plain), bytes);
      if (answer.error !== undefined) throw answer.error;
      if (answer.raw !== undefined) return fromHex(answer.raw);
      return answer.value;
    },
  };
})()`;


module.exports = { FAKE_HOST, MARKER, hex, makeHost, zip };

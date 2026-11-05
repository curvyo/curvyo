// A synced workspace that is not open (`specs/0096-own-devices-sync` criteria 10, 11, 18 to 20,
// ADR 0040 decision 7): its own session and socket, driven like the open workspace of ADR 0037
// decision 8 (the session opens and seals, the background writer's worker writes the files), with
// every host command aimed at it by its id. Every rule is Rust's; this carries bytes and time.

import { BackgroundWriterLink, type WriterItem, type WriterResponse, sendLocal } from "@/lib/backgroundWriterJobs";
import { answerBytes } from "@/lib/binaryArgs";
import { mintPeerId, wasmReady } from "@/lib/editorSession";
import { type DiskStep, type DocumentPlace, type SharedScan, parseDiskPlan, parsePlace } from "@/lib/sharedTree";
import { SyncTransport, type SocketLike } from "@/lib/syncTransport";
import { invoke } from "@/platform/host";
import { type GroupFacts, isSwitchOff, parseEnded } from "@/lib/endedRecord";
import { WasmSession, devices_no_longer_synced_notice } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** How often the files, the marker and the disk plan are looked at (ms). */
const SLOW_MS = 250;
const MARKER_MS = 2000;
const SCAN_MS = 2000;

/** This device's identity (hex) and a room's epoch keys, as `useCollaboration` hands them out. */
export interface RoomKeys {
  identity: string;
  keys: [number, string][];
}

/** What a background workspace needs from the page. */
export interface BackgroundHost {
  /** The device identity and the room's keys, `null` while they cannot be had. */
  keys: (room: string) => Promise<RoomKeys | null>;
  /** Shows a canvas notice. */
  notice: (text: string, alert: boolean) => void;
  /** The workspace's room was left or this device was removed from it. */
  ended: (id: string, removed: boolean) => void;
}

function random(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  crypto.getRandomValues(bytes);
  return bytes;
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  }
  return bytes;
}

interface MarkerPayload {
  share_json: number[];
  keyring_log: number[];
  list: number[];
}

/** The background workspaces of this page, by workspace id (hex). */
const running = new Map<string, BackgroundWorkspace>();
/** Starts that are on their way, so a claim waits for them. */
const starting = new Map<string, Promise<unknown>>();
/** The workspace the editor holds open: it never runs in the background. */
let openRoom = "";

/** The editor opens the workspace `room`: its background session hands over first (its marker
 * written, its socket closed), so one device keeps one connection per workspace. */
export async function claimOpenRoom(room: string): Promise<void> {
  openRoom = room;
  await starting.get(room);
  const runner = running.get(room);
  if (runner) {
    await runner.stop();
  }
}

/** The editor closed its workspace. */
export function releaseOpenRoom(): void {
  openRoom = "";
}

/** The workspace the editor holds open, "" when none. */
export function openRoomNow(): string {
  return openRoom;
}

/** What My devices last told about its group's list. */
let group: GroupFacts = { listed: new Set(), switchedOn: new Set(), members: new Set() };

/** My devices tells which workspaces its group's list names (`on` among them) and its devices. */
export function setGroupWorkspaces(workspaces: Iterable<{ id: string; on: boolean }>, members: Iterable<string>): void {
  const all = [...workspaces];
  group = {
    listed: new Set(all.map((w) => w.id)),
    switchedOn: new Set(all.filter((w) => w.on).map((w) => w.id)),
    members: new Set(members),
  };
}

/** Whether the end of workspace `id` is a switch-off by my own devices ([`isSwitchOff`]). */
export function endedBySwitchOff(id: string, removedBy: string, left: boolean): boolean {
  return isSwitchOff(group, id, removedBy, left);
}

/** The running background workspaces. */
export function backgroundWorkspaces(): ReadonlyMap<string, BackgroundWorkspace> {
  return running;
}

/** Starts `id` from its marker (it is on this device already): `null` when it cannot. */
export function resumeBackground(id: string, name: string, label: string, host: BackgroundHost): Promise<BackgroundWorkspace | null> {
  return track(id, async () => {
    const marker = await invoke<MarkerPayload | null>("workspace_marker_share", { workspace: id }).catch(() => null);
    const keys = marker ? await host.keys(id) : null;
    if (!marker || !keys || openRoom === id || running.has(id)) {
      return null;
    }
    await wasmReady();
    const session = new WasmSession(mintPeerId());
    const resumed = session.resume_workspace(
      new Uint8Array(marker.share_json),
      new Uint8Array(marker.keyring_log),
      new Uint8Array(marker.list),
      name,
      label,
      JSON.stringify(keys.keys),
      fromHex(keys.identity),
      random(32),
      random(32),
      Date.now(),
    );
    if (!resumed) {
      session.free();
      return null;
    }
    return new BackgroundWorkspace(id, name, session, host, false);
  });
}

/** Arrives in `id` as a member the device group admitted (criteria 10, 11): its folder exists
 * (empty, or a copy the maker chose to connect, criterion 14). */
export function arriveBackground(
  id: string,
  name: string,
  server: string,
  label: string,
  host: BackgroundHost,
): Promise<BackgroundWorkspace | null> {
  return track(id, async () => {
    const keys = await host.keys("");
    if (!keys || openRoom === id || running.has(id)) {
      return null;
    }
    await wasmReady();
    const session = new WasmSession(mintPeerId());
    const started = session.join_synced_workspace(
      server,
      id,
      name,
      label,
      fromHex(keys.identity),
      random(32),
      random(32),
      Date.now(),
    );
    if (!started) {
      session.free();
      return null;
    }
    return new BackgroundWorkspace(id, name, session, host, true);
  });
}

/** Runs `session`, which shares `id` from a folder that is not open (Sync these): its files are
 * left alone until `release` (the Share writes them first). */
export function adoptBackground(id: string, name: string, session: WasmSession, host: BackgroundHost): BackgroundWorkspace {
  const runner = new BackgroundWorkspace(id, name, session, host, false);
  runner.held = true;
  running.set(id, runner);
  return runner;
}

async function track(id: string, start: () => Promise<BackgroundWorkspace | null>): Promise<BackgroundWorkspace | null> {
  const pending = starting.get(id);
  if (pending) {
    await pending;
    return running.get(id) ?? null;
  }
  const started = start();
  starting.set(id, started);
  try {
    const runner = await started;
    if (runner) {
      running.set(id, runner);
    }
    return runner;
  } finally {
    starting.delete(id);
  }
}

/** One synced workspace held in the background. */
export class BackgroundWorkspace {
  readonly id: string;
  readonly name: string;
  readonly session: WasmSession;
  private readonly host: BackgroundHost;
  private readonly transport: SyncTransport;
  private writer: BackgroundWriterLink | null = null;
  /** File writes run one after the other. */
  private chain: Promise<void> = Promise.resolve();
  private joining: boolean;
  private joined = new Map<string, string[]>();
  private slowAt = 0;
  private markerAt = 0;
  private scanAt = 0;
  private scanning = false;
  private building = false;
  private stopped = false;
  private index = "";
  /** While Share writes the files, the marker and the list are not touched. */
  held = false;

  constructor(id: string, name: string, session: WasmSession, host: BackgroundHost, joining: boolean) {
    this.id = id;
    this.name = name;
    this.session = session;
    this.host = host;
    this.joining = joining;
    this.transport = new SyncTransport(
      (address) => new WebSocket(address) as unknown as SocketLike,
      undefined,
      () => !this.stopped,
    );
    this.transport.attach(session);
    this.transport.drain();
  }

  /** Whether the room admitted the session (`connected`), it gave up (`not-connected`), or
   * neither yet. */
  get state(): string {
    try {
      return (JSON.parse(this.session.share_view_json()) as { state?: string }).state ?? "";
    } catch {
      return "";
    }
  }

  /** Whether the session still holds its link. */
  get live(): boolean {
    return !this.stopped && this.session.has_workspace_link();
  }

  /** One step of the page's timer: the socket, notices and keys; the files every 250 ms. */
  tick(now: number): void {
    if (this.stopped) {
      return;
    }
    this.transport.tick();
    for (let notice = this.session.take_sync_notice(); notice !== ""; notice = this.session.take_sync_notice()) {
      const { text, alert } = JSON.parse(notice) as { text: string; alert: boolean };
      this.host.notice(text, alert);
    }
    const keys = this.session.take_key_update();
    if (keys !== "") {
      const { room, keys: list } = JSON.parse(keys) as { room: string; keys: [number, string][] };
      void invoke("write_document_keys", { room, keys: list }).catch(() => undefined);
    }
    if (now - this.slowAt < SLOW_MS) {
      return;
    }
    this.slowAt = now;
    if (this.session.workspace_removed()) {
      this.removed();
      return;
    }
    if (!this.session.has_workspace_link() || this.held) {
      return;
    }
    this.collectWrites();
    this.feedWriter();
    if (!this.building && this.session.workspace_snapshot_ready() >= 0) {
      void this.buildSnapshot();
    }
    if (!this.joining && now - this.markerAt >= MARKER_MS) {
      this.markerAt = now;
      this.writeMarker();
    }
    if (!this.joining && !this.scanning && now - this.scanAt >= SCAN_MS) {
      this.scanAt = now;
      this.scanning = true;
      this.chain = this.chain.then(() => this.followList()).finally(() => {
        this.scanning = false;
      });
    }
  }

  /** The network is back: reconnect at once. */
  networkBack(): void {
    this.transport.networkBack();
  }

  /** Carries out what the session just queued (after a reconcile). */
  drive(): void {
    this.transport.drain();
  }

  /** Writes the marker, closes the socket and ends the worker; the session is freed. */
  async stop(): Promise<void> {
    if (this.stopped) {
      return;
    }
    if (running.get(this.id) === this) {
      running.delete(this.id);
    }
    this.writeMarker();
    await this.chain;
    this.stopped = true;
    this.session.leave_sync();
    this.transport.drain();
    this.transport.attach(null);
    this.writer?.stop();
    this.writer = null;
    this.session.free();
  }

  /** Leave my devices here: its own removal goes out, then it stops (criterion 27). */
  async leave(): Promise<boolean> {
    if (!this.session.leave_synced_workspace()) {
      return false;
    }
    this.transport.drain();
    await this.chain;
    this.stopped = true;
    running.delete(this.id);
    this.transport.attach(null);
    this.writer?.stop();
    this.session.free();
    return true;
  }

  private removed(): void {
    // Criteria 13 and 24: the folder stays as a local workspace with every document. A
    // switch-off drops the share; any other end keeps the way back (`specs/0097` criterion 20).
    const ended = Array.from(this.session.workspace_ended_json());
    const switchedOff = endedBySwitchOff(this.id, this.session.share_removed_by_hex(), parseEnded(ended)?.left ?? false);
    const end =
      switchedOff || ended.length === 0
        ? invoke("clear_workspace_marker_share", { workspace: this.id })
        : invoke("end_workspace_marker_share", { workspace: this.id, ended_json: ended });
    this.chain = this.chain.then(() =>
      end.then(
        () => undefined,
        () => undefined,
      ),
    );
    if (switchedOff) {
      // Another maker's removal or a leave was told by the session in its own words.
      this.host.notice(devices_no_longer_synced_notice(this.name), false);
    }
    this.host.ended(this.id, true);
    void this.stop();
  }

  private writeMarker(): void {
    if (this.stopped || this.joining) {
      return;
    }
    const marker = this.session.take_workspace_marker();
    if (!marker) {
      return;
    }
    const payload = {
      share_json: Array.from(marker.share_json),
      keyring_log: Array.from(marker.keyring_log),
      list: Array.from(marker.list),
    };
    marker.free();
    const joined = this.joined;
    this.joined = new Map();
    this.chain = this.chain.then(async () => {
      await invoke("write_workspace_marker_share", { marker: payload, workspace: this.id }).catch(() => undefined);
      if (joined.size > 0) {
        await invoke("write_workspace_index", {
          documents: [...joined].map(([document, path]) => ({ document, path })),
          workspace: this.id,
        }).catch(() => undefined);
      }
    });
  }

  /** The first copy's folders and documents (criterion 10); a document whose file is here
   * already (Connect this folder, criterion 14) is merged into it by the writer. */
  private collectWrites(): void {
    const writes: { kind: string; path: string[]; document: string; bytes: Uint8Array }[] = [];
    for (let write = this.session.next_workspace_write(); write; write = this.session.next_workspace_write()) {
      writes.push({
        kind: write.kind,
        path: JSON.parse(write.path) as string[],
        document: write.document,
        bytes: write.bytes,
      });
      write.free();
    }
    if (this.joining && this.session.workspace_id_hex() !== "" && writes.length === 0) {
      this.joining = false;
    }
    if (writes.length === 0) {
      return;
    }
    this.chain = this.chain.then(async () => {
      for (const write of writes) {
        if (write.kind === "deleted") {
          await invoke("write_deleted_document", { document: write.document, workspace: this.id }, write.bytes).catch(
            () => undefined,
          );
          this.session.workspace_document_written(write.document);
          continue;
        }
        const op = { kind: write.kind === "folder" ? "create-folder" : "create-document", path: write.path };
        const created = await invoke("apply_workspace_op", { op, workspace: this.id }, write.kind === "folder" ? undefined : write.bytes).then(
          () => true,
          (code: unknown) => code !== "exists",
        );
        if (write.kind !== "document") {
          continue;
        }
        this.joined.set(write.document, write.path);
        this.session.workspace_document_written(write.document);
        if (!created) {
          // A copy holds this document: both histories meet in its file.
          this.send([{ document: write.document, seq: 0, kind: "container", bytes: write.bytes }]);
        }
      }
    });
  }

  private send(items: WriterItem[]): void {
    this.writer ??= new BackgroundWriterLink(mintPeerId(), (answer) => this.onWriter(answer));
    this.writer.send({ type: "records", items }, items.map((item) => item.bytes.buffer));
  }

  private feedWriter(): void {
    const items: WriterItem[] = [];
    for (let item = this.session.next_background_item(); item; item = this.session.next_background_item()) {
      items.push({ document: item.document, seq: item.seq, kind: item.kind, bytes: item.bytes });
      item.free();
    }
    if (items.length > 0) {
      this.send(items);
    }
  }

  private onWriter(answer: WriterResponse): void {
    const link = this.writer;
    if (!link || this.stopped) {
      return;
    }
    const { document } = answer as { document?: string };
    if (answer.type === "need" && document) {
      const place = parsePlace(this.session.workspace_document_place(document));
      if (place.kind === "unknown") {
        link.send({ type: "later", document });
        return;
      }
      void this.read(place, document).then((file) => {
        if (file) {
          link.send({ type: "file", document, workspace: this.id, bytes: file }, [file.buffer]);
        } else {
          link.send({ type: "create", document, workspace: this.id });
        }
      });
    } else if (answer.type === "written") {
      this.chain = this.chain.then(async () => {
        if (this.stopped) {
          return;
        }
        const place = parsePlace(this.session.workspace_document_place(answer.document));
        const bytes = sendLocal(this.session, answer.document, answer.bytes, answer.outgoing);
        if (await this.write(place, answer.document, bytes, answer.created)) {
          this.session.background_written(answer.document, answer.through);
        }
      });
    } else if (answer.type === "skipped" && answer.through >= 0) {
      this.session.background_written(answer.document, answer.through);
    }
  }

  private read(place: DocumentPlace, document: string): Promise<Uint8Array<ArrayBuffer> | null> {
    const read =
      place.kind === "file"
        ? invoke<ArrayBuffer>("read_workspace_document", { path: place.path, workspace: this.id })
        : invoke<ArrayBuffer>("read_deleted_document", { document, workspace: this.id });
    return read.then(answerBytes, () => null);
  }

  private write(place: DocumentPlace, document: string, bytes: Uint8Array, created: boolean): Promise<boolean> {
    const workspace = this.id;
    const write =
      place.kind === "deleted"
        ? invoke("write_deleted_document", { document, workspace }, bytes)
        : place.kind === "file" && created
          ? invoke("apply_workspace_op", { op: { kind: "create-document", path: place.path }, workspace }, bytes)
          : place.kind === "file"
            ? invoke("replace_workspace_document", { path: place.path, workspace }, bytes)
            : Promise.reject(new Error("no place"));
    return write.then(
      () => true,
      () => false,
    );
  }

  /** A workspace snapshot the room asked for, from the files once they hold its records. */
  private async buildSnapshot(): Promise<void> {
    this.building = true;
    try {
      const wanted = this.session.workspace_snapshot_documents_json();
      const files: [string, Uint8Array][] = [];
      for (const entry of JSON.parse(wanted) as { document: string; path?: string[] }[]) {
        const place: DocumentPlace = entry.path ? { kind: "file", path: entry.path } : { kind: "deleted" };
        const bytes = await this.read(place, entry.document);
        if (bytes) {
          files.push([entry.document, bytes]);
        }
      }
      if (this.stopped || this.session.workspace_snapshot_ready() < 0 || this.session.workspace_snapshot_documents_json() !== wanted) {
        return;
      }
      for (const [document, bytes] of files) {
        this.session.add_workspace_snapshot_document(document, bytes);
      }
      this.session.finish_workspace_snapshot(random(16));
      this.transport.drain();
    } finally {
      this.building = false;
    }
  }

  /** Brings the folder to the shared list once (`0095` criteria 35 to 41): the session plans
   * from the host's scan, this runs the steps. Nothing is open here, so a deleted document goes
   * to the trash at once. */
  private async followList(): Promise<void> {
    if (this.stopped) {
      return;
    }
    const scan = await invoke<SharedScan>("scan_shared_workspace", { workspace: this.id }).catch(() => null);
    if (!scan || this.stopped) {
      return;
    }
    const plan = parseDiskPlan(this.session.workspace_disk_plan(JSON.stringify(scan), Date.now(), random(16)));
    if (!plan) {
      return;
    }
    for (const step of plan.steps) {
      if (!(await this.runStep(step))) {
        return;
      }
    }
    const index = JSON.stringify(plan.settled);
    if (index !== this.index) {
      this.index = index;
      await invoke("write_workspace_index", { documents: plan.settled, workspace: this.id }).catch(() => undefined);
    }
  }

  private runStep(step: DiskStep): Promise<boolean> {
    const workspace = this.id;
    const done = (call: Promise<unknown>) =>
      call.then(
        () => true,
        () => false,
      );
    switch (step.kind) {
      case "create-folder":
        return done(invoke("apply_workspace_op", { op: { kind: "create-folder", path: step.path }, workspace }));
      case "move":
        return done(invoke("apply_workspace_op", { op: { kind: "move", from: step.from, to: step.to }, workspace }));
      case "write":
        // Restore writes the kept copy; a new document is written by the background writer.
        return done(invoke("restore_workspace_document", { document: step.document, path: step.path, workspace })).then(
          () => true,
        );
      case "keep":
        return done(invoke("keep_workspace_document", { path: step.path, document: step.document, trash: false, workspace }));
      case "trash-document":
        return done(invoke("keep_workspace_document", { path: step.path, document: step.document, trash: true, workspace }));
      case "trash":
        return done(invoke("apply_workspace_op", { op: { kind: "trash", path: step.path }, workspace }));
    }
  }
}

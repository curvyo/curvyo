// Take-in of a document shared on its own (`specs/0095-workspace` criteria 42 and 49, ADR 0037
// decision 10): after its state went into the workspace, a short-lived session of its own room
// appends the sealed `Moved` record there, so that no member syncs that room or admits anyone to
// it again. The record and the rules are Rust's; this carries the session to the relay and back.

import { openSession } from "@/lib/editorSession";
import { type SocketLike, SyncTransport } from "@/lib/syncTransport";

/** How long the take-in waits for the document's own room (ms). */
const MOVED_WAIT_MS = 15_000;
const POLL_MS = 100;

/** The device keys of a room (`useCollaboration`'s `deviceKeys`). */
export interface RoomKeys {
  keys: [number, string][];
  identity: string;
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

/** Waits until `done` holds, ticking the transport; whether it held in time. */
function until(transport: SyncTransport, done: () => boolean): Promise<boolean> {
  return new Promise((resolve) => {
    const started = Date.now();
    const timer = window.setInterval(() => {
      transport.tick();
      if (done()) {
        window.clearInterval(timer);
        resolve(true);
      } else if (Date.now() - started > MOVED_WAIT_MS) {
        window.clearInterval(timer);
        resolve(false);
      }
    }, POLL_MS);
  });
}

/** A document's own room, connected and caught up, for its `Moved` record. */
export interface OldRoom {
  /** Appends `Moved{workspace, document}` (hex); whether the relay stored it. */
  push: (workspace: string, document: string) => Promise<boolean>;
  /** Leaves the room. */
  close: () => void;
}

/** Connects to the own room of the document file `original`; `null` when it cannot be reached
 * within 15 s. A take-in checks this before anything changes (ADR 0037 decision 10, Proposal):
 * an unreachable room means the document is not taken in and keeps syncing on its own. */
export async function openOldRoom(
  original: Uint8Array,
  label: string,
  keysFor: (room: string) => Promise<RoomKeys | null>,
): Promise<OldRoom | null> {
  const session = await openSession(original);
  const transport = new SyncTransport((address) => new WebSocket(address) as unknown as SocketLike);
  const close = () => {
    session.leave_sync();
    transport.drain();
    transport.attach(null);
    session.free();
  };
  const keys = await keysFor(session.shared_room_hex()).catch(() => null);
  if (!keys) {
    session.free();
    return null;
  }
  session.resume_shared(label, JSON.stringify(keys.keys), fromHex(keys.identity), random(32), random(32), Date.now());
  transport.attach(session);
  transport.drain();
  if (!(await until(transport, () => session.sync_online()))) {
    close();
    return null;
  }
  return {
    push: async (workspace, document) => {
      if (!session.push_moved(workspace, document)) {
        return false;
      }
      transport.drain();
      return until(transport, () => session.moved_stored());
    },
    close,
  };
}

/** Appends `Moved{workspace, document}` (hex) to the own room of the document file `original`;
 * whether the relay stored it. */
export async function pushMoved(
  original: Uint8Array,
  workspace: string,
  document: string,
  label: string,
  keysFor: (room: string) => Promise<RoomKeys | null>,
): Promise<boolean> {
  const room = await openOldRoom(original, label, keysFor);
  if (!room) {
    return false;
  }
  try {
    return await room.push(workspace, document);
  } finally {
    room.close();
  }
}

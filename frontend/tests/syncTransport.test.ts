import assert from "node:assert/strict";
import { test } from "node:test";

import { SyncTransport, type SocketLike, type SyncSessionPort } from "../src/lib/syncTransport.ts";

/** A session that queues the actions a test gives it and records what it is told. */
class FakeSession implements SyncSessionPort {
  actions: { kind: string; address: string; bytes: Uint8Array }[] = [];
  told: string[] = [];
  sync_next_action() {
    return this.actions.shift();
  }
  sync_connected(now: number) {
    this.told.push(`connected@${now}`);
  }
  sync_disconnected(now: number) {
    this.told.push(`disconnected@${now}`);
  }
  sync_received(bytes: Uint8Array, now: number) {
    this.told.push(`received ${Array.from(bytes).join(",")}@${now}`);
  }
  sync_tick(now: number) {
    this.told.push(`tick@${now}`);
  }
  sync_network_back(now: number) {
    this.told.push(`back@${now}`);
  }
}

class FakeSocket implements SocketLike {
  binaryType = "blob";
  onopen: ((event: unknown) => void) | null = null;
  onclose: ((event: unknown) => void) | null = null;
  onerror: ((event: unknown) => void) | null = null;
  onmessage: ((event: { data: unknown }) => void) | null = null;
  sent: number[][] = [];
  closed = false;
  readonly address: string;
  constructor(address: string) {
    this.address = address;
  }
  send(data: Uint8Array) {
    this.sent.push(Array.from(data));
  }
  close() {
    this.closed = true;
  }
}

function rig() {
  const sockets: FakeSocket[] = [];
  let clock = 1000;
  const transport = new SyncTransport(
    (address) => {
      const socket = new FakeSocket(address);
      sockets.push(socket);
      return socket;
    },
    () => clock,
  );
  const session = new FakeSession();
  transport.attach(session);
  return { transport, session, sockets, advance: (ms: number) => (clock += ms) };
}

const action = (kind: string, address = "", bytes: number[] = []) => ({
  kind,
  address,
  bytes: new Uint8Array(bytes),
});

test("the session's connect opens a binary socket, and its open and messages are reported", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://relay.test:8787/"));
  transport.tick();
  assert.equal(sockets.length, 1);
  assert.equal(sockets[0].address, "ws://relay.test:8787/");
  assert.equal(sockets[0].binaryType, "arraybuffer");
  sockets[0].onopen?.({});
  sockets[0].onmessage?.({ data: new Uint8Array([7, 8]).buffer });
  assert.deepEqual(session.told, ["tick@1000", "connected@1000", "received 7,8@1000"]);
});

test("sends go out on the open socket and close closes it without a report", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://h:1/"), action("send", "", [1, 2, 3]));
  transport.drain();
  assert.deepEqual(sockets[0].sent, [[1, 2, 3]]);
  session.actions.push(action("close"));
  transport.drain();
  assert.equal(sockets[0].closed, true);
  sockets[0].onclose?.({});
  assert.ok(!session.told.some((t) => t.startsWith("disconnected")), "a closed socket is not reported");
});

test("a lost socket is reported once and its late events are ignored", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://h:1/"));
  transport.drain();
  sockets[0].onerror?.({});
  sockets[0].onclose?.({});
  sockets[0].onmessage?.({ data: new Uint8Array([1]).buffer });
  assert.deepEqual(session.told, ["disconnected@1000"]);
});

test("the network coming back reaches the session at once", () => {
  const { transport, session, advance } = rig();
  advance(500);
  transport.networkBack();
  assert.deepEqual(session.told, ["back@1500"]);
});

test("another session closes the old socket; its events no longer reach anyone", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://h:1/"));
  transport.drain();
  const next = new FakeSession();
  transport.attach(next);
  assert.equal(sockets[0].closed, true);
  sockets[0].onopen?.({});
  assert.deepEqual(session.told, []);
  assert.deepEqual(next.told, []);
});

test("a socket that cannot be made counts as a failed attempt", () => {
  const session = new FakeSession();
  const transport = new SyncTransport(
    () => {
      throw new Error("blocked");
    },
    () => 5,
  );
  transport.attach(session);
  session.actions.push(action("connect", "nonsense"));
  transport.drain();
  assert.deepEqual(session.told, ["disconnected@5"]);
});

test("events of a socket whose session was freed reach nothing and do not throw", () => {
  const sockets: FakeSocket[] = [];
  const session = new FakeSession();
  let live: FakeSession | null = session;
  const transport = new SyncTransport(
    (address) => {
      const socket = new FakeSocket(address);
      sockets.push(socket);
      return socket;
    },
    () => 1,
    (s) => s === live,
  );
  transport.attach(session);
  session.actions.push(action("connect", "ws://x"));
  transport.drain();
  live = null; // New/Open frees the session before the timer re-attaches
  const before = [...session.told];
  assert.doesNotThrow(() => {
    sockets[0].onmessage?.({ data: new Uint8Array([1]).buffer });
    sockets[0].onclose?.({});
    sockets[0].onerror?.({});
  });
  assert.deepEqual(session.told, before);
});

test("a workspace link handed to the next session keeps its socket and holds what arrives", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://h:1/"));
  transport.drain();
  sockets[0].onopen?.({});
  const next = new FakeSession();
  transport.handOver(next);
  transport.attach(null);
  sockets[0].onmessage?.({ data: new Uint8Array([4]).buffer });
  assert.equal(sockets[0].closed, false, "the socket stays open");
  assert.deepEqual(next.told, [], "nothing reaches a session still being attached");
  transport.attach(next);
  assert.deepEqual(next.told, ["received 4@1000"]);
  next.actions.push(action("send", "", [9]));
  transport.drain();
  assert.deepEqual(sockets[0].sent, [[9]]);
  assert.deepEqual(session.told, ["connected@1000"]);
});

test("attaching another session than the one handed to closes the socket", () => {
  const { transport, session, sockets } = rig();
  session.actions.push(action("connect", "ws://h:1/"));
  transport.drain();
  transport.handOver(new FakeSession());
  transport.attach(new FakeSession());
  assert.equal(sockets[0].closed, true);
});

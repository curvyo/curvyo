// The WebSocket of a shared document or workspace (ADR 0031 decision 2): it opens, sends, receives and closes
// when the wasm session says so, and hands the clock to it. It holds no protocol logic; every
// decision (when to connect, what to send, when to give up) is the session's.

/** What the transport reads from and reports to the editing session (`WasmSession`). */
export interface SyncSessionPort {
  sync_next_action():
    | { kind: string; address: string; bytes: Uint8Array; free?: () => void }
    | undefined;
  sync_connected(now: number): void;
  sync_disconnected(now: number): void;
  sync_received(bytes: Uint8Array, now: number): void;
  sync_tick(now: number): void;
  sync_network_back(now: number): void;
}

/** The parts of a `WebSocket` the transport uses. */
export interface SocketLike {
  binaryType: string;
  onopen: ((event: unknown) => void) | null;
  onclose: ((event: unknown) => void) | null;
  onerror: ((event: unknown) => void) | null;
  onmessage: ((event: { data: unknown }) => void) | null;
  send(data: Uint8Array): void;
  close(): void;
}

/** What a socket reported while its session was being handed over. */
type Held = { kind: "received"; bytes: Uint8Array } | { kind: "connected" } | { kind: "lost" };

/** One socket per open shared document or shared workspace, driven by the session's transport
 * actions. A shared workspace's link moves from one document session to the next with its socket
 * (`specs/0094-workspace`): `handOver` keeps the socket and holds what arrives until the new
 * session is attached. */
export class SyncTransport {
  private session: SyncSessionPort | null = null;
  private socket: SocketLike | null = null;
  /** Counts sockets, so an event of a socket that was replaced or closed is ignored. */
  private generation = 0;
  /** The session the socket was handed to, not yet usable (its canvas is being attached). */
  private handedTo: SyncSessionPort | null = null;
  private held: Held[] = [];

  private readonly makeSocket: (address: string) => SocketLike;
  private readonly now: () => number;

  constructor(makeSocket: (address: string) => SocketLike, now: () => number = () => Date.now()) {
    this.makeSocket = makeSocket;
    this.now = now;
  }

  /** The session this transport serves; a different one closes the old socket first, unless the
   * socket was handed to it. `null` while a handed-over session is being attached keeps it. */
  attach(session: SyncSessionPort | null): void {
    if (this.handedTo) {
      if (session === null) {
        return;
      }
      if (session === this.handedTo) {
        this.handedTo = null;
        this.session = session;
        this.replay(session);
        return;
      }
      this.handedTo = null;
      this.held = [];
    }
    if (session === this.session) {
      return;
    }
    this.dropSocket();
    this.session = session;
  }

  /** The workspace's link moved to `next` with its socket: the socket stays open, and what it
   * reports waits until `next` is attached. */
  handOver(next: SyncSessionPort): void {
    this.handedTo = next;
    this.session = null;
    this.held = [];
  }

  /** The timer: the session's clock moves and what it asks for is carried out. */
  tick(): void {
    this.session?.sync_tick(this.now());
    this.drain();
  }

  /** The platform reports the network is back: the session reconnects at once. */
  networkBack(): void {
    this.session?.sync_network_back(this.now());
    this.drain();
  }

  /** Carries out every action the session has queued. */
  drain(): void {
    const session = this.session;
    if (!session) {
      return;
    }
    for (;;) {
      const action = session.sync_next_action();
      if (!action) {
        return;
      }
      const { kind, address, bytes } = action;
      action.free?.();
      if (kind === "connect") {
        this.open(address);
      } else if (kind === "send") {
        this.send(bytes);
      } else if (kind === "close") {
        this.dropSocket();
      }
    }
  }

  /** Hands what arrived during a hand-over to the attached session, in order. */
  private replay(session: SyncSessionPort): void {
    const held = this.held;
    this.held = [];
    for (const event of held) {
      if (event.kind === "received") {
        session.sync_received(event.bytes, this.now());
      } else if (event.kind === "connected") {
        session.sync_connected(this.now());
      } else {
        session.sync_disconnected(this.now());
      }
    }
    this.drain();
  }

  /** Reports a socket event to the served session, or holds it during a hand-over. */
  private report(event: Held): void {
    if (this.handedTo) {
      this.held.push(event);
      return;
    }
    const session = this.session;
    if (!session) {
      return;
    }
    if (event.kind === "received") {
      session.sync_received(event.bytes, this.now());
    } else if (event.kind === "connected") {
      session.sync_connected(this.now());
    } else {
      session.sync_disconnected(this.now());
    }
    this.drain();
  }

  private open(address: string): void {
    this.dropSocket();
    const generation = ++this.generation;
    const current = () => generation === this.generation;
    let socket: SocketLike;
    try {
      socket = this.makeSocket(address);
    } catch {
      // A malformed address or a blocked scheme: the session hears a failed attempt.
      this.session?.sync_disconnected(this.now());
      return;
    }
    socket.binaryType = "arraybuffer";
    this.socket = socket;
    socket.onopen = () => {
      if (current()) {
        this.report({ kind: "connected" });
      }
    };
    socket.onmessage = (event) => {
      if (current() && event.data instanceof ArrayBuffer) {
        this.report({ kind: "received", bytes: new Uint8Array(event.data) });
      }
    };
    const lost = () => {
      if (current()) {
        this.generation += 1;
        this.socket = null;
        this.report({ kind: "lost" });
      }
    };
    socket.onclose = lost;
    socket.onerror = lost;
  }

  private send(bytes: Uint8Array): void {
    const socket = this.socket;
    if (!socket) {
      return;
    }
    try {
      socket.send(bytes);
    } catch {
      this.dropSocket();
      this.session?.sync_disconnected(this.now());
    }
  }

  /** Closes the socket without reporting it: the session asked for it, or is gone. */
  private dropSocket(): void {
    this.generation += 1;
    const socket = this.socket;
    this.socket = null;
    if (socket) {
      socket.onopen = null;
      socket.onmessage = null;
      socket.onclose = null;
      socket.onerror = null;
      try {
        socket.close();
      } catch {
        // Already closed.
      }
    }
  }
}

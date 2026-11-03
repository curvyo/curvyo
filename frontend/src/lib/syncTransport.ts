// The WebSocket of a shared document (ADR 0031 decision 2): it opens, sends, receives and closes
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

/** One socket per open shared document, driven by the session's transport actions. */
export class SyncTransport {
  private session: SyncSessionPort | null = null;
  private socket: SocketLike | null = null;
  /** Counts sockets, so an event of a socket that was replaced or closed is ignored. */
  private generation = 0;

  private readonly makeSocket: (address: string) => SocketLike;
  private readonly now: () => number;

  constructor(makeSocket: (address: string) => SocketLike, now: () => number = () => Date.now()) {
    this.makeSocket = makeSocket;
    this.now = now;
  }

  /** The session this transport serves; a different one closes the old socket first. */
  attach(session: SyncSessionPort | null): void {
    if (session === this.session) {
      return;
    }
    this.dropSocket();
    this.session = session;
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
        this.open(session, address);
      } else if (kind === "send") {
        this.send(session, bytes);
      } else if (kind === "close") {
        this.dropSocket();
      }
    }
  }

  private open(session: SyncSessionPort, address: string): void {
    this.dropSocket();
    const generation = ++this.generation;
    const current = () => generation === this.generation && this.session === session;
    let socket: SocketLike;
    try {
      socket = this.makeSocket(address);
    } catch {
      // A malformed address or a blocked scheme: the session hears a failed attempt.
      session.sync_disconnected(this.now());
      return;
    }
    socket.binaryType = "arraybuffer";
    this.socket = socket;
    socket.onopen = () => {
      if (current()) {
        session.sync_connected(this.now());
        this.drain();
      }
    };
    socket.onmessage = (event) => {
      if (current() && event.data instanceof ArrayBuffer) {
        session.sync_received(new Uint8Array(event.data), this.now());
        this.drain();
      }
    };
    const lost = () => {
      if (current()) {
        this.generation += 1;
        this.socket = null;
        session.sync_disconnected(this.now());
        this.drain();
      }
    };
    socket.onclose = lost;
    socket.onerror = lost;
  }

  private send(session: SyncSessionPort, bytes: Uint8Array): void {
    const socket = this.socket;
    if (!socket) {
      return;
    }
    try {
      socket.send(bytes);
    } catch {
      this.dropSocket();
      session.sync_disconnected(this.now());
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

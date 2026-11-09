// Cooperative editing in the page (`specs/0092-cooperative-editing`, through a shared workspace
// since `specs/0105`): the WebSocket transport and its 50 ms timer, the two settings, the device
// keys from the host, and the Share tab's commands.
// Every rule (when to connect, what is sent, what the tab says) is the wasm session's; this hook
// carries bytes, time, entropy and keys across.

import { useCallback, useEffect, useRef, useState } from "react";

import type { EditorSession } from "@/hooks/useEditorSession";
import { SyncTransport, type SocketLike } from "@/lib/syncTransport";
import { onWorkspaceLinkMoved } from "@/lib/workspaceLink";
import { invoke } from "@/platform/host";
import {
  invite_expiry_text,
  normalize_server_address,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One row of the participant list. */
export interface ShareParticipant {
  /** The member id as hex, the handle of Remove. */
  member: string;
  label: string;
  initials: string;
  colour: number;
  fingerprint: string;
  self: boolean;
  online: boolean;
  admin: boolean;
  /** Whether this device may remove the participant (an admin, not the own row). */
  removable: boolean;
  /** `specs/0097` criteria 2 and 14: the role slot's and the own row's actions. */
  canMakeAdmin?: boolean;
  canMakeMember?: boolean;
  canStepDown?: boolean;
  canLeave?: boolean;
  /** An own act on this row waits for the server (`membership_word` name), else null. */
  pending?: string | null;
}

/** The Share tab and the pill, as the session describes them. */
export interface ShareView {
  shared: boolean;
  live: boolean;
  server?: string | null;
  serverShown?: string;
  state?: string;
  stateLine?: string;
  pillText?: string;
  pillName?: string;
  subject: string;
  epoch?: number | null;
  admin?: boolean;
  online?: boolean;
  participants: ShareParticipant[];
  join?: { phase: string; text: string } | null;
  ended?: string | null;
  /** The participants grouped by person for display (`specs/0105` criterion 14). */
  people?: SharePerson[];
  /** The shared workspace's link (`specs/0095-workspace`). */
  workspace?: WorkspaceShareView | null;
}

/** One person of the people view: its line and its devices' member ids. */
export interface SharePerson {
  line: string;
  you: boolean;
  members: string[];
}

/** The shared workspace as the Share tab and the File tab's header line show it. */
export interface WorkspaceShareView {
  id: string;
  name: string;
  headerLine: string;
  receiving: string | null;
  openInWorkspace: boolean;
  documentLine: string;
  people: string;
  joining: boolean;
}

/** The invite on screen. */
export interface InviteShown {
  link: string;
  expiry: string;
}

/** Whether the server answered the last check. */
export type Reach = "idle" | "reaching" | "reachable" | "unreachable";

/** The passphrase of the key file, asked only on a system without a keychain (ADR 0007 §1). */
export interface PassphraseAsk {
  exists: boolean;
  wrong: boolean;
}

export interface CollaborationApi {
  view: ShareView;
  server: string;
  name: string;
  /** Sets the server address; the refusal's sentence, or "" when it was taken. */
  setServer: (text: string) => string;
  /** Sets the name shown to others; the refusal's sentence, or "". */
  setName: (text: string) => string;
  reach: Reach;
  checkServer: () => void;
  /** The open document's or workspace's link reconnects at once (Try again). */
  reconnect: () => void;
  /** Leaves the room (Cancel while joining, or a share that could not reach the server). */
  leave: () => void;
  invite: InviteShown | null;
  /** Writes an invite valid for 24 hours (Rust); "" when done, else why not. */
  createInvite: () => string;
  clearInvite: () => void;
  /** Removes a participant with a key rotation; "" when done, else the sentence why not. */
  remove: (member: string) => string;
  passphrase: PassphraseAsk | null;
  submitPassphrase: (text: string) => void;
  /** This device's identity and the epoch keys of `room` (hex), `null` while the passphrase is
   * asked (then `retry` runs once it is given) or the store failed (the reason is shown). */
  deviceKeys: (room: string, retry: () => void) => Promise<DeviceKeys | null>;
  /** Carries out what `session` just queued on the transport (a shared workspace's start). */
  drive: (session: SessionLike) => void;
}

/** This device's identity (hex) and a room's epoch keys. */
export interface DeviceKeys {
  identity: string;
  keys: [number, string][];
}

/** A session as the transport drives it. */
export type SessionLike = Parameters<SyncTransport["attach"]>[0];

const SERVER_KEY = "curvyo.share-server";
const NAME_KEY = "curvyo.share-name";
/** Other participants' labels by member id, kept per data directory (adrs.md decision 5). */
const LABELS_KEY = "curvyo.share-labels";
const TICK_MS = 50;

function stored(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function store(key: string, value: string) {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // A refused write keeps the value for this run only.
  }
}

/** The kept labels as the session reads them, `{}` when none or unreadable. */
function keptLabels(): Record<string, string> {
  try {
    const parsed: unknown = JSON.parse(stored(LABELS_KEY) || "{}");
    return parsed && typeof parsed === "object" ? (parsed as Record<string, string>) : {};
  } catch {
    return {};
  }
}

/** Keeps the labels the session learned this run next to the earlier ones. */
function keepLabels(learned: string): void {
  try {
    const next = { ...keptLabels(), ...(JSON.parse(learned) as Record<string, string>) };
    const text = JSON.stringify(next);
    if (text !== stored(LABELS_KEY)) {
      store(LABELS_KEY, text);
    }
  } catch {
    // An unreadable answer keeps the stored labels.
  }
}

type KeysAnswer =
  | { status: "ok"; identity: string; keys: [number, string][] }
  | { status: "needs-passphrase"; exists: boolean }
  | { status: "wrong-passphrase" }
  | { status: "failed"; message: string };

/** Carries out what `session` just queued. The transport serves `session` first: a join
 * replaced the project's session since the last tick, and the replaced one is freed. */
function carry(transport: SyncTransport | null, session: Parameters<SyncTransport["attach"]>[0]): void {
  transport?.attach(session);
  transport?.drain();
}

/** How long a reachability probe waits for the socket to open (ms, ADR 0044 decision 9). */
const PROBE_GIVE_UP_MS = 4000;

const NO_VIEW: ShareView = { shared: false, live: false, subject: "Not shared", participants: [] };

export function useCollaboration(editor: EditorSession): CollaborationApi {
  const { getSession } = editor;
  const showNotice = editor.showHistoryNotice;
  const [server, setServerState] = useState(() => stored(SERVER_KEY));
  const [name, setNameState] = useState(() => stored(NAME_KEY));
  const nameRef = useRef(name);
  const [view, setView] = useState<ShareView>(NO_VIEW);
  const [reach, setReach] = useState<Reach>("idle");
  const [invite, setInvite] = useState<InviteShown | null>(null);
  const [passphrase, setPassphrase] = useState<PassphraseAsk | null>(null);
  const passphraseRef = useRef<string | null>(null);
  /** What waits for the passphrase. */
  const pendingRef = useRef<(() => void) | null>(null);
  const transportRef = useRef<SyncTransport | null>(null);
  transportRef.current ??= new SyncTransport(
    (address) => new WebSocket(address) as unknown as SocketLike,
    undefined,
    (session) => (session as unknown) === getSession(),
  );

  /** This device's identity and the epoch keys of `room`, or `null` while a passphrase is
   * asked or the store failed (the reason is shown). */
  const keysFor = useCallback(async (room: string, retry: () => void) => {
    const answer = await invoke<KeysAnswer>("read_device_keys", {
      room,
      passphrase: passphraseRef.current,
    }).catch((error: unknown): KeysAnswer => ({ status: "failed", message: String(error) }));
    if (answer.status === "ok") {
      setPassphrase(null);
      return answer;
    }
    if (answer.status === "needs-passphrase" || answer.status === "wrong-passphrase") {
      pendingRef.current = retry;
      setPassphrase({
        exists: answer.status === "wrong-passphrase" || answer.exists,
        wrong: answer.status === "wrong-passphrase",
      });
      return null;
    }
    showNotice(`The device keys are not available: ${answer.message}`, 8000, "canvas", "alert");
    return null;
  }, [showNotice]);

  // The timer: the session's clock, the transport, and what the tab and the canvas show.
  useEffect(() => {
    const transport = transportRef.current;
    let lastSession: unknown = null;
    let revision = -1;
    const timer = window.setInterval(() => {
      const session = getSession();
      transport?.attach(session);
      if (session !== lastSession) {
        lastSession = session;
        revision = -1;
        setInvite(null);
        session?.remember_share_labels(JSON.stringify(keptLabels()));
      }
      if (!session) {
        return;
      }
      transport?.tick();
      const next = session.share_revision();
      if (next !== revision) {
        revision = next;
        const shown = JSON.parse(session.share_view_json()) as ShareView;
        setView(shown);
        keepLabels(session.share_labels_json());
      }
      for (let notice = session.take_sync_notice(); notice !== ""; notice = session.take_sync_notice()) {
        const { text, alert } = JSON.parse(notice) as { text: string; alert: boolean };
        showNotice(text, alert ? 8000 : 3000, "canvas", alert ? "alert" : "status");
      }
      const contested = session.take_contested_notice();
      if (contested !== "") {
        showNotice(contested, 3000, "canvas", "status");
      }
      const keys = session.take_key_update();
      if (keys !== "") {
        const { room, keys: list } = JSON.parse(keys) as { room: string; keys: [number, string][] };
        void invoke("write_document_keys", { room, keys: list }).catch(() =>
          showNotice("The document keys could not be stored.", 8000, "canvas", "alert"),
        );
      }
    }, TICK_MS);
    // A network that comes back reconnects at once, not after the backoff (criterion 18).
    const onOnline = () => transport?.networkBack();
    window.addEventListener("online", onOnline);
    // A shared workspace's link moves to the next document's session with its socket.
    const stopHandOver = onWorkspaceLinkMoved((next) => {
      transport?.handOver(next as unknown as Parameters<SyncTransport["handOver"]>[0]);
    });
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("online", onOnline);
      stopHandOver();
      transport?.attach(null);
    };
  }, [showNotice, getSession]);

  // The window title says "(shared)" while the open document is (`specs/0095` criterion 25).
  const sharedNow = view.shared && view.live;
  useEffect(() => {
    void invoke("set_shared", { shared: sharedNow }).catch(() => undefined);
  }, [sharedNow]);

  /** One probe at a time; the address the shown answer is for. */
  const probingRef = useRef(false);
  const reachForRef = useRef("");
  /** Opens a WebSocket to the server and closes it as soon as it opens, nothing sent (ADR 0044
   * decision 9); given up after 4 s. A repeat keeps the last answer shown until it has its own. */
  const checkServer = useCallback(() => {
    const address = normalize_server_address(server);
    if (address === "") {
      reachForRef.current = "";
      setReach("idle");
      return;
    }
    if (probingRef.current) {
      return;
    }
    probingRef.current = true;
    if (reachForRef.current !== address) {
      reachForRef.current = address;
      setReach("reaching");
    }
    let settled = false;
    const settle = (result: Reach) => {
      if (!settled) {
        settled = true;
        probingRef.current = false;
        if (reachForRef.current === address) {
          setReach(result);
        }
      }
    };
    try {
      const socket = new WebSocket(address);
      const timeout = window.setTimeout(() => {
        socket.close();
        settle("unreachable");
      }, PROBE_GIVE_UP_MS);
      socket.onopen = () => {
        window.clearTimeout(timeout);
        socket.close();
        settle("reachable");
      };
      socket.onerror = () => {
        window.clearTimeout(timeout);
        settle("unreachable");
      };
    } catch {
      settle("unreachable");
    }
  }, [server]);

  const setServer = useCallback((text: string) => {
    const address = normalize_server_address(text);
    if (address === "") {
      return "Not a server address.";
    }
    store(SERVER_KEY, address);
    setServerState(address);
    return "";
  }, []);

  const setName = useCallback(
    (text: string) => {
      const trimmed = text.trim();
      if (trimmed === "" || [...trimmed].length > 24) {
        return "Enter a name.";
      }
      store(NAME_KEY, trimmed);
      nameRef.current = trimmed;
      setNameState(trimmed);
      getSession()?.set_share_label(trimmed);
      return "";
    },
    [getSession],
  );

  const createInvite = useCallback(
    () => {
      const session = getSession();
      if (!session) {
        return "";
      }
      const answer = JSON.parse(session.create_invite(Date.now())) as
        | { link: string; expires: number }
        | { error: string };
      carry(transportRef.current, session);
      if ("error" in answer) {
        return answer.error;
      }
      const offset = -new Date().getTimezoneOffset();
      setInvite({ link: answer.link, expiry: invite_expiry_text(answer.expires, offset) });
      return "";
    },
    [getSession],
  );

  const remove = useCallback(
    (member: string) => {
      const session = getSession();
      if (!session) {
        return "";
      }
      const error = session.remove_participant(member);
      carry(transportRef.current, session);
      return error;
    },
    [getSession],
  );

  const leave = useCallback(() => {
    const session = getSession();
    session?.leave_sync();
    carry(transportRef.current, session);
  }, [getSession]);

  const drive = useCallback((session: SessionLike) => carry(transportRef.current, session), []);

  const submitPassphrase = useCallback((text: string) => {
    passphraseRef.current = text;
    const pending = pendingRef.current;
    pendingRef.current = null;
    pending?.();
  }, []);

  return {
    view,
    server,
    name,
    setServer,
    setName,
    reach,
    checkServer,
    reconnect: () => transportRef.current?.networkBack(),
    leave,
    invite,
    createInvite,
    clearInvite: () => setInvite(null),
    remove,
    passphrase,
    submitPassphrase,
    deviceKeys: keysFor,
    drive,
  };
}

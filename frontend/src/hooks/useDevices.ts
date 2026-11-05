// My devices in the page (`specs/0096-own-devices-sync`, ADR 0040): the device group's session
// and its socket, one pairing attempt and its socket, the files and keys the host keeps, and the
// open workspace's reconciliation with the group; the synced workspaces that are not open run in
// `useSyncedWorkspaces`. Every rule and word is Rust's; this carries bytes, time, entropy and keys
// across.

import { useCallback, useEffect, useRef, useState } from "react";

import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { WorkspaceApi } from "@/hooks/useWorkspace";
import { type OfferedWorkspace, useSyncedWorkspaces } from "@/hooks/useSyncedWorkspaces";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import type { DevicesApi, DevicesView, PairingView, SyncedFolder } from "@/lib/devicesApi";
import { openRoomNow, setGroupWorkspaces } from "@/lib/backgroundWorkspace";
import { mintPeerId, wasmLoaded, wasmReady } from "@/lib/editorSession";
import { SyncTransport, type SocketLike, type SyncSessionPort } from "@/lib/syncTransport";
import { onStorageFull, workspaceNames } from "@/platform/browserWorkspaces";
import { invoke, isTauri } from "@/platform/host";
import {
  WasmDeviceGroup,
  WasmPairing,
  devices_browser_storage,
  devices_browser_storage_full,
  devices_browser_text,
  devices_text,
  membership_word,
  devices_synced_folder_refusal,
  normalize_server_address,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const TICK_MS = 50;
/** How often the files, keys, reconciliation and arrivals are looked at (ms). */
const SLOW_MS = 1000;
const DEVICE_NAME_KEY = "curvyo.device-name";
/** How long the switch says "Leaving..." at most before it is usable again (ux-notes 5.4). */
const LEAVING_MS = 30_000;

export type {
  DeviceDetail,
  DeviceRow,
  DevicesApi,
  DevicesView,
  GroupWorkspace,
  PairingView,
  SyncedFolder,
} from "@/lib/devicesApi";

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

interface GroupPayload {
  server: string;
  room: string;
  server_seq: number;
  synced_version: string;
  synced_folder: string;
  folders: Record<string, string>;
  keyring_log: number[];
  list: number[];
}

function transport(isLive: (session: SyncSessionPort) => boolean): SyncTransport {
  return new SyncTransport((address) => new WebSocket(address) as unknown as SocketLike, undefined, isLive);
}

export function useDevices(
  editor: EditorSession,
  collab: CollaborationApi,
  workspace: WorkspaceApi,
  workspaceSync: WorkspaceSyncApi,
): DevicesApi {
  const { getSession } = editor;
  const showNotice = editor.showHistoryNotice;
  const refs = useRef({ collab, workspace, workspaceSync });
  useEffect(() => {
    refs.current = { collab, workspace, workspaceSync };
  });
  const groupRef = useRef<WasmDeviceGroup | null>(null);
  const pairingRef = useRef<WasmPairing | null>(null);
  const groupTransport = useRef<SyncTransport | null>(null);
  const pairTransport = useRef<SyncTransport | null>(null);
  groupTransport.current ??= transport((s) => (s as unknown) === groupRef.current);
  pairTransport.current ??= transport((s) => (s as unknown) === pairingRef.current);
  /** The group's server, Synced workspaces folder and where each synced workspace is here. */
  const recordRef = useRef({ server: "", syncedFolder: "", folders: {} as Record<string, string> });
  /** The paired device the opener adds, until its `AddMember` went out. */
  const addingRef = useRef<Uint8Array | null>(null);
  /** The record's folders changed since the group's files were written. */
  const foldersDirtyRef = useRef(false);
  /** Whether the pairing on screen is this device's code (Add a device). */
  const openerRef = useRef(false);
  const pendingSwitchRef = useRef<"" | "on" | "off">("");
  const [view, setView] = useState<DevicesView | null>(null);
  const viewRef = useRef<DevicesView | null>(null);
  useEffect(() => {
    viewRef.current = view;
    // A workspace the maker left ends as a leave with its record, not as a switch-off.
    const shown = view && !view.removed ? view : null;
    setGroupWorkspaces(
      shown ? shown.workspaces.filter((w) => !w.left) : [],
      shown ? shown.rows.map((row) => row.member).filter((m) => m !== "") : [],
    );
  });
  const [pairing, setPairing] = useState<{ view: PairingView; opener: boolean } | null>(null);
  const [deviceName, setDeviceNameState] = useState(() => stored(DEVICE_NAME_KEY));
  /** The name as set last, for a pairing started in the same press (Connect after typing it). */
  const deviceNameRef = useRef(deviceName);
  useEffect(() => {
    deviceNameRef.current = deviceName;
  }, [deviceName]);
  const [syncedFolder, setSyncedFolder] = useState<SyncedFolder>({ path: "", refusal: "" });
  const [switching, setSwitching] = useState<"" | "on" | "off" | "leave">("");
  /** When the person leave's mark went out (0 when none runs): "Leaving..." until it ends. */
  const leavingRef = useRef(0);
  /** The last switch-on failed (ux-notes 6.2: the alert under the switch). */
  const [switchFailed, setSwitchFailed] = useState(false);
  const [removedShown, setRemovedShown] = useState(false);
  const [justPaired, setJustPaired] = useState("");
  const [loaded, setLoaded] = useState<{ label: string; rows: OfferedWorkspace[] }>({ label: "", rows: [] });

  /** The open shared workspace's id (hex), or "". */
  const openWorkspaceId = useCallback((): string => {
    const shown = refs.current.collab.view.workspace;
    return shown && !shown.joining ? shown.id : "";
  }, []);

  const synced = useSyncedWorkspaces({
    group: () => groupRef.current,
    record: recordRef,
    recordChanged: () => {
      foldersDirtyRef.current = true;
    },
    label: () => refs.current.collab.name,
    keys: (room) => refs.current.collab.deviceKeys(room, () => undefined),
    notice: (text, alert) => showNotice(text, alert ? 8000 : 5000, "canvas", alert ? "alert" : "status"),
    openId: openWorkspaceId,
    workspaceOpen: () => refs.current.workspace.open,
    openSession: getSession,
    ownMember: () => viewRef.current?.rows.find((row) => row.own)?.member ?? "",
    recentsChanged: () => refs.current.workspace.reloadRecents(),
  });
  const syncedRef = useRef(synced);
  useEffect(() => {
    syncedRef.current = synced;
  });

  // The default device name is the host name (criterion 2).
  useEffect(() => {
    if (deviceName !== "") {
      return;
    }
    void invoke<string>("device_host_name")
      .then((name) => {
        setDeviceNameState(name);
        store(DEVICE_NAME_KEY, name);
      })
      .catch(() => undefined);
  }, [deviceName]);

  const identity = useCallback(async (): Promise<Uint8Array | null> => {
    const keys = await refs.current.collab.deviceKeys("", () => undefined);
    return keys ? fromHex(keys.identity) : null;
  }, []);

  const attachGroup = useCallback((group: WasmDeviceGroup | null) => {
    const old = groupRef.current;
    groupRef.current = group;
    groupTransport.current?.attach(group);
    if (old && old !== group) {
      old.free();
    }
    if (group && !isTauri) {
      // The other devices show this one as a browser (ADR 0040 decision 1).
      group.set_browser();
    }
    if (!group) {
      setView(null);
    }
  }, []);

  /** Writes the group's files and keys when they changed. */
  const persist = useCallback(async (group: WasmDeviceGroup) => {
    const keys = group.take_key_update();
    if (keys) {
      await invoke("write_document_keys", { room: group.room(), keys: JSON.parse(keys) as [number, string][] }).catch(
        () => undefined,
      );
    }
    // The group may have been let go (removed, left) and freed while the keys were written.
    if (groupRef.current !== group) {
      return;
    }
    const files = group.take_files_json(foldersDirtyRef.current);
    if (!files) {
      return;
    }
    foldersDirtyRef.current = false;
    const parsed = JSON.parse(files) as { keyring_log: number[]; list: number[]; server_seq: number; synced_version: string };
    const record = recordRef.current;
    const payload: GroupPayload = {
      server: record.server,
      room: group.room(),
      server_seq: parsed.server_seq,
      synced_version: parsed.synced_version,
      synced_folder: record.syncedFolder,
      folders: record.folders,
      keyring_log: parsed.keyring_log,
      list: parsed.list,
    };
    await invoke("write_device_group", { group: payload }).catch(() => undefined);
  }, []);

  // A browser profile is one device: only the tab holding the lock runs it (criterion 39).
  const [deviceTab, setDeviceTab] = useState<"mine" | "other" | "pending">(() =>
    isTauri || !(navigator as Navigator & { locks?: LockManager }).locks ? "mine" : "pending",
  );
  const [persisted, setPersisted] = useState(true);
  const [storageLine, setStorageLine] = useState("");
  useEffect(() => {
    if (isTauri) {
      return undefined;
    }
    const locks = (navigator as Navigator & { locks?: LockManager }).locks;
    if (!locks) {
      return undefined;
    }
    let release: (() => void) | null = null;
    void locks.request("curvyo-device", { ifAvailable: true }, (lock) => {
      if (!lock) {
        setDeviceTab("other");
        return undefined;
      }
      setDeviceTab("mine");
      // Held while the page lives.
      return new Promise<void>((resolve) => {
        release = resolve;
      });
    });
    void navigator.storage?.persisted?.().then(setPersisted, () => undefined);
    return () => release?.();
  }, []);

  /** The browser asks to keep its data at pairing and shows the answer (ADR 0040 decision 9). */
  const askPersist = useCallback(() => {
    if (!isTauri) {
      void navigator.storage?.persist?.().then(setPersisted, () => setPersisted(false));
    }
  }, []);

  // The browser's storage: what it holds of what it may hold, and an alert when it is full
  // (criterion 37).
  useEffect(() => {
    if (isTauri || deviceTab !== "mine") {
      return undefined;
    }
    const measure = async () => {
      const estimate = await navigator.storage?.estimate?.().catch(() => null);
      return { used: estimate?.usage ?? 0, quota: estimate?.quota ?? 0 };
    };
    const refresh = async () => {
      const { used, quota } = await measure();
      // The words are Rust's: the module may still be loading at the first measure.
      await wasmReady();
      setStorageLine(devices_browser_storage(used, quota, JSON.stringify(await workspaceNames().catch(() => []))));
    };
    void refresh();
    const timer = window.setInterval(() => void refresh(), 10_000);
    const stop = onStorageFull((name) => {
      void measure().then(({ used, quota }) =>
        showNotice(devices_browser_storage_full(name, used, quota), 8000, "canvas", "alert"),
      );
    });
    return () => {
      window.clearInterval(timer);
      stop();
    };
  }, [deviceTab, showNotice]);

  // The group as this device saved it reconnects at start.
  useEffect(() => {
    if (deviceTab !== "mine") {
      return undefined;
    }
    let cancelled = false;
    void (async () => {
      const saved = await invoke<GroupPayload | null>("read_device_group").catch(() => null);
      if (!saved || cancelled) {
        return;
      }
      const keys = await refs.current.collab.deviceKeys(saved.room, () => undefined);
      if (!keys || cancelled) {
        return;
      }
      recordRef.current = { server: saved.server, syncedFolder: saved.synced_folder, folders: saved.folders };
      const group = WasmDeviceGroup.resume(
        saved.server,
        saved.room,
        new Uint8Array(saved.keyring_log),
        new Uint8Array(saved.list),
        saved.server_seq,
        JSON.stringify(keys.keys),
        fromHex(keys.identity),
        random(32),
        random(32),
        mintPeerId(),
        Date.now(),
      );
      if (group) {
        attachGroup(group);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [attachGroup, deviceTab]);

  /** Reconciles the open workspace with the group, and finishes a pending switch. */
  const reconcileOpen = useCallback(() => {
    const group = groupRef.current;
    const session = getSession();
    const id = openWorkspaceId();
    if (
      leavingRef.current !== 0 &&
      (!session || id === "" || session.workspace_removed() || Date.now() - leavingRef.current > LEAVING_MS)
    ) {
      // The person leave ended here (or gave up waiting): the switch is no longer busy.
      leavingRef.current = 0;
      setSwitching((now) => (now === "leave" ? "" : now));
    }
    if (pendingSwitchRef.current === "on" && refs.current.workspaceSync.share.kind === "failed") {
      // A failed Share ends the switch-on; the switch row says why.
      pendingSwitchRef.current = "";
      setSwitching("");
      setSwitchFailed(true);
      return;
    }
    if (!group || !session || id === "" || !group.is_online()) {
      return;
    }
    if (group.workspace_left(id)) {
      // `specs/0097` criterion 16: the maker left it from every device. The leaving device waits
      // for the relay to store the mark first; then every device runs the left plan.
      if (pendingSwitchRef.current !== "") {
        // A switch pressed meanwhile ends: the leave goes on (no busy switch is left behind).
        pendingSwitchRef.current = "";
        setSwitching("");
      }
      if (group.list_acked()) {
        session.reconcile_left_workspace(group);
        refs.current.collab.drive(session);
      }
      return;
    }
    const pending = pendingSwitchRef.current;
    if (pending === "off" && group.is_switched_on(id)) {
      // Off: the list entry first, then the devices leave (ADR 0040 decision 6).
      group.switch_workspace(id, refs.current.workspace.name, false);
    }
    const on = pending === "on" || group.is_switched_on(id);
    const answer = session.reconcile_workspace(group, on);
    refs.current.collab.drive(session);
    if (pending === "on" && answer !== "") {
      // On: the devices were admitted, then the list entry (criterion 11).
      group.switch_workspace(id, refs.current.workspace.name, true);
    }
    if (group.is_switched_on(id) && !recordRef.current.folders[id]) {
      // Where the open synced workspace lives here, so it syncs in the background once closed.
      recordRef.current.folders = { ...recordRef.current.folders, [id]: "" };
      void invoke<string>("register_synced_workspace", { id, path: "" }).then(
        (path) => {
          recordRef.current.folders = { ...recordRef.current.folders, [id]: path };
          foldersDirtyRef.current = true;
        },
        () => undefined,
      );
    }
    if (pending !== "" && answer !== "") {
      pendingSwitchRef.current = "";
      setSwitching("");
    }
  }, [getSession, openWorkspaceId]);

  // The timer: both transports, the views, and every second the files, reconciliation and
  // arrivals.
  useEffect(() => {
    let groupRevision = -1;
    let pairRevision = -1;
    let lastFacts = "";
    let slow = 0;
    const timer = window.setInterval(() => {
      const group = groupRef.current;
      const attempt = pairingRef.current;
      groupTransport.current?.attach(group);
      pairTransport.current?.attach(attempt);
      groupTransport.current?.tick();
      pairTransport.current?.tick();
      if (attempt && attempt.revision() !== pairRevision) {
        pairRevision = attempt.revision();
        const shown = JSON.parse(attempt.view_json()) as PairingView;
        if (shown.phase === "paired" && openerRef.current) {
          // The opener's side is done: the block closes and the workspaces are chosen (3.1).
          setJustPaired(shown.label ?? "");
          pairingRef.current = null;
          pairTransport.current?.attach(null);
          attempt.free();
          setPairing(null);
          return;
        }
        setPairing((current) => ({ view: shown, opener: current?.opener ?? false }));
      }
      if (attempt && addingRef.current && groupRef.current?.has_device(addingRef.current)) {
        // The joiner is a member now: the welcome names the group room (criterion 3).
        addingRef.current = null;
        attempt.pair(groupRef.current.room());
      }
      syncedRef.current.tick(Date.now());
      if (group) {
        // The workspaces' facts are read once a second (the slow branch).
        if (slow + TICK_MS >= SLOW_MS) {
          lastFacts = syncedRef.current.facts();
          groupRevision = -1;
        }
        if (group.revision() !== groupRevision) {
          const facts = lastFacts === "" ? "[]" : lastFacts;
          groupRevision = group.revision();
          setView(JSON.parse(group.view_json(-new Date().getTimezoneOffset(), facts)) as DevicesView);
          if (group.removed()) {
            setRemovedShown(true);
          }
        }
        for (let notice = group.take_notice(); notice !== ""; notice = group.take_notice()) {
          showNotice(notice, 5000, "canvas", "status");
        }
      }
      slow += TICK_MS;
      if (slow < SLOW_MS) {
        return;
      }
      slow = 0;
      if (group) {
        const ended = openRoomNow();
        if (ended !== "" && getSession()?.workspace_removed() && recordRef.current.folders[ended] !== undefined) {
          // The open workspace was switched off: its copy stays a local workspace, and switched
          // on again it is offered to connect (ux-notes 6.4), as a background one is.
          const folders = { ...recordRef.current.folders };
          delete folders[ended];
          recordRef.current.folders = folders;
          foldersDirtyRef.current = true;
        }
        void persist(group);
        reconcileOpen();
        syncedRef.current.slow();
        if (group.removed()) {
          void syncedRef.current.stopAll();
          void invoke("remove_device_group").catch(() => undefined);
          attachGroup(null);
        }
      }
    }, TICK_MS);
    const onOnline = () => {
      groupTransport.current?.networkBack();
      syncedRef.current.networkBack();
    };
    window.addEventListener("online", onOnline);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("online", onOnline);
    };
  }, [attachGroup, getSession, persist, reconcileOpen, showNotice]);

  const setDeviceName = useCallback((text: string) => {
    const trimmed = text.trim();
    if (trimmed === "") {
      return "Enter a name.";
    }
    if (trimmed.length > 24) {
      return "Use at most 24 characters.";
    }
    setDeviceNameState(trimmed);
    deviceNameRef.current = trimmed;
    store(DEVICE_NAME_KEY, trimmed);
    groupRef.current?.set_own_label(trimmed);
    return "";
  }, []);

  const startPairing = useCallback((attempt: WasmPairing | undefined, opener: boolean) => {
    if (!attempt) {
      return;
    }
    pairingRef.current?.free();
    pairingRef.current = attempt;
    openerRef.current = opener;
    pairTransport.current?.attach(attempt);
    setPairing({ view: JSON.parse(attempt.view_json()) as PairingView, opener });
    pairTransport.current?.drain();
  }, []);

  const addDevice = useCallback(() => {
    void (async () => {
      const id = await identity();
      const server = normalize_server_address(groupRef.current ? recordRef.current.server : refs.current.collab.server);
      if (!id || server === "") {
        return;
      }
      startPairing(WasmPairing.open(server, id, deviceName, random(32), Date.now()), true);
    })();
  }, [deviceName, identity, startPairing]);

  const joinDevices = useCallback(
    (code: string) => {
      void (async () => {
        const id = await identity();
        const server = normalize_server_address(refs.current.collab.server);
        if (!id || server === "") {
          return;
        }
        let folder = syncedFolder.path;
        if (folder === "") {
          const answer = await invoke<{ kind: string; path?: string; code?: string }>("prepare_synced_folder", {
            path: "",
          }).catch((): { kind: string; path?: string; code?: string } => ({ kind: "refused", code: "not-writable" }));
          if (answer.kind !== "picked") {
            setSyncedFolder({ path: "", refusal: devices_synced_folder_refusal(answer.code ?? "not-writable") });
            return;
          }
          folder = answer.path ?? "";
          setSyncedFolder({ path: folder, refusal: "" });
        }
        recordRef.current = { server, syncedFolder: folder, folders: {} };
        askPersist();
        startPairing(WasmPairing.join(server, code, id, deviceNameRef.current, random(32), Date.now()), false);
      })();
    },
    [askPersist, identity, startPairing, syncedFolder.path],
  );

  // The joiner's pairing finished: it joins the group room (criterion 3).
  useEffect(() => {
    const attempt = pairingRef.current;
    if (!pairing || pairing.opener || pairing.view.phase !== "paired" || !attempt || groupRef.current) {
      return;
    }
    void (async () => {
      const id = await identity();
      if (!id) {
        return;
      }
      const group = WasmDeviceGroup.join(
        recordRef.current.server,
        attempt.group_room(),
        deviceName,
        id,
        random(32),
        random(32),
        mintPeerId(),
        Date.now(),
      );
      if (group) {
        attachGroup(group);
      }
      // The new device's block closes; the list shows both devices (ux-notes 4.4).
      pairingRef.current = null;
      pairTransport.current?.attach(null);
      attempt.free();
      setPairing(null);
    })();
  }, [attachGroup, deviceName, identity, pairing]);

  const restoreGroup = useCallback(
    async (server: string, room: string, label: string) => {
      const id = await identity();
      if (!id || groupRef.current) {
        return;
      }
      recordRef.current = { server, syncedFolder: "browser", folders: {} };
      askPersist();
      if (label !== "") {
        setDeviceNameState(label);
        store(DEVICE_NAME_KEY, label);
      }
      const group = WasmDeviceGroup.join(server, room, label || deviceName, id, random(32), random(32), mintPeerId(), Date.now());
      if (group) {
        attachGroup(group);
      }
    },
    [askPersist, attachGroup, deviceName, identity],
  );

  const pair = useCallback(() => {
    const attempt = pairingRef.current;
    if (!attempt) {
      return;
    }
    void (async () => {
      const id = await identity();
      if (!id) {
        return;
      }
      attempt.mark_pairing();
      let group = groupRef.current;
      if (!group) {
        const server = normalize_server_address(refs.current.collab.server);
        recordRef.current = { server, syncedFolder: "", folders: {} };
        group =
          WasmDeviceGroup.create(server, random(16), deviceName, id, random(32), random(32), mintPeerId(), Date.now()) ??
          null;
        if (!group) {
          return;
        }
        attachGroup(group);
      }
      const peer = attempt.peer_identity();
      addingRef.current = peer;
      group.add_device(peer, attempt.peer_label());
    })();
  }, [attachGroup, deviceName, identity]);

  const cancelPairing = useCallback((wordsDiffer: boolean) => {
    const attempt = pairingRef.current;
    if (attempt) {
      attempt.cancel(wordsDiffer);
      pairTransport.current?.drain();
    }
    addingRef.current = null;
    pairingRef.current = null;
    pairTransport.current?.attach(null);
    attempt?.free();
    setPairing(null);
  }, []);

  const closePairing = useCallback(() => {
    const attempt = pairingRef.current;
    pairingRef.current = null;
    pairTransport.current?.attach(null);
    attempt?.free();
    setPairing(null);
  }, []);

  const chooseSyncedFolder = useCallback(() => {
    void invoke<{ kind: string; path?: string; code?: string }>("pick_synced_folder")
      .then((answer) => {
        if (answer.kind === "picked") {
          setSyncedFolder({ path: answer.path ?? "", refusal: "" });
        } else if (answer.kind === "refused") {
          setSyncedFolder({ path: "", refusal: devices_synced_folder_refusal(answer.code ?? "") });
        }
      })
      .catch(() => setSyncedFolder({ path: "", refusal: devices_synced_folder_refusal("not-writable") }));
  }, []);

  const remove = useCallback(
    (member: string) => {
      const group = groupRef.current;
      if (!group) {
        return "";
      }
      const refusal = group.remove_device(member);
      groupTransport.current?.drain();
      if (refusal === "") {
        // The open workspace drops the device at once (criterion 23).
        reconcileOpen();
      }
      return refusal;
    },
    [reconcileOpen],
  );

  const leave = useCallback(() => {
    const group = groupRef.current;
    if (!group) {
      return "";
    }
    const refusal = group.leave();
    groupTransport.current?.drain();
    if (refusal === "") {
      // Criterion 27: its own removal in every synced workspace too; the files stay.
      const session = getSession();
      if (session && group.is_switched_on(openWorkspaceId()) && session.leave_synced_workspace()) {
        refs.current.collab.drive(session);
      }
      void syncedRef.current.leaveAll();
      recordRef.current = { server: "", syncedFolder: "", folders: {} };
      void invoke("remove_device_group").catch(() => undefined);
      attachGroup(null);
    }
    return refusal;
  }, [attachGroup, getSession, openWorkspaceId]);

  /** Leave of the open synced workspace for the person (`specs/0097` criterion 16): the `left`
   * mark first; every device then runs the left plan once the relay stored it. */
  const leavePerson = useCallback((): string => {
    const group = groupRef.current;
    const session = getSession();
    const id = openWorkspaceId();
    if (!group || !session || id === "") {
      return "";
    }
    if (!group.is_online() || !session.sync_online()) {
      return membership_word("not-connected");
    }
    if (!session.person_leave_allowed(group)) {
      return membership_word("person-only-admin");
    }
    group.leave_workspace(id, refs.current.workspace.name);
    groupTransport.current?.drain();
    leavingRef.current = Date.now();
    setSwitching("leave");
    return "";
  }, [getSession, openWorkspaceId]);

  /** Clears the `left` mark of workspace `id` (Sync again, ux-notes 5.4): the entry is written
   * switched off without the mark, and this resolves once the relay stored it (`false` after
   * 10 s or offline: nothing connects then). */
  const clearLeft = useCallback((id: string, name: string): Promise<boolean> => {
    const group = groupRef.current;
    if (!group || !group.workspace_left(id)) {
      return Promise.resolve(true);
    }
    if (!group.is_online()) {
      return Promise.resolve(false);
    }
    group.switch_workspace(id, name, false);
    groupTransport.current?.drain();
    const started = Date.now();
    return new Promise((resolve) => {
      const timer = window.setInterval(() => {
        if (groupRef.current?.list_acked()) {
          window.clearInterval(timer);
          resolve(true);
        } else if (Date.now() - started > 10_000) {
          window.clearInterval(timer);
          resolve(false);
        }
      }, TICK_MS);
    });
  }, []);

  const switchOn = useCallback(() => {
    pendingSwitchRef.current = "on";
    setSwitching("on");
    setSwitchFailed(false);
    if (openWorkspaceId() === "") {
      // Not shared yet: Share as `0095` first (ADR 0040 decision 6); the switch finishes once
      // the workspace room is online.
      refs.current.workspaceSync.shareWorkspace();
    }
  }, [openWorkspaceId]);

  const switchOff = useCallback(() => {
    // Refused before anything is written while not connected (ux-notes 6.3).
    if (!groupRef.current?.is_online()) {
      return devices_text("not-connected");
    }
    pendingSwitchRef.current = "off";
    setSwitching("off");
    reconcileOpen();
    return "";
  }, [reconcileOpen]);


  // The workspaces offered right after Pair (ux-notes 3.1).
  useEffect(() => {
    if (justPaired === "") {
      return undefined;
    }
    let cancelled = false;
    void syncedRef.current.loadOffered().then((rows) => {
      if (!cancelled) {
        setLoaded({ label: justPaired, rows });
      }
    });
    return () => {
      cancelled = true;
    };
  }, [justPaired]);

  const syncChosen = useCallback(
    (chosen: OfferedWorkspace[]) => {
      if (chosen.some((c) => c.open)) {
        switchOn();
      }
      syncedRef.current.syncChosen(chosen);
    },
    [switchOn],
  );

  // The page renders once before the wasm module is ready; the words follow on the next render.
  const words = wasmLoaded();
  const browserServer = words ? normalize_server_address(collab.server) : "";
  const pairReason = !words
    ? ""
    : deviceTab === "other"
      ? devices_browser_text("other-tab")
      : !isTauri && browserServer !== "" && !browserServer.startsWith("wss://")
        ? devices_browser_text("needs-tls")
        : "";
  const browserLines = isTauri || !words
    ? []
    : [
        devices_browser_text("own-row"),
        ...(persisted ? [] : [devices_browser_text("not-persisted")]),
        ...(storageLine ? [storageLine] : []),
      ];

  const openId = collab.view.workspace && !collab.view.workspace.joining ? collab.view.workspace.id : "";
  const openSynced = view !== null && openId !== "" && view.workspaces.some((w) => w.id === openId && w.on);
  const otherNames = view ? view.rows.filter((row) => !row.own).map((row) => row.label) : [];
  const shown = view && removedShown ? { ...view, removed: true } : view;
  return {
    view: removedShown && !view
      ? { room: "", online: false, removed: true, rows: [], count: "", workspaces: [], one_device_warning: "" }
      : shown,
    pairing,
    deviceName,
    setDeviceName,
    addDevice,
    joinDevices,
    pair,
    cancelPairing,
    closePairing,
    justPaired,
    clearJustPaired: () => setJustPaired(""),
    syncedFolder,
    chooseSyncedFolder,
    remove,
    leave,
    dismissRemoved: () => setRemovedShown(false),
    openSynced,
    switchOn,
    switchOff,
    leavePerson,
    clearLeft,
    leftIds: view ? view.workspaces.filter((w) => w.left).map((w) => w.id) : [],
    ownMembers: view && !view.removed ? view.rows.map((row) => row.member).filter((m) => m !== "") : [],
    switching,
    switchFailed,
    otherNames,
    offered: loaded.label === justPaired && justPaired !== "" ? loaded.rows : [],
    syncChosen,
    syncProgress: synced.progress,
    syncAlerts: synced.alerts,
    decisions: synced.decisions,
    connectCopy: synced.connectCopy,
    secondCopy: synced.secondCopy,
    browser: !isTauri,
    pairReason,
    browserLines,
    restoreGroup,
  };
}

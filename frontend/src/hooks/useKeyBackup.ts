// The browser's passkey key backup in the page (`specs/0096-own-devices-sync` M9, ADR 0040
// decision 13, Frage 145 A): Back up with passkey on a paired browser, Remove backup, and Restore
// with passkey on a browser without a group. The words and every byte rule are Rust's; this keeps
// the state of one run and talks to the passkey, the key store and the server.

import { useCallback, useEffect, useState } from "react";

import type { DevicesApi } from "@/hooks/useDevices";
import { browserKeyStore } from "@/lib/deviceKeyStore";
import {
  BACKUP_RECORD,
  type BackupMark,
  type BackupOutcome,
  askPasskey,
  makePasskey,
  passkeysAvailable,
  runExchange,
  sealNonce,
} from "@/lib/keyBackup";
import { invoke } from "@/platform/host";
import { WasmKeyBackup, key_backup_text, normalize_server_address } from "@/wasm-bindings/curvyo_editor_wasm.js";

export interface KeyBackupApi {
  /** Whether this browser holds a backup mark (a backup was stored from here). */
  backedUp: boolean;
  /** "Waiting for the passkey..." while a run is on, else "". */
  busy: string;
  /** The last run's sentence, and whether it is an alert. */
  message: { text: string; alert: boolean } | null;
  /** Whether this page can ask for a passkey at all; without one, the no-PRF sentence shows. */
  available: boolean;
  backUp: () => void;
  remove: () => void;
  /** After Leave my devices: deletes the backup on the server (the passkey is asked) and forgets
   * the mark either way, so a later pairing starts without a stale backup. */
  forget: () => void;
  /** Restores this device from the passkey's backup on `server` (a browser without a group). */
  restore: (server: string) => void;
}

interface GroupRecord {
  server: string;
  room: string;
}

function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(2 * i, 2 * i + 2), 16);
  }
  return bytes;
}

async function readMark(): Promise<BackupMark | null> {
  const bytes = await browserKeyStore.load(BACKUP_RECORD).catch(() => null);
  return bytes ? (JSON.parse(new TextDecoder().decode(bytes)) as BackupMark) : null;
}

function writeMark(mark: BackupMark): Promise<void> {
  return browserKeyStore.save(BACKUP_RECORD, new TextEncoder().encode(JSON.stringify(mark)));
}

function failure(outcome: BackupOutcome): string {
  return "text" in outcome ? outcome.text : key_backup_text("failed");
}

export function useKeyBackup(devices: DevicesApi): KeyBackupApi {
  const [backedUp, setBackedUp] = useState(false);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState<{ text: string; alert: boolean } | null>(null);
  const available = passkeysAvailable();

  useEffect(() => {
    if (devices.browser) {
      void readMark().then((mark) => setBackedUp(mark !== null));
    }
  }, [devices.browser, devices.view?.room]);

  /** Runs `work` once at a time; its answer becomes the message. */
  const run = useCallback(
    (work: () => Promise<{ text: string; alert: boolean }>) => {
      if (busy !== "") {
        return;
      }
      setMessage(null);
      setBusy(key_backup_text("waiting"));
      void work()
        .catch(() => ({ text: key_backup_text("failed"), alert: true }))
        .then((answer) => {
          setBusy("");
          setMessage(answer.text ? answer : null);
        });
    },
    [busy],
  );

  const backUp = useCallback(() => {
    run(async () => {
      const group = await invoke<GroupRecord | null>("read_device_group").catch(() => null);
      const identity = await browserKeyStore.load("identity");
      if (!group || !identity) {
        return { text: key_backup_text("failed"), alert: true };
      }
      // A passkey made for this device before is asked again: same record, no second passkey.
      const mark = await readMark();
      const asked = mark ? await askPasskey(new Uint8Array(mark.credential)) : await makePasskey(devices.deviceName);
      if (asked.kind !== "prf") {
        return { text: key_backup_text(asked.kind), alert: true };
      }
      setBusy(key_backup_text("working"));
      const backup = WasmKeyBackup.write(asked.prf, identity, group.room, group.server, devices.deviceName, sealNonce());
      if (!backup) {
        return { text: key_backup_text("failed"), alert: true };
      }
      const outcome = await runExchange(group.server, backup);
      if (outcome.kind !== "done") {
        return { text: failure(outcome), alert: true };
      }
      await writeMark({ server: group.server, credential: Array.from(asked.credential) });
      setBackedUp(true);
      return { text: key_backup_text("stored"), alert: false };
    });
  }, [devices.deviceName, run]);

  const remove = useCallback(() => {
    run(async () => {
      const mark = await readMark();
      if (!mark) {
        return { text: "", alert: false };
      }
      const asked = await askPasskey(new Uint8Array(mark.credential));
      if (asked.kind !== "prf") {
        return { text: key_backup_text(asked.kind), alert: true };
      }
      setBusy(key_backup_text("working"));
      const backup = WasmKeyBackup.delete(asked.prf);
      const outcome = backup ? await runExchange(mark.server, backup) : null;
      // A backup the server no longer has is removed as well.
      if (!outcome || (outcome.kind !== "done" && outcome.kind !== "refused-unknown")) {
        return { text: outcome ? failure(outcome) : key_backup_text("failed"), alert: true };
      }
      await browserKeyStore.remove(BACKUP_RECORD);
      setBackedUp(false);
      return { text: key_backup_text("removed"), alert: false };
    });
  }, [run]);

  const forget = useCallback(() => {
    void (async () => {
      const mark = await readMark();
      if (!mark) {
        return;
      }
      const asked = await askPasskey(new Uint8Array(mark.credential));
      const backup = asked.kind === "prf" ? WasmKeyBackup.delete(asked.prf) : null;
      if (backup) {
        await runExchange(mark.server, backup);
      }
      await browserKeyStore.remove(BACKUP_RECORD).catch(() => undefined);
      setBackedUp(false);
    })().catch(() => undefined);
  }, []);

  // A removed device's backup holds an identity the group no longer admits: the mark goes.
  useEffect(() => {
    if (devices.browser && devices.view?.removed) {
      void browserKeyStore
        .remove(BACKUP_RECORD)
        .catch(() => undefined)
        .then(() => setBackedUp(false));
    }
  }, [devices.browser, devices.view?.removed]);

  const { restoreGroup } = devices;
  const restore = useCallback(
    (server: string) => {
      run(async () => {
        const address = normalize_server_address(server);
        // A browser in a group keeps its identity: nothing is overwritten.
        const group = await invoke<GroupRecord | null>("read_device_group").catch(() => null);
        if (group) {
          return { text: key_backup_text("in-group"), alert: true };
        }
        const asked = await askPasskey();
        if (asked.kind !== "prf") {
          return { text: key_backup_text(asked.kind), alert: true };
        }
        setBusy(key_backup_text("working"));
        const backup = WasmKeyBackup.read(asked.prf);
        const outcome = backup ? await runExchange(address, backup) : null;
        if (!outcome || outcome.kind !== "read" || !("identity" in outcome)) {
          return { text: outcome ? failure(outcome) : key_backup_text("failed"), alert: true };
        }
        // The identity first: the group's join reads it from the store.
        await browserKeyStore.save("identity", fromHex(outcome.identity));
        await writeMark({ server: outcome.server, credential: Array.from(asked.credential) });
        setBackedUp(true);
        await restoreGroup(outcome.server, outcome.room, outcome.label);
        return { text: key_backup_text("restored"), alert: false };
      });
    },
    [restoreGroup, run],
  );

  return { backedUp, busy, message, available, backUp, remove, forget, restore };
}

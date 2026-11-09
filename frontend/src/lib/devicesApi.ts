// What the page's My devices hook gives the Share tab (`specs/0096-own-devices-sync`): the
// block's view as the group session describes it, the pairing view, and the actions.

import type { CopyDecision, OfferedWorkspace } from "@/hooks/useSyncedWorkspaces";
import { devices_synced_header } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** One synced workspace's line under a row (ux-notes 2.1, criterion 22). */
export interface DeviceDetail {
  name: string;
  state: string;
}

/** One row of My devices. */
export interface DeviceRow {
  member: string;
  label: string;
  fingerprint: string;
  own: boolean;
  /** The device is the app in a browser (it marked itself in the devices list). */
  browser?: boolean;
  seen: string;
  details?: DeviceDetail[];
  /** A removed device still in a workspace this device cannot remove it from (criterion 26). */
  removing?: boolean;
}

/** A workspace the devices list knows. */
export interface GroupWorkspace {
  id: string;
  name: string;
  on: boolean;
  /** The maker left it from every device (`specs/0097` criterion 16). */
  left: boolean;
}

/** The My devices block as the group session describes it. */
export interface DevicesView {
  room: string;
  online: boolean;
  removed: boolean;
  rows: DeviceRow[];
  count: string;
  workspaces: GroupWorkspace[];
  /** "Key renewal pending for 2 workspaces", or "" (criterion 27). */
  renewal?: string;
  /** Devices still in workspaces this device cannot remove them from (criterion 26). */
  alerts?: string[];
}

/** The pairing block as the pairing attempt describes it. */
export interface PairingView {
  phase: "connecting" | "code" | "joining" | "compare" | "pairing" | "paired" | "failed";
  code?: string;
  spelled?: string;
  valid?: string;
  /** "1 minute left" once, for the polite status; else "". */
  announce?: string;
  seconds_left?: number;
  wrong?: string;
  label?: string;
  fingerprint?: string;
  words?: string[];
  heading?: string;
  must_match?: string;
  text?: string;
  problem?: string;
}

/** The Synced workspaces folder of a join, or why it was refused. */
export interface SyncedFolder {
  path: string;
  refusal: string;
}

export interface DevicesApi {
  /** The group, or `null` while this device is in none. */
  view: DevicesView | null;
  /** The pairing attempt on screen and which side this device is. */
  pairing: { view: PairingView; opener: boolean } | null;
  /** The name of this device (separate from "Name shown to others"). */
  deviceName: string;
  setDeviceName: (text: string) => string;
  /** Add a device: this device shows a code. */
  addDevice: () => void;
  /** Join my devices with `code` into the Synced workspaces folder. */
  joinDevices: (code: string) => void;
  /** The opener's Pair. */
  pair: () => void;
  /** Cancel (or Words differ) the pairing on screen, and close its block. */
  cancelPairing: (wordsDiffer: boolean) => void;
  /** Closes a finished or failed pairing block. */
  closePairing: () => void;
  /** The name of the device this one just paired (its workspaces are chosen now), or "". */
  justPaired: string;
  clearJustPaired: () => void;
  syncedFolder: SyncedFolder;
  chooseSyncedFolder: () => void;
  /** Removes a device; "" when done, else the sentence why not. */
  remove: (member: string) => string;
  /** Leave my devices; "" when done, else the sentence why not. */
  leave: () => string;
  /** Dismisses the removed-device alert. */
  dismissRemoved: () => void;
  /** Whether the open shared workspace is synced to my devices. */
  openSynced: boolean;
  /** Switches the open workspace on (after the confirm); Share runs first if needed. */
  switchOn: () => void;
  /** Switches the open workspace off for every device; "" when started, else why not. */
  switchOff: () => string;
  /** Turns a running switch back; a switch-on's Share stops and nothing is written. */
  cancelSwitch: () => void;
  /** Reconnects every link of the group and the synced workspaces at once (Try again). */
  retryNow: () => void;
  /** The person leave of the open synced workspace (`specs/0097` criterion 16); "" when the
   * `left` mark went out, else why not. */
  leavePerson: () => string;
  /** Clears the `left` mark of a workspace and waits for the relay; `false` when it could not. */
  clearLeft: (id: string, name: string) => Promise<boolean>;
  /** The workspaces the devices list marks as left by the maker. */
  leftIds: string[];
  /** The member ids of my devices (the own group for the second-admin line), empty without one. */
  ownMembers: string[];
  /** "Uploading..." or "Stopping..." while a switch runs. */
  switching: "" | "on" | "off" | "leave";
  /** Why the last switch failed, or empty: the switch row shows the alert with Try again. */
  switchFailure: string;
  /** The other devices' names, for the switch-off confirm. */
  otherNames: string[];
  /** The workspaces offered after Pair (ux-notes 3.1); empty while none is being chosen. */
  offered: OfferedWorkspace[];
  /** Sync these: the chosen workspaces are switched on, open or not. */
  syncChosen: (chosen: OfferedWorkspace[]) => void;
  /** "Syncing 2 workspaces: uploading 3 of 14" while Sync these runs. */
  syncProgress: string;
  /** Workspaces Sync these could not switch on. */
  syncAlerts: string[];
  /** Copies of synced workspaces found here (criterion 14). */
  decisions: CopyDecision[];
  connectCopy: (id: string) => void;
  secondCopy: (id: string) => void;
  /** This device is a browser (ux-notes 9). */
  browser: boolean;
  /** Why this device cannot pair now beyond the server's reason ("" when it can): another tab
   * runs this browser's device, or a browser without a group cannot start one. */
  pairReason: string;
  /** The browser's own-row lines: the loss sentence, persistence, the storage it holds. */
  browserLines: string[];
  /** Joins the group a passkey backup named, with the restored identity (ADR 0040 decision 13):
   * the joiner's path of pairing, without a pairing. */
  restoreGroup: (server: string, room: string, label: string) => Promise<void>;
}


/** The member ids of my devices while the open workspace is synced to them, else none. */
export function myDeviceIds(devices: DevicesApi | undefined): Set<string> {
  return new Set(devices?.openSynced && devices.view && !devices.view.removed ? devices.view.rows.map((row) => row.member) : []);
}

/** "Synced, 2 devices" for a synced workspace every member of which is one of my devices, connected
 * and not receiving (0096 ux-notes 5.3 and 6.1); `undefined` otherwise (the `0095` text stays). */
export function syncedHeader(
  devices: DevicesApi | undefined,
  view: { online?: boolean; participants: { self: boolean; member: string }[]; workspace?: { receiving?: string | null } | null },
): string | undefined {
  const mine = myDeviceIds(devices);
  const allMine = view.participants.length > 0 && view.participants.every((p) => p.self || mine.has(p.member));
  return mine.size > 0 && allMine && view.online && !view.workspace?.receiving
    ? devices_synced_header(view.participants.length)
    : undefined;
}

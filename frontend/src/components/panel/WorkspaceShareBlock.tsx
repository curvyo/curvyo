import { RefreshCw, UserPlus } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";

import { ALERT, APPLY, ArmLines, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { InviteBlock } from "@/components/panel/ShareInviteJoin";
import { Tooltip } from "@/components/ui/tooltip";
import { PanelSwitch } from "@/components/ui/switch";
import { useArm } from "@/hooks/useArm";
import type { CollaborationApi, ShareView } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import {
  devices_switch_on_line,
  devices_text,
  membership_word,
  sharing_invite_blocker,
  sharing_word,
  sharing_workspace_state,
  workspace_not_added_text,
  workspace_uploading_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The switch "Sync to my devices" (`specs/0096-own-devices-sync` ux-notes 6): present while this
 * device is in a device group. */
export interface DevicesSwitch {
  /** Whether the devices list marks the open workspace on. */
  on: boolean;
  /** "on", "off" or "leave" while a switch runs. */
  switching: "" | "on" | "off" | "leave";
  /** Why the last switch failed, or empty (the alert with Try again). */
  failure: string;
  /** The other devices' names. */
  otherNames: string[];
  switchOn: () => void;
  /** "" when started, else why not (shown under the armed switch). */
  switchOff: () => string;
  /** Turns a running switch back (the switch is always operable, customer feedback PR #281). */
  cancel: () => void;
  /** Reconnects every waiting link at once (Try again). */
  reconnect: () => void;
  /** "You left Box job." when the maker left it from every device (`specs/0097` 5.4). */
  leftLine?: string;
}

/** The 28 px "Try again" icon button (`RefreshCw`), on the switch row and the Server field. */
export function TryAgain({ onPress }: { onPress: () => void }) {
  const label = devices_text("try-again");
  return (
    <Tooltip side="left" content={label}>
      <button type="button" aria-label={label} className={`${BUTTON} ${OUTLINE} w-7 border-transparent pointer-coarse:w-9`} onClick={onPress}>
        <RefreshCw size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      </button>
    </Tooltip>
  );
}

/** The facts of the Share tab that decide whether Invite person can work, apart from the
 * workspace's own state (customer feedback on PR #298; the rule is `curvyo-ui-core`'s). */
export interface InviteContext {
  desktop: boolean;
  serverSet: boolean;
  serverReachable: boolean;
  nameSet: boolean;
}

/** Why Invite person is greyed, or "" when it works. */
export function inviteBlocker(context: InviteContext, workspaceOpen: boolean, linked: boolean, admin: boolean, busy: boolean): string {
  return sharing_invite_blocker(
    context.desktop,
    workspaceOpen,
    context.serverSet,
    context.serverReachable || linked,
    context.nameSet,
    linked,
    admin,
    busy,
  );
}

/** Invite person, greyed with its reason as tooltip and description when it cannot work. */
export function InvitePersonButton({ blocker, onPress }: { blocker: string; onPress: () => void }) {
  const button = (
    <button
      type="button"
      aria-disabled={blocker !== ""}
      aria-description={blocker || undefined}
      className={`${BUTTON} ${OUTLINE} px-3 ${blocker ? "opacity-40" : ""}`}
      onClick={() => {
        if (blocker === "") {
          onPress();
        }
      }}
    >
      <UserPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      {sharing_word("invite-person")}
    </button>
  );
  return blocker ? (
    <Tooltip side="left" content={blocker}>
      {button}
    </Tooltip>
  ) : (
    button
  );
}

interface WorkspaceShareBlockProps {
  /** The open workspace's name (its folder's). */
  name: string;
  view: ShareView;
  collab: CollaborationApi;
  /** What else decides whether Invite person can work. */
  invite: InviteContext;
  /** The people other than my devices in the workspace. */
  others: string[];
  sync: WorkspaceSyncApi;
  /** My devices' switch; absent while this device is in no device group. */
  devices?: DevicesSwitch;
  /** The participant list of the shared workspace. */
  participants: ReactNode;
  /** The Notices block (`specs/0097` ux-notes 4), above the people. */
  notices?: ReactNode;
  /** The Removed block of a workspace this device was removed from or left (ux-notes 7). */
  ended?: ReactNode;
  onReturnFocus: () => void;
}

/**
 * The Share tab's Workspace block (`specs/0105` criteria 10 to 15, the Sync switch removed by
 * the customer on PR #298): headed by the workspace's name and its state (Local, Synced, Shared);
 * Invite person, which in a Local workspace starts sharing it in one confirm; "Sync to my
 * devices" while this device is in a device group; the people once it syncs.
 */
export function WorkspaceShareBlock({
  name,
  view,
  collab,
  invite,
  others,
  sync,
  devices,
  participants,
  notices,
  ended,
  onReturnFocus,
}: WorkspaceShareBlockProps) {
  const arm = useArm("sync-switch");
  const [confirm, setConfirm] = useState(false);
  const [inviteAfterSync, setInviteAfterSync] = useState(false);
  const linked = view.workspace != null && view.live && !view.workspace.joining;
  const sharing = sync.share.kind === "uploading" || sync.share.kind === "waiting" || devices?.switching === "on";
  const synced = linked || devices?.on === true;
  const othersIn = linked && others.length > 0;
  const switching = sharing || (devices !== undefined && devices.switching !== "");
  const state = sharing ? devices_text("sharing-header") : sharing_workspace_state(synced, othersIn);
  // Invite person in a Local workspace: the invite is written once the workspace syncs, once.
  const inviteNow = inviteAfterSync && linked && view.admin === true;
  const invited = useRef(false);
  const connected = view.state === "connected";
  useEffect(() => {
    if (inviteNow && connected && !invited.current) {
      invited.current = collab.createInvite() === "";
    }
  }, [inviteNow, connected, collab]);
  // A Share that failed ends the pending invite.
  const failed = sync.share.kind === "failed";
  const [seenFailed, setSeenFailed] = useState(failed);
  if (failed !== seenFailed) {
    setSeenFailed(failed);
    if (failed) {
      setInviteAfterSync(false);
    }
  }

  // Invite person starts sharing as needed; in a device group it also syncs to my devices.
  const startSharing = () => (devices ? devices.switchOn() : sync.shareWorkspace());
  const failure = devices?.failure || (sync.share.kind === "failed" ? sync.share.text : "");
  const blocker = inviteBlocker(invite, true, linked, view.admin === true, sharing);
  const line =
    sync.share.kind === "uploading"
      ? workspace_uploading_text(sync.share.done, sync.share.total)
      : devices?.switching === "off"
        ? devices_text("stopping")
        : devices?.switching === "leave"
          ? membership_word("leaving")
          : devices?.on && !sharing
            ? devices_switch_on_line(devices.otherNames.length)
            : (devices?.leftLine ?? "");

  return (
    <section aria-label={`Workspace ${name}`} className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3">
      <div className="flex items-baseline justify-between gap-2">
        <p className="min-w-0 truncate text-sm font-semibold text-[var(--toolbar-icon)]" title={name}>
          {name}
        </p>
        <span className={MUTED}>{state}</span>
      </div>
      {!synced && ended ? ended : null}
      {devices ? (
        <div {...arm.zone} className="flex flex-col gap-1" aria-busy={switching}>
          <PanelSwitch
            label={sharing_word("sync-to-my-devices")}
            checked={(devices.on && devices.switching !== "leave") || sharing}
            armed={arm.armed}
            extra={arm.control}
            onReturnFocus={() => {
              if (!devices.on) {
                onReturnFocus();
              }
            }}
            onCheckedChange={(next) => {
              if (devices.switching === "leave") {
                return;
              }
              if (switching) {
                // A running switch turns back at once: nothing was written yet.
                devices.cancel();
                setInviteAfterSync(false);
                return;
              }
              if (!next) {
                arm.press(devices.switchOff);
                return;
              }
              // On acts at once (`0096`, customer 2026-11-09); greyed for the same reasons as
              // Invite person, apart from the role.
              if (inviteBlocker(invite, true, linked, true, sharing) === "") {
                devices.switchOn();
              }
            }}
          />
          <ArmLines arm={arm} />
        </div>
      ) : null}
      {line ? (
        <p role="status" aria-live="polite" className={MUTED}>
          {line}
        </p>
      ) : null}
      {confirm ? (
        <div role="group" aria-label={sharing_word("sync-confirm")} className="flex flex-col gap-2">
          <p className="text-sm text-[var(--toolbar-icon)]">{sharing_word("sync-confirm")}</p>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              autoFocus
              className={`${BUTTON} ${APPLY} px-3`}
              onClick={() => {
                invited.current = false;
                setInviteAfterSync(true);
                setConfirm(false);
                startSharing();
              }}
            >
              {sharing_word("invite-person")}
            </button>
            <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={() => setConfirm(false)}>
              Cancel
            </button>
          </div>
        </div>
      ) : null}
      {failure && !switching && !synced ? (
        <div className="flex items-start gap-2">
          <p role="alert" className={`${ALERT} min-w-0 flex-1`}>
            {failure}
          </p>
          <TryAgain
            onPress={() => {
              devices?.reconnect();
              startSharing();
            }}
          />
        </div>
      ) : null}
      {linked ? (
        <p role="status" aria-live="polite" className={MUTED}>
          {view.workspace?.receiving ?? view.stateLine}
        </p>
      ) : null}
      {linked ? notices : null}
      {linked && blocker === "" ? (
        <InviteBlock
          collab={collab}
          shown={inviteNow && collab.invite !== null}
          onClose={() => {
            setInviteAfterSync(false);
            invited.current = false;
          }}
        />
      ) : confirm || (inviteAfterSync && !linked) ? null : (
        <InvitePersonButton blocker={blocker} onPress={() => setConfirm(true)} />
      )}
      {linked ? participants : null}
      {sync.notAdded.length > 0 ? (
        <p role="status" className={MUTED}>
          {workspace_not_added_text(JSON.stringify(sync.notAdded))}
        </p>
      ) : null}
    </section>
  );
}

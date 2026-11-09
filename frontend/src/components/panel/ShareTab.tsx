import { FolderPlus } from "lucide-react";
import { useEffect } from "react";

import { ALERT, APPLY, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { EndedBlock } from "@/components/panel/EndedBlock";
import { AdminChangeLine, MembershipNotices } from "@/components/panel/MembershipNotices";
import { MyDevicesBlock } from "@/components/panel/MyDevicesBlock";
import { ParticipantList } from "@/components/panel/ParticipantList";
import { InviteBlock, JoinEntry } from "@/components/panel/ShareInviteJoin";
import { SettingField } from "@/components/panel/ShareSettingField";
import { InvitePersonButton, TryAgain, WorkspaceShareBlock, inviteBlocker } from "@/components/panel/WorkspaceShareBlock";
import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { DevicesApi } from "@/hooks/useDevices";
import type { MakeWorkspaceApi } from "@/hooks/useMakeWorkspace";
import type { MembershipApi } from "@/hooks/useMembership";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import { myDeviceIds } from "@/lib/devicesApi";
import { frozenWarning } from "@/lib/frozenWarning";
import {
  membership_you_left_line,
  normalize_server_address,
  server_shown,
  sharing_make_workspace_confirm,
  sharing_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The open workspace as the Share tab's Workspace block needs it (`specs/0095-workspace`). */
export interface ShareTabWorkspace {
  /** A workspace is open. */
  open: boolean;
  name: string;
  /** Its document count from the scan. */
  documents: number;
  sync: WorkspaceSyncApi;
}

interface ShareTabProps {
  collab: CollaborationApi;
  onReturnFocus: () => void;
  /** The open workspace's sharing; absent while none is open and in the browser without one. */
  workspace?: ShareTabWorkspace;
  /** The desktop: a person's link joins and Invite person is offered (`specs/0105` criterion
   * 19: the browser takes part only as one of my devices). */
  desktop: boolean;
  /** Make a workspace for this file (`specs/0105` criteria 7 to 9). */
  makeWorkspace: MakeWorkspaceApi;
  /** The panel's body is shown: the reachability check runs only then (ADR 0044 decision 9). */
  visible: boolean;
  /** My devices (`specs/0096-own-devices-sync`); absent in the browser until its slice. */
  devices?: DevicesApi;
  /** Roles, leave, add back and the notices (`specs/0097-membership-lifecycle`). */
  membership: MembershipApi;
}

/** How often the reachability line re-checks the server while the tab is shown (ms). */
const REACH_POLL_MS = 5000;

/** Sharing works on workspaces (`specs/0105` criteria 7 to 9): one line, and for a saved file on
 * the desktop the button with its inline confirm naming the new folder. */
function MakeWorkspaceBlock({ make }: { make: MakeWorkspaceApi }) {
  if (make.browser) {
    return <p className={MUTED}>{sharing_word("workspaces-on-desktop")}</p>;
  }
  return (
    <div className="flex flex-col gap-2">
      <p className={MUTED}>{sharing_word("works-on-workspaces")}</p>
      {make.offered && make.folder === null ? (
        <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={make.ask}>
          <FolderPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          {sharing_word("make-workspace")}
        </button>
      ) : null}
      {make.offered && make.folder !== null ? (
        <div role="group" aria-label={sharing_word("make-workspace")} className="flex flex-col gap-2">
          <p className="break-all text-sm text-[var(--toolbar-icon)]">
            {sharing_make_workspace_confirm(make.folder, make.fileName)}
          </p>
          <div className="flex flex-wrap gap-2">
            <button type="button" autoFocus className={`${BUTTON} ${APPLY} px-3`} onClick={make.confirm}>
              Make workspace
            </button>
            <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={make.cancel}>
              Cancel
            </button>
          </div>
        </div>
      ) : null}
      {make.failure ? (
        <p role="alert" className={ALERT}>
          {make.failure}
        </p>
      ) : null}
    </div>
  );
}

/**
 * The Share tab (`specs/0105-sharing-concept`): the server and name settings, the open
 * workspace with its Sync switch, Invite person and people, Make a workspace for a file outside
 * one, My devices, and the one Join entry. Nothing here shares a single document.
 */
export function ShareTab({ collab, onReturnFocus, workspace, desktop, makeWorkspace, visible, devices, membership }: ShareTabProps) {
  const { view } = collab;
  const workspaceView = view.workspace ?? null;
  const server = normalize_server_address(collab.server);
  const { checkServer } = collab;
  const ended = membership.ended;
  // While the tab is shown with a server set and no link, the reachability line re-checks every
  // 5 s; a link shows its own state. The check stops with the tab (ADR 0044 decision 9).
  const linked = view.live && (view.shared || workspaceView != null);
  useEffect(() => {
    if (!visible || server === "" || linked) {
      return undefined;
    }
    checkServer();
    const timer = window.setInterval(checkServer, REACH_POLL_MS);
    return () => window.clearInterval(timer);
  }, [visible, server, linked, checkServer]);

  const serverReason =
    server === ""
      ? "Set a server address."
      : collab.reach === "unreachable"
        ? "Cannot reach the server."
        : collab.name === ""
          ? "Enter a name."
          : "";
  // My own devices among the participants of a synced workspace (0096 ux-notes 6.1).
  const myDevices = myDeviceIds(devices);
  const others = view.participants.filter((p) => !p.self && !myDevices.has(p.member)).map((p) => p.label);
  const participantList =
    view.participants.length > 0 ? <ParticipantList view={view} myDevices={myDevices} membership={membership} /> : null;
  const notices = <MembershipNotices view={view} membership={membership} />;
  // A workspace the maker left from every device: the switch says so (ux-notes 5.4).
  const workspaceId = workspaceView?.id ?? "";
  const leftLine =
    workspace?.open && devices && (devices.leftIds.includes(workspaceId) || ended?.left === true)
      ? membership_you_left_line(workspace.name)
      : "";
  const reachLine =
    collab.reach === "reaching"
      ? "Reaching..."
      : collab.reach === "reachable"
        ? "Connected"
        : collab.reach === "unreachable"
          ? `Cannot reach ${server_shown(server)}.`
          : "";
  const paired = devices?.view != null && !devices.view.removed;
  const inviteContext = {
    desktop,
    serverSet: server !== "",
    serverReachable: collab.reach !== "unreachable",
    nameSet: collab.name !== "",
  };
  return (
    <div className="flex flex-col gap-3">
      <AdminChangeLine membership={membership} />
      <div className="flex flex-col gap-2">
        {linked ? (
          <div className="flex flex-col gap-1">
            <span className="text-xs text-[var(--toolbar-icon)]">Server</span>
            <p className="truncate text-sm text-[var(--toolbar-icon)]">{view.serverShown}</p>
          </div>
        ) : (
          <>
            <SettingField
              key={collab.server}
              label="Server"
              value={collab.server}
              placeholder="Server address"
              onCommit={collab.setServer}
            />
            {reachLine ? (
              <div className="flex items-center gap-2">
                <p role="status" aria-live="polite" className={`${MUTED} min-w-0 flex-1`}>
                  {reachLine}
                </p>
                {collab.reach === "unreachable" ? (
                  <TryAgain
                    onPress={() => {
                      // Checks the server again and retries every waiting link at once.
                      collab.checkServer();
                      collab.reconnect();
                      devices?.retryNow();
                    }}
                  />
                ) : null}
              </div>
            ) : null}
          </>
        )}
        <SettingField
          key={collab.name}
          label="Name shown to others"
          value={collab.name}
          onCommit={collab.setName}
        />
        {collab.passphrase ? (
          <SettingField
            label={
              collab.passphrase.exists
                ? "This device has no credential store. Passphrase of the key file"
                : "This device has no credential store. Set a passphrase for the key file"
            }
            value=""
            onCommit={(text) => {
              collab.submitPassphrase(text);
              return "";
            }}
          />
        ) : null}
        {collab.passphrase?.wrong ? (
          <p role="alert" className={ALERT}>
            This passphrase does not open the key file.
          </p>
        ) : null}
      </div>

      {workspace?.open ? (
        <WorkspaceShareBlock
          name={workspace.name}
          view={view}
          collab={collab}
          invite={inviteContext}
          others={others}
          sync={workspace.sync}
          participants={participantList}
          notices={notices}
          ended={ended ? <EndedBlock membership={membership} /> : null}
          onReturnFocus={onReturnFocus}
          devices={
            devices && paired
              ? {
                  on: devices.openSynced,
                  switching: devices.switching,
                  failure: devices.switchFailure,
                  otherNames: devices.otherNames,
                  // An ended workspace is never shared anew by the switch (`specs/0097` 5.4).
                  switchOn: ended ? membership.clearLeftMark : devices.switchOn,
                  switchOff: devices.switchOff,
                  cancel: devices.cancelSwitch,
                  reconnect: () => {
                    collab.reconnect();
                    devices.retryNow();
                  },
                  leftLine,
                }
              : undefined
          }
        />
      ) : workspaceView && !workspaceView.joining ? (
        /* The session carries a shared workspace's link while no workspace is open in the tree
           (a document of it opened on its own, a join still settling): its people and Invite
           person stay reachable (customer 2026-11-09). */
        <section aria-label={`Workspace ${workspaceView.name}`} className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3">
          <div className="flex items-baseline justify-between gap-2">
            <p className="min-w-0 truncate text-sm font-semibold text-[var(--toolbar-icon)]" title={workspaceView.name}>
              {workspaceView.name}
            </p>
            <span className={MUTED}>{view.live ? workspaceView.people : ""}</span>
          </div>
          {view.live ? (
            <>
              <p role="status" aria-live="polite" className={MUTED}>
                {workspaceView.receiving ?? view.stateLine}
              </p>
              {notices}
              {inviteBlocker(inviteContext, true, true, view.admin === true, false) === "" ? (
                <InviteBlock collab={collab} />
              ) : (
                <InvitePersonButton blocker={inviteBlocker(inviteContext, true, true, view.admin === true, false)} onPress={() => undefined} />
              )}
              {participantList}
            </>
          ) : null}
        </section>
      ) : workspaceView ? null : (
        <>
          <MakeWorkspaceBlock make={makeWorkspace} />
          {/* Invite person needs a workspace: greyed with the reason (customer feedback on
              PR #298). */}
          <InvitePersonButton blocker={inviteBlocker(inviteContext, false, false, false, false)} onPress={() => undefined} />
        </>
      )}

      {devices ? (
        <MyDevicesBlock
          devices={devices}
          server={server}
          reason={serverReason}
          onReturnFocus={onReturnFocus}
          frozenWarning={devices.openSynced ? frozenWarning(membership.frozen) : ""}
        />
      ) : null}

      <JoinEntry collab={collab} workspace={desktop ? workspace?.sync : undefined} devices={devices} server={server} />
    </div>
  );
}

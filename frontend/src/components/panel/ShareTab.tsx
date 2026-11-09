import { Share2 } from "lucide-react";
import { useEffect, useId, useState } from "react";

import { APPLY, BUTTON, MUTED } from "@/components/panel/devicesUi";
import { EndedBlock } from "@/components/panel/EndedBlock";
import { AdminChangeLine, MembershipNotices } from "@/components/panel/MembershipNotices";
import { MyDevicesBlock } from "@/components/panel/MyDevicesBlock";
import { ParticipantList } from "@/components/panel/ParticipantList";
import { InviteBlock, JoinBlock } from "@/components/panel/ShareInviteJoin";
import { SettingField } from "@/components/panel/ShareSettingField";
import { TryAgain, WorkspaceShareBlock } from "@/components/panel/WorkspaceShareBlock";
import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { DevicesApi } from "@/hooks/useDevices";
import type { MembershipApi } from "@/hooks/useMembership";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import { myDeviceIds, syncedHeader } from "@/lib/devicesApi";
import { frozenWarning } from "@/lib/frozenWarning";
import {
  membership_you_left_line,
  normalize_server_address,
  server_shown,
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
  /** Whether the open project has a saved file (criterion 1: a saved document is shared). */
  saved: boolean;
  onReturnFocus: () => void;
  /** The workspace's sharing; absent in the browser (desktop only). */
  workspace?: ShareTabWorkspace;
  /** My devices (`specs/0096-own-devices-sync`); absent in the browser until its slice. */
  devices?: DevicesApi;
  /** Roles, leave, add back and the notices (`specs/0097-membership-lifecycle`). */
  membership: MembershipApi;
}

/** How often the reachability line re-checks the server while the tab is shown (ms). */
const REACH_POLL_MS = 5000;

/**
 * The Share tab (`specs/0092-cooperative-editing` UX notes §2): the server and name settings,
 * Share with its inline confirm, Invite, the participant list and Open shared document.
 */
export function ShareTab({ collab, saved, onReturnFocus, workspace, devices, membership }: ShareTabProps) {
  const { view } = collab;
  // A session that carries a shared workspace's link shows the workspace in the Workspace block
  // and only a line in the Document block (`specs/0095-workspace` ux-notes section 9).
  const workspaceView = view.workspace ?? null;
  const [sharing, setSharing] = useState(false);
  const [shareError, setShareError] = useState("");
  const reasonId = useId();
  const shared = view.shared && view.live;
  const server = normalize_server_address(collab.server);
  const { checkServer } = collab;
  const ended = membership.ended;
  // While the tab is shown with a server set and no link, the reachability line re-checks every
  // 5 s; a link shows its own state. The check stops with the tab (ADR 0044 decision 9).
  const linked = view.live && (view.shared || workspaceView != null);
  useEffect(() => {
    if (server === "" || linked) {
      return undefined;
    }
    checkServer();
    const timer = window.setInterval(checkServer, REACH_POLL_MS);
    return () => window.clearInterval(timer);
  }, [server, linked, checkServer]);

  const serverReason =
    server === ""
      ? "Set a server address."
      : collab.reach === "unreachable"
        ? "Cannot reach the server."
        : collab.name === ""
          ? "Enter a name."
          : "";
  const reason = !saved ? "Save the document first." : serverReason;
  // My own devices among the participants of a synced workspace (0096 ux-notes 6.1).
  const myDevices = myDeviceIds(devices);
  const others = view.participants.filter((p) => !p.self && !myDevices.has(p.member)).map((p) => p.label);
  const participantList =
    view.participants.length > 0 ? <ParticipantList view={view} myDevices={myDevices} membership={membership} /> : null;
  const notices = <MembershipNotices view={view} membership={membership} />;
  // A workspace the maker left from every device: the switch says so (ux-notes 5.4).
  const workspaceId = workspaceView?.id ?? "";
  const leftLine =
    workspace?.open && devices && (devices.leftIds.includes(workspaceId) || (ended?.workspace === true && ended.left))
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
                ? "This computer has no credential store. Passphrase of the key file"
                : "This computer has no credential store. Set a passphrase for the key file"
            }
            value=""
            onCommit={(text) => {
              collab.submitPassphrase(text);
              return "";
            }}
          />
        ) : null}
        {collab.passphrase?.wrong ? (
          <p role="alert" className="text-xs text-[var(--field-invalid)]">
            This passphrase does not open the key file.
          </p>
        ) : null}
      </div>

      {devices ? (
        <MyDevicesBlock
          devices={devices}
          server={server}
          reason={serverReason}
          onReturnFocus={onReturnFocus}
          frozenWarning={devices.openSynced ? frozenWarning(membership.frozen) : ""}
        />
      ) : null}

      {workspace?.open ? (
        <WorkspaceShareBlock
          name={workspace.name}
          documents={workspace.documents}
          view={view}
          server={collab.server}
          reason={serverReason}
          sync={workspace.sync}
          invite={
            workspaceView ? (
              <InviteBlock collab={collab} />
            ) : null
          }
          participants={workspaceView ? participantList : null}
          notices={workspaceView ? notices : null}
          ended={ended?.workspace ? <EndedBlock membership={membership} /> : null}
          endedAsks={ended?.workspace === true && ended.check === "addedBack"}
          syncedHeader={syncedHeader(devices, view)}
          devices={
            devices && devices.view && !devices.view.removed
              ? {
                  on: devices.openSynced,
                  reason: serverReason,
                  switching: devices.switching,
                  failure: devices.switchFailure,
                  otherNames: devices.otherNames,
                  others,
                  // An ended workspace is never shared anew by the switch (`specs/0097` 5.4).
                  switchOn: ended?.workspace ? membership.clearLeftMark : devices.switchOn,
                  switchOff: devices.switchOff,
                  cancel: devices.cancelSwitch,
                  reconnect: () => {
                    collab.reconnect();
                    devices.retryNow();
                  },
                  leftLine: leftLine,
                  onReturnFocus,
                }
              : undefined
          }
        />
      ) : null}

      {/* The session carries a shared workspace's link while no workspace is open in the tree
          (a document of it opened on its own, a join still settling): its people, Invite and
          Stop sharing stay reachable (customer 2026-11-09). */}
      {workspaceView && !workspace?.open && !workspaceView.joining ? (
        <section aria-label={`Workspace ${workspaceView.name}`} className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3">
          <div className="flex items-baseline justify-between gap-2">
            <p className="min-w-0 truncate text-sm font-semibold text-[var(--toolbar-icon)]" title={workspaceView.name}>
              {workspaceView.name}
            </p>
            <span className={MUTED}>{view.live ? workspaceView.people : "Not shared"}</span>
          </div>
          {view.live ? (
            <>
              <div className="flex flex-col gap-1">
                <p className="text-sm text-[var(--toolbar-icon)]">Shared on {view.serverShown}</p>
                <p role="status" aria-live="polite" className={MUTED}>
                  {workspaceView.receiving ?? view.stateLine}
                </p>
              </div>
              {notices}
              {view.admin ? <InviteBlock collab={collab} /> : null}
              {participantList}
            </>
          ) : null}
        </section>
      ) : null}

      {workspaceView ? (
        workspaceView.openInWorkspace ? (
          <div className="flex flex-col gap-1">
            <p className="text-sm text-[var(--toolbar-icon)]">{workspaceView.documentLine}</p>
            <p role="status" aria-live="polite" className={MUTED}>
              {view.stateLine}
            </p>
          </div>
        ) : null
      ) : workspace?.sync.sharedOpen ? null : (
        <>
          {ended && !ended.workspace ? <EndedBlock membership={membership} /> : null}
          {shared ? (
            <div className="flex flex-col gap-1">
              <p className="text-sm text-[var(--toolbar-icon)]">Shared on {view.serverShown}</p>
              <p role="status" aria-live="polite" className={MUTED}>
                {view.stateLine}
              </p>
            </div>
          ) : ended?.check === "addedBack" ? null : (
            <div className="flex flex-col gap-1">
              <button
                type="button"
                aria-disabled={reason !== "" || sharing}
                aria-busy={sharing}
                aria-describedby={reason ? reasonId : undefined}
                className={`${BUTTON} ${APPLY} w-[118px]`}
                onClick={() => {
                  if (reason === "" && !sharing) {
                    // Share asks nothing (customer 2026-11-09: no confirm sentences here).
                    setShareError("");
                    setSharing(true);
                    void collab.share().then((error) => {
                      setSharing(false);
                      if (error) {
                        setShareError(`Could not share: ${error}`);
                      }
                    });
                  }
                }}
              >
                <Share2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                {sharing ? "Sharing..." : "Share"}
              </button>
              {reason ? (
                <p id={reasonId} className={MUTED}>
                  {reason}
                </p>
              ) : null}
              {shareError ? (
                <p role="alert" className="text-xs text-[var(--field-invalid)]">
                  {shareError}
                </p>
              ) : null}
            </div>
          )}

          {shared ? notices : null}
          {shared && view.admin ? <InviteBlock collab={collab} /> : null}

          {participantList}
        </>
      )}

      {shared && !workspaceView?.joining ? null : <JoinBlock collab={collab} workspace={workspace?.sync} />}
    </div>
  );
}

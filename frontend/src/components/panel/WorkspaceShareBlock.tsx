import { RefreshCw, Share2 } from "lucide-react";
import { type ReactNode, useId } from "react";

import { ALERT, APPLY, ArmLines, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { useArm } from "@/hooks/useArm";
import { Tooltip } from "@/components/ui/tooltip";
import { PanelSwitch } from "@/components/ui/switch";
import type { ShareView } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import {
  devices_switch_on_line,
  devices_text,
  membership_word,
  workspace_not_added_text,
  workspace_uploading_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The switch "Sync to my devices" (`specs/0096-own-devices-sync` ux-notes 6). */
export interface SyncSwitch {
  /** Whether the devices list marks the open workspace on. */
  on: boolean;
  /** Why it cannot be turned on, or empty. */
  reason: string;
  /** "on" or "off" while a switch runs. */
  switching: "" | "on" | "off" | "leave";
  /** Why the last switch failed, or empty (the alert with Try again). */
  failure: string;
  /** The other devices' names (the off sentence names them). */
  otherNames: string[];
  /** The people other than my devices who share the workspace (the off sentence names them). */
  others: string[];
  switchOn: () => void;
  /** "" when started, else why not (shown under the armed switch). */
  switchOff: () => string;
  /** Turns a running switch back (the switch is always operable, customer feedback PR #281). */
  cancel: () => void;
  /** Reconnects every waiting link at once (Try again, before the switch is turned on again). */
  reconnect: () => void;
  /** "You left Box job." when the maker left it from every device (`specs/0097` 5.4). */
  leftLine?: string;
  onReturnFocus: () => void;
}

/** The switch row: on asks with an Apply inline confirm, off is armed (a second press turns it
 * off), a running switch turns back at once, a failure says why with Try again (ux-notes 6.1 to
 * 6.3, customer feedback on PR #281). */
function SyncSwitchRow({
  sync,
  share,
}: {
  sync: SyncSwitch;
  /** Share workspace's progress, which the switch-on runs first. */
  share: WorkspaceSyncApi["share"];
}) {
  const arm = useArm("sync-switch");
  const line =
    sync.switching === "on"
      ? share.kind === "uploading"
        ? workspace_uploading_text(share.done, share.total)
        : devices_text("sharing")
      : sync.switching === "off"
        ? devices_text("stopping")
        : sync.switching === "leave"
          ? membership_word("leaving")
          : sync.on
            ? devices_switch_on_line(sync.otherNames.length)
            : sync.leftLine || sync.reason;
  const checked = (sync.on && sync.switching !== "leave") || sync.switching === "on";
  // Both presses of the armed switch-off keep the focus on the switch: a focus that left the
  // row would disarm it before the second press acts.
  const armable = sync.on && sync.switching === "";
  return (
    <div {...arm.zone} className="flex flex-col gap-1" aria-busy={sync.switching !== ""}>
      <PanelSwitch
        label="Sync to my devices"
        checked={checked}
        armed={arm.armed}
        extra={arm.control}
        onReturnFocus={() => {
          if (!armable) {
            sync.onReturnFocus();
          }
        }}
        onCheckedChange={(next) => {
          if (sync.switching === "leave") {
            return;
          }
          if (sync.switching !== "") {
            // A running switch turns back at once: nothing was written yet.
            sync.cancel();
            return;
          }
          if (!next) {
            arm.press(sync.switchOff);
            return;
          }
          if (sync.reason !== "") {
            return;
          }
          // On asks nothing (customer 2026-11-09: no confirm sentences in the Share tab).
          sync.switchOn();
        }}
      />
      {line ? (
        <p role="status" aria-live="polite" className={MUTED}>
          {line}
        </p>
      ) : null}
      <ArmLines arm={arm} />
      {sync.failure && sync.switching === "" && !sync.on ? (
        <div className="flex items-start gap-2">
          <p role="alert" className={`${ALERT} min-w-0 flex-1`}>
            {sync.failure}
          </p>
          <TryAgain
            onPress={() => {
              sync.reconnect();
              sync.switchOn();
            }}
          />
        </div>
      ) : null}
    </div>
  );
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

interface WorkspaceShareBlockProps {
  /** The open workspace's name (its folder's). */
  name: string;
  /** Its document count from the scan. */
  documents: number;
  view: ShareView;
  server: string;
  /** Why Share workspace is unavailable, or empty. */
  reason: string;
  sync: WorkspaceSyncApi;
  /** The Invite block, shown to an admin of the shared workspace. */
  invite: ReactNode;
  /** The participant list of the shared workspace. */
  participants: ReactNode;
  /** The Notices block (`specs/0097` ux-notes 4), above Invite. */
  notices?: ReactNode;
  /** The Removed block of a workspace this device was removed from or left (ux-notes 7). */
  ended?: ReactNode;
  /** The Removed block's add-back prompt is up: Share workspace waits. */
  endedAsks?: boolean;
  /** The switch "Sync to my devices"; absent in the browser. */
  devices?: SyncSwitch;
  /** The heading's right text when every member is one of my devices ("Synced, 2 devices"). */
  syncedHeader?: string;
}

/**
 * The Share tab's Workspace block (`specs/0095-workspace` `ux-notes.md` section 4, criteria 25
 * and 26): headed by the workspace's name and state; Share workspace with its inline confirm and
 * progress while it is local; the server, connection, Invite and participants once shared.
 */
export function WorkspaceShareBlock({
  name,
  view,
  reason,
  sync,
  invite,
  participants,
  notices,
  ended,
  endedAsks = false,
  devices,
  syncedHeader,
}: WorkspaceShareBlockProps) {
  const reasonId = useId();
  const shared = view.workspace != null && view.live && !view.workspace.joining;
  const busy = sync.share.kind === "uploading" || sync.share.kind === "waiting";
  return (
    <section aria-label={`Workspace ${name}`} className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3">
      <div className="flex items-baseline justify-between gap-2">
        <p className="min-w-0 truncate text-sm font-semibold text-[var(--toolbar-icon)]" title={name}>
          {name}
        </p>
        <span className={MUTED}>
          {shared
            ? syncedHeader || view.workspace?.people
            : busy || devices?.switching === "on"
              ? devices_text("sharing-header")
              : "Not shared"}
        </span>
      </div>
      {devices ? (
        <SyncSwitchRow sync={devices} share={sync.share} />
      ) : null}
      {shared ? (
        <>
          <div className="flex flex-col gap-1">
            <p className="text-sm text-[var(--toolbar-icon)]">Shared on {view.serverShown}</p>
            <p role="status" aria-live="polite" className={MUTED}>
              {view.workspace?.receiving ?? view.stateLine}
            </p>
          </div>
          {notices}
          {view.admin ? invite : null}
          {participants}
        </>
      ) : null}
      {/* A removed or left workspace keeps its Removed block above Share workspace, which stays
          available: it starts something new (ux-notes 7.1.3); hidden while the add-back prompt
          asks. */}
      {!shared && ended ? ended : null}
      {shared || endedAsks ? null : devices && devices.switching === "on" ? null : (
        <div className="flex flex-col gap-1">
          <button
            type="button"
            aria-disabled={reason !== "" || busy}
            aria-busy={busy}
            aria-describedby={reason ? reasonId : undefined}
            className={`${BUTTON} ${APPLY} w-[148px]`}
            onClick={() => {
              if (reason === "" && !busy) {
                // Share asks nothing (customer 2026-11-09: no confirm sentences in the Share tab).
                sync.shareWorkspace();
              }
            }}
          >
            <Share2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            {busy ? "Sharing..." : "Share workspace"}
          </button>
          {reason ? (
            <p id={reasonId} className={MUTED}>
              {reason}
            </p>
          ) : null}
          {sync.share.kind === "uploading" ? (
            <p role="status" aria-live="polite" className={MUTED}>
              {workspace_uploading_text(sync.share.done, sync.share.total)}
            </p>
          ) : null}
          {sync.share.kind === "failed" ? (
            <p role="alert" className="text-xs text-[var(--field-invalid)]">
              {sync.share.text}
            </p>
          ) : null}
        </div>
      )}
      {sync.notAdded.length > 0 ? (
        <p role="status" className={MUTED}>
          {workspace_not_added_text(JSON.stringify(sync.notAdded))}
        </p>
      ) : null}
    </section>
  );
}

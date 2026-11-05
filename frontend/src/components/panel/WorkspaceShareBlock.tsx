import { Share2 } from "lucide-react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, DestructiveConfirm, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { PanelSwitch } from "@/components/ui/switch";
import type { ShareView } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import {
  devices_switch_on_line,
  devices_sync_off_confirm,
  devices_sync_on_confirm,
  devices_text,
  membership_word,
  normalize_server_address,
  workspace_not_added_text,
  workspace_share_confirm_text,
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
  /** The last switch-on failed. */
  failed: boolean;
  /** The other devices' names (the off confirm names them). */
  otherNames: string[];
  /** The people other than my devices who share the workspace (the off confirm names them). */
  others: string[];
  switchOn: () => void;
  /** "" when started, else why not (the confirm stays and shows it). */
  switchOff: () => string;
  /** "You left Box job." when the maker left it from every device (`specs/0097` 5.4). */
  leftLine?: string;
  onReturnFocus: () => void;
}

/** The switch row with its inline confirms (ux-notes 6.1 to 6.3). */
function SyncSwitchRow({
  name,
  documents,
  sync,
  share,
  takenIn,
  outside,
  check,
}: {
  name: string;
  documents: number;
  sync: SyncSwitch;
  /** Share workspace's progress, which the switch-on runs first. */
  share: WorkspaceSyncApi["share"];
  takenIn: string[];
  outside: string[];
  /** Finds the documents shared on their own that the on confirm names. */
  check: () => void;
}) {
  const [confirm, setConfirm] = useState<"" | "on" | "off">("");
  const row = useRef<HTMLDivElement>(null);
  const returnFocus = useRef(false);
  // Cancel, Escape and the action put the focus back on the switch the confirm replaced.
  useEffect(() => {
    if (confirm === "" && returnFocus.current) {
      returnFocus.current = false;
      row.current?.querySelector<HTMLElement>('[role="switch"]')?.focus();
    }
  }, [confirm]);
  const close = () => {
    returnFocus.current = true;
    setConfirm("");
  };
  const busy = sync.switching !== "";
  if (confirm === "off") {
    return (
      <DestructiveConfirm
        text={devices_sync_off_confirm(name, JSON.stringify(sync.otherNames), JSON.stringify(sync.others))}
        label={`Stop syncing ${name}`}
        action="Stop syncing"
        width={110}
        onCancel={close}
        onConfirm={() => {
          const refusal = sync.switchOff();
          if (refusal === "") {
            close();
          }
          return refusal;
        }}
      />
    );
  }
  if (confirm === "on") {
    const text = devices_sync_on_confirm(
      name,
      documents,
      sync.otherNames.length === 0,
      JSON.stringify(takenIn),
      JSON.stringify(outside),
    );
    return (
      <div
        role="group"
        aria-label={`Sync ${name} to your devices`}
        className="flex flex-col gap-2"
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
      >
        <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
        <div className="flex flex-wrap gap-2">
          <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
            Cancel
          </button>
          <button
            type="button"
            className={`${BUTTON} ${APPLY} w-[110px]`}
            onClick={() => {
              close();
              sync.switchOn();
            }}
          >
            Sync
          </button>
        </div>
      </div>
    );
  }
  const line =
    sync.switching === "on"
      ? share.kind === "uploading"
        ? workspace_uploading_text(share.done, share.total)
        : "Uploading..."
      : sync.switching === "off"
        ? "Stopping..."
        : sync.switching === "leave"
          ? membership_word("leaving")
          : sync.on
            ? devices_switch_on_line(sync.otherNames.length)
            : sync.leftLine || sync.reason;
  return (
    <div ref={row} className="flex flex-col gap-1" aria-busy={busy}>
      <PanelSwitch
        label="Sync to my devices"
        checked={(sync.on && sync.switching !== "leave") || sync.switching === "on"}
        onReturnFocus={sync.onReturnFocus}
        onCheckedChange={(checked) => {
          if (busy || (checked && sync.reason !== "")) {
            return;
          }
          if (checked) {
            check();
          }
          setConfirm(checked ? "on" : "off");
        }}
      />
      {line ? (
        <p role="status" aria-live="polite" className={MUTED}>
          {line}
        </p>
      ) : null}
      {sync.failed && !busy && !sync.on ? (
        <p role="alert" className={ALERT}>
          {devices_text("sync-failed")}
        </p>
      ) : null}
    </div>
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
  documents,
  view,
  server,
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
  const [confirming, setConfirming] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const returnFocus = useRef(false);
  // Cancel and Escape put the focus back on Share workspace, which the confirm replaced.
  useEffect(() => {
    if (!confirming && returnFocus.current) {
      returnFocus.current = false;
      trigger.current?.focus();
    }
  }, [confirming]);
  const cancel = () => {
    returnFocus.current = true;
    setConfirming(false);
  };
  const reasonId = useId();
  const shared = view.workspace != null && view.live && !view.workspace.joining;
  const busy = sync.share.kind === "uploading" || sync.share.kind === "waiting";
  const confirmText = workspace_share_confirm_text(
    name,
    documents,
    normalize_server_address(server),
    JSON.stringify(sync.shareCheck.takenIn),
    JSON.stringify(sync.shareCheck.outside),
  );
  return (
    <section aria-label={`Workspace ${name}`} className="flex flex-col gap-2 border-b border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] pb-3">
      <div className="flex items-baseline justify-between gap-2">
        <p className="min-w-0 truncate text-sm font-semibold text-[var(--toolbar-icon)]" title={name}>
          {name}
        </p>
        <span className={MUTED}>{shared ? syncedHeader || view.workspace?.people : "Not shared"}</span>
      </div>
      {devices ? (
        <SyncSwitchRow
          name={name}
          documents={documents}
          sync={devices}
          share={sync.share}
          takenIn={sync.shareCheck.takenIn}
          outside={sync.shareCheck.outside}
          check={() => void sync.checkShare()}
        />
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
      {shared || endedAsks ? null : devices && devices.switching === "on" ? null : confirming ? (
        <div
          role="group"
          aria-label={confirmText}
          className="flex flex-col gap-2"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              cancel();
            }
          }}
        >
          <p className="text-xs text-[var(--toolbar-icon)]">{confirmText}</p>
          <div className="flex flex-wrap gap-2">
            <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={cancel}>
              Cancel
            </button>
            <button
              type="button"
              className={`${BUTTON} ${APPLY} w-[76px]`}
              onClick={() => {
                setConfirming(false);
                sync.shareWorkspace();
              }}
            >
              Share
            </button>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          <button
            ref={trigger}
            type="button"
            aria-disabled={reason !== "" || busy}
            aria-busy={busy}
            aria-describedby={reason ? reasonId : undefined}
            className={`${BUTTON} ${APPLY} w-[148px]`}
            onClick={() => {
              if (reason === "" && !busy) {
                // Criterion 42: the confirm names the documents taken in beforehand.
                void sync.checkShare().then(() => setConfirming(true));
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

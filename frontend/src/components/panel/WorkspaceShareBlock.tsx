import { Share2 } from "lucide-react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";

import type { ShareView } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import {
  normalize_server_address,
  workspace_not_added_text,
  workspace_share_confirm_text,
  workspace_uploading_text,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const BUTTON =
  "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-[5px] border text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9 aria-disabled:opacity-40";
const OUTLINE =
  "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
const APPLY =
  "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:brightness-110";
const MUTED = "text-xs text-[var(--panel-muted-fg)]";

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
        <span className={MUTED}>{shared ? view.workspace?.people : "Not shared"}</span>
      </div>
      {shared ? (
        <>
          <div className="flex flex-col gap-1">
            <p className="text-sm text-[var(--toolbar-icon)]">Shared on {view.serverShown}</p>
            <p role="status" aria-live="polite" className={MUTED}>
              {view.workspace?.receiving ?? view.stateLine}
            </p>
          </div>
          {view.admin ? invite : null}
          {participants}
        </>
      ) : confirming ? (
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
          <div className="flex gap-2">
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

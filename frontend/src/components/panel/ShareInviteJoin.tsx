// The Share tab's Invite and Join blocks (`specs/0092-cooperative-editing` UX notes §2.3 and
// §2.5, `specs/0095-workspace` ux-notes section 8).

import { Copy, FolderOpen, LogIn, UserPlus } from "lucide-react";
import { useRef, useState } from "react";

import { APPLY, BUTTON, FIELD, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { SettingField } from "@/components/panel/ShareSettingField";
import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import { invite_link_kind } from "@/wasm-bindings/curvyo_editor_wasm.js";

export /** The Invite block (UX notes §2.3): Invite writes a link valid for 24 hours at once (customer
 * 2026-11-09); the expiry, the link and Copy. */
function InviteBlock({ collab }: { collab: CollaborationApi }) {
  const [open, setOpen] = useState(false);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState("Copy");
  const inviteButton = useRef<HTMLButtonElement>(null);
  const linkField = useRef<HTMLInputElement>(null);
  const close = () => {
    setOpen(false);
    collab.clearInvite();
    window.setTimeout(() => inviteButton.current?.focus(), 0);
  };
  if (!open) {
    return (
      <button ref={inviteButton} type="button" className={`${BUTTON} ${OUTLINE} w-[118px]`}
        onClick={() => {
          setError(collab.createInvite());
          setOpen(true);
        }}
      >
        <UserPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        Invite
      </button>
    );
  }
  const invite = collab.invite;
  return (
    <div
      className="flex flex-col gap-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      {invite ? (
        <>
          <p className={MUTED}>{invite.expiry}</p>
          <input
            ref={linkField}
            readOnly
            aria-label="Invite link"
            className={`${FIELD} font-mono text-xs`}
            value={invite.link}
            onFocus={(event) => event.currentTarget.select()}
          />
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              className={`${BUTTON} ${APPLY} w-[76px]`}
              onClick={() => {
                navigator.clipboard?.writeText(invite.link).then(
                  () => {
                    setCopied("Copied");
                    window.setTimeout(() => setCopied("Copy"), 2000);
                  },
                  () => {
                    linkField.current?.select();
                    setCopied("Select and copy");
                  },
                );
              }}
            >
              <Copy size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
              <span role="status" aria-live="polite">{copied}</span>
            </button>
            <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
              Done
            </button>
          </div>
        </>
      ) : (
        <>
          {error ? (
            <p role="alert" className="text-xs text-[var(--field-invalid)]">
              {error}
            </p>
          ) : null}
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
            Done
          </button>
        </>
      )}
    </div>
  );
}

export /** The Join block (UX notes §2.5, `specs/0095-workspace` ux-notes section 8): link, name if
 * none, for a workspace link the folder, Cancel and Join, then the phases. The link decides the
 * route: `1.` opens a document, `2.` joins a workspace. */
function JoinBlock({ collab, workspace }: { collab: CollaborationApi; workspace?: WorkspaceSyncApi }) {
  const [open, setOpen] = useState(false);
  const [link, setLink] = useState("");
  const join = collab.view.join;
  const phase = join?.phase ?? "";
  const busy = phase === "connecting" || phase === "waiting" || phase === "downloading";
  const refusal = phase === "refused" ? join?.text ?? "" : collab.joinError || (workspace?.joinFolder.refusal ?? "");
  const isWorkspace = invite_link_kind(link.trim()) === "workspace";
  const folder = workspace?.joinFolder.path ?? "";
  if (!open && !busy && !refusal) {
    return (
      <div className="flex flex-col gap-2">
        <button type="button" className={`${BUTTON} ${OUTLINE} w-full`} onClick={() => setOpen(true)}>
          <LogIn size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          Open shared document...
        </button>
        {workspace ? (
          <button type="button" className={`${BUTTON} ${OUTLINE} w-full`} onClick={() => setOpen(true)}>
            <LogIn size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            Join shared workspace...
          </button>
        ) : null}
      </div>
    );
  }
  const joinReason = collab.name === "" ? "Enter a name." : isWorkspace && folder === "" ? "Choose a folder." : "";
  const submit = () => {
    if (link.trim() === "" || joinReason !== "") {
      return;
    }
    if (isWorkspace) {
      if (workspace && folder !== "") {
        workspace.joinWorkspace(link.trim());
      }
      return;
    }
    collab.join(link.trim());
  };
  return (
    <div
      className="flex flex-col gap-2"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !busy) {
          event.preventDefault();
          event.stopPropagation();
          setOpen(false);
        }
      }}
    >
      <label className="flex flex-col gap-1">
        <span className="text-xs text-[var(--toolbar-icon)]">Invite link</span>
        <input
          autoFocus
          className={`${FIELD} font-mono text-xs`}
          value={link}
          aria-invalid={refusal !== ""}
          onChange={(event) => setLink(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              submit();
            }
          }}
        />
      </label>
      {refusal ? (
        <p role="alert" className="text-xs text-[var(--field-invalid)]">
          {refusal}
        </p>
      ) : null}
      {collab.name === "" ? (
        <SettingField label="Name shown to others" value="" onCommit={collab.setName} />
      ) : null}
      {isWorkspace && workspace && !busy ? (
        <div className="flex flex-col gap-1">
          <button type="button" className={`${BUTTON} ${OUTLINE} w-[148px]`} onClick={workspace.chooseJoinFolder}>
            <FolderOpen size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            Choose folder...
          </button>
          {folder ? <p className={`${MUTED} truncate`} title={folder}>{folder}</p> : null}
        </div>
      ) : null}
      {busy ? (
        <p role="status" aria-live="polite" className={MUTED}>
          {join?.text}
        </p>
      ) : null}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          className={`${BUTTON} ${OUTLINE} w-[76px]`}
          onClick={() => {
            if (busy) {
              collab.leave();
            }
            setOpen(false);
          }}
        >
          Cancel
        </button>
        {busy ? null : (
          <button
            type="button"
            aria-disabled={joinReason !== ""}
            className={`${BUTTON} ${APPLY} w-[76px]`}
            onClick={submit}
          >
            Join
          </button>
        )}
      </div>
      {joinReason && !busy ? <p className={MUTED}>{joinReason}</p> : null}
    </div>
  );
}

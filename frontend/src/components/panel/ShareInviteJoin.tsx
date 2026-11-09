// The Share tab's Invite person and Join blocks (`specs/0105-sharing-concept` criteria 11, 16 and
// 19, `specs/0095-workspace` ux-notes section 8): Invite person writes a 24-hour link; Join… has
// one field that takes a person's workspace link or a device code.

import { Copy, FolderOpen, LogIn, UserPlus } from "lucide-react";
import { useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, FIELD, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { JoinerBlock } from "@/components/panel/PairingBlocks";
import { SettingField } from "@/components/panel/ShareSettingField";
import type { CollaborationApi } from "@/hooks/useCollaboration";
import type { DevicesApi } from "@/hooks/useDevices";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import { formatPairCode, pairCodeComplete } from "@/lib/pairCode";
import { invite_link_kind, sharing_word } from "@/wasm-bindings/curvyo_editor_wasm.js";

export /** Invite person: writes a link valid for 24 hours at once (customer 2026-11-09); the
 * expiry, the link and Copy. `shown` shows a link written without a press (Invite person of a
 * Local workspace once Sync is on); `onClose` ends it. */
function InviteBlock({
  collab,
  shown = false,
  onClose,
}: {
  collab: CollaborationApi;
  shown?: boolean;
  onClose?: () => void;
}) {
  const [pressed, setOpen] = useState(false);
  const open = pressed || shown;
  const [error, setError] = useState("");
  const [copied, setCopied] = useState("Copy");
  const inviteButton = useRef<HTMLButtonElement>(null);
  const linkField = useRef<HTMLInputElement>(null);
  const close = () => {
    setOpen(false);
    collab.clearInvite();
    onClose?.();
    window.setTimeout(() => inviteButton.current?.focus(), 0);
  };
  if (!open) {
    return (
      <button ref={inviteButton} type="button" className={`${BUTTON} ${OUTLINE} px-3`}
        onClick={() => {
          setError(collab.createInvite());
          setOpen(true);
        }}
      >
        <UserPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        {sharing_word("invite-person")}
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
              className={`${BUTTON} ${APPLY} px-3`}
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
            <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={close}>
              Done
            </button>
          </div>
        </>
      ) : (
        <>
          {error ? (
            <p role="alert" className={ALERT}>
              {error}
            </p>
          ) : null}
          <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={close}>
            Done
          </button>
        </>
      )}
    </div>
  );
}

/** What the Join field holds: a person's workspace link, a `1.` link, a device code, or not yet
 * any of them. */
function joinKind(text: string): "workspace" | "document" | "code" | "" {
  const kind = invite_link_kind(text);
  if (kind === "workspace" || kind === "document") {
    return kind;
  }
  const raw = text.replace(/[\s-]/g, "");
  return /^[A-Za-z0-9]+$/.test(raw) && pairCodeComplete(raw) && raw.length === 8 ? "code" : "";
}

/**
 * Join… (criteria 16 and 19): one field. A person's `2.` link joins a workspace into a chosen
 * folder (the desktop; the browser says to open it there); a `1.` link is refused with its reason
 * and nothing is sent; a device code joins my devices with the check words of `0096`.
 */
export function JoinEntry({
  collab,
  workspace,
  devices,
  server,
}: {
  collab: CollaborationApi;
  /** The desktop's workspace join; absent in the browser. */
  workspace?: WorkspaceSyncApi;
  devices?: DevicesApi;
  server: string;
}) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  const join = collab.view.join;
  const phase = join?.phase ?? "";
  const busy = phase === "connecting" || phase === "waiting" || phase === "downloading";
  const pairing = devices?.pairing && !devices.pairing.opener ? devices.pairing.view : null;
  const trimmed = text.trim();
  const kind = joinKind(trimmed);
  const paired = devices?.view != null && !devices.view.removed;
  const folder = workspace?.joinFolder.path ?? "";
  // A join that finished closes the field: the workspace block shows it now.
  const finished = phase === "done";
  const [seenFinished, setSeenFinished] = useState(finished);
  if (finished !== seenFinished) {
    setSeenFinished(finished);
    if (finished) {
      setText("");
      setOpen(false);
    }
  }
  // A device code that paired this device closes the field: My devices lists it now.
  const [wasPaired, setWasPaired] = useState(paired);
  if (paired !== wasPaired) {
    setWasPaired(paired);
    if (paired && kind === "code") {
      setText("");
      setOpen(false);
    }
  }
  const refusal =
    phase === "refused"
      ? (join?.text ?? "")
      : kind === "document"
        ? sharing_word("document-link")
        : kind === "workspace" && !workspace
          ? sharing_word("desktop-link")
          : (workspace?.joinFolder.refusal ?? "");
  if (!open && !busy && !pairing && phase !== "refused") {
    return (
      <button type="button" className={`${BUTTON} ${OUTLINE} w-full`} onClick={() => setOpen(true)}>
        <LogIn size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        {sharing_word("join")}
      </button>
    );
  }
  const close = () => {
    if (busy) {
      collab.leave();
    }
    setText("");
    setOpen(false);
  };
  const linkReason = collab.name === "" ? "Enter a name." : folder === "" ? "Choose a folder." : "";
  const joinLink = () => {
    if (kind === "workspace" && workspace && linkReason === "") {
      workspace.joinWorkspace(trimmed);
    }
  };
  return (
    <div
      className="flex flex-col gap-2"
      onKeyDown={(event) => {
        if (event.key === "Escape" && !busy && !pairing) {
          event.preventDefault();
          event.stopPropagation();
          close();
        }
      }}
    >
      {pairing ? null : (
        <label className="flex flex-col gap-1">
          <span className="text-xs text-[var(--toolbar-icon)]">{sharing_word("join-field")}</span>
          <input
            autoFocus
            autoComplete="off"
            spellCheck={false}
            className={`${FIELD} font-mono text-xs`}
            value={text}
            aria-invalid={refusal !== ""}
            readOnly={busy}
            onChange={(event) => setText(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                joinLink();
              }
            }}
          />
        </label>
      )}
      {refusal ? (
        <p role="alert" className={ALERT}>
          {refusal}
        </p>
      ) : null}
      {(kind === "code" && devices && !paired) || pairing ? (
        devices ? (
          <JoinerBlock devices={devices} view={pairing} server={server} code={kind === "code" ? formatPairCode(trimmed) : undefined} onClose={close} />
        ) : null
      ) : (
        <>
          {kind === "workspace" && workspace && collab.name === "" ? (
            <SettingField label="Name shown to others" value="" onCommit={collab.setName} />
          ) : null}
          {kind === "workspace" && workspace && !busy ? (
            <div className="flex flex-col gap-1">
              <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={workspace.chooseJoinFolder}>
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
            <button type="button" className={`${BUTTON} ${OUTLINE} px-3`} onClick={close}>
              Cancel
            </button>
            {busy || kind !== "workspace" || !workspace ? null : (
              <button type="button" aria-disabled={linkReason !== ""} className={`${BUTTON} ${APPLY} px-3`} onClick={joinLink}>
                Join
              </button>
            )}
          </div>
          {kind === "workspace" && workspace && linkReason && !busy ? <p className={MUTED}>{linkReason}</p> : null}
        </>
      )}
    </div>
  );
}

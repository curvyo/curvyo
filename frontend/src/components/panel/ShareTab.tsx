import { Copy, FolderOpen, LogIn, Share2, UserMinus, UserPlus } from "lucide-react";
import { useEffect, useId, useRef, useState } from "react";

import { PanelRadioGroup } from "@/components/panel/PanelRadioGroup";
import { WorkspaceShareBlock } from "@/components/panel/WorkspaceShareBlock";
import type { CollaborationApi, ShareParticipant } from "@/hooks/useCollaboration";
import type { WorkspaceSyncApi } from "@/hooks/useWorkspaceSync";
import {
  invite_link_kind,
  invite_sentence,
  normalize_server_address,
  removal_confirm_text,
  server_shown,
  share_confirm_text,
  workspace_invite_sentence,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const BUTTON =
  "inline-flex h-7 shrink-0 items-center justify-center gap-1.5 rounded-[5px] border text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9 aria-disabled:opacity-40";
const OUTLINE =
  "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";
const APPLY =
  "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:brightness-110";
const FIELD =
  "h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-[var(--background)] px-2 text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-invalid:border-[var(--field-invalid)]";
const MUTED = "text-xs text-[var(--panel-muted-fg)]";
const DESTRUCTIVE =
  "border-[var(--destructive)] bg-[var(--background)] text-[var(--destructive)] hover:bg-[var(--destructive)] hover:text-white focus-visible:bg-[var(--destructive)] focus-visible:text-white";

/** How long a just-shown destructive confirm ignores presses (design system "Inline confirm"). */
const CONFIRM_GUARD_MS = 400;

const VALIDITY: readonly (readonly ["3600" | "86400" | "604800", string])[] = [
  ["3600", "1 hour"],
  ["86400", "24 hours"],
  ["604800", "7 days"],
];

/** The open workspace as the Share tab's Workspace block needs it (`specs/0094-workspace`). */
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
}

/** A labelled text field that commits on Enter or when it loses focus, Escape restores. */
function SettingField({
  label,
  value,
  placeholder,
  onCommit,
  readOnly = false,
  mono = false,
  autoFocus = false,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onCommit: (text: string) => string;
  readOnly?: boolean;
  mono?: boolean;
  autoFocus?: boolean;
}) {
  const [text, setText] = useState(value);
  const [message, setMessage] = useState("");
  const id = useId();
  const commit = () => {
    if (text === value) {
      setMessage("");
      return;
    }
    const refusal = onCommit(text);
    setMessage(refusal);
  };
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-xs text-[var(--toolbar-icon)]">
        {label}
      </label>
      <input
        id={id}
        className={`${FIELD} ${mono ? "font-mono text-xs" : ""}`}
        value={text}
        placeholder={placeholder}
        readOnly={readOnly}
        autoFocus={autoFocus}
        aria-invalid={message !== ""}
        aria-describedby={message ? `${id}-message` : undefined}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => !readOnly && commit()}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            setText(value);
            setMessage("");
          }
        }}
      />
      {message ? (
        <p id={`${id}-message`} className="text-xs text-[var(--field-invalid)]">
          {message}
        </p>
      ) : null}
    </div>
  );
}

/** A participant's disc: initials on the participant's colour (UX notes §4). */
export function AvatarDisc({ participant, size = 24 }: { participant: Pick<ShareParticipant, "initials" | "colour" | "online">; size?: number }) {
  return (
    <span
      aria-hidden
      className="inline-flex shrink-0 items-center justify-center rounded-full text-[11px] font-bold text-white"
      style={{
        width: size,
        height: size,
        background: `var(--peer-${participant.colour})`,
        opacity: participant.online ? 1 : 0.55,
      }}
    >
      {participant.initials}
    </span>
  );
}

/**
 * One participant (UX notes §2.4). An admin gets Remove in the row's action slot on the other
 * rows; it asks with the sentence of criterion 32 in place before anything is written.
 */
function ParticipantRow({ participant, onRemove }: { participant: ShareParticipant; onRemove: (member: string) => string }) {
  const [asking, setAsking] = useState(false);
  const [error, setError] = useState("");
  const removeButton = useRef<HTMLButtonElement>(null);
  // Design system "Inline confirm": the action ignores presses for its first 400 ms, so a
  // double click on the row's Remove cannot confirm.
  const armed = useRef(false);
  useEffect(() => {
    armed.current = false;
    if (!asking) {
      return;
    }
    const handle = window.setTimeout(() => {
      armed.current = true;
    }, CONFIRM_GUARD_MS);
    return () => window.clearTimeout(handle);
  }, [asking]);
  const close = () => {
    setAsking(false);
    window.setTimeout(() => removeButton.current?.focus(), 0);
  };
  const sentence = removal_confirm_text(participant.label);
  return (
    <li className="flex flex-col">
      <div className="flex h-10 items-center gap-2 pointer-coarse:h-11">
        <AvatarDisc participant={participant} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm text-[var(--toolbar-icon)]">
            {participant.label}
            {participant.self ? <span className="text-[var(--panel-muted-fg)]"> this device</span> : null}
          </p>
          <p className="truncate font-mono text-xs text-[var(--panel-muted-fg)]">{participant.fingerprint}</p>
        </div>
        {participant.online ? null : <span className={MUTED}>offline</span>}
        {participant.removable ? (
          <button
            ref={removeButton}
            type="button"
            aria-label={`Remove ${participant.label}`}
            title="Remove"
            className={`${BUTTON} ${OUTLINE} w-7 border-transparent`}
            onClick={() => {
              setError("");
              setAsking(true);
            }}
          >
            <UserMinus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          </button>
        ) : (
          <span aria-hidden className="w-7 shrink-0" />
        )}
      </div>
      {asking ? (
        <div
          role="group"
          aria-label={sentence}
          className="flex flex-col gap-2 pb-2"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              close();
            }
          }}
        >
          <p className="text-xs text-[var(--toolbar-icon)]">{sentence}</p>
          <div className="flex gap-2">
            <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
              Cancel
            </button>
            <button
              type="button"
              className={`${BUTTON} ${DESTRUCTIVE} w-[76px]`}
              onClick={() => {
                if (!armed.current) {
                  return;
                }
                const refusal = onRemove(participant.member);
                setError(refusal);
                if (refusal === "") {
                  setAsking(false);
                }
              }}
            >
              Remove
            </button>
          </div>
        </div>
      ) : null}
      {error ? (
        <p role="alert" className="pb-2 text-xs text-[var(--field-invalid)]">
          {error}
        </p>
      ) : null}
    </li>
  );
}

/** The Invite block (UX notes §2.3): validity, Create link, the sentence, the link and Copy. */
function InviteBlock({
  collab,
  onReturnFocus,
  sentence = invite_sentence(),
}: {
  collab: CollaborationApi;
  onReturnFocus: () => void;
  sentence?: string;
}) {
  const [open, setOpen] = useState(false);
  const [validity, setValidity] = useState<"3600" | "86400" | "604800">("86400");
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
      <button ref={inviteButton} type="button" className={`${BUTTON} ${OUTLINE} w-[118px]`} onClick={() => setOpen(true)}>
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
          <p className="text-xs text-[var(--toolbar-icon)]">
            {sentence} {invite.expiry}
          </p>
          <input
            ref={linkField}
            readOnly
            aria-label="Invite link"
            className={`${FIELD} font-mono text-xs`}
            value={invite.link}
            onFocus={(event) => event.currentTarget.select()}
          />
          <div className="flex gap-2">
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
              <PanelRadioGroup
                legend="Valid for"
                options={VALIDITY}
                value={validity}
                onChange={setValidity}
                onReturnFocus={onReturnFocus}
                stacked
              />
              <button
                type="button"
                className={`${BUTTON} ${APPLY} w-[118px]`}
                onClick={() => setError(collab.createInvite(Number(validity)))}
              >
                Create link
              </button>
              {error ? (
                <p role="alert" className="text-xs text-[var(--field-invalid)]">
                  {error}
                </p>
              ) : null}
        </>
      )}
    </div>
  );
}

/** The Join block (UX notes §2.5, `specs/0094-workspace` ux-notes section 8): link, name if
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
      <div className="flex gap-2">
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

/**
 * The Share tab (`specs/0092-cooperative-editing` UX notes §2): the server and name settings,
 * Share with its inline confirm, Invite, the participant list and Open shared document.
 */
export function ShareTab({ collab, saved, onReturnFocus, workspace }: ShareTabProps) {
  const { view } = collab;
  // A session that carries a shared workspace's link shows the workspace in the Workspace block
  // and only a line in the Document block (`specs/0094-workspace` ux-notes section 9).
  const workspaceView = view.workspace ?? null;
  const [confirming, setConfirming] = useState(false);
  const [sharing, setSharing] = useState(false);
  const [shareError, setShareError] = useState("");
  const reasonId = useId();
  const shared = view.shared && view.live;
  const server = normalize_server_address(collab.server);
  const { checkServer } = collab;
  // A check runs when the tab is shown with a server set, never in the background otherwise.
  useEffect(() => {
    if (server !== "" && !shared) {
      checkServer();
    }
  }, [server, shared, checkServer]);

  const serverReason =
    server === ""
      ? "Set a server address."
      : collab.reach === "unreachable"
        ? "Cannot reach the server."
        : collab.name === ""
          ? "Enter a name."
          : "";
  const reason = !saved ? "Save the document first." : serverReason;
  const participantList =
    view.participants.length > 0 ? (
      <div className="flex flex-col gap-1">
        <div className="flex items-baseline justify-between">
          <span className="text-xs text-[var(--toolbar-icon)]">Participants</span>
          {view.epoch !== null && view.epoch !== undefined ? (
            <span className={MUTED}>Key epoch {view.epoch}</span>
          ) : null}
        </div>
        <ul aria-label="Participants" className="flex flex-col">
          {view.participants.map((participant) => (
            <ParticipantRow key={participant.fingerprint} participant={participant} onRemove={collab.remove} />
          ))}
        </ul>
      </div>
    ) : null;
  const reachLine =
    collab.reach === "reaching"
      ? "Reaching..."
      : collab.reach === "reachable"
        ? "Reachable"
        : collab.reach === "unreachable"
          ? `Cannot reach ${server_shown(server)}.`
          : "";
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-col gap-2">
        {view.live && (view.shared || workspaceView != null) ? (
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
              <p role="status" aria-live="polite" className={MUTED}>
                {reachLine}
              </p>
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
              <InviteBlock collab={collab} onReturnFocus={onReturnFocus} sentence={workspace_invite_sentence()} />
            ) : null
          }
          participants={workspaceView ? participantList : null}
        />
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
          {shared ? (
            <div className="flex flex-col gap-1">
              <p className="text-sm text-[var(--toolbar-icon)]">Shared on {view.serverShown}</p>
              <p role="status" aria-live="polite" className={MUTED}>
                {view.stateLine}
              </p>
            </div>
          ) : view.ended === "removed" ? null : confirming ? (
            <div
              role="group"
              aria-label={share_confirm_text(server)}
              className="flex flex-col gap-2"
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  event.preventDefault();
                  event.stopPropagation();
                  setConfirming(false);
                }
              }}
            >
              <p className="text-xs text-[var(--toolbar-icon)]">{share_confirm_text(server)}</p>
              <div className="flex gap-2">
                <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={() => setConfirming(false)}>
                  Cancel
                </button>
                <button
                  type="button"
                  className={`${BUTTON} ${APPLY} w-[76px]`}
                  onClick={() => {
                    setConfirming(false);
                    setShareError("");
                    setSharing(true);
                    void collab.share().then((error) => {
                      setSharing(false);
                      if (error) {
                        setShareError(`Could not share: ${error}`);
                      }
                    });
                  }}
                >
                  Share
                </button>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-1">
              <button
                type="button"
                aria-disabled={reason !== "" || sharing}
                aria-busy={sharing}
                aria-describedby={reason ? reasonId : undefined}
                className={`${BUTTON} ${APPLY} w-[118px]`}
                onClick={() => {
                  if (reason === "" && !sharing) {
                    setConfirming(true);
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

          {shared && view.admin ? <InviteBlock collab={collab} onReturnFocus={onReturnFocus} /> : null}

          {participantList}
        </>
      )}

      {shared && !workspaceView?.joining ? null : <JoinBlock collab={collab} workspace={workspace?.sync} />}
    </div>
  );
}

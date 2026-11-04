// The browser's passkey key backup in My devices (`specs/0096-own-devices-sync` ux-notes 9.1,
// ADR 0040 decision 13): on a paired browser Back up with passkey (an Apply confirm naming the
// server) or Remove backup (a destructive confirm); on a browser without a group Restore with
// passkey beside Join my devices. One status or alert line under them.

import { KeyRound } from "lucide-react";
import { useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, DestructiveConfirm, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { useReturnFocus } from "@/hooks/useReturnFocus";
import type { KeyBackupApi } from "@/hooks/useKeyBackup";
import { key_backup_sentence, key_backup_text, normalize_server_address } from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The busy line or the last run's sentence. */
function BackupLines({ backup }: { backup: KeyBackupApi }) {
  if (backup.busy) {
    return (
      <p role="status" aria-live="polite" className={MUTED}>
        {backup.busy}
      </p>
    );
  }
  if (!backup.message) {
    return null;
  }
  return backup.message.alert ? (
    <p role="alert" className={ALERT}>
      {backup.message.text}
    </p>
  ) : (
    <p role="status" aria-live="polite" className={MUTED}>
      {backup.message.text}
    </p>
  );
}

/** Back up with passkey or Remove backup, on a paired browser. */
export function KeyBackupRow({ backup, server }: { backup: KeyBackupApi; server: string }) {
  const [confirming, setConfirming] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  // Cancel, Escape and the action return the focus to the button the confirm replaced.
  const armReturn = useReturnFocus(!confirming, () => button.current);
  const close = () => {
    armReturn();
    setConfirming(false);
  };
  if (!backup.available) {
    return <p className={MUTED}>{key_backup_text("no-prf")}</p>;
  }
  const busy = backup.busy !== "";
  return (
    <div className="flex flex-col gap-1">
      {confirming && backup.backedUp ? (
        <DestructiveConfirm
          text={key_backup_text("remove-confirm")}
          label={key_backup_text("remove")}
          action="Remove"
          onCancel={close}
          onConfirm={() => {
            close();
            backup.remove();
            return "";
          }}
        />
      ) : confirming ? (
        <div
          role="group"
          aria-label={key_backup_text("back-up")}
          className="flex flex-col gap-2 py-1"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              close();
            }
          }}
        >
          <p className="text-xs text-[var(--toolbar-icon)]">{key_backup_sentence(normalize_server_address(server))}</p>
          <div className="flex gap-2">
            <button autoFocus type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
              Cancel
            </button>
            <button
              type="button"
              className={`${BUTTON} ${APPLY} w-[76px]`}
              onClick={() => {
                close();
                backup.backUp();
              }}
            >
              Back up
            </button>
          </div>
        </div>
      ) : (
        <button
          ref={button}
          type="button"
          aria-disabled={busy}
          className={`${BUTTON} ${OUTLINE} w-[176px]`}
          onClick={() => {
            if (!busy) {
              setConfirming(true);
            }
          }}
        >
          <KeyRound size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
          {key_backup_text(backup.backedUp ? "remove" : "back-up")}
        </button>
      )}
      <BackupLines backup={backup} />
    </div>
  );
}

/** Restore with passkey, on a browser without a group; `reason` disables it like Join. */
export function RestoreButton({
  backup,
  server,
  reason,
  reasonId,
}: {
  backup: KeyBackupApi;
  server: string;
  reason: string;
  reasonId: string;
}) {
  if (!backup.available) {
    return null;
  }
  const blocked = reason !== "" || backup.busy !== "";
  return (
    <button
      type="button"
      aria-disabled={blocked}
      aria-describedby={reason ? reasonId : undefined}
      className={`${BUTTON} ${OUTLINE} w-[176px]`}
      onClick={() => {
        if (!blocked) {
          backup.restore(server);
        }
      }}
    >
      <KeyRound size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      {key_backup_text("restore")}
    </button>
  );
}

export { BackupLines };

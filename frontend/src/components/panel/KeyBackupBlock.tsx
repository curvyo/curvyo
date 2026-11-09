// The browser's passkey key backup in My devices (`specs/0096-own-devices-sync` ux-notes 9.1,
// ADR 0040 decision 13): on a paired browser Back up with passkey (at once; the passkey prompt
// confirms) or Remove backup (an armed button); on a browser without a group Restore with
// passkey beside Join my devices. One status or alert line under them.

import { KeyRound } from "lucide-react";

import { ALERT, ARMED, ArmLines, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { useArm } from "@/hooks/useArm";
import type { KeyBackupApi } from "@/hooks/useKeyBackup";
import { key_backup_text } from "@/wasm-bindings/curvyo_editor_wasm.js";

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
export function KeyBackupRow({ backup }: { backup: KeyBackupApi }) {
  const arm = useArm("key-backup");
  if (!backup.available) {
    return <p className={MUTED}>{key_backup_text("no-prf")}</p>;
  }
  const busy = backup.busy !== "";
  return (
    <div className="flex flex-col gap-1">
      <span {...arm.zone} className="flex">
          <button
            {...arm.control}
            type="button"
            aria-disabled={busy}
            className={`${BUTTON} ${arm.armed ? ARMED : OUTLINE} w-[176px]`}
            onClick={() => {
              if (busy) {
                return;
              }
              if (backup.backedUp) {
                // Remove backup is armed: a second press removes it (customer feedback, PR #281).
                arm.press(() => {
                  backup.remove();
                  return "";
                });
              } else {
                // Back up asks nothing: the passkey prompt is the confirmation (customer
                // 2026-11-09: no confirm sentences in the Share tab).
                backup.backUp();
              }
            }}
          >
            <KeyRound size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            {key_backup_text(backup.backedUp ? "remove" : "back-up")}
          </button>
      </span>
      {backup.backedUp ? <ArmLines arm={arm} /> : null}
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

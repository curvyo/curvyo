// The choices My devices asks for (`specs/0096-own-devices-sync` ux-notes 2.3 and 3.1): the
// decision row for a copy that is here already, and the workspaces to sync right after Pair.

import { useEffect, useId, useRef, useState } from "react";

import { MUTED } from "@/components/panel/devicesUi";
import type { DevicesApi } from "@/hooks/useDevices";
import type { OfferedWorkspace } from "@/hooks/useSyncedWorkspaces";
import { firstSyncAnswer } from "@/lib/librarySyncView";
import {
  devices_already_synced,
  devices_documents_text,
  devices_text,
  devices_workspaces_for,
} from "@/wasm-bindings/curvyo_editor_wasm.js";
import { Button } from "@/components/ui/button";
import { Copy, Plug, RefreshCw, X } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { SECTION_HEADING } from "@/components/ui/looks";

/** A copy of a synced workspace that is here already (ux-notes 2.3, criterion 14). */
export function DecisionRow({ devices, id, name, text }: { devices: DevicesApi; id: string; name: string; text: string }) {
  return (
    <div role="group" aria-label={name} className="flex flex-col gap-2 py-1">
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      <div className="flex flex-wrap gap-2">
        <Button variant="primary" icon={Plug} className="min-w-[148px]" onClick={() => devices.connectCopy(id)}>
          Connect this folder
        </Button>
        <Button icon={Copy} className="min-w-[148px]" onClick={() => devices.secondCopy(id)}>
          Make a second copy
        </Button>
      </div>
    </div>
  );
}

/** The libraries' first-sync confirm (`specs/0116-library-sync` criterion 3): the spec's
 * sentence with the choice to confirm (every item on) or decline (nothing leaves; each item can
 * be switched on from its row). Its decline is "Keep local", so the pick block has one "Not now"
 * (its own, for the workspaces) and the two groups cannot be mistaken for each other. */
export function LibrarySyncConfirm({
  sentence,
  onSync,
  onDecline,
}: {
  sentence: string;
  onSync: () => void;
  onDecline: () => void;
}) {
  return (
    <div role="group" aria-label="Sync to my devices" className="flex flex-col gap-2">
      <p className="text-xs text-[var(--toolbar-icon)]">{sentence}</p>
      <div className="flex flex-wrap gap-2">
        <Button variant="primary" icon={RefreshCw}
          onClick={() => firstSyncAnswer(true, onSync, onDecline)}
        >
          Sync libraries
        </Button>
        <Button icon={X}
          onClick={() => firstSyncAnswer(false, onSync, onDecline)}
        >
          Keep local
        </Button>
      </div>
    </div>
  );
}

/** Choosing workspaces right after Pair (ux-notes 3.1, criterion 8): the open and recent
 * workspaces that are not synced yet, all checked; the synced ones are named above the list.
 * The libraries' first-sync confirm sits under the list, where the workspaces' own confirm
 * sentence sits. */
export function PickWorkspaces({
  label,
  frozenWarning,
  offered,
  synced,
  libraryConfirm,
  onLibrarySync,
  onLibraryDecline,
  onDone,
}: {
  label: string;
  /** The open synced workspace's frozen-room warning (`specs/0097` ux-notes 3.2), or empty. */
  frozenWarning: string;
  offered: OfferedWorkspace[];
  /** Ids of workspaces synced already. */
  synced: string[];
  /** The libraries' confirm sentence (criterion 3), or "" while the ask does not apply. */
  libraryConfirm: string;
  /** Confirms the libraries' first sync. */
  onLibrarySync: () => void;
  /** Declines it: the libraries' items stay local. */
  onLibraryDecline: () => void;
  onDone: (chosen: OfferedWorkspace[]) => void;
}) {
  const rows = offered.filter((o) => !synced.includes(o.id));
  const already = offered.filter((o) => synced.includes(o.id)).map((o) => o.name);
  // A workspace where this device is only a member starts unchecked and stays so (criterion 17).
  const [unchecked, setUnchecked] = useState<string[]>(() => rows.filter((r) => r.memberOnly).map((r) => r.id));
  const chosen = rows.filter((row) => !unchecked.includes(row.id) && !row.memberOnly);
  const notNow = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (rows.length === 0) {
      notNow.current?.focus();
    }
  }, [rows.length]);
  const reasonId = useId();
  const alreadyLine = devices_already_synced(JSON.stringify(already), label);
  return (
    <div
      className="flex flex-col gap-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onDone([]);
        }
      }}
    >
      <p className={SECTION_HEADING}>{devices_workspaces_for(label)}</p>
      {alreadyLine ? <p className={MUTED}>{alreadyLine}</p> : null}
      {rows.length > 0 ? (
        <ul aria-label="Workspaces to sync" className="flex max-h-[160px] flex-col overflow-y-auto">
          {rows.map((row, index) => (
            <li key={row.id}>
              <label className="flex min-h-8 items-center gap-2 text-sm text-[var(--toolbar-icon)] pointer-coarse:min-h-9">
                <Checkbox
                  autoFocus={index === 0}
                  aria-disabled={row.memberOnly || undefined}
                  aria-label={row.name}
                  checked={!unchecked.includes(row.id)}
                  className={row.memberOnly ? "opacity-40" : ""}
                  onClick={() => {
                    if (row.memberOnly) {
                      return;
                    }
                    setUnchecked(unchecked.includes(row.id) ? unchecked.filter((id) => id !== row.id) : [...unchecked, row.id]);
                  }}
                />
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate" title={row.name}>
                    {row.name}
                  </span>
                  {row.memberOnly ? <span className={MUTED}>{devices_text("not-admin-here")}</span> : null}
                </span>
                <span className={MUTED}>{devices_documents_text(row.documents)}</span>
              </label>
            </li>
          ))}
        </ul>
      ) : null}
      {libraryConfirm ? <LibrarySyncConfirm sentence={libraryConfirm} onSync={onLibrarySync} onDecline={onLibraryDecline} /> : null}
      {frozenWarning ? <p className="text-xs text-[var(--toolbar-icon)]">{frozenWarning}</p> : null}
      <div className="flex flex-wrap gap-2">
        <Button icon={X} ref={notNow} className="min-w-[90px]" onClick={() => onDone([])}>
          Not now
        </Button>
        <Button variant="primary" icon={RefreshCw}
          aria-disabled={chosen.length === 0}
          aria-describedby={chosen.length > 0 ? undefined : reasonId} className="min-w-[100px]"
          onClick={() => {
            if (chosen.length > 0) {
              onDone(chosen);
            }
          }}
        >
          Sync these
        </Button>
      </div>
      {chosen.length > 0 ? null : (
        <p id={reasonId} className={MUTED}>
          Choose a workspace.
        </p>
      )}
    </div>
  );
}

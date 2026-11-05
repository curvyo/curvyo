// The choices My devices asks for (`specs/0096-own-devices-sync` ux-notes 2.3 and 3.1): the
// decision row for a copy that is here already, and the workspaces to sync right after Pair.

import { useEffect, useId, useRef, useState } from "react";

import { APPLY, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import type { DevicesApi } from "@/hooks/useDevices";
import type { OfferedWorkspace } from "@/hooks/useSyncedWorkspaces";
import {
  devices_already_synced,
  devices_documents_text,
  devices_privacy_sentence,
  devices_text,
  devices_workspaces_for,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** A copy of a synced workspace that is here already (ux-notes 2.3, criterion 14). */
export function DecisionRow({ devices, id, name, text }: { devices: DevicesApi; id: string; name: string; text: string }) {
  return (
    <div role="group" aria-label={name} className="flex flex-col gap-2 py-1">
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      <div className="flex flex-wrap gap-2">
        <button type="button" className={`${BUTTON} ${APPLY} w-[148px]`} onClick={() => devices.connectCopy(id)}>
          Connect this folder
        </button>
        <button type="button" className={`${BUTTON} ${OUTLINE} w-[148px]`} onClick={() => devices.secondCopy(id)}>
          Make a second copy
        </button>
      </div>
    </div>
  );
}

/** Choosing workspaces right after Pair (ux-notes 3.1, criterion 8): the open and recent
 * workspaces that are not synced yet, all checked; the synced ones are named above the list. */
export function PickWorkspaces({
  label,
  frozenWarning,
  offered,
  synced,
  onDone,
}: {
  label: string;
  /** The open synced workspace's frozen-room warning (`specs/0097` ux-notes 3.2), or empty. */
  frozenWarning: string;
  offered: OfferedWorkspace[];
  /** Ids of workspaces synced already. */
  synced: string[];
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
      <p className="text-sm font-semibold text-[var(--toolbar-icon)]">{devices_workspaces_for(label)}</p>
      {alreadyLine ? <p className={MUTED}>{alreadyLine}</p> : null}
      {rows.length > 0 ? (
        <ul aria-label="Workspaces to sync" className="flex max-h-[160px] flex-col overflow-y-auto">
          {rows.map((row, index) => (
            <li key={row.id}>
              <label className="flex min-h-8 items-center gap-2 text-sm text-[var(--toolbar-icon)] pointer-coarse:min-h-9">
                <input
                  autoFocus={index === 0}
                  type="checkbox"
                  aria-disabled={row.memberOnly || undefined}
                  checked={!unchecked.includes(row.id)}
                  onChange={(event) => {
                    if (row.memberOnly) {
                      return;
                    }
                    setUnchecked(event.target.checked ? unchecked.filter((id) => id !== row.id) : [...unchecked, row.id]);
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
      <p className="text-xs text-[var(--toolbar-icon)]">
        {frozenWarning ? `${devices_privacy_sentence()} ${frozenWarning}` : devices_privacy_sentence()}
      </p>
      <div className="flex flex-wrap gap-2">
        <button ref={notNow} type="button" className={`${BUTTON} ${OUTLINE} w-[90px]`} onClick={() => onDone([])}>
          Not now
        </button>
        <button
          type="button"
          aria-disabled={chosen.length === 0}
          aria-describedby={chosen.length > 0 ? undefined : reasonId}
          className={`${BUTTON} ${APPLY} w-[100px]`}
          onClick={() => {
            if (chosen.length > 0) {
              onDone(chosen);
            }
          }}
        >
          Sync these
        </button>
      </div>
      {chosen.length > 0 ? null : (
        <p id={reasonId} className={MUTED}>
          Choose a workspace.
        </p>
      )}
    </div>
  );
}

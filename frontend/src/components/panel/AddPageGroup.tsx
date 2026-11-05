import { createElement, type KeyboardEvent, useEffect, useRef, useState } from "react";

import type { MachineProfileRow } from "@/hooks/useWorkbenches";
import type { PageKindEntry } from "@/workbenches/registry";
import { glyphOf } from "@/workbenches/iconNames";

/** Where the kind last added on this computer is kept (UX notes §2, per OS user, not the file). */
const KIND_KEY = "curvyo.addPage.kind";

function storedKind(): string | null {
  try {
    return window.localStorage.getItem(KIND_KEY);
  } catch {
    return null;
  }
}

function storeKind(kind: string) {
  try {
    window.localStorage.setItem(KIND_KEY, kind);
  } catch {
    // Unreadable storage: the next group starts with Drawing.
  }
}

const SEGMENT =
  "flex h-7 min-w-0 flex-1 basis-0 items-center justify-center gap-1 rounded-md px-1 text-[13px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-checked:bg-[var(--toolbar-icon-active-bg)] aria-checked:text-[var(--toolbar-icon-active-fg)] [@media(pointer:coarse)]:h-9";
const TEXT_BUTTON =
  "h-7 rounded-md px-3 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] [@media(pointer:coarse)]:h-9";

/**
 * The Add page group (`specs/0098-page-kinds` criterion 1, UX notes §2): inline under the page
 * list, no popup. The Kind switch offers the registered kinds; Laser (any output kind) shows the
 * machine list. Add (or Enter) adds and closes, Cancel (or Escape) closes and changes nothing.
 */
export function AddPageGroup({
  kinds,
  profiles,
  onAdd,
  onCancel,
}: {
  kinds: PageKindEntry[];
  profiles: MachineProfileRow[];
  /** Adds a page of `kind` with the machine `profile` (empty for a page without one). */
  onAdd: (kind: string, profile: string) => void;
  onCancel: () => void;
}) {
  const remembered = storedKind();
  const [kind, setKind] = useState(
    kinds.some((k) => k.id === remembered) ? (remembered as string) : "drawing",
  );
  const chosen = profiles.find((p) => p.chosen) ?? profiles[0];
  const [profile, setProfile] = useState(chosen?.id ?? "");
  const kindRow = useRef<HTMLDivElement>(null);
  const machineRow = useRef<HTMLDivElement>(null);
  const entry = kinds.find((k) => k.id === kind);
  const output = entry?.workbench === "manufacturing";

  useEffect(() => {
    kindRow.current?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  }, []);
  useEffect(() => {
    machineRow.current
      ?.querySelector<HTMLElement>('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [profile, output]);

  const add = () => {
    storeKind(kind);
    onAdd(kind, output ? profile : "");
  };
  const onKindKey = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = kinds[Math.max(0, Math.min(kinds.length - 1, index + step))];
    setKind(next.id);
    requestAnimationFrame(() =>
      kindRow.current?.querySelector<HTMLElement>(`[data-kind="${CSS.escape(next.id)}"]`)?.focus(),
    );
  };
  const onMachineKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = profiles.findIndex((p) => p.id === profile);
    const target =
      event.key === "ArrowDown"
        ? index + 1
        : event.key === "ArrowUp"
          ? index - 1
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? profiles.length - 1
              : null;
    if (target === null) return;
    event.preventDefault();
    const next = profiles[Math.max(0, Math.min(profiles.length - 1, target))];
    if (next) setProfile(next.id);
  };

  return (
    <div
      role="group"
      aria-label="Add page"
      className="flex flex-col gap-1 rounded-md bg-[var(--toolbar-bg)] p-2"
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Escape") {
          event.preventDefault();
          onCancel();
        } else if (event.key === "Enter" && !(event.target as HTMLElement).dataset.cancel) {
          event.preventDefault();
          add();
        }
      }}
    >
      <span className="text-xs text-[var(--panel-muted-fg)]" id="add-page-kind">
        Kind
      </span>
      <div
        ref={kindRow}
        role="radiogroup"
        aria-labelledby="add-page-kind"
        className="flex flex-wrap gap-1"
      >
        {kinds.map((k, index) => (
          <button
            key={k.id}
            type="button"
            role="radio"
            data-kind={k.id}
            aria-checked={k.id === kind}
            tabIndex={k.id === kind ? 0 : -1}
            className={SEGMENT}
            onClick={() => setKind(k.id)}
            onKeyDown={(event) => onKindKey(event, index)}
          >
            {createElement(glyphOf(k.icon), {
              size: 14,
              strokeWidth: 1.5,
              absoluteStrokeWidth: true,
              "aria-hidden": true,
            })}
            <span className="truncate">{k.label}</span>
          </button>
        ))}
      </div>
      {output && (
        <>
          <span className="mt-1 text-xs text-[var(--panel-muted-fg)]" id="add-page-machine">
            Machine
          </span>
          <div
            ref={machineRow}
            role="listbox"
            aria-labelledby="add-page-machine"
            tabIndex={0}
            className="max-h-[176px] overflow-y-auto rounded-md outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
            aria-activedescendant={profile ? `add-page-profile-${profile}` : undefined}
            onKeyDown={onMachineKey}
          >
            {profiles.map((p) => (
              <div
                key={p.id}
                id={`add-page-profile-${p.id}`}
                role="option"
                aria-selected={p.id === profile}
                className="flex h-11 cursor-default flex-col justify-center rounded-md px-2 text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] aria-selected:bg-[var(--toolbar-icon-active-bg)] aria-selected:text-[var(--toolbar-icon-active-fg)]"
                onClick={() => setProfile(p.id)}
              >
                <span className="truncate">{p.name}</span>
                <span className="text-xs tabular-nums opacity-70">
                  {`${p.width} × ${p.height} mm`}
                </span>
              </div>
            ))}
          </div>
        </>
      )}
      <div className="mt-1 flex justify-end gap-1">
        <button type="button" data-cancel="true" className={TEXT_BUTTON} onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className={TEXT_BUTTON} onClick={add}>
          Add
        </button>
      </div>
    </div>
  );
}

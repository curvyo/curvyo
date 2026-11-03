import { ChevronDown, ChevronRight, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import type { WorkspaceApi } from "@/hooks/useWorkspace";

/** At most this many Deleted rows are shown (`ux-notes.md` section 5). */
const MAX_DELETED_ROWS = 200;

const OUTLINE_SMALL =
  "h-6 shrink-0 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent px-2 text-xs text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9";

/**
 * The Deleted list of a shared workspace (`specs/0094-workspace` criterion 40, `ux-notes.md`
 * section 5): a disclosure with the count, collapsed by default; each row names the document,
 * who deleted it and when, with Restore. Shown only while the workspace is shared.
 */
export function WorkspaceDeleted({ workspace, emptyText }: { workspace: WorkspaceApi; emptyText: string }) {
  const [open, setOpen] = useState(false);
  const rows = workspace.deleted;
  const shown = rows.slice(0, MAX_DELETED_ROWS);
  const list = useRef<HTMLUListElement>(null);
  const disclosure = useRef<HTMLButtonElement>(null);
  // The row of a Restore leaves with the next scan: the focus goes to the Restore that took its
  // place, else the one before it, else the disclosure.
  const restored = useRef<{ index: number; count: number } | null>(null);
  useEffect(() => {
    const pending = restored.current;
    if (!pending || rows.length === pending.count) {
      return;
    }
    restored.current = null;
    const buttons = list.current?.querySelectorAll<HTMLButtonElement>("button") ?? [];
    const next = buttons[Math.min(pending.index, buttons.length - 1)] ?? disclosure.current;
    next?.focus();
  }, [rows]);
  return (
    <div className="mt-2 flex flex-col">
      <button
        ref={disclosure}
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="flex h-7 items-center gap-1 rounded-[5px] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {open ? <ChevronDown size={16} strokeWidth={1.5} aria-hidden /> : <ChevronRight size={16} strokeWidth={1.5} aria-hidden />}
        <span className="flex-1 text-left">Deleted</span>
        <span className="pr-1 text-xs text-[var(--panel-muted-fg)]">{rows.length}</span>
      </button>
      {open ? (
        rows.length === 0 ? (
          <p className="px-1 text-xs text-[var(--panel-muted-fg)]">{emptyText}</p>
        ) : (
          <ul ref={list} aria-label="Deleted documents" className="flex flex-col">
            {shown.map((row, index) => (
              <li key={row.node} className="flex h-11 items-center gap-2 px-1">
                <Trash2 size={16} strokeWidth={1.5} aria-hidden className="shrink-0 text-[var(--toolbar-icon)]" />
                <div className="flex min-w-0 flex-1 flex-col">
                  <span title={row.name} className="truncate text-sm text-[var(--toolbar-icon)]">
                    {row.name}
                  </span>
                  <span className="truncate text-xs text-[var(--panel-muted-fg)]">{row.line}</span>
                </div>
                <button
                  type="button"
                  aria-label={`Restore ${row.name}`}
                  onClick={() => {
                    restored.current = { index, count: rows.length };
                    workspace.restore(row);
                  }}
                  className={OUTLINE_SMALL}
                >
                  Restore
                </button>
              </li>
            ))}
            {rows.length > shown.length ? (
              <li className="px-1 text-xs text-[var(--panel-muted-fg)]">and {rows.length - shown.length} more</li>
            ) : null}
          </ul>
        )
      ) : null}
    </div>
  );
}

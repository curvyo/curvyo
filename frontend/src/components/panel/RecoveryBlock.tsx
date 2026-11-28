import { ArchiveRestore, Trash2, TriangleAlert } from "lucide-react";
import { GLYPH, SECTION_HEADING } from "@/components/ui/looks";
import { type KeyboardEvent, useEffect, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import type { RecoveryApi, RecoveryRowItem } from "@/hooks/useRecovery";
import { ROW_PX, ROW_THREE_LINES_PX, cutPath, focusAfterRemoval, rowKey } from "@/lib/recoveryRows";

const ICON_BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:size-9";

interface RecoveryBlockProps {
  recovery: RecoveryApi;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
}

/**
 * "Unsaved work from a crash" (`specs/0136-autosave-and-restore` criteria 4 to 7 and 9,
 * `ux-notes.md`): one row per recovery copy with Restore and an armed Discard, at the top of the
 * File tab. There is nothing when no copy waits; no dialog, no popup.
 */
export function RecoveryBlock({ recovery, onReturnFocus }: RecoveryBlockProps) {
  const { rows } = recovery;
  const arm = useDeleteArm();
  const list = useRef<HTMLUListElement>(null);
  /** The row the roving tab stop is on. */
  const current = useRef(0);

  const rowElements = () => Array.from(list.current?.querySelectorAll<HTMLElement>("[data-recovery-row]") ?? []);
  const focusRow = (index: number) => {
    const elements = rowElements();
    current.current = Math.max(0, Math.min(index, elements.length - 1));
    elements.forEach((element, i) => element.setAttribute("tabindex", i === current.current ? "0" : "-1"));
    elements[current.current]?.focus();
  };

  useEffect(() => {
    // The roving stop stays inside the list when rows go.
    current.current = Math.min(current.current, Math.max(rows.length - 1, 0));
  }, [rows.length]);

  if (rows.length === 0) {
    return null;
  }

  const removed = (index: number) => {
    const next = focusAfterRemoval(index, rows.length);
    window.setTimeout(() => {
      if (next === null) {
        document.querySelector<HTMLElement>('[role="tab"][aria-label^="File"]')?.focus();
      } else {
        focusRow(next);
      }
    }, 0);
  };

  const press = (item: RecoveryRowItem, index: number) => {
    arm.press(`recovery:${item.row.key}`, item.view.discard_armed_label, () => {
      removed(index);
      void recovery.discard(item);
      return "";
    });
  };

  const onKeyDown = (event: KeyboardEvent<HTMLLIElement>, item: RecoveryRowItem, index: number) => {
    if (event.target !== event.currentTarget && (event.target as HTMLElement).tagName === "BUTTON" && event.key !== "ArrowLeft" && event.key !== "Escape") {
      return;
    }
    const armed = arm.armed === `recovery:${item.row.key}`;
    const action = rowKey(event.key, index, rows.length, item.row.problem === null);
    if (armed && (action.kind === "restore" || action.kind === "arm")) {
      // Once armed, Enter and Delete mean Discard.
      event.preventDefault();
      press(item, index);
      return;
    }
    switch (action.kind) {
      case "move":
        event.preventDefault();
        focusRow(action.to);
        break;
      case "restore":
        event.preventDefault();
        recovery.restore(item, () => focusRow(index));
        break;
      case "to-button":
        event.preventDefault();
        event.currentTarget.querySelector<HTMLElement>("[data-recovery-restore]")?.focus();
        break;
      case "to-row":
        event.preventDefault();
        focusRow(index);
        break;
      case "arm":
        event.preventDefault();
        press(item, index);
        break;
      case "escape":
        if (!armed) {
          event.preventDefault();
          onReturnFocus();
        }
        break;
      default:
        break;
    }
  };

  return (
    <section aria-labelledby="recovery-heading" className="mb-4">
      <h3 id="recovery-heading" className={`h-5 ${SECTION_HEADING}`}>
        {recovery.heading}
      </h3>
      <Tooltip side="left" content={recovery.where ?? recovery.help}>
        <p className="h-4 truncate text-xs leading-4 text-[var(--panel-muted-fg)]">
          {recovery.where === null ? recovery.help : `Kept in ${cutPath(recovery.where, 30)}`}
        </p>
      </Tooltip>
      <ul ref={list} aria-label={recovery.heading} className="mt-1 flex flex-col">
        {rows.map((item, index) => {
          const { row, view } = item;
          const zone = `recovery:${row.key}`;
          const armed = arm.armed === zone;
          const damaged = row.problem !== null;
          return (
            <li
              key={row.key}
              data-recovery-row
              {...{ [ARM_ZONE]: zone }}
              tabIndex={index === 0 ? 0 : -1}
              aria-label={view.label}
              onKeyDown={(event) => onKeyDown(event, item, index)}
              onFocus={() => {
                current.current = index;
              }}
              style={{ minHeight: view.line3 || damaged ? ROW_THREE_LINES_PX : ROW_PX }}
              className={`flex items-center gap-1 rounded-[5px] pr-1 pl-2 outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${armed ? "bg-[var(--destructive-soft)]" : ""}`}
            >
              {damaged ? <TriangleAlert {...GLYPH} aria-hidden className="shrink-0 text-[var(--panel-muted-fg)]" /> : null}
              <div className="min-w-0 flex-1 py-1">
                <Tooltip side="left" content={view.name}>
                  <p className="truncate text-sm text-[var(--toolbar-icon)]">{view.name}</p>
                </Tooltip>
                <p className="text-xs text-[var(--panel-muted-fg)] tabular-nums">{view.line2}</p>
                {view.line3 ? <p className="text-xs text-[var(--panel-muted-fg)]">{view.line3}</p> : null}
              </div>
              <div className="flex w-16 shrink-0 justify-end gap-1">
                {damaged ? (
                  <span className="size-7 shrink-0 pointer-coarse:size-9" aria-hidden />
                ) : (
                  <Tooltip side="left" content={view.restore_label}>
                    <button
                      type="button"
                      data-recovery-restore
                      tabIndex={-1}
                      aria-label={view.restore_label}
                      onClick={(event) => {
                        recovery.restore(item, () => focusRow(index));
                        if (event.detail > 0) {
                          onReturnFocus();
                        }
                      }}
                      className={ICON_BUTTON}
                    >
                      <ArchiveRestore {...GLYPH} aria-hidden />
                    </button>
                  </Tooltip>
                )}
                <Tooltip side="left" content={armed ? view.discard_armed_label : view.discard_label}>
                  <button
                    type="button"
                    tabIndex={-1}
                    {...{ [ARM_TRASH]: zone }}
                    aria-label={armed ? view.discard_armed_label : view.discard_label}
                    onClick={() => press(item, index)}
                    className={
                      armed
                        ? "flex size-7 shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white pointer-coarse:size-9"
                        : ICON_BUTTON
                    }
                  >
                    <Trash2 {...GLYPH} aria-hidden />
                  </button>
                </Tooltip>
              </div>
            </li>
          );
        })}
      </ul>
      <span role="status" className="sr-only">
        {arm.status}
      </span>
      <div
        aria-hidden
        className="mt-4 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
    </section>
  );
}

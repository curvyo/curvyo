// The Fonts tab's tag strip (`specs/0075-font-library` criteria 1, 5 to 8 and 18; UX tags
// amendment §6 and the strip editing of 2026-10-20): one radio group "Font tag" in the preset
// strip's look, single choice, at most four rows, the Cuttable cell's progress bar; an own tag's
// cell carries Rename, edited in place, and Delete, an inline delete confirm
// (`specs/0081-inline-delete-confirm` Part B).
import { CircleAlert, Pencil, Trash2 } from "lucide-react";
import { RadioGroup } from "radix-ui";
import { useEffect, useRef, useState } from "react";

import { ARMED_TRASH, ARM_TRANSITION } from "@/components/panel/FontTreeRows";
import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { FontTagView } from "@/hooks/useFontLibrary";
import { armedTagLabel, checkingDescription, tagDeletedText, tagNameError } from "@/lib/fontLibraryText";
import { stripCells, stripKeyAction } from "@/lib/tagStrip";

const BORDER = "color-mix(in srgb, var(--toolbar-icon) 60%, transparent)";
/** Four rows of 28 px with 1 px between them (UX notes §2). */
const MAX_STRIP_PX = 4 * 28 + 3;
const CELL =
  "relative flex h-7 min-w-8 flex-[1_1_auto] items-center justify-center bg-[var(--panel-bg)] text-sm whitespace-nowrap text-[var(--toolbar-icon)]";
const CELL_ICON =
  "flex size-5 shrink-0 items-center justify-center rounded-[5px] outline-none hover:bg-[var(--editor-accent-hover)]";

interface RenameFieldProps {
  initial: string;
  onSave: (name: string) => string | null;
  onCancel: () => void;
}

/** The in-place name field of a strip cell: Enter or blur saves (or shows the chip), Escape
 * cancels. */
function RenameField({ initial, onSave, onCancel }: RenameFieldProps) {
  const [value, setValue] = useState(initial);
  const [error, setError] = useState<string | null>(null);
  const ref = useRef<HTMLInputElement | null>(null);
  /** Escape or a save closed the field: the blur that its removal may cause does nothing. */
  const closed = useRef(false);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.select();
  }, []);
  const save = () => {
    if (closed.current) {
      return;
    }
    if (value === initial) {
      closed.current = true;
      onCancel();
      return;
    }
    const failed = onSave(value);
    closed.current = failed === null;
    setError(failed);
  };
  return (
    <div className="relative flex h-7 min-w-24 flex-[1_1_auto] items-center bg-[var(--panel-bg)] px-0.5">
      <input
        ref={ref}
        type="text"
        value={value}
        aria-label={`Rename tag ${initial}`}
        aria-invalid={error !== null}
        onChange={(event) => {
          setValue(event.target.value);
          setError(null);
        }}
        onBlur={save}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Enter") {
            event.preventDefault();
            save();
          } else if (event.key === "Escape") {
            event.preventDefault();
            closed.current = true;
            onCancel();
          }
        }}
        className="h-6 w-full rounded-[5px] bg-white px-1.5 text-sm outline-none ring-1 ring-border focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      />
      {error ? (
        <p
          role="alert"
          className="absolute top-full left-0 z-30 mt-1 flex w-max max-w-[244px] items-center gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]"
        >
          <CircleAlert size={12} aria-hidden />
          {error}
        </p>
      ) : null}
    </div>
  );
}

interface FontTagStripProps {
  tags: FontTagView[];
  chosen: string;
  progress: { done: number; total: number } | null;
  onChoose: (id: string, keyboard: boolean) => void;
  onRename: (id: string, name: string) => { error?: string; taken?: string };
  onDelete: (id: string) => void;
  /** The tab's one armed trash. */
  arm: Pick<DeleteArm, "armed" | "press">;
  onReturnFocus: () => void;
}

export function FontTagStrip({
  tags,
  chosen,
  progress,
  onChoose,
  onRename,
  onDelete,
  arm,
  onReturnFocus,
}: FontTagStripProps) {
  const stripRef = useRef<HTMLDivElement | null>(null);
  const [editing, setEditing] = useState<string | null>(null);
  const cells = stripCells(tags);

  /** Puts the focus back on the cell `id` after an edit (the strip's one Tab stop). */
  const focusCell = (id: string) =>
    window.requestAnimationFrame(() =>
      stripRef.current?.querySelector<HTMLElement>(`[data-tag="${CSS.escape(id)}"]`)?.focus(),
    );

  /** A press on an own tag's trash: arms it, or deletes the tag when armed (`0081` criterion 12). */
  const pressDelete = (cell: { id: string; name: string; members: number }, byKeyboard: boolean) =>
    arm.press(`tag:${cell.id}`, armedTagLabel(cell.name, cell.members), () => {
      const inCell = document.activeElement?.closest(`[${ARM_ZONE}="${CSS.escape(`tag:${cell.id}`)}"]`);
      // A deleted chosen tag gives way to All; a keyboard delete puts the focus on All.
      onDelete(cell.id);
      if (byKeyboard || inCell) {
        focusCell("all");
      }
      return tagDeletedText(cell.name);
    });

  // The chosen cell is always scrolled into the strip's view (the 4-row cap).
  useEffect(() => {
    stripRef.current
      ?.querySelector<HTMLElement>(`[data-tag="${CSS.escape(chosen)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [chosen]);

  return (
    <div className="mb-2 flex shrink-0">
      <RadioGroup.Root
        ref={stripRef}
        aria-label="Font tag"
        orientation="horizontal"
        value={chosen}
        onValueChange={() => undefined}
        className="flex w-full flex-wrap gap-px overflow-y-auto rounded-[5px] border [scrollbar-width:thin]"
        style={{ borderColor: BORDER, background: BORDER, maxHeight: MAX_STRIP_PX }}
      >
        {cells.map((cell) => {
          if (editing === cell.id) {
            return (
              <RenameField
                key={cell.id}
                initial={cell.name}
                onSave={(name) => {
                  const out = onRename(cell.id, name);
                  if (out.error) {
                    return tagNameError(out.error, out.taken);
                  }
                  setEditing(null);
                  focusCell(name.trim());
                  return null;
                }}
                onCancel={() => {
                  setEditing(null);
                  focusCell(cell.id);
                }}
              />
            );
          }
          const checking = cell.id === "cuttable" && progress;
          const tip = checking ? checkingDescription(progress.done, progress.total) : null;
          const pressed = cell.id === chosen;
          const armed = arm.armed === `tag:${cell.id}`;
          const armedLabel = armed ? armedTagLabel(cell.name, cell.members) : null;
          const item = (
            <RadioGroup.Item
              value={cell.id}
              data-tag={cell.id}
              aria-description={tip ?? undefined}
              onKeyDown={(event) => {
                const action = stripKeyAction(event.key, cell.own) ?? (armed && event.key === "Enter" ? "delete" : null);
                if (action) {
                  event.preventDefault();
                  event.stopPropagation();
                  if (action === "rename") {
                    setEditing(cell.id);
                  } else {
                    pressDelete(cell, true);
                  }
                }
              }}
              onClick={(event) => {
                onChoose(cell.id, event.detail === 0);
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className={`flex h-full flex-1 items-center justify-center outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${cell.own ? "pl-1.5" : "px-1.5"}`}
            >
              {cell.name}
            </RadioGroup.Item>
          );
          return (
            <div
              key={cell.id}
              {...(cell.own ? { [ARM_ZONE]: `tag:${cell.id}` } : {})}
              className={`${CELL} ${ARM_TRANSITION} ${
                armed
                  ? "!bg-[var(--destructive-soft)] !text-[var(--toolbar-icon)]"
                  : pressed
                    ? "!bg-[var(--toolbar-icon-active-bg)] !text-[var(--toolbar-icon-active-fg)]"
                    : "hover:[background-image:linear-gradient(var(--editor-accent-hover),var(--editor-accent-hover))]"
              }`}
            >
              {tip ? (
                <Tooltip side="left" content={tip}>
                  {item}
                </Tooltip>
              ) : (
                item
              )}
              {cell.own ? (
                <span className="flex items-center pr-1">
                  <Tooltip side="bottom" content="Rename">
                    <button
                      type="button"
                      tabIndex={-1}
                      aria-label={`Rename tag ${cell.name}`}
                      onClick={() => setEditing(cell.id)}
                      className={CELL_ICON}
                    >
                      <Pencil size={12} aria-hidden />
                    </button>
                  </Tooltip>
                  <Tooltip side="bottom" content={armedLabel ?? "Delete"}>
                    <button
                      type="button"
                      tabIndex={-1}
                      aria-label={armedLabel ?? `Delete tag ${cell.name}`}
                      {...{ [ARM_TRASH]: `tag:${cell.id}` }}
                      onMouseDown={(event) => event.preventDefault()}
                      onClick={() => pressDelete(cell, false)}
                      className={`${CELL_ICON} ${ARM_TRANSITION} ${armed ? `${ARMED_TRASH} !bg-[var(--destructive)]` : ""}`}
                    >
                      <Trash2 size={12} aria-hidden />
                    </button>
                  </Tooltip>
                </span>
              ) : null}
              {checking ? (
                <span
                  aria-hidden
                  className="absolute bottom-0 left-0 h-0.5"
                  style={{
                    width: `${(100 * progress.done) / Math.max(1, progress.total)}%`,
                    background: pressed ? "var(--toolbar-icon-active-fg)" : "var(--editor-accent)",
                  }}
                />
              ) : null}
            </div>
          );
        })}
      </RadioGroup.Root>
    </div>
  );
}

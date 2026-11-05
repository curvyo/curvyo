import { Check, Copy, Pencil, Ruler, Trash2 } from "lucide-react";
import { Fragment, type KeyboardEvent, useEffect, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import type { MachineProfileRow } from "@/hooks/useWorkbenches";
import { areaText, profileRowKey } from "@/lib/machineBlockKeys";

const ROW_BUTTON =
  "inline-flex h-7 w-6 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)]";

/** The armed trash's text (UX notes §3). */
const ARMED_DELETE = "Delete profile. Projects keep their copy.";

interface MachineProfileListProps {
  rows: MachineProfileRow[];
  onPick: (id: string) => void;
  onDuplicate: (id: string) => string;
  onRename: (id: string, name: string) => void;
  onDelete: (id: string) => void;
  /** Edit work area of a user profile (`specs/0098` criterion 15): a library write; the notice. */
  onWorkArea: (id: string, width: number, height: number) => string;
  onClose: () => void;
}

/**
 * The inline profile list of the Machine block (`specs/0026-machine-profile` UX notes §3): one
 * listbox, at most five rows visible, one Tab stop (the chosen row). A row picks on Enter, Space
 * or a press; Duplicate, Rename and Delete are mouse buttons with keys (Ctrl+D, F2, Delete).
 * Delete uses the armed press of `0081`; the library has no undo.
 */
export function MachineProfileList({
  rows,
  onPick,
  onDuplicate,
  onRename,
  onDelete,
  onWorkArea,
  onClose,
}: MachineProfileListProps) {
  const [sizing, setSizing] = useState<string | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const chosen = Math.max(
    0,
    rows.findIndex((row) => row.chosen),
  );
  const [active, setActive] = useState(rows[chosen]?.id ?? null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const pendingFocus = useRef<string | null>(rows[chosen]?.id ?? null);
  const arm = useDeleteArm();

  const focusRow = (id: string) => {
    listRef.current?.querySelector<HTMLElement>(`[data-profile-row="${CSS.escape(id)}"]`)?.focus();
  };
  useEffect(() => {
    const target = pendingFocus.current;
    if (target !== null && renaming === null) {
      pendingFocus.current = null;
      focusRow(target);
    }
  });

  const remove = (index: number) => {
    const row = rows[index];
    arm.press(row.id, ARMED_DELETE, () => {
      onDelete(row.id);
      const next = rows[index + 1] ?? rows[index - 1];
      pendingFocus.current = next?.id ?? null;
      setActive(next?.id ?? null);
      return "Profile deleted.";
    });
  };
  const duplicate = (id: string) => {
    const made = onDuplicate(id);
    if (made !== "") {
      setActive(made);
      setRenaming(made);
    }
  };
  const onRowKey = (event: KeyboardEvent<HTMLElement>, index: number) => {
    if (renaming !== null) {
      return;
    }
    const row = rows[index];
    const action = profileRowKey(event.key, event.ctrlKey || event.metaKey, index, rows.length);
    if (!row || action === null) {
      return;
    }
    if (action.kind === "rename" || action.kind === "delete") {
      if (row.builtin) {
        return;
      }
    }
    event.preventDefault();
    event.stopPropagation();
    switch (action.kind) {
      case "focus":
        setActive(rows[action.index].id);
        focusRow(rows[action.index].id);
        break;
      case "pick":
        if (arm.armed === row.id) {
          remove(index);
        } else {
          onPick(row.id);
        }
        break;
      case "rename":
        setRenaming(row.id);
        break;
      case "duplicate":
        duplicate(row.id);
        break;
      case "delete":
        remove(index);
        break;
      case "close":
        onClose();
        break;
    }
  };

  return (
    <div
      ref={listRef}
      role="listbox"
      aria-label="Machine profiles"
      className="flex max-h-[220px] flex-col overflow-y-auto overflow-x-hidden rounded-md"
      style={{ border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
    >
      {rows.map((row, index) => {
        const armed = arm.armed === row.id;
        return (
          <Fragment key={row.id}>
          <div
            role="option"
            aria-selected={row.chosen}
            data-profile-row={row.id}
            {...{ [ARM_ZONE]: row.id }}
            tabIndex={row.id === active ? 0 : -1}
            onFocus={() => setActive(row.id)}
            onKeyDown={(event) => onRowKey(event, index)}
            onClick={() => {
              if (renaming !== row.id) {
                onPick(row.id);
              }
            }}
            className={`flex h-11 shrink-0 cursor-default items-center gap-1 px-1 outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${armed ? "bg-[var(--destructive-soft)]" : ""}`}
          >
            <span className="flex w-4 shrink-0 justify-center text-[var(--toolbar-icon)]">
              {row.chosen ? <Check size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden /> : null}
            </span>
            <span className="flex min-w-0 flex-1 flex-col">
              {renaming === row.id ? (
                <RenameField
                  name={row.name}
                  onDone={(name) => {
                    setRenaming(null);
                    pendingFocus.current = row.id;
                    if (name !== null) {
                      onRename(row.id, name);
                    }
                  }}
                />
              ) : (
                <Tooltip side="left" content={row.name}>
                  <span className="truncate text-sm">{row.name}</span>
                </Tooltip>
              )}
              <span className="text-xs text-[var(--panel-muted-fg)]">{areaText(row.width, row.height)}</span>
            </span>
            <RowButton label={`Duplicate ${row.name}`} onPress={() => duplicate(row.id)}>
              <Copy size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            </RowButton>
            {row.builtin ? null : (
              <>
                <RowButton label={`Rename ${row.name}`} onPress={() => setRenaming(row.id)}>
                  <Pencil size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </RowButton>
                <RowButton
                  label={`Edit work area of ${row.name}`}
                  onPress={() => setSizing(sizing === row.id ? null : row.id)}
                >
                  <Ruler size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </RowButton>
                <RowButton
                  label={armed ? ARMED_DELETE : `Delete ${row.name}`}
                  armTarget={row.id}
                  armed={armed}
                  onPress={() => remove(index)}
                >
                  <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
                </RowButton>
              </>
            )}
          </div>
          {sizing === row.id ? (
            <WorkAreaRow
              row={row}
              onCommit={(width, height) => onWorkArea(row.id, width, height)}
              onClose={() => {
                setSizing(null);
                pendingFocus.current = row.id;
              }}
            />
          ) : null}
          </Fragment>
        );
      })}
      <span role="status" className="sr-only">
        {arm.status}
      </span>
    </div>
  );
}

function RowButton({
  label,
  armTarget,
  armed = false,
  onPress,
  children,
}: {
  label: string;
  armTarget?: string;
  armed?: boolean;
  onPress: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip side="left" content={label}>
      <button
        type="button"
        tabIndex={-1}
        aria-label={label}
        {...(armTarget === undefined ? {} : { [ARM_TRASH]: armTarget })}
        className={armed ? ROW_BUTTON.replace("hover:bg-[var(--editor-accent-hover)]", "bg-[var(--destructive)] text-white") : ROW_BUTTON}
        onClick={(event) => {
          event.stopPropagation();
          onPress();
        }}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/** The name line as a text field: Enter or blur commits, Escape or an empty name reverts
 * (`null`). The session cuts an over-long name and says so. */
function RenameField({ name, onDone }: { name: string; onDone: (name: string | null) => void }) {
  const [text, setText] = useState(name);
  const done = useRef(false);
  const finish = (value: string | null) => {
    if (!done.current) {
      done.current = true;
      onDone(value);
    }
  };
  return (
    <input
      aria-label="Profile name"
      autoFocus
      value={text}
      onFocus={(event) => event.currentTarget.select()}
      onChange={(event) => setText(event.target.value)}
      onClick={(event) => event.stopPropagation()}
      onBlur={() => finish(text.trim() === "" || text === name ? null : text)}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") {
          event.preventDefault();
          finish(text.trim() === "" || text === name ? null : text);
        } else if (event.key === "Escape") {
          event.preventDefault();
          finish(null);
        }
      }}
      className="h-7 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
    />
  );
}

/** The in-place sub-row of Edit work area (UX notes §9): Width and Height of a user profile, the
 * profile range and clamp of `0026` criterion 1; Enter or blur commits, Escape closes. */
function WorkAreaRow({
  row,
  onCommit,
  onClose,
}: {
  row: MachineProfileRow;
  onCommit: (width: number, height: number) => string;
  onClose: () => void;
}) {
  const [notice, setNotice] = useState("");
  const field = (label: "Width" | "Height", value: number) => (
    <label className="flex h-7 min-w-0 flex-1 items-center gap-1 rounded-md border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] px-1 text-xs">
      <span className="text-[var(--panel-muted-fg)]">{label}</span>
      <input
        aria-label={`${label} of ${row.name} (mm)`}
        type="number"
        step="any"
        defaultValue={value}
        className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none"
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Enter") {
            event.preventDefault();
            event.currentTarget.blur();
          } else if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          }
        }}
        onBlur={(event) => {
          const typed = Number(event.target.value);
          if (!event.target.value.trim() || !Number.isFinite(typed)) {
            event.target.value = String(value);
            return;
          }
          const width = label === "Width" ? typed : row.width;
          const height = label === "Height" ? typed : row.height;
          setNotice(onCommit(width, height));
        }}
      />
      <span className="text-[var(--panel-muted-fg)]">mm</span>
    </label>
  );
  return (
    <div role="group" aria-label={`Work area of ${row.name}`} className="flex flex-col gap-1 px-2 pb-2">
      <div className="flex gap-1">
        {field("Width", row.width)}
        {field("Height", row.height)}
      </div>
      {notice ? (
        <span role="status" className="text-xs text-[var(--toolbar-icon)]">
          {notice}
        </span>
      ) : null}
    </div>
  );
}

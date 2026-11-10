import {
  CircleAlert,
  ChevronDown,
  ChevronRight,
  FileInput,
  FileOutput,
  Pencil,
  Plus,
  Star,
} from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { ARM_TRANSITION, RowTrash } from "@/components/panel/FontTreeRows";
import { FormatForm } from "@/components/panel/FormatForm";
import { scrollBodyTo } from "@/lib/panelScroll";
import { PanelSwitch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { ARM_ZONE } from "@/hooks/useDeleteArm";
import type { FormatGroup, FormatNotice, FormatRow, FormatsApi } from "@/hooks/useFormats";

const ICON_BUTTON =
  "flex shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]";

/** The pencil of a user format: muted, 16 px, like the Fonts tab's tag pencil. */
const EDIT_BUTTON = `${ICON_BUTTON} h-7 w-6 !text-[var(--panel-muted-fg)]`;

/** The 28 x 28 outline symbol buttons of the "All formats" row (the look of "Add font..."). */
const SYMBOL_BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";
const SYMBOL_PRESSED =
  "!border-transparent !bg-[var(--toolbar-icon-active-bg)] !text-[var(--toolbar-icon-active-fg)]";

/** A failure notice 4 px under the button it belongs to. */
function NoticeUnder({ notice }: { notice: FormatNotice | null }) {
  if (!notice) {
    return null;
  }
  return (
    <div
      role="alert"
      className="pointer-events-none absolute top-full right-0 z-30 mt-1 flex max-w-[244px] items-start gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]"
    >
      <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
      {notice.text}
    </div>
  );
}

interface RenameFieldProps {
  formats: FormatsApi;
  kind: "format" | "group";
  id: string;
  name: string;
}

/** The inline rename field that replaces a name (`0045` criterion 43): Enter confirms, Escape
 * cancels, blur commits a valid change; an invalid name keeps it open with its chip. */
function RenameField({ formats, kind, id, name }: RenameFieldProps) {
  const rename = formats.renaming;
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  if (!rename || rename.kind !== kind || rename.id !== id) {
    return null;
  }
  return (
    <div className="relative min-w-0 flex-1 px-1">
      <input
        ref={input}
        type="text"
        autoComplete="off"
        spellCheck={false}
        aria-label={name}
        aria-invalid={rename.error ? true : undefined}
        value={rename.text}
        onChange={(event) => formats.changeRename(event.target.value)}
        onBlur={() => formats.commitRename("blur")}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Enter") {
            event.preventDefault();
            formats.commitRename("enter");
          } else if (event.key === "Escape") {
            event.preventDefault();
            formats.cancelRename();
          }
        }}
        className={`style-field h-6 w-full rounded-[5px] bg-white px-1.5 text-sm outline-none ${
          rename.error
            ? "border-2 border-[var(--field-invalid)]"
            : "border border-[var(--editor-accent)]"
        }`}
      />
      {rename.error && (
        <div
          role="alert"
          className="pointer-events-none absolute top-full right-0 z-30 mt-1 flex w-max max-w-[226px] items-start gap-1 rounded-md bg-popover px-2 py-1 text-xs text-[var(--field-invalid)] ring-1 ring-[var(--field-invalid)]"
        >
          <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
          {rename.error}
        </div>
      )}
    </div>
  );
}

interface RowProps {
  row: FormatRow;
  formats: FormatsApi;
  onReturnFocus: () => void;
}

function FormatRowView({ row, formats, onReturnFocus }: RowProps) {
  if (formats.form?.editing === row.id) {
    return <FormatForm formats={formats} onReturnFocus={onReturnFocus} />;
  }
  const click = (action: () => void) => (event: { detail: number }) => {
    action();
    if (event.detail > 0) {
      onReturnFocus();
    }
  };
  const zone = `fmt:${row.id}`;
  const armed = formats.arm.armed === zone;
  const renaming = formats.renaming?.kind === "format" && formats.renaming.id === row.id;
  return (
    <div
      data-rk={zone}
      data-user={row.canEdit ? "1" : undefined}
      {...(row.canEdit ? { [ARM_ZONE]: zone } : {})}
      className={`flex h-7 items-center rounded-[5px] ${ARM_TRANSITION} ${
        armed
          ? "bg-[var(--destructive-soft)]"
          : row.selected
            ? "bg-[var(--value-fill)] shadow-[inset_3px_0_0_var(--editor-accent)]"
            : "hover:bg-[color-mix(in_srgb,var(--editor-accent)_8%,white)]"
      }`}
    >
      {row.canStar && (
        <Tooltip
          side="left"
          content={row.favourite ? "Remove from the quick selection" : "Show in the quick selection"}
        >
          <button
            type="button"
            data-control="star"
            aria-label={row.starName}
            aria-pressed={row.favourite}
            onClick={click(() => formats.setFavourite(row.id, !row.favourite))}
            className={`${ICON_BUTTON} h-7 w-7 ${row.favourite ? "" : "!text-[var(--panel-muted-fg)]"}`}
          >
            <Star size={16} aria-hidden fill={row.favourite ? "currentColor" : "none"} strokeWidth={1.5} />
          </button>
        </Tooltip>
      )}
      {renaming ? (
        <>
          <RenameField formats={formats} kind="format" id={row.id} name={row.renameName} />
          <span className="mr-1 min-w-0 shrink truncate text-xs tabular-nums text-[var(--panel-muted-fg)]">
            {row.size}
          </span>
        </>
      ) : (
        <Tooltip side="left" content={`${row.name}, ${row.size}`}>
          <button
            type="button"
            data-control="apply"
            aria-label={row.applyName}
            onClick={click(() => formats.pick(row.id))}
            className="flex h-7 min-w-0 flex-1 items-center gap-2 rounded-[5px] px-1.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]"
          >
            <span
              onDoubleClick={row.canEdit ? () => formats.startRename("format", row.id) : undefined}
              className="min-w-[56px] flex-1 truncate text-sm text-[var(--toolbar-icon)]"
            >
              {row.name}
            </span>
            <span className="min-w-0 shrink truncate text-xs tabular-nums text-[var(--panel-muted-fg)]">
              {row.size}
            </span>
          </button>
        </Tooltip>
      )}
      <span className="flex w-[52px] shrink-0 justify-end gap-1">
        {row.canEdit && (
          <>
            <Tooltip side="left" content={row.editName}>
              <button
                type="button"
                data-control="edit"
                aria-label={row.editName}
                onClick={click(() => formats.openEdit(row.id))}
                className={EDIT_BUTTON}
              >
                <Pencil size={16} aria-hidden strokeWidth={1.5} />
              </button>
            </Tooltip>
            <RowTrash
              target={zone}
              armed={armed}
              label={armed ? row.armedName : row.deleteName}
              tip={armed ? row.armedName : row.deleteName}
              onPress={() => formats.pressDelete("format", row.id, null)}
            />
          </>
        )}
      </span>
    </div>
  );
}

interface GroupProps {
  group: FormatGroup;
  formats: FormatsApi;
  onReturnFocus: () => void;
}

function FormatGroupView({ group, formats, onReturnFocus }: GroupProps) {
  const folded = formats.folded.has(group.id);
  const showRows = group.enabled && !folded;
  const click = (action: () => void) => (event: { detail: number }) => {
    action();
    if (event.detail > 0) {
      onReturnFocus();
    }
  };
  const zone = `grp:${group.id}`;
  const armed = formats.arm.armed === zone;
  const renaming = formats.renaming?.kind === "group" && formats.renaming.id === group.id;
  return (
    <div className="flex flex-col">
      <div
        data-rk={zone}
        data-user={group.canEdit ? "1" : undefined}
        {...(group.canEdit ? { [ARM_ZONE]: zone } : {})}
        className={`flex h-7 items-center rounded-[5px] ${ARM_TRANSITION} ${armed ? "bg-[var(--destructive-soft)]" : ""}`}
      >
        <button
          type="button"
          data-control="fold"
          aria-label={group.foldName}
          aria-expanded={showRows}
          disabled={!group.enabled}
          onClick={click(() => formats.toggleFold(group.id))}
          className={`${ICON_BUTTON} h-7 w-7 disabled:opacity-40`}
        >
          {showRows ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
        </button>
        {renaming ? (
          <RenameField formats={formats} kind="group" id={group.id} name={group.renameName} />
        ) : (
          <Tooltip side="left" content={group.name}>
            <span
              onDoubleClick={group.canEdit ? () => formats.startRename("group", group.id) : undefined}
              className="min-w-0 flex-1 truncate text-xs font-semibold text-[var(--panel-muted-fg)]"
            >
              {group.name}
            </span>
          </Tooltip>
        )}
        <span className="mx-1 shrink-0 text-xs text-[var(--panel-muted-fg)]">{group.count}</span>
        {group.canEdit && (
          <RowTrash
            target={zone}
            armed={armed}
            label={armed ? group.armedName : group.deleteName}
            tip={armed ? group.armedName : group.deleteName}
            onPress={() => formats.pressDelete("group", group.id, null)}
          />
        )}
        {group.canToggle && (
          <span className="ml-1">
            <PanelSwitch
              label="Show"
              name={group.showName}
              control="show"
              checked={group.enabled}
              onCheckedChange={(on) => formats.setGroupEnabled(group.id, on)}
              onReturnFocus={onReturnFocus}
            />
          </span>
        )}
      </div>
      {showRows &&
        group.rows.map((row) => (
          <FormatRowView key={row.id} row={row} formats={formats} onReturnFocus={onReturnFocus} />
        ))}
    </div>
  );
}

/** Roving focus over the list body: one Tab stop, Up and Down between rows,
 * Left and Right between the controls of a row (`0045` criterion 17); `Delete` arms the trash of a
 * user row and a second one deletes, `F2` starts the rename (criteria 36 and 43). */
function useRovingList(root: React.RefObject<HTMLDivElement | null>, formats: FormatsApi) {
  const current = useRef("");
  const keyOf = (control: HTMLElement) =>
    `${control.closest<HTMLElement>("[data-rk]")?.dataset.rk ?? ""}:${control.dataset.control ?? ""}`;
  useLayoutEffect(() => {
    const element = root.current;
    if (!element) {
      return;
    }
    const controls = Array.from(element.querySelectorAll<HTMLElement>("[data-control]"));
    const active = controls.find((control) => keyOf(control) === current.current) ?? controls[0];
    for (const control of controls) {
      control.tabIndex = control === active ? 0 : -1;
    }
  });
  const onFocus = (event: React.FocusEvent) => {
    const target = event.target as HTMLElement;
    if (target.dataset.control) {
      current.current = keyOf(target);
    }
  };
  const onKeyDown = (event: React.KeyboardEvent) => {
    const target = event.target as HTMLElement;
    if (!target.dataset.control || event.altKey || event.ctrlKey || event.metaKey) {
      return;
    }
    const element = root.current;
    const row = target.closest<HTMLElement>("[data-rk]");
    if (!element || !row) {
      return;
    }
    const rows = Array.from(element.querySelectorAll<HTMLElement>("[data-rk]"));
    if (event.key === "F2" || event.key === "Delete") {
      // Built-in rows do nothing (and the key is not taken from the app).
      if (row.dataset.user !== "1") {
        return;
      }
      event.preventDefault();
      const [kind, ...rest] = (row.dataset.rk ?? "").split(":");
      const id = rest.join(":");
      if (event.key === "F2") {
        formats.startRename(kind === "grp" ? "group" : "format", id);
        return;
      }
      // After a keyboard delete: the next format row, else the previous one, else the disclosure.
      let next: string | null = null;
      if (kind === "grp") {
        next = "disclosure";
      } else {
        const formatRows = rows.filter((r) => r.dataset.rk?.startsWith("fmt:"));
        const at = formatRows.indexOf(row);
        const neighbour = formatRows[at + 1] ?? formatRows[at - 1];
        next = neighbour ? `${neighbour.dataset.rk}:apply` : "disclosure";
      }
      formats.pressDelete(kind === "grp" ? "group" : "format", id, next);
      return;
    }
    const controlsOf = (r: HTMLElement) => Array.from(r.querySelectorAll<HTMLElement>("[data-control]"));
    const here = controlsOf(row);
    let next: HTMLElement | undefined;
    const rowAt = (index: number) => {
      const wanted = rows[Math.min(Math.max(index, 0), rows.length - 1)];
      const controls = controlsOf(wanted);
      return controls.find((c) => c.dataset.control === target.dataset.control) ?? controls[0];
    };
    switch (event.key) {
      case "ArrowDown":
        next = rowAt(rows.indexOf(row) + 1);
        break;
      case "ArrowUp":
        next = rowAt(rows.indexOf(row) - 1);
        break;
      case "Home":
        next = controlsOf(rows[0])[0];
        break;
      case "End":
        next = controlsOf(rows[rows.length - 1])[0];
        break;
      case "ArrowRight":
        next = here[Math.min(here.indexOf(target) + 1, here.length - 1)];
        break;
      case "ArrowLeft":
        next = here[Math.max(here.indexOf(target) - 1, 0)];
        break;
      default:
        return;
    }
    event.preventDefault();
    next?.focus();
  };
  return { onFocus, onKeyDown };
}

interface FormatListProps {
  formats: FormatsApi;
  onReturnFocus: () => void;
}

/**
 * "All formats": the disclosure under Orientation with its symbol buttons and the inline list it
 * opens (`specs/0045-document-formats-library/` criteria 12 to 17, 22 to 25, 35 to 43). No popup:
 * the list is in the flow of the panel scroll. Everything shown, every name and which buttons
 * exist is Rust's; this holds what is open.
 */
export function FormatList({ formats, onReturnFocus }: FormatListProps) {
  const { list, expanded } = formats;
  const listId = useId();
  const addFormId = useId();
  const root = useRef<HTMLDivElement>(null);
  const disclosure = useRef<HTMLButtonElement>(null);
  const addButton = useRef<HTMLButtonElement>(null);
  const roving = useRovingList(root, formats);
  const [opened, setOpened] = useState(false);
  const addOpen = formats.form !== null && formats.form.editing === null;

  // Opening scrolls the panel so the row is at the top of the body.
  useEffect(() => {
    if (expanded && opened) {
      scrollBodyTo(disclosure.current);
    }
  }, [expanded, opened]);

  // Another tab or a collapsed panel takes the list away: an armed trash disarms.
  const { disarm } = formats.arm;
  useEffect(() => disarm, [disarm]);

  // Puts the keyboard back where a form, delete or rename left it, once.
  const returnTo = formats.focusReturn;
  const { takeFocusReturn } = formats;
  useEffect(() => {
    if (!returnTo) {
      return;
    }
    let target: HTMLElement | null | undefined;
    if (returnTo.target === "add") {
      target = addButton.current;
    } else if (returnTo.target === "disclosure") {
      target = disclosure.current;
    } else {
      const [kind, id, control] = returnTo.target.split(":");
      target = root.current?.querySelector<HTMLElement>(
        `[data-rk="${kind}:${id}"] [data-control="${control}"]`,
      );
    }
    // A row that is gone, or a list that is closed: the disclosure is the next best stop.
    (target ?? disclosure.current)?.focus();
    takeFocusReturn(returnTo.n);
  }, [returnTo, takeFocusReturn]);

  const symbol = (
    name: string,
    Icon: typeof Plus,
    onClick: () => void,
    extra: Record<string, unknown> = {},
    pressed = false,
  ) => (
    <Tooltip side="left" content={name}>
      <button
        type="button"
        aria-label={name}
        onClick={onClick}
        className={`${SYMBOL_BUTTON} ${pressed ? SYMBOL_PRESSED : ""}`}
        {...extra}
      >
        <Icon size={16} strokeWidth={1.5} aria-hidden />
      </button>
    </Tooltip>
  );

  return (
    <div className="flex flex-col">
      <div className="relative flex h-7 items-center gap-1">
        <button
          ref={disclosure}
          type="button"
          aria-expanded={expanded}
          aria-controls={listId}
          onClick={(event) => {
            setOpened(true);
            formats.toggleExpanded();
            if (event.detail > 0) {
              onReturnFocus();
            }
          }}
          className="flex h-7 min-w-0 flex-1 items-center gap-1 rounded-[5px] px-1 text-left text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          {expanded ? <ChevronDown size={12} aria-hidden /> : <ChevronRight size={12} aria-hidden />}
          <span className="flex-1">All formats</span>
          <span className="text-xs text-[var(--panel-muted-fg)]">{list.total}</span>
        </button>
        {expanded && list.canAdd && (
          <>
            {symbol(
              "Add format",
              Plus,
              formats.openAdd,
              { ref: addButton, "aria-expanded": addOpen, "aria-controls": addFormId },
              addOpen,
            )}
            {symbol("Import formats...", FileInput, formats.importFile)}
            {list.canExport && symbol("Export my formats...", FileOutput, formats.exportFile)}
          </>
        )}
        <NoticeUnder notice={formats.notice && formats.notice.anchor !== "broken" ? formats.notice : null} />
      </div>
      <div id={listId} hidden={!expanded} className="mt-1 flex flex-col gap-1">
        {expanded && (
          <>
            <p role="status" aria-live="polite" className="sr-only">
              {formats.arm.status}
            </p>
            {addOpen && (
              <div id={addFormId}>
                <FormatForm formats={formats} onReturnFocus={onReturnFocus} />
              </div>
            )}
            <div
              ref={root}
              role="group"
              aria-label="Formats"
              onFocus={roving.onFocus}
              onKeyDown={roving.onKeyDown}
              className="flex flex-col gap-1"
            >
              {list.groups.map((group) => (
                <FormatGroupView
                  key={group.id}
                  group={group}
                  formats={formats}
                  onReturnFocus={onReturnFocus}
                />
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}

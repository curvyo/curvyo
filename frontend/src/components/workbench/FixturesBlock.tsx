import { Check, Eye, EyeOff, Trash2 } from "lucide-react";
import { type KeyboardEvent, useRef, useState } from "react";

import { FixtureGlyph, SlotGlyph } from "@/components/workbench/FixtureGlyphs";
import { ARM_TRASH, ARM_ZONE, useDeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import type { FixtureRow, FixturesView, SlotRow } from "@/lib/fixturesView";
import {
  ADD_LABELS,
  KIND_WORDS,
  MAX_SLOTS,
  addKinds,
  rowKey,
  rowKeyAction,
  rowLine,
  rowName,
  slotCountText,
  tabStopRow,
} from "@/lib/fixtureRows";

/** The `fixture_list` block of a laser page's Document tab (`specs/0102-laser-fixtures` `ux-notes.md`
 * sections 3, 4 and 11): the add group, the rows with Active, eye and armed trash, and the typed
 * fields of the selected fixture. Nothing opens a dialog; the column holds every control. */
export function FixtureList({ api }: { api: FixturesApi }) {
  const arm = useDeleteArm();
  // The row last focused: the list's one Tab stop follows it (roving focus, `ux-notes.md` 10).
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const view = api.view;
  if (view === null) return null;
  const stop = tabStopRow(
    view.rows.map((row) => row.id),
    view.selected,
    focusedId,
  );
  return (
    <section aria-label="Fixtures" className="flex flex-col gap-2">
      <h3 className="text-sm text-[var(--toolbar-icon)]">Fixtures</h3>
      <AddGroup view={view} onAdd={(kind) => api.add(kind)} />
      {view.rows.length > 0 ? (
        <ul aria-label="Fixtures" className="flex flex-col">
          {view.rows.map((row, index) => (
            <FixtureItem
              key={rowKey(row)}
              row={row}
              index={index}
              count={view.rows.length}
              tabStop={row.id === stop}
              onFocusRow={() => setFocusedId(row.id)}
              view={view}
              api={api}
              arm={arm}
            />
          ))}
        </ul>
      ) : null}
      {view.damageNotices.map((text) => (
        <p key={text} role="status" className="text-xs text-[var(--panel-muted-fg)]">
          {text}
        </p>
      ))}
      {api.notice ? (
        <p role="status" className="text-xs text-[var(--toolbar-icon)]">
          {api.notice}
        </p>
      ) : null}
      <span role="status" className="sr-only">
        {arm.status}
      </span>
      {/* The selection announcer of `0044` (UX notes §11): the sizes are Rust's words. */}
      <span role="status" className="sr-only">
        {view.selectionAnnouncement ?? ""}
      </span>
    </section>
  );
}

const ICON_BUTTON =
  "inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The add group: a toolbar of the kinds, Left and Right move, the pressed button keeps focus. */
function AddGroup({
  view,
  onAdd,
}: {
  view: FixturesView;
  onAdd: (kind: string) => boolean;
}) {
  const group = useRef<HTMLDivElement>(null);
  const kinds = addKinds(view);
  // The group is one Tab stop: the button last focused, else the first (`ux-notes.md` 3.2, 10).
  const [current, setCurrent] = useState<string | null>(null);
  const stop = current !== null && kinds.includes(current as (typeof kinds)[number]) ? current : kinds[0];
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0) return;
    if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const to = event.key === "ArrowRight" ? at + 1 : at - 1;
      buttons[(to + buttons.length) % buttons.length]?.focus();
    }
  };
  return (
    <div ref={group} role="toolbar" aria-label="Add fixture" className="flex gap-1" onKeyDown={move}>
      {kinds.map((kind) => {
        const word = KIND_WORDS[kind];
        return (
          <button
            key={kind}
            type="button"
            aria-label={ADD_LABELS[kind]}
            title={ADD_LABELS[kind]}
            tabIndex={kind === stop ? 0 : -1}
            onFocus={() => setCurrent(kind)}
            className="flex h-10 w-11 flex-col items-center justify-center gap-0.5 rounded-md text-[11px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-12"
            onClick={() => {
              onAdd(kind);
              // The Grid button leaves the group with its press: focus moves to the first one left.
              if (kind === "grid") requestAnimationFrame(() => group.current?.querySelector("button")?.focus());
            }}
          >
            <FixtureGlyph kind={kind} />
            <span aria-hidden>{word}</span>
          </button>
        );
      })}
    </div>
  );
}

type Arm = ReturnType<typeof useDeleteArm>;

/** One row and, under it, the expanded fields of the selected fixture. */
function FixtureItem({
  row,
  index,
  count,
  tabStop,
  onFocusRow,
  view,
  api,
  arm,
}: {
  row: FixtureRow;
  index: number;
  count: number;
  tabStop: boolean;
  onFocusRow: () => void;
  view: FixturesView;
  api: FixturesApi;
  arm: Arm;
}) {
  const [renaming, setRenaming] = useState(false);
  const item = useRef<HTMLLIElement>(null);
  // Enter or Escape in the name field gives focus back to the row (the field is gone by then).
  const backToRow = () => requestAnimationFrame(() => item.current?.focus());
  const selected = view.selected === row.id;
  const known = row.kind !== "unknown";
  const armed = arm.armed === row.id;
  const removeRow = () => {
    arm.press(row.id, `Press again to delete ${row.name}`, () => {
      api.remove(row.id);
      return `${row.name} removed.`;
    });
  };
  const trash = (
    <button
      type="button"
      {...{ [ARM_TRASH]: row.id }}
      aria-label={armed ? `Press again to delete ${row.name}` : `Delete ${row.name}`}
      title={armed ? `Press again to delete ${row.name}` : `Delete ${row.name}`}
      tabIndex={-1}
      className={
        armed
          ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
          : `${ICON_BUTTON} text-[var(--panel-muted-fg)]`
      }
      onClick={removeRow}
    >
      <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
    </button>
  );
  // The keys of the focused row (`ux-notes.md` section 10); a key on a button inside the row is
  // the button's own.
  const onRowKey = (event: KeyboardEvent<HTMLLIElement>) => {
    // Left and Right walk the row's check box, eye and trash (line 1 order).
    if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && !event.altKey && !event.ctrlKey && !event.metaKey) {
      const stops = Array.from(event.currentTarget.querySelectorAll<HTMLElement>(":scope > div:first-child > button[role=checkbox], :scope > div:first-child > button[aria-pressed], :scope > div:first-child > button[data-arm-trash]"));
      const at = event.target === event.currentTarget ? -1 : stops.indexOf(event.target as HTMLElement);
      if (stops.length > 0 && (event.target === event.currentTarget || at >= 0)) {
        event.preventDefault();
        const to = event.key === "ArrowRight" ? at + 1 : at - 1;
        if (to >= 0 && to < stops.length) stops[to]?.focus();
        else if (to < 0) event.currentTarget.focus();
        return;
      }
    }
    if (event.target !== event.currentTarget || event.altKey || event.ctrlKey || event.metaKey) return;
    const action = rowKeyAction(event.key, index, count, armed);
    if (action === null) return;
    event.preventDefault();
    if (action.kind === "focus") {
      const rows = event.currentTarget.parentElement?.children;
      (rows?.[action.index] as HTMLElement | undefined)?.focus();
    } else if (action.kind === "rename") {
      setRenaming(true);
    } else {
      removeRow();
    }
  };
  return (
    <li
      ref={item}
      {...{ [ARM_ZONE]: row.id }}
      tabIndex={tabStop ? 0 : -1}
      onFocus={onFocusRow}
      onKeyDown={onRowKey}
      className={`flex flex-col rounded-md px-1 ${armed ? "bg-[var(--destructive-soft)]" : selected ? "bg-[var(--value-fill)]" : ""} ${selected ? "shadow-[inset_3px_0_0_var(--editor-accent)]" : ""}`}
    >
      <div className="flex h-6 items-center gap-2 pointer-coarse:h-7">
        {known && !renaming ? (
          <button
            type="button"
            role="checkbox"
            aria-checked={row.active}
            aria-label={`Active ${row.name}`}
            tabIndex={-1}
            className={`flex h-4 w-4 shrink-0 items-center justify-center rounded-[3px] border-[1.5px] border-[var(--toolbar-icon)] text-white outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${row.active ? "bg-[var(--toolbar-icon)]" : "bg-white"}`}
            onClick={() => api.setActive(row.id, !row.active)}
          >
            {row.active ? <Check size={12} strokeWidth={3} absoluteStrokeWidth aria-hidden /> : null}
          </button>
        ) : null}
        <FixtureGlyph kind={row.kind} className={`shrink-0 ${known ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`} />
        {renaming ? (
          <RenameField
            name={row.name}
            onCommit={(name, byKey) => {
              setRenaming(false);
              if (name.trim() !== "" && name !== row.name) api.rename(row.id, name.trim());
              if (byKey) backToRow();
            }}
            onCancel={() => {
              setRenaming(false);
              backToRow();
            }}
          />
        ) : (
          <button
            type="button"
            aria-label={rowName(row)}
            title={row.name}
            tabIndex={-1}
            className={`min-w-0 flex-1 truncate text-left text-[13px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${row.active ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
            onClick={() => api.select(row.id)}
            onDoubleClick={() => setRenaming(true)}
          >
            {row.name}
          </button>
        )}
        {known && !renaming ? (
          <button
            type="button"
            aria-label={`Show ${row.name}`}
            aria-pressed={row.visible}
            tabIndex={-1}
            className={ICON_BUTTON}
            onClick={() => api.setVisible(row.id, !row.visible)}
          >
            {row.visible ? (
              <Eye size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            ) : (
              <EyeOff size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            )}
          </button>
        ) : null}
        {renaming ? null : trash}
      </div>
      <p className="h-4 pl-6 text-[11px] leading-4 text-[var(--panel-muted-fg)] pointer-coarse:h-5 pointer-coarse:leading-5">{rowLine(row)}</p>
      {selected ? <FixtureFields row={row} view={view} api={api} arm={arm} /> : null}
    </li>
  );
}

/** The name field of an open rename: Enter or blur commits, Escape cancels. */
function RenameField({ name, onCommit, onCancel }: { name: string; onCommit: (text: string, byKey: boolean) => void; onCancel: () => void }) {
  const [text, setText] = useState(name);
  const done = useRef(false);
  const finish = (commit: boolean, byKey = false) => {
    if (done.current) return;
    done.current = true;
    if (commit) onCommit(text, byKey);
    else onCancel();
  };
  return (
    <input
      autoFocus
      onFocus={(event) => event.currentTarget.select()}
      aria-label={`Name of ${name}`}
      maxLength={40}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => finish(true)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          finish(true, true);
        } else if (event.key === "Escape") {
          event.preventDefault();
          finish(false);
        }
      }}
      className="h-6 min-w-0 flex-1 rounded-[4px] border border-[var(--editor-accent)] bg-white px-1 text-[13px] outline-none"
    />
  );
}

/** The typed fields of a selected fixture, by kind (`ux-notes.md` section 4.2). */
function FixtureFields({ row, view, api, arm }: { row: FixtureRow; view: FixturesView; api: FixturesApi; arm: Arm }) {
  if (row.kind === "grid" && row.grid) {
    const grid = row.grid;
    const tapes: Array<["bottom" | "left" | "top" | "right", string]> = [
      ["bottom", "Bottom"],
      ["left", "Left"],
      ["top", "Top"],
      ["right", "Right"],
    ];
    return (
      <div className="flex flex-col gap-2">
        <MmField label="Spacing" unit="mm" value={grid.spacingMm} onCommit={(mm) => api.setLength(row.id, "spacing_mm", mm)} />
        <CountField label="Major every" unit="lines" value={grid.majorEvery} onCommit={(n) => api.setMajorEvery(row.id, n)} />
        <div role="group" aria-label="Tapes on" className="flex gap-1">
          {tapes.map(([edge, word]) => (
            <button
              key={edge}
              type="button"
              aria-label={`Tape on ${edge}`}
              aria-pressed={grid.tapes[edge]}
              className={`h-7 w-[58px] rounded-md border outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] text-xs ${grid.tapes[edge] ? "bg-[var(--toolbar-icon)] text-white" : "bg-white text-[var(--toolbar-icon)]"}`}
              onClick={() => api.setTape(row.id, edge, !grid.tapes[edge])}
            >
              {word}
            </button>
          ))}
        </div>
        <div className="flex gap-1">
          <MmField label="Origin X" unit="mm" value={grid.originXMm} onCommit={(mm) => api.setLength(row.id, "origin_x_mm", mm)} half />
          <MmField label="Origin Y" unit="mm" value={grid.originYMm} onCommit={(mm) => api.setLength(row.id, "origin_y_mm", mm)} half />
        </div>
      </div>
    );
  }
  // A holder has no Width or Height field: its handles and chips size it (revised criterion 18).
  if (row.kind === "holder") {
    const selectedSlot = row.slots.find((slot) => slot.id === view.selectedSlot) ?? null;
    return (
      <div className="flex flex-col gap-2">
        <div className="flex h-6 items-center justify-between text-xs text-[var(--panel-muted-fg)]">
          <span>Slots</span>
          <span>{slotCountText(row.slots.length)}</span>
        </div>
        <ul aria-label="Slots" className="flex flex-col">
          {row.slots.map((slot) => (
            <SlotItem key={slot.id} slot={slot} selected={slot.id === view.selectedSlot} api={api} arm={arm} />
          ))}
        </ul>
        {row.canAddSlot && row.slots.length < MAX_SLOTS ? (
          <button
            type="button"
            className="h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
            onClick={() => api.addSlot(row.id)}
          >
            Add slot
          </button>
        ) : null}
        {selectedSlot ? <SlotFields slot={selectedSlot} api={api} /> : null}
      </div>
    );
  }
  // Clamp, stop and screw have no field in the block: their handles and chips size them
  // (revised criteria 22, 23, 38).
  return null;
}

/** One slot row: its shape, name, "Outside the board" as text, and the armed trash. */
function SlotItem({ slot, selected, api, arm }: { slot: SlotRow; selected: boolean; api: FixturesApi; arm: Arm }) {
  const armed = arm.armed === slot.id;
  const label = armed ? `Press again to delete ${slot.name}` : `Delete ${slot.name}`;
  return (
    <li
      {...{ [ARM_ZONE]: slot.id }}
      className={`flex h-7 items-center gap-2 rounded-md px-1 ${armed ? "bg-[var(--destructive-soft)]" : selected ? "bg-[var(--value-fill)]" : ""}`}
    >
      <button
        type="button"
        aria-label={`${slot.name}, ${slot.shape === "rect" ? "rectangle" : "circle"}, ${slot.wMm} by ${slot.hMm} mm`}
        className="flex min-w-0 flex-1 items-center gap-2 text-left text-[13px] text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        onClick={() => api.selectSlot(slot.id)}
      >
        <SlotGlyph shape={slot.shape} className="shrink-0" />
        <span className="truncate">{slot.name}</span>
        {slot.outside ? <span className="shrink-0 text-[11px] text-[var(--panel-muted-fg)]">Outside the board</span> : null}
      </button>
      <button
        type="button"
        {...{ [ARM_TRASH]: slot.id }}
        aria-label={label}
        title={label}
        className={
          armed
            ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
            : `${ICON_BUTTON} text-[var(--panel-muted-fg)]`
        }
        onClick={() =>
          arm.press(slot.id, label, () => {
            api.removeSlot(slot.id);
            return `${slot.name} removed.`;
          })
        }
      >
        <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
      </button>
    </li>
  );
}

/** The fields of the selected slot: name, shape, centre, size. */
function SlotFields({ slot, api }: { slot: SlotRow; api: FixturesApi }) {
  const round = slot.shape === "circle";
  return (
    <div className="flex flex-col gap-2 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_20%,transparent)] pt-2">
      <TextField key={`${slot.id}:${slot.name}`} label="Name" value={slot.name} onCommit={(name) => api.renameSlot(slot.id, name)} />
      <div role="radiogroup" aria-label="Shape" className="flex gap-1">
        {(["rect", "circle"] as const).map((shape) => (
          <button
            key={shape}
            type="button"
            role="radio"
            aria-checked={slot.shape === shape}
            className={`h-7 w-[118px] rounded-md border outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] text-xs ${slot.shape === shape ? "bg-[var(--toolbar-icon)] text-white" : "bg-white text-[var(--toolbar-icon)]"}`}
            onClick={() => api.setSlotShape(slot.id, shape)}
          >
            {shape === "rect" ? "Rectangle" : "Circle"}
          </button>
        ))}
      </div>
      <div className="flex gap-1">
        <MmField label="Centre X" unit="mm" value={slot.cxMm} onCommit={(mm) => api.setSlotLength(slot.id, "cx_mm", mm)} half />
        <MmField label="Centre Y" unit="mm" value={slot.cyMm} onCommit={(mm) => api.setSlotLength(slot.id, "cy_mm", mm)} half />
      </div>
      {round ? (
        <MmField label="Diameter" unit="mm" value={slot.wMm} onCommit={(mm) => api.setSlotLength(slot.id, "d_mm", mm)} />
      ) : (
        <div className="flex gap-1">
          <MmField label="Width" unit="mm" value={slot.wMm} onCommit={(mm) => api.setSlotLength(slot.id, "w_mm", mm)} half />
          <MmField label="Height" unit="mm" value={slot.hMm} onCommit={(mm) => api.setSlotLength(slot.id, "h_mm", mm)} half />
        </div>
      )}
    </div>
  );
}

/** A bar number field: the label inside the left edge, the unit inside the right; Enter or blur
 * commits, Escape reverts, Up and Down step by 1 (Shift 10) and commit on key-up. While nothing is
 * typed the field shows the stored value, so a value the session clamped reads as what was kept. */
function MmField({ label, unit, value, onCommit, half = false }: { label: string; unit: string; value: number; onCommit: (mm: number) => void; half?: boolean }) {
  const shown = String(Number(value.toFixed(3)));
  const [draft, setDraft] = useState<string | null>(null);
  const text = draft ?? shown;
  const commit = () => {
    if (draft === null) return;
    const number = Number(draft.replace(",", "."));
    setDraft(null);
    if (draft.trim() === "" || !Number.isFinite(number)) return;
    if (number !== value) onCommit(number);
    // A clamp that lands on the stored value changes nothing in the view; the field reads it again.
  };
  return (
    <label className={`flex h-7 items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)] ${half ? "w-[118px]" : "w-[244px]"}`}>
      <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{label}</span>
      <input
        aria-label={`${label} (${unit})`}
        inputMode="decimal"
        value={text}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            setDraft(null);
          } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
            event.preventDefault();
            const current = Number((draft ?? shown).replace(",", "."));
            if (!Number.isFinite(current)) return;
            const step = (event.shiftKey ? 10 : 1) * (event.key === "ArrowUp" ? 1 : -1);
            setDraft(String(Number((current + step).toFixed(3))));
          }
        }}
        onKeyUp={(event) => {
          if (event.key === "ArrowUp" || event.key === "ArrowDown") commit();
        }}
        className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none"
      />
      <span aria-hidden className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{unit}</span>
    </label>
  );
}

/** A whole-count field (Major every): the same bar as the mm fields, the unit "lines". */
function CountField({ label, unit, value, onCommit }: { label: string; unit: string; value: number; onCommit: (n: number) => void }) {
  return (
    <MmField
      label={label}
      unit={unit}
      value={value}
      onCommit={(n) => onCommit(Math.round(n))}
    />
  );
}

/** A text field (a slot's name); Enter or blur commits, Escape reverts. */
function TextField({ label, value, onCommit }: { label: string; value: string; onCommit: (text: string) => void }) {
  const [text, setText] = useState(value);
  const commit = () => {
    if (text.trim() !== "" && text !== value) onCommit(text.trim());
  };
  return (
    <label className="flex h-7 w-[244px] items-center gap-1 rounded-[5px] border border-[var(--field-border)] bg-white px-2 text-sm focus-within:ring-2 focus-within:ring-[var(--editor-accent)]">
      <span className="shrink-0 text-xs text-[var(--panel-muted-fg)]">{label}</span>
      <input
        aria-label={label}
        maxLength={40}
        value={text}
        onChange={(event) => setText(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            setText(value);
          }
        }}
        className="min-w-0 flex-1 bg-transparent outline-none"
      />
    </label>
  );
}

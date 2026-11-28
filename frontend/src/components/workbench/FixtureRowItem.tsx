import { Check, Eye, EyeOff, Trash2 } from "lucide-react";
import { GLYPH, SMALL_GLYPH } from "@/components/ui/looks";
import { type KeyboardEvent, useRef, useState } from "react";

import { FixtureBody } from "@/components/workbench/FixtureBody";
import { RenameField } from "@/components/workbench/FixtureFields";
import { FixtureGlyph } from "@/components/workbench/FixtureGlyphs";
import { FixtureLockButton } from "@/components/workbench/FixtureLockButton";
import { ARMED_TRASH, ICON_BUTTON, addBarStop, checkBoxClass, focusAfterRemoval, walkRowControls } from "@/components/workbench/rowKeys";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { rowKey, rowKeyAction, rowName, tabStopRow } from "@/lib/fixtureRows";
import { fixtureArmStatus, removedStatus, renamedText } from "@/lib/fixtureSets";
import type { FixtureRow, FixturesView } from "@/lib/fixturesView";
import { Tooltip } from "@/components/ui/tooltip";

const LINE1_CONTROLS =
  "[data-line1] > button[role=checkbox], [data-line1] > button[aria-pressed], [data-line1] > button[data-arm-trash]";

/** What a list of fixture rows shares: the view, the commands, the one armed trash and the one unfolded row. */
export interface FixtureListProps {
  view: FixturesView;
  api: FixturesApi;
  arm: DeleteArm;
  /** The one unfolded fixture row of the whole tab (F13). */
  openId: string | null;
  onOpen: (row: FixtureRow) => void;
  /** The list's accessible name. */
  label: string;
  rows: readonly FixtureRow[];
  /** The rows are job fixtures: they carry the Lock toggle (criterion 31). */
  job?: boolean;
  /** The set the rows belong to: its name is in their Lock tooltip (criterion 32); `null` for job fixtures. */
  setName?: string | null;
  /** The name of the add bar above the list. */
  addBarLabel: string;
}

/**
 * A list of fixture rows, the one Tab stop following the row last focused (roving focus,
 * `0102` UX notes 10). The rows of a set and the job fixtures are each one such list.
 */
export function FixtureRows(props: FixtureListProps) {
  const { rows, view } = props;
  const [focusedId, setFocusedId] = useState<string | null>(null);
  if (rows.length === 0) return null;
  const stop = tabStopRow(
    rows.map((row) => row.id),
    view.selected,
    focusedId,
  );
  return (
    <ul aria-label={props.label} className="flex flex-col">
      {rows.map((row, index) => (
        <FixtureRowItem key={rowKey(row)} {...props} row={row} index={index} tabStop={row.id === stop} onFocusRow={() => setFocusedId(row.id)} />
      ))}
    </ul>
  );
}

interface RowProps extends FixtureListProps {
  row: FixtureRow;
  index: number;
  tabStop: boolean;
  onFocusRow: () => void;
}

/**
 * One fixture row (`specs/0102-laser-fixtures` `ux-notes.md` 3.3, `0113` 4.3 and 5.2): the Active
 * check, glyph, name (renamed in place), the Lock of a job fixture, the eye and the armed trash,
 * line 2 from the session, and, unfolded, the fields of its kind.
 */
function FixtureRowItem({ row, index, rows, tabStop, onFocusRow, view, api, arm, openId, onOpen, addBarLabel, job = false, setName = null }: RowProps) {
  // The sets of a page whose profile is not here are read only: no content control (criterion 46).
  const readOnly = !job && view.readOnly !== null;
  const [renaming, setRenaming] = useState(false);
  const item = useRef<HTMLLIElement>(null);
  // Enter or Escape in the name field gives focus back to the row (the field is gone by then).
  const backToRow = () => requestAnimationFrame(() => item.current?.focus());
  const selected = view.selected === row.id;
  const unfolded = openId === row.id;
  const known = row.kind !== "unknown";
  const armed = arm.armed === row.id;
  const removeRow = () =>
    arm.press(row.id, fixtureArmStatus(row.name), () => {
      focusAfterRemoval(item.current, () => addBarStop(addBarLabel));
      api.remove(row.id);
      return removedStatus(row.name);
    });
  const trashLabel = armed ? `Press again to delete ${row.name}` : `Delete ${row.name}`;
  const onRowKey = (event: KeyboardEvent<HTMLLIElement>) => {
    if (walkRowControls(event, LINE1_CONTROLS)) return;
    if (event.target !== event.currentTarget || event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onOpen(row);
      return;
    }
    const action = rowKeyAction(event.key, index, rows.length, armed);
    if (action === null) return;
    event.preventDefault();
    if (action.kind === "focus") {
      const list = event.currentTarget.parentElement?.children;
      (list?.[action.index] as HTMLElement | undefined)?.focus();
    } else if (readOnly) {
      return;
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
      aria-current={selected ? "true" : undefined}
      onFocus={(event) => {
        if (event.target === event.currentTarget) onFocusRow();
      }}
      onKeyDown={onRowKey}
      className={`flex flex-col rounded-[5px] px-1 outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${armed ? "bg-[var(--destructive-soft)]" : selected ? "bg-[var(--value-fill)] shadow-[inset_3px_0_0_var(--editor-accent)]" : ""}`}
    >
      <div data-line1 className="flex h-6 items-center gap-2 pointer-coarse:h-7">
        {known && !renaming ? (
          <button
            type="button"
            role="checkbox"
            aria-checked={row.active}
            aria-label={`On the bed ${row.name}`}
            tabIndex={-1}
            className={checkBoxClass(row.active)}
            onClick={() => api.setActive(row.id, !row.active)}
          >
            {row.active ? <Check {...SMALL_GLYPH} aria-hidden /> : null}
          </button>
        ) : null}
        <FixtureGlyph kind={row.kind} className={`shrink-0 ${known && row.onBed ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`} />
        {renaming ? (
          <RenameField
            name={row.name}
            label={`Name of ${row.name}`}
            onCommit={(text, byKey) => {
              setRenaming(false);
              const name = renamedText(text, row.name);
              if (name !== null) api.rename(row.id, name);
              if (byKey) backToRow();
            }}
            onCancel={() => {
              setRenaming(false);
              backToRow();
            }}
          />
        ) : (
          <Tooltip side="left" content={row.name}>
            <button
              type="button"
              aria-label={rowName(row)}
              aria-expanded={unfolded}
              tabIndex={-1}
              className={`min-w-0 flex-1 truncate text-left text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${row.onBed ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}
              onClick={() => onOpen(row)}
              onDoubleClick={readOnly ? undefined : () => setRenaming(true)}
            >
              {row.name}
            </button>
          </Tooltip>
        )}
        {!readOnly && !renaming ? (
          <FixtureLockButton
            name={row.name}
            locked={row.locked}
            lockedWith={row.lockedBySet ? (setName ?? "its set") : null}
            onPress={() => api.setLocked(row.id, !row.locked)}
          />
        ) : null}
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
              <Eye {...GLYPH} aria-hidden />
            ) : (
              <EyeOff {...GLYPH} aria-hidden />
            )}
          </button>
        ) : null}
        {renaming || readOnly ? null : (
          <Tooltip side="left" content={trashLabel}>
            <button
              type="button"
              {...{ [ARM_TRASH]: row.id }}
              aria-label={trashLabel}
              tabIndex={-1}
              className={armed ? ARMED_TRASH : `${ICON_BUTTON} text-[var(--panel-muted-fg)]`}
              onClick={removeRow}
            >
              <Trash2 {...GLYPH} aria-hidden />
            </button>
          </Tooltip>
        )}
      </div>
      <p title={row.line2} className="h-4 truncate pl-6 text-[11px] leading-4 text-[var(--panel-muted-fg)] pointer-coarse:h-5 pointer-coarse:leading-5">
        {row.line2}
      </p>
      {unfolded ? (
        <div className="pl-4 pt-1">
          <FixtureBody row={row} selectedSlot={view.selectedSlot} api={api} arm={arm} readOnly={readOnly} />
        </div>
      ) : null}
    </li>
  );
}

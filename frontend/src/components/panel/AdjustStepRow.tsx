import { ChevronDown, ChevronUp, MoveHorizontal, Power, PowerOff, Trash2 } from "lucide-react";
import type { KeyboardEvent, PointerEvent, ReactNode } from "react";

import { AdjustGlyph } from "@/components/panel/AdjustGlyphs";
import { EntryField } from "@/components/panel/EntryField";
import { LevelsGraph } from "@/components/panel/LevelsGraph";
import { Tooltip } from "@/components/ui/tooltip";
import { ARM_TRASH, ARM_ZONE } from "@/hooks/useDeleteArm";
import { fieldStep, fieldText, typedField } from "@/lib/adjustText";
import type { StepField, StepRow } from "@/lib/adjustmentsView";

const CONTROL =
  "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** What a manual row asks of the list: the session calls and the row's own states. */
export interface StepActions {
  fold: (index: number) => void;
  setEnabled: (index: number, on: boolean) => void;
  /** A press on the trash: arms it, or removes the row when armed. */
  trash: (row: StepRow) => void;
  move: (index: number, up: boolean) => void;
  editField: (index: number, field: StepField, text: string) => string;
  /** The Stretch button of a Levels row. */
  stretch: (index: number) => void;
  /** A pointer press on the glyph or name starts a reorder drag. */
  dragStart: (index: number, event: PointerEvent<HTMLElement>) => void;
}

interface RowProps {
  row: StepRow;
  position: number;
  total: number;
  unfolded: boolean;
  armed: boolean;
  dragSource: boolean;
  insertion: "top" | "bottom" | null;
  /** The roving Tab stop of the list. */
  tabStop: boolean;
  onFocusRow: () => void;
  actions: StepActions;
}

function FieldView({ row, field, actions }: { row: StepRow; field: StepField; actions: StepActions }) {
  const index = row.index as number;
  return (
    <div data-adjust-field={field.key}>
      <EntryField
        label={`${row.name} ${field.label}${field.unit === "" ? "" : `, ${field.unit}`}`}
        prefix={field.label}
        shown={fieldText(field.value, field.decimals)}
        mixed={false}
        suffix={field.unit === "" ? undefined : field.unit}
        width={112}
        gutter={field.unit === "" ? 8 : undefined}
        commitOnBlur
        inputMode="decimal"
        messages={{ range: field.range }}
        stepText={fieldStep(field.decimals)}
        onSubmit={(text) => {
          const typed = typedField(text);
          if (!Number.isFinite(typed)) return "invalid:range";
          if (Number(typed.toFixed(field.decimals)) === Number(field.value.toFixed(field.decimals))) return "unchanged";
          return actions.editField(index, field, text) === "" ? "committed" : "invalid:range";
        }}
        onReturnFocus={() => undefined}
      />
    </div>
  );
}

/** A key on the name button: Enter or Space folds, Delete arms or removes, Alt+Up and Alt+Down
 * move inside the band, Right goes to the row's power toggle (Space there); the plain Up and Down
 * are the list's. The row's other buttons are no Tab stops (`ux-notes.md` §13). */
function onNameKey(event: KeyboardEvent, row: StepRow, armed: boolean, actions: StepActions) {
  const index = row.index as number;
  if (!event.altKey && event.key === "ArrowRight") {
    const power = (event.currentTarget as HTMLElement).closest("li")?.querySelector<HTMLElement>("[data-adjust-power]");
    if (power) {
      event.preventDefault();
      power.focus();
    }
  } else if (event.altKey && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
    event.preventDefault();
    if (event.key === "ArrowUp" ? row.up : row.down) actions.move(index, event.key === "ArrowUp");
  } else if (event.key === "Delete" || (armed && event.key === "Enter")) {
    event.preventDefault();
    actions.trash(row);
  }
}

/** One manual step row (`ux-notes.md` §3.6): glyph, name (a button that folds), power, armed trash,
 * a gap, the arrows; line 2 the numbers, or the fields when unfolded. An unknown step has its
 * trash only. */
export function AdjustStepRow(props: RowProps) {
  const { row, armed, actions } = props;
  const index = row.index as number;
  const muted = !row.enabled || row.presetInUse;
  const key = `adj:${index}`;
  return (
    <li
      data-adjust-row={index}
      data-adjust-kind={row.kind}
      aria-posinset={props.position + 1}
      aria-setsize={props.total}
      {...{ [ARM_ZONE]: key }}
      className={`relative flex flex-col rounded px-1 ${props.dragSource ? "bg-[color-mix(in_srgb,var(--toolbar-icon)_8%,transparent)]" : ""}`}
    >
      {props.insertion ? (
        <div
          aria-hidden
          data-insertion={props.insertion}
          className="pointer-events-none absolute left-0 right-0 h-[2px] bg-[var(--editor-accent)]"
          style={props.insertion === "top" ? { top: 0 } : { bottom: 0 }}
        />
      ) : null}
      <div className={`flex flex-col rounded ${armed ? "bg-[var(--destructive-soft)]" : ""}`}>
        <div
          className="flex h-[26px] shrink-0 items-center gap-1"
          onPointerDown={(event) => {
            if (!row.unknown && (event.target as HTMLElement).closest("button:not([data-adjust-name]), input") === null) {
              actions.dragStart(index, event);
            }
          }}
        >
          <span className="flex h-4 w-4 shrink-0 items-center justify-center text-[var(--toolbar-icon)]">
            <AdjustGlyph kind={row.kind} />
          </span>
          <button
            type="button"
            data-adjust-name=""
            tabIndex={props.tabStop ? 0 : -1}
            aria-expanded={row.unknown ? undefined : props.unfolded}
            aria-label={row.accessibleName}
            title={row.name}
            onFocus={props.onFocusRow}
            onClick={() => !row.unknown && actions.fold(index)}
            onKeyDown={(event) => onNameKey(event, row, armed, actions)}
            className={`min-w-0 flex-1 truncate rounded-sm text-left text-[13px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] ${muted ? "text-[var(--panel-muted-fg)]" : "text-[var(--toolbar-icon)]"}`}
          >
            {row.name}
          </button>
          {row.unknown ? null : (
            <button
              type="button"
              data-adjust-power=""
              tabIndex={-1}
              onKeyDown={(event) => {
                if (event.key !== "ArrowLeft") return;
                event.preventDefault();
                (event.currentTarget.closest("li")?.querySelector("[data-adjust-name]") as HTMLElement | null)?.focus();
              }}
              aria-pressed={row.enabled}
              aria-label={`Use ${row.name}`}
              title={`Use ${row.name}`}
              onClick={() => actions.setEnabled(index, !row.enabled)}
              className={CONTROL}
            >
              {row.enabled ? <Power size={16} aria-hidden /> : <PowerOff size={16} aria-hidden />}
            </button>
          )}
          <button
            type="button"
            {...{ [ARM_TRASH]: key }}
            tabIndex={-1}
            aria-label={armed ? `Press again to remove ${row.name}` : `Remove ${row.name}`}
            title={armed ? "Press again to remove" : "Remove"}
            onClick={() => actions.trash(row)}
            className={
              armed
                ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
                : CONTROL
            }
          >
            <Trash2 size={16} aria-hidden />
          </button>
          {row.unknown ? null : (
            <>
              <span className="w-1 shrink-0" />
              {row.up ? (
                <button type="button" tabIndex={-1} aria-label={`Move ${row.name} up`} title="Move up" onClick={() => actions.move(index, true)} className={CONTROL}>
                  <ChevronUp size={16} aria-hidden />
                </button>
              ) : (
                <span className="h-6 w-6 shrink-0" aria-hidden />
              )}
              {row.down ? (
                <button type="button" tabIndex={-1} aria-label={`Move ${row.name} down`} title="Move down" onClick={() => actions.move(index, false)} className={CONTROL}>
                  <ChevronDown size={16} aria-hidden />
                </button>
              ) : (
                <span className="h-6 w-6 shrink-0" aria-hidden />
              )}
            </>
          )}
        </div>
        {props.unfolded && !row.unknown ? null : (
          <p title={row.line} className="h-4 truncate pl-5 text-[11px] leading-4 tabular-nums text-[var(--panel-muted-fg)]">
            {row.line}
          </p>
        )}
      </div>
      {props.unfolded && !row.unknown ? (
        <div data-adjust-body="" className="ml-5 mt-0.5 mb-2 flex w-[228px] flex-wrap gap-1">
          {row.kind === "levels" ? (
            <LevelsGraph
              row={row}
              onSet={(key, value) => {
                const field = row.fields.find((f) => f.key === key);
                if (field) actions.editField(index, field, String(value));
              }}
            />
          ) : null}
          {row.fields.map((field) => (
            <FieldView key={field.key} row={row} field={field} actions={actions} />
          ))}
          {row.kind === "levels" ? (
            <Tooltip side="top" content="Stretch to the picture's range">
              <button
                type="button"
                aria-label="Stretch"
                onClick={() => actions.stretch(row.index as number)}
                className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                <MoveHorizontal aria-hidden size={16} strokeWidth={1.5} />
              </button>
            </Tooltip>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

/** One read-only row of the derived block (`ux-notes.md` §3.4): glyph, name, the resolved numbers. */
export function DerivedRow({ row }: { row: StepRow }): ReactNode {
  return (
    <li data-derived-row={row.kind} className="flex flex-col px-1" aria-label={row.accessibleName}>
      <div className="flex h-[26px] items-center gap-1 text-[13px] text-[var(--toolbar-icon)]">
        <span className="flex h-4 w-4 shrink-0 items-center justify-center">
          <AdjustGlyph kind={row.kind} />
        </span>
        <span className="min-w-0 flex-1 truncate">{row.name}</span>
      </div>
      <p title={row.line} className="h-4 truncate pl-5 text-[11px] leading-4 tabular-nums text-[var(--panel-muted-fg)]">
        {row.line}
      </p>
    </li>
  );
}

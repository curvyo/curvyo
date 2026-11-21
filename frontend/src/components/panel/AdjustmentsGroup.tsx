import { ClipboardPaste, Columns2, Copy, RotateCcw } from "lucide-react";
import { Fragment, useEffect, useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { AdjustGlyph } from "@/components/panel/AdjustGlyphs";
import { AdjustStrength } from "@/components/panel/AdjustStrength";
import { AdjustStepRow, DerivedRow, type StepActions } from "@/components/panel/AdjustStepRow";
import { DitherFields, DitherModeStrip } from "@/components/panel/DitherStrip";
import { PresetStrip } from "@/components/panel/PresetStrip";
import { Tooltip } from "@/components/ui/tooltip";
import { useDeleteArm } from "@/hooks/useDeleteArm";
import type { AdjustmentsApi } from "@/hooks/useAdjustments";
import { useStepDrag } from "@/hooks/useStepDrag";
import {
  bandBreak,
  insertedAt,
  kindsOf,
  openAfterInsert,
  openAfterMove,
  openAfterRemove,
  typedField,
} from "@/lib/adjustText";
import type { AddButton, AdjustmentsView, PresetKey, StepRow } from "@/lib/adjustmentsView";

const RULE = { background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" };
const HEADER_BUTTON =
  "flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-disabled:cursor-default aria-disabled:opacity-40 aria-disabled:hover:bg-transparent";
const OUTLINE_BUTTON =
  "h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The strip's two fixed rows: a break after Black and white (`ux-notes.md` §3.2). */
const PRESET_BREAK = ["black_white"];

function Hairline() {
  return <div aria-hidden className="my-1 h-px" style={RULE} />;
}

/** The add toolbar (`ux-notes.md` §3.5): step glyph buttons, one Tab stop, arrows move, Enter or
 * Space presses. Rows with no button are not drawn. */
function AddToolbar({ rows, onAdd }: { rows: AddButton[][]; onAdd: (kind: AddButton["kind"]) => void }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const [stop, setStop] = useState(0);
  const flat = rows.flat();
  if (flat.length === 0) return null;
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const next = Math.min(flat.length - 1, Math.max(0, stop + step));
    setStop(next);
    refs.current[next]?.focus();
  };
  const starts = rows.map((_, r) => rows.slice(0, r).reduce((n, row) => n + row.length, 0));
  return (
    <div role="toolbar" aria-label="Add adjustment" className="flex flex-col gap-1" onKeyDown={onKeyDown}>
      {rows.map((row, r) => (
        <div key={r} className="flex gap-1">
          {row.map((button, b) => {
            const index = starts[r] + b;
            return (
              <Tooltip key={button.kind} side="top" content={button.name}>
                <button
                  ref={(node) => {
                    refs.current[index] = node;
                  }}
                  type="button"
                  aria-label={button.name}
                  tabIndex={index === stop ? 0 : -1}
                  onFocus={() => setStop(index)}
                  onClick={() => onAdd(button.kind)}
                  className={HEADER_BUTTON}
                >
                  <AdjustGlyph kind={button.kind} />
                </button>
              </Tooltip>
            );
          })}
        </div>
      ))}
    </div>
  );
}

/** The step list with its bands, drag and arrows (`ux-notes.md` §3.6, §3.7). */
function StepList({
  rows,
  adjust,
  open,
  setOpen,
}: {
  rows: StepRow[];
  adjust: AdjustmentsApi;
  open: ReadonlySet<number>;
  setOpen: (next: Set<number>) => void;
}) {
  const list = useRef<HTMLUListElement>(null);
  const arm = useDeleteArm();
  const [stop, setStop] = useState(0);
  // The row a keyboard move went to: its name takes the focus again once the list is drawn.
  const moved = useRef<number | null>(null);
  useEffect(() => {
    const at = moved.current;
    if (at === null) return;
    moved.current = null;
    setStop(at);
    list.current?.querySelector<HTMLElement>(`[data-adjust-row="${at}"] [data-adjust-name]`)?.focus();
  }, [rows]);
  const moveTo = (from: number, to: number) => {
    // One drag is one step "Edit adjustments" (criterion 2).
    if (adjust.moveTo(from, to)) setOpen(openAfterMove(open, from, to));
  };
  const { drag, start } = useStepDrag(rows, list, moveTo);
  const actions: StepActions = {
    fold: (index) => {
      const next = new Set(open);
      if (!next.delete(index)) next.add(index);
      setOpen(next);
    },
    setEnabled: (index, on) => void adjust.setEnabled(index, on),
    trash: (row) => {
      const index = row.index as number;
      arm.press(`adj:${index}`, `Press again to remove ${row.name}`, () => {
        if (adjust.remove(index)) setOpen(openAfterRemove(open, index));
        return `${row.name} removed`;
      });
    },
    move: (index, up) => {
      const to = index + (up ? -1 : 1);
      if (!adjust.move(index, up)) return;
      setOpen(openAfterMove(open, index, to));
      if (list.current?.querySelector(`[data-adjust-row="${index}"]`)?.contains(document.activeElement)) moved.current = to;
    },
    editField: (index, field, text) => adjust.editField(index, field.key, typedField(text)),
    stretch: (index) => void adjust.stretch(index),
    dragStart: (index, event) => start(index, event),
    curve: (index) => ({
      add: (input, output) => adjust.addCurvePoint(index, input, output),
      move: (point, input, output) => void adjust.moveCurvePoint(index, point, input, output),
      remove: (point) => void adjust.removeCurvePoint(index, point),
    }),
    straighten: (index) => void adjust.straightenCurve(index),
    channelPreset: (index, preset) => void adjust.setChannelPreset(index, preset),
    openCrop: (index) => void adjust.openCrop(index),
    cropBarRow: adjust.view?.cropBarRow ?? null,
  };
  const onKeyDown = (event: KeyboardEvent<HTMLUListElement>) => {
    if (!(event.target as HTMLElement).hasAttribute("data-adjust-name") || event.altKey) return;
    const names = [...(list.current?.querySelectorAll<HTMLElement>("[data-adjust-name]") ?? [])];
    const at = names.indexOf(event.target as HTMLElement);
    const next =
      event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? names.length - 1 : null;
    if (next === null || next < 0 || next >= names.length) return;
    event.preventDefault();
    setStop(next);
    names[next].focus();
  };
  return (
    <>
      <ul
        ref={list}
        role="list"
        aria-label="Adjustment steps"
        className="flex flex-col gap-1"
        style={drag && drag.slot === null ? { cursor: "not-allowed" } : undefined}
        onKeyDown={onKeyDown}
      >
        {rows.map((row, i) => (
          // Keyed by position: a move changes the rows' contents, not their elements, and the focus
          // follows the moved row (`moved` above).
          <Fragment key={i}>
            {bandBreak(rows, i) ? <li aria-hidden role="presentation" className="my-1 h-px" style={RULE} /> : null}
            <AdjustStepRow
              row={row}
              position={i}
              total={rows.length}
              unfolded={open.has(i)}
              armed={arm.armed === `adj:${i}`}
              dragSource={drag?.from === i && drag.slot !== null}
              insertion={drag && drag.slot === i ? "top" : drag && drag.slot === rows.length && i === rows.length - 1 ? "bottom" : null}
              tabStop={i === Math.min(stop, rows.length - 1)}
              onFocusRow={() => setStop(i)}
              actions={actions}
            />
          </Fragment>
        ))}
      </ul>
      {drag ? (
        <div
          aria-hidden
          className="pointer-events-none fixed z-50 rounded bg-[var(--toolbar-bg)] px-2 py-1 text-[13px] text-[var(--toolbar-icon)] opacity-70"
          style={{ left: drag.x + 8, top: drag.y + 8 }}
        >
          {drag.name}
        </div>
      ) : null}
      <p role="status" className="sr-only">
        {arm.status}
      </p>
    </>
  );
}

/** The header: the name and the symbol buttons (`ux-notes.md` §3.1): Before and after, Copy and
 * Paste adjustments, Restore original. Paste and Restore original are greyed, never hidden, while
 * there is nothing to paste or restore. */
function Header({ view, adjust, onToggleBeforeAfter }: { view: AdjustmentsView; adjust: AdjustmentsApi; onToggleBeforeAfter: () => void }) {
  return (
    <div className="flex h-7 items-center justify-between">
      <h3 className="text-sm font-semibold text-[var(--toolbar-icon)]">Adjustments</h3>
      <div className="flex gap-1">
        <Tooltip side="top" content="Before and after">
          <button
            type="button"
            aria-label="Before and after"
            aria-pressed={view.beforeAfterOpen}
            onClick={onToggleBeforeAfter}
            className={HEADER_BUTTON}
          >
            <Columns2 aria-hidden size={16} strokeWidth={1.5} />
          </button>
        </Tooltip>
        <Tooltip side="top" content="Copy adjustments">
          <button type="button" aria-label="Copy adjustments" onClick={() => void adjust.copyAdjustments()} className={HEADER_BUTTON}>
            <Copy aria-hidden size={16} strokeWidth={1.5} />
          </button>
        </Tooltip>
        <Tooltip side="top" content="Paste adjustments">
          <button
            type="button"
            aria-label="Paste adjustments"
            aria-disabled={!view.pasteEnabled}
            onClick={() => {
              if (view.pasteEnabled) adjust.pasteAdjustments();
            }}
            className={HEADER_BUTTON}
          >
            <ClipboardPaste aria-hidden size={16} strokeWidth={1.5} />
          </button>
        </Tooltip>
        <Tooltip side="top" content="Restore original">
          <button
            type="button"
            aria-label="Restore original"
            aria-disabled={!view.restoreEnabled}
            onClick={() => {
              if (view.restoreEnabled) adjust.restoreOriginal();
            }}
            className={HEADER_BUTTON}
          >
            <RotateCcw aria-hidden size={16} strokeWidth={1.5} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

interface GroupProps {
  adjust: AdjustmentsApi;
  view: AdjustmentsView;
  onToggleBeforeAfter: () => void;
  onReturnFocus: () => void;
}

/**
 * The Adjustments group of the selected picture's Style tab (`specs/0135-image-optimisation`
 * `ux-notes.md` §3): header, state line, the Preset strip in two fixed rows, the picture's Dither
 * strip in three (§3.10), Strength, the dither mode's fields, the derived
 * rows with Make editable while a preset runs, the add toolbar and the manual step list. The
 * session decides everything; the group shows its view and sends presses back.
 */
export function AdjustmentsGroup({ adjust, view, onToggleBeforeAfter, onReturnFocus }: GroupProps) {
  const headingId = useId();
  const [open, setOpen] = useState<ReadonlySet<number>>(() => new Set());
  const group = useRef<HTMLDivElement>(null);
  const focusFirst = useRef(false);
  const added = useRef<string[] | null>(null);
  const kinds = useRef<string[]>([]);
  useEffect(() => {
    kinds.current = kindsOf(view.steps);
  });
  // After Make editable the focus goes to the first manual row; after an add the new row is
  // unfolded and its first field takes the focus (`ux-notes.md` §3.4, §3.5), once it is drawn.
  useEffect(() => {
    if (focusFirst.current && view.steps.length > 0) {
      focusFirst.current = false;
      group.current?.querySelector<HTMLElement>("[data-adjust-name]")?.focus();
    }
    const before = added.current;
    if (before === null) return;
    added.current = null;
    const at = insertedAt(before, kindsOf(view.steps));
    if (at === null) return;
    setOpen((now) => openAfterInsert(now, at));
    window.requestAnimationFrame(() => {
      group.current?.querySelector<HTMLElement>(`[data-adjust-row="${at}"] input`)?.focus();
    });
  }, [view.steps]);

  const addRows = [view.addTone, view.addGeometry].filter((row) => row.length > 0);
  const add = (kind: AddButton["kind"]) => {
    const before = kinds.current;
    // The session inserts at the pipeline place: the new row is where the kinds first differ.
    if (adjust.addStep(kind)) added.current = before;
  };

  return (
    <div ref={group} role="group" aria-labelledby={headingId} data-adjustments="" className="flex flex-col gap-1">
      <span id={headingId} className="sr-only">
        Adjustments
      </span>
      <Header view={view} adjust={adjust} onToggleBeforeAfter={onToggleBeforeAfter} />
      {view.memoryLine ? (
        <p role="alert" className="text-xs text-[var(--field-invalid)]">
          {view.memoryLine}
        </p>
      ) : null}
      {view.stateLine ? <p className="h-4 text-xs text-[var(--panel-muted-fg)]">{view.stateLine}</p> : null}
      <PresetStrip
        label="Preset"
        width="w-full"
        tooltipSide="top"
        fixedRows={{ breakAfter: PRESET_BREAK }}
        cells={view.presets.map((preset) => ({
          id: preset.key,
          content: preset.cell,
          label: preset.name,
          tooltip: preset.tooltip,
        }))}
        value={view.presets.find((preset) => preset.pressed)?.key ?? null}
        onPress={(key) => adjust.pickPreset(key as PresetKey)}
        // The focus stays on the strip: the arrows keep working.
        onReturnFocus={() => undefined}
      />
      {/* The picture's Dither strip, 8 px under the Preset strip (the column's 4 plus 4), no
          label (P28); its mode's fields after Strength, so a preset pick moves nothing above. */}
      <div className="mt-1">
        <DitherModeStrip dither={view.dither} onPress={(mode) => void adjust.setDither(mode)} />
      </div>
      {view.strength ? (
        <AdjustStrength
          value={view.strength.value}
          onHold={adjust.holdStrength}
          onRelease={() => void adjust.releaseStrength()}
          onCancel={adjust.cancelStrength}
          onSet={adjust.setStrength}
          onReturnFocus={onReturnFocus}
        />
      ) : null}
      <DitherFields dither={view.dither} onEdit={adjust.editDitherField} onReturnFocus={onReturnFocus} />
      {view.derived ? (
        <>
          <Hairline />
          <p className="h-4 text-xs text-[var(--panel-muted-fg)]">{view.derived.line}</p>
          <ul role="list" aria-label="Steps from preset" className="flex flex-col gap-1">
            {view.derived.rows.map((row, i) => (
              <DerivedRow key={`${i}:${row.kind}`} row={row} />
            ))}
          </ul>
          <button
            type="button"
            className={OUTLINE_BUTTON}
            onClick={() => {
              if (adjust.makeEditable()) focusFirst.current = true;
            }}
          >
            Make editable
          </button>
        </>
      ) : null}
      {/* No lone hairline: it parts the add toolbar or the steps from what is above. */}
      {addRows.length > 0 || view.steps.length > 0 ? <Hairline /> : null}
      {addRows.length > 0 ? <AddToolbar rows={addRows} onAdd={add} /> : null}
      {view.steps.length > 0 ? <StepList rows={view.steps} adjust={adjust} open={open} setOpen={setOpen} /> : null}
    </div>
  );
}

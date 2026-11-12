import { PencilLine } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { EntryField } from "@/components/panel/EntryField";
import { SaveAsMaterialForm } from "@/components/panel/SaveAsMaterialForm";
import { SegmentGroup } from "@/components/panel/SegmentGroup";
import { TestGridResult } from "@/components/panel/TestGridResult";
import { PanelSwitch } from "@/components/ui/switch";
import type { MaterialsApi } from "@/hooks/useMaterials";
import type { TestGridApi } from "@/hooks/useTestGrid";
import { dropSaveDraft, keptDraft } from "@/lib/saveDraft";
import { CHIP_IS_MESSAGE, outcomeOf, stepText } from "@/lib/settingsFieldText";
import {
  CELL_FIELDS,
  GRID_KINDS,
  LABEL_FIELDS,
  POWER_FIELDS,
  SPEED_FIELDS,
  fieldShown,
  sizeLine,
  type GridFieldSpec,
} from "@/lib/testGridFields";
import type { TestGridView } from "@/lib/testGridView";

const HEADING = "flex h-4 items-center text-[11px] font-semibold text-[var(--panel-muted-fg)]";
const OUTLINE_BUTTON =
  "flex items-center justify-center rounded border border-[var(--lane-line)] text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** One field of the block: the short label inside the left edge, the value, the unit; Enter or leaving commits. */
function GridField({ spec, view, api, onReturnFocus }: { spec: GridFieldSpec; view: TestGridView; api: TestGridApi; onReturnFocus: () => void }) {
  return (
    <div className="relative" style={{ width: spec.width }}>
      {spec.short ? (
        <span aria-hidden className="pointer-events-none absolute top-1/2 left-1.5 z-10 -translate-y-1/2 text-[11px] text-[var(--panel-muted-fg)]">
          {spec.short}
        </span>
      ) : null}
      <EntryField
        label={spec.name}
        shown={fieldShown(view, spec.key)}
        mixed={false}
        suffix={spec.unit || undefined}
        smallSuffix
        width={spec.width}
        gutter={spec.unit === "mm/min" ? 52 : undefined}
        commitOnBlur
        inputMode="decimal"
        onSubmit={(text) => outcomeOf(api.setField(spec.key, text))}
        messages={CHIP_IS_MESSAGE}
        stepText={stepText}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

function Fields({ specs, view, api, onReturnFocus }: { specs: readonly GridFieldSpec[]; view: TestGridView; api: TestGridApi; onReturnFocus: () => void }) {
  return (
    <div className="flex gap-1">
      {specs.map((spec) => (
        <GridField key={spec.key} spec={spec} view={view} api={api} onReturnFocus={onReturnFocus} />
      ))}
    </div>
  );
}

/**
 * The Grid block of the selected test grid (`specs/0028-material-test-library` `ux-notes.md` 6, 7,
 * 9): head with Select, machine and size, the Kind, the fields and Labels, "Edited by hand" with
 * Rebuild, the result table with the Worked line, and Save as material. Nothing here decides a
 * rule; the session does, and the view is re-read after each call.
 */
export function TestGridBlock({ view, api, materials, onReturnFocus }: { view: TestGridView; api: TestGridApi; materials: MaterialsApi; onReturnFocus: () => void }) {
  const [saving, setSaving] = useState(() => view.worked !== null && keptDraft(`${view.group.peer}:${view.group.counter}`) !== null);
  const [status, setStatus] = useState("");
  const [cause, setCause] = useState("");
  const root = useRef<HTMLDivElement>(null);
  const saveButton = useRef<HTMLButtonElement>(null);
  const mark = view.worked ? `${view.worked.cell.column},${view.worked.cell.row}` : "";

  useEffect(() => {
    root.current?.scrollIntoView?.({ block: "nearest" });
  }, []);
  // The mark going away takes the form with it: Save as material needs a marked cell.
  useEffect(() => {
    if (mark === "") setSaving(false);
  }, [mark]);

  const dropDraft = () => {
    dropSaveDraft();
    setSaving(false);
  };
  const withCause = (text: string) => setCause(text);

  return (
    <div ref={root} role="group" aria-label={view.name} className="flex flex-col gap-1">
      <div className="flex h-6 items-center gap-2 text-[13px] font-semibold text-[var(--toolbar-icon)]">
        <OperationGlyph kind={view.kind} />
        <span className="min-w-0 flex-1 truncate" title={view.name}>
          {view.name}
        </span>
        <button
          type="button"
          aria-label={`Select ${view.name} on the canvas`}
          onClick={() => api.selectOnCanvas()}
          className="text-xs font-normal underline-offset-2 hover:underline focus-visible:underline focus-visible:outline-none"
        >
          Select
        </button>
      </div>
      <p className="h-4 truncate text-[11px] tabular-nums text-[var(--panel-muted-fg)]">{sizeLine(view)}</p>
      <SegmentGroup
        name="Kind"
        options={GRID_KINDS.map((entry) => [entry.kind, entry.label] as const)}
        value={view.kind}
        width={78}
        onChange={(kind) => {
          dropDraft();
          withCause(api.setKind(kind));
        }}
        onReturnFocus={onReturnFocus}
      />
      {cause ? <p role="alert" className="text-xs text-[var(--field-invalid)]">{cause}</p> : null}
      <p className={HEADING}>Power, %</p>
      <Fields specs={POWER_FIELDS} view={view} api={api} onReturnFocus={onReturnFocus} />
      <p className={HEADING}>Speed, mm/min</p>
      <Fields specs={SPEED_FIELDS} view={view} api={api} onReturnFocus={onReturnFocus} />
      <p className={HEADING}>Cell, mm</p>
      <Fields specs={CELL_FIELDS} view={view} api={api} onReturnFocus={onReturnFocus} />
      <div className="flex h-7 items-center">
        <PanelSwitch
          label="Labels"
          checked={view.labels.on}
          onCheckedChange={(on) => withCause(api.setLabels(on))}
          onReturnFocus={onReturnFocus}
        />
      </div>
      {view.labels.on ? (
        <div className="flex flex-wrap gap-1">
          {LABEL_FIELDS.map((spec) => (
            <GridField key={spec.key} spec={spec} view={view} api={api} onReturnFocus={onReturnFocus} />
          ))}
        </div>
      ) : null}
      {view.handEdited ? (
        <div className="flex h-7 items-center gap-1 text-xs text-[var(--toolbar-icon)]">
          <PencilLine size={16} aria-hidden className="shrink-0" />
          <span className="flex-1">Edited by hand</span>
          <button
            type="button"
            aria-label={`Rebuild ${view.name}`}
            onClick={() => {
              dropDraft();
              api.rebuild();
            }}
            className={`${OUTLINE_BUTTON} h-6 w-16`}
          >
            Rebuild
          </button>
        </div>
      ) : null}
      <div className="mt-1">
        <TestGridResult view={view} api={api} onMarkChanged={dropDraft} />
      </div>
      {view.worked && !saving ? (
        <button ref={saveButton} type="button" onClick={() => setSaving(true)} className={`${OUTLINE_BUTTON} h-7 w-full`}>
          Save as material
        </button>
      ) : null}
      {view.worked && saving ? (
        <SaveAsMaterialForm
          view={view}
          materials={materials}
          onClose={(saved) => {
            setSaving(false);
            if (saved !== null) setStatus(saved);
            // The form leaves the tree: the focus goes to the button that opened it.
            window.requestAnimationFrame(() => saveButton.current?.focus());
          }}
        />
      ) : null}
      <div role="status" aria-atomic="true" className="sr-only">
        {status}
      </div>
    </div>
  );
}

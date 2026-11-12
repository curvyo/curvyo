import { Diff, Link2Off, RotateCcw, TriangleAlert } from "lucide-react";
import { useId, useRef, useState } from "react";

import { MaterialPick } from "@/components/panel/MaterialPick";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { SettingsNumberFields } from "@/components/panel/SettingsNumberFields";
import type { OperationSettingsApi } from "@/hooks/useOperationSettings";
import {
  blockHead,
  blockSummary,
  canEditMaterial,
  disabledReason,
  fieldName,
  materialDescription,
  materialLine,
  pickValue,
  resetStatus,
  stateLines,
  type StateGlyph,
} from "@/lib/operationSettingsRows";
import type { OperationRow } from "@/lib/operationsView";
import type { OperationSettings, PickMaterial, SetView } from "@/lib/operationSettingsView";

interface OperationSettingsBlockProps {
  /** The selected operations in list order. */
  rows: readonly OperationRow[];
  pick: readonly PickMaterial[];
  api: OperationSettingsApi;
  /** "Edit material": opens the Materials section on that material. */
  onEditMaterial: (id: string) => void;
  onReturnFocus: () => void;
}

const GLYPHS: Record<StateGlyph, typeof Link2Off> = {
  "not-found": Link2Off,
  differs: Diff,
  warning: TriangleAlert,
};

const BUTTON =
  "flex h-7 items-center gap-1 rounded border border-[var(--lane-line)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The two sets of Engrave + Outline have sub-headings; any other kind has one set and none. */
function SetHeading({ set }: { set: SetView }) {
  return (
    <div className="flex h-6 items-center gap-1 text-xs font-semibold text-[var(--toolbar-icon)]">
      <OperationGlyph kind={set.role === "outline" ? "score" : "engrave"} size={14} />
      {set.role === "outline" ? "Outline" : "Hatch"}
    </div>
  );
}

/**
 * The Settings block of the selected operations (`specs/0028-material-test-library`
 * `ux-notes.md` section 8): the head, the Material pick, the state lines, the number fields (also
 * while pointing: a typed number keeps the link and shows the material's beside it) and the
 * buttons Reset to material and Edit material, each only where it applies. With several
 * operations selected only the pick shows.
 */
export function OperationSettingsBlock({ rows, pick, api, onEditMaterial, onReturnFocus }: OperationSettingsBlockProps) {
  const withSettings = rows.filter((row): row is OperationRow & { settings: OperationSettings } => row.settings !== null);
  const [status, setStatus] = useState("");
  const fieldsRef = useRef<HTMLDivElement>(null);
  const summaryId = useId();
  if (withSettings.length === 0) return null;

  const single = withSettings.length === 1 ? withSettings[0] : null;
  const value = pickValue(withSettings);
  const subject = blockHead(withSettings.map((row) => row.name));
  const material = single?.settings.material ?? null;
  const currentId = single ? (material?.id ?? null) : null;
  const ids = withSettings.map((row) => row.id);

  const picker = (
    <MaterialPick
      subject={subject}
      value={value.text}
      mixed={value.mixed}
      currentId={currentId}
      entries={pick}
      disabledFor={(id) => (single && id !== "" ? disabledReason(single.settings, id) : null)}
      onPick={(id) => {
        setStatus("");
        api.pick(ids, id);
      }}
    />
  );
  // The skip notice belongs to a pick over several operations: it shows under the source row of that group.
  const skip = single === null && api.skip !== null ? (
    <p role="status" className="mt-1 text-xs text-[var(--toolbar-icon)]">
      {api.skip}
    </p>
  ) : null;

  if (single === null) {
    return (
      <div role="group" aria-label={`Settings of ${subject}`} className="flex flex-col gap-1">
        <div className="flex h-6 items-center text-[13px] font-semibold text-[var(--toolbar-icon)]">{subject}</div>
        {picker}
        {skip}
      </div>
    );
  }

  const { settings } = single;
  const lines = stateLines(settings);
  const twoSets = settings.sets.length > 1;
  const showEdit = canEditMaterial(settings, pick);
  return (
    <div role="group" aria-label={`Settings of ${single.name}`} aria-describedby={summaryId} className="flex flex-col gap-1">
      <p id={summaryId} className="sr-only">
        {blockSummary(settings)}
      </p>
      <div className="flex h-6 items-center gap-2 text-[13px] font-semibold text-[var(--toolbar-icon)]">
        {single.kind === "unknown" ? null : <OperationGlyph kind={single.kind} />}
        <span className="min-w-0 truncate" title={single.name}>
          {single.name}
        </span>
      </div>
      {picker}
      {skip}
      {settings.state === "no_settings" ? (
        <p className="text-xs text-[var(--panel-muted-fg)]">No settings</p>
      ) : null}
      {settings.state === "damaged" ? (
        <p className="flex items-start gap-1 text-xs text-[var(--toolbar-icon)]">
          <TriangleAlert size={16} aria-hidden className="shrink-0" />
          Damaged settings
        </p>
      ) : null}
      {lines.map((line) => {
        const Glyph = GLYPHS[line.glyph];
        return (
          <p key={line.text} className="flex items-start gap-1 text-xs text-[var(--toolbar-icon)]">
            <Glyph size={16} aria-hidden className="shrink-0" />
            {line.text}
          </p>
        );
      })}
      <div ref={fieldsRef} className="flex flex-col gap-1">
        {settings.sets.map((set) => (
          <div key={set.role} className="flex flex-col">
            {twoSets ? <SetHeading set={set} /> : null}
            <SettingsNumberFields
              numbers={set.run}
              nameOf={(field) => fieldName(field, set.role, twoSets)}
              lineOf={(field) => materialLine(set, field)}
              descriptionOf={(field) => materialDescription(set, field)}
              onCommit={(field, number) => api.edit(single.id, set.role, field, number)}
              onReturnFocus={onReturnFocus}
            />
          </div>
        ))}
      </div>
      {settings.reset || showEdit ? (
        <div className="flex h-7 items-center justify-between gap-2">
          {settings.reset && material ? (
            <button
              type="button"
              aria-label={`Reset to material ${material.name}`}
              onClick={() => {
                if (api.reset(single.id)) setStatus(resetStatus(material.name));
                // The button leaves the tree: the focus moves to the first number field once it
                // shows the material's number (a focused field keeps the text it had).
                window.requestAnimationFrame(() => fieldsRef.current?.querySelector<HTMLInputElement>("input")?.focus());
              }}
              className={BUTTON}
            >
              <RotateCcw size={16} aria-hidden />
              Reset to material
            </button>
          ) : (
            <span />
          )}
          {showEdit && material ? (
            <button
              type="button"
              aria-label={`Edit material ${material.name}`}
              onClick={() => onEditMaterial(material.id)}
              className={BUTTON}
            >
              Edit material
            </button>
          ) : null}
        </div>
      ) : null}
      <div role="status" aria-atomic="true" className="sr-only">
        {status}
      </div>
    </div>
  );
}

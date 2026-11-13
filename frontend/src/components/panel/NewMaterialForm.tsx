import { useEffect, useRef, useState, type ReactNode } from "react";

import type { MaterialsApi } from "@/hooks/useMaterials";
import { NAME_MAX, newMaterialRefusal, typedNumber } from "@/lib/materialRows";

const INPUT = "h-full min-w-0 flex-1 bg-transparent px-0.5 text-right text-[13px] tabular-nums text-[var(--toolbar-icon)] outline-none";

/** A plain field with its word inside the left edge and its unit inside the right (the app's
 * field look); the border marks a refused value. */
function FormField({ label, unit, invalid, width, children }: { label: string; unit?: string; invalid: boolean; width: number; children: ReactNode }) {
  return (
    <div
      className={`flex h-7 items-center rounded bg-white focus-within:ring-2 focus-within:ring-[var(--editor-accent)] ${
        invalid ? "border-2 border-[var(--field-invalid)]" : "border border-[var(--lane-line)]"
      }`}
      style={{ width }}
    >
      <span aria-hidden className="shrink-0 pl-1.5 text-[11px] text-[var(--panel-muted-fg)]">
        {label}
      </span>
      {children}
      {unit ? (
        <span aria-hidden className="shrink-0 pr-1.5 text-[11px] text-[var(--panel-muted-fg)]">
          {unit}
        </span>
      ) : null}
    </div>
  );
}

/**
 * The New material form (`specs/0112` `ux-notes.md` 7.2, the "Add and Edit form" look of `0045`):
 * Name, Thickness, Cancel and Add. Enter in a field presses Add, Escape or Cancel drops the
 * draft, the first invalid field takes focus.
 */
export function NewMaterialForm({ api, defaultName, onDone }: { api: MaterialsApi; defaultName: string; onDone: (id: string | null) => void }) {
  const { draft, setDraft } = api;
  const name = draft.name ?? defaultName;
  const [refusal, setRefusal] = useState<{ field: "name" | "thickness"; text: string } | null>(null);
  const nameField = useRef<HTMLInputElement>(null);
  const thicknessField = useRef<HTMLInputElement>(null);
  useEffect(() => {
    nameField.current?.focus();
    nameField.current?.select();
  }, []);

  const add = () => {
    const refused = newMaterialRefusal(name, draft.thickness);
    if (refused) {
      setRefusal(refused);
      (refused.field === "name" ? nameField : thicknessField).current?.focus();
      return;
    }
    const result = api.add(name.trim(), typedNumber(draft.thickness) ?? Number.NaN);
    if (result.ok) {
      onDone(result.id);
      return;
    }
    const field = /thickness/i.test(result.text) ? "thickness" : "name";
    setRefusal({ field, text: result.text });
    (field === "name" ? nameField : thicknessField).current?.focus();
  };
  const cancel = () => {
    api.cancelForm();
    onDone(null);
  };
  const keys = (event: React.KeyboardEvent) => {
    if (event.key === "Enter") {
      event.preventDefault();
      add();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancel();
    }
  };
  return (
    <div
      role="group"
      aria-label="New material"
      className="flex w-[244px] flex-col gap-1 rounded-[5px] p-2"
      style={{ border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
    >
      <span className="text-[11px] font-semibold text-[var(--panel-muted-fg)]">New material</span>
      <FormField label="Name" invalid={refusal?.field === "name"} width={228}>
        <input
          ref={nameField}
          aria-label="Name"
          aria-invalid={refusal?.field === "name" || undefined}
          value={name}
          maxLength={NAME_MAX}
          onChange={(event) => setDraft({ ...draft, name: event.target.value })}
          onKeyDown={keys}
          className={INPUT}
        />
      </FormField>
      <FormField label="Thickness" unit="mm" invalid={refusal?.field === "thickness"} width={112}>
        <input
          ref={thicknessField}
          aria-label="Thickness, millimetres"
          aria-invalid={refusal?.field === "thickness" || undefined}
          inputMode="decimal"
          value={draft.thickness}
          onChange={(event) => setDraft({ ...draft, thickness: event.target.value })}
          onKeyDown={keys}
          className={INPUT}
        />
      </FormField>
      {refusal ? (
        <p role="alert" className="text-xs text-[var(--field-invalid)]">
          {refusal.text}
        </p>
      ) : null}
      <div className="flex gap-1">
        <button
          type="button"
          onClick={cancel}
          className="h-7 w-[112px] rounded border border-[var(--lane-line)] text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={add}
          className="h-7 w-[112px] rounded bg-[var(--toolbar-icon-active-bg)] text-xs text-white focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Add
        </button>
      </div>
    </div>
  );
}

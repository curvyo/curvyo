import { useEffect, useRef, useState, type ReactNode } from "react";

import type { MaterialsApi } from "@/hooks/useMaterials";
import { NAME_MAX, newMaterialRefusal, typedNumber } from "@/lib/materialRows";
import { Button } from "@/components/ui/button";
import { Plus, X } from "lucide-react";
import { FIELD_FRAME_LABEL, fieldFrameClass, SECTION_HEADING } from "@/components/ui/looks";
import { AlertLine } from "@/components/ui/notice";

const INPUT = "h-full min-w-0 flex-1 bg-transparent px-0.5 text-right text-sm tabular-nums text-[var(--toolbar-icon)] outline-none";

/** A plain field with its word inside the left edge and its unit inside the right (the app's
 * field look); the border marks a refused value. */
function FormField({ label, unit, invalid, width, children }: { label: string; unit?: string; invalid: boolean; width: number; children: ReactNode }) {
  return (
    <div
      className={fieldFrameClass(invalid)}
      style={{ width }}
    >
      <span aria-hidden className={`${FIELD_FRAME_LABEL} pl-1.5`}>
        {label}
      </span>
      {children}
      {unit ? (
        <span aria-hidden className={`${FIELD_FRAME_LABEL} pr-1.5`}>
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
      <span className={SECTION_HEADING}>New material</span>
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
        <AlertLine>
          {refusal.text}
        </AlertLine>
      ) : null}
      <div className="flex gap-1">
        <Button icon={X}
          onClick={cancel} className="min-w-[112px]"
        >
          Cancel
        </Button>
        <Button variant="primary" icon={Plus}
          onClick={add} className="min-w-[112px]"
        >
          Add
        </Button>
      </div>
    </div>
  );
}

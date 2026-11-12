import { Trash2 } from "lucide-react";

import { EntryField } from "@/components/panel/EntryField";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { SettingsNumberFields } from "@/components/panel/SettingsNumberFields";
import { ARM_TRASH, ARM_ZONE } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import { fieldsHeight, setArmKey, setArmText, usedByText } from "@/lib/materialRows";
import type { SelectedMaterial } from "@/lib/materialsView";
import { SET_WORDS, fieldName } from "@/lib/operationSettingsRows";
import { CHIP_IS_MESSAGE, outcomeOf } from "@/lib/settingsFieldText";

interface MaterialFieldsProps {
  material: SelectedMaterial;
  api: MaterialsApi;
  /** The armed set trash's target id, `null` when none. */
  armedSet: string | null;
  /** A press on a set's trash: arms it, or removes the set when armed. */
  onSetTrash: (kind: SelectedMaterial["sets"][number]["kind"]) => void;
  onReturnFocus: () => void;
}

/**
 * The selected material's fields (`specs/0028` `ux-notes.md` 10.3): Thickness, the use line, and
 * per set (Cut, Score, Engrave) a sub-heading with its armed trash, the number fields and the
 * source line. Each commit is one undo step of the open project; the session writes the record.
 * The block has a fixed height, so the list's window is exact.
 */
export function MaterialFields({ material, api, armedSet, onSetTrash, onReturnFocus }: MaterialFieldsProps) {
  const used = usedByText(material.uses);
  return (
    <div style={{ height: fieldsHeight(material.sets.length, material.uses) }} className="flex flex-col gap-1 overflow-hidden pl-2 pt-1">
      <div className="h-7">
        <EntryField
          label="Thickness, millimetres"
          shown={String(material.thicknessMm)}
          mixed={false}
          suffix="mm"
          width={118}
          commitOnBlur
          inputMode="decimal"
          onSubmit={(text) => outcomeOf(api.setThickness(material.id, Number(text.trim().replace(",", "."))))}
          messages={CHIP_IS_MESSAGE}
          onReturnFocus={onReturnFocus}
        />
      </div>
      {used ? <p className="h-4 text-[11px] leading-4 text-[var(--panel-muted-fg)]">{used}</p> : null}
      {material.sets.map((set) => {
        const word = SET_WORDS[set.kind];
        const key = setArmKey(material.id, set.kind);
        const armed = armedSet === key;
        return (
          <div key={set.kind} role="group" aria-label={`${word} set of ${material.name}`} {...{ [ARM_ZONE]: key }} className="flex flex-col">
            <div className="flex h-6 items-center gap-1 text-xs font-semibold text-[var(--toolbar-icon)]">
              <OperationGlyph kind={set.kind} size={14} />
              <span className="flex-1">{word}</span>
              <button
                type="button"
                {...{ [ARM_TRASH]: key }}
                aria-label={armed ? setArmText(word, material.name) : `Delete ${word} set of ${material.name}`}
                title={armed ? "Press again to delete" : "Delete"}
                onClick={() => onSetTrash(set.kind)}
                className={
                  armed
                    ? "flex h-[22px] w-[22px] items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
                    : "flex h-6 w-6 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                }
              >
                <Trash2 size={16} aria-hidden />
              </button>
            </div>
            <SettingsNumberFields
              numbers={{ powerPct: set.numbers.powerPct, speedMmMin: set.numbers.speedMmMin, passes: set.numbers.passes }}
              nameOf={(field) => `${fieldName(field, "main", false)}, ${word}`}
              onCommit={(field, value) => api.setNumber(material.id, set.kind, field, value)}
              onReturnFocus={onReturnFocus}
            />
            <p className="h-4 truncate text-[11px] leading-4 text-[var(--panel-muted-fg)]" title={set.source}>
              {set.source}
            </p>
          </div>
        );
      })}
    </div>
  );
}

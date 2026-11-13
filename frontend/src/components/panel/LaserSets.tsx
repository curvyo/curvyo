import { ChevronDown, ChevronRight, Plus, Trash2, Zap } from "lucide-react";

import { EntryField } from "@/components/panel/EntryField";
import { OperationGlyph } from "@/components/panel/OperationGlyphs";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import {
  KIND_ORDER,
  KIND_WORD,
  SET_FIELD_UNIT,
  SET_FIELD_WORD,
  laserCountText,
  numberText,
  setArmKey,
  setArmText,
  setFieldValue,
  setFields,
  setHeadingName,
  type SetField,
} from "@/lib/materialRows";
import type { Laser, LaserBlock, MaterialSet } from "@/lib/materialsView";
import type { SetKind } from "@/lib/operationSettingsView";
import { CHIP_IS_MESSAGE, outcomeOf, stepText, typedNumber } from "@/lib/settingsFieldText";

const OUTLINE_BUTTON =
  "flex h-7 w-[228px] items-center gap-1 rounded border border-[var(--lane-line)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface LaserSetsProps {
  materialId: string;
  block: LaserBlock;
  api: MaterialsApi;
  arm: DeleteArm;
  onReturnFocus: () => void;
}

/** One set: the kind heading with its source and armed trash, and its fields (UX 7.4 item 3). */
function SetFields({ materialId, laser, set, api, arm, onReturnFocus }: LaserSetsProps & { laser: Laser; set: MaterialSet }) {
  const key = setArmKey(materialId, laser.id, set.kind);
  const armed = arm.armed === key;
  const field = (name: SetField, width: number) => {
    const value = setFieldValue(set, name);
    if (value === null) return null;
    return (
      <EntryField
        key={name}
        label={`${SET_FIELD_WORD[name]}, ${KIND_WORD[set.kind]} set of ${laser.name}`}
        prefix={SET_FIELD_WORD[name]}
        shown={numberText(value)}
        mixed={false}
        suffix={SET_FIELD_UNIT[name]}
        smallSuffix={name === "speed"}
        width={width}
        gutter={name === "speed" ? 52 : undefined}
        commitOnBlur
        inputMode="decimal"
        stepText={stepText}
        onSubmit={(text) => outcomeOf(api.editSet(materialId, laser.id, set.kind, name, typedNumber(text)))}
        messages={CHIP_IS_MESSAGE}
        onReturnFocus={onReturnFocus}
      />
    );
  };
  const fields = setFields(set.kind);
  return (
    <div role="group" aria-label={setHeadingName(set, laser.name)} {...{ [ARM_ZONE]: key }} className="flex flex-col gap-1">
      <div className="flex h-6 items-center gap-1 text-xs text-[var(--toolbar-icon)]">
        <OperationGlyph kind={set.kind} />
        <span className="font-semibold">{KIND_WORD[set.kind]}</span>
        <span className="min-w-0 flex-1 truncate text-right text-[11px] text-[var(--panel-muted-fg)]">{set.source}</span>
        <button
          type="button"
          {...{ [ARM_TRASH]: key }}
          aria-label={armed ? setArmText(set.kind, laser.name) : `Delete the ${KIND_WORD[set.kind]} set of ${laser.name}`}
          title={armed ? "Press again to delete" : "Delete"}
          onClick={() =>
            arm.press(key, setArmText(set.kind, laser.name), () => {
              api.deleteSet(materialId, laser.id, set.kind);
              return `${KIND_WORD[set.kind]} set of ${laser.name} deleted.`;
            })
          }
          className={
            armed
              ? "flex h-[22px] w-[22px] shrink-0 items-center justify-center rounded-[5px] bg-[var(--destructive)] text-white"
              : "flex h-6 w-6 shrink-0 items-center justify-center rounded text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          }
        >
          <Trash2 size={16} aria-hidden />
        </button>
      </div>
      <div className="flex gap-1">
        {field(fields[0], 112)}
        {field(fields[1], 112)}
      </div>
      {fields.slice(2).map((name) => field(name, 228))}
    </div>
  );
}

/** One laser block of a material (UX 7.4 items 2 to 4): the header folds it; inside, one set per
 * kind in the order Cut, Score, Engrave, then one "+ <kind> set" button per missing kind. */
export function LaserSets(props: LaserSetsProps) {
  const { materialId, block, api } = props;
  const Chevron = block.open ? ChevronDown : ChevronRight;
  const sets = KIND_ORDER.flatMap((kind) => block.sets.filter((set) => set.kind === kind));
  return (
    <div role="group" aria-label={`${block.laser.name} sets`} className="flex flex-col gap-1">
      <button
        type="button"
        aria-expanded={block.open}
        onClick={() => api.toggleLaser(block.laser.id)}
        className="flex h-7 w-[228px] items-center gap-1 rounded text-left text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <Chevron size={12} aria-hidden className="shrink-0" />
        <Zap size={12} aria-hidden className="shrink-0" />
        <span className="min-w-0 flex-1 truncate font-semibold">{block.laser.name}</span>
        {block.thisPage ? (
          <span
            className="shrink-0 rounded-md px-1.5 text-[11px] text-[var(--toolbar-icon)]"
            style={{ border: "1px solid color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
          >
            This page
          </span>
        ) : block.open ? null : (
          <span className="shrink-0 text-[11px] text-[var(--panel-muted-fg)]">{laserCountText(block)}</span>
        )}
      </button>
      {block.open ? (
        <div className="flex flex-col gap-2">
          {sets.map((set) => (
            <SetFields key={set.kind} {...props} laser={block.laser} set={set} />
          ))}
          {block.missing.map((kind: SetKind) => (
            <button
              key={kind}
              type="button"
              onClick={(event) => {
                // The pressed button leaves: the focus goes to the new set's Power (UX 7.4 item 4).
                const box = event.currentTarget.parentElement;
                api.addSet(materialId, block.laser.id, kind);
                const power = `Power, ${KIND_WORD[kind]} set of ${block.laser.name}`;
                window.requestAnimationFrame(() => {
                  [...(box?.querySelectorAll<HTMLInputElement>("input[aria-label]") ?? [])].find((input) => input.getAttribute("aria-label") === power)?.focus();
                });
              }}
              className={OUTLINE_BUTTON}
            >
              <Plus size={16} aria-hidden />
              <OperationGlyph kind={kind} size={14} />
              {KIND_WORD[kind]} set
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

import { Fragment, useId } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { PresetStrip } from "@/components/panel/PresetStrip";
import { IMAGE_FIELD_LOOK, imageFieldStep, imageFieldText } from "@/lib/imageSettingsRows";
import type { DitherSettings, ImageField, ImageFieldName } from "@/lib/operationSettingsView";
import { CHIP_IS_MESSAGE, outcomeOf, typedNumber } from "@/lib/settingsFieldText";

/** The strip's three fixed rows: a break after the third cell and after Grey (`specs/0123`
 * `ux-notes.md` 2.5; `specs/0135` UX notes §3.10, §10: the screens Halftone and Lines are the
 * third row). */
const STRIP_BREAK = ["floyd_steinberg", "grey"];

/** Floyd-Steinberg and the two screens are shortened; every cell's accessible name is the full
 * name, and a cell whose word is shortened has the full name as its tooltip (the others none). */
function cellWord(label: string): string {
  if (label === "Floyd-Steinberg") return "Floyd";
  if (label === "Halftone dots") return "Halftone";
  if (label === "Line screen") return "Lines";
  return label;
}

interface FieldProps {
  field: ImageField;
  /** The chip text of a refusal, empty when taken. */
  onEdit: (name: ImageFieldName, value: number) => string;
  onReturnFocus: () => void;
}

/** One number field of the Image settings or of a Dither block. */
export function ImageNumberField({ field, onEdit, onReturnFocus }: FieldProps) {
  const look = IMAGE_FIELD_LOOK[field.field];
  return (
    <div data-image-field={field.field}>
      <EntryField
        label={look.name}
        prefix={field.label}
        shown={imageFieldText(field.value)}
        mixed={false}
        suffix={field.unit}
        width={look.width}
        gutter={field.unit === "" ? 8 : undefined}
        commitOnBlur
        inputMode="decimal"
        onSubmit={(text) => outcomeOf(onEdit(field.field, typedNumber(text)))}
        messages={CHIP_IS_MESSAGE}
        stepText={imageFieldStep(field.field)}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

interface StripProps {
  dither: DitherSettings;
  /** The id of a visible label the strip is named by; without it the strip is named "Dither". */
  labelledBy?: string;
  onPress: (mode: string) => void;
}

/**
 * The nine-mode Dither strip in three fixed rows (`specs/0135` UX notes §3.10, §10): always
 * visible, nothing opens, one Tab stop, arrows move and select, each selection one step. A damaged
 * dither presses no cell and says "Damaged" under the strip.
 */
export function DitherModeStrip({ dither, labelledBy, onPress }: StripProps) {
  const damagedId = useId();
  return (
    <div data-image-dither={dither.dither ?? "damaged"} className="flex flex-col gap-1">
      <PresetStrip
        label="Dither"
        labelledBy={labelledBy}
        describedBy={dither.dither === null ? damagedId : undefined}
        width="w-full"
        tooltipSide="top"
        fixedRows={{ breakAfter: STRIP_BREAK }}
        cells={dither.modes.map((mode) => ({
          id: mode.value,
          content: cellWord(mode.label),
          label: mode.label,
          tooltip: cellWord(mode.label) === mode.label ? "" : mode.label,
        }))}
        value={dither.dither}
        onPress={onPress}
        // The focus stays on the strip: the arrows keep working (UX 2.5).
        onReturnFocus={() => undefined}
      />
      {dither.dither === null ? (
        <p id={damagedId} className="text-xs text-[var(--field-invalid)]">
          Damaged
        </p>
      ) : null}
    </div>
  );
}

/** The mode's own fields of a Dither block: Threshold, or Cell size or Line pitch and Angle;
 * nothing for the other modes. */
export function DitherFields({
  dither,
  onEdit,
  onReturnFocus,
}: {
  dither: DitherSettings;
  onEdit: FieldProps["onEdit"];
  onReturnFocus: () => void;
}) {
  const fields = dither.modeField ? [dither.modeField] : dither.screenFields;
  if (fields.length === 0) return null;
  return (
    <div data-dither-fields="" className="flex flex-wrap gap-1">
      {fields.map((one) => (
        <Fragment key={one.field}>
          <ImageNumberField field={one} onEdit={onEdit} onReturnFocus={onReturnFocus} />
        </Fragment>
      ))}
    </div>
  );
}

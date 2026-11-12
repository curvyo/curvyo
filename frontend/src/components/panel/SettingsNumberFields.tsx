import { EntryField } from "@/components/panel/EntryField";
import { CHIP_IS_MESSAGE, outcomeOf, stepText, typedNumber } from "@/lib/settingsFieldText";
import { FIELDS, numberText } from "@/lib/operationSettingsRows";
import type { SettingsField } from "@/lib/operationSettingsView";

interface SettingsNumberFieldsProps {
  /** The numbers the fields show. */
  numbers: { powerPct: number | null; speedMmMin: number | null; passes: number | null };
  /** The accessible name of a field ("Power, percent"). */
  nameOf: (field: SettingsField) => string;
  /** The material's number beside a typed field ("Material 60"), `null` for none. */
  lineOf?: (field: SettingsField) => string | null;
  /** The description for assistive technology ("Material 60 %"). */
  descriptionOf?: (field: SettingsField) => string | null;
  /** Types a number; the chip text, empty when taken. */
  onCommit: (field: SettingsField, value: number) => string;
  onReturnFocus: () => void;
  /** The first field takes the panel's first focus. */
  firstFocus?: boolean;
  /** A field beside Passes (the Interval of a hatch, `specs/0111` ux-notes 6.2). */
  beside?: React.ReactNode;
  /** The width of each field: 118 px below the list, 112 px in the unfolded row (`specs/0111`
   * `ux-notes.md` 6.2, two fields and a gap in its 228 px column). */
  fieldWidth?: number;
}

function valueOf(numbers: SettingsNumberFieldsProps["numbers"], field: SettingsField): number | null {
  return field === "power" ? numbers.powerPct : field === "speed" ? numbers.speedMmMin : numbers.passes;
}

/**
 * The Power, Speed and Passes fields of one set (`specs/0028` `ux-notes.md` 8.3): Power and Speed
 * in a row (118 px each, 4 px apart), Passes under them. A field with a material number beside it
 * shows "Material 60" under itself. Enter or leaving commits; the session answers with the chip.
 */
export function SettingsNumberFields(props: SettingsNumberFieldsProps) {
  const { numbers, nameOf, lineOf, descriptionOf, onCommit, onReturnFocus, firstFocus, beside, fieldWidth = 118 } = props;
  const cell = (field: SettingsField, first: boolean) => {
    const entry = FIELDS.find((candidate) => candidate.field === field);
    const line = lineOf?.(field) ?? null;
    const description = descriptionOf?.(field) ?? null;
    return (
      <div key={field} className="flex flex-col" style={{ width: fieldWidth }}>
        <EntryField
          label={description ? `${nameOf(field)}, ${description}` : nameOf(field)}
          shown={numberText(valueOf(numbers, field))}
          mixed={false}
          suffix={entry?.unit}
          smallSuffix={field === "speed"}
          width={fieldWidth}
          gutter={field === "speed" ? 52 : undefined}
          commitOnBlur
          inputMode="decimal"
          onSubmit={(text) => outcomeOf(onCommit(field, typedNumber(text)))}
          messages={CHIP_IS_MESSAGE}
          stepText={stepText}
          onReturnFocus={onReturnFocus}
          firstFocus={first && firstFocus}
        />
        {line ? <span className="h-4 text-[11px] leading-4 tabular-nums text-[var(--panel-muted-fg)]">{line}</span> : null}
      </div>
    );
  };
  return (
    <div className="flex flex-col gap-1">
      <div className="flex gap-1">
        {cell("power", true)}
        {cell("speed", false)}
      </div>
      <div className="flex flex-wrap gap-1">
        {cell("passes", false)}
        {beside}
      </div>
    </div>
  );
}

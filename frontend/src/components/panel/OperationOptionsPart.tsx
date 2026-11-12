import { ArrowRightLeft, Hash } from "lucide-react";
import { Toggle } from "radix-ui";

import { EntryField } from "@/components/panel/EntryField";
import { Tooltip } from "@/components/ui/tooltip";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import { lowerPart, optionStep, toggleNames, type OptionField, type OptionToggle } from "@/lib/operationOptionRows";
import type { OperationRow } from "@/lib/operationsView";
import { CHIP_IS_MESSAGE, outcomeOf, typedNumber } from "@/lib/settingsFieldText";

const GLYPHS = { crosshatch: Hash, bidirectional: ArrowRightLeft } as const;

interface OptionFieldViewProps {
  field: OptionField & { shown: string };
  row: OperationRow;
  tools: OperationToolsApi;
  onReturnFocus: () => void;
}

/** One typed field of the unfolded row (Interval, Scan angle, Overscanning, Z offset): the value
 * goes to the session, which answers with the chip text (`ux-notes.md` 6.4, 10). */
export function OptionFieldView({ field, row, tools, onReturnFocus }: OptionFieldViewProps) {
  return (
    <EntryField
      label={field.name}
      prefix={field.label}
      shown={field.shown}
      mixed={false}
      suffix={field.unit}
      width={field.width}
      gutter={field.unit.length > 1 ? 32 : 20}
      commitOnBlur
      inputMode="decimal"
      onSubmit={(text) => outcomeOf(tools.edit(row.id, field.key, typedNumber(text)))}
      messages={CHIP_IS_MESSAGE}
      stepText={optionStep(field)}
      onReturnFocus={onReturnFocus}
    />
  );
}

function OptionToggleView({ toggle, row, tools }: { toggle: OptionToggle; row: OperationRow; tools: OperationToolsApi }) {
  const Glyph = GLYPHS[toggle.option];
  const name = toggleNames(toggle.option);
  return (
    <Tooltip side="bottom" content={name}>
      <Toggle.Root
        pressed={toggle.pressed}
        onPressedChange={(on) => tools.edit(row.id, toggle.option, on ? 1 : 0)}
        aria-label={name}
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
      >
        <Glyph aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
      </Toggle.Root>
    </Tooltip>
  );
}

/**
 * The lower part of an unfolded row (`specs/0111-laser-operation-tools` `ux-notes.md` 6.3): under
 * a hairline, the Crosshatch and Bidirectional fill toggles with Scan angle, then Overscanning and
 * Z offset for Engrave and Engrave + Outline; Z offset alone for the other kinds. Z offset is
 * absent, not disabled, without a motorised Z axis; with nothing to show the hairline goes too.
 */
export function OperationOptionsPart({ row, tools, onReturnFocus }: { row: OperationRow; tools: OperationToolsApi; onReturnFocus: () => void }) {
  if (row.settings === null) return null;
  const part = lowerPart(row.kind, row.settings);
  if (part === null) return null;
  const first = part.toggles.length > 0 ? part.fields[0] : undefined;
  const rest = first ? part.fields.slice(1) : part.fields;
  const view = (field: (typeof part.fields)[number]) => (
    <OptionFieldView key={field.key} field={field} row={row} tools={tools} onReturnFocus={onReturnFocus} />
  );
  return (
    <div className="flex flex-col gap-1">
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      {part.toggles.length > 0 ? (
        <div className="flex items-center gap-1">
          {part.toggles.map((toggle) => (
            <OptionToggleView key={toggle.option} toggle={toggle} row={row} tools={tools} />
          ))}
          {first ? view(first) : null}
        </div>
      ) : null}
      {rest.length > 0 ? <div className="flex flex-wrap gap-1">{rest.map(view)}</div> : null}
    </div>
  );
}

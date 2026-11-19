import { Contrast } from "lucide-react";
import { Toggle } from "radix-ui";
import { useId } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { PresetStrip } from "@/components/panel/PresetStrip";
import { Tooltip } from "@/components/ui/tooltip";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import { IMAGE_FIELD_LOOK, imageFieldStep, imageFieldText } from "@/lib/imageSettingsRows";
import type { ImageField, ImageSettings } from "@/lib/operationSettingsView";
import type { OperationRow } from "@/lib/operationsView";
import { CHIP_IS_MESSAGE, outcomeOf, typedNumber } from "@/lib/settingsFieldText";

/** The strip's two fixed rows: a break after the third cell (`ux-notes.md` 2.5). */
const STRIP_BREAK = ["floyd_steinberg"];

/** Only Floyd-Steinberg is shortened; every cell's accessible name is the full name, and a cell
 * whose word is shortened has the full name as its tooltip (the others none, UX notes 2.5). */
function cellWord(label: string): string {
  return label === "Floyd-Steinberg" ? "Floyd" : label;
}

interface GroupProps {
  row: OperationRow;
  image: ImageSettings;
  tools: OperationToolsApi;
  onReturnFocus: () => void;
}

function ImageFieldView({ field, row, tools, onReturnFocus }: Omit<GroupProps, "image"> & { field: ImageField }) {
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
        onSubmit={(text) => outcomeOf(tools.edit(row.id, field.field, typedNumber(text)))}
        messages={CHIP_IS_MESSAGE}
        stepText={imageFieldStep(field.field)}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

/**
 * The Image settings group of an Image row (`specs/0123-raster-engraving` `ux-notes.md` 2.4, 2.5):
 * under a hairline, no heading, the Dither as an always-visible choice strip (arrows move and
 * select, each selection one step, the canvas redraw is the preview), Brightness and Contrast,
 * Gamma and the Invert toggle, then the mode's own field (Threshold or Lowest power) last, so a
 * change of mode never moves the strip. The session shows it on every Image row and holds every
 * range.
 */
export function ImageSettingsGroup({ row, image, tools, onReturnFocus }: GroupProps) {
  const damagedId = useId();
  const [brightness, contrast, gamma] = image.fields;
  const field = (one: ImageField | undefined) =>
    one ? <ImageFieldView field={one} row={row} tools={tools} onReturnFocus={onReturnFocus} /> : null;
  return (
    <div role="group" aria-label={`Image settings of ${row.name}`} data-image-settings="" className="flex flex-col gap-1">
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <div data-image-dither={image.dither ?? "damaged"} className="flex flex-col gap-1">
        <PresetStrip
          label="Dither"
          describedBy={image.dither === null ? damagedId : undefined}
          width="w-full"
          tooltipSide="top"
          fixedRows={{ breakAfter: STRIP_BREAK }}
          cells={image.modes.map((mode) => ({
            id: mode.value,
            content: cellWord(mode.label),
            label: mode.label,
            tooltip: cellWord(mode.label) === mode.label ? "" : mode.label,
          }))}
          value={image.dither}
          onPress={(mode) => tools.setDither(row.id, mode)}
          // The focus stays on the strip: the arrows keep working (UX 2.5).
          onReturnFocus={() => undefined}
        />
        {image.dither === null ? (
          <p id={damagedId} className="text-xs text-[var(--field-invalid)]">
            Damaged
          </p>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-1">
        {field(brightness)}
        {field(contrast)}
      </div>
      <div className="flex items-center gap-1">
        {field(gamma)}
        <Tooltip side="bottom" content="Invert">
          <Toggle.Root
            pressed={image.invert}
            onPressedChange={(on) => tools.edit(row.id, "invert", on ? 1 : 0)}
            aria-label="Invert"
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
          >
            <Contrast aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
          </Toggle.Root>
        </Tooltip>
      </div>
      {image.modeField ? <div className="flex flex-wrap gap-1">{field(image.modeField)}</div> : null}
    </div>
  );
}

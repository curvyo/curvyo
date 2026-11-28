import { Contrast } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";
import { Toggle } from "radix-ui";
import { useId } from "react";

import { DitherFields, DitherModeStrip, ImageNumberField } from "@/components/panel/DitherStrip";
import { Tooltip } from "@/components/ui/tooltip";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import type { ImageField, ImageSettings } from "@/lib/operationSettingsView";
import type { OperationRow } from "@/lib/operationsView";

interface GroupProps {
  row: OperationRow;
  image: ImageSettings;
  tools: OperationToolsApi;
  onReturnFocus: () => void;
}

/**
 * The Image settings group of an Image row (`specs/0123-raster-engraving` `ux-notes.md` 2.4;
 * `specs/0135` UX notes §10, P30): under a hairline, no heading, Brightness and Contrast, Gamma
 * and the Invert toggle, Lowest power while an item burns Grey, then, only while the operation
 * holds a gradient fill, its own Dither block labelled "Dither for gradient fills" (the strip and
 * the mode's fields last). A picture burns its own dither (its Adjustments group); the session
 * holds every range.
 */
export function ImageSettingsGroup({ row, image, tools, onReturnFocus }: GroupProps) {
  const labelId = useId();
  const [brightness, contrast, gamma] = image.fields;
  const onEdit = (name: string, value: number) => tools.edit(row.id, name, value);
  const field = (one: ImageField | undefined | null) =>
    one ? <ImageNumberField field={one} onEdit={onEdit} onReturnFocus={onReturnFocus} /> : null;
  return (
    <div role="group" aria-label={`Image settings of ${row.name}`} data-image-settings="" className="flex flex-col gap-1">
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <div className="flex flex-wrap gap-1">
        {field(brightness)}
        {field(contrast)}
      </div>
      <div className="flex items-center gap-1">
        {field(gamma)}
        <Tooltip side="left" content="Invert">
          <Toggle.Root
            pressed={image.invert}
            onPressedChange={(on) => tools.edit(row.id, "invert", on ? 1 : 0)}
            aria-label="Invert"
            className="flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
          >
            <Contrast {...GLYPH} aria-hidden />
          </Toggle.Root>
        </Tooltip>
      </div>
      {image.lowestPower ? <div className="flex flex-wrap gap-1">{field(image.lowestPower)}</div> : null}
      {image.dither ? (
        <div data-gradient-dither="" className="flex flex-col gap-1">
          <p id={labelId} className="h-4 text-xs leading-4 text-[var(--panel-muted-fg)]">
            Dither for gradient fills
          </p>
          <DitherModeStrip
            dither={image.dither}
            labelledBy={labelId}
            onPress={(mode) => tools.setDither(row.id, mode)}
          />
          <DitherFields dither={image.dither} onEdit={onEdit} onReturnFocus={onReturnFocus} />
        </div>
      ) : null}
    </div>
  );
}

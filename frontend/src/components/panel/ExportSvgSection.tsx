import { useId, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { ProjectApi } from "@/hooks/useProject";
import {
  loadSvgExportOptions,
  saveSvgExportOptions,
  type SvgExportOptions,
} from "@/lib/svgExport";
import { Button } from "@/components/ui/button";
import { File, FileOutput, MousePointerClick, Spline, Type, type LucideIcon } from "lucide-react";
import { ToggleButton } from "@/components/ui/toggle-button";


/** The labels of the four export switches, in tab order (`0024` `ux-notes.md`, exact). */
const EXPORT_SWITCHES: [keyof SvgExportOptions, string, LucideIcon][] = [
  ["textAsPaths", "Text as paths", Type],
  ["strokesAsPaths", "Strokes as paths", Spline],
  ["selectionOnly", "Selection only", MousePointerClick],
  ["activePageOnly", "Active page only", File],
];

/** The page's storage, or `null` where reading it throws. */
function pageStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/**
 * Export SVG… and its four switches (`specs/0024-svg-import-export` criteria 4 and 147, `ux-notes.md`
 * "SVG export: UX notes"): its own section below the button block (divider, heading "SVG export"), so the
 * unsaved-changes check never replaces it; the button, then the switches in one column (the two-column grid does not fit "Strokes as
 * paths" in 118 px at 12 px). The switches are remembered per user and change nothing in the
 * drawing.
 */
export function ExportSvgSection({
  project,
  onReturnFocus,
}: {
  project: ProjectApi;
  /** After a mouse press the keyboard goes back to the canvas. */
  onReturnFocus: () => void;
}) {
  const [options, setOptions] = useState<SvgExportOptions>(() => loadSvgExportOptions(pageStorage()));
  const toggle = (name: keyof SvgExportOptions) => {
    const next = { ...options, [name]: !options[name] };
    setOptions(next);
    saveSvgExportOptions(pageStorage(), next);
  };
  const headingId = useId();
  return (
    <section aria-labelledby={headingId} className="flex flex-col">
      <div
        aria-hidden
        className="my-4 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <h3 id={headingId} className="mb-2 text-xs font-semibold text-[var(--toolbar-icon)]">
        SVG export
      </h3>
      <Tooltip side="left" content="Export SVG">
        <Button icon={FileOutput}
          data-export-svg
          onClick={(event) => project.exportSvg(options, event.detail > 0)} className="min-w-[118px]"
        >
          Export SVG…
        </Button>
      </Tooltip>
      <div role="group" aria-label="Export options" className="mt-2 flex flex-col gap-1">
        {EXPORT_SWITCHES.map(([name, label, icon]) => (
          <ToggleButton
            key={name}
            icon={icon}
            label={label}
            pressed={options[name]}
            onPressedChange={() => toggle(name)}
            onReturnFocus={onReturnFocus}
            extra={{ "data-export-option": name }}
          />
        ))}
      </div>
    </section>
  );
}


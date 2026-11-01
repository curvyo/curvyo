import { useId, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { ProjectApi } from "@/hooks/useProject";
import {
  loadSvgExportOptions,
  saveSvgExportOptions,
  type SvgExportOptions,
} from "@/lib/svgExport";

const BUTTON =
  "h-7 shrink-0 rounded-[5px] border text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9";
const OUTLINE =
  "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]";

/** The labels of the four export switches, in tab order (`0024` `ux-notes.md`, exact). */
const EXPORT_SWITCHES: [keyof SvgExportOptions, string][] = [
  ["textAsPaths", "Text as paths"],
  ["strokesAsPaths", "Strokes as paths"],
  ["selectionOnly", "Selection only"],
  ["activePageOnly", "Active page only"],
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
        <button
          type="button"
          data-export-svg
          onClick={(event) => project.exportSvg(options, event.detail > 0)}
          className={`${BUTTON} ${OUTLINE} w-[118px]`}
        >
          Export SVG…
        </button>
      </Tooltip>
      <div role="group" aria-label="Export options" className="mt-2 flex flex-col">
        {EXPORT_SWITCHES.map(([name, label]) => (
          <button
            key={name}
            type="button"
            role="switch"
            aria-checked={options[name]}
            data-export-option={name}
            onClick={(event) => {
              toggle(name);
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className="flex h-6 items-center gap-2 rounded-[5px] text-left text-xs text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
          >
            <span
              aria-hidden
              className={`relative inline-block h-[14px] w-6 shrink-0 rounded-full border ${
                options[name]
                  ? "border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)]"
                  : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent"
              }`}
            >
              <span
                className={`absolute top-[2px] h-2 w-2 rounded-full transition-[left] motion-reduce:transition-none ${
                  options[name]
                    ? "left-[12px] bg-[var(--toolbar-icon-active-fg)]"
                    : "left-[2px] bg-[var(--toolbar-icon)]"
                }`}
              />
            </span>
            {label}
          </button>
        ))}
      </div>
    </section>
  );
}


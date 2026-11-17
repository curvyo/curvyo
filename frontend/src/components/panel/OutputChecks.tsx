import { OctagonX, TriangleAlert } from "lucide-react";

import { checkPrefix } from "@/lib/outputText";
import type { Check } from "@/lib/outputView";

/**
 * The checks of the Output tab (`specs/0029-laser-job-preview-and-output` UX notes 6.3): blocks,
 * then warns, each a `listitem` with its glyph (block `--destructive`, warn ink), an sr-only
 * prefix and the text. The name or count that leads the text is an underlined text button: its
 * way to where the cause is fixed. Every button is a Tab stop in list order.
 */
export function OutputChecks({ checks, onPress }: { checks: Check[]; onPress: (check: Check) => void }) {
  return (
    <ul role="list" aria-label="Checks" className="flex flex-col gap-2">
      {checks.map((check, index) => (
        <li key={index} className="flex items-start gap-1.5 py-0.5 text-xs leading-4 text-[var(--toolbar-icon)]">
          {check.severity === "block" ? (
            <OctagonX
              size={16}
              strokeWidth={1.5}
              absoluteStrokeWidth
              aria-hidden
              className="mt-px shrink-0 text-[var(--destructive)]"
            />
          ) : (
            <TriangleAlert size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden className="mt-px shrink-0" />
          )}
          <span className="min-w-0">
            <span className="sr-only">{checkPrefix(check)}</span>
            {check.before}
            {check.button !== "" && check.target.kind !== "none" ? (
              <button
                type="button"
                onClick={() => onPress(check)}
                className="rounded-sm underline outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {check.button}
              </button>
            ) : (
              check.button
            )}
            {check.after}
          </span>
        </li>
      ))}
    </ul>
  );
}

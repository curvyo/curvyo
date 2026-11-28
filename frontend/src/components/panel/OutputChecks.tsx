import { OctagonX, TriangleAlert } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";
import { ALERT_GLYPH_CLASS, AlertLine } from "@/components/ui/notice";

import { checkPrefix } from "@/lib/outputText";
import type { Check } from "@/lib/outputView";

/**
 * The checks of the Output tab (`specs/0029-laser-job-preview-and-output` UX notes 6.3): blocks,
 * then warns, each a `listitem` on the alert line look (block `--field-invalid`, warn ink), an sr-only
 * prefix and the text. The name or count that leads the text is an underlined text button: its
 * way to where the cause is fixed. Every button is a Tab stop in list order.
 */
export function OutputChecks({ checks, onPress }: { checks: Check[]; onPress: (check: Check) => void }) {
  return (
    <ul role="list" aria-label="Checks" className="flex flex-col gap-2">
      {checks.map((check, index) => (
        <li key={index} className="py-0.5">
          {/* The alert line look (`docs/ui-elements.md` §16): a block in its colour with
           * OctagonX, a warning in ink with TriangleAlert; a standing list, so no live role. */}
          <AlertLine
            announce={false}
            icon={
              check.severity === "block" ? (
                <OctagonX {...SMALL_GLYPH} aria-hidden className={ALERT_GLYPH_CLASS} />
              ) : (
                <TriangleAlert {...SMALL_GLYPH} aria-hidden className={ALERT_GLYPH_CLASS} />
              )
            }
            tone={check.severity === "block" ? "alert" : "warn"}
          >
            <span className="sr-only">{checkPrefix(check)}</span>
            {check.before}
            {check.button !== "" && check.target.kind !== "none" ? (
              <button
                type="button"
                onClick={() => onPress(check)}
                className="rounded-[5px] underline outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {check.button}
              </button>
            ) : (
              check.button
            )}
            {check.after}
          </AlertLine>
        </li>
      ))}
    </ul>
  );
}

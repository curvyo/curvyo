import type { NumbersLine } from "@/lib/numbersLine";
import { numbersParts } from "@/lib/outputText";

/** One numbers line, 14 px: a typed number is followed by the word "typed" in ink (`specs/0029-laser-job-preview-and-output` UX 6.1). */
export function NumbersText({ line, prefix = "" }: { line: NumbersLine; prefix?: string }) {
  return (
    <span className="block h-[14px] truncate tabular-nums">
      {prefix}
      {numbersParts(line.text, line.typed).map((part, at) => (
        <span key={at}>
          {at > 0 ? ", " : ""}
          {part.text}
          {part.typed ? <span className="text-[var(--toolbar-icon)]"> typed</span> : null}
        </span>
      ))}
    </span>
  );
}

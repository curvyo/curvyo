import type { PointerEvent } from "react";

import type { NumbersLine } from "@/lib/numbersLine";
import { numbersParts, numbersTitle } from "@/lib/outputText";

/** One numbers line, 14 px: a typed number is followed by the word "typed" in ink (`specs/0029-laser-job-preview-and-output` UX 6.1).
 * A line cut by the ellipsis shows the whole line as its title (`specs/0123` UX review U7). */
export function NumbersText({ line, prefix = "" }: { line: NumbersLine; prefix?: string }) {
  const titleWhenCut = (event: PointerEvent<HTMLSpanElement>) => {
    const span = event.currentTarget;
    if (span.scrollWidth > span.clientWidth) span.title = numbersTitle(line.text, line.typed, prefix);
    else span.removeAttribute("title");
  };
  return (
    <span className="block h-[14px] truncate tabular-nums" onPointerEnter={titleWhenCut}>
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

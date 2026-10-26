// One value of the Info mode's fact list (`specs/0083-font-info-and-zip` criterion 3, UX notes
// §2): selectable text, clamped to 3 lines with a Show all / Show less text button when it is
// really cut; the expanded state resets with the subject face (the parent keys it by face).
import { useLayoutEffect, useRef, useState } from "react";

import { NOT_STATED } from "@/lib/fontInfoText";

export function InfoValue({ text, clamp = true, part = false }: { text: string | null; clamp?: boolean; part?: boolean }) {
  // A part of a value (the Licence row's name, text and URL) is a `div` inside the row's `dd`.
  const Tag = part ? "div" : "dd";
  const ref = useRef<HTMLParagraphElement | null>(null);
  const [open, setOpen] = useState(false);
  const [cut, setCut] = useState(false);

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element || open) {
      return undefined;
    }
    const measure = () => setCut(element.scrollHeight > element.clientHeight + 1);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [text, open]);

  if (text === null || text === "") {
    return <Tag className="m-0 text-sm text-[var(--panel-muted-fg)]">{NOT_STATED}</Tag>;
  }
  const clamped = clamp && !open;
  return (
    <Tag className="m-0 min-w-0">
      <p
        ref={ref}
        className="m-0 text-sm whitespace-pre-line text-[var(--toolbar-icon)] select-text [overflow-wrap:anywhere]"
        style={clamped ? { display: "-webkit-box", WebkitLineClamp: 3, WebkitBoxOrient: "vertical", overflow: "hidden" } : undefined}
      >
        {text}
      </p>
      {clamp && (cut || open) ? (
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className="h-7 rounded-[5px] text-sm text-[var(--editor-accent)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          {open ? "Show less" : "Show all"}
        </button>
      ) : null}
    </Tag>
  );
}

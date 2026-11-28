import { useEffect, useRef } from "react";

import { CONFIRM_GUARD_MS, type ListEntry, restartAt, restartQuestion } from "@/lib/jobList";
import { formatCount } from "@/lib/connectorText";
import { ignoredKey } from "@/lib/sendArm";
import { Button } from "@/components/ui/button";
import { RotateCcw, X } from "lucide-react";
import { GLYPH } from "@/components/ui/looks";

const BUTTON =
  "flex h-7 shrink-0 items-center justify-center rounded-[5px] border text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 pointer-coarse:h-9";

/**
 * The confirm of Restart from line in the interrupted row's place (`specs/0118` ux-notes W69, W70;
 * `specs/0132` criterion 12 b; Frage 391 C): the question, Cancel focused (no default button) and
 * the action with the line, in the accent look of the Inline confirm's start variant. The action
 * ignores presses for 400 ms and a held key; Escape, Cancel and a press outside close it.
 */
export function RestartConfirm({
  row,
  onCancel,
  onConfirm,
}: {
  row: ListEntry;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const box = useRef<HTMLDivElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);
  const opened = useRef(Number.POSITIVE_INFINITY);
  const question = restartQuestion(row);
  const action = `Restart from line ${formatCount(restartAt(row))}`;

  const latestCancel = useRef(onCancel);
  useEffect(() => {
    latestCancel.current = onCancel;
  }, [onCancel]);
  // Once, when it opens: the focus on Cancel, the guard's start, a press outside closes.
  useEffect(() => {
    opened.current = performance.now();
    cancel.current?.focus();
    const outside = (event: PointerEvent) => {
      if (box.current !== null && !box.current.contains(event.target as Node)) latestCancel.current();
    };
    document.addEventListener("pointerdown", outside, true);
    return () => document.removeEventListener("pointerdown", outside, true);
  }, []);

  return (
    <div
      ref={box}
      role="group"
      aria-label={question}
      className="flex flex-col gap-2 py-1 pl-1"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onCancel();
        }
      }}
    >
      <p className="text-xs leading-4 text-[var(--toolbar-icon)]">{question}</p>
      <div className="flex gap-2">
        <Button icon={X}
          ref={cancel} className="min-w-[76px]"
          onClick={onCancel}
        >
          Cancel
        </Button>
        <button
          type="button"
          className={`${BUTTON} min-w-[160px] gap-1 px-3 border-[var(--armed-accent)] bg-white text-[var(--armed-accent)] hover:bg-[var(--armed-accent)] hover:text-white focus-visible:bg-[var(--armed-accent)] focus-visible:text-white active:bg-[var(--armed-accent)] active:text-white`}
          onKeyDown={(event) => {
            if (ignoredKey(event.key, event.repeat)) event.preventDefault();
          }}
          onClick={() => {
            if (performance.now() - opened.current >= CONFIRM_GUARD_MS) onConfirm();
          }}
        >
          <RotateCcw {...GLYPH} aria-hidden />
          {action}
        </button>
      </div>
    </div>
  );
}

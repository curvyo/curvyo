import { useEffect, useId, useRef, useState } from "react";

/** The longest name, characters (`ux-notes.md` section 3.2; Rust's `MAX_ENTRY_NAME_CHARS`). */
const MAX_NAME_CHARS = 120;

interface WorkspaceNameFieldProps {
  /** The accessible name ("Name of the new document", "Rename Lid"). */
  label: string;
  /** The text the field starts with, selected. */
  initial: string;
  /** The refusal of the last commit, shown under the field; empty for none. */
  error: string;
  /** Enter or Tab: the session's refusal, or empty once done. */
  onCommit: (text: string) => Promise<string>;
  /** Escape: nothing is written. */
  onCancel: () => void;
}

/**
 * The inline name field of the workspace tree (`specs/0094-workspace` criteria 13, 14, 18;
 * `ux-notes.md` section 3.2): the text selected, Enter or Tab commits, Escape cancels and writes
 * nothing; a refused name keeps the field with the reason in `--field-invalid` below it.
 */
export function WorkspaceNameField({ label, initial, error, onCommit, onCancel }: WorkspaceNameFieldProps) {
  const input = useRef<HTMLInputElement>(null);
  const [text, setText] = useState(initial);
  const [shownError, setShownError] = useState(error);
  const busy = useRef(false);
  const messageId = useId();
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, []);
  // A new refusal from the session replaces what the field shows (derived during render).
  const [seenError, setSeenError] = useState(error);
  if (seenError !== error) {
    setSeenError(error);
    setShownError(error);
  }
  const commit = async () => {
    if (busy.current) {
      return;
    }
    busy.current = true;
    const refusal = await onCommit(text);
    busy.current = false;
    if (refusal !== "") {
      setShownError(refusal);
      input.current?.select();
    }
  };
  return (
    <div className="min-w-0 flex-1">
      <input
        ref={input}
        type="text"
        value={text}
        maxLength={MAX_NAME_CHARS}
        aria-label={label}
        aria-invalid={shownError !== ""}
        aria-describedby={shownError !== "" ? messageId : undefined}
        onChange={(event) => {
          setText(event.target.value);
          setShownError("");
        }}
        onClick={(event) => event.stopPropagation()}
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Enter" || event.key === "Tab") {
            event.preventDefault();
            void commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            onCancel();
          }
        }}
        className={`h-6 w-full rounded-[4px] border bg-[var(--background)] px-1.5 text-sm text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
          shownError !== "" ? "border-[var(--field-invalid)]" : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]"
        }`}
      />
      {shownError !== "" ? (
        <p id={messageId} role="alert" className="mt-0.5 text-xs text-[var(--field-invalid)]">
          {shownError}
        </p>
      ) : null}
    </div>
  );
}

// The masked access token field (`specs/0157-library-git-token` UX notes 2, criterion 8): an
// UNCONTROLLED password input, so no `value` attribute ever holds the typed text (React syncs the
// attribute of a controlled input). Its text is read through the handle at Add or Save and cleared
// by the caller; it is never in React state, a store, `localStorage` or a `title`.
import { forwardRef, useId, useImperativeHandle, useRef } from "react";

import { tokenWords } from "@/lib/tokenWords";
import { AlertLine } from "@/components/ui/notice";

export interface TokenFieldHandle {
  /** The typed text; the only way out of the field. */
  read: () => string;
  /** Empties the field (success, Cancel, kind switch, unmount). */
  clear: () => void;
  focus: () => void;
}

interface TokenFieldProps {
  /** The label above the field. */
  label: string;
  /** Read only while a check runs. */
  busy: boolean;
  /** The refusal under the statement, or empty. */
  refusal: string;
  /** Enter in the field. */
  onEnter: () => void;
  /** Whether the field takes the focus when drawn. */
  autoFocus?: boolean;
}

export const TokenField = forwardRef<TokenFieldHandle, TokenFieldProps>(function TokenField(
  { label, busy, refusal, onEnter, autoFocus },
  handle,
) {
  const input = useRef<HTMLInputElement>(null);
  const ids = { field: useId(), statement: useId(), refusal: useId() };
  useImperativeHandle(handle, () => ({
    read: () => input.current?.value ?? "",
    clear: () => {
      if (input.current) {
        input.current.value = "";
      }
    },
    focus: () => input.current?.focus(),
  }));
  return (
    <>
      <label htmlFor={ids.field}>{label}</label>
      <input
        ref={input}
        id={ids.field}
        type="password"
        autoComplete="off"
        autoCapitalize="off"
        autoCorrect="off"
        spellCheck={false}
        data-1p-ignore
        data-lpignore="true"
        data-token-field
        readOnly={busy}
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus={autoFocus}
        aria-invalid={refusal ? true : undefined}
        aria-describedby={refusal ? `${ids.statement} ${ids.refusal}` : ids.statement}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            onEnter();
          }
        }}
      />
      <p id={ids.statement} className="library-line" style={{ margin: 0 }}>
        {tokenWords().statement}
      </p>
      {refusal ? (
        <AlertLine id={ids.refusal}>
          {refusal}
        </AlertLine>
      ) : null}
    </>
  );
});

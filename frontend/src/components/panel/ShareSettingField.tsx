// A labelled text field of the Share tab that commits on Enter or blur (`specs/0092` UX notes
// §2.1).

import { useId, useState } from "react";

import { FIELD } from "@/components/panel/devicesUi";

export /** A labelled text field that commits on Enter or when it loses focus, Escape restores. */
function SettingField({
  label,
  value,
  placeholder,
  onCommit,
  readOnly = false,
  mono = false,
  autoFocus = false,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onCommit: (text: string) => string;
  readOnly?: boolean;
  mono?: boolean;
  autoFocus?: boolean;
}) {
  const [text, setText] = useState(value);
  const [message, setMessage] = useState("");
  const id = useId();
  const commit = () => {
    if (text === value) {
      setMessage("");
      return;
    }
    const refusal = onCommit(text);
    setMessage(refusal);
  };
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-xs text-[var(--toolbar-icon)]">
        {label}
      </label>
      <input
        id={id}
        className={`${FIELD} ${mono ? "font-mono text-xs" : ""}`}
        value={text}
        placeholder={placeholder}
        readOnly={readOnly}
        autoFocus={autoFocus}
        aria-invalid={message !== ""}
        aria-describedby={message ? `${id}-message` : undefined}
        onChange={(event) => setText(event.target.value)}
        onBlur={() => !readOnly && commit()}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            setText(value);
            setMessage("");
          }
        }}
      />
      {message ? (
        <p id={`${id}-message`} className="text-xs text-[var(--field-invalid)]">
          {message}
        </p>
      ) : null}
    </div>
  );
}

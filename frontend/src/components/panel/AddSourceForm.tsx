// The inline Add source form (`specs/0138-template-library` UX notes section 3): Kind, Address,
// the line that says what is sent, Cancel and Add. Add checks the address with one request, which
// the line under the field says; nothing else is sent. The refusals are Rust's words.
import { GitBranch, Globe, LoaderCircle } from "lucide-react";
import { useId, useRef, useState } from "react";

import { addOnlineSource, cancelOnlineAdd, removeOnlineSource } from "@/lib/onlineApi";
import { library_hosting_guide, library_online_add_sent_text } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { openGuide } from "@/lib/libraryBrowserStore";

/** "The repository must be public.", worded in Rust (read when drawn: the wasm is loaded then). */
const publicOnly = () => (JSON.parse(library_hosting_guide()) as { private: string }).private;

type Kind = "git" | "iconify";

/** The kind last used, for the session (UX notes 3: not in a file). */
let lastKind: Kind = "git";

interface AddSourceFormProps {
  /** The source was added: its key. */
  onAdded: (key: string) => void;
  /** Cancel or Escape. */
  onCancel: () => void;
}

export function AddSourceForm({ onAdded, onCancel }: AddSourceFormProps) {
  const [kind, setKind] = useState<Kind>(lastKind);
  const [address, setAddress] = useState("");
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState("");
  const ticket = useRef(0);
  const ids = { address: useId(), sent: useId(), refusal: useId() };

  const add = () => {
    if (busy) {
      return;
    }
    const mine = ++ticket.current;
    setBusy(true);
    setRefusal("");
    void addOnlineSource(kind, address).then(
      (answer) => {
        if (mine !== ticket.current) {
          // Cancelled while the check ran: the host still added it, so it leaves the list again.
          if (answer.kind === "added") {
            void removeOnlineSource(answer.key).catch(() => undefined);
          }
          return;
        }
        setBusy(false);
        if (answer.kind === "added") {
          lastKind = kind;
          onAdded(answer.key);
        } else if (answer.kind === "refused") {
          setRefusal(answer.text);
        }
      },
      () => {
        if (mine === ticket.current) {
          setBusy(false);
          setRefusal("The server did not answer.");
        }
      },
    );
  };
  // Cancel stops the check: the host discards it when it ends and the answer is dropped here; a source
  // the host had already added by then is removed.
  const cancel = () => {
    ticket.current += 1;
    if (busy) {
      void cancelOnlineAdd().catch(() => undefined);
    }
    onCancel();
  };

  return (
    <div
      role="group"
      aria-label="Add source"
      className="library-source-form"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          cancel();
        }
      }}
    >
      <div
        role="radiogroup"
        aria-label="Kind"
        className="library-kind"
        onKeyDown={(event) => {
          // The Kind switch: one Tab stop, the arrows choose (design system "Add page group").
          if (busy || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) {
            return;
          }
          event.preventDefault();
          const next: Kind = kind === "git" ? "iconify" : "git";
          setKind(next);
          event.currentTarget.querySelector<HTMLElement>(`[data-kind="${next}"]`)?.focus();
        }}
      >
        {(["git", "iconify"] as const).map((k) => (
          <button
            key={k}
            type="button"
            role="radio"
            data-kind={k}
            aria-checked={kind === k}
            tabIndex={kind === k ? 0 : -1}
            disabled={busy}
            onClick={() => setKind(k)}
          >
            {k === "git" ? <GitBranch size={14} aria-hidden="true" /> : <Globe size={14} aria-hidden="true" />}
            {k === "git" ? "Git catalogue" : "Iconify"}
          </button>
        ))}
      </div>
      <label htmlFor={ids.address}>Address</label>
      <input
        id={ids.address}
        type="url"
        inputMode="url"
        aria-label="Address"
        value={address}
        readOnly={busy}
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-invalid={refusal ? true : undefined}
        aria-describedby={refusal ? ids.refusal : undefined}
        onChange={(event) => setAddress(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            add();
          }
        }}
      />
      {refusal ? (
        <p id={ids.refusal} role="alert" className="library-refusal">
          {refusal}
        </p>
      ) : null}
      <p id={ids.sent} className="library-line" style={{ margin: 0 }}>
        {library_online_add_sent_text(kind === "iconify")}
      </p>
      {kind === "git" ? (
        <>
          {/* Frage 376 B: no token; the statement of what works (`specs/0152` criterion 33). */}
          <p className="library-line" style={{ margin: 0 }}>
            {publicOnly()}
          </p>
          {/* Where the catalogue format is explained (criterion 32, UX U48). */}
          <button type="button" className="library-guide-link" onClick={() => openGuide()}>
            Host your own library…
          </button>
        </>
      ) : null}
      <div className="library-source-buttons">
        <button type="button" onClick={cancel}>
          Cancel
        </button>
        <button type="button" aria-busy={busy || undefined} aria-describedby={ids.sent} onClick={add}>
          {busy ? <LoaderCircle size={16} aria-hidden="true" className="motion-safe:animate-spin" /> : "Add"}
        </button>
      </div>
    </div>
  );
}

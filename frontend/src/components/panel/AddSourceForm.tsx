// The inline Add source form (`specs/0138-template-library` UX notes section 3): Kind, Address,
// the line that says what is sent, for a Git catalogue the optional Access token (`specs/0156`),
// Cancel and Add. Add checks the address with one request, which the line under the field says;
// nothing else is sent. The refusals are Rust's words, each under the field it is about.
import { GitBranch, Globe, LoaderCircle } from "lucide-react";
import { useId, useRef, useState } from "react";

import { TokenField, type TokenFieldHandle } from "@/components/panel/TokenField";
import { addOnlineSource, cancelOnlineAdd, removeOnlineSource, type RefusalField } from "@/lib/onlineApi";
import { library_online_add_sent_text } from "@/wasm-bindings/curvyo_editor_wasm.js";
import { openGuide } from "@/lib/libraryBrowserStore";
import { tokenWords } from "@/lib/tokenWords";

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
  const [refusal, setRefusal] = useState<{ field: RefusalField; text: string } | null>(null);
  const ticket = useRef(0);
  const token = useRef<TokenFieldHandle>(null);
  const addressField = useRef<HTMLInputElement>(null);
  const ids = { address: useId(), sent: useId(), refusal: useId() };
  const addressRefusal = refusal?.field === "address" ? refusal.text : "";
  const tokenRefusal = refusal?.field === "token" ? refusal.text : "";

  const add = () => {
    if (busy) {
      return;
    }
    const mine = ++ticket.current;
    setBusy(true);
    setRefusal(null);
    // The typed token goes to the host in this one call; an empty field is no token.
    const typed = kind === "git" ? (token.current?.read() ?? "") : "";
    void addOnlineSource(kind, address, typed.trim() === "" ? null : typed).then(
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
          token.current?.clear();
          onAdded(answer.key);
        } else if (answer.kind === "refused") {
          // The token field keeps its text for a retry within the form; focus goes to the field the
          // refusal is about (UX notes T1).
          setRefusal({ field: answer.field, text: answer.text });
          window.setTimeout(() => (answer.field === "token" ? token.current : addressField.current)?.focus());
        }
      },
      () => {
        if (mine === ticket.current) {
          setBusy(false);
          setRefusal({ field: "address", text: "The server did not answer." });
        }
      },
    );
  };
  // Cancel stops the check: the host discards it when it ends and the answer is dropped here; a source
  // the host had already added by then is removed.
  const cancel = () => {
    ticket.current += 1;
    token.current?.clear();
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
          token.current?.clear();
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
            onClick={() => {
              if (k !== kind) {
                // Switching the kind drops the token text (UX notes T2).
                token.current?.clear();
              }
              setKind(k);
            }}
          >
            {k === "git" ? <GitBranch size={14} aria-hidden="true" /> : <Globe size={14} aria-hidden="true" />}
            {k === "git" ? "Git catalogue" : "Iconify"}
          </button>
        ))}
      </div>
      <label htmlFor={ids.address}>Address</label>
      <input
        ref={addressField}
        id={ids.address}
        type="url"
        inputMode="url"
        aria-label="Address"
        value={address}
        readOnly={busy}
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-invalid={addressRefusal ? true : undefined}
        aria-describedby={addressRefusal ? ids.refusal : undefined}
        onChange={(event) => setAddress(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            add();
          }
        }}
      />
      {addressRefusal ? (
        <p id={ids.refusal} role="alert" className="library-refusal">
          {addressRefusal}
        </p>
      ) : null}
      <p id={ids.sent} className="library-line" style={{ margin: 0 }}>
        {library_online_add_sent_text(kind === "iconify")}
      </p>
      {kind === "git" ? (
        <>
          <TokenField
            ref={token}
            label={tokenWords().add_label}
            busy={busy}
            refusal={tokenRefusal}
            onEnter={add}
          />
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

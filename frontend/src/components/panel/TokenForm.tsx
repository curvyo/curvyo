// The inline access token form of one Git catalogue (`specs/0157-library-git-token` UX notes 4):
// "Change token…" or "Add token…" opens it in the place of the Add source form. The field is empty
// on opening (the old token is never shown), Save checks the new token with one request and keeps it
// only on success, Remove token is an armed action. The refusals are Rust's words.
import { Lock, Save, Trash2, X } from "lucide-react";
import { SMALL_GLYPH } from "@/components/ui/looks";
import { useEffect, useRef, useState } from "react";

import { TokenField, type TokenFieldHandle } from "@/components/panel/TokenField";
import { forgetOnlineToken, setOnlineToken, type TokenCleanup } from "@/lib/onlineApi";
import { tokenWords } from "@/lib/tokenWords";
import { Button } from "@/components/ui/button";

/** How long an armed press waits for the second one (`specs/0137` UX notes 4.4). */
const ARM_MS = 4000;

interface TokenFormProps {
  /** The source key (`online:<address>`). */
  source: string;
  name: string;
  /** Whether the source has a token now. */
  hasToken: boolean;
  /** The token was saved or removed; the key store line when a delete failed. */
  onDone: (cleanup: TokenCleanup | null) => void;
  /** Cancel or Escape. */
  onCancel: () => void;
}

export function TokenForm({ source, name, hasToken, onDone, onCancel }: TokenFormProps) {
  const words = tokenWords();
  const token = useRef<TokenFieldHandle>(null);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState("");
  const [armed, setArmed] = useState(false);
  const ticket = useRef(0);

  useEffect(() => {
    if (!armed) {
      return;
    }
    const timer = window.setTimeout(() => setArmed(false), ARM_MS);
    return () => window.clearTimeout(timer);
  }, [armed]);

  const save = () => {
    if (busy) {
      return;
    }
    const mine = ++ticket.current;
    setBusy(true);
    setRefusal("");
    void setOnlineToken(source, token.current?.read() ?? "").then(
      (answer) => {
        if (mine !== ticket.current) {
          return;
        }
        setBusy(false);
        if (answer.kind === "done") {
          token.current?.clear();
          onDone(null);
        } else {
          // Every refusal of this form is shown under the token field (it has no Address).
          setRefusal(answer.text);
          window.setTimeout(() => token.current?.focus());
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
  const cancel = () => {
    ticket.current += 1;
    token.current?.clear();
    onCancel();
  };
  const remove = () => {
    if (!armed) {
      setArmed(true);
      return;
    }
    ticket.current += 1;
    token.current?.clear();
    void forgetOnlineToken(source).then(
      (cleanup) => onDone(cleanup),
      () => onDone(null),
    );
  };

  return (
    <div
      role="group"
      aria-label={`Access token of ${name}`}
      className="library-source-form"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          cancel();
        }
      }}
    >
      <p className="library-line" style={{ margin: 0, fontWeight: 600 }}>
        {name}
        {hasToken ? <Lock {...SMALL_GLYPH} aria-hidden="true" className="library-lock" /> : null}
      </p>
      <TokenField
        ref={token}
        label={hasToken ? words.new_label : words.label}
        busy={busy}
        refusal={refusal}
        onEnter={save}
        autoFocus
      />
      {hasToken ? (
        <Button
          variant="destructive"
          icon={Trash2}
          className={armed ? "self-start bg-[var(--destructive)] text-white" : "self-start"}
          onClick={remove}
        >
          {armed ? words.remove_armed : words.remove}
        </Button>
      ) : null}
      <div className="grid grid-cols-2 gap-2">
        <Button icon={X} onClick={cancel}>
          Cancel
        </Button>
        <Button variant="primary" icon={Save} busy={busy} onClick={save}>
          Save
        </Button>
      </div>
    </div>
  );
}

// The Removed or left section an admin sees (`specs/0097-membership-lifecycle` ux-notes 8): a
// collapsed disclosure, one row per former participant with Add back, and the add-back dialog
// (a document's History choice, Full history preselected; a workspace's one sentence).

import { ChevronDown, ChevronRight, UserPlus } from "lucide-react";
import { type KeyboardEvent, useId, useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import type { FormerParticipant } from "@/hooks/useCollaboration";
import type { MembershipApi } from "@/hooks/useMembership";
import { usePendingLine } from "@/hooks/usePendingLine";
import { focusParticipants } from "@/lib/shareFocus";
import {
  membership_add_back_name,
  membership_add_back_texts,
  membership_former_disclosure,
  membership_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
const OPTION =
  "flex flex-col gap-0.5 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] px-2 py-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-checked:border-[var(--toolbar-icon-active-bg)]";

/** The History choice with its consequence under each item (ux-notes 8.2): arrows choose, Enter
 * does not confirm. */
function HistoryChoice({ full, onChange, lines }: { full: boolean; onChange: (full: boolean) => void; lines: [string, string] }) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const keys = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(event.key)) {
      event.preventDefault();
      onChange(!full);
      refs.current[full ? 1 : 0]?.focus();
    } else if (event.key === "Enter") {
      event.preventDefault();
    }
  };
  const options: [boolean, string, string][] = [
    [true, membership_word("full-history"), lines[0]],
    [false, membership_word("from-now-on"), lines[1]],
  ];
  return (
    <div role="radiogroup" aria-label={membership_word("history")} className="flex flex-col gap-1">
      {options.map(([value, label, line], index) => (
        <button
          key={label}
          ref={(element) => {
            refs.current[index] = element;
          }}
          type="button"
          role="radio"
          aria-checked={full === value}
          // The dialog opens on the selected radio (ux-notes 9).
          autoFocus={full === value}
          tabIndex={full === value ? 0 : -1}
          className={OPTION}
          onKeyDown={keys}
          onClick={() => onChange(value)}
        >
          <span className="text-sm text-[var(--toolbar-icon)]">{label}</span>
          <span className={MUTED}>{line}</span>
        </button>
      ))}
    </div>
  );
}

interface AddBackTexts {
  text?: string;
  full?: string;
  fromNow?: string;
  common?: string;
}

/** The dialog's texts from Rust's JSON; none when it cannot be read. */
function addBackTexts(json: string): AddBackTexts {
  try {
    return JSON.parse(json) as AddBackTexts;
  } catch {
    return {};
  }
}

function FormerRow({
  former,
  workspace,
  documents,
  membership,
  onDone,
}: {
  former: FormerParticipant;
  workspace: boolean;
  documents: number;
  membership: MembershipApi;
  /** After the add back: where the focus goes (ux-notes 8.3). */
  onDone: (member: string) => void;
}) {
  const [asking, setAsking] = useState(false);
  const [full, setFull] = useState(true);
  const [refusal, setRefusal] = useState("");
  const opener = useRef<HTMLButtonElement>(null);
  const label = former.label;
  const texts = addBackTexts(membership_add_back_texts(label, workspace, membership.name, documents));
  // "Adding back..." until the server holds the add, then the row leaves (ADR 0041 decision 11).
  const busyLine = usePendingLine(former.adding ? "adding-back" : null);
  const busy = busyLine !== null;
  const close = () => {
    setAsking(false);
    setRefusal("");
    window.setTimeout(() => opener.current?.focus(), 0);
  };
  const shown = label || membership_word("unnamed");
  return (
    <li data-former={former.member} className="flex flex-col" aria-busy={busy}>
      <div className="flex h-14 items-center gap-2 pointer-coarse:h-[60px]">
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm text-[var(--toolbar-icon)]" title={shown}>
            {shown}
          </p>
          <p className="truncate font-mono text-xs text-[var(--panel-muted-fg)]">{former.fingerprint}</p>
          <p className={MUTED}>{busyLine ?? membership_word(former.left ? "left" : "removed")}</p>
        </div>
        <button
          disabled={busy}
          ref={opener}
          type="button"
          data-add-back
          aria-label={membership_add_back_name(label)}
          title={membership_word("add-back")}
          className={`${BUTTON} ${OUTLINE} w-7 border-transparent pointer-coarse:w-9`}
          onClick={() => setAsking(true)}
        >
          <UserPlus {...GLYPH} />
        </button>
      </div>
      {asking ? (
        <div
          role="group"
          aria-label={membership_add_back_name(label)}
          className="flex flex-col gap-2 pb-2"
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              event.stopPropagation();
              close();
            }
          }}
        >
          {workspace ? (
            <p className="text-xs text-[var(--toolbar-icon)]">{texts.text}</p>
          ) : (
            <>
              <HistoryChoice full={full} onChange={setFull} lines={[texts.full ?? "", texts.fromNow ?? ""]} />
              <p className="text-xs text-[var(--toolbar-icon)]">{texts.common}</p>
            </>
          )}
          <div className="flex flex-wrap gap-2">
            <button autoFocus={workspace} type="button" className={`${BUTTON} ${OUTLINE} w-[76px]`} onClick={close}>
              Cancel
            </button>
            <button
              type="button"
              className={`${BUTTON} ${APPLY} w-[90px]`}
              onClick={() => {
                const answer = membership.addBack(former.member, workspace || full, label);
                setRefusal(answer);
                if (answer === "") {
                  setAsking(false);
                  onDone(former.member);
                }
              }}
            >
              {membership_word("add-back")}
            </button>
          </div>
          {refusal ? (
            <p role="alert" className={ALERT}>
              {refusal}
            </p>
          ) : null}
        </div>
      ) : null}
    </li>
  );
}

/** Removed or left (criteria 17 to 19, 23, 24), for admins; absent when nobody is out. */
export function FormerList({
  former,
  workspace,
  documents,
  membership,
}: {
  former: FormerParticipant[];
  workspace: boolean;
  documents: number;
  membership: MembershipApi;
}) {
  const [open, setOpen] = useState(false);
  const list = useRef<HTMLUListElement>(null);
  const listId = useId();
  if (former.length === 0) {
    return null;
  }
  const Chevron = open ? ChevronDown : ChevronRight;
  const done = (member: string) => {
    // The next row's Add back, else the previous one's, else the Participants list (8.3).
    const rows = [...(list.current?.querySelectorAll<HTMLElement>("[data-former]") ?? [])];
    const index = rows.findIndex((r) => r.dataset.former === member);
    const next = rows[index + 1] ?? rows[index - 1];
    const button = next?.querySelector<HTMLElement>("[data-add-back]");
    if (button) {
      window.setTimeout(() => button.focus(), 0);
    } else {
      focusParticipants();
    }
  };
  return (
    <div className="flex flex-col gap-1">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={listId}
        className={`${BUTTON} ${OUTLINE} w-full justify-start border-transparent px-1`}
        onClick={() => setOpen(!open)}
      >
        <Chevron {...GLYPH} />
        {membership_former_disclosure(former.length)}
      </button>
      {open ? (
        <ul ref={list} id={listId} aria-label={membership_word("former-list")} className="flex flex-col">
          {former.map((row) => (
            <FormerRow
              key={row.member}
              former={row}
              workspace={workspace}
              documents={documents}
              membership={membership}
              onDone={done}
            />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

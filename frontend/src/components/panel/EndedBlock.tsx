// The Removed block of a file or workspace this device was removed from or left
// (`specs/0097-membership-lifecycle` ux-notes 7): its heading, Check whether I was added back
// with what it contacts, and the add-back prompt with Sync again and Keep as it is.

import { UserCheck } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import type { MembershipApi } from "@/hooks/useMembership";
import { focusParticipants, takeEndedHeadingFocus } from "@/lib/shareFocus";
import {
  membership_added_back_prompt,
  membership_cannot_reach,
  membership_contacts_line,
  membership_ended_heading,
  membership_keep_consequence,
  membership_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

export function EndedBlock({ membership }: { membership: MembershipApi }) {
  const ended = membership.ended;
  const heading = useRef<HTMLParagraphElement>(null);
  const prompt = useRef<HTMLDivElement>(null);
  const [phase, setPhase] = useState<"" | "syncing" | "leaving">("");
  const [failure, setFailure] = useState("");
  const check = ended?.check ?? "";
  useEffect(() => {
    if (takeEndedHeadingFocus()) {
      heading.current?.focus();
    }
  }, []);
  // The prompt takes the focus as a group, never a button, so Enter cannot sync by reflex.
  useEffect(() => {
    if (check === "addedBack") {
      prompt.current?.focus();
    }
    if (check === "leftAgain") {
      heading.current?.focus();
    }
  }, [check]);
  if (!ended) {
    return null;
  }
  const name = membership.name;
  const sentence = membership_added_back_prompt(ended.addedBy, name);
  return (
    <section aria-label={membership_ended_heading(name, ended.left)} className="flex flex-col gap-2">
      <p ref={heading} tabIndex={-1} className="text-sm text-[var(--toolbar-icon)] outline-none">
        {membership_ended_heading(name, ended.left)}
      </p>
      {check === "addedBack" ? (
        <div ref={prompt} role="group" tabIndex={-1} aria-label={sentence} aria-busy={phase !== ""} className="flex flex-col gap-2 outline-none">
          <p className="text-xs text-[var(--toolbar-icon)]">{sentence}</p>
          <p className="text-xs text-[var(--toolbar-icon)]">{membership_word("merge")}</p>
          <p className="text-xs text-[var(--toolbar-icon)]">{membership_keep_consequence(name)}</p>
          {phase !== "" ? (
            <p role="status" aria-live="polite" className={MUTED}>
              {membership_word(phase === "syncing" ? "syncing" : "leaving")}
            </p>
          ) : (
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                className={`${BUTTON} ${APPLY} w-[100px]`}
                onClick={() => {
                  setFailure("");
                  setPhase("syncing");
                  // The buttons give way to a status line: the group keeps the focus.
                  prompt.current?.focus();
                  void membership.syncAgain().then((refusal) => {
                    if (refusal === "") {
                      focusParticipants();
                    } else {
                      setPhase("");
                      setFailure(refusal);
                      prompt.current?.focus();
                    }
                  });
                }}
              >
                {membership_word("sync-again")}
              </button>
              <button
                type="button"
                className={`${BUTTON} ${OUTLINE} w-[110px]`}
                onClick={() => {
                  setFailure("");
                  setPhase("leaving");
                  prompt.current?.focus();
                  void membership.keepAsItIs().then((refusal) => {
                    if (refusal !== "") {
                      setPhase("");
                      setFailure(refusal);
                      prompt.current?.focus();
                    }
                  });
                }}
              >
                {membership_word("keep-as-it-is")}
              </button>
            </div>
          )}
          {failure ? (
            <p role="alert" className={ALERT}>
              {failure}
            </p>
          ) : null}
        </div>
      ) : (
        <div className="flex flex-col gap-1">
          <button
            type="button"
            aria-busy={check === "checking"}
            aria-disabled={check === "checking"}
            className={`${BUTTON} ${OUTLINE} w-[244px]`}
            onClick={() => {
              if (check !== "checking") {
                // A new check: the prompt it may bring starts without a busy line.
                setPhase("");
                membership.check();
              }
            }}
          >
            <UserCheck size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
            {membership_word(check === "checking" ? "checking" : "check")}
          </button>
          <p className={MUTED}>{membership_contacts_line(ended.server)}</p>
          {/* Mounted with the block, so the first answer is announced (ux-notes 7.1). */}
          <p role="status" aria-live="polite" className={MUTED}>
            {check === "notYet" ? membership_word("not-added-back") : check === "notFound" ? membership_word("check-not-found") : ""}
          </p>
          {check === "unverified" ? (
            <p role="alert" className={ALERT}>
              {membership_word("check-unverified")}
            </p>
          ) : null}
          {check === "failed" ? (
            <p role="alert" className={ALERT}>
              {membership_cannot_reach(ended.server)}
            </p>
          ) : null}
        </div>
      )}
    </section>
  );
}

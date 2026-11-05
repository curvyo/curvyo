// The Share tab's admin-change line and Notices block (`specs/0097-membership-lifecycle`
// ux-notes 2.3 and 4): who became or stopped being an admin, the rotation pending notice with
// Change keys now, and the mutual-removal notice with OK and its held-back lines.

import { useEffect, useRef, useState } from "react";

import { ALERT, APPLY, BUTTON, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import type { ShareView } from "@/hooks/useCollaboration";
import type { MembershipApi } from "@/hooks/useMembership";
import type { MembershipNotice } from "@/lib/membershipState";
import { focusParticipants } from "@/lib/shareFocus";
import {
  membership_admin_change,
  membership_more_changes,
  membership_mutual_removal,
  membership_not_applied,
  membership_rotation_pending,
  membership_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** The admin-change line under the header (criterion 30): the newest change, one line. The
 * live region stays mounted while empty, so the first change is announced. */
export function AdminChangeLine({ membership }: { membership: MembershipApi }) {
  const line = membership.line;
  return (
    <p role="status" aria-live="polite" className={line ? "flex items-baseline gap-1 text-xs text-[var(--toolbar-icon)]" : "sr-only"}>
      {line ? (
        <>
          <span className="min-w-0 truncate">{membership_admin_change(line.kind, line.label)}</span>
          {line.more > 0 ? <span className="text-[var(--panel-muted-fg)]">{membership_more_changes(line.more)}</span> : null}
          <span className="ml-auto shrink-0 text-[var(--panel-muted-fg)]">{line.at}</span>
        </>
      ) : null}
    </p>
  );
}

/** How long Change keys now stays busy at most when the notice does not go (ms). */
const ROTATE_MS = 10000;

function RotationNotice({ pending, admin, membership }: { pending: MembershipNotice[]; admin: boolean; membership: MembershipApi }) {
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState("");
  const text = membership_rotation_pending(JSON.stringify(pending.map((n) => [n.label ?? "", n.left === true])), admin);
  // The rotation never completed in time: the button is usable again.
  useEffect(() => {
    if (!busy) {
      return undefined;
    }
    const timer = window.setTimeout(() => setBusy(false), ROTATE_MS);
    return () => window.clearTimeout(timer);
  }, [busy]);
  return (
    <div role="group" aria-label={text} className="flex flex-col gap-1">
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      {admin ? (
        <button
          type="button"
          aria-busy={busy}
          aria-disabled={busy}
          className={`${BUTTON} ${APPLY} w-[140px]`}
          onClick={() => {
            if (busy) {
              return;
            }
            const answer = membership.rotate();
            setRefusal(answer);
            if (answer === "") {
              // No confirm: nothing is lost. The focus stays here and moves to the Participants
              // list when the notice goes (ux-notes 9).
              setBusy(true);
            }
          }}
        >
          {membership_word(busy ? "changing-keys" : "change-keys-now")}
        </button>
      ) : null}
      {refusal ? (
        <p role="alert" className={ALERT}>
          {refusal}
        </p>
      ) : null}
    </div>
  );
}

function MutualNotice({
  notice,
  held,
  me,
  membership,
}: {
  notice: MembershipNotice;
  held: MembershipNotice[];
  me: string;
  membership: MembershipApi;
}) {
  const admins = notice.admins ?? [];
  const text = membership_mutual_removal(
    JSON.stringify(admins.filter((a) => a.member !== me).map((a) => a.label)),
    admins.some((a) => a.member === me),
  );
  return (
    <div role="group" aria-label={text} className="flex flex-col gap-1">
      <p className="text-xs text-[var(--toolbar-icon)]">{text}</p>
      {held.map((h) => (
        <p key={h.entries?.[0] ?? ""} className={MUTED}>
          {membership_not_applied(h.label ?? "", h.removal === true)}
        </p>
      ))}
      <button
        type="button"
        className={`${BUTTON} ${OUTLINE} w-[60px]`}
        onClick={() => {
          // OK removes the notice and its button: the focus goes to the Participants list.
          membership.acknowledge(notice);
          focusParticipants();
        }}
      >
        {membership_word("ok")}
      </button>
    </div>
  );
}

/** The words of the shown notices, for the one live region that stays mounted. */
function spoken(pending: MembershipNotice[], mutual: MembershipNotice | undefined, held: MembershipNotice[], admin: boolean, me: string): string {
  const parts: string[] = [];
  if (pending.length > 0) {
    parts.push(membership_rotation_pending(JSON.stringify(pending.map((n) => [n.label ?? "", n.left === true])), admin));
  }
  if (mutual) {
    const admins = mutual.admins ?? [];
    parts.push(
      membership_mutual_removal(
        JSON.stringify(admins.filter((a) => a.member !== me).map((a) => a.label)),
        admins.some((a) => a.member === me),
      ),
    );
  }
  parts.push(...held.map((h) => membership_not_applied(h.label ?? "", h.removal === true)));
  return parts.join(" ");
}

/** The Notices block (ux-notes 4): its live region is always mounted (empty when there is
 * nothing to say), so a notice that arrives is announced. */
export function MembershipNotices({ view, membership }: { view: ShareView; membership: MembershipApi }) {
  const shown = membership.notices;
  const pending = shown.filter((n) => n.kind === "rotationPending");
  const mutual = shown.find((n) => n.kind === "mutualRemoval");
  const held = shown.filter((n) => n.kind === "notApplied");
  const me = view.participants.find((p) => p.self)?.member ?? "";
  const admin = view.admin === true;
  // Change keys now done: the notice goes, and with it the focused button (ux-notes 9).
  const block = useRef<HTMLDivElement>(null);
  const hadPending = useRef(false);
  useEffect(() => {
    const hadFocus = hadPending.current && block.current?.contains(document.activeElement) !== true && document.activeElement === document.body;
    if (pending.length === 0 && hadFocus) {
      focusParticipants();
    }
    hadPending.current = pending.length > 0;
  }, [pending.length]);
  const empty = pending.length === 0 && !mutual && held.length === 0;
  return (
    <div ref={block} className={empty ? "" : "flex flex-col gap-2"}>
      <p role="status" aria-live="polite" className="sr-only">
        {spoken(pending, mutual, held, admin, me)}
      </p>
      {pending.length > 0 ? <RotationNotice pending={pending} admin={admin} membership={membership} /> : null}
      {mutual ? (
        <MutualNotice notice={mutual} held={held} me={me} membership={membership} />
      ) : (
        held.map((h) => (
          <p key={h.entries?.[0] ?? ""} className="text-xs text-[var(--toolbar-icon)]">
            {membership_not_applied(h.label ?? "", h.removal === true)}
          </p>
        ))
      )}
    </div>
  );
}

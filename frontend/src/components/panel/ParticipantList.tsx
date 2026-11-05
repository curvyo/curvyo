// The Share tab's participants (`specs/0097-membership-lifecycle` ux-notes 3): one row per
// device with its role line, a role slot and an action slot, their inline confirms, and the
// second-admin line on top. Words come from Rust; every confirm returns focus to its opener.

import { LogOut, ShieldMinus, ShieldPlus, TriangleAlert, UserMinus, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { ALERT, BUTTON, DestructiveConfirm, MUTED, OUTLINE } from "@/components/panel/devicesUi";
import { AvatarDisc } from "@/components/panel/ParticipantDisc";
import type { ShareParticipant, ShareView } from "@/hooks/useCollaboration";
import type { MembershipApi } from "@/hooks/useMembership";
import { usePendingLine } from "@/hooks/usePendingLine";
import { frozenWarning } from "@/lib/frozenWarning";
import { focusEndedHeading, focusParticipants, takeParticipantsFocus } from "@/lib/shareFocus";
import {
  membership_key_epoch,
  membership_leave_name,
  membership_make_admin_confirm,
  membership_make_member_confirm,
  membership_removal_confirm,
  membership_remove_name,
  membership_role_action_name,
  membership_role_line,
  membership_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
const ICON = `${BUTTON} ${OUTLINE} w-7 border-transparent pointer-coarse:w-9`;

type Asking = "" | "admin" | "member" | "step-down" | "remove" | "leave";

/** Focus after a row's act: the list when the row is gone (ux-notes 9). */
function focusList(list: HTMLUListElement | null, member: string, slot: "role" | "action") {
  window.setTimeout(() => {
    const target = list?.querySelector<HTMLElement>(`[data-member="${member}"] [data-slot="${slot}"]`);
    (target ?? list)?.focus();
  }, 0);
}

function ParticipantRow({
  participant,
  myDevice,
  onlyAdmin,
  sameLabel,
  leaveText,
  name,
  membership,
  list,
  roomState,
}: {
  participant: ShareParticipant;
  myDevice: boolean;
  /** This device is the only admin among several participants (criteria 5 and 14). */
  onlyAdmin: boolean;
  /** Another row's label equal to this one ("" when none). */
  sameLabel: string;
  leaveText: string;
  name: string;
  membership: MembershipApi;
  list: React.RefObject<HTMLUListElement | null>;
  /** Who is in the room and who is an admin: a refusal shown holds while it is unchanged. */
  roomState: string;
}) {
  const [asking, setAsking] = useState<Asking>("");
  // A refusal holds for the room as it was: a role change or another admin hides it.
  const [refused, setAlert] = useState({ text: "", room: "" });
  const roleButton = useRef<HTMLButtonElement>(null);
  const actionButton = useRef<HTMLButtonElement>(null);
  const room = roomState;
  const alert = refused.room === room ? refused.text : "";
  const own = participant.self;
  const label = participant.label;
  // The busy line holds until the server confirms the act (ADR 0041 decision 11).
  const busy = usePendingLine(participant.pending);

  const close = (opener: "role" | "action") => {
    setAsking("");
    window.setTimeout(() => (opener === "role" ? roleButton : actionButton).current?.focus(), 0);
  };
  const refuseAlone = () => {
    // No confirm opens: the reason, and the next step's control gets the focus (ux-notes 3.3).
    setAlert({ text: membership_word("only-admin"), room });
    window.setTimeout(() => list.current?.querySelector<HTMLElement>('[data-slot="role"][data-make-admin]')?.focus(), 0);
  };
  const makeAdmin = participant.canMakeAdmin === true;
  const roleAction = makeAdmin || participant.canMakeMember === true || participant.canStepDown === true;
  const frozenText = frozenWarning(membership.frozen);
  const roleLine = busy ?? membership_role_line(participant.admin, participant.online);

  let confirm = null;
  if (asking === "admin") {
    confirm = (
      <DestructiveConfirm
        text={frozenText || membership_make_admin_confirm(label, sameLabel)}
        label={membership_role_action_name(label, true, false)}
        action={membership_word(frozenText ? "make-admin-anyway" : "make-admin")}
        width={frozenText ? 140 : 100}
        apply={frozenText === ""}
        onCancel={() => close("role")}
        onConfirm={() => {
          const refusal = membership.setRole(participant.member, true);
          if (refusal === "") {
            close("role");
          }
          return refusal;
        }}
      />
    );
  } else if (asking === "member" || asking === "step-down") {
    const stepDown = asking === "step-down";
    confirm = (
      <DestructiveConfirm
        text={stepDown ? membership_word("step-down-confirm") : membership_make_member_confirm(label)}
        label={membership_role_action_name(label, false, stepDown)}
        action={membership_word(stepDown ? "step-down" : "make-member")}
        width={110}
        onCancel={() => close("role")}
        onConfirm={() => {
          const refusal = membership.setRole(participant.member, false);
          if (refusal === "") {
            setAsking("");
            // Step down empties the role slot: the focus goes to the row's Leave (ux-notes 9).
            window.setTimeout(() => (stepDown ? actionButton : roleButton).current?.focus(), 0);
          }
          return refusal;
        }}
      />
    );
  } else if (asking === "remove") {
    confirm = (
      <DestructiveConfirm
        text={membership_removal_confirm(label, participant.admin)}
        label={membership_remove_name(label)}
        action={membership_word("remove")}
        onCancel={() => close("action")}
        onConfirm={() => {
          const refusal = membership.remove(participant.member);
          if (refusal === "") {
            setAsking("");
            const rows = [...(list.current?.querySelectorAll<HTMLElement>("[data-member]") ?? [])];
            const index = rows.findIndex((r) => r.dataset.member === participant.member);
            const next = rows[index + 1] ?? rows[index - 1];
            focusList(list.current, next?.dataset.member ?? "", "role");
          }
          return refusal;
        }}
      />
    );
  } else if (asking === "leave") {
    confirm = (
      <DestructiveConfirm
        text={leaveText}
        label={membership_leave_name(name)}
        action={membership_word("leave")}
        onCancel={() => close("action")}
        onConfirm={() => {
          const refusal = membership.leave();
          if (refusal === "") {
            focusEndedHeading();
            setAsking("");
          }
          return refusal;
        }}
      />
    );
  }

  return (
    <li data-member={participant.member} className="flex flex-col" aria-busy={busy !== null}>
      <div className="flex h-14 items-center gap-2 pointer-coarse:h-[60px]">
        <AvatarDisc participant={participant} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm text-[var(--toolbar-icon)]" title={label}>
            {label}
            {own ? (
              <span className="text-[var(--panel-muted-fg)]"> {membership_word("this-device")}</span>
            ) : myDevice ? (
              <span className="text-[var(--panel-muted-fg)]"> {membership_word("my-device")}</span>
            ) : null}
          </p>
          <p className="truncate font-mono text-xs text-[var(--panel-muted-fg)]">{participant.fingerprint}</p>
          <p className={`text-xs ${participant.admin ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}>{roleLine}</p>
        </div>
        {roleAction ? (
          <button
            ref={roleButton}
            type="button"
            data-slot="role"
            data-make-admin={makeAdmin ? "" : undefined}
            aria-label={membership_role_action_name(label, makeAdmin, own)}
            aria-disabled={own && onlyAdmin}
            title={own && onlyAdmin ? membership_word("only-admin-title") : membership_word(makeAdmin ? "make-admin" : own ? "step-down" : "make-member")}
            className={ICON}
            onClick={() => {
              setAlert({ text: "", room });
              if (own && onlyAdmin) {
                refuseAlone();
                return;
              }
              setAsking(makeAdmin ? "admin" : own ? "step-down" : "member");
            }}
          >
            {makeAdmin ? <ShieldPlus {...GLYPH} /> : <ShieldMinus {...GLYPH} />}
          </button>
        ) : (
          <span aria-hidden className="w-7 shrink-0 pointer-coarse:w-9" />
        )}
        {participant.removable && !myDevice ? (
          <button
            ref={actionButton}
            type="button"
            data-slot="action"
            aria-label={membership_remove_name(label)}
            title={membership_word("remove")}
            className={ICON}
            onClick={() => {
              setAlert({ text: "", room });
              setAsking("remove");
            }}
          >
            <UserMinus {...GLYPH} />
          </button>
        ) : own && participant.canLeave ? (
          <button
            ref={actionButton}
            type="button"
            data-slot="action"
            aria-label={membership_leave_name(name)}
            aria-disabled={onlyAdmin && !membership.personLeave}
            title={onlyAdmin && !membership.personLeave ? membership_word("only-admin-title") : membership_word("leave")}
            className={ICON}
            onClick={() => {
              setAlert({ text: "", room });
              if (onlyAdmin && !membership.personLeave) {
                refuseAlone();
                return;
              }
              setAsking("leave");
            }}
          >
            <LogOut {...GLYPH} />
          </button>
        ) : (
          <span aria-hidden className="w-7 shrink-0 pointer-coarse:w-9" />
        )}
      </div>
      {confirm}
      {alert ? (
        <p role="alert" className={`${ALERT} pb-2`}>
          {alert}
        </p>
      ) : null}
    </li>
  );
}

/** The second-admin line (ux-notes 3.5): a state line with Dismiss, per tier. Its live region
 * stays mounted while empty, so the line is announced when it appears. */
function SecondAdminLine({ membership }: { membership: MembershipApi }) {
  const tier = membership.notices.find((n) => n.kind === "oneAdmin" || n.kind === "onlyOwnAdmins");
  const text = tier ? membership_word(tier.kind === "oneAdmin" ? "one-admin" : "only-own-admins") : "";
  return (
    <div className={tier ? "flex items-start gap-2 py-1" : ""}>
      {tier ? <TriangleAlert {...GLYPH} className="mt-0.5 shrink-0 text-[var(--toolbar-icon)]" /> : null}
      <p role="status" aria-live="polite" className={tier ? "min-w-0 flex-1 text-xs text-[var(--toolbar-icon)]" : "sr-only"}>
        {text}
      </p>
      {tier ? (
        <button
          type="button"
          aria-label={membership_word("dismiss")}
          title={membership_word("dismiss")}
          className={ICON}
          onClick={() => {
            // Dismiss removes itself: the focus goes to the Participants list (ux-notes 9).
            membership.dismissTier(tier.kind);
            focusParticipants();
          }}
        >
          <X {...GLYPH} />
        </button>
      ) : null}
    </div>
  );
}

/** The Participants list of a shared document or workspace (criteria 1 to 9, 14, 16). */
export function ParticipantList({
  view,
  myDevices,
  leaveText,
  membership,
}: {
  view: ShareView;
  myDevices: Set<string>;
  /** Leave's confirm sentence (document, workspace, or the person leave). */
  leaveText: string;
  membership: MembershipApi;
}) {
  const list = useRef<HTMLUListElement>(null);
  const admins = view.participants.filter((p) => p.admin).length;
  const me = view.participants.find((p) => p.self);
  const onlyAdmin = me?.admin === true && admins === 1 && view.participants.length > 1;
  // A row that disappears while it holds the focus hands it to the list (ux-notes 9).
  const members = view.participants.map((p) => p.member).join(",");
  const roomState = view.participants.map((p) => `${p.member}:${p.admin}`).join(",");
  useEffect(() => {
    if (takeParticipantsFocus()) {
      list.current?.focus();
    }
  }, []);
  useEffect(() => {
    if (document.activeElement === document.body && list.current?.dataset.hadFocus === "1") {
      list.current.focus();
    }
  }, [members]);
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-baseline justify-between">
        <span className="text-xs text-[var(--toolbar-icon)]">{membership_word("participants")}</span>
        {view.epoch !== null && view.epoch !== undefined ? <span className={MUTED}>{membership_key_epoch(view.epoch)}</span> : null}
      </div>
      {view.admin ? <SecondAdminLine membership={membership} /> : null}
      <ul
        ref={list}
        tabIndex={-1}
        aria-label={membership_word("participants")}
        className="flex flex-col outline-none"
        onFocus={() => {
          if (list.current) list.current.dataset.hadFocus = "1";
        }}
        onBlur={(event) => {
          if (list.current && !list.current.contains(event.relatedTarget as Node | null)) list.current.dataset.hadFocus = "";
        }}
      >
        {view.participants.map((participant) => (
          <ParticipantRow
            key={participant.fingerprint}
            participant={participant}
            myDevice={myDevices.has(participant.member)}
            onlyAdmin={onlyAdmin}
            sameLabel={
              view.participants.find((p) => p.member !== participant.member && p.label === participant.label)?.fingerprint.slice(0, 9) ?? ""
            }
            leaveText={leaveText}
            name={membership.name}
            membership={membership}
            list={list}
            roomState={roomState}
          />
        ))}
      </ul>
    </div>
  );
}

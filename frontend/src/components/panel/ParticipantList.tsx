// The Share tab's participants (`specs/0097-membership-lifecycle` ux-notes 3, grouped by person
// since `specs/0105` criterion 14): one row per device with its role line, a role slot and an
// action slot, their inline confirms, and the second-admin line on top. Words come from Rust;
// every act is an armed button (customer feedback on PR #281): a first press arms it, a second
// performs it.

import { ChevronRight, CircleOff, LogOut, ShieldMinus, ShieldPlus, UserMinus } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { APPLY, ARMED, ArmLines, BUTTON, OUTLINE } from "@/components/panel/devicesUi";
import { useArm } from "@/hooks/useArm";
import { AvatarDisc } from "@/components/panel/ParticipantDisc";
import type { ShareParticipant, ShareView } from "@/hooks/useCollaboration";
import type { MembershipApi } from "@/hooks/useMembership";
import { usePendingLine } from "@/hooks/usePendingLine";
import { frozenWarning } from "@/lib/frozenWarning";
import { focusEndedHeading, takeParticipantsFocus } from "@/lib/shareFocus";
import {
  membership_leave_name,
  membership_remove_name,
  membership_role_action_name,
  membership_role_line,
  membership_word,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

const GLYPH = { size: 16, strokeWidth: 1.5, absoluteStrokeWidth: true, "aria-hidden": true } as const;
const ICON = `${BUTTON} ${OUTLINE} w-7 border-transparent pointer-coarse:w-9`;
/** An icon action armed: Apply look for Make admin (nothing is lost), else red. */
const iconLook = (armed: boolean, apply = false) => (armed ? `${BUTTON} ${apply ? APPLY : ARMED} w-7 pointer-coarse:w-9` : ICON);

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
  lastAdmin,
  alone,
  name,
  membership,
  list,
}: {
  participant: ShareParticipant;
  myDevice: boolean;
  /** This device is the only admin among several participants (criteria 5 and 14). */
  onlyAdmin: boolean;
  /** This device is the room's only admin: Step down is greyed (customer 2026-11-09). */
  lastAdmin: boolean;
  /** This device is the only participant left: its action is Stop sharing. */
  alone: boolean;
  name: string;
  membership: MembershipApi;
  list: React.RefObject<HTMLUListElement | null>;
}) {
  const own = participant.self;
  const label = participant.label;
  const roleArm = useArm(`${participant.member}:role`);
  const actionArm = useArm(`${participant.member}:action`);
  const actionButton = useRef<HTMLButtonElement>(null);
  // The busy line holds until the server confirms the act (ADR 0041 decision 11).
  const busy = usePendingLine(participant.pending);

  const makeAdmin = participant.canMakeAdmin === true;
  const roleAction = makeAdmin || participant.canMakeMember === true || participant.canStepDown === true;
  const frozenText = frozenWarning(membership.frozen);
  const roleLine = busy ?? membership_role_line(participant.admin, participant.online);
  // Greyed, with the reason as tooltip and description; a press does nothing (ux-notes 3.3).
  const roleBlocked = own && lastAdmin;
  const leaveBlocked = onlyAdmin && !membership.personLeave;
  const blocked = membership_word("only-admin-title");
  const removable = participant.removable && !myDevice;

  const pressRole = () => {
    if (roleBlocked) {
      return;
    }
    roleArm.press(() => {
      const refusal = membership.setRole(participant.member, makeAdmin);
      if (refusal === "" && own) {
        // Step down empties the role slot: the focus goes to the row's Leave (ux-notes 9).
        window.setTimeout(() => actionButton.current?.focus(), 0);
      }
      return refusal;
    });
  };
  const stopping = own && alone;
  const pressAction = () => {
    if (!removable && !stopping && leaveBlocked) {
      return;
    }
    actionArm.press(() => {
      if (stopping) {
        return membership.stopSharing();
      }
      if (removable) {
        const refusal = membership.remove(participant.member);
        if (refusal === "") {
          const rows = [...(list.current?.querySelectorAll<HTMLElement>("[data-member]") ?? [])];
          const index = rows.findIndex((r) => r.dataset.member === participant.member);
          const next = rows[index + 1] ?? rows[index - 1];
          focusList(list.current, next?.dataset.member ?? "", "role");
        }
        return refusal;
      }
      const refusal = membership.leave();
      if (refusal === "") {
        focusEndedHeading();
      }
      return refusal;
    });
  };

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
          <span {...roleArm.zone} className="flex">
            <button
              {...roleArm.control}
              type="button"
              data-slot="role"
              data-make-admin={makeAdmin ? "" : undefined}
              aria-label={membership_role_action_name(label, makeAdmin, own)}
              aria-disabled={roleBlocked}
              aria-description={roleBlocked ? blocked : undefined}
              title={roleBlocked ? blocked : membership_word(makeAdmin ? (frozenText ? "make-admin-anyway" : "make-admin") : own ? "step-down" : "make-member")}
              className={iconLook(roleArm.armed, makeAdmin && !frozenText)}
              onClick={pressRole}
            >
              {makeAdmin ? <ShieldPlus {...GLYPH} /> : <ShieldMinus {...GLYPH} />}
            </button>
          </span>
        ) : (
          <span aria-hidden className="w-7 shrink-0 pointer-coarse:w-9" />
        )}
        {stopping ? (
          <span {...actionArm.zone} className="flex">
            <button
              {...actionArm.control}
              type="button"
              data-slot="action"
              aria-label={membership_word("stop-sharing")}
              title={membership_word("stop-sharing")}
              className={iconLook(actionArm.armed)}
              onClick={pressAction}
            >
              <CircleOff {...GLYPH} />
            </button>
          </span>
        ) : removable || (own && participant.canLeave) ? (
          <span {...actionArm.zone} className="flex">
            <button
              {...actionArm.control}
              ref={actionButton}
              type="button"
              data-slot="action"
              aria-label={removable ? membership_remove_name(label) : membership_leave_name(name)}
              aria-disabled={!removable && leaveBlocked}
              aria-description={!removable && leaveBlocked ? blocked : undefined}
              title={!removable && leaveBlocked ? blocked : membership_word(removable ? "remove" : "leave")}
              className={iconLook(actionArm.armed)}
              onClick={pressAction}
            >
              {removable ? <UserMinus {...GLYPH} /> : <LogOut {...GLYPH} />}
            </button>
          </span>
        ) : (
          <span aria-hidden className="w-7 shrink-0 pointer-coarse:w-9" />
        )}
      </div>
      <ArmLines arm={roleArm} className="pb-2" />
      <ArmLines arm={actionArm} className="pb-2" />
    </li>
  );
}

/** One person of the people view (`specs/0105` criterion 14): its line, which expands to one row
 * per device; roles, Remove and Leave act on a device. */
function PersonGroup({
  line,
  children,
}: {
  line: string;
  children: React.ReactNode;
}) {
  const [expanded, setExpanded] = useState(true);
  return (
    <li className="flex flex-col">
      <button
        type="button"
        aria-expanded={expanded}
        className="flex min-h-[32px] items-center gap-1 text-left text-sm text-[var(--toolbar-icon)]"
        onClick={() => setExpanded((now) => !now)}
      >
        <ChevronRight {...GLYPH} className={`shrink-0 transition-transform ${expanded ? "rotate-90" : ""}`} />
        <span className="min-w-0 truncate">{line}</span>
      </button>
      {expanded ? <ul className="flex flex-col pl-5">{children}</ul> : null}
    </li>
  );
}

/** The Participants list of a shared workspace (`specs/0097` criteria 1 to 9, 14, 16), grouped by
 * person for display (`specs/0105` criterion 14). */
export function ParticipantList({
  view,
  myDevices,
  membership,
}: {
  view: ShareView;
  myDevices: Set<string>;
  membership: MembershipApi;
}) {
  const list = useRef<HTMLUListElement>(null);
  const admins = view.participants.filter((p) => p.admin).length;
  const me = view.participants.find((p) => p.self);
  const onlyAdmin = me?.admin === true && admins === 1 && view.participants.length > 1;
  const lastAdmin = me?.admin === true && admins === 1;
  // A row that disappears while it holds the focus hands it to the list (ux-notes 9).
  const members = view.participants.map((p) => p.member).join(",");
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
  const row = (participant: ShareParticipant) => (
    <ParticipantRow
      key={participant.fingerprint}
      participant={participant}
      myDevice={myDevices.has(participant.member)}
      onlyAdmin={onlyAdmin}
      lastAdmin={lastAdmin}
      alone={view.participants.length === 1}
      name={membership.name}
      membership={membership}
      list={list}
    />
  );
  const people = view.people ?? [];
  const grouped = new Set(people.flatMap((person) => person.members));
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-baseline justify-between">
        <span className="text-xs text-[var(--toolbar-icon)]">{membership_word("participants")}</span>
      </div>
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
        {people.map((person) => (
          <PersonGroup key={person.members.join(",")} line={person.line}>
            {person.members
              .map((member) => view.participants.find((p) => p.member === member))
              .filter((p): p is ShareParticipant => p !== undefined)
              .map(row)}
          </PersonGroup>
        ))}
        {view.participants.filter((p) => !grouped.has(p.member)).map(row)}
      </ul>
    </div>
  );
}

// The Removed block of a file or workspace this device was removed from or left
// (`specs/0097-membership-lifecycle` ux-notes 7): its heading. The way back is a new invite
// (customer 2026-11-09: no add back, so no check and no prompt).

import { useEffect, useRef } from "react";

import type { MembershipApi } from "@/hooks/useMembership";
import { takeEndedHeadingFocus } from "@/lib/shareFocus";
import { membership_ended_heading } from "@/wasm-bindings/curvyo_editor_wasm.js";

export function EndedBlock({ membership }: { membership: MembershipApi }) {
  const ended = membership.ended;
  const heading = useRef<HTMLParagraphElement>(null);
  useEffect(() => {
    if (takeEndedHeadingFocus()) {
      heading.current?.focus();
    }
  }, []);
  if (!ended) {
    return null;
  }
  const name = membership.name;
  return (
    <section aria-label={membership_ended_heading(name, ended.left)} className="flex flex-col gap-2">
      <p ref={heading} tabIndex={-1} className="text-sm text-[var(--toolbar-icon)] outline-none">
        {membership_ended_heading(name, ended.left)}
      </p>
    </section>
  );
}

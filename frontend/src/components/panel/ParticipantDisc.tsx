// A participant's disc in the Share tab and the collaboration pill (`specs/0092` UX notes §4).

import type { ShareParticipant } from "@/hooks/useCollaboration";

/** A participant's disc: initials on the participant's colour (UX notes §4). */
export function AvatarDisc({ participant, size = 24 }: { participant: Pick<ShareParticipant, "initials" | "colour" | "online">; size?: number }) {
  return (
    <span
      aria-hidden
      className="inline-flex shrink-0 items-center justify-center rounded-full text-[11px] font-bold text-white"
      style={{
        width: size,
        height: size,
        background: `var(--peer-${participant.colour})`,
        opacity: participant.online ? 1 : 0.55,
      }}
    >
      {participant.initials}
    </span>
  );
}

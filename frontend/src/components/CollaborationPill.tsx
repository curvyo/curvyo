import { CloudOff, Radio, RefreshCw } from "lucide-react";

import { AvatarDisc } from "@/components/panel/ShareTab";
import { Tooltip } from "@/components/ui/tooltip";
import type { ShareView } from "@/hooks/useCollaboration";

/**
 * The collaboration pill (`specs/0092-cooperative-editing` UX notes §3): at the canvas's bottom
 * right while the open document is shared, it shows who is here and the connection, and opens the
 * Share tab. Online participants only; the list keeps the offline ones.
 */
export function CollaborationPill({ view, onOpen }: { view: ShareView; onOpen: () => void }) {
  if (!view.shared || !view.live) {
    return null;
  }
  const online = view.participants.filter((p) => p.online);
  const shown = online.slice(0, 4);
  const state = view.state ?? "";
  const Glyph = state === "connected" ? Radio : state === "offline" || state === "not-connected" ? CloudOff : RefreshCw;
  const tooltip = view.participants
    .map((p) => (p.self ? `${p.label} (this device)` : p.label))
    .join("\n");
  return (
    <div className="pointer-events-auto absolute right-3 bottom-3 z-40 flex flex-col gap-2">
      <Tooltip side="top" content={<span className="whitespace-pre-line">{tooltip}</span>}>
        <button
          type="button"
          aria-label={view.pillName}
          onClick={onOpen}
          className="flex h-8 items-center gap-1.5 rounded-md bg-[var(--toolbar-bg)] px-1.5 py-1 text-xs text-[var(--toolbar-icon)] shadow-[var(--panel-elevation-shadow)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          <Glyph
            size={16}
            strokeWidth={1.5}
            absoluteStrokeWidth
            aria-hidden
            className={state === "reconnecting" || state === "resyncing" ? "motion-safe:animate-spin" : undefined}
          />
          <span className="flex">
            {shown.map((participant, index) => (
              <span
                key={participant.fingerprint}
                className="rounded-full ring-2 ring-[var(--toolbar-bg)]"
                style={{ marginLeft: index === 0 ? 0 : -6 }}
              >
                <AvatarDisc participant={participant} />
              </span>
            ))}
          </span>
          {online.length > 4 ? <span>+{online.length - 4}</span> : null}
          {view.pillText ? <span>{view.pillText}</span> : null}
        </button>
      </Tooltip>
    </div>
  );
}

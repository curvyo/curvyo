import { useEffect, useRef, useState } from "react";

import { CloneBlock, OriginLine } from "@/components/panel/CloneBlock";
import { FillSection } from "@/components/panel/FillSection";
import { StrokeSection } from "@/components/panel/StrokeSection";
import type { ClonesApi } from "@/hooks/useClones";
import type { StylePanelApi } from "@/hooks/useStylePanel";

interface StyleSectionProps {
  panel: StylePanelApi;
  /** Linked clones: the clone block replaces the sections for a selection of clones only, and
   * the origin line sits above them (`specs/0049-linked-clone` criteria 22 and 23). */
  clones: ClonesApi;
  onReturnFocus: () => void;
}

/**
 * The "Style" section of the properties panel (`specs/0017-style-panel-
 * rework`, `docs/design-system.md`, "Properties panel: Style section"): the
 * stroke and the fill (the subject line is in the panel's header row, since
 * `0043`). Every value comes from the session; every
 * control sends one command back.
 */
export function StyleSection({ panel, clones, onReturnFocus }: StyleSectionProps) {
  const body = useRef<HTMLDivElement>(null);
  const { focusRequest, panelNotice } = clones;
  // After a key press on a clone command the focus goes where the UX notes say ("Focus"): Stroke
  // Paint after Unlink, the first control of the new body after Select original or root.
  useEffect(() => {
    const element = body.current;
    if (!focusRequest || !element) {
      return;
    }
    const scope =
      focusRequest.target === "stroke"
        ? (element.querySelector('[aria-label="Stroke paint"]') ?? element)
        : element;
    const first = scope.querySelector<HTMLElement>(
      'button:not([tabindex="-1"]), input, [tabindex="0"]',
    );
    first?.focus();
  }, [focusRequest]);
  // The notice of the last block command: an overlay 4 px under the pressed button (for Unlink
  // where the button was), never part of the layout, so nothing below moves (UX notes "Focus",
  // customer rule 2026-10-12).
  const [anchor, setAnchor] = useState(0);
  const pressedAt = (button: DOMRect) => {
    const top = body.current?.getBoundingClientRect().top ?? button.top;
    setAnchor(button.bottom - top + 4);
  };
  const notice = panelNotice ? (
    <p
      key={panelNotice.id}
      role={panelNotice.alert ? "alert" : "status"}
      className="absolute left-0 z-10 max-w-[256px] break-words rounded-md px-2 py-1 text-xs"
      style={{
        top: anchor,
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {panelNotice.text}
    </p>
  ) : null;
  if (clones.panel.clonesOnly) {
    return (
      <div ref={body} className="relative flex flex-col">
        <CloneBlock clones={clones} onReturnFocus={onReturnFocus} onPressedAt={pressedAt} />
        {notice}
      </div>
    );
  }
  return (
    <div ref={body} className="relative flex flex-col gap-2">
      {notice}
      <OriginLine clones={clones} onReturnFocus={onReturnFocus} onPressedAt={pressedAt} />
      <StrokeSection panel={panel} onReturnFocus={onReturnFocus} />
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <FillSection panel={panel} onReturnFocus={onReturnFocus} />
    </div>
  );
}

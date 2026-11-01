import { type ReactNode, useEffect, useLayoutEffect, useRef, useState } from "react";

import { CloneBlock, GreyedStyleHeaders, OriginLine } from "@/components/panel/CloneBlock";
import { FillSection } from "@/components/panel/FillSection";
import { collapseSpacer } from "@/lib/collapseSpacer";
import { StrokeSection } from "@/components/panel/StrokeSection";
import type { ClonesApi } from "@/hooks/useClones";
import type { StylePanelApi } from "@/hooks/useStylePanel";

/** The element that scrolls the panel body, or null. */
function scrollerOf(element: HTMLElement | null): HTMLElement | null {
  for (let node = element?.parentElement ?? null; node; node = node.parentElement) {
    const overflow = getComputedStyle(node).overflowY;
    if (overflow === "auto" || overflow === "scroll") {
      return node;
    }
  }
  return null;
}

/** A notice of the Style tab: an overlay `top` px below the body's top, right-aligned, never part
 * of the layout and never in the way of the pointer (UX notes "Notices", review must-fix 3). */
function Notice({ top, alert, children }: { top: number; alert: boolean; children: ReactNode }) {
  return (
    <p
      role={alert ? "alert" : "status"}
      className="pointer-events-none absolute right-0 z-10 max-w-[256px] break-words rounded-md px-2 py-1 text-right text-xs"
      style={{
        top,
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {children}
    </p>
  );
}

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
  // The panel at a press on the clone block, before anything re-renders: its scroll offset and
  // the body's height, for the switch back to Original's (review must-fix 2).
  const atPress = useRef<{ scroll: number; height: number } | null>(null);
  const pressedAt = (button: DOMRect) => {
    const top = body.current?.getBoundingClientRect().top ?? button.top;
    setAnchor(button.bottom - top + 4);
    const scroller = scrollerOf(body.current);
    atPress.current =
      scroller && body.current
        ? { scroll: scroller.scrollTop, height: body.current.getBoundingClientRect().height }
        : null;
  };
  // The skip notice of criterion 18 sits 4 px under the header row of the section the edit was
  // made in (UX notes "Mixed edit ... anchored where the edit was made").
  const [sectionAnchor, setSectionAnchor] = useState(0);
  // The next pointer or key press in the panel hides a notice that is showing (review must-fix 3).
  const [hidden, setHidden] = useState<{ block: number; skip: number }>({ block: 0, skip: 0 });
  const skipNotice = panel.skipNotice;
  const onPress = (target: EventTarget | null) => {
    setHidden({ block: panelNotice?.id ?? 0, skip: skipNotice?.id ?? 0 });
    const section = target instanceof Element ? target.closest("[data-style-section]") : null;
    const header = section?.firstElementChild;
    const top = body.current?.getBoundingClientRect().top;
    if (header && top !== undefined) {
      setSectionAnchor(header.getBoundingClientRect().bottom - top + 4);
    }
  };
  const press = {
    onPointerDownCapture: (event: { target: EventTarget | null }) => onPress(event.target),
    onKeyDownCapture: (event: { target: EventTarget | null }) => onPress(event.target),
  };
  const notice = (
    <>
      {panelNotice && panelNotice.id !== hidden.block ? (
        <Notice key={panelNotice.id} top={anchor} alert={panelNotice.alert}>
          {panelNotice.text}
        </Notice>
      ) : null}
      {skipNotice && skipNotice.id !== hidden.skip ? (
        <Notice key={-skipNotice.id} top={sectionAnchor} alert={false}>
          {skipNotice.text}
        </Notice>
      ) : null}
    </>
  );
  // Fill and its rule are not in the tree while only single-line texts are edited: they have no
  // fill (`specs/0076-single-line-fonts` criterion 8, UX notes §4, as Markers for a text).
  const fill = panel.view.fillShown ? (
    <>
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <FillSection panel={panel} onReturnFocus={onReturnFocus} />
    </>
  ) : null;
  // Switching back to Original's closes the open sections; with the panel scrolled the browser
  // would clamp the scroll and the block would jump under the pointer. A spacer of the lost
  // height keeps the scroll offset, so the pressed row stays where it was; it goes with the next
  // selection change or when the maker scrolls up (review must-fix 2).
  const own = clones.panel.style !== "original";
  const [spacer, setSpacer] = useState(0);
  const lastOwn = useRef(own);
  // The offset the spacer makes reachable again.
  const keepScroll = useRef(0);
  // After the collapse is laid out: the spacer is the body's height at the press minus its height
  // now, so the scroll offset of the press is reachable again.
  useLayoutEffect(() => {
    const was = lastOwn.current;
    lastOwn.current = own;
    const scroller = scrollerOf(body.current);
    const press = atPress.current;
    atPress.current = null;
    if (!was || own || !scroller || !body.current || press === null || !clones.panel.clonesOnly) {
      return;
    }
    const lost = collapseSpacer(press.height, body.current.getBoundingClientRect().height);
    keepScroll.current = press.scroll;
    if (lost > 0) {
      setSpacer(lost);
    }
    scroller.scrollTop = press.scroll;
  }, [own, clones.panel.clonesOnly]);
  useLayoutEffect(() => {
    const scroller = scrollerOf(body.current);
    if (scroller && spacer > 0) {
      scroller.scrollTop = keepScroll.current;
    }
  }, [spacer]);
  useEffect(() => {
    if (spacer === 0) {
      return undefined;
    }
    const scroller = scrollerOf(body.current);
    const keep = scroller?.scrollTop ?? 0;
    const onScroll = () => {
      if (scroller && scroller.scrollTop < keep) {
        setSpacer(0);
      }
    };
    scroller?.addEventListener("scroll", onScroll);
    return () => scroller?.removeEventListener("scroll", onScroll);
  }, [spacer]);
  // The selection, not the style subject: the subject can change with the switch itself.
  const selectionKey = `${clones.panel.clonesOnly}|${clones.panel.count}|${clones.panel.original}`;
  const [spacerKey, setSpacerKey] = useState(selectionKey);
  if (selectionKey !== spacerKey) {
    // A change of the selection during render, the pattern React documents for derived state.
    setSpacerKey(selectionKey);
    setSpacer(0);
  }
  if (clones.panel.clonesOnly) {
    // With the original's style the sections stay as greyed headers; with an own style (or a
    // mix) they open as for a path and write to the clones with own style (`0057` criterion 10).
    return (
      <div ref={body} className="relative flex flex-col gap-2" {...press}>
        <CloneBlock clones={clones} onReturnFocus={onReturnFocus} onPressedAt={pressedAt} />
        {notice}
        <div className="mt-2 flex flex-col gap-2">
          {own ? (
            <>
              <StrokeSection panel={panel} onReturnFocus={onReturnFocus} />
              {fill}
            </>
          ) : (
            <GreyedStyleHeaders clones={clones} />
          )}
        </div>
        {spacer > 0 ? <div aria-hidden style={{ height: spacer }} /> : null}
      </div>
    );
  }
  return (
    <div ref={body} className="relative flex flex-col gap-2" {...press}>
      {notice}
      <OriginLine clones={clones} onReturnFocus={onReturnFocus} onPressedAt={pressedAt} />
      {/* A selection of dimensions only has nothing to edit here (`0070` UI review item 3). */}
      {panel.view.sectionsShown ? (
        <>
          <StrokeSection panel={panel} onReturnFocus={onReturnFocus} />
          {fill}
        </>
      ) : null}
    </div>
  );
}

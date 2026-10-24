// The input sink of text edit mode (ADR 0020 §5; `specs/0074-text-objects` criterion 3): one
// transparent 1 x 1 px textarea at the caret, so the IME's candidate window opens there. It holds
// focus while edit mode lasts and mirrors the string and selection for screen readers and IMEs;
// the canvas draws everything the maker sees.
import { useEffect, useLayoutEffect } from "react";

import type { TextApi } from "@/hooks/useText";
import { clampSinkPoint } from "@/lib/pageScroll";

interface TextSinkProps {
  text: TextApi;
  /** The Text tool is active: the sink is mounted, so a press that starts edit mode can focus it
   * at once. Outside edit mode it takes no Tab stop. */
  mounted: boolean;
}

export function TextSink({ text, mounted }: TextSinkProps) {
  const { sinkRef, sinkValue, sinkPoint, bar, editing } = text;

  // The value is set here only, never as `defaultValue`: React writes that prop again on every
  // render, and WebKitGTK (the Linux desktop build) then puts the caret back at 0, so Backspace
  // found nothing to delete.
  useLayoutEffect(() => {
    const sink = sinkRef.current;
    if (!sink || !sinkValue) {
      return;
    }
    if (sink.value !== sinkValue.text) {
      sink.value = sinkValue.text;
    }
    // Scalar positions to UTF-16 offsets.
    const units = (scalars: number) =>
      Array.from(sinkValue.text).slice(0, scalars).join("").length;
    sink.setSelectionRange(units(sinkValue.start), units(sinkValue.end));
    // `editing` and `mounted`: a sink mounted after the value arrived takes it too.
  }, [sinkRef, sinkValue, editing, mounted]);

  // The sink follows the caret but stays inside the canvas box: a focused field past the view made
  // the webview scroll the whole app to reveal it (customer report 2026-10-24).
  useLayoutEffect(() => {
    const sink = sinkRef.current;
    const box = sink?.offsetParent;
    if (!sink || !(box instanceof HTMLElement)) {
      return;
    }
    const point = clampSinkPoint(sinkPoint, box.clientWidth, box.clientHeight);
    sink.style.left = `${point.x}px`;
    sink.style.top = `${point.y}px`;
  }, [sinkRef, sinkPoint, editing, mounted]);

  const { onBeforeInput, ...handlers } = text.sinkHandlers;
  useEffect(() => {
    const sink = sinkRef.current;
    if (!sink) {
      return undefined;
    }
    sink.addEventListener("beforeinput", onBeforeInput);
    return () => sink.removeEventListener("beforeinput", onBeforeInput);
  }, [sinkRef, onBeforeInput, editing]);

  if (!editing && !mounted) {
    return null;
  }
  const size = bar.sizeMm === null ? "" : `, ${Math.round(bar.sizeMm * 100) / 100} mm`;
  return (
    <textarea
      ref={sinkRef}
      aria-label="Text"
      aria-description={`${bar.font}${size}`}
      autoCapitalize="off"
      autoComplete="off"
      autoCorrect="off"
      spellCheck={false}
      tabIndex={editing ? 0 : -1}
      {...handlers}
      className="pointer-events-none absolute m-0 resize-none overflow-hidden border-0 p-0 opacity-0 outline-none"
      style={{ left: 0, top: 0, width: 1, height: 1 }}
    />
  );
}

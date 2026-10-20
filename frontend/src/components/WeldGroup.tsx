// The Select bar's Weld group (`specs/0068-laserable-lettering/` criteria 1 to 3, UX notes §1 to
// §5): the Weld button and the Gap field, with the notices 4 px under them.
import { CircleAlert } from "lucide-react";
import { Tooltip } from "radix-ui";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { dimmedPress } from "@/lib/dimmedPress";

import type { ActionNotice } from "@/hooks/useActionNotice";
import type { WeldApi } from "@/hooks/useWeld";
import {
  WELD_DIMMED_LINE,
  WELD_GAP_NAME,
  WELD_GAP_RULE,
  WELD_NAME,
  WELD_RULE,
  gapBlurCommit,
  gapText,
  parseGap,
  stepGap,
} from "@/lib/weldText";

const BUTTON_CLASS =
  "flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

const TOOLTIP_CLASS = "z-50 max-w-[260px] rounded-md px-2 py-1 text-xs";

const TOOLTIP_STYLE = {
  background: "var(--toolbar-bg)",
  color: "var(--toolbar-icon)",
  boxShadow: "var(--panel-elevation-shadow)",
} as const;

/** How long the clamp notice and the "Enter a number" line stay under the field. */
const FIELD_NOTICE_MS = 3000;

/** Two discs as one solid silhouette, no inner arc (UX notes §2, `WeldGlyph`, the fallback: the
 * three-disc form read as a capsule at 1x). */
export function WeldGlyph() {
  return (
    <svg aria-hidden viewBox="0 0 16 16" className="size-4" fill="none">
      <path
        d="M8 5.35 A4 4 0 1 1 8 10.65 A4 4 0 1 1 8 5.35 Z"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/** The notice of the last Weld, 4 px under the button, left-aligned, shifted to stay right of the
 * tool rail and inside the window. */
function WeldNotice({ notice }: { notice: ActionNotice | null }) {
  const ref = useRef<HTMLDivElement>(null);
  const [shift, setShift] = useState(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!notice || !element) {
      setShift(0);
      return;
    }
    const rail = parseFloat(getComputedStyle(element).getPropertyValue("--rail-right")) || 0;
    const box = element.getBoundingClientRect();
    const room = document.documentElement.clientWidth - 8;
    setShift(Math.max(rail + 12 - (box.left - shift), Math.min(0, room - (box.right - shift))));
    // `shift` is read to undo the last one; the measure runs once per notice.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [notice]);
  const refusal = notice?.kind === "refusal";
  return (
    <div
      ref={ref}
      // A refusal is an alert; a success is spoken by the group's polite region instead.
      role={refusal ? "alert" : undefined}
      aria-hidden={refusal ? undefined : true}
      className={`absolute top-full left-0 z-50 mt-1 flex w-max max-w-[360px] items-start gap-1 rounded-md px-2 py-1 text-xs ${
        refusal ? "ring-1 ring-[var(--field-invalid)]" : ""
      }`}
      style={
        notice
          ? {
              background: refusal ? "var(--popover)" : "var(--toolbar-bg)",
              color: refusal ? "var(--field-invalid)" : "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
              transform: shift !== 0 ? `translateX(${shift}px)` : undefined,
            }
          : { display: "none" }
      }
    >
      {refusal ? <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" /> : null}
      <span className="break-words">{notice?.text}</span>
    </div>
  );
}

/** The Gap field: a bar number field in mm, committed by Enter and by blur. */
function GapField({ weld, onReturnFocus }: { weld: WeldApi; onReturnFocus: () => void }) {
  const shown = gapText(weld.gapMm);
  const [draft, setDraft] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  /** Escape was pressed: the blur it causes discards the edit instead of saving it. */
  const escaped = useRef(false);

  useEffect(() => {
    if (!notice) {
      return undefined;
    }
    const timer = window.setTimeout(() => setNotice(null), FIELD_NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  /** Commits the typed text; `false` when it is not a number (the stored value stays). */
  const commitDraft = (text: string | null = draft): boolean => {
    if (text === null) {
      return true;
    }
    const mm = parseGap(text);
    if (mm === null) {
      setInvalid(true);
      return false;
    }
    setNotice(weld.setGap(mm));
    setDraft(null);
    setInvalid(false);
    return true;
  };

  return (
    <div className="relative">
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <label
            className={`flex h-7 w-[104px] items-center rounded-[5px] bg-white px-2 text-sm focus-within:ring-2 ${
              invalid
                ? "ring-2 ring-[var(--field-invalid)] focus-within:ring-[var(--field-invalid)]"
                : "ring-1 ring-border focus-within:ring-[var(--editor-accent)]"
            }`}
          >
            <span className="text-[var(--toolbar-icon)]">Gap</span>
            <input
              type="text"
              inputMode="decimal"
              aria-label={WELD_GAP_NAME}
              aria-invalid={invalid}
              title={WELD_GAP_NAME}
              value={draft ?? shown}
              onChange={(event) => {
                setDraft(event.target.value);
                setInvalid(false);
              }}
              onKeyDown={(event) => {
                if (event.key === "ArrowUp" || event.key === "ArrowDown") {
                  event.preventDefault();
                  weld.setGap(stepGap(weld.gapMm, event.key === "ArrowUp", event.shiftKey));
                  setDraft(null);
                  setInvalid(false);
                } else if (event.key === "Enter") {
                  event.preventDefault();
                  if (commitDraft()) {
                    onReturnFocus();
                  }
                } else if (event.key === "Escape") {
                  event.preventDefault();
                  event.stopPropagation();
                  escaped.current = true;
                  setDraft(null);
                  setInvalid(false);
                  onReturnFocus();
                }
              }}
              // Blur commits too: the next press is usually the Weld button beside it (UX §3).
              onBlur={() => {
                const text = gapBlurCommit(draft, escaped.current);
                escaped.current = false;
                if (text === null) {
                  setDraft(null);
                  setInvalid(false);
                } else {
                  commitDraft(text);
                }
              }}
              className="min-w-0 flex-1 bg-transparent text-right tabular-nums outline-none"
            />
            <span className="pl-1 text-[var(--toolbar-icon)]">mm</span>
          </label>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content side="bottom" sideOffset={6} className={TOOLTIP_CLASS} style={TOOLTIP_STYLE}>
            <div className="font-semibold">{WELD_GAP_NAME}</div>
            <div style={{ color: "var(--panel-muted-fg)" }}>{WELD_GAP_RULE}</div>
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
      {invalid ? (
        <div className="absolute top-full left-0 mt-1 text-xs whitespace-nowrap text-[var(--field-invalid)]">
          Enter a number
        </div>
      ) : notice ? (
        <div
          role="status"
          className="absolute top-full left-0 z-50 mt-1 rounded-md px-2 py-1 text-xs whitespace-nowrap"
          style={TOOLTIP_STYLE}
        >
          {notice}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The Weld group: shown in every selection state of the Select tool, the button dimmed while
 * nothing is selected (`aria-disabled`, keeps its Tab stop and tooltip), the Gap field never.
 */
export function WeldGroup({
  enabled,
  weld,
  onReturnFocus,
}: {
  enabled: boolean;
  weld: WeldApi;
  onReturnFocus: () => void;
}) {
  return (
    <Tooltip.Provider delayDuration={400}>
      <div role="group" aria-label={WELD_NAME} aria-busy={weld.busy} className="flex items-center gap-1 py-1">
        <div className="relative">
          <Tooltip.Root>
            <Tooltip.Trigger asChild>
              <button
                type="button"
                aria-label={WELD_NAME}
                aria-disabled={!enabled}
                data-pressed={weld.busy ? "true" : undefined}
                onPointerDown={(event) => {
                  dimmedPress(event, enabled);
                }}
                onClick={(event) => {
                  if (dimmedPress(event, enabled) || weld.busy) {
                    return;
                  }
                  weld.weld();
                  if (event.detail > 0) {
                    onReturnFocus();
                  }
                }}
                className={`${BUTTON_CLASS} data-[pressed=true]:bg-[var(--editor-accent-hover)] ${
                  enabled ? "hover:bg-[var(--editor-accent-hover)]" : "opacity-40"
                }`}
              >
                <WeldGlyph />
              </button>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content side="bottom" sideOffset={6} className={TOOLTIP_CLASS} style={TOOLTIP_STYLE}>
                <div className="font-semibold">{WELD_NAME}</div>
                <div style={{ color: "var(--panel-muted-fg)" }}>{WELD_RULE}</div>
                {!enabled && <div style={{ color: "var(--panel-muted-fg)" }}>{WELD_DIMMED_LINE}</div>}
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
          <WeldNotice notice={weld.notice} />
        </div>
        <GapField weld={weld} onReturnFocus={onReturnFocus} />
        <div role="status" aria-live="polite" className="sr-only">
          {weld.notice?.kind === "success" ? <span key={weld.notice.id}>{weld.notice.text}</span> : null}
        </div>
      </div>
    </Tooltip.Provider>
  );
}

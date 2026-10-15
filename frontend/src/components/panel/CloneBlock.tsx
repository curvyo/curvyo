import { Link2 } from "lucide-react";
import { useEffect, useId, useRef } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { CloneCommand, ClonesApi } from "@/hooks/useClones";
import {
  cloneBlockLines,
  type OverrideSegment,
  type OverrideState,
  PLACEMENT_SEGMENTS,
  SPREAD_COLOURS_DISABLED_TITLE,
  SPREAD_COLOURS_TITLE,
  STYLE_SEGMENTS,
} from "@/lib/cloneText";

const OUTLINE_BUTTON =
  "h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-transparent text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface CloneBlockProps {
  clones: ClonesApi;
  onReturnFocus: () => void;
  /** The pressed button's box, for the notice that shows 4 px under it (UX notes "Focus"). */
  onPressedAt: (button: DOMRect) => void;
}

/** A button of the block: a mouse press returns the focus to the canvas, a key press keeps it
 * (and the block moves it where the UX notes say). Its tooltip is the panel's, at the left. */
function BlockButton({
  label,
  title,
  onPress,
  onReturnFocus,
  onHover,
  onPressedAt,
}: {
  label: string;
  title: string;
  onPressedAt: (button: DOMRect) => void;
  /** `keyboard` is true for Enter or Space. */
  onPress: (keyboard: boolean) => void;
  onReturnFocus: () => void;
  /** The targets show their hover box while the pointer or focus is on the button. */
  onHover?: (on: boolean) => void;
}) {
  // The button can go away without a pointerleave or a blur (the selection changed under it):
  // its hover box goes with it (UX review must-fix 3).
  const hoverRef = useRef(onHover);
  useEffect(() => {
    hoverRef.current = onHover;
  }, [onHover]);
  useEffect(() => () => hoverRef.current?.(false), []);
  return (
    <Tooltip side="left" content={title}>
      <button
        type="button"
        onPointerEnter={() => onHover?.(true)}
        onPointerLeave={() => onHover?.(false)}
        onFocus={() => onHover?.(true)}
        onBlur={() => onHover?.(false)}
        className={OUTLINE_BUTTON}
        onClick={(event) => {
          onHover?.(false);
          onPressedAt(event.currentTarget.getBoundingClientRect());
          const keyboard = event.detail === 0;
          onPress(keyboard);
          if (!keyboard) {
            onReturnFocus();
          }
        }}
      >
        {label}
      </button>
    </Tooltip>
  );
}

/**
 * One override row of the clone block (`specs/0057-clone-overrides` criteria 1, 10, 26 and 27;
 * UX notes "The clone block"): a label and a two-segment pill, a radiogroup with one Tab stop
 * (roving tabindex), arrows move and set, Space and Enter set. Mixed: no segment pressed. A mouse
 * press returns the focus to the canvas; a key press keeps it in the group.
 */
function OverrideRow({
  label,
  segments,
  state,
  onSet,
  onReturnFocus,
  onPressedAt,
}: {
  label: string;
  segments: OverrideSegment[];
  state: OverrideState;
  onSet: (own: boolean, keyboard: boolean) => void;
  onReturnFocus: () => void;
  onPressedAt: (button: DOMRect) => void;
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const mixedId = useId();
  const pressed = (segment: OverrideSegment) =>
    state !== "mixed" && segment.own === (state === "own");
  const focusable = (index: number) =>
    state === "mixed" ? index === 0 : pressed(segments[index] as OverrideSegment);
  const set = (index: number, keyboard: boolean) => {
    const segment = segments[index];
    const button = refs.current[index];
    if (!segment || !button) {
      return;
    }
    onPressedAt(button.getBoundingClientRect());
    if (!pressed(segment)) {
      onSet(segment.own, keyboard);
    }
    if (keyboard) {
      button.focus();
    } else {
      onReturnFocus();
    }
  };
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-[var(--panel-muted-fg)]">{label}</span>
      <div
        role="radiogroup"
        aria-label={label}
        aria-describedby={state === "mixed" ? mixedId : undefined}
        className="flex h-7 w-full rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]"
      >
        {segments.map((segment, index) => {
          const on = pressed(segment);
          return (
            <Tooltip key={segment.name} side="left" content={segment.title}>
              <button
                ref={(node) => {
                  refs.current[index] = node;
                }}
                type="button"
                role="radio"
                aria-checked={on}
                aria-label={segment.name}
                tabIndex={focusable(index) ? 0 : -1}
                className="flex-1 rounded-[4px] text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                style={
                  on
                    ? { background: "var(--editor-accent)", color: "#ffffff" }
                    : { color: "var(--toolbar-icon)" }
                }
                onClick={(event) => set(index, event.detail === 0)}
                onKeyDown={(event) => {
                  const next =
                    event.key === "ArrowRight" || event.key === "ArrowDown"
                      ? (index + 1) % segments.length
                      : event.key === "ArrowLeft" || event.key === "ArrowUp"
                        ? (index + segments.length - 1) % segments.length
                        : null;
                  if (next !== null) {
                    event.preventDefault();
                    set(next, true);
                  }
                }}
              >
                {segment.label}
              </button>
            </Tooltip>
          );
        })}
      </div>
      {state === "mixed" && (
        <span id={mixedId} className="sr-only">
          Mixed
        </span>
      )}
    </div>
  );
}

/**
 * The clone block of the Style tab (`specs/0049-linked-clone` criteria 22 and 56; UX notes "The
 * Style tab"): shown above the style sections when the selection holds only linked clones.
 * It names the original and offers Select original, Select root
 * original (chains only) and Unlink clone, then the Placement and Style rows and Spread colours
 * (`specs/0057-clone-overrides`). Inline, no popup.
 */
export function CloneBlock({ clones, onReturnFocus, onPressedAt }: CloneBlockProps) {
  const { panel } = clones;
  if (!panel.clonesOnly) {
    return null;
  }
  const many = panel.count > 1;
  const { title, chain } = cloneBlockLines(panel);
  return (
    <div role="group" aria-label="Linked clone" className="flex flex-col gap-2">
      <div className="flex items-start gap-1 text-sm text-[var(--toolbar-icon)]">
        <Link2 aria-hidden size={16} className="mt-0.5 shrink-0" />
        <span className="min-w-0 break-words">{title}</span>
      </div>
      {chain !== "" && <p className="text-xs text-[var(--panel-muted-fg)]">{chain}</p>}
      <div className="mt-2 flex flex-col gap-2">
        <BlockButton
          label={many ? "Select originals" : "Select original"}
          title="Selects the object this clone follows."
          onPress={(keyboard) => clones.apply("select-original", "panel", keyboard)}
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
          onHover={(on) => clones.hover(on ? "original" : "")}
        />
        {panel.deepest >= 2 && (
          <BlockButton
            label={many ? "Select root originals" : "Select root original"}
            title="Selects the path at the end of the chain."
            onPress={(keyboard) => clones.apply("select-root", "panel", keyboard)}
            onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
            onHover={(on) => clones.hover(on ? "root" : "")}
          />
        )}
        <BlockButton
          label={many ? "Unlink clones" : "Unlink clone"}
          title="Turns the clone into an ordinary path. Shape and style stay."
          onPress={(keyboard) => clones.apply("unlink", "panel", keyboard)}
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
        />
      </div>
      <div className="mt-2 flex flex-col gap-2">
        <OverrideRow
          label="Placement"
          segments={PLACEMENT_SEGMENTS}
          state={panel.placement}
          onSet={(own, keyboard) =>
            clones.apply((own ? "own-placement" : "follow-placement") as CloneCommand, "panel", keyboard)
          }
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
        />
        <OverrideRow
          label="Style"
          segments={STYLE_SEGMENTS}
          state={panel.style}
          onSet={(own, keyboard) =>
            clones.apply((own ? "own-style" : "original-style") as CloneCommand, "panel", keyboard)
          }
          onReturnFocus={onReturnFocus}
          onPressedAt={onPressedAt}
        />
        <Tooltip side="left" content={panel.spreadReady ? SPREAD_COLOURS_TITLE : SPREAD_COLOURS_DISABLED_TITLE}>
          <button
            type="button"
            aria-disabled={!panel.spreadReady}
            className={`${OUTLINE_BUTTON} ${panel.spreadReady ? "" : "cursor-default opacity-50 hover:bg-transparent"}`}
            onClick={(event) => {
              if (!panel.spreadReady) {
                return;
              }
              onPressedAt(event.currentTarget.getBoundingClientRect());
              const keyboard = event.detail === 0;
              clones.apply("spread-colours", "panel", keyboard);
              if (!keyboard) {
                onReturnFocus();
              }
            }}
          >
            Spread colours
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

/** The Stroke, Markers and Fill headers of a selection of clones that show their original's
 * style (`0057` criterion 10, UX notes "Style sections for a clone"): collapsed and greyed, a
 * muted summary at the right, no control, not a Tab stop. */
export function GreyedStyleHeaders({ clones }: { clones: ClonesApi }) {
  const { panel } = clones;
  const swatch = (hex: string) =>
    hex === "" ? null : (
      <span
        aria-hidden
        className="inline-block h-3 w-3 rounded-[2px] border border-[var(--panel-muted-fg)]"
        style={{ background: hex }}
      />
    );
  const header = (title: string, summary: string, hex: string) => (
    <div role="heading" aria-level={3} className="flex h-7 items-center justify-between text-sm text-[var(--panel-muted-fg)]">
      <span>{title}</span>
      <span className="flex items-center gap-1 text-xs">
        {swatch(hex)}
        {summary}
      </span>
    </div>
  );
  // The order of the open sections: Stroke with its Markers rows, the divider, Fill. Markers only
  // while the open state would show its rows (with a stroke); the summary is then never empty.
  return (
    <div className="flex flex-col">
      {header("Stroke", panel.strokeSummary, panel.strokeHex)}
      {panel.markersSummary !== "" && header("Markers", panel.markersSummary, "")}
      <div
        aria-hidden
        className="my-1 h-px"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      {header("Fill", panel.fillHex === "" ? "None" : "", panel.fillHex)}
    </div>
  );
}

/** The origin line (criterion 23): Select clones, while a clone follows the selection. */
export function OriginLine({ clones, onReturnFocus, onPressedAt }: CloneBlockProps) {
  if (clones.panel.followers === 0) {
    return null;
  }
  return (
    <div className="flex flex-col gap-2">
      <BlockButton
        label={clones.panel.followers === 1 ? "Select clone" : "Select clones"}
        title="Selects every clone of this object."
        onPress={(keyboard) => clones.apply("select-clones", "panel", keyboard)}
        onReturnFocus={onReturnFocus}
        onPressedAt={onPressedAt}
      />
    </div>
  );
}

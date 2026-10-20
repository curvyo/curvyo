import { CircleAlert, Link, Unlink } from "lucide-react";
import { TextControls } from "@/components/TextBar";
import type { TextBarState } from "@/hooks/useText";
import { Toggle, Tooltip } from "radix-ui";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { ArcFieldGroup, CurveGroup, EllipseKindGroup } from "@/components/ArcCurveGroups";
import { FlipGroup } from "@/components/FlipGroup";
import { GuideButton } from "@/components/GuideButton";
import { RatioSlider } from "@/components/RatioSlider";
import { RATIO_SLIDER_NAMES } from "@/lib/sliderNames";
import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import type { SelectBarState } from "@/hooks/useEditorSession";
import type { CloneBarNotice } from "@/hooks/useClones";
import type { ArcBarApi } from "@/hooks/useArcBar";
import type { FlipApi } from "@/hooks/useFlip";
import type { GuideState } from "@/lib/guideText";
import { LINKED_CLONE_TOOLTIP } from "@/lib/cloneText";

export interface SelectToolbarProps {
  scaleStrokeWidth: boolean;
  onSetScaleStrokeWidth: (on: boolean) => void;
  scaleCornerRadius: boolean;
  onSetScaleCornerRadius: (on: boolean) => void;
  /** The "Link corners" switch (`specs/0013-rectangle-corner-radii/` criterion 2). */
  linkCorners: boolean;
  onSetLinkCorners: (on: boolean) => void;
  /** What the bar shows for the current selection (rule of
   * `specs/0009-unified-object-editing/` criteria 21, 21a, 22). */
  bar: SelectBarState;
  /** Enter in "Radius": `"committed"`, `"unchanged"`, `"invalid:number"` or
   * `"invalid:negative"`. */
  onSetRadius: (text: string) => string;
  onRemoveRounding: () => void;
  onSetPointCount: (count: number) => void;
  /** The ratio slider's live preview on every tick; nothing is written. */
  onPreviewRatio: (ratio: number) => void;
  /** One commit, when the slider drag or a key run ends. */
  onCommitRatio: () => void;
  /** A click on the slider or a typed value: set and commit in one go. */
  onSetRatio: (ratio: number) => void;
  /** Escape in the middle of a slider drag: the preview is dropped. */
  onCancelRatio: () => void;
  onConvertToPaths: () => void;
  /** The refusal of Object to path for texts with a missing face, 4 px under the button
   * (`specs/0074-text-objects`, UX notes §4). */
  objectToPathNotice?: string | null;
  /** The Text group (`specs/0074-text-objects` criterion 11): shown while the selection holds a
   * text, acting on the texts only. */
  textBar?: TextBarState;
  onShowFonts?: () => void;
  onSetTextSize?: (mm: number) => string | null;
  onPreviewTextSize?: (mm: number | null) => void;
  /** The Linked clone button is shown (`specs/0049-linked-clone` criterion 4). */
  linkedCloneShown: boolean;
  onLinkedClone: () => void;
  /** The notice of the last Linked clone, 4 px under the button (UX notes). */
  linkedCloneNotice: CloneBarNotice | null;
  /** The flip buttons act: something is selected (`specs/0056-mirror-and-negative-scale/`
   * criterion 1). */
  flipEnabled: boolean;
  flip: FlipApi;
  /** The Guide button (`specs/0046-document-shapes` criteria 9 and 10). */
  guideState: GuideState;
  onToggleGuide: () => void;
  /** The ellipse group and the Curve field (`specs/0021-ellipse-arcs-and-shaping/`). */
  arcBar: ArcBarApi;
  /** Returns the keyboard focus to the canvas (after Enter or Escape in a
   * field). */
  onReturnFocus: () => void;
}

const MIN_POINT_COUNT = 3;
const MAX_POINT_COUNT = 1024;

/** A kind group: its controls wrap inside it in a bar too narrow for all of them (the first row
 * keeps its y; every row is 36px: 28px controls and 4px above and below). */
const GROUP_CLASS = "flex flex-wrap content-start items-center gap-x-3 py-1";

const DIVIDER_STYLE = {
  background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
} as const;

/** Up to two decimals, no trailing zeros: "3.5", "12", "0.25". */
function formatMm(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** The divider before a kind group. It stays in the layout but is invisible
 * when its group starts a wrapped row (`data-row-start` on the group), so the
 * hiding cannot change the wrapping. */
function Divider() {
  return (
    <span
      aria-hidden
      className="mt-2 h-5 w-px shrink-0 group-data-[row-start]/row:invisible"
      style={DIVIDER_STYLE}
    />
  );
}

/** Marks every group of `container` that starts a wrapped row, so no divider
 * is drawn at the start of a row. Measured, not guessed: re-run after every
 * render and every resize. */
export function useRowStarts(container: React.RefObject<HTMLDivElement | null>) {
  useLayoutEffect(() => {
    const element = container.current;
    if (!element) {
      return undefined;
    }
    const mark = () => {
      let rowTop = -1;
      for (const child of Array.from(element.children) as HTMLElement[]) {
        const top = child.offsetTop;
        const startsRow: boolean = rowTop >= 0 && top > rowTop;
        if (startsRow) {
          child.dataset.rowStart = "";
        } else {
          delete child.dataset.rowStart;
        }
        if (rowTop < 0 || startsRow) {
          rowTop = top;
        }
      }
    };
    mark();
    const observer = new ResizeObserver(mark);
    observer.observe(element);
    return () => observer.disconnect();
  });
}

const BUTTON_CLASS =
  "h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface RadiusFieldProps {
  bar: SelectBarState;
  onSetRadius: (text: string) => string;
  onReturnFocus: () => void;
}

/**
 * The bar's "Radius" field (`specs/0009-unified-object-editing/` criterion 21a):
 * the effective radius of the selected rectangles, always enabled. Enter
 * commits once; Escape or a press elsewhere restores the shown value and
 * writes nothing (a typo never reaches a machine job by blur); an empty,
 * non-numeric or negative value marks the field invalid and writes nothing.
 * Mixed rectangles show it empty with the placeholder "Mixed"; a stored radius
 * larger than the rectangle allows shows the effective value with a muted
 * "limited" tag. The rules are Rust's; this holds only the text, the caret
 * and the invalid mark.
 */
function RadiusField({ bar, onSetRadius, onReturnFocus }: RadiusFieldProps) {
  const messageId = useId();
  const shown = bar.radiusMixed ? "" : formatMm(bar.radius);
  const [text, setText] = useState(shown);
  const [invalid, setInvalid] = useState<"number" | "negative" | null>(null);
  const editing = useRef(false);

  // Follow the document while the maker is not typing.
  useEffect(() => {
    if (!editing.current) {
      setText(shown);
      setInvalid(null);
    }
  }, [shown]);

  const submit = (input: HTMLInputElement) => {
    if (text === shown) {
      editing.current = false;
      input.blur();
      onReturnFocus();
      return;
    }
    const outcome = onSetRadius(text);
    if (outcome.startsWith("invalid:")) {
      setInvalid(outcome === "invalid:negative" ? "negative" : "number");
      input.select();
      return;
    }
    editing.current = false;
    input.blur();
    onReturnFocus();
  };

  const corners = bar.radiusCorners;
  const tooltip = bar.radiusLimited
    ? `Stored ${formatMm(bar.radiusStored)} mm, limited to ${formatMm(bar.radius)} mm by the size; enlarging brings it back.`
    : corners
      ? `Corner radius. Typing sets all four.\nTop-left ${formatMm(corners[0])}, top-right ${formatMm(corners[1])}, bottom-right ${formatMm(corners[2])}, bottom-left ${formatMm(corners[3])} mm.\nZoom in to change one corner.`
      : "Corner radius. The corner handles appear on the canvas when the rectangle is at least 72 px across on screen; zoom in or type a value.";

  return (
    <div className="relative flex items-center gap-1.5 text-[var(--toolbar-icon)]">
      <label htmlFor={`${messageId}-field`}>Radius</label>
      <Tooltip.Provider>
      <Tooltip.Root delayDuration={400}>
      <div className="relative w-24">
        <Tooltip.Trigger asChild>
        <input
          id={`${messageId}-field`}
          type="text"
          inputMode="decimal"
          autoComplete="off"
          spellCheck={false}
          aria-label="Corner radius"
          aria-invalid={invalid ? true : undefined}
          aria-describedby={invalid ? messageId : undefined}
          placeholder={bar.radiusMixed ? "Mixed" : undefined}
          value={text}
          onFocus={(event) => {
            editing.current = true;
            event.currentTarget.select();
          }}
          onChange={(event) => {
            setText(event.target.value);
            setInvalid(null);
          }}
          onBlur={() => {
            editing.current = false;
            setText(shown);
            setInvalid(null);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              submit(event.currentTarget);
            } else if (event.key === "Escape") {
              event.preventDefault();
              editing.current = false;
              setText(shown);
              setInvalid(null);
              event.currentTarget.blur();
              onReturnFocus();
            }
          }}
          className={`h-7 w-full rounded-[5px] border bg-white pl-1.5 text-right text-sm tabular-nums outline-none ${
            bar.radiusLimited ? "pr-[74px]" : "pr-8"
          } ${
            invalid
              ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_2px_var(--field-invalid)]"
              : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
          }`}
          style={{ color: "var(--toolbar-icon)" }}
        />
        </Tooltip.Trigger>
        {bar.radiusLimited && (
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 right-[30px] -translate-y-1/2 text-[12px] opacity-60"
          >
            limited
          </span>
        )}
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
          style={{ opacity: 0.7 }}
        >
          mm
        </span>
      </div>
      <Tooltip.Portal>
        <Tooltip.Content
          side="bottom"
          sideOffset={6}
          className="z-50 max-w-[300px] rounded-md bg-popover px-2 py-1 text-xs text-popover-foreground ring-1 ring-foreground/10"
        >
          {tooltip.split("\n").map((line) => (
            <div key={line}>{line}</div>
          ))}
        </Tooltip.Content>
      </Tooltip.Portal>
      </Tooltip.Root>
      </Tooltip.Provider>
      <div
        id={messageId}
        role="status"
        aria-live="polite"
        className={
          invalid
            ? "absolute top-full left-0 z-30 mt-1 flex w-max items-center gap-1 rounded-[8px] px-1.5 py-1 text-xs whitespace-nowrap"
            : "absolute h-0 overflow-hidden text-xs"
        }
        style={
          invalid
            ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }
            : undefined
        }
      >
        {invalid && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>{invalid === "negative" ? "Must be 0 or more" : "Enter a number"}</span>
          </>
        )}
      </div>
    </div>
  );
}

interface PointsFieldProps {
  bar: SelectBarState;
  onSetPointCount: (count: number) => void;
}

/** "Points" (3 to 1024): a number field whose every valid value, a typed
 * one or a stepper click, is one commit. Empty with "Mixed" for polygons and
 * stars that differ; a typed value applies to all of them. */
function PointsField({ bar, onSetPointCount }: PointsFieldProps) {
  const shown = bar.pointsMixed ? "" : String(bar.points);
  const [text, setText] = useState(shown);
  const editing = useRef(false);
  useEffect(() => {
    if (!editing.current) {
      setText(shown);
    }
  }, [shown]);

  /** The text as a point count, or `null` while it is not a valid one. */
  const parse = (value: string): number | null => {
    const next = Math.round(Number(value));
    return value !== "" &&
      Number.isFinite(next) &&
      next >= MIN_POINT_COUNT &&
      next <= MAX_POINT_COUNT
      ? next
      : null;
  };

  /** Typed text is one commit on Enter or blur; nothing is written for an
   * invalid value, and an unchanged one writes nothing either. */
  const commitTyped = () => {
    const next = parse(text);
    if (next !== null && String(next) !== shown) {
      onSetPointCount(next);
    }
  };

  return (
    <label
      className="flex items-center gap-1.5 text-[var(--toolbar-icon)]"
      title="Points of the selected polygons and stars"
    >
      Points
      <input
        type="number"
        min={MIN_POINT_COUNT}
        max={MAX_POINT_COUNT}
        step={1}
        aria-label="Points"
        placeholder={bar.pointsMixed ? "Mixed" : undefined}
        value={text}
        onFocus={() => {
          editing.current = true;
        }}
        onBlur={() => {
          editing.current = false;
          commitTyped();
          setText(shown);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commitTyped();
            event.currentTarget.blur();
          }
        }}
        onChange={(event) => {
          setText(event.target.value);
          // A stepper click or an arrow key fires an input event without a
          // text-editing input type: one commit each. Typing waits for Enter
          // or blur, so "1024" never writes 10 and 102 on the way.
          const inputType = (event.nativeEvent as InputEvent).inputType ?? "";
          const typed = /^(insert(?!Replacement)|delete)/.test(inputType);
          const next = parse(event.target.value);
          if (!typed && next !== null) {
            onSetPointCount(next);
          }
        }}
        className="h-7 w-20 rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] bg-white px-1.5 text-right text-sm tabular-nums outline-none focus:ring-2 focus:ring-[var(--editor-accent)]"
        style={{ color: "var(--toolbar-icon)" }}
      />
    </label>
  );
}

interface LinkCornersToggleProps {
  linked: boolean;
  onLinkedChange: (on: boolean) => void;
}

/**
 * The "Link corners" toggle (`specs/0013-rectangle-corner-radii/` criteria 2 and 3;
 * `docs/design-system.md`, row "Link corners toggle"): a 28 px icon button, no
 * label, between the Radius field and "Remove rounding". On (the default) the
 * glyph is a closed chain on the active fill, off a broken chain on no fill, so
 * the state never rests on colour alone. A Radix `Toggle` (`aria-pressed`,
 * Space and Enter toggle), named "Link corners". The tooltip follows the state.
 * The state is owned by `useEditorSession` (session state, on per session and
 * after New or Open, never saved), not here.
 */
function LinkCornersToggle({ linked, onLinkedChange }: LinkCornersToggleProps) {
  return (
    <Tooltip.Provider>
      <Tooltip.Root delayDuration={400}>
        <Tooltip.Trigger asChild>
          <Toggle.Root
            pressed={linked}
            onPressedChange={onLinkedChange}
            aria-label="Link corners"
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
          >
            {linked ? (
              <Link aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
            ) : (
              <Unlink aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
            )}
          </Toggle.Root>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className="z-50 max-w-[280px] rounded-md bg-popover px-2 py-1 text-xs text-popover-foreground ring-1 ring-foreground/10"
          >
            <div>
              {linked
                ? "Link corners: on. A corner handle sets all four radii. Hold Shift to change one corner."
                : "Link corners: off. A corner handle changes its own corner. Hold Shift to set all four."}
            </div>
            <div className="opacity-70">
              Applies to the corner handles of one selected rectangle. The Radius field always sets all
              four.
            </div>
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}

/** The Linked clone button with its three-line tooltip below (UX notes). */
function LinkedCloneButton({
  onLinkedClone,
  onReturnFocus,
  notice,
}: {
  onLinkedClone: () => void;
  onReturnFocus: () => void;
  notice: CloneBarNotice | null;
}) {
  // Right-aligned to the button, but never left of the tool rail (`--rail-right` + 12) or off
  // the canvas: the shift that keeps it there (`0049` UX re-check).
  const noticeRef = useRef<HTMLDivElement>(null);
  const [shift, setShift] = useState(0);
  useLayoutEffect(() => {
    const element = noticeRef.current;
    if (!notice || !element) {
      setShift(0);
      return;
    }
    const rail = parseFloat(getComputedStyle(element).getPropertyValue("--rail-right")) || 0;
    const left = element.getBoundingClientRect().left - shift;
    setShift(Math.max(0, rail + 12 - left));
    // `shift` is read to undo the last one; the measure runs once per notice.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [notice]);
  return (
    <div className="relative">
    <Tooltip.Provider delayDuration={400}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            onClick={(event) => {
              onLinkedClone();
              if (event.detail > 0) {
                onReturnFocus();
              }
            }}
            className={`${BUTTON_CLASS} my-1`}
          >
            Linked clone
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className="z-50 rounded-md px-2 py-1 text-xs"
            style={{ background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" }}
          >
            {LINKED_CLONE_TOOLTIP.map((line) => (
              <div key={line}>{line}</div>
            ))}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
    {/* Takes no focus; a refusal is an alert (criteria 5 and 52). */}
    <div
      ref={noticeRef}
      role={notice?.alert ? "alert" : "status"}
      // At most 320 px wide, wrapping, right-aligned to the button (UX review must-fix 8).
      className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-md px-2 py-1 text-xs"
      style={
        notice
          ? {
              background: "var(--toolbar-bg)",
              color: "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
              transform: shift > 0 ? `translateX(${shift}px)` : undefined,
            }
          : { display: "none" }
      }
    >
      {notice?.text}
    </div>
    </div>
  );
}

/**
 * The Select tool's contextual bar (`docs/design-system.md`, "Select bar
 * layout"; `specs/0009-unified-object-editing/` criteria 21 to 23): left-aligned
 * after the tool rail so the switches never move. Left to right: the two
 * switches ("Scale stroke width", "Scale corner radius"), shown with or
 * without a selection and never disabled or dimmed; the Flip group, always shown and
 * disabled only while nothing is selected (`specs/0056-mirror-and-negative-scale/`
 * criterion 1); the kind groups, each
 * shown when the selection contains the kind it acts on (Radius and Remove
 * rounding for a rectangle, Points for a polygon or star, Ratio for a star),
 * acting on exactly those objects; and "Object to path" last. Wraps by whole
 * groups when the canvas is narrower than the row.
 */
export function SelectToolbar({
  scaleStrokeWidth,
  onSetScaleStrokeWidth,
  scaleCornerRadius,
  onSetScaleCornerRadius,
  linkCorners,
  onSetLinkCorners,
  bar,
  onSetRadius,
  onRemoveRounding,
  onSetPointCount,
  onPreviewRatio,
  onCommitRatio,
  onSetRatio,
  onCancelRatio,
  onConvertToPaths,
  objectToPathNotice = null,
  textBar,
  onShowFonts,
  onSetTextSize,
  onPreviewTextSize,
  linkedCloneShown,
  onLinkedClone,
  linkedCloneNotice,
  flipEnabled,
  flip,
  guideState,
  onToggleGuide,
  arcBar,
  onReturnFocus,
}: SelectToolbarProps) {
  const barRef = useRef<HTMLDivElement>(null);
  useRowStarts(barRef);
  const groups: React.ReactNode[] = [];
  if (bar.radiusShown) {
    groups.push(
      <div key="rectangle" className={GROUP_CLASS}>
        <RadiusField bar={bar} onSetRadius={onSetRadius} onReturnFocus={onReturnFocus} />
        <LinkCornersToggle linked={linkCorners} onLinkedChange={onSetLinkCorners} />
        <button
          type="button"
          aria-disabled={!bar.removeRoundingEnabled}
          title="Remove rounding of the selected rectangles (all corners)"
          onClick={() => {
            if (bar.removeRoundingEnabled) {
              onRemoveRounding();
            }
          }}
          className={`${BUTTON_CLASS} ${bar.removeRoundingEnabled ? "" : "opacity-40"}`}
        >
          Remove rounding
        </button>
      </div>,
    );
  }
  if (bar.pointsShown || bar.ratioShown) {
    groups.push(
      <div key={"star"} className={GROUP_CLASS}>
        {bar.pointsShown && <PointsField bar={bar} onSetPointCount={onSetPointCount} />}
        {bar.ratioShown && (
          <div className="flex items-center gap-1.5 text-[var(--toolbar-icon)]">
            Ratio
            <RatioSlider
              label={RATIO_SLIDER_NAMES.selectedStars}
              value={bar.ratio}
              mixed={bar.ratioMixed}
              onPreview={onPreviewRatio}
              onRelease={onCommitRatio}
              onSet={onSetRatio}
              onCancel={onCancelRatio}
              onReturnFocus={onReturnFocus}
            />
          </div>
        )}
      </div>,
    );
  }
  // After the kind groups: the ellipse group, then the curve group (UX notes, "Select bar").
  if (arcBar.state.ellipseShown) {
    groups.push(<EllipseKindGroup key="ellipse" arc={arcBar} onReturnFocus={onReturnFocus} />);
    groups.push(<ArcFieldGroup key="arc-fields" arc={arcBar} onReturnFocus={onReturnFocus} />);
  }
  if (arcBar.state.curveShown && !arcBar.state.ellipseShown) {
    groups.push(<CurveGroup key="curve" arc={arcBar} onReturnFocus={onReturnFocus} />);
  }
  if (textBar?.shown && onShowFonts && onSetTextSize && onPreviewTextSize) {
    groups.push(
      <div key="text" role="group" aria-label="Text" className={GROUP_CLASS}>
        <TextControls
          bar={textBar}
          onShowFonts={onShowFonts}
          onSetSize={onSetTextSize}
          onPreviewSize={onPreviewTextSize}
          onReturnFocus={onReturnFocus}
        />
      </div>,
    );
  }
  if (bar.objectToPathShown) {
    groups.push(
      <div key="object-to-path" className="relative">
        <button
          type="button"
          title="Convert the selected shapes and texts to paths"
          onClick={onConvertToPaths}
          className={`${BUTTON_CLASS} my-1`}
        >
          Object to path
        </button>
        {/* Takes no focus; the refusal look of the design system ("Action notice"). */}
        <div
          role="alert"
          className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-md px-2 py-1 text-xs ring-1 ring-[var(--field-invalid)]"
          style={
            objectToPathNotice
              ? { background: "var(--popover)", color: "var(--field-invalid)" }
              : { display: "none" }
          }
        >
          {objectToPathNotice}
        </div>
      </div>,
    );
  }
  if (linkedCloneShown) {
    // Last, in a group of its own, so no earlier control moves (UX notes, "The button in the
    // Select bar"); no link glyph, which the Link corners toggle already uses.
    groups.push(
      <LinkedCloneButton
        key="linked-clone"
        onLinkedClone={onLinkedClone}
        onReturnFocus={onReturnFocus}
        notice={linkedCloneNotice}
      />,
    );
  }
  return (
    <div
      ref={barRef}
      data-context-bar=""
      // Every row is 36 px (28 px controls), so the switches, always on the
      // first row, sit at the same y whether the bar has one row or two. Groups are
      // top-aligned so a tall settings group does not move the others.
      className="pointer-events-auto flex min-w-0 flex-wrap items-start gap-x-3 gap-y-0 rounded-lg px-2 text-sm"
      style={{
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {/* The two switches wrap inside their group in a bar too narrow for both (the 356px row
       * of a window of 800px with the second rail column); the first row keeps its y. */}
      {/* With only linked clones selected the switches would do nothing: they leave the bar
       * (`specs/0049-linked-clone` criterion 18). */}
      {bar.scaleSwitchesShown && (
      <div className="flex min-h-9 flex-wrap content-start items-center gap-x-1 gap-y-0 py-1">
        <ToolbarSwitch
          label="Scale stroke width"
          shortLabel="Scale stroke"
          tooltip="Scale stroke width with the object. Off: a resize keeps the stroke thickness."
          checked={scaleStrokeWidth}
          onCheckedChange={onSetScaleStrokeWidth}
        />
        <ToolbarSwitch
          label="Scale corner radius"
          shortLabel="Scale corners"
          tooltip="Scale corner radius with the object. Off: a resize keeps the corner radius."
          checked={scaleCornerRadius}
          onCheckedChange={onSetScaleCornerRadius}
        />
      </div>
      )}
      {/* The Flip group: second, after the switches, in every selection state, so the first
       * row does not move with the selection (`specs/0056-mirror-and-negative-scale/` D7). */}
      <div className="group/row flex min-h-9 items-start gap-3">
        {bar.scaleSwitchesShown && <Divider />}
        {/* The Guide button directly after the Flip group, in every selection state, 4 px from
         * it like the two flip buttons from each other, so the row stays as narrow as it can
         * (`specs/0046-document-shapes` criterion 9, UX review 2). */}
        <div className="flex items-start gap-1">
          <FlipGroup enabled={flipEnabled} flip={flip} onReturnFocus={onReturnFocus} />
          <div className="py-1">
            <GuideButton state={guideState} onToggle={onToggleGuide} onReturnFocus={onReturnFocus} />
          </div>
        </div>
      </div>
      {groups.map((group, index) => (
        <div key={index} className="group/row flex min-h-9 items-start gap-3">
          <Divider />
          {group}
        </div>
      ))}
    </div>
  );
}

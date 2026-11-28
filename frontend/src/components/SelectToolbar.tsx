import { InlinePickList, type PickRow } from "@/components/InlinePickList";
import { CircleAlert, Link, Link2, MapPin, Scaling, Spline, Square, SquareRoundCorner, Unlink } from "lucide-react";
import { AlignmentGroup } from "@/components/AlignmentGroup";
import type { OrientationCode, WritingModeCode } from "@/lib/textWritingText";
import { StyleGroup } from "@/components/StyleGroup";
import { BoxFromPathButton } from "@/components/BoxFromPathButton";
import { BridgeGroup } from "@/components/BridgeGroup";
import { StrokeToPathButton } from "@/components/StrokeToPathButton";
import { TraceButton } from "@/components/TraceBar";
import { RecogniseTextButton } from "@/components/RecogniseBar";
import type { RecogniseBarNotice } from "@/hooks/useRecognise";
import type { StrokeToPathBarNotice } from "@/hooks/useStrokeToPath";
import { OnPathButton, type TextPathActions } from "@/components/TextPathGroups";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import type { WeightCode } from "@/lib/styleText";
import type { AlignCode } from "@/lib/textLayoutText";
import {
  BarGroup,
  TextExpandButton,
  type GlyphsButtonState,
  type Row2State,
  TextControls,
  TextRow2,
  TextRow2Groups,
} from "@/components/TextBar";
import type { TextBarState } from "@/hooks/useText";
import { shiftStyle, useNoticeShift } from "@/hooks/useNoticeShift";
import { Toggle, Tooltip } from "radix-ui";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import {
  ArcFieldGroup,
  CurveGroup,
  EllipseKindGroup,
  RandomizedGroup,
} from "@/components/ArcCurveGroups";
import { AlignBarGroup, DistributeBarGroup, GapBarGroup } from "@/components/AlignGroup";
import { GridBarGroup } from "@/components/GridGroup";
import { FlipGroup } from "@/components/FlipGroup";
import { GuideButton } from "@/components/GuideButton";
import { TileGroup } from "@/components/TileGroup";
import type { TileSlot } from "@/lib/tilingText";
import { RatioSlider } from "@/components/RatioSlider";
import { RATIO_SLIDER_NAMES } from "@/lib/sliderNames";
import { OperationKindGroup } from "@/components/OperationKindGroup";
import { enteredBarShown } from "@/lib/kindButtons";
import type { KindButton } from "@/lib/operationsView";
import type { SelectBarState } from "@/hooks/useEditorSession";
import type { CloneBarNotice } from "@/hooks/useClones";
import type { ArcBarApi } from "@/hooks/useArcBar";
import type { FlipApi } from "@/hooks/useFlip";
import type { GuideState } from "@/lib/guideText";
import type { AlignApi } from "@/hooks/useAlign";
import { LINKED_CLONE_TOOLTIP } from "@/lib/cloneText";
import { Button } from "@/components/ui/button";
import { buttonClass, GLYPH } from "@/components/ui/looks";
import { ToggleButton } from "@/components/ui/toggle-button";
import { TOOLTIP_CLASS } from "@/components/ui/tooltip";
import { NumberField } from "@/components/ui/number-field";
import { Tooltip as UiTooltip } from "@/components/ui/tooltip";
import { arrangeGroupsShown, isNarrowSelectBar, kindGroupShown } from "@/lib/selectBarGroups";

export interface SelectToolbarProps {
  /** The selection holds an object out of context: move commands only (`specs/0106`). */
  moveOnly?: boolean;
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
  /** The Glyphs button, on row 2 (`specs/0082-glyph-table`; `specs/0084-text-bar-second-row`). */
  glyphs?: GlyphsButtonState;
  /** Row 2 of the Text controls, open or closed (`0084` criterion 14). */
  textRow2?: Row2State;
  onShowFonts?: () => void;
  onSetTextSize?: (mm: number) => string | null;
  onPreviewTextSize?: (mm: number | null) => void;
  /** The Style group's presses (`specs/0078-font-style-buttons`); each returns its notice. */
  onPressTextWeight?: (code: WeightCode) => string | null;
  onPressTextItalic?: () => string | null;
  /** The Alignment group's press (`specs/0079-text-layout-controls` criterion 1). */
  onSetTextAlign?: (align: AlignCode) => void;
  /** The Writing mode and Glyph orientation presses (`0079` criterion 20). */
  onSetTextWritingMode?: (mode: WritingModeCode) => void;
  onSetTextOrientation?: (orientation: OrientationCode) => void;
  /** The On path button and groups (`specs/0080-text-on-path`). */
  textPath?: TextPathActions;
  /** The Stroke to path button (`specs/0085-stroke-to-path` criterion 1): shown whenever
   * something is selected, enabled when a selected object draws a stroke. */
  strokeToPath?: {
    shown: boolean;
    enabled: boolean;
    onPress: () => void;
    notice: StrokeToPathBarNotice | null;
  };
  /** The Box from path button (`specs/0149-path-box` UX notes §1.1): hidden unless one closed
   * outline is selected. */
  boxFromPath?: { shown: boolean; onPress: () => void; notice: { id: number; text: string } | null };
  /** The Linked clone button is shown (`specs/0049-linked-clone` criterion 4). */
  linkedCloneShown: boolean;
  onLinkedClone: () => void;
  /** The notice of the last Linked clone, 4 px under the button (UX notes). */
  linkedCloneNotice: CloneBarNotice | null;
  /** Place on… (`specs/0100` criterion 25): the laser pages; none hides the button. */
  placeOn?: { rows: PickRow[]; onPick: (id: string) => void };
  /** The Trace button: shown with only images selected (`specs/0025-raster-trace` criterion 1). */
  traceShown?: boolean;
  onTrace?: () => void;
  /** Recognise text (`specs/0088-trace-recognition` UX notes §1): shown while a closed path is
   * selected, or while the notice of its last run is shown. */
  recognise?: {
    shown: boolean;
    onEnter: (byKeyboard: boolean) => void;
    notice: RecogniseBarNotice | null;
  };
  /** The Bridge group (`specs/0156-fused-lettering`): shown on a laser page while the selection
   * allows Bridge. */
  bridge?: { shown: boolean; ready: boolean; onBridge: () => void };
  /** Edit width (`specs/0115` criterion 4), last in the bar; renders nothing when not offered. */
  widthEdit?: React.ReactNode;
  /** The flip buttons act: something is selected (`specs/0056-mirror-and-negative-scale/`
   * criterion 1). */
  flipEnabled: boolean;
  /** A whole test grid is selected: Flip and Guide are hidden (`specs/0028` UX notes §6.5). */
  gridCommandsHidden?: boolean;
  flip: FlipApi;
  /** The Guide button (`specs/0046-document-shapes` criteria 9 and 10). */
  guideState: GuideState;
  onToggleGuide: () => void;
  /** The Align and Distribute groups (`specs/0061-align-and-distribute/`
   * criterion 1): shown while something is selected, the distribute group
   * from two. */
  align?: AlignApi;
  /** The ellipse group and the Curve field (`specs/0021-ellipse-arcs-and-shaping/`). */
  arcBar: ArcBarApi;
  /** The Tile slot (`specs/0069-tiling` UX notes §1): Tile, or Edit tiling and Release tiling. */
  tile?: { slot: TileSlot; onEnter: (byKeyboard: boolean) => void; onRelease: () => void };
  /** The Operation group (`specs/0111-laser-operation-tools` criteria 1 and 11): the five kind
   * buttons after the switches on a laser page; `buttons` is `null` while they are hidden.
   * Inside an entered laser group (`entered`) the bar holds only this group. */
  operationKinds?: {
    buttons: readonly KindButton[] | null;
    entered: boolean;
    onPress: (kind: string, shift: boolean) => void;
    /** Escape on a kind button inside the entered group leaves it. */
    onLeaveGroup: () => void;
    /** The Tabs toggle (`specs/0139` UX 4.2); absent while hidden. */
    tabs?: { on: boolean; onToggle: (on: boolean) => void };
  };
  /** Returns the keyboard focus to the canvas (after Enter or Escape in a
   * field). */
  onReturnFocus: () => void;
}

const MIN_POINT_COUNT = 3;
const MAX_POINT_COUNT = 1024;

/** A kind group: its controls wrap inside it in a bar too narrow for all of them (the first row
 * keeps its y; every row is 36px: 28px controls and 4px above and below). */
const GROUP_CLASS = "flex min-w-0 flex-wrap content-start items-center gap-x-3 py-1";

const DIVIDER_STYLE = {
  background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)",
} as const;

/** Up to two decimals, no trailing zeros: "3.5", "12", "0.25". */
function formatMm(value: number): string {
  return String(Math.round(value * 100) / 100);
}

/** The divider before a kind group, drawn 13px left of its group (which is `relative`), in the
 * 25px gap, so it takes no width: hidden when its group starts a wrapped row
 * (`data-row-start` on the group), every line starts at the pill's padding, and the hiding cannot
 * change the wrapping (`specs/0084-text-bar-second-row` UX notes §2). */
export function Divider() {
  return (
    <span
      aria-hidden
      className="pointer-events-none absolute top-2 -left-[13px] h-5 w-px group-data-[row-start]/row:invisible"
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
        // A hidden (empty) group starts no row.
        if (child.getClientRects().length === 0) {
          delete child.dataset.rowStart;
          continue;
        }
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

const BUTTON_CLASS = buttonClass("secondary", "bar");

/** The bar's side padding, px (`px-2` on each side). */
const BAR_PADDING_PX = 16;

/** Whether the bar whose pill is `pill` is in the narrow window class: measured on the bar area
 * (the pill's parent), the room the bar may take, after every resize. */
function useNarrowBar(pill: React.RefObject<HTMLDivElement | null>): boolean {
  const [narrow, setNarrow] = useState(false);
  useLayoutEffect(() => {
    const area = pill.current?.parentElement;
    if (!area) {
      return undefined;
    }
    const measure = () => setNarrow(isNarrowSelectBar(area.clientWidth - BAR_PADDING_PX));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(area);
    return () => observer.disconnect();
    // After every render: the pill mounts late when the bar starts inside an entered laser group.
  });
  return narrow;
}

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
  const messageRef = useRef<HTMLDivElement>(null);
  const messageShift = useNoticeShift(messageRef, invalid);
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
      <Tooltip.Trigger asChild>
      <div className="relative w-24">
        <NumberField
          unit="mm"
          limited={bar.radiusLimited}
          invalid={invalid !== null}
          id={`${messageId}-field`}
          aria-label="Corner radius"
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
        />
      </div>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          side="bottom"
          sideOffset={6}
          className={TOOLTIP_CLASS}
        >
          {tooltip.split("\n").map((line) => (
            <div key={line}>{line}</div>
          ))}
        </Tooltip.Content>
      </Tooltip.Portal>
      </Tooltip.Root>
      </Tooltip.Provider>
      <div
        ref={messageRef}
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
            ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)", ...shiftStyle(messageShift) }
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
    <UiTooltip side="bottom" content="Points of the selected polygons and stars">
      <label
      className="flex items-center gap-1.5 text-[var(--toolbar-icon)]"
    >
      Points
      <NumberField
        numeric
        width={80}
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
      />
    </label>
    </UiTooltip>
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
            className="flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-pressed:hover:bg-[var(--toolbar-icon-active-bg)]"
          >
            {linked ? (
              <Link {...GLYPH} aria-hidden />
            ) : (
              <Unlink {...GLYPH} aria-hidden />
            )}
          </Toggle.Root>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className={TOOLTIP_CLASS}
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

/** The Convert group and the Linked clone button as one wrap unit of the bar, so Linked clone
 * never sits alone on a trailing row; only a bar narrower than both splits them, by group, and
 * the divider between them is hidden when Linked clone starts a line (`0061` UI review item 4). */
function ConvertAndLinkedClone({ convert, linkedClone }: { convert: React.ReactNode; linkedClone: React.ReactNode }) {
  const unitRef = useRef<HTMLDivElement>(null);
  useRowStarts(unitRef);
  return (
    <div ref={unitRef} className="flex flex-wrap items-start gap-x-[25px]">
      {convert}
      <div className="group/clone relative">
        <span
          aria-hidden
          className="pointer-events-none absolute top-2 -left-[13px] h-5 w-px group-data-[row-start]/clone:invisible"
          style={DIVIDER_STYLE}
        />
        {linkedClone}
      </div>
    </div>
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
  const shift = useNoticeShift(noticeRef, notice);
  return (
    <div className="relative">
    <Tooltip.Provider delayDuration={400}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <Button size="bar" icon={Link2} iconOnly
            onClick={(event) => {
              onLinkedClone();
              if (event.detail > 0) {
                onReturnFocus();
              }
            }} className="my-1"
          >
            Linked clone
          </Button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            side="bottom"
            sideOffset={6}
            className={TOOLTIP_CLASS}
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
      className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-[8px] px-2 py-1 text-xs"
      style={
        notice
          ? {
              background: "var(--toolbar-bg)",
              color: "var(--toolbar-icon)",
              boxShadow: "var(--panel-elevation-shadow)",
              ...shiftStyle(shift),
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
  moveOnly = false,
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
  glyphs,
  textRow2,
  onShowFonts,
  onSetTextSize,
  onPreviewTextSize,
  onPressTextWeight,
  onPressTextItalic,
  onSetTextAlign,
  onSetTextWritingMode,
  onSetTextOrientation,
  textPath,
  strokeToPath,
  boxFromPath,
  linkedCloneShown,
  onLinkedClone,
  placeOn,
  linkedCloneNotice,
  traceShown = false,
  onTrace,
  recognise,
  bridge,
  widthEdit,
  tile,
  flipEnabled,
  gridCommandsHidden = false,
  flip,
  align,
  guideState,
  onToggleGuide,
  arcBar,
  operationKinds,
  onReturnFocus,
}: SelectToolbarProps) {
  const barRef = useRef<HTMLDivElement>(null);
  useRowStarts(barRef);
  const pillRef = useRef<HTMLDivElement>(null);
  const narrow = useNarrowBar(pillRef);
  // A kind group only while every selected object is of its kind (Frage 433 B).
  const only = (group: Parameters<typeof kindGroupShown>[0]) => kindGroupShown(group, bar.kinds);
  const objectToPathRef = useRef<HTMLDivElement>(null);
  const objectToPathShift = useNoticeShift(objectToPathRef, objectToPathNotice);
  const groups: React.ReactNode[] = [];
  // A move-only selection (`specs/0106` criterion 11, UX notes §3): the move groups stay in their
  // places, every edit control is hidden, not dimmed.
  const edits = !moveOnly;
  // The arrange groups directly after Guide, before the kind groups, so they do not move with
  // the selection; Grid from two selected objects (`specs/0061-align-and-distribute/` UX notes,
  // "Placement" and "Grid group"); in the narrow window class Align and Distribute from two too
  // (`specs/0159-ui-consistency`, Frage 433 B). Align is two groups of three, so the bar wraps
  // between the axes.
  const arrange = arrangeGroupsShown(align?.selectedCount ?? 0, narrow);
  if (align?.shown) {
    if (arrange.align) {
      groups.push(<AlignBarGroup key="align-h" axis="horizontal" align={align} onReturnFocus={onReturnFocus} />);
      groups.push(<AlignBarGroup key="align-v" axis="vertical" align={align} onReturnFocus={onReturnFocus} />);
    }
    if (arrange.distribute) {
      groups.push(<DistributeBarGroup key="distribute" align={align} onReturnFocus={onReturnFocus} />);
      // The one Gap field, between Distribute and Grid, which both use.
      groups.push(<GapBarGroup key="gap" align={align} />);
    }
    if (align.grid.shown && arrange.grid) {
      groups.push(<GridBarGroup key="grid" align={align} onReturnFocus={onReturnFocus} />);
    }
  }
  // The Tile slot after the arrange groups, before the kind groups, in every selection state of
  // the Select tool, so the bar does not jump (`specs/0069-tiling` UX notes §1).
  if (tile && edits) {
    groups.push(<TileGroup key="tile" slot={tile.slot} onEnter={tile.onEnter} onRelease={tile.onRelease} onReturnFocus={onReturnFocus} />);
  }
  if (bar.radiusShown && edits && only("rectangle")) {
    groups.push(
      <div key="rectangle" className={GROUP_CLASS}>
        <RadiusField bar={bar} onSetRadius={onSetRadius} onReturnFocus={onReturnFocus} />
        <LinkCornersToggle linked={linkCorners} onLinkedChange={onSetLinkCorners} />
        <Button size="bar" icon={Square} iconOnly
          aria-disabled={!bar.removeRoundingEnabled}
          tooltip="Remove rounding: the selected rectangles get sharp corners"
          onClick={() => {
            if (bar.removeRoundingEnabled) {
              onRemoveRounding();
            }
          }}
        >
          Remove rounding
        </Button>
      </div>,
    );
  }
  const pointsShown = bar.pointsShown && only("points");
  const ratioShown = bar.ratioShown && only("ratio");
  if ((pointsShown || ratioShown) && edits) {
    groups.push(
      <div key={"star"} className={GROUP_CLASS}>
        {pointsShown && <PointsField bar={bar} onSetPointCount={onSetPointCount} />}
        {ratioShown && (
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
  const ellipseShown = arcBar.state.ellipseShown && only("ellipse");
  if (ellipseShown && edits) {
    groups.push(<EllipseKindGroup key="ellipse" arc={arcBar} onReturnFocus={onReturnFocus} />);
    groups.push(<ArcFieldGroup key="arc-fields" arc={arcBar} onReturnFocus={onReturnFocus} />);
  }
  if ((arcBar.state.curveShown || arcBar.state.roundedShown) && !ellipseShown && only("curve") && edits) {
    groups.push(<CurveGroup key="curve" arc={arcBar} onReturnFocus={onReturnFocus} />);
  }
  // Directly after the Curve group: Randomized and "Re-roll seed", on polygons and stars only.
  if (arcBar.state.randomizedShown && only("randomized") && edits) {
    groups.push(<RandomizedGroup key="randomized" arc={arcBar} onReturnFocus={onReturnFocus} />);
  }
  // The Text controls' row 1 in place, in the Text bar's order (`specs/0084-text-bar-second-row`
  // criteria 6 and 8): Font and Size, Alignment, Style, the expand button. Row 2 is the bottom
  // line of the pill, under every other row.
  const textShown = Boolean(edits && textBar?.shown && only("text") && onShowFonts && onSetTextSize && onPreviewTextSize);
  const row2Id = useId();
  if (textBar && textShown && onShowFonts && onSetTextSize && onPreviewTextSize) {
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
    if (onSetTextAlign) {
      groups.push(
        <div key="text-align" className={GROUP_CLASS}>
          <AlignmentGroup align={textBar.align} onPress={onSetTextAlign} onReturnFocus={onReturnFocus} />
        </div>,
      );
    }
    if (onPressTextWeight && onPressTextItalic) {
      groups.push(
        <div key="text-style" className={GROUP_CLASS}>
          <StyleGroup
            style={textBar.style}
            editing={false}
            onPressWeight={onPressTextWeight}
            onPressItalic={onPressTextItalic}
            onReturnFocus={onReturnFocus}
          />
        </div>,
      );
    }
    if (textPath) {
      groups.push(
        <div key="text-on-path" className={GROUP_CLASS}>
          <OnPathButton path={textPath} onReturnFocus={onReturnFocus} />
        </div>,
      );
    }
    if (textRow2) {
      groups.push(
        <div key="text-expand" className={GROUP_CLASS}>
          <TextExpandButton row2={textRow2} controls={row2Id} bar={textBar} onReturnFocus={onReturnFocus} />
        </div>,
      );
    }
  }
  // Object to path and Stroke to path are one group "Convert", 4 px apart, so they wrap together
  // (`specs/0085-stroke-to-path` UX notes, "Button").
  const strokeToPathShown = Boolean(edits && strokeToPath?.shown);
  const linkedClone = linkedCloneShown ? (
    <div key="linked-clone" className="flex items-start gap-1">
      <LinkedCloneButton onLinkedClone={onLinkedClone} onReturnFocus={onReturnFocus} notice={linkedCloneNotice} />
      {placeOn && placeOn.rows.length > 0 && (
        // The list opens under the button, over the canvas, and closes on a choice or Escape.
        <div className="relative my-1 w-fit has-[[role=listbox]]:w-60">
          <InlinePickList
            icon={MapPin}
            label="Place on…"
            className={`${BUTTON_CLASS} flex items-center gap-1`}
            rows={placeOn.rows}
            onPick={placeOn.onPick}
          />
        </div>
      )}
    </div>
  ) : null;
  const objectToPathShown = edits && bar.objectToPathShown && only("objectToPath");
  if (objectToPathShown || strokeToPathShown) {
    const convert = (
      <div role="group" aria-label="Convert" className="flex items-start gap-1">
      {objectToPathShown && (
      <div className="relative">
        <Button size="bar" icon={Spline} iconOnly
          tooltip="Object to path: the selected shapes and texts become paths"
          onClick={onConvertToPaths} className="my-1"
        >
          Object to path
        </Button>
        {/* Takes no focus; the refusal look of the design system ("Action notice"). */}
        <div
          ref={objectToPathRef}
          role="alert"
          className="absolute top-full right-0 mt-1 w-max max-w-[320px] break-words rounded-[8px] px-2 py-1 text-xs ring-1 ring-[var(--field-invalid)]"
          style={
            objectToPathNotice
              ? { background: "var(--popover)", color: "var(--field-invalid)", ...shiftStyle(objectToPathShift) }
              : { display: "none" }
          }
        >
          {objectToPathNotice}
        </div>
      </div>
      )}
      {strokeToPath && strokeToPathShown && (
        <StrokeToPathButton
          enabled={strokeToPath.enabled}
          onPress={strokeToPath.onPress}
          onReturnFocus={onReturnFocus}
          notice={strokeToPath.notice}
        />
      )}
      </div>
    );
    // Linked clone wraps together with the Convert group, so it never sits alone on a trailing
    // row (`specs/0061-align-and-distribute/` UI review 2026-11-01, item 4).
    groups.push(
      linkedClone ? (
        <ConvertAndLinkedClone key="convert" convert={convert} linkedClone={linkedClone} />
      ) : (
        convert
      ),
    );
  } else if (linkedClone) {
    // Last, in a group of its own, so no earlier control moves (UX notes, "The button in the
    // Select bar"); no link glyph, which the Link corners toggle already uses.
    groups.push(linkedClone);
  }
  if (edits && boxFromPath && ((boxFromPath.shown && only("path")) || boxFromPath.notice)) {
    // Last in the bar but for the tool commands, in a group of its own, so no earlier control
    // moves (`0149` UX notes §1.1).
    groups.push(
      <BoxFromPathButton
        key="box-from-path"
        onPress={boxFromPath.onPress}
        onReturnFocus={onReturnFocus}
        notice={boxFromPath.notice}
      />,
    );
  }
  if (traceShown && onTrace && edits) {
    // Last, in a group of its own, so no earlier control moves (`0025` UX notes section 1).
    groups.push(<TraceButton key="trace" onTrace={onTrace} onReturnFocus={onReturnFocus} />);
  }
  if (recognise && ((recognise.shown && only("path")) || recognise.notice) && edits) {
    // Last, in a group of its own, so no earlier control moves (`0088` UX notes §1).
    groups.push(
      <RecogniseTextButton
        key="recognise-text"
        onEnter={recognise.onEnter}
        onReturnFocus={onReturnFocus}
        notice={recognise.notice}
      />,
    );
  }
  if (bridge?.shown && edits) {
    // Near the end, in a group of its own, so no earlier control moves.
    groups.push(<BridgeGroup key="bridge" ready={bridge.ready} onBridge={bridge.onBridge} onReturnFocus={onReturnFocus} />);
  }
  if (widthEdit) {
    // Last, in a group of its own, so no earlier control moves.
    groups.push(<div key="width-edit">{widthEdit}</div>);
  }
  const kindButtons = operationKinds && enteredBarShown(operationKinds.buttons) ? operationKinds.buttons : null;
  // Inside an entered laser group the bar holds only the Operation group, and is absent with no
  // member selected (`ux-notes.md` 4.2, M3).
  if (operationKinds?.entered) {
    if (!kindButtons) return null;
    return (
      <div
        data-context-bar=""
        className="pointer-events-auto flex min-w-0 flex-col rounded-[8px] px-2 text-sm"
        style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
      >
        <div className={BAR_ROW_CLASS}>
          <OperationKindGroup
            kinds={kindButtons}
            onPress={operationKinds.onPress}
            tabs={operationKinds.tabs}
            onLeaveGroup={() => {
              operationKinds.onLeaveGroup();
              onReturnFocus();
            }}
          />
        </div>
      </div>
    );
  }
  return (
    <div
      ref={pillRef}
      data-context-bar=""
      className="pointer-events-auto flex min-w-0 flex-col rounded-[8px] px-2 text-sm"
      style={{
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
    {/* Every row is 36 px (28 px controls), so the switches, always on the first row, sit at the
     * same y whether the bar has one row or two. */}
    <div ref={barRef} className={BAR_ROW_CLASS}>
      {/* The two switches wrap inside their group in a bar too narrow for both (the 356px row
       * of a window of 800px with the second rail column); the first row keeps its y. */}
      {/* With only linked clones selected the switches would do nothing: they leave the bar
       * (`specs/0049-linked-clone` criterion 18). */}
      {bar.scaleSwitchesShown && (
      <div className="flex min-h-9 flex-wrap content-start items-center gap-x-1 gap-y-0 py-1">
        <ToggleButton icon={Scaling} look="icon"
          label="Scale stroke width"
          tooltip="Scale stroke width with the object. Off: a resize keeps the stroke thickness."
          pressed={scaleStrokeWidth}
          onPressedChange={onSetScaleStrokeWidth}
        />
        <ToggleButton icon={SquareRoundCorner} look="icon"
          label="Scale corner radius"
          tooltip="Scale corner radius with the object. Off: a resize keeps the corner radius."
          pressed={scaleCornerRadius}
          onPressedChange={onSetScaleCornerRadius}
        />
      </div>
      )}
      {/* The Operation group: right after the switches, before Flip, on a laser page with an
       * in-context selection (`specs/0111` criterion 1, `ux-notes.md` 3.1, T1). */}
      {kindButtons && operationKinds ? (
        <div className="group/row relative flex min-h-9 items-start">
          {bar.scaleSwitchesShown && <Divider />}
          <OperationKindGroup kinds={kindButtons} onPress={operationKinds.onPress} tabs={operationKinds.tabs} />
        </div>
      ) : null}
      {/* The Flip group: second, after the switches, in every selection state, so the first
       * row does not move with the selection (`specs/0056-mirror-and-negative-scale/` D7). */}
      <div className="group/row relative flex min-h-9 items-start">
        {(bar.scaleSwitchesShown || kindButtons) && <Divider />}
        {/* The Guide button directly after the Flip group, in every selection state, 4 px from
         * it like the two flip buttons from each other, so the row stays as narrow as it can
         * (`specs/0046-document-shapes` criterion 9, UX review 2). */}
        {/* A whole test grid hides both: its shape comes from its fields (`specs/0028` UX notes
         * §6.5). */}
        {gridCommandsHidden ? null : (
          <div className="flex items-start gap-1">
            <FlipGroup enabled={flipEnabled} flip={flip} onReturnFocus={onReturnFocus} />
            {edits ? (
              <div className="py-1">
                <GuideButton state={guideState} onToggle={onToggleGuide} onReturnFocus={onReturnFocus} />
              </div>
            ) : null}
          </div>
        )}
      </div>
      {groups.map((group, index) => (
        <BarGroup key={index} className="flex min-h-9 max-w-full min-w-0 items-start">
          {group}
        </BarGroup>
      ))}
    </div>
    {textBar && textShown && textRow2?.open && onSetTextWritingMode && onSetTextOrientation ? (
      <TextRow2 id={row2Id} role="group">
        <TextRow2Groups
          bar={textBar}
          glyphs={glyphs}
          path={textPath}
          onSetWritingMode={onSetTextWritingMode}
          onSetOrientation={onSetTextOrientation}
          onReturnFocus={onReturnFocus}
        />
      </TextRow2>
    ) : null}
    </div>
  );
}

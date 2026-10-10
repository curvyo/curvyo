import { CircleAlert } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";

import { HandleHintChip } from "@/components/HandleHintChip";
import { type MoveEntryState, REFERENCE_COUNT } from "@/hooks/useEditorSession";
import { placeMoveEntryChip } from "@/lib/readoutPlacement";

/** The tool rail's clearance from the canvas's left edge, px (`App.tsx`'s
 * `left-[calc(var(--rail-right)+12px)]` for the contextual bars: `--rail-right` + 12 = 128px with two
 * rail columns). */
const TOOL_RAIL_CLEAR_PX = 128;

/** Height of the message card plus its gap, in px. */
const MESSAGE_CARD_PX = 28;

/** What "Absolute" measures from (`specs/0047-transform-polish/` criterion 42): the title and the
 * accessible description of the segment. It names no colour. */
const ABSOLUTE_HINT =
  "Position of the chosen point of the selection's box. Enter uses the chosen point, the centre until you choose another. Click a marked point on the canvas to use it.";

/** The two lines under the name of a mark in its hint chip (criterion 43). */
const MARK_CLICK_LINE = "Click: move it to the typed X, Y";
const MARK_NOTHING_LINE = "Click: nothing to apply, type X and Y first";

/** The Reference control's room: a 76 px grid and the 6 px above it. The chip flips by its
 * height with this row, so the mode never changes the side it opens on. */
const REFERENCE_ROW_PX = 82;

/** The labels and the accessible names of the two fields, per mode
 * (`edit-interaction-polish` criterion 18). */
const FIELDS = [
  { label: "X", relative: "Horizontal offset", absolute: "X position" },
  { label: "Y", relative: "Vertical offset", absolute: "Y position" },
] as const;

interface MoveEntryChipProps {
  entry: MoveEntryState;
  /** The canvas container: the chip's coordinate space, and where focus
   * returns to. */
  containerRef: React.RefObject<HTMLDivElement | null>;
  /** Enter: `"committed"`, `"unchanged"` or `"invalid:<field>:number"`.
   * `copy` is the state of the Copy check. */
  onCommit: (first: string, second: string, absolute: boolean, copy: boolean) => string;
  onCancel: () => void;
  /** The chip's mode, reported to Rust: the marks exist in Absolute only. */
  onMode: (absolute: boolean) => void;
  /** The reference point the Reference control chose (0 to 8). */
  onReference: (reference: number) => void;
  /** The radio with the pointer or the focus (0 to 8), or -1. */
  onRadioHover: (reference: number) => void;
  /** A click on a mark: Enter with that mark's point as the reference. */
  onCommitAt: (first: string, second: string, copy: boolean, reference: number) => string;
  /** The mark under the pointer (0 to 8), or -1. */
  markHover: number;
  /** Registers what a click on a mark does while the chip is open. */
  onMarkClick: (handler: ((mark: number) => void) | null) => void;
}

/**
 * The typed move chip (`specs/0010-edit-interaction-polish/specification.md`
 * criteria 15 to 25; `specs/0047-transform-polish/` Part C; `docs/design-system.md`,
 * "Move entry chip"): two fields X and Y and a Relative | Absolute switch, in Absolute a
 * Reference control of nine points, opened on the object by a double-click on the centre
 * handle or by the key M. It holds only the text, the mode, which fields the maker has edited
 * and the focus: parsing, the two readings, the tight bounds, the reference points and the
 * limits are Rust's (`curvyo-ui-core::move_entry`, `reference_point`). It opens in Relative every
 * time, with the centre as its reference and the Copy check off unless Ctrl was held at the
 * second press of the double-click (criterion 23; the key M cannot carry a Ctrl). A field the
 * maker has not edited shows the prefill of the current mode and reference (the point under the
 * pointer while it is on a mark) and means "no change on that axis"; an edited field keeps its
 * text across a mode switch and is read in the mode current at Enter. Enter in any control
 * applies, with the chosen point; a click on a mark on the canvas applies with that mark's point.
 * Escape, a press elsewhere, a tool switch or a window blur cancels and writes nothing.
 */
export function MoveEntryChip({
  entry,
  containerRef,
  onCommit,
  onCancel,
  onMode,
  onReference,
  onRadioHover,
  onCommitAt,
  markHover,
  onMarkClick,
}: MoveEntryChipProps) {
  const chipRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const inputRefs = useRef<Array<HTMLInputElement | null>>([]);
  const switchRef = useRef<HTMLButtonElement>(null);
  const copyRef = useRef<HTMLInputElement>(null);
  const radioRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const messageId = useId();
  const [absolute, setAbsolute] = useState(false);
  const [copy, setCopy] = useState(entry.copyPreset);
  const [edited, setEdited] = useState<[boolean, boolean]>([false, false]);
  const [typed, setTyped] = useState<[string, string]>(["", ""]);
  const [invalid, setInvalid] = useState<number | null>(null);
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });

  // The reference whose position an untouched field shows: the mark under the pointer, else the
  // chosen point (criterion 44).
  const shownReference = absolute && markHover >= 0 ? markHover : entry.reference;
  const prefill = (axis: 0 | 1, mode: boolean, reference: number) =>
    mode ? (entry.absolute[reference]?.[axis] ?? "") : entry.relative[axis];
  const textOfAt = (axis: 0 | 1, reference: number) =>
    edited[axis] ? typed[axis] : prefill(axis, absolute, reference);
  const textOf = (axis: 0 | 1) => textOfAt(axis, shownReference);

  // Rust draws the marks in Absolute only.
  useEffect(() => {
    onMode(absolute);
  }, [absolute, onMode]);

  // Focus X with its text selected, once per open.
  useEffect(() => {
    inputRefs.current[0]?.focus();
    inputRefs.current[0]?.select();
  }, []);

  // Measured before paint so the chip is placed correctly on its first frame.
  useLayoutEffect(() => {
    const chip = chipRef.current;
    const body = bodyRef.current;
    const container = containerRef.current;
    if (!chip || !body || !container) {
      return;
    }
    const next = {
      chip: { width: chip.offsetWidth, height: body.offsetHeight + 12 },
      canvas: { width: container.clientWidth, height: container.clientHeight },
    };
    setSizes((previous) =>
      previous.chip.width === next.chip.width &&
      previous.chip.height === next.chip.height &&
      previous.canvas.width === next.canvas.width &&
      previous.canvas.height === next.canvas.height
        ? previous
        : next,
    );
  }, [entry, invalid, absolute, containerRef]);

  const placed = placeMoveEntryChip(
    { center: entry.center, boxBottomY: entry.boxBottomY },
    sizes.chip,
    sizes.canvas,
    sizes.chip.height + (absolute ? 0 : REFERENCE_ROW_PX),
  );
  const placement = { ...placed, left: Math.max(placed.left, TOOL_RAIL_CLEAR_PX) };
  const roomBelow = sizes.canvas.height - (placement.top + sizes.chip.height) >= MESSAGE_CARD_PX;
  const messageLeft = placement.left + sizes.chip.width / 2 <= sizes.canvas.width / 2;

  const returnFocusToCanvas = () => containerRef.current?.focus();

  /** The outcome of an Enter or of a click on a mark: an invalid field stays open and marked. */
  const finish = (outcome: string) => {
    if (outcome.startsWith("invalid:")) {
      const field = Number(outcome.split(":")[1]);
      setInvalid(field);
      inputRefs.current[field]?.focus();
      inputRefs.current[field]?.select();
      return;
    }
    returnFocusToCanvas();
  };

  const submit = () => finish(onCommit(textOf(0), textOf(1), absolute, copy));

  // A click on a mark executes the move with that mark's point (criterion 40), with the fields
  // compared to the prefill of that point (criterion 44). Registered again after every render, so
  // the handler always reads the current text.
  useEffect(() => {
    onMarkClick((mark) => finish(onCommitAt(textOfAt(0, mark), textOfAt(1, mark), copy, mark)));
    return () => onMarkClick(null);
  });

  /** Tab order X, Y, the mode switch, the Reference control, the Copy check and back to X
   * (Shift+Tab reverses): the chip is a closed loop, so Tab never leaves it
   * and cancels it by a blur (`edit-interaction-polish` criteria 18, 23). The
   * two ends are handled here; the steps in between are the browser's own. */
  const loopFocus = (event: React.KeyboardEvent<HTMLElement>, first: boolean) => {
    if (event.key !== "Tab" || event.shiftKey !== first) {
      return false;
    }
    event.preventDefault();
    const target = first ? copyRef.current : inputRefs.current[0];
    target?.focus();
    if (target instanceof HTMLInputElement && target.type === "text") {
      target.select();
    }
    return true;
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLElement>) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
      returnFocusToCanvas();
    }
  };

  const onChange = (axis: 0 | 1, value: string) => {
    setEdited((previous) => (axis === 0 ? [true, previous[1]] : [previous[0], true]));
    setTyped((previous) => (axis === 0 ? [value, previous[1]] : [previous[0], value]));
    setInvalid(null);
  };

  const onSwitchKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === " " || event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      setAbsolute((previous) => !previous);
    } else {
      onKeyDown(event);
    }
  };

  // Cancel on a blur that leaves the chip (a press elsewhere, a tool switch,
  // the window losing focus). A press on the canvas is also processed by the
  // canvas itself, so it is not swallowed.
  const onBlur = (event: React.FocusEvent<HTMLElement>) => {
    const next = event.relatedTarget;
    if (next instanceof Node && chipRef.current?.contains(next)) {
      return;
    }
    onCancel();
  };

  /** The Reference control's keys (criterion 41): the arrows follow the layout and do not wrap,
   * Home and End are the first and last point, Space selects, Enter applies. */
  const onRadioKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    const column = index % 3;
    const row = Math.floor(index / 3);
    const target: Record<string, number | null> = {
      ArrowLeft: column > 0 ? index - 1 : null,
      ArrowRight: column < 2 ? index + 1 : null,
      ArrowUp: row > 0 ? index - 3 : null,
      ArrowDown: row < 2 ? index + 3 : null,
      Home: 0,
      End: REFERENCE_COUNT - 1,
    };
    if (event.key in target) {
      event.preventDefault();
      const next = target[event.key];
      if (next !== null && next !== undefined) {
        onReference(next);
        radioRefs.current[next]?.focus();
      }
    } else if (event.key === " ") {
      event.preventDefault();
      onReference(index);
    } else if (!loopFocus(event, false)) {
      onKeyDown(event);
    }
  };

  const hintLines = useMemo(
    () =>
      absolute && markHover >= 0
        ? [
            entry.referenceHints[markHover] ?? "",
            edited[0] || edited[1] ? MARK_CLICK_LINE : MARK_NOTHING_LINE,
          ]
        : [],
    [absolute, markHover, entry.referenceHints, edited],
  );

  const segment = (active: boolean) =>
    `flex-1 px-2 text-center text-xs leading-[24px] ${active ? "rounded-[5px]" : ""}`;

  return (
    <>
      <div
        ref={chipRef}
        role="group"
        aria-label="Move"
        tabIndex={-1}
        data-transform-entry
        className="absolute z-30 rounded-[8px] p-1.5 outline-none"
        style={{
          left: placement.left,
          top: placement.top,
          background: "var(--toolbar-bg)",
          boxShadow: "var(--panel-elevation-shadow)",
          color: "var(--toolbar-icon)",
        }}
      >
        <div ref={bodyRef}>
          <div className="flex items-center gap-1">
            {FIELDS.map((field, index) => {
              const axis = index as 0 | 1;
              const isInvalid = invalid === axis;
              return (
                <div key={field.label} className="relative" style={{ width: 100 }}>
                  <span
                    aria-hidden
                    className="pointer-events-none absolute top-1/2 left-1.5 -translate-y-1/2 text-xs"
                  >
                    {field.label}
                  </span>
                  <input
                    ref={(node) => {
                      inputRefs.current[axis] = node;
                    }}
                    type="text"
                    inputMode="decimal"
                    autoComplete="off"
                    spellCheck={false}
                    aria-label={absolute ? field.absolute : field.relative}
                    aria-invalid={isInvalid ? true : undefined}
                    aria-describedby={isInvalid ? messageId : undefined}
                    value={textOf(axis)}
                    onChange={(event) => onChange(axis, event.target.value)}
                    onKeyDown={(event) => {
                      if (!(axis === 0 && loopFocus(event, true))) {
                        onKeyDown(event);
                      }
                    }}
                    onBlur={onBlur}
                    className={`h-7 w-full rounded-[5px] border bg-white pr-8 pl-5 text-right text-sm tabular-nums outline-none ${
                      isInvalid
                        ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_1px_var(--field-invalid)]"
                        : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]"
                    }`}
                    style={{ color: "var(--toolbar-icon)" }}
                  />
                  <span
                    aria-hidden
                    className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-xs"
                    style={{ opacity: 0.7 }}
                  >
                    mm
                  </span>
                </div>
              );
            })}
          </div>
          <button
            ref={switchRef}
            type="button"
            role="switch"
            aria-checked={absolute}
            aria-label="Absolute position"
            aria-description={ABSOLUTE_HINT}
            title={ABSOLUTE_HINT}
            onClick={() => setAbsolute((previous) => !previous)}
            onKeyDown={onSwitchKeyDown}
            onBlur={onBlur}
            className="mt-1.5 flex h-7 w-full cursor-pointer rounded-[7px] p-[1.5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            // A 1.5 px ring as a shadow: a CSS border of 1.5 px computes to 1 px
            // at a device pixel ratio of 1, a shadow spread does not snap.
            style={{
              boxShadow: "inset 0 0 0 1.5px var(--toolbar-icon)",
              color: "var(--toolbar-icon)",
            }}
          >
            <span
              aria-hidden
              className={segment(!absolute)}
              style={
                absolute
                  ? undefined
                  : {
                      background: "var(--toolbar-icon-active-bg)",
                      color: "var(--toolbar-icon-active-fg)",
                    }
              }
            >
              Relative
            </span>
            <span
              aria-hidden
              className={segment(absolute)}
              style={
                absolute
                  ? {
                      background: "var(--toolbar-icon-active-bg)",
                      color: "var(--toolbar-icon-active-fg)",
                    }
                  : undefined
              }
            >
              Absolute
            </span>
          </button>
          {absolute && (
            <div className="mt-1.5 flex items-center gap-2">
              <div
                role="radiogroup"
                aria-label="Reference point"
                className="grid shrink-0 grid-cols-3 gap-0.5"
              >
                {Array.from({ length: REFERENCE_COUNT }, (_, index) => (
                  <ReferenceRadio
                    key={index}
                    index={index}
                    name={entry.referenceNames[index] ?? ""}
                    selected={entry.reference === index}
                    buttonRef={(node) => {
                      radioRefs.current[index] = node;
                    }}
                    onSelect={() => onReference(index)}
                    onHover={onRadioHover}
                    onKeyDown={onRadioKeyDown}
                    onBlur={onBlur}
                  />
                ))}
              </div>
              <span aria-hidden className="text-xs">
                Reference
              </span>
            </div>
          )}
          <label className="mt-1.5 flex h-6 cursor-pointer items-center gap-1.5 px-1 text-xs">
            <input
              ref={copyRef}
              type="checkbox"
              checked={copy}
              aria-label="Copy"
              onChange={(event) => setCopy(event.target.checked)}
              onKeyDown={(event) => {
                if (!loopFocus(event, false)) {
                  onKeyDown(event);
                }
              }}
              onBlur={onBlur}
              className="size-3.5 cursor-pointer accent-[var(--editor-accent)]"
            />
            <span aria-hidden>Copy</span>
          </label>
        </div>
        {/* Always mounted so the polite live region announces a change; empty it
            has no height. The message sits outside the body (the chip is placed
            by the body, so it never jumps), below it unless there is no room. */}
        <div
          id={messageId}
          role="status"
          aria-live="polite"
          className={
            invalid !== null
              ? `absolute ${messageLeft ? "left-0" : "right-0"} flex w-max min-w-full items-center gap-1 whitespace-nowrap rounded-[8px] px-1.5 py-1 text-xs ${
                  roomBelow ? "top-full mt-1" : "bottom-full mb-1"
                }`
              : "absolute h-0 overflow-hidden text-xs"
          }
          style={
            invalid !== null
              ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }
              : undefined
          }
        >
          {invalid !== null && (
            <>
              <CircleAlert aria-hidden className="size-3 shrink-0" />
              <span>Enter a number</span>
            </>
          )}
        </div>
      </div>
      {/* The name of the mark under the pointer after 600 ms, the hint chip of the handles. */}
      <HandleHintChip
        hint={hintLines.length > 0 ? "group" : ""}
        cornerLines={hintLines}
        containerRef={containerRef}
      />
    </>
  );
}

interface ReferenceRadioProps {
  index: number;
  name: string;
  selected: boolean;
  buttonRef: (node: HTMLButtonElement | null) => void;
  onSelect: () => void;
  onHover: (index: number) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => void;
  onBlur: (event: React.FocusEvent<HTMLElement>) => void;
}

/**
 * One radio of the Reference control (`specs/0047-transform-polish/` criterion 41): a 24 px
 * button with a 14 px square and a 4 px dot at the point it names. Roving tabindex (the chosen
 * point is the one Tab stop), `aria-posinset` and `aria-setsize`; the pointer or the focus on it
 * draws its mark in the hover state on the canvas.
 */
function ReferenceRadio({
  index,
  name,
  selected,
  buttonRef,
  onSelect,
  onHover,
  onKeyDown,
  onBlur,
}: ReferenceRadioProps) {
  // The dot at the column and row of the point inside the 14 px square (5 px in, 14 px across).
  const dotX = 5 + (index % 3) * 7;
  const dotY = 5 + Math.floor(index / 3) * 7;
  return (
    <button
      ref={buttonRef}
      type="button"
      role="radio"
      aria-checked={selected}
      aria-label={name}
      aria-posinset={index + 1}
      aria-setsize={REFERENCE_COUNT}
      tabIndex={selected ? 0 : -1}
      title={name}
      onClick={onSelect}
      onKeyDown={(event) => onKeyDown(event, index)}
      onPointerEnter={() => onHover(index)}
      onPointerLeave={() => onHover(-1)}
      onFocus={() => onHover(index)}
      onBlur={(event) => {
        onHover(-1);
        onBlur(event);
      }}
      className="flex size-6 cursor-pointer items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      style={
        selected
          ? {
              background: "var(--toolbar-icon-active-bg)",
              color: "var(--toolbar-icon-active-fg)",
            }
          : { color: "var(--toolbar-icon)" }
      }
    >
      <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden>
        <rect x="5" y="5" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1" />
        <circle cx={dotX} cy={dotY} r="2" fill="currentColor" />
      </svg>
    </button>
  );
}

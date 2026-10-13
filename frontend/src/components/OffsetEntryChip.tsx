import { CircleAlert, Plus, RefreshCw } from "lucide-react";
import { useId, useLayoutEffect, useRef, useState } from "react";

import { JoinIcon } from "@/components/panel/StyleIcons";
import type { OffsetApi, OffsetChipAnchor } from "@/hooks/useOffset";
import {
  JOIN_NAMES,
  type OffsetDirectionName,
  type OffsetJoinName,
  type OffsetResultName,
  chipDistanceText,
  clampChipDistance,
  parseChipDistance,
  stepChipDistance,
} from "@/lib/offsetText";
import { placeOffsetEntryChip } from "@/lib/readoutPlacement";

/** The tool rail's clearance from the canvas's left edge, px (`--rail-right` + 12, as the Move
 * chip). */
const TOOL_RAIL_CLEAR_PX = 128;

/** Height of the message card plus its gap, in px. */
const MESSAGE_CARD_PX = 28;

const DIRECTIONS: RadioItem<OffsetDirectionName>[] = [
  { value: "out", label: "Out", title: "Outset: grow" },
  { value: "in", label: "In", title: "Inset: shrink" },
  { value: "both", label: "Both", title: "Both sides: one copy outside, one inside" },
];

const CORNERS: RadioItem<OffsetJoinName>[] = (["round", "miter", "bevel"] as const).map((join) => ({
  value: join,
  name: JOIN_NAMES[join],
  title: JOIN_NAMES[join],
  icon: <JoinIcon join={join} />,
}));

const MODES: RadioItem<OffsetResultName>[] = [
  {
    value: "new",
    name: "New object",
    title: "New object: the original stays",
    icon: <Plus aria-hidden className="size-4" strokeWidth={1.5} />,
  },
  {
    value: "change",
    name: "Change original",
    title: "Change original: no copy is made",
    icon: <RefreshCw aria-hidden className="size-4" strokeWidth={1.5} />,
  },
];

interface OffsetEntryChipProps {
  offset: OffsetApi;
  /** The canvas container: the chip's coordinate space, and where focus returns to. */
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The Offset entry chip (`specs/0038-path-offset` criteria 25, 53, 54; `ux-rework.md` section 2;
 * design-system row "Offset entry chip"): Distance, Direction, Corners and Mode, opened by a
 * double-click on the center handle or by O or Enter. Its values are the remembered ones: every
 * choice goes to the session at once and the standing preview follows; a typed Distance does so
 * when it is committed (Tab, an arrow step, Enter). Enter writes one commit, also with the
 * untouched text; Escape, a press elsewhere, a tool switch or a blur closes it without writing.
 * A refusal keeps it open with its sentence in the message card.
 */
export function OffsetEntryChip({ offset, containerRef }: OffsetEntryChipProps) {
  const { view } = offset;
  const chipRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const modeRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const closedRef = useRef(false);
  const messageId = useId();
  const [text, setText] = useState(() => chipDistanceText(view.valuesDistanceMm));
  const [message, setMessage] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);
  // Bumped to select Distance after React has rendered the text set with it (the clamped value).
  const [selectRequest, setSelectRequest] = useState(0);
  const fresh = offset.readAnchor();
  const [anchor, setAnchor] = useState<OffsetChipAnchor | null>(fresh);
  const [sizes, setSizes] = useState({
    chip: { width: 168, height: 108 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });

  // Fixed while open: a new value grows the preview but does not move the chip. Pan and zoom move
  // the center handle, and then the anchor is taken again (`ux-rework.md` 2.4).
  if (fresh && (!anchor || fresh.center.x !== anchor.center.x || fresh.center.y !== anchor.center.y)) {
    setAnchor(fresh);
  }

  // Focus in Distance with its text selected, once per open and after every invalid Enter, so
  // the next keystroke replaces the whole (clamped) text.
  useLayoutEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [selectRequest]);

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
  }, [anchor, message, containerRef]);

  if (!anchor) {
    return null;
  }
  const placement = placeOffsetEntryChip(anchor, sizes.chip, sizes.canvas, TOOL_RAIL_CLEAR_PX);
  const roomBelow = sizes.canvas.height - (placement.top + sizes.chip.height) >= MESSAGE_CARD_PX;

  /** Sends a valid typed Distance to the session; returns whether it was in range. */
  const commitText = (): boolean | null => {
    const value = parseChipDistance(text);
    if (Number.isNaN(value)) {
      return null;
    }
    const inRange = offset.setDistance(value);
    setText(chipDistanceText(clampChipDistance(value)));
    return inRange;
  };

  /** Closes without writing; a valid typed Distance stays remembered (criterion 54). */
  const close = (returnFocus: boolean) => {
    if (closedRef.current) {
      return;
    }
    closedRef.current = true;
    commitText();
    offset.closeChip();
    if (returnFocus) {
      containerRef.current?.focus();
    }
  };

  const markInvalid = (sentence: string) => {
    setInvalid(true);
    setMessage(sentence);
    setSelectRequest((n) => n + 1);
  };

  const submit = () => {
    const inRange = commitText();
    if (inRange === null) {
      markInvalid("Enter a number");
      return;
    }
    if (!inRange) {
      markInvalid("From 0.01 to 1000 mm");
      return;
    }
    const refusal = offset.apply();
    if (refusal !== null) {
      setMessage(refusal);
      return;
    }
    closedRef.current = true;
    containerRef.current?.focus();
  };

  /** Keys every control shares: Enter writes, Escape closes, Tab loops at the two ends. */
  const onKeyDown = (event: React.KeyboardEvent<HTMLElement>, stop: "first" | "last" | null) => {
    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      close(true);
    } else if (event.key === "Tab" && stop === "first" && event.shiftKey) {
      event.preventDefault();
      commitText();
      modeRefs.current[view.result === "change" ? 1 : 0]?.focus();
    } else if (event.key === "Tab" && stop === "last" && !event.shiftKey) {
      event.preventDefault();
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  };

  const onDistanceKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      const typed = parseChipDistance(text);
      const from = Number.isNaN(typed) ? view.valuesDistanceMm : typed;
      const next = stepChipDistance(from, event.key === "ArrowUp" ? 1 : -1, {
        shift: event.shiftKey,
        ctrl: event.ctrlKey || event.metaKey,
      });
      offset.setDistance(next);
      setText(chipDistanceText(next));
      setInvalid(false);
      setMessage(null);
      return;
    }
    onKeyDown(event, "first");
  };

  // A blur that leaves the chip closes it (a press elsewhere, a tool switch, the window losing
  // focus); a press on the canvas is still processed there, so it is not swallowed.
  const onBlur = (event: React.FocusEvent<HTMLElement>) => {
    const next = event.relatedTarget;
    if (next instanceof Node && chipRef.current?.contains(next)) {
      if (event.target === inputRef.current) {
        commitText();
      }
      return;
    }
    close(false);
  };

  const choose = <T extends string>(set: (value: T) => void) => (value: T) => {
    set(value);
    setMessage(null);
  };

  return (
    <div
      ref={chipRef}
      role="group"
      aria-label="Offset"
      tabIndex={-1}
      data-offset-entry
      className="absolute z-30 rounded-[8px] p-1.5 outline-none"
      style={{
        left: placement.left,
        top: placement.top,
        width: 168,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
      }}
    >
      <div ref={bodyRef} className="flex flex-col gap-1.5">
        <div className="flex items-center gap-1">
          <span aria-hidden className="w-[52px] text-xs">
            Distance
          </span>
          <div className="relative" style={{ width: 100 }}>
            <input
              ref={inputRef}
              type="text"
              inputMode="decimal"
              autoComplete="off"
              spellCheck={false}
              aria-label="Distance, millimetres"
              aria-invalid={invalid ? true : undefined}
              aria-describedby={message !== null ? messageId : undefined}
              value={text}
              onChange={(event) => {
                setText(event.target.value);
                setInvalid(false);
                setMessage(null);
              }}
              onKeyDown={onDistanceKeyDown}
              onBlur={onBlur}
              className={`h-7 w-full rounded-[5px] border bg-white pr-8 pl-1.5 text-right text-sm tabular-nums outline-none ${
                invalid
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
        </div>
        <RadioGroup
          name="Direction"
          items={DIRECTIONS}
          selected={view.direction}
          onSelect={choose(offset.setDirection)}
          onKeyDown={(event) => onKeyDown(event, null)}
          onBlur={onBlur}
          wide
        />
        <div className="flex items-center justify-between">
          <RadioGroup
            name="Corners"
            items={CORNERS}
            selected={view.join}
            onSelect={choose(offset.setJoin)}
            onKeyDown={(event) => onKeyDown(event, null)}
            onBlur={onBlur}
          />
          <RadioGroup
            name="Mode"
            items={MODES}
            selected={view.result}
            onSelect={choose(offset.setResult)}
            onKeyDown={(event) => onKeyDown(event, "last")}
            onBlur={onBlur}
            buttonRefs={modeRefs}
          />
        </div>
      </div>
      {/* Always mounted so the polite live region announces a change; the card sits outside the
          body, so the chip never moves (`ux-rework.md` 2.3). */}
      <div
        id={messageId}
        role="status"
        aria-live="polite"
        className={
          message !== null
            ? `absolute left-0 flex w-max max-w-[320px] min-w-full items-center gap-1 rounded-[8px] px-1.5 py-1 text-xs ${
                roomBelow ? "top-full mt-1" : "bottom-full mb-1"
              }`
            : "absolute h-0 overflow-hidden text-xs"
        }
        style={
          message !== null
            ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }
            : undefined
        }
      >
        {message !== null && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>{message}</span>
          </>
        )}
      </div>
    </div>
  );
}

interface RadioItem<T extends string> {
  value: T;
  /** Visible text; icon segments have none. */
  label?: string;
  /** The accessible name of an icon segment. */
  name?: string;
  title: string;
  icon?: React.ReactNode;
}

interface RadioGroupProps<T extends string> {
  name: string;
  items: RadioItem<T>[];
  selected: T;
  onSelect: (value: T) => void;
  onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
  onBlur: (event: React.FocusEvent<HTMLElement>) => void;
  /** The full-width pill of text segments (Direction); otherwise 28 px icon segments. */
  wide?: boolean;
  buttonRefs?: React.MutableRefObject<Array<HTMLButtonElement | null>>;
}

/**
 * One of the chip's radio groups (`ux-rework.md` 2.5, 2.6): the Move mode pill's look, one Tab stop
 * (roving tabindex on the chosen item); Left/Right and Up/Down move and choose with wrapping,
 * Home/End the first and last, Space chooses the focused item.
 */
function RadioGroup<T extends string>({
  name,
  items,
  selected,
  onSelect,
  onKeyDown,
  onBlur,
  wide,
  buttonRefs,
}: RadioGroupProps<T>) {
  const ownRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const refs = buttonRefs ?? ownRefs;
  const move = (index: number) => {
    const next = (index + items.length) % items.length;
    const item = items[next];
    if (item) {
      onSelect(item.value);
      refs.current[next]?.focus();
    }
  };
  const onItemKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    const steps: Record<string, number> = {
      ArrowLeft: index - 1,
      ArrowUp: index - 1,
      ArrowRight: index + 1,
      ArrowDown: index + 1,
      Home: 0,
      End: items.length - 1,
    };
    if (event.key in steps) {
      event.preventDefault();
      move(steps[event.key] ?? index);
    } else if (event.key === " ") {
      event.preventDefault();
      const item = items[index];
      if (item) {
        onSelect(item.value);
      }
    } else {
      onKeyDown(event);
    }
  };
  return (
    <div
      role="radiogroup"
      aria-label={name}
      className={`flex h-7 rounded-[7px] p-[1.5px] ${wide ? "w-full" : ""}`}
      // A 1.5 px ring as a shadow: a 1.5 px border computes to 1 px at a device pixel ratio of 1.
      style={{ boxShadow: "inset 0 0 0 1.5px var(--toolbar-icon)" }}
    >
      {items.map((item, index) => {
        const active = item.value === selected;
        return (
          <button
            key={item.value}
            ref={(node) => {
              refs.current[index] = node;
            }}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={item.name}
            tabIndex={active ? 0 : -1}
            title={item.title}
            onClick={() => onSelect(item.value)}
            onKeyDown={(event) => onItemKeyDown(event, index)}
            onBlur={onBlur}
            className={`flex cursor-pointer items-center justify-center rounded-[5px] text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
              wide ? "flex-1" : "w-[27px]"
            }`}
            style={
              active
                ? {
                    background: "var(--toolbar-icon-active-bg)",
                    color: "var(--toolbar-icon-active-fg)",
                  }
                : { color: "var(--toolbar-icon)" }
            }
          >
            {item.icon ?? item.label}
          </button>
        );
      })}
    </div>
  );
}

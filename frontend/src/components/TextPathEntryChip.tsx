// The entry box of the position handle of a text on path (`specs/0080-text-on-path` criteria 17,
// 18; customer 2026-10-24): opened by a double-click on the handle, the "Transform entry chip"
// surface with Position and Distance, then Side, Line on path and Mirror. Tab cycles fields and
// buttons, the buttons preview, Enter writes one step, Escape (or leaving the box) closes it with
// no change. Values and steps are the session's.
import { CircleAlert } from "lucide-react";
import { type ReactNode, useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { MirrorGlyph } from "@/components/TextPathGroups";
import { Glyph } from "@/components/WritingGroups";
import { Tooltip } from "@/components/ui/tooltip";
import { placeMoveChip } from "@/lib/readoutPlacement";
import {
  LINE_CODES,
  LINE_NAMES,
  type LineCode,
  SIDE_CODES,
  SIDE_NAMES,
  type SideCode,
  type TextPathEntry,
  pathFieldText,
  positionUnit,
} from "@/lib/textPathText";

/** The tool rail's clearance from the canvas's left edge, px (as the transform entry chip). */
const TOOL_RAIL_CLEAR_PX = 128;

/** The 24-grid glyphs of the Side and Line on path groups, drawn on a 16 grid scaled by 1.5. */
const SIDE_GLYPHS: Record<SideCode, string> = {
  above: "M3 15h18M12 12V4M9 7l3-3 3 3",
  below: "M3 9h18M12 12v8M9 17l3 3 3-3",
};
const LINE_GLYPHS: Record<LineCode, string> = {
  baseline: "M3 19.5h18M8 18V7M16 18V7M8 12.5h8",
  top: "M3 4.5h18M8 6v11M16 6v11M8 11.5h8",
};

const BUTTON_CLASS =
  "flex size-7 shrink-0 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";

interface TextPathEntryChipProps {
  entry: TextPathEntry;
  /** The canvas container: the chip's coordinate space, and where focus returns to. */
  containerRef: React.RefObject<HTMLDivElement | null>;
  /** Enter: `"committed"`, `"unchanged"` or `"invalid:<field>"`. */
  onCommit: (position: string, distance: string) => string;
  onCancel: () => void;
  onSide: (side: SideCode) => void;
  onLine: (line: LineCode) => void;
  onMirror: () => void;
}

/** One 28 x 28 button of the box: a radio of Side or Line on path, or Mirror. */
function EntryButton({
  label,
  checked,
  onPress,
  children,
}: {
  label: string;
  checked?: boolean;
  onPress: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip side="bottom" content={label}>
      <button
        type="button"
        role={checked === undefined ? undefined : "radio"}
        aria-label={label}
        aria-checked={checked}
        onClick={onPress}
        className={`${BUTTON_CLASS} ${
          checked
            ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]"
            : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
        }`}
      >
        {children}
      </button>
    </Tooltip>
  );
}

/** A number field of the box, 136px, its label inside the left edge and the unit at the right. */
function EntryField({
  label,
  unit,
  value,
  invalid,
  describedBy,
  inputRef,
  onChange,
}: {
  label: string;
  unit: string;
  value: string;
  invalid: boolean;
  describedBy: string | undefined;
  inputRef: (node: HTMLInputElement | null) => void;
  onChange: (text: string) => void;
}) {
  return (
    <div className="relative w-[136px]">
      <span aria-hidden className="pointer-events-none absolute top-1/2 left-1.5 -translate-y-1/2 text-xs">
        {label}
      </span>
      <input
        ref={inputRef}
        type="text"
        inputMode="decimal"
        autoComplete="off"
        spellCheck={false}
        aria-label={label}
        aria-invalid={invalid ? true : undefined}
        aria-describedby={describedBy}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={`h-7 w-full rounded-[5px] border bg-white pr-8 pl-[62px] text-right text-sm tabular-nums outline-none ${
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
        {unit}
      </span>
    </div>
  );
}

/** The entry box next to the position handle. */
export function TextPathEntryChip({
  entry,
  containerRef,
  onCommit,
  onCancel,
  onSide,
  onLine,
  onMirror,
}: TextPathEntryChipProps) {
  const chipRef = useRef<HTMLDivElement>(null);
  const inputs = useRef<Array<HTMLInputElement | null>>([]);
  const messageId = useId();
  const [texts, setTexts] = useState(() => [pathFieldText(entry.position), pathFieldText(entry.distance)]);
  const [invalid, setInvalid] = useState<number | null>(null);
  const [sizes, setSizes] = useState({
    chip: { width: 0, height: 0 },
    canvas: { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY },
  });

  // Position takes the focus with its text selected, once per open.
  useEffect(() => {
    inputs.current[0]?.focus();
    inputs.current[0]?.select();
  }, []);

  useLayoutEffect(() => {
    const chip = chipRef.current;
    const container = containerRef.current;
    if (!chip || !container) {
      return;
    }
    const next = {
      chip: { width: chip.offsetWidth, height: chip.offsetHeight },
      canvas: { width: container.clientWidth, height: container.clientHeight },
    };
    setSizes((previous) => (JSON.stringify(previous) === JSON.stringify(next) ? previous : next));
  }, [entry, invalid, containerRef]);

  const placed = placeMoveChip(entry.handle, sizes.chip, sizes.canvas);
  const placement = { ...placed, left: Math.max(placed.left, TOOL_RAIL_CLEAR_PX) };
  const returnFocusToCanvas = () => containerRef.current?.focus();

  const submit = () => {
    const outcome = onCommit(texts[0] ?? "", texts[1] ?? "");
    if (outcome.startsWith("invalid:")) {
      const field = Number(outcome.split(":")[1]);
      setInvalid(field);
      inputs.current[field]?.focus();
      inputs.current[field]?.select();
      return;
    }
    returnFocusToCanvas();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
      returnFocusToCanvas();
    } else if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
      event.preventDefault();
      submit();
    } else if (event.key === "Tab") {
      // Tab cycles the fields and the buttons inside the box.
      const focusable = Array.from(chipRef.current?.querySelectorAll<HTMLElement>("input, button") ?? []);
      const at = focusable.indexOf(event.target as HTMLElement);
      if (at >= 0 && focusable.length > 0) {
        event.preventDefault();
        const next = focusable[(at + (event.shiftKey ? -1 : 1) + focusable.length) % focusable.length];
        next?.focus();
        if (next instanceof HTMLInputElement) {
          next.select();
        }
      }
    }
    // Keys typed in the box never reach the canvas's shortcuts.
    event.stopPropagation();
  };

  // Leaving the box (a press elsewhere, a tool switch, the window losing focus) closes it.
  const onBlur = (event: React.FocusEvent<HTMLDivElement>) => {
    const next = event.relatedTarget;
    if (next instanceof Node && chipRef.current?.contains(next)) {
      return;
    }
    onCancel();
  };

  const change = (index: number, text: string) => {
    const next = [...texts];
    next[index] = text;
    setTexts(next);
    setInvalid(null);
  };

  return (
    <div
      ref={chipRef}
      role="group"
      aria-label="Text on path"
      data-text-path-entry
      onKeyDown={onKeyDown}
      onBlur={onBlur}
      className="absolute z-30 flex items-center gap-1 rounded-[8px] p-1.5"
      style={{
        left: placement.left,
        top: placement.top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
      }}
    >
      <EntryField
        label="Position"
        unit={positionUnit(entry)}
        value={texts[0] ?? ""}
        invalid={invalid === 0}
        describedBy={invalid === 0 ? messageId : undefined}
        inputRef={(node) => {
          inputs.current[0] = node;
        }}
        onChange={(text) => change(0, text)}
      />
      <EntryField
        label="Distance"
        unit="mm"
        value={texts[1] ?? ""}
        invalid={invalid === 1}
        describedBy={invalid === 1 ? messageId : undefined}
        inputRef={(node) => {
          inputs.current[1] = node;
        }}
        onChange={(text) => change(1, text)}
      />
      <div role="radiogroup" aria-label="Side" className="ml-1 flex items-center gap-1">
        {SIDE_CODES.map((code) => (
          <EntryButton key={code} label={SIDE_NAMES[code]} checked={entry.side === code} onPress={() => onSide(code)}>
            <Glyph d={SIDE_GLYPHS[code]} />
          </EntryButton>
        ))}
      </div>
      <div role="radiogroup" aria-label="Line on path" className="flex items-center gap-1">
        {LINE_CODES.map((code) => (
          <EntryButton key={code} label={LINE_NAMES[code]} checked={entry.line === code} onPress={() => onLine(code)}>
            <Glyph d={LINE_GLYPHS[code]} />
          </EntryButton>
        ))}
      </div>
      <EntryButton label="Mirror" onPress={onMirror}>
        <MirrorGlyph />
      </EntryButton>
      <div
        id={messageId}
        role="status"
        aria-live="polite"
        className={
          invalid === null
            ? "absolute h-0 overflow-hidden text-xs"
            : "absolute top-full left-0 mt-1 flex w-max items-center gap-1 whitespace-nowrap rounded-[8px] px-1.5 py-1 text-xs"
        }
        style={
          invalid === null ? undefined : { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }
        }
      >
        {invalid === null ? null : (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>Enter a number</span>
          </>
        )}
      </div>
    </div>
  );
}

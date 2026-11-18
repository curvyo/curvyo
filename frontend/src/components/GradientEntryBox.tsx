import { CircleAlert } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { ColourPicker } from "@/components/panel/ColourPicker";
import { Swatch } from "@/components/panel/Swatch";
import type { GradientApi } from "@/hooks/useGradient";
import { stepAngle, stepRadius, stopColour } from "@/lib/gradientText";
import type { GradientEntry } from "@/lib/gradientView";
import { clampInside } from "@/lib/readoutPlacement";

/** The box lies 16 px right of and below its handle (UX notes §7). */
const OFFSET_PX = 16;
/** The tool rail's clearance from the canvas's left edge. */
const RAIL_CLEAR_PX = 128;

const MESSAGES: Record<string, string> = {
  number: "Enter a number.",
  range: "Enter a number from 0 to 100.",
  hex: "Enter 3, 4, 6 or 8 hex digits",
};

const FIELD_LABELS = {
  angle: { prefix: "Angle", suffix: "°", name: "Gradient angle, degrees", width: 112 },
  radius: { prefix: "Radius", suffix: "mm", name: "Gradient radius, millimetres", width: 112 },
  // The stop's own name is put in front where the box is drawn (`Stop N position, percent`).
  position: { prefix: "Position", suffix: "%", name: "position, percent", width: 88 },
} as const;

const INPUT =
  "style-field h-7 w-full rounded-[5px] border bg-white text-sm tabular-nums outline-none focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]";
const BORDER = "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";

interface Props {
  entry: GradientEntry;
  gradient: GradientApi;
  containerRef: React.RefObject<HTMLDivElement | null>;
}

/**
 * The entry box at a gradient handle (`specs/0124-gradient-fill` criterion 7, UX notes §7): the
 * Angle, Radius or Position field, the stop's swatch button and its hex; the swatch opens the
 * inline colour picker inside the box. Enter writes the whole box as one step "Edit gradient";
 * Escape or a press outside closes it and writes nothing. The rules are the session's.
 */
export function GradientEntryBox({ entry, gradient, containerRef }: Props) {
  const box = useRef<HTMLDivElement>(null);
  const [field, setField] = useState(entry.fieldText);
  const [hex, setHex] = useState(entry.hex);
  const [invalid, setInvalid] = useState<string | null>(null);
  const [picker, setPicker] = useState(false);
  const [size, setSize] = useState({ width: 0, height: 0, canvasW: Infinity, canvasH: Infinity });
  const label = entry.field === "" ? null : FIELD_LABELS[entry.field];
  const colour = stopColour(hex);
  const stopName = `Stop ${entry.stop + 1}`;

  // A picker drag previews the stop's colour: the hex follows it.
  const [seenHex, setSeenHex] = useState(entry.hex);
  if (seenHex !== entry.hex) {
    setSeenHex(entry.hex);
    setHex(entry.hex);
  }
  // A press outside the box only closes it (criterion 7, UX review U8): it is taken before the
  // canvas or the panel sees it, so it selects, drags or presses nothing.
  useEffect(() => {
    const swallow = (event: Event) => {
      event.preventDefault();
      event.stopPropagation();
    };
    const press = (event: PointerEvent) => {
      if (event.target instanceof Node && box.current?.contains(event.target)) {
        return;
      }
      swallow(event);
      // The release of the same press goes nowhere either; the box is gone by then.
      window.addEventListener("pointerup", swallow, { capture: true, once: true });
      gradient.closeEntry();
      containerRef.current?.focus();
    };
    window.addEventListener("pointerdown", press, true);
    return () => window.removeEventListener("pointerdown", press, true);
  }, [gradient, containerRef]);
  // Opens on the first field with its text selected (the hex of a Radial centre).
  useEffect(() => {
    const first = box.current?.querySelector<HTMLInputElement>("input");
    first?.focus();
    first?.select();
  }, []);
  useLayoutEffect(() => {
    const el = box.current;
    const container = containerRef.current;
    if (!el || !container) {
      return;
    }
    setSize((previous) =>
      previous.width === el.offsetWidth && previous.height === el.offsetHeight
        ? previous
        : { width: el.offsetWidth, height: el.offsetHeight, canvasW: container.clientWidth, canvasH: container.clientHeight },
    );
  }, [picker, invalid, containerRef]);
  const place = clampInside(
    entry.x + OFFSET_PX,
    entry.y + OFFSET_PX,
    { width: size.width, height: size.height },
    { width: size.canvasW, height: size.canvasH },
  );
  // Flipped above the handle when below has no room.
  const top = entry.y + OFFSET_PX + size.height > size.canvasH ? Math.max(0, entry.y - OFFSET_PX - size.height) : place.top;
  const left = Math.max(place.left, RAIL_CLEAR_PX);

  const commit = () => {
    const outcome = gradient.commitEntry(field, hex);
    if (outcome.startsWith("invalid:")) {
      setInvalid(outcome.slice("invalid:".length));
      return;
    }
    containerRef.current?.focus();
  };
  const cancel = () => {
    gradient.closeEntry();
    containerRef.current?.focus();
  };
  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "Enter" && (event.target as HTMLElement).tagName === "INPUT") {
      event.preventDefault();
      commit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      cancel();
    } else if (event.key === "Tab") {
      // Tab and Shift+Tab cycle inside the box.
      const items = [...(box.current?.querySelectorAll<HTMLElement>("input, button, [tabindex='0']") ?? [])];
      const at = items.indexOf(event.target as HTMLElement);
      if (items.length > 0 && at >= 0) {
        event.preventDefault();
        items[(at + (event.shiftKey ? -1 : 1) + items.length) % items.length].focus();
      }
    } else if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      const target = event.target as HTMLInputElement;
      const stepper = entry.field === "angle" ? stepAngle : entry.field === "radius" ? stepRadius : null;
      if (stepper && target.dataset.gradientField !== undefined) {
        const next = stepper(field, event.key, event.shiftKey);
        if (next !== null) {
          event.preventDefault();
          setField(next);
        }
      }
    }
  };

  return (
    <div
      ref={box}
      role="group"
      aria-label={entry.kind === "end" ? "Gradient end" : "Gradient stop"}
      data-gradient-entry=""
      onKeyDown={onKeyDown}
      onBlur={(event) => {
        // Focus left for another element (a press on the canvas): the box closes, nothing is written.
        const next = event.relatedTarget;
        if (next instanceof Node && !box.current?.contains(next)) {
          gradient.closeEntry();
        }
      }}
      className="pointer-events-auto absolute flex flex-col gap-1 rounded-lg p-1.5"
      style={{
        left,
        top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
      }}
    >
      <div className="flex items-center gap-1">
        {label && (
          <div className="relative" style={{ width: label.width }}>
            <input
              type="text"
              data-gradient-field=""
              aria-label={entry.field === "position" ? `${stopName} ${label.name}` : label.name}
              aria-invalid={invalid === "number" || invalid === "range" ? true : undefined}
              autoComplete="off"
              spellCheck={false}
              value={field}
              onChange={(event) => {
                setField(event.target.value);
                setInvalid(null);
              }}
              className={`${INPUT} ${BORDER} text-right`}
              style={{ paddingLeft: 8 + label.prefix.length * 6.5, paddingRight: label.suffix.length > 1 ? 32 : 24 }}
            />
            <span aria-hidden className="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-xs text-[var(--panel-muted-fg)]">
              {label.prefix}
            </span>
            <span aria-hidden className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-sm text-[var(--panel-muted-fg)]">
              {label.suffix}
            </span>
          </div>
        )}
        <button
          type="button"
          aria-label="Edit colour"
          aria-expanded={picker}
          onClick={() => setPicker((open) => !open)}
          className="flex size-7 shrink-0 items-center justify-center rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          <Swatch rgb={colour?.rgb ?? 0} opacity={colour?.opacity ?? 100} mixed={false} size={20} />
        </button>
        <input
          type="text"
          aria-label={`${stopName} colour`}
          aria-invalid={invalid === "hex" ? true : undefined}
          autoComplete="off"
          spellCheck={false}
          value={hex}
          onChange={(event) => {
            setHex(event.target.value);
            setInvalid(null);
          }}
          className={`${INPUT} ${BORDER} pl-1.5 text-left`}
          style={{ width: 92 }}
        />
      </div>
      {picker && (
        <div className="py-1" data-gradient-entry-picker="">
          <ColourPicker
            name={`${stopName} entry`}
            rgb={colour?.rgb ?? 0}
            mixed={colour === null}
            hsvOf={gradient.hsvOf}
            rgbOf={gradient.rgbOf}
            onPreview={(h, s, v) => gradient.entryPicker.preview(entry.stop, h, s, v)}
            onSettle={gradient.entryPicker.settle}
            onOpenEntry={() => box.current?.querySelector<HTMLInputElement>(`input[aria-label="${stopName} colour"]`)?.focus()}
            width={236}
            areaHeight={80}
          />
        </div>
      )}
      {invalid && (
        <p role="alert" className="flex items-start gap-1 text-xs text-[var(--field-invalid)]">
          <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
          {invalid === "range" && entry.field === "radius" ? "Enter a number above 0." : (MESSAGES[invalid] ?? "Not a valid value")}
        </p>
      )}
    </div>
  );
}

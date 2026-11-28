// The Pen strip's layer pick (`specs/0158-pen-display` slice 2b criterion 3, UX 10.3, design system
// "Pen layer pick"): a 48 px button with the active layer's name that opens the active page's
// layers, front-most first; choosing one sets this window's active layer, no step.
import { ChevronDown, EyeOff, Lock } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { ignoresPress, type PenLayerRow } from "@/pen/penLayout";

const ROW =
  "flex h-12 w-full items-center gap-2 rounded-[5px] px-3 text-left text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

/** The pick: `rows` from the session, `onPick` with the chosen row's id. */
export function PenLayerPick({ rows, onPick }: { rows: PenLayerRow[]; onPick: (id: unknown) => void }) {
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement>(null);
  const touch = useRef(false);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", outside, true);
    document.addEventListener("keydown", escape, true);
    return () => {
      document.removeEventListener("pointerdown", outside, true);
      document.removeEventListener("keydown", escape, true);
    };
  }, [open]);
  const active = rows.find((row) => row.active);
  const none = rows.length === 0;
  const name = none ? "Page" : (active?.name ?? rows[0].name);
  const pressedBy = (run: () => void) => ({
    onPointerDown: (event: React.PointerEvent) => {
      touch.current = ignoresPress(event.pointerType);
    },
    onClick: () => {
      const fromTouch = touch.current;
      touch.current = false;
      if (!fromTouch) run();
    },
  });
  return (
    <div ref={box} className="relative">
      <button
        type="button"
        aria-label={`Layer: ${name}`}
        title={`Layer: ${name}`}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-disabled={none ? true : undefined}
        className="flex h-12 w-[180px] items-center gap-1 rounded-[5px] px-3 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-disabled:opacity-40"
        {...pressedBy(() => {
          if (!none) setOpen((was) => !was);
        })}
      >
        <span className="min-w-0 flex-1 truncate text-left">{name}</span>
        <ChevronDown size={16} aria-hidden />
      </button>
      {open ? (
        <div
          role="listbox"
          aria-label="Layers"
          className="absolute top-full left-0 z-40 mt-1 w-[220px] rounded-lg bg-[var(--toolbar-bg)] p-1"
          style={{ boxShadow: "var(--panel-elevation-shadow)" }}
        >
          {rows.map((row, index) => (
            <button
              key={index}
              type="button"
              role="option"
              aria-selected={row.active}
              className={`${ROW} relative`}
              {...pressedBy(() => {
                onPick(row.id);
                setOpen(false);
              })}
            >
              {row.active ? <span aria-hidden className="absolute inset-y-2 left-0 w-[3px] rounded bg-[var(--editor-accent)]" /> : null}
              <span className="min-w-0 flex-1 truncate">{row.name}</span>
              {row.hidden ? <EyeOff size={16} aria-label="Hidden" /> : null}
              {row.locked ? <Lock size={16} aria-label="Locked" /> : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

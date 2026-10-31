import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

/** A row of the menu: its stable key, its name and its second line. */
export interface MenuChoice {
  key: string;
  label: string;
  detail: string;
  /** Unavailable choices remain focusable so their reason can be read. */
  disabled?: boolean;
}

/** The menu's width (UX notes section 11); a longer name widens it rather than wrap. */
const MENU_WIDTH = 232;
/** The gap between the button and the menu. */
const MENU_GAP = 4;

interface TracePresetMenuProps<T extends MenuChoice> {
  presets: readonly T[];
  /** The preset the values came from. */
  current: string;
  /** The values were edited since the pick: the button shows "Custom", no item is checked. */
  edited: boolean;
  /** The button's text: the preset's name or "Custom". */
  label: string;
  onPick: (preset: T) => void;
  /** The button's and the menu's accessible name: "Preset", or "Method" (`specs/0089` UX notes
   * §5, the same look and keys). */
  name?: string;
  /** The button's width class: 132 px for Preset, 176 px for Method ("Remove background" whole). */
  widthClass?: string;
  /** Right-side docks anchor the menu to the button's trailing edge. */
  align?: "start" | "end";
}

/** The index the menu moves to on `key` from `at` (no wrap), or `null` for another key. */
function menuMove(key: string, at: number, count: number): number | null {
  switch (key) {
    case "ArrowDown":
      return Math.min(count - 1, at + 1);
    case "ArrowUp":
      return Math.max(0, at - 1);
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}

/**
 * The Preset button and its menu (`specs/0025-raster-trace` criteria 37, 40, 59, 66; UX notes
 * section 11): a 132 px text button with the preset's name or "Custom" and a chevron; the menu
 * below it (above when it would leave the viewport) lists the presets with their second lines as
 * `menuitemradio` rows. Enter or Down opens it on the current preset; Up, Down, Home and End move
 * without wrap; Enter or a click picks and closes; Escape or a click outside closes. Focus returns
 * to the button.
 */
export function TracePresetMenu<T extends MenuChoice>({
  presets,
  current,
  edited,
  label,
  onPick,
  name = "Preset",
  widthClass = "w-[132px]",
  align = "start",
}: TracePresetMenuProps<T>) {
  const [open, setOpen] = useState(false);
  const [focused, setFocused] = useState(0);
  const [above, setAbove] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const items = useRef<(HTMLDivElement | null)[]>([]);
  const baseId = useId();
  const currentIndex = Math.max(
    0,
    presets.findIndex((p) => p.key === current),
  );

  const openMenu = () => {
    setFocused(currentIndex);
    setOpen(true);
  };
  const close = () => {
    setOpen(false);
    buttonRef.current?.focus();
  };

  useLayoutEffect(() => {
    if (!open) {
      return;
    }
    const button = buttonRef.current?.getBoundingClientRect();
    const height = menuRef.current?.offsetHeight ?? 0;
    setAbove(Boolean(button) && button!.bottom + MENU_GAP + height > window.innerHeight && button!.top - MENU_GAP - height >= 0);
    items.current[focused]?.focus();
  }, [open, focused]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const outside = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!menuRef.current?.contains(target) && !buttonRef.current?.contains(target)) {
        setOpen(false);
      }
    };
    window.addEventListener("pointerdown", outside, true);
    return () => window.removeEventListener("pointerdown", outside, true);
  }, [open]);

  const buttonKey = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      openMenu();
    }
  };
  const menuKey = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (presets[focused].disabled) return;
      onPick(presets[focused]);
      close();
      return;
    }
    if (event.key === "Tab") {
      setOpen(false);
      return;
    }
    const next = menuMove(event.key, focused, presets.length);
    if (next !== null) {
      event.preventDefault();
      setFocused(next);
    }
  };

  return (
    <div className="relative">
      <button
        ref={buttonRef}
        type="button"
        aria-label={name}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={buttonKey}
        className={`my-1 flex h-7 ${widthClass} items-center justify-between gap-1 rounded-md border border-[var(--toolbar-icon)]/40 px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]`}
      >
        <span className="truncate">{label}</span>
        <svg aria-hidden width="10" height="10" viewBox="0 0 10 10" className="shrink-0">
          <path d="M2 3.5 L5 6.5 L8 3.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </button>
      {open ? (
        <div
          ref={menuRef}
          role="menu"
          aria-label={name}
          onKeyDown={menuKey}
          className={`absolute ${align === "end" ? "right-0" : "left-0"} z-50 flex flex-col rounded-lg py-1`}
          style={{
            minWidth: MENU_WIDTH,
            width: "max-content",
            background: "var(--toolbar-bg)",
            boxShadow: "var(--panel-elevation-shadow)",
            ...(above ? { bottom: `calc(100% + ${MENU_GAP}px)` } : { top: `calc(100% + ${MENU_GAP}px)` }),
          }}
        >
          {presets.map((preset, index) => {
            const checked = !edited && preset.key === current;
            const marked = edited && preset.key === current;
            return (
              <div
                key={preset.key}
                ref={(element) => {
                  items.current[index] = element;
                }}
                role="menuitemradio"
                aria-checked={checked}
                aria-disabled={preset.disabled || undefined}
                aria-describedby={`${baseId}-${preset.key}`}
                tabIndex={index === focused ? 0 : -1}
                onClick={() => {
                  if (preset.disabled) return;
                  onPick(preset);
                  close();
                }}
                onPointerMove={() => setFocused(index)}
                className="aria-disabled:opacity-40 mx-1 flex cursor-default flex-col rounded-md px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                style={{
                  background: index === focused || marked ? "var(--editor-accent-hover)" : undefined,
                }}
              >
                <span className="flex items-center gap-2 whitespace-nowrap font-semibold text-[var(--toolbar-icon)]">
                  <span aria-hidden className="w-3">
                    {checked ? "✓" : ""}
                  </span>
                  {preset.label}
                </span>
                <span id={`${baseId}-${preset.key}`} className="pl-5 text-xs text-[var(--toolbar-icon)] opacity-70">
                  {preset.detail}
                </span>
              </div>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

import { Check, Plus, SwatchBook } from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

import { MATERIAL_FILTER_ABOVE, filterMaterials, materialButtonText, thicknessLabel } from "@/lib/testGridBar";
import type { TestGridBarView } from "@/lib/testGridView";

const MENU_GAP = 4;

interface TestGridMaterialMenuProps {
  view: TestGridBarView;
  /** A pick, `id` of the material. */
  onPick: (id: string) => void;
  /** "New material": the Materials tab opens with its form. */
  onNewMaterial: () => void;
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
 * The material dropdown of the Test grid bar (`specs/0112` `ux-notes.md` 4.4, the Trace Preset
 * menu look): a button with the material's name, a menu below it (above when it would leave the
 * viewport) with "New material" first and the materials in the Materials tab's order, the picked
 * one checked; above 8 materials a Filter field heads it. Enter, Space or Down opens on the picked
 * row; Up, Down, Home and End move; Enter picks; Escape closes and the focus returns.
 */
export function TestGridMaterialMenu({ view, onPick, onNewMaterial }: TestGridMaterialMenuProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [focused, setFocused] = useState(0);
  const [above, setAbove] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const filterRef = useRef<HTMLInputElement>(null);
  const items = useRef<(HTMLDivElement | null)[]>([]);
  const filtered = useMemo(() => filterMaterials(view.materials, query), [view.materials, query]);
  // Row 0 is "New material"; the materials follow.
  const count = filtered.length + 1;
  const pickedIndex = Math.max(0, filtered.findIndex((one) => one.id === view.material?.id) + 1);
  const showFilter = view.materials.length > MATERIAL_FILTER_ABOVE;
  const button = materialButtonText(view);

  const setItemRef = (index: number, element: HTMLDivElement | null) => {
    items.current[index] = element;
  };
  const openMenu = () => {
    setQuery("");
    setFocused(pickedIndex);
    setOpen(true);
  };
  const close = () => {
    setOpen(false);
    buttonRef.current?.focus();
  };
  const choose = (index: number) => {
    if (index === 0) onNewMaterial();
    else onPick(filtered[index - 1].id);
    close();
  };

  useLayoutEffect(() => {
    if (!open) return;
    const box = buttonRef.current?.getBoundingClientRect();
    const height = menuRef.current?.offsetHeight ?? 0;
    setAbove(Boolean(box) && box!.bottom + MENU_GAP + height > window.innerHeight && box!.top - MENU_GAP - height >= 0);
    items.current[focused]?.focus();
  }, [open, focused]);

  useEffect(() => {
    if (!open) return undefined;
    const outside = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!menuRef.current?.contains(target) && !buttonRef.current?.contains(target)) setOpen(false);
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
    if (event.target === filterRef.current) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        items.current[focused]?.focus();
      }
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      choose(focused);
      return;
    }
    if (event.key === "Tab") {
      setOpen(false);
      return;
    }
    const next = menuMove(event.key, focused, count);
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
        aria-label="Material"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => (open ? setOpen(false) : openMenu())}
        onKeyDown={buttonKey}
        className="my-1 flex h-7 w-[200px] items-center gap-1 rounded-md border border-[var(--toolbar-icon)]/40 px-2 text-sm outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        <SwatchBook size={16} aria-hidden className="shrink-0 text-[var(--toolbar-icon)]" />
        <span className={`min-w-0 flex-1 truncate text-left ${button.picked ? "text-[var(--toolbar-icon)]" : "text-[var(--panel-muted-fg)]"}`}>{button.text}</span>
        <svg aria-hidden width="10" height="10" viewBox="0 0 10 10" className="shrink-0 text-[var(--toolbar-icon)]">
          <path d="M2 3.5 L5 6.5 L8 3.5" fill="none" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </button>
      {open ? (
        <div
          ref={menuRef}
          role="menu"
          aria-label="Material"
          onKeyDown={menuKey}
          className="absolute left-0 z-50 flex flex-col rounded-lg py-1"
          style={{
            minWidth: 232,
            width: "max-content",
            background: "var(--toolbar-bg)",
            boxShadow: "var(--panel-elevation-shadow)",
            ...(above ? { bottom: `calc(100% + ${MENU_GAP}px)` } : { top: `calc(100% + ${MENU_GAP}px)` }),
          }}
        >
          {showFilter ? (
            <input
              ref={filterRef}
              aria-label="Filter"
              placeholder="Filter"
              value={query}
              onChange={(event) => {
                setQuery(event.target.value);
                setFocused(0);
              }}
              className="mx-1 mb-1 h-7 rounded border border-[var(--lane-line)] bg-white px-2 text-[13px] text-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            />
          ) : null}
          <MenuRow
            index={0}
            focused={focused}
            setRef={setItemRef}
            checked={false}
            onChoose={choose}
            onHover={setFocused}
            label={
              <span className="flex items-center gap-2 text-[13px] text-[var(--toolbar-icon)]">
                <Plus size={16} aria-hidden className="shrink-0" />
                New material
              </span>
            }
          />
          {filtered.map((one, at) => {
            const index = at + 1;
            const checked = one.id === view.material?.id;
            return (
              <MenuRow
                key={one.id}
                index={index}
                focused={focused}
                setRef={setItemRef}
                checked={checked}
                onChoose={choose}
                onHover={setFocused}
                label={
                  <span className="flex w-full items-center gap-2 whitespace-nowrap text-[13px] text-[var(--toolbar-icon)]">
                    <span aria-hidden className="flex w-4 shrink-0 justify-center">
                      {checked ? <Check size={16} /> : null}
                    </span>
                    <span className="min-w-0 flex-1">{one.name}</span>
                    <span className="pl-3 text-xs tabular-nums text-[var(--panel-muted-fg)]">{thicknessLabel(one)}</span>
                  </span>
                }
              />
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

interface MenuRowProps {
  index: number;
  focused: number;
  setRef: (index: number, element: HTMLDivElement | null) => void;
  checked: boolean;
  label: React.ReactNode;
  onChoose: (index: number) => void;
  onHover: (index: number) => void;
}

/** One 32 px row of the menu. */
function MenuRow({ index, focused, setRef, checked, label, onChoose, onHover }: MenuRowProps) {
  return (
    <div
      ref={(element) => setRef(index, element)}
      role="menuitemradio"
      aria-checked={checked}
      tabIndex={index === focused ? 0 : -1}
      onClick={() => onChoose(index)}
      onPointerMove={() => onHover(index)}
      className="mx-1 flex h-8 cursor-default items-center rounded-md px-2 outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      style={{ background: index === focused ? "var(--editor-accent-hover)" : undefined }}
    >
      {label}
    </div>
  );
}

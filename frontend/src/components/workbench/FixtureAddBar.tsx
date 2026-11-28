import { type KeyboardEvent, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import { FixtureGlyph } from "@/components/workbench/FixtureGlyphs";
import { KIND_WORDS } from "@/lib/fixtureRows";
import type { FixtureKind } from "@/lib/fixturesView";

/**
 * An add bar of the Fixtures tab (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 4.3 and 5.1): a
 * toolbar of 28px glyph buttons, one Tab stop, arrows move, each named and tipped with its kind
 * word. With `pressed` it is the Job fixtures bar: a button arms placement and shows `aria-pressed`
 * while it is the armed kind. `onKeyDown` lets the section answer keys of the armed button.
 */
export function FixtureAddBar({
  label,
  kinds,
  pressed,
  onPress,
  onKeyDown,
}: {
  label: string;
  kinds: readonly FixtureKind[];
  /** The armed kind of a placement bar; `undefined` for a bar whose buttons only add. */
  pressed?: FixtureKind | null;
  onPress: (kind: FixtureKind) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>, kind: FixtureKind) => void;
}) {
  const group = useRef<HTMLDivElement>(null);
  // The bar is one Tab stop: the button last focused, else the first.
  const [current, setCurrent] = useState<FixtureKind | null>(null);
  const stop = current !== null && kinds.includes(current) ? current : kinds[0];
  const move = (event: KeyboardEvent<HTMLDivElement>) => {
    // The armed button took the arrow to move the hanging fixture (UX notes 5.1).
    if (event.defaultPrevented) return;
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>("button"));
    const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0 || (event.key !== "ArrowRight" && event.key !== "ArrowLeft")) return;
    event.preventDefault();
    const to = event.key === "ArrowRight" ? at + 1 : at - 1;
    buttons[(to + buttons.length) % buttons.length]?.focus();
  };
  return (
    <div ref={group} role="toolbar" aria-label={label} className="flex gap-1" onKeyDown={move}>
      {kinds.map((kind) => {
        const word = KIND_WORDS[kind];
        const on = pressed === kind;
        return (
          <Tooltip key={kind} side="left" content={word}>
            <button
              type="button"
              aria-label={word}
              aria-pressed={pressed === undefined ? undefined : on}
              tabIndex={kind === stop ? 0 : -1}
              onFocus={() => setCurrent(kind)}
              onKeyDown={(event) => onKeyDown?.(event, kind)}
              className={`flex h-7 w-7 items-center justify-center rounded-[5px] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9 pointer-coarse:w-9 ${on ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]" : "text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"}`}
              onClick={(event) => {
                const button = event.currentTarget;
                onPress(kind);
                // A button that left the bar with its press (a set's Grid): focus moves to the first one left.
                requestAnimationFrame(() => {
                  if (!button.isConnected) group.current?.querySelector("button")?.focus();
                });
              }}
            >
              <FixtureGlyph kind={kind} />
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}

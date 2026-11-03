import { Plus } from "lucide-react";
import { type KeyboardEvent, useRef, useState } from "react";

import { Tooltip } from "@/components/ui/tooltip";
import type { WorkbenchesApi } from "@/hooks/useWorkbenches";
import { WorkbenchIcon } from "@/workbenches/icons";
import { switcherButtons } from "@/workbenches/registry";

interface WorkbenchSwitcherProps {
  workbenches: WorkbenchesApi;
  /** Whether the panel body is shown: hidden, the buttons stack in the 32 px strip. */
  open: boolean;
}

const BUTTON =
  "inline-flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)] aria-pressed:bg-[var(--toolbar-icon-active-bg)] aria-pressed:text-[var(--toolbar-icon-active-fg)] aria-disabled:opacity-40 [@media(pointer:coarse)]:size-9";

/**
 * The workbench switcher (`specs/0093-laser-workbench-shell` UX notes §1): the top row of the
 * Properties panel, one icon button per workbench the registry draws, in the session's order,
 * then a divider and Add manufacturing, which opens its group inline below (§2). Exactly one
 * button is pressed; one Tab stop; Left and Right move and activate, Home and End jump.
 */
export function WorkbenchSwitcher({ workbenches, open }: WorkbenchSwitcherProps) {
  const { list } = workbenches;
  const buttons = switcherButtons(list);
  const [adding, setAdding] = useState(false);
  const rowRef = useRef<HTMLDivElement>(null);
  const addRef = useRef<HTMLButtonElement>(null);
  const canAdd = list.add.length > 0;

  const focusButton = (id: string) =>
    rowRef.current?.querySelector<HTMLElement>(`[data-workbench="${CSS.escape(id)}"]`)?.focus();
  const activate = (id: string, enabled: boolean) => {
    setAdding(false);
    if (enabled) {
      workbenches.switchTo(id);
    }
  };
  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const last = buttons.length - 1 + (canAdd ? 1 : 0);
    const target =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? Math.min(index + 1, last)
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? Math.max(index - 1, 0)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    if (target === null || target === index) {
      return;
    }
    event.preventDefault();
    if (target === buttons.length) {
      addRef.current?.focus();
      return;
    }
    const button = buttons[target];
    activate(button.id, button.enabled);
    focusButton(button.id);
  };
  const add = (process: string) => {
    const id = workbenches.addManufacturing(process);
    setAdding(false);
    if (id) {
      requestAnimationFrame(() => focusButton(id));
    }
  };

  return (
    <div className={`bg-[var(--toolbar-bg)] ${open ? "px-[7px] py-1" : "w-8 py-1"}`}>
      <div
        ref={rowRef}
        role="toolbar"
        aria-label="Workbench"
        aria-orientation={open ? "horizontal" : "vertical"}
        className={open ? "flex flex-wrap items-center gap-1" : "flex max-h-[204px] flex-col items-center gap-1 overflow-y-auto"}
      >
        {buttons.map((button, index) => (
          <Tooltip key={button.id} side="left" content={button.label}>
            <button
              type="button"
              data-workbench={button.id}
              aria-label={button.label}
              aria-pressed={button.pressed}
              aria-disabled={!button.enabled || undefined}
              tabIndex={button.pressed ? 0 : -1}
              className={BUTTON}
              onClick={() => activate(button.id, button.enabled)}
              onKeyDown={(event) => onKeyDown(event, index)}
            >
              <WorkbenchIcon name={button.icon} />
            </button>
          </Tooltip>
        ))}
        {canAdd ? (
          <>
            <div
              aria-hidden
              className={open ? "mx-0.5 h-4 w-px" : "my-0.5 h-px w-4"}
              style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
            />
            <Tooltip side="left" content="Add manufacturing">
              <button
                ref={addRef}
                type="button"
                aria-label="Add manufacturing"
                aria-expanded={adding}
                tabIndex={-1}
                className={BUTTON}
                onClick={() => setAdding(!adding)}
                onKeyDown={(event) => onKeyDown(event, buttons.length)}
              >
                <Plus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
              </button>
            </Tooltip>
          </>
        ) : null}
      </div>
      {adding && open ? (
        <AddManufacturingGroup
          choices={list.add}
          onChoose={add}
          onCancel={() => {
            setAdding(false);
            addRef.current?.focus();
          }}
        />
      ) : null}
    </div>
  );
}

const TEXT_BUTTON =
  "flex h-7 w-full items-center gap-2 rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--editor-accent)]";

/** The inline Add manufacturing group (UX notes §2): one button per available process and
 * Cancel; it pushes the tabs down and closes on a choice, Escape or Cancel. */
function AddManufacturingGroup({
  choices,
  onChoose,
  onCancel,
}: {
  choices: { process: string; label: string; icon: string }[];
  onChoose: (process: string) => void;
  onCancel: () => void;
}) {
  return (
    <div
      role="group"
      aria-label="Add manufacturing"
      className="mt-1 flex flex-col gap-1 p-2"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          onCancel();
        }
      }}
    >
      {choices.map((choice, index) => (
        <button
          key={choice.process}
          type="button"
          // The first process button takes the focus when the group opens.
          autoFocus={index === 0}
          className={TEXT_BUTTON}
          onClick={() => onChoose(choice.process)}
        >
          <WorkbenchIcon name={choice.icon} />
          {choice.label}
        </button>
      ))}
      <button type="button" className={TEXT_BUTTON} onClick={onCancel}>
        Cancel
      </button>
    </div>
  );
}

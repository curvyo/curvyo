import { Droplet, File, FolderOpen, History, PanelRightClose, PanelRightOpen } from "lucide-react";
import { type ComponentType, useEffect, useRef, useState } from "react";

import { TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip } from "@/components/ui/tooltip";
import type { PanelTabView } from "@/hooks/useDocumentPanel";
import { isMac } from "@/lib/historyText";
import { panelHandlesEscape } from "@/lib/panelEscape";

type Glyph = ComponentType<{
  size: number;
  strokeWidth: number;
  absoluteStrokeWidth: boolean;
  "aria-hidden": boolean;
}>;

/** The glyph of each tab (Lucide, 16 px, 1.5 px absolute stroke). */
const GLYPHS: Record<string, Glyph> = {
  file: FolderOpen,
  document: File,
  style: Droplet,
  history: History,
};

/** The tooltips come from the session with the Ctrl key; macOS says Cmd (criterion 4). */
const MAC = isMac(navigator.platform);
const tooltipText = (text: string) => (MAC ? text.replace("Shift+Ctrl+", "Shift+Cmd+") : text);

interface PanelStripProps {
  tabs: readonly PanelTabView[];
  /** The active tab's name: the one the body shows, or would show. */
  active: string;
  /** Whether the body is shown. Hidden, no tab is shown pressed. */
  open: boolean;
  /** The id of the body, for `aria-controls` of the collapse control. */
  bodyId: string;
  /** The project has unsaved changes: the File tab carries a dot, also while the body is hidden. */
  fileDirty: boolean;
  /** The collapse control was pressed. */
  onToggle: (event: React.MouseEvent<HTMLButtonElement>) => void;
  /** A tab was pressed or chosen from the keyboard: the body is shown with it. */
  onActivate: (name: string) => void;
  /** After a mouse press the keyboard goes back to the canvas; a key press
   * keeps the focus on the strip so the arrows keep working. */
  onReturnFocus: () => void;
}

const BUTTON_CLASS =
  "relative flex size-7 shrink-0 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] pointer-coarse:size-9 " +
  // The ring is drawn inside the 28 px box: the strip scrolls and the margin is 2 px, so a ring
  // outside it would be cut. Two tones, because the pressed ground has the colour of the accent.
  "focus-visible:shadow-[inset_0_0_0_2px_var(--editor-accent),inset_0_0_0_4px_var(--toolbar-bg)]";

const TAB_CLASS = `${BUTTON_CLASS} group aria-selected:!bg-[var(--toolbar-icon-active-bg)] aria-selected:!text-[var(--toolbar-icon-active-fg)] disabled:cursor-default disabled:opacity-40 disabled:hover:bg-transparent`;

/**
 * The strip: a column of icon buttons on the panel's inner edge, next to the canvas
 * (`specs/0051-sidebar-and-no-menu/` criteria 1 to 9, 11). From the top: the collapse control, a
 * divider and the vertical tablist. Which tabs exist, their names, tooltips and which is dimmed
 * come from the session. The tooltip trigger shares the tab's element and replaces its
 * `data-state`, so the pressed look is keyed on `aria-selected`. A dimmed tab is a disabled button
 * (no pointer events, so its tooltip sits on a wrapper), skipped by the arrows and never a Tab stop.
 * While the body is hidden `aria-selected` is false on every tab (the pressed ground says that the
 * content is open) but the active tab stays the Tab stop.
 */
export function PanelStrip({
  tabs,
  active,
  open,
  bodyId,
  fileDirty,
  onToggle,
  onActivate,
  onReturnFocus,
}: PanelStripProps) {
  // The tablist is one Tab stop and it is the active tab. Radix keeps the tab that had focus last
  // as the stop, so when the active tab changes while focus is elsewhere (a shortcut, a selection)
  // the list is rebuilt: a fresh group forwards the Tab stop to the active tab.
  const focusInside = useRef(false);
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    if (!focusInside.current) {
      setGeneration((n) => n + 1);
    }
  }, [active]);
  const Collapse = open ? PanelRightClose : PanelRightOpen;
  return (
    <div
      onKeyDown={(event) => {
        // Escape from any strip control returns focus to the canvas and never clears the selection.
        if (panelHandlesEscape(event)) {
          event.preventDefault();
          onReturnFocus();
        }
      }}
      className="properties-panel box-border flex h-full w-8 shrink-0 flex-col items-start border-l border-[color-mix(in_srgb,var(--toolbar-icon)_25%,transparent)] bg-[var(--panel-bg)] pt-[10px] pl-px pointer-coarse:w-10"
    >
      <Tooltip side="left" content={open ? "Hide panel" : "Show panel"}>
        <button
          type="button"
          aria-label={open ? "Hide panel" : "Show panel"}
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={onToggle}
          className={BUTTON_CLASS}
        >
          <Collapse size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      </Tooltip>
      <div
        aria-hidden
        className="my-1 ml-1.5 h-px w-4 shrink-0 pointer-coarse:ml-2.5"
        style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
      />
      <TabsList
        key={generation}
        aria-label="Panel"
        loop={false}
        onFocus={() => {
          focusInside.current = true;
        }}
        onBlur={(event) => {
          if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
            focusInside.current = false;
          }
        }}
        className="flex min-h-0 w-full flex-1 flex-col items-start gap-1 overflow-y-auto overflow-x-hidden [scrollbar-width:thin]"
      >
        {tabs.map((tab) => {
          const Glyph = GLYPHS[tab.name];
          const dot = tab.name === "file" && fileDirty;
          const trigger = (
            <TabsTrigger
              value={tab.name}
              aria-label={dot ? `${tab.label}, unsaved changes` : tab.label}
              aria-selected={open && tab.name === active}
              disabled={!tab.enabled}
              aria-disabled={tab.enabled ? undefined : true}
              onClick={(event) => {
                onActivate(tab.name);
                // `detail` is 0 for keyboard activation, 1 or more for a click.
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className={TAB_CLASS}
            >
              {Glyph && <Glyph size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />}
              {dot ? (
                // A shape badge in the glyph's colour, inside the 28 px box so the strip's scroll
                // area never clips it; the casing has the ground's colour (the pressed ground
                // while File is pressed).
                <span
                  aria-hidden
                  data-dirty-dot
                  className="absolute top-[2px] right-[2px] size-1.5 rounded-full bg-current shadow-[0_0_0_1.5px_var(--panel-bg)] group-aria-selected:shadow-[0_0_0_1.5px_var(--toolbar-icon-active-bg)]"
                />
              ) : null}
            </TabsTrigger>
          );
          return (
            <Tooltip key={tab.name} side="left" content={tooltipText(tab.tooltip)}>
              {tab.enabled ? trigger : <span className="inline-flex">{trigger}</span>}
            </Tooltip>
          );
        })}
      </TabsList>
    </div>
  );
}

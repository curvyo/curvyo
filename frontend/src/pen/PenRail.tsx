// The pen window's rail and its Canvas only corner (`specs/0158-pen-display` slice 2a criterion 22,
// 2b criteria 1, 4, 8, 9; UX 10.1, 10.2, 10.4; design system "Pen rail"): Tools, Edit and View
// cards with 48 x 48 px buttons that work with the pen tip alone, and the corner toggle.
import { type ReactNode, useRef } from "react";
import {
  Eraser,
  Fullscreen,
  Hand,
  Link,
  Maximize2,
  Minimize2,
  MousePointer,
  MousePointer2,
  PencilLine,
  Redo2,
  Trash2,
  Undo2,
  ZoomIn,
  ZoomOut,
} from "lucide-react";

import { ignoresPress, type PenControl, type PenGroup } from "@/pen/penLayout";

const ICONS: Record<string, ReactNode> = {
  pencil: <PencilLine size={20} />,
  eraser: <Eraser size={20} />,
  select: <MousePointer size={20} />,
  node: <MousePointer2 size={20} />,
  undo: <Undo2 size={20} />,
  redo: <Redo2 size={20} />,
  delete: <Trash2 size={20} />,
  "zoom-in": <ZoomIn size={20} />,
  "zoom-out": <ZoomOut size={20} />,
  fit: <Fullscreen size={20} />,
  pan: <Hand size={20} />,
  follow: <Link size={20} />,
};

const BUTTON =
  "flex size-12 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] aria-disabled:opacity-40 aria-pressed:bg-[var(--editor-accent)] aria-pressed:text-white";
const CARD = "flex w-14 flex-col items-center gap-0 rounded-lg bg-[var(--toolbar-bg)] p-1 shadow";

/** One 48 px control; a touch press does nothing (UX 10.1). */
export function PenButton({ control, icon }: { control: PenControl; icon: ReactNode }) {
  const touch = useRef(false);
  return (
    <button
      type="button"
      className={BUTTON}
      aria-label={control.label}
      title={control.label}
      aria-pressed={control.pressed}
      aria-disabled={control.disabled ? true : undefined}
      data-pen-control={control.id}
      onPointerDown={(event) => {
        touch.current = ignoresPress(event.pointerType);
      }}
      onClick={() => {
        const fromTouch = touch.current;
        touch.current = false;
        if (!fromTouch && !control.disabled) control.press();
      }}
    >
      {icon}
    </button>
  );
}

function Card({ group, controls }: { group: PenGroup; controls: PenControl[] }) {
  return (
    <div className={CARD} role="group" aria-label={group}>
      {controls
        .filter((control) => control.group === group)
        .map((control) => (
          <PenButton key={control.id} control={control} icon={ICONS[control.id]} />
        ))}
    </div>
  );
}

/** The rail, left of the canvas; the View card stands in a second column on a low window. */
export function PenRail({ controls, twoColumns }: { controls: PenControl[]; twoColumns: boolean }) {
  return (
    <nav aria-label="Pen tools" className="absolute top-3 left-3 z-20 flex items-start gap-2">
      <div className="flex flex-col gap-2">
        <Card group="Tools" controls={controls} />
        <Card group="Edit" controls={controls} />
        {twoColumns ? null : <Card group="View" controls={controls} />}
      </div>
      {twoColumns ? <Card group="View" controls={controls} /> : null}
    </nav>
  );
}

/** The Canvas only toggle, its own 56 px card 12 px from the top and right edges, in the same
 * place in both states (UX 10.4). */
export function PenCorner({ controls }: { controls: PenControl[] }) {
  const control = controls.find((c) => c.group === "Corner");
  if (!control) return null;
  return (
    <div className={`${CARD} absolute top-3 right-3 z-30`}>
      <PenButton control={control} icon={control.pressed ? <Minimize2 size={20} /> : <Maximize2 size={20} />} />
    </div>
  );
}

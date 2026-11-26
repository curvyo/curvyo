import { Trash2 } from "lucide-react";

import { SlotGlyph } from "@/components/workbench/FixtureGlyphs";
import { NumberField, TextField } from "@/components/workbench/FixtureFields";
import { PinScrewBody } from "@/components/workbench/PinScrewBody";
import { ARMED_TRASH, ICON_BUTTON } from "@/components/workbench/rowKeys";
import { ARM_TRASH, ARM_ZONE, type DeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { MAX_SLOTS, slotCountText } from "@/lib/fixtureRows";
import { fixtureArmStatus, removedStatus } from "@/lib/fixtureSets";
import type { FixtureRow, SlotRow } from "@/lib/fixturesView";

/**
 * The unfolded body of a fixture row, by kind: a grid's fields, a holder's slots, a pin's or
 * screw's fields and Jobs toggles. A clamp and a stop have none: their handles size them.
 */
export function FixtureBody({
  row,
  selectedSlot,
  api,
  arm,
  readOnly = false,
}: {
  row: FixtureRow;
  selectedSlot: string | null;
  api: FixturesApi;
  arm: DeleteArm;
  /** The set is read only here (criterion 46): no field, only a pin's or screw's Output toggles. */
  readOnly?: boolean;
}) {
  if (readOnly && row.kind !== "pin" && row.kind !== "screw") return null;
  // A locked fixture keeps its values (criterion 32): shown as text, no field, no slot change.
  const locked = row.locked || row.lockedBySet;
  if (row.kind === "grid" && row.grid) return <GridFields row={row} grid={row.grid} api={api} locked={locked} />;
  if (row.kind === "holder") return <HolderFields row={row} selectedSlot={selectedSlot} api={api} arm={arm} locked={locked} />;
  if (row.kind === "pin" || row.kind === "screw") return <PinScrewBody row={row} api={api} readOnly={readOnly} />;
  return null;
}

function GridFields({ row, grid, api, locked }: { row: FixtureRow; grid: NonNullable<FixtureRow["grid"]>; api: FixturesApi; locked: boolean }) {
  const tapes: Array<["bottom" | "left" | "top" | "right", string]> = [
    ["bottom", "Bottom"],
    ["left", "Left"],
    ["top", "Top"],
    ["right", "Right"],
  ];
  return (
    // Every field fills the row, so nothing leaves the 280 px panel; Origin X over Origin Y
    // (customer, 2026-11-16).
    <div className="flex flex-col gap-1.5 pb-1">
      <NumberField fill readOnly={locked} label="Spacing" unit="mm" value={grid.spacingMm} onCommit={(mm) => api.setLength(row.id, "spacing_mm", mm)} />
      <NumberField fill readOnly={locked} label="Major every" unit="lines" value={grid.majorEvery} whole onCommit={(n) => api.setMajorEvery(row.id, n)} />
      {locked ? (
        <div className="flex h-7 items-center gap-1 px-2 text-sm">
          <span className="text-xs text-[var(--panel-muted-fg)]">Tapes</span>
          <span className="ml-auto truncate text-[var(--toolbar-icon)]">{tapes.filter(([edge]) => grid.tapes[edge]).map(([, word]) => word).join(", ") || "None"}</span>
        </div>
      ) : (
        <div role="group" aria-label="Tapes on" className="flex gap-1">
          {tapes.map(([edge, word]) => (
            <button
              key={edge}
              type="button"
              aria-label={`Tape on ${edge}`}
              aria-pressed={grid.tapes[edge]}
              className={`h-7 min-w-0 flex-1 rounded-md border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${grid.tapes[edge] ? "bg-[var(--toolbar-icon)] text-white" : "bg-white text-[var(--toolbar-icon)]"}`}
              onClick={() => api.setTape(row.id, edge, !grid.tapes[edge])}
            >
              {word}
            </button>
          ))}
        </div>
      )}
      <NumberField fill readOnly={locked} label="Origin X" unit="mm" value={grid.originXMm} onCommit={(mm) => api.setLength(row.id, "origin_x_mm", mm)} />
      <NumberField fill readOnly={locked} label="Origin Y" unit="mm" value={grid.originYMm} onCommit={(mm) => api.setLength(row.id, "origin_y_mm", mm)} />
    </div>
  );
}

function HolderFields({
  row,
  selectedSlot,
  api,
  arm,
  locked,
}: {
  row: FixtureRow;
  selectedSlot: string | null;
  api: FixturesApi;
  arm: DeleteArm;
  locked: boolean;
}) {
  const slot = row.slots.find((candidate) => candidate.id === selectedSlot) ?? null;
  return (
    <div className="flex flex-col gap-2 pb-1">
      <div className="flex h-6 items-center justify-between text-xs text-[var(--panel-muted-fg)]">
        <span>Slots</span>
        <span>{slotCountText(row.slots.length)}</span>
      </div>
      <ul aria-label="Slots" className="flex flex-col">
        {row.slots.map((candidate) => (
          <SlotItem key={candidate.id} slot={candidate} selected={candidate.id === selectedSlot} api={api} arm={arm} locked={locked} />
        ))}
      </ul>
      {row.canAddSlot && !locked && row.slots.length < MAX_SLOTS ? (
        <button
          type="button"
          className="h-7 rounded border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          onClick={() => api.addSlot(row.id)}
        >
          Add slot
        </button>
      ) : null}
      {slot ? <SlotFields slot={slot} api={api} locked={locked} /> : null}
    </div>
  );
}

/** One slot row: its shape, name, "Outside the board" as text, and the armed trash (not locked). */
function SlotItem({ slot, selected, api, arm, locked }: { slot: SlotRow; selected: boolean; api: FixturesApi; arm: DeleteArm; locked: boolean }) {
  const armed = arm.armed === slot.id;
  const label = armed ? `Press again to delete ${slot.name}` : `Delete ${slot.name}`;
  return (
    <li
      {...{ [ARM_ZONE]: slot.id }}
      className={`flex h-7 items-center gap-2 rounded-md px-1 ${armed ? "bg-[var(--destructive-soft)]" : selected ? "bg-[var(--value-fill)]" : ""}`}
    >
      <button
        type="button"
        aria-label={`${slot.name}, ${slot.shape === "rect" ? "rectangle" : "circle"}, ${slot.wMm} by ${slot.hMm} mm`}
        className="flex min-w-0 flex-1 items-center gap-2 text-left text-[13px] text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        onClick={() => api.selectSlot(slot.id)}
      >
        <SlotGlyph shape={slot.shape} className="shrink-0" />
        <span className="truncate">{slot.name}</span>
        {api.filledSlots?.has(slot.id) ? <span className="shrink-0 text-[11px] text-[var(--panel-muted-fg)]">filled</span> : null}
        {slot.outside ? <span className="shrink-0 text-[11px] text-[var(--panel-muted-fg)]">Outside the board</span> : null}
      </button>
      {locked ? null : (
        <button
          type="button"
          {...{ [ARM_TRASH]: slot.id }}
          aria-label={label}
          title={label}
          className={armed ? ARMED_TRASH : `${ICON_BUTTON} text-[var(--panel-muted-fg)]`}
          onClick={() =>
            arm.press(slot.id, fixtureArmStatus(slot.name), () => {
              api.removeSlot(slot.id);
              return removedStatus(slot.name);
            })
          }
        >
          <Trash2 size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        </button>
      )}
    </li>
  );
}

/** The fields of the selected slot: name, shape, centre, size; as text while locked. */
function SlotFields({ slot, api, locked }: { slot: SlotRow; api: FixturesApi; locked: boolean }) {
  const round = slot.shape === "circle";
  if (locked) {
    return (
      <div className="flex flex-col gap-1 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_20%,transparent)] pt-2">
        <NumberField readOnly fill label="Centre X" unit="mm" value={slot.cxMm} onCommit={() => {}} />
        <NumberField readOnly fill label="Centre Y" unit="mm" value={slot.cyMm} onCommit={() => {}} />
        {round ? (
          <NumberField readOnly fill label="Diameter" unit="mm" value={slot.wMm} onCommit={() => {}} />
        ) : (
          <>
            <NumberField readOnly fill label="Width" unit="mm" value={slot.wMm} onCommit={() => {}} />
            <NumberField readOnly fill label="Height" unit="mm" value={slot.hMm} onCommit={() => {}} />
          </>
        )}
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-2 border-t border-[color-mix(in_srgb,var(--toolbar-icon)_20%,transparent)] pt-2">
      <TextField key={`${slot.id}:${slot.name}`} label="Name" value={slot.name} onCommit={(name) => api.renameSlot(slot.id, name)} />
      <div role="radiogroup" aria-label="Shape" className="flex gap-1">
        {(["rect", "circle"] as const).map((shape) => (
          <button
            key={shape}
            type="button"
            role="radio"
            aria-checked={slot.shape === shape}
            className={`h-7 w-[118px] rounded-md border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] text-xs outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${slot.shape === shape ? "bg-[var(--toolbar-icon)] text-white" : "bg-white text-[var(--toolbar-icon)]"}`}
            onClick={() => api.setSlotShape(slot.id, shape)}
          >
            {shape === "rect" ? "Rectangle" : "Circle"}
          </button>
        ))}
      </div>
      <div className="flex gap-1">
        <NumberField label="Centre X" unit="mm" width={118} value={slot.cxMm} onCommit={(mm) => api.setSlotLength(slot.id, "cx_mm", mm)} />
        <NumberField label="Centre Y" unit="mm" width={118} value={slot.cyMm} onCommit={(mm) => api.setSlotLength(slot.id, "cy_mm", mm)} />
      </div>
      {round ? (
        <NumberField label="Diameter" unit="mm" value={slot.wMm} onCommit={(mm) => api.setSlotLength(slot.id, "d_mm", mm)} />
      ) : (
        <div className="flex gap-1">
          <NumberField label="Width" unit="mm" width={118} value={slot.wMm} onCommit={(mm) => api.setSlotLength(slot.id, "w_mm", mm)} />
          <NumberField label="Height" unit="mm" width={118} value={slot.hMm} onCommit={(mm) => api.setSlotLength(slot.id, "h_mm", mm)} />
        </div>
      )}
    </div>
  );
}

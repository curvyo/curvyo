// The Into slot… button and its inline list, which its surface places so it pushes what follows (`specs/0154-library-into-slots` criterion 10, UX
// notes section 8): the holders of the active laser page with a Fill empty slots row and their
// slots with size, outcome and "filled"; arrows move, Enter takes, Escape closes. A filled row
// arms Replace in place. The button is hidden, not disabled, without a target slot.
import { Circle, Grid2x2Plus, Magnet, Square } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";

import { useArm } from "@/hooks/useArm";
import {
  type SlotContext,
  type SlotUnitSource,
  fillSlots,
  hasSlotTargets,
  putIntoSlot,
  slotTargets,
  unitSize,
} from "@/lib/intoSlot";
import { type IntoSlotRow, intoSlotKey, intoSlotRows } from "@/lib/intoSlotKeys";

type Row = IntoSlotRow;

/** A source resolved for a slot, and the call that frees its reader. */
export type SlotResolve = () => Promise<{ source: SlotUnitSource; done: () => void } | null>;

/** The Into slot… button, or nothing while the active page has no target slot (hidden, not
 * disabled, V11). */
export function IntoSlotButton({
  ctx,
  look,
  open,
  onToggle,
}: {
  ctx: SlotContext;
  /** `icon`: a 24 px icon button; `text`: an outline text button. */
  look: "icon" | "text";
  open: boolean;
  onToggle: () => void;
}) {
  if (!hasSlotTargets(ctx.getSession())) {
    return null;
  }
  return (
    <button
      type="button"
      className={look === "icon" ? "library-action into-slot-icon" : "into-slot-button"}
      data-into-slot-button=""
      aria-label="Into slot…"
      aria-expanded={open}
      title="Into slot…"
      onClick={(event) => {
        event.stopPropagation();
        onToggle();
      }}
    >
      {look === "icon" ? <Magnet size={16} /> : "Into slot…"}
    </button>
  );
}

/** The inline list, placed by its surface where it pushes what follows (UX notes 8.2, V20). */
export function IntoSlotList({
  ctx,
  resolve,
  caption,
  close,
  style,
}: {
  ctx: SlotContext;
  /** The source to put, resolved when the list opens or a row is taken. */
  resolve: SlotResolve;
  /** A short name of the unit for the caption ("Lid", "3 elements"). */
  caption: string;
  /** Closes the list and returns the focus to its button. */
  close: () => void;
  /** Its place in a virtualised list. */
  style?: React.CSSProperties;
}) {
  const [realSize, setRealSize] = useState(false);
  const [size, setSize] = useState<[number, number] | null>(null);
  // A whole page can be placed live instead of copied (`specs/0154` slice 2, V5); session only.
  const [whole, setWhole] = useState(false);
  const [placed, setPlaced] = useState(false);
  const [focus, setFocus] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const arm = useArm("into-slot-row");
  // The source is read once, when the list opens.
  const first = useRef(resolve);
  useEffect(() => {
    let live = true;
    void first.current().then((got) => {
      if (got && live) {
        setSize(unitSize(got.source));
        setWhole(got.source.whole);
      }
      got?.done();
    });
    return () => {
      live = false;
    };
  }, []);
  const session = ctx.getSession();
  const targets = useMemo(
    () => (session ? slotTargets(session, size, realSize) : { holders: [], selected: null }),
    [session, size, realSize],
  );
  const rows: Row[] = intoSlotRows(targets);
  const opened = useRef(false);
  useEffect(() => {
    if (opened.current) {
      return;
    }
    opened.current = true;
    const at = rows.findIndex((r) => r.id === targets.selected);
    setFocus(at >= 0 ? at : 0);
    list.current?.focus();
  }, [rows, targets.selected]);
  const take = (row: Row | undefined) => {
    if (!row) {
      return;
    }
    const run = (replace: boolean) =>
      void resolve().then((got) => {
        if (!got) {
          return;
        }
        if (row.kind === "fill") {
          fillSlots(ctx, got.source, row.id, realSize);
          got.done();
        } else {
          putIntoSlot(ctx, got.source, row.id, realSize, replace, got.done, placed);
        }
        close();
      });
    if (row.filled) {
      arm.press(() => {
        run(true);
        return "";
      });
      return;
    }
    run(false);
  };
  const keyDown = (event: React.KeyboardEvent) => {
    const action = intoSlotKey(event.key, event.repeat, focus, rows.length);
    if (action === null) {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
      }
      return;
    }
    event.preventDefault();
    if ("move" in action) {
      setFocus(action.move);
    } else if ("take" in action) {
      take(rows[action.take]);
    } else {
      event.stopPropagation();
      if (!arm.armed) {
        close();
      }
    }
  };
  return (
    <div
      className={style ? "into-slot-list into-slot-in-tree" : "into-slot-list"}
      style={style}
      onClick={(event) => event.stopPropagation()}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) {
          close();
        }
      }}
      {...arm.zone}
    >
      <div className="into-slot-caption">
        <span>{size ? `${caption} · ${size[0].toFixed(0)} × ${size[1].toFixed(0)} mm` : caption}</span>
        {whole ? (
          <button type="button" aria-pressed={placed} title="Place the page live: it follows the source page" onClick={() => setPlaced((p) => !p)}>
            Placed page
          </button>
        ) : null}
        <button type="button" aria-pressed={realSize} onClick={() => setRealSize((r) => !r)}>
          Real size
        </button>
      </div>
      <div
        ref={list}
        role="listbox"
        aria-label="Into slot"
        tabIndex={0}
        aria-activedescendant={rows[focus] ? `into-slot-${rows[focus].key}` : undefined}
        onKeyDown={keyDown}
      >
        {targets.holders.map((holder) => (
          <div key={holder.id} role="group" aria-label={holder.name}>
            <div className="into-slot-heading" aria-hidden>
              <span>
                {holder.name}
                {holder.hidden ? " · hidden" : ""}
              </span>
              <span>
                {holder.filled} of {holder.of} filled
              </span>
            </div>
            {rows
              .filter((r) => (r.kind === "fill" ? r.id === holder.id : holder.slots.some((s) => s.id === r.id)))
              .map((row) => {
                const at = rows.indexOf(row);
                const slot = holder.slots.find((s) => s.id === row.id);
                const armedHere = arm.armed && row.filled && focus === at;
                return (
                  <div
                    key={row.key}
                    id={`into-slot-${row.key}`}
                    role="option"
                    aria-selected={focus === at}
                    aria-label={row.label}
                    className={`into-slot-row${focus === at ? " current" : ""}${armedHere ? " armed" : ""}`}
                    {...(row.filled ? arm.control : {})}
                    onClick={() => {
                      setFocus(at);
                      take(row);
                    }}
                  >
                    {row.kind === "fill" ? (
                      <>
                        <Grid2x2Plus size={16} />
                        <span className="into-slot-name">Fill empty slots</span>
                        <span className="into-slot-outcome">
                          {holder.of - holder.filled} of {holder.of}
                        </span>
                      </>
                    ) : (
                      <>
                        {slot?.shape === "circle" ? <Circle size={16} /> : <Square size={16} />}
                        <span className="into-slot-name">{slot?.name}</span>
                        <span className="into-slot-size">{slot?.size}</span>
                        <span className="into-slot-outcome">
                          {armedHere ? <span className="into-slot-replace">Replace</span> : slot?.filled ? "filled" : slot?.outcome}
                        </span>
                      </>
                    )}
                  </div>
                );
              })}
          </div>
        ))}
      </div>
    </div>
  );
}

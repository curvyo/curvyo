import { CircleAlert, Link, Unlink } from "lucide-react";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";

import { EntryField } from "@/components/EntryField";
import type { EditorSession } from "@/hooks/useEditorSession";
import { placeEntryChip } from "@/lib/readoutPlacement";
import { type SketchKnobEntry, decodeKnobEntry } from "@/lib/sketchInstanceView";

/** The tool rail's clearance from the canvas's left edge, px (`--rail-right` + 12). */
const TOOL_RAIL_CLEAR_PX = 128;

type Editor = Pick<EditorSession, "getSession" | "sync" | "syncRevision" | "containerRef">;

/**
 * The entry chip of a sketch clone's knob (`specs/0099-sketch-pages` criterion 25, UX notes 12.3
 * and 12.9.4; the "Transform entry chip" surface of the tiling and move chips): one field with the
 * value's name and, for a linked value, its link mark. Enter commits one step "Change parameter"
 * coupled through the links; Escape and a blur close it and write nothing.
 */
export function SketchKnobEntryChip({ editor }: { editor: Editor }) {
  const { getSession, sync, syncRevision, containerRef } = editor;
  void syncRevision;
  const entry = decodeKnobEntry(getSession()?.sketch_knob_entry_json() ?? "");
  return entry ? <Chip key={`${entry.key}:${entry.x}:${entry.y}`} entry={entry} editor={{ getSession, sync, containerRef }} /> : null;
}

function Chip({ entry, editor }: { entry: SketchKnobEntry; editor: Pick<Editor, "getSession" | "sync" | "containerRef"> }) {
  const { getSession, sync, containerRef } = editor;
  const chipRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const messageId = useId();
  const decimals = entry.unit === "deg" ? 1 : 2;
  const [text, setText] = useState(() => String(Number(entry.value.toFixed(decimals))));
  const [invalid, setInvalid] = useState(false);
  const [sizes, setSizes] = useState({ chip: { width: 0, height: 0 }, canvas: { width: Infinity, height: Infinity } });

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);
  useLayoutEffect(() => {
    const chip = chipRef.current;
    const container = containerRef.current;
    if (!chip || !container) {
      return;
    }
    const next = {
      chip: { width: chip.offsetWidth, height: chip.offsetHeight },
      canvas: { width: container.clientWidth, height: container.clientHeight },
    };
    setSizes((previous) =>
      previous.chip.width === next.chip.width && previous.chip.height === next.chip.height &&
      previous.canvas.width === next.canvas.width && previous.canvas.height === next.canvas.height
        ? previous
        : next,
    );
  }, [entry, invalid, containerRef]);

  const placed = placeEntryChip({ x: entry.x, y: entry.y }, { x: entry.x, y: entry.y + 1 }, sizes.chip, sizes.canvas);
  const close = () => {
    getSession()?.cancel_sketch_knob_entry();
    sync();
  };
  const commit = () => {
    const session = getSession();
    if (!session) {
      return;
    }
    const answer = JSON.parse(session.commit_sketch_knob_entry(text)) as { error?: string };
    if (answer.error === "number") {
      setInvalid(true);
      inputRef.current?.select();
      return;
    }
    sync();
  };
  const Glyph = entry.glyph === "link" ? Link : entry.glyph === "unlink" ? Unlink : null;
  const label = (
    <span aria-hidden className="pointer-events-none absolute top-1/2 left-1.5 flex -translate-y-1/2 items-center gap-1 text-xs">
      {entry.label}
      {Glyph && <Glyph size={12} strokeWidth={1.5} />}
    </span>
  );
  return (
    <div
      ref={chipRef}
      role="group"
      aria-label={entry.label}
      data-sketch-knob-entry=""
      className="absolute z-30 rounded-[8px] p-1.5 outline-none"
      style={{
        left: Math.max(placed.left, TOOL_RAIL_CLEAR_PX),
        top: placed.top,
        background: "var(--toolbar-bg)",
        boxShadow: "var(--panel-elevation-shadow)",
        color: "var(--toolbar-icon)",
      }}
    >
      <EntryField
        label={label}
        width={180}
        labelWidth={110}
        suffix={entry.unit === "deg" ? "°" : "mm"}
        ariaLabel={`${entry.label}${entry.glyph === "link" ? ", linked" : entry.glyph === "unlink" ? ", unlinked" : ""}`}
        invalid={invalid}
        messageId={messageId}
        value={text}
        inputRef={(node) => {
          inputRef.current = node;
        }}
        onChange={(next) => {
          setText(next);
          setInvalid(false);
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          } else if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
        onBlur={close}
      />
      <div id={messageId} role="status" aria-live="polite" className={invalid ? "absolute top-full left-0 mt-1 flex w-max items-center gap-1 rounded-[8px] px-1.5 py-1 text-xs" : "absolute h-0 overflow-hidden text-xs"}
        style={invalid ? { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" } : undefined}>
        {invalid && (
          <>
            <CircleAlert aria-hidden className="size-3 shrink-0" />
            <span>Enter a number</span>
          </>
        )}
      </div>
    </div>
  );
}

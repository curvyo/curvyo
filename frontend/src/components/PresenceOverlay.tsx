import { MousePointer2, PenTool, Square, Type } from "lucide-react";
import { type ComponentType, useEffect, useState } from "react";

import type { WasmSession } from "@/lib/editorSession";

/** One other participant's marks, in canvas CSS pixels (the session's `presence_marks_json`). */
interface PeerMark {
  label: string;
  colour: number;
  tool: string;
  pointer: [number, number] | null;
  selection: [number, number, number, number] | null;
  marker: string | null;
}

/** The small tool icon in a chip while the tool is not Select (criterion 16, Proposal). */
const TOOL_GLYPHS: Record<string, ComponentType<{ size: number; strokeWidth: number; "aria-hidden": boolean }>> = {
  pen: PenTool,
  rectangle: Square,
  text: Type,
};

function cut(text: string, max: number): string {
  return [...text].length <= max ? text : `${[...text].slice(0, max - 1).join("")}…`;
}

function Chip({ mark, text }: { mark: PeerMark; text: string }) {
  const Glyph = mark.tool !== "select" ? TOOL_GLYPHS[mark.tool] : undefined;
  return (
    <span
      className="inline-flex items-center gap-1 whitespace-nowrap rounded-[4px] px-1.5 py-0.5 text-xs font-bold text-white"
      style={{ background: `var(--peer-${mark.colour})` }}
    >
      {text}
      {Glyph ? <Glyph size={14} strokeWidth={1.5} aria-hidden /> : null}
    </span>
  );
}

/**
 * The other participants on the canvas (`specs/0092-cooperative-editing` criteria 14 to 16, UX
 * notes §5): a pointer with a label chip, the selection's box in the participant's colour, and
 * the chip grown to "Ben is editing this" over a contested object. Presence only: nothing here
 * is document content, and nothing intercepts the pointer.
 */
export function PresenceOverlay({ getSession }: { getSession: () => WasmSession | null }) {
  const [marks, setMarks] = useState<PeerMark[]>([]);
  useEffect(() => {
    let last = "[]";
    let frame = requestAnimationFrame(function read() {
      frame = requestAnimationFrame(read);
      const text = getSession()?.presence_marks_json() ?? "[]";
      if (text !== last) {
        last = text;
        setMarks(JSON.parse(text) as PeerMark[]);
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [getSession]);
  if (marks.length === 0) {
    return null;
  }
  return (
    <div aria-hidden className="pointer-events-none absolute inset-0 overflow-hidden">
      {marks.map((mark, index) => (
        <div key={`${mark.label}-${index}`}>
          {mark.selection ? (
            <div
              className="absolute"
              style={{
                left: mark.selection[0] - 2 - 3 * index,
                top: mark.selection[1] - 2 - 3 * index,
                width: mark.selection[2] + 4 + 6 * index,
                height: mark.selection[3] + 4 + 6 * index,
                border: `1.5px solid var(--peer-${mark.colour})`,
                boxShadow: "0 0 0 1px var(--selection-casing)",
              }}
            >
              <span className="absolute -top-6 left-0">
                <Chip mark={mark} text={mark.marker ? cut(mark.marker, 36) : cut(mark.label, 16)} />
              </span>
            </div>
          ) : null}
          {mark.pointer ? (
            <div className="absolute" style={{ left: mark.pointer[0], top: mark.pointer[1] }}>
              <MousePointer2
                size={16}
                strokeWidth={1}
                aria-hidden
                style={{ fill: `var(--peer-${mark.colour})`, color: "var(--selection-casing)" }}
              />
              <span className="absolute top-4 left-3">
                <Chip mark={mark} text={cut(mark.label, 16)} />
              </span>
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}

// The Pen strip (`specs/0158-pen-display` slice 2b criterion 3, UX 10.3, design system "Pen strip"):
// with the Pencil and the Eraser the layer pick, Max width and Smoothing at 48 px and the slice-1
// "No pen pressure" state; with Node the Node bar's pen subset; with Select nothing.
import { type ReactNode, useSyncExternalStore } from "react";

import { TraceField } from "@/components/TraceField";
import type { EditorSession } from "@/hooks/useEditorSession";
import { SMOOTHING_PARAM } from "@/lib/penModes";
import { openPenProbe } from "@/lib/penProbeStore";
import {
  MAX_RANGE,
  PENCIL_DEFAULTS,
  pencilSettings,
  setPencilSettings,
  subscribePencilSettings,
  withMax,
} from "@/lib/pencilSettings";
import type { TraceParam } from "@/lib/traceText";
import { PenLayerPick } from "@/pen/PenLayerPick";
import { type PenTool, readPenLayers } from "@/pen/penLayout";

const MAX_PARAM: TraceParam = {
  name: "tolerance",
  label: "Max",
  kind: "length",
  unit: "mm",
  min: MAX_RANGE[0],
  max: MAX_RANGE[1],
  default: PENCIL_DEFAULTS.max,
  step: 0.1,
  big_step: 1,
  fine_step: 0.01,
  group: "pencil",
  advanced: false,
  choices: [],
};

function Divider() {
  return (
    <span aria-hidden className="mx-3 h-5 w-px" style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }} />
  );
}

/** The strip for `tool`; `node` is the Node bar's pen subset, shown with the Node tool. */
export function PenStrip({ editor, tool, node }: { editor: EditorSession; tool: PenTool; node: ReactNode }) {
  const settings = useSyncExternalStore(subscribePencilSettings, pencilSettings);
  if (tool === "select") return null;
  if (tool === "node") return <>{node}</>;
  const session = editor.getSession();
  const rows = readPenLayers(session?.pen_layers_json() ?? "[]");
  const back = () => editor.containerRef.current?.focus();
  return (
    <div
      role="toolbar"
      aria-label="Pen options"
      className="pointer-events-auto flex h-14 min-w-0 shrink items-center rounded-lg px-1 text-sm"
      style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}
    >
      <PenLayerPick
        rows={rows}
        onPick={(id) => {
          editor.getSession()?.set_pen_layer(JSON.stringify(id));
          editor.sync();
        }}
      />
      <Divider />
      <TraceField
        param={MAX_PARAM}
        name="Maximum width"
        tall
        value={settings.max}
        onValue={(value) => setPencilSettings(withMax(pencilSettings(), value))}
        onEndDrag={() => undefined}
        onCreate={() => undefined}
        onReturnFocus={back}
      />
      <Divider />
      <TraceField
        param={SMOOTHING_PARAM}
        name="Pencil smoothing"
        tall
        value={settings.smoothing}
        onValue={(value) => setPencilSettings({ ...pencilSettings(), smoothing: value })}
        onEndDrag={() => undefined}
        onCreate={() => undefined}
        onReturnFocus={back}
      />
      {settings.penSeen && settings.noPressure ? (
        <>
          <Divider />
          <span role="status" className="text-sm whitespace-nowrap text-[var(--panel-muted-fg)]">
            No pen pressure
          </span>
          <button
            type="button"
            onClick={openPenProbe}
            className="ml-2 h-12 rounded-md px-3 text-sm whitespace-nowrap text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
          >
            Pen probe...
          </button>
        </>
      ) : null}
    </div>
  );
}

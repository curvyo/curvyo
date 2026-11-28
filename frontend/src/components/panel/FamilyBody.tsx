import { EntryField } from "@/components/panel/EntryField";
import { PresetStrip } from "@/components/panel/PresetStrip";
import type { FamilyBody, OperationRow } from "@/lib/operationsView";

interface FamilyBodyProps {
  row: OperationRow;
  body: FamilyBody;
  /** Types `text` into the row's field `field`: the field message, empty when it took. */
  onEdit: (field: string, text: string) => string;
  onReturnToRow: () => void;
}

/** One number field of a family row: the laser's white `EntryField` (UX unify 8). */
interface NumberField {
  key: string;
  label: string;
  unit: string;
}

const PLOTTER: NumberField[] = [
  { key: "pressure", label: "Pressure", unit: "%" },
  { key: "speed", label: "Speed", unit: "mm/s" },
  { key: "passes", label: "Passes", unit: "×" },
];

const EMBROIDERY: Record<string, NumberField[]> = {
  running: [{ key: "stitch_length_mm", label: "Stitch length", unit: "mm" }],
  tatami: [
    { key: "angle_deg", label: "Angle", unit: "°" },
    { key: "row_spacing_mm", label: "Row spacing", unit: "mm" },
    { key: "max_stitch_mm", label: "Max stitch", unit: "mm" },
  ],
  satin: [{ key: "zigzag_spacing_mm", label: "Spacing", unit: "mm" }],
};

const CNC: NumberField[] = [
  { key: "depth_mm", label: "Depth", unit: "mm" },
  { key: "feed_mm_min", label: "Feed", unit: "mm/min" },
  { key: "plunge_mm_min", label: "Plunge", unit: "mm/min" },
];

function text(value: unknown): string {
  return value === null || value === undefined ? "" : String(value);
}

/**
 * The unfolded body of a plotter, embroidery or CNC row (ADR 0098 decision 3, UX unify 6 and 8):
 * the family's numbers in the laser's white fields, its choices as Choice strips, the thread as a
 * hex field and the CNC tool as a strip of the page's tools. Every commit is the family's own step.
 */
export function FamilyBodyView({ row, body, onEdit, onReturnToRow }: FamilyBodyProps) {
  const submit = (key: string) => (typed: string) => {
    const message = onEdit(key, typed);
    return message === "" ? "committed" : `invalid:${key}`;
  };
  const numberFields = (fields: NumberField[], values: Record<string, unknown>) =>
    fields.map((field) => {
      const shown = text(values[field.key]);
      return (
        <EntryField
          key={field.key}
          label={`${field.label}, ${field.unit}`}
          prefix={field.label}
          shown={shown}
          mixed={false}
          suffix={field.unit}
          width={228}
          onSubmit={submit(field.key)}
          messages={{ [field.key]: `Check ${field.label.toLowerCase()}.` }}
          onReturnFocus={onReturnToRow}
          commitOnBlur
          inputMode="decimal"
        />
      );
    });
  if (body.family === "plotter") {
    const fields = body.kind === "cut" ? PLOTTER : PLOTTER.slice(0, 2);
    return <div className="flex flex-col gap-2 py-2">{numberFields(fields, body.fields)}</div>;
  }
  if (body.family === "embroidery") {
    const fields = EMBROIDERY[body.kind] ?? [];
    return (
      <div className="flex flex-col gap-2 py-2">
        <EntryField
          label="Thread, hex"
          prefix="Thread"
          shown={text(body.fields.thread ?? row.thread)}
          mixed={false}
          width={228}
          align="left"
          onSubmit={submit("thread")}
          messages={{ thread: "Enter a colour as #RRGGBB." }}
          onReturnFocus={onReturnToRow}
          commitOnBlur
        />
        {body.kind === "running" ? (
          <PresetStrip
            label="Repeats"
            cells={["1", "3", "5"].map((n) => ({ id: n, label: `${n} ×`, tooltip: `${n} ×`, content: `${n} ×` }))}
            value={text(body.fields.repeats)}
            onPress={(id) => onEdit("repeats", id)}
            onReturnFocus={onReturnToRow}
            width="w-[228px]"
          />
        ) : null}
        {numberFields(fields, body.fields)}
      </div>
    );
  }
  const values = body.fields;
  return (
    <div className="flex flex-col gap-2 py-2">
      <PresetStrip
        label="Side"
        cells={[
          { id: "outside", label: "Outside", tooltip: "Outside", content: "Outside" },
          { id: "inside", label: "Inside", tooltip: "Inside", content: "Inside" },
        ]}
        value={text(values.side)}
        onPress={(id) => onEdit("side", id)}
        onReturnFocus={onReturnToRow}
        width="w-[228px]"
      />
      <PresetStrip
        label="Direction"
        cells={[
          { id: "climb", label: "Climb", tooltip: "Climb", content: "Climb" },
          { id: "conventional", label: "Conventional", tooltip: "Conventional", content: "Conventional" },
        ]}
        value={text(values.direction)}
        onPress={(id) => onEdit("direction", id)}
        onReturnFocus={onReturnToRow}
        width="w-[228px]"
      />
      {body.tools.length > 1 ? (
        <PresetStrip
          label="Tool"
          cells={body.tools.map((tool) => ({ id: tool.id, label: tool.name, tooltip: tool.line2, content: tool.name }))}
          value={text(values.tool)}
          onPress={(id) => onEdit("tool", id)}
          onReturnFocus={onReturnToRow}
          width="w-[228px]"
        />
      ) : null}
      {numberFields(CNC, values)}
    </div>
  );
}

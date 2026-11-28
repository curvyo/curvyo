import { EntryField } from "@/components/panel/EntryField";
import type { PrintAndCutApi } from "@/hooks/usePrintAndCut";
import { parseMeasure } from "@/lib/printAndCutView";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";

const SIZE_MESSAGES = { range: "Between 3 and 20 mm" } as const;
const LINE_MESSAGES = { range: "Between 0.05 and 1 mm" } as const;

interface Props {
  api: PrintAndCutApi;
  onReturnFocus: () => void;
}

/**
 * The Registration marks block of the Style tab (`specs/0121-print-and-cut` UX notes §3.3): the
 * count, Size and Line width for every mark of the object in one step, and Add mark n. It
 * replaces Stroke, Fill and Markers for a marks object.
 */
export function RegistrationMarksBlock({ api, onReturnFocus }: Props) {
  const marks = api.marks;
  if (!marks) return null;
  const range = (text: string, low: number, high: number): number | null => {
    const value = parseMeasure(text);
    return typeof value === "number" && value >= low && value <= high ? value : null;
  };
  return (
    <div role="group" aria-label="Registration marks" className="flex flex-col gap-1" data-registration-marks>
      <div className="flex h-6 items-center">
        <span className="text-sm font-semibold text-[var(--toolbar-icon)]">Registration marks</span>
        <span className="ml-auto text-xs text-[var(--panel-muted-fg)]">
          {marks.mark !== null ? `Mark ${marks.mark} of ${marks.count} marks` : `${marks.count} marks`}
        </span>
      </div>
      <EntryField
        label="Size"
        prefix="Size"
        shown={marks.diameter.toFixed(2)}
        mixed={false}
        suffix="mm"
        width={148}
        commitOnBlur
        inputMode="decimal"
        onSubmit={(text) => {
          const value = range(text, 3, 20);
          if (value === null) return "invalid:range";
          return api.setMarkSize(value, marks.line) ? "committed" : "unchanged";
        }}
        messages={SIZE_MESSAGES}
        onReturnFocus={onReturnFocus}
      />
      <EntryField
        label="Line width"
        prefix="Line width"
        shown={marks.line.toFixed(2)}
        mixed={false}
        suffix="mm"
        width={148}
        commitOnBlur
        inputMode="decimal"
        onSubmit={(text) => {
          const value = range(text, 0.05, 1);
          if (value === null) return "invalid:range";
          return api.setMarkSize(marks.diameter, value) ? "committed" : "unchanged";
        }}
        messages={LINE_MESSAGES}
        onReturnFocus={onReturnFocus}
      />
      {marks.add !== null && (
        <Button icon={Plus}
          onClick={() => api.addMark()} className="w-full"
        >
          Add mark {marks.add}
        </Button>
      )}
    </div>
  );
}

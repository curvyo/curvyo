import { useLayoutEffect, useRef } from "react";

import { OperationOptionsPart, OptionFieldView } from "@/components/panel/OperationOptionsPart";
import { OperationSettingsBlock } from "@/components/panel/OperationSettingsBlock";
import type { OperationSettingsApi } from "@/hooks/useOperationSettings";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import { INTERVAL_FIELD, optionText } from "@/lib/operationOptionRows";
import type { PickMaterial } from "@/lib/operationSettingsView";
import type { OperationRow } from "@/lib/operationsView";

interface OperationBodyProps {
  row: OperationRow;
  pick: readonly PickMaterial[];
  settings: OperationSettingsApi;
  tools: OperationToolsApi;
  onEditMaterial: (id: string) => void;
  /** Escape or Enter in a field, a pick or a switch: the keyboard goes back to the row. */
  onReturnToRow: () => void;
  /** The row's key, given back with the height so the list knows which body was measured. */
  rowKey: string;
  /** The body's height once drawn, for the window's one tall item (`ux-notes.md` 6.1). */
  onHeight: (key: string, height: number) => void;
}

/**
 * The body of the unfolded row (`specs/0111-laser-operation-tools` `ux-notes.md` 6): the upper
 * part is the Settings block of `0028` with Interval beside Passes for the hatching kinds; under
 * a hairline the lower part holds the operation-only settings.
 */
export function OperationBody({ row, pick, settings, tools, onEditMaterial, onReturnToRow, rowKey, onHeight }: OperationBodyProps) {
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = box.current;
    if (!element) return undefined;
    const report = () => onHeight(rowKey, Math.ceil(element.getBoundingClientRect().height));
    report();
    const observer = new ResizeObserver(report);
    observer.observe(element);
    return () => observer.disconnect();
  }, [onHeight, rowKey]);
  const intervalMm = row.settings?.intervalMm ?? null;
  const interval =
    intervalMm === null ? undefined : (
      <OptionFieldView
        field={{ ...INTERVAL_FIELD, shown: optionText(intervalMm) }}
        row={row}
        tools={tools}
        onReturnFocus={onReturnToRow}
      />
    );
  return (
    <div
      ref={box}
      data-row-body=""
      onKeyDown={(event) => {
        // Escape in the body (outside a field, which reverts and hands the focus back itself).
        if (event.key === "Escape" && !(event.target instanceof HTMLInputElement)) {
          event.preventDefault();
          event.stopPropagation();
          onReturnToRow();
        }
      }}
      className="ml-3 flex flex-col gap-1 pb-2 pr-1 pt-1"
    >
      <OperationSettingsBlock
        embedded
        rows={[row]}
        pick={pick}
        api={settings}
        onEditMaterial={onEditMaterial}
        onReturnFocus={onReturnToRow}
        interval={interval}
      />
      <OperationOptionsPart row={row} tools={tools} onReturnFocus={onReturnToRow} />
    </div>
  );
}

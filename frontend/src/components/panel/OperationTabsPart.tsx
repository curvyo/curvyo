import { RotateCcw } from "lucide-react";

import { OverrideRow } from "@/components/panel/CloneBlock";
import { EntryField } from "@/components/panel/EntryField";
import { OptionFieldView } from "@/components/panel/OperationOptionsPart";
import { OperationItemPathRows } from "@/components/panel/OperationPathPart";
import { TabsGlyph } from "@/components/panel/OperationGlyphs";
import { Tooltip } from "@/components/ui/tooltip";
import { PanelSwitch } from "@/components/ui/switch";
import type { OperationToolsApi } from "@/hooks/useOperationTools";
import { TAB_FIELDS, optionText } from "@/lib/operationOptionRows";
import type { OperationRow, TabChip } from "@/lib/operationsView";
import { CHIP_IS_MESSAGE, outcomeOf, typedNumber } from "@/lib/settingsFieldText";

const HAIRLINE = { background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" };

/**
 * The Tabs group of an unfolded Cut or Cut-out row (`specs/0139` criteria 1 to 3, `ux-notes.md`
 * 2): the heading, Count and Width, the muted placed line when some path gets fewer tabs, and
 * Skip inner shapes. Absent on every other kind (hidden, not disabled).
 */
export function OperationTabsGroup({ row, tools, onReturnFocus }: { row: OperationRow; tools: OperationToolsApi; onReturnFocus: () => void }) {
  const tabs = row.settings?.options?.tabs ?? null;
  if (tabs === null) return null;
  const [count, width] = TAB_FIELDS as [(typeof TAB_FIELDS)[number], (typeof TAB_FIELDS)[number]];
  return (
    <div className="flex flex-col gap-1">
      <div aria-hidden className="my-1 h-px" style={HAIRLINE} />
      <div className="flex h-6 items-center gap-1 text-xs font-semibold text-[var(--toolbar-icon)]">
        <TabsGlyph size={14} />
        Tabs
      </div>
      <div className="flex flex-wrap gap-1">
        <OptionFieldView field={{ ...count, shown: String(tabs.count) }} row={row} tools={tools} onReturnFocus={onReturnFocus} />
        <OptionFieldView field={{ ...width, shown: optionText(tabs.widthMm) }} row={row} tools={tools} onReturnFocus={onReturnFocus} />
      </div>
      {tabs.placed !== null ? <p className="text-xs text-[var(--panel-muted-fg)]">{tabs.placed}</p> : null}
      <PanelSwitch
        label="Skip inner shapes"
        checked={tabs.skipInner}
        onCheckedChange={(on) => tools.edit(row.id, "tab_skip_inner", on ? 1 : 0)}
        onReturnFocus={onReturnFocus}
      />
    </div>
  );
}

/**
 * The item part of an unfolded Cut or Cut-out row (`specs/0139` criteria 5, 6, 19 to 21,
 * `ux-notes.md` 3): the Tabs pill "Operation's | Own" with its readout for the selected items,
 * while a tab handle is current its Width field with the reset to the operation's width, then the
 * Path options, Start and Direction rows.
 */
export function OperationItemPart({
  row,
  chip,
  tools,
  onReturnFocus,
}: {
  row: OperationRow;
  chip: TabChip | null;
  tools: OperationToolsApi;
  onReturnFocus: () => void;
}) {
  const item = row.itemTabs;
  if (item === null) return null;
  return (
    <div
      role="group"
      aria-label={`Selected items in ${row.name}`}
      // Room under the Width field for its message chip, which hangs below the field and would
      // be cut off at the end of the list otherwise.
      className={`flex flex-col gap-1 ${chip ? "pb-7" : ""}`}
    >
      <div aria-hidden className="my-1 h-px" style={HAIRLINE} />
      <OverrideRow
        label="Tabs"
        segments={[
          { own: false, label: "Operation's", name: "Operation's", title: "Operation's" },
          { own: true, label: "Own", name: "Own", title: "Own" },
        ]}
        state={item.own === null ? "mixed" : item.own ? "own" : "original"}
        onSet={(own) => tools.setTabsOwn(row.id, own)}
        onReturnFocus={onReturnFocus}
        onPressedAt={() => undefined}
      />
      <p className="text-xs text-[var(--panel-muted-fg)]">{item.readout}</p>
      {chip ? (
        <div className="flex items-center gap-1">
          <EntryField
            label="Tab width, millimetres"
            prefix="Width"
            shown={optionText(chip.widthMm)}
            mixed={false}
            suffix="mm"
            width={128}
            gutter={32}
            messageFromLeft
            commitOnBlur
            inputMode="decimal"
            onSubmit={(text) => {
              const mm = typedNumber(text);
              return Number.isFinite(mm) ? outcomeOf(tools.setTabWidth(mm)) : outcomeOf("Enter a number.");
            }}
            messages={CHIP_IS_MESSAGE}
            onReturnFocus={onReturnFocus}
          />
          {chip.ownWidth ? (
            <Tooltip side="bottom" content="Operation's width">
              <button
                type="button"
                tabIndex={-1}
                aria-label="Operation's width"
                onClick={() => {
                  tools.setTabWidth(null);
                  onReturnFocus();
                }}
                className="flex size-7 items-center justify-center rounded-md text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]"
              >
                <RotateCcw aria-hidden size={16} strokeWidth={1.5} absoluteStrokeWidth />
              </button>
            </Tooltip>
          ) : null}
        </div>
      ) : null}
      <OperationItemPathRows row={row} tools={tools} onReturnFocus={onReturnFocus} />
    </div>
  );
}

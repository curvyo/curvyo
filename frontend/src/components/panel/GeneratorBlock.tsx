import { EntryField } from "@/components/panel/EntryField";
import { PanelSwitch } from "@/components/ui/switch";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { GeneratorBlockApi } from "@/hooks/useGenerators";
import {
  type GeneratedRow,
  parseNumber,
  rowRange,
  rowText,
  stepRowText,
  unitOf,
} from "@/lib/generatedView";

const MESSAGES = { number: "Enter a number" } as const;
const WIDTH = 244;

/** One number row: the Params field of the schema-rendered tab (`specs/0141` UX notes §4). */
function NumberRow({
  row,
  api,
  onReturnFocus,
}: {
  row: GeneratedRow;
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  return (
    <EntryField
      label={row.label}
      shown={rowText(row)}
      mixed={false}
      suffix={unitOf(row.kind)}
      width={WIDTH}
      prefix={row.label}
      align="right"
      inputMode="decimal"
      range={rowRange(row) ?? undefined}
      commitOnBlur
      messages={MESSAGES}
      onReturnFocus={onReturnFocus}
      stepText={(text, key, shift, ctrl) => {
        const next = stepRowText(row, text, key, shift, ctrl);
        // The canvas follows the arrow key before the key-up commits it (UX notes §7).
        const number = next === null ? null : parseNumber(next);
        if (number !== null) {
          api.preview(row.key, number);
        }
        return next;
      }}
      onClose={api.cancelPreview}
      onSubmit={(text) => {
        const number = parseNumber(text);
        return number === null ? "invalid:number" : api.setValue(row.key, number);
      }}
    />
  );
}

function ChoiceRow({
  row,
  api,
  onReturnFocus,
}: {
  row: GeneratedRow;
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  if (row.kind.kind !== "choice") {
    return null;
  }
  const options: ToggleOption<string>[] = row.kind.options.map((option) => ({
    value: option.id,
    label: option.label,
    tooltip: "",
    icon: <span className="text-sm">{option.label}</span>,
  }));
  return (
    <ToggleGroup
      label={row.label}
      options={options}
      value={typeof row.value === "string" ? row.value : null}
      onChange={(next) => api.setValue(row.key, next)}
      itemWidth={Math.floor(WIDTH / options.length)}
      onReturnFocus={onReturnFocus}
    />
  );
}

/** A clamp notice under the edited row (UX notes §6): the "Action notice" status look, an
 * overlay that moves no row and takes no focus. */
function ClampNotice({ text }: { text: string }) {
  return (
    <p
      role="status"
      className="pointer-events-none absolute top-full right-0 z-10 mt-1 w-max max-w-[244px] break-words rounded-md p-2 text-right text-xs"
      style={{
        background: "var(--toolbar-bg)",
        color: "var(--toolbar-icon)",
        boxShadow: "var(--panel-elevation-shadow)",
      }}
    >
      {text}
    </p>
  );
}

/**
 * The Generator block of the Style tab (`specs/0141-parametric-generators` criteria 1 to 4, UX
 * notes §3): drawn only from the generator's schema, a heading per group and a field per row,
 * then Convert to paths. A row that does not apply is not in the tree. The rules (clamping, the
 * derived Pitch diameter, the notice texts) are Rust's.
 */
export function GeneratorBlock({
  api,
  onReturnFocus,
}: {
  api: GeneratorBlockApi;
  onReturnFocus: () => void;
}) {
  const { view } = api;
  if (!view) {
    return null;
  }
  const groups: { label: string; rows: GeneratedRow[] }[] = [];
  for (const row of view.rows) {
    const last = groups[groups.length - 1];
    if (last && last.label === row.group) {
      last.rows.push(row);
    } else {
      groups.push({ label: row.group, rows: [row] });
    }
  }
  return (
    <div role="group" aria-label={view.label} className="flex flex-col gap-4" data-generator-block>
      {groups.map((group) => (
        <section key={group.label} className="flex flex-col gap-2">
          <h3 className="text-sm text-[var(--toolbar-icon)]">{group.label}</h3>
          {group.rows.map((row) => (
            // Each row is the anchor of its clamp notice, 4 px under it (UX notes §6).
            <div key={row.key} className="relative w-fit">
              {row.kind.kind === "switch" ? (
                <div className="flex h-7 items-center">
                  <PanelSwitch
                    label={row.label}
                    checked={row.value === true}
                    onCheckedChange={(on) => api.setValue(row.key, on)}
                    onReturnFocus={onReturnFocus}
                  />
                </div>
              ) : row.kind.kind === "choice" ? (
                <ChoiceRow row={row} api={api} onReturnFocus={onReturnFocus} />
              ) : (
                <NumberRow row={row} api={api} onReturnFocus={onReturnFocus} />
              )}
              {api.notice?.key === row.key ? <ClampNotice text={api.notice.text} /> : null}
            </div>
          ))}
        </section>
      ))}
      <button
        type="button"
        data-generator-convert
        onClick={() => {
          api.convert();
          // The button leaves the tree with the block, by pointer or by key: focus goes to the
          // canvas (UX notes §11).
          onReturnFocus();
        }}
        className="h-7 w-[244px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        Convert to paths
      </button>
    </div>
  );
}

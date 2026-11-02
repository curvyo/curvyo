import { CircleAlert, Dices } from "lucide-react";
import type { ReactNode } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { GeneratorGlyph } from "@/components/panel/PatternIcons";
import { TileGroup, type Tile } from "@/components/panel/TileGroup";
import { ValueField } from "@/components/panel/ValueField";
import { PanelSwitch } from "@/components/ui/switch";
import { Tooltip } from "@/components/ui/tooltip";
import { ToggleGroup, type ToggleOption } from "@/components/ui/toggle-group";
import type { PatternFieldView, StylePanelApi } from "@/hooks/useStylePanel";

/** The generators in the picker's order (criterion 4). */
const GENERATORS: readonly [string, string][] = [
  ["hatch", "Hatch"],
  ["crosshatch", "Crosshatch"],
  ["grid", "Grid"],
  ["dots", "Dots"],
  ["honeycomb", "Honeycomb"],
  ["brick_wall", "Brick wall"],
  ["leaf_forest", "Leaf forest"],
  ["sea_waves", "Sea waves"],
  ["waves", "Waves"],
  ["interpolation_waves", "Interpolation waves"],
];

const TILES: readonly Tile[] = GENERATORS.map(([value, label]) => ({
  value,
  label,
  icon: <GeneratorGlyph id={value} />,
}));

const MODE_OPTIONS: readonly ToggleOption<"lines" | "shapes">[] = [
  { value: "lines", label: "Lines", tooltip: "Lines", icon: <span className="text-sm">Lines</span> },
  { value: "shapes", label: "Shapes", tooltip: "Shapes", icon: <span className="text-sm">Shapes</span> },
];

/** The keys are the session's (`set_pattern_field_text`): a cross-field key names the other field. */
const FIELD_MESSAGES = {
  range: "Enter a number in the field's range",
  number: "Enter a number",
  cell_size: "Must be smaller than the cell size",
  brick_height: "Must be smaller than the brick height",
  brick_width: "Must be smaller than the brick width",
  wall: "Must be larger than the wall",
  joint: "Must be larger than the joint",
} as const;
const SEED_MESSAGES = { seed: "Enter a whole number from 0 to 4294967295" } as const;

interface PatternBlockProps {
  panel: StylePanelApi;
  onReturnFocus: () => void;
  /** The fill's colour rows, shown in Shapes mode (UX notes §3, row 11). */
  colourRows: ReactNode;
}

function PatternValueField({
  field,
  panel,
  onReturnFocus,
}: {
  field: PatternFieldView;
  panel: StylePanelApi;
  onReturnFocus: () => void;
}) {
  return (
    <ValueField
      label={field.label}
      name={`Pattern ${field.label.toLowerCase()}`}
      text={field.text}
      bar={field.bar}
      barFrom={field.barFrom}
      zero={field.zero}
      mixed={field.mixed}
      resettable={field.resettable}
      unit={field.unit}
      valueNow={field.value}
      unitWords={field.unitWords}
      valueMin={field.valueMin}
      valueMax={field.valueMax}
      defaultText={field.defaultText}
      messages={FIELD_MESSAGES}
      onPreview={(p, grid) => panel.previewPattern(field.name, p, grid)}
      onSettle={panel.settle}
      onStep={(steps, grid) => panel.stepPattern(field.name, steps, grid)}
      onReset={() => panel.resetPattern(field.name)}
      onSubmit={(text) => panel.setPatternText(field.name, text)}
      onReturnFocus={onReturnFocus}
    />
  );
}

/**
 * The Pattern block of the Fill section (`specs/0071-pattern-fill` UX notes §3 to §10): the
 * state lines, the generator tiles, the generator's fields, Margin, Seed for a seeded generator,
 * Draw as, Whole pieces only for the generators of shapes (criterion 29), the colour rows in
 * Shapes mode, and Convert to paths. Every rule is the session's.
 */
export function PatternBlock({ panel, onReturnFocus, colourRows }: PatternBlockProps) {
  const { pattern } = panel;
  const refusal = panel.patternRefusal;
  const lines = refusal !== "" ? [refusal, ...pattern.state.filter((l) => !l.startsWith("Not generated"))] : pattern.state;
  const alert = refusal !== "" || pattern.alert;
  return (
    <div data-pattern-block className="flex flex-col gap-2">
      {/* Directly under the Fill kind row, so a refusal is in sight however long the block is
          (UI review 2026-11-02, item 1). */}
      {lines.length > 0 && (
        <div data-pattern-state className="flex flex-col gap-0.5 text-xs">
          {lines.map((line, index) =>
            index === 0 && alert ? (
              <p key={line} role="alert" className="flex items-center gap-1 text-[var(--field-invalid)]">
                <CircleAlert size={12} aria-hidden />
                {line}
              </p>
            ) : (
              <p key={line} className="text-[var(--panel-muted-fg)]">
                {line}
              </p>
            ),
          )}
        </div>
      )}
      <TileGroup
        label="Pattern generator"
        tiles={TILES}
        value={pattern.generator === "" ? null : pattern.generator}
        onChange={panel.setPatternGenerator}
        columns={2}
        onReturnFocus={onReturnFocus}
      />
      {pattern.fields.map((field) => (
        <PatternValueField key={field.name} field={field} panel={panel} onReturnFocus={onReturnFocus} />
      ))}
      {pattern.margin && <PatternValueField field={pattern.margin} panel={panel} onReturnFocus={onReturnFocus} />}
      {pattern.seeded && (
        <div className="flex h-7 items-center gap-2">
          <span className="w-[60px] shrink-0 text-xs text-[var(--toolbar-icon)]">Seed</span>
          <EntryField
            label="Pattern seed"
            shown={pattern.seed}
            mixed={pattern.seed === ""}
            width={136}
            align="left"
            onSubmit={panel.setPatternSeedText}
            messages={SEED_MESSAGES}
            onReturnFocus={onReturnFocus}
          />
          <Tooltip side="left" content="Re-roll seed">
            <button
              type="button"
              aria-label="Re-roll seed"
              data-pattern-reroll
              onClick={(event) => {
                panel.rerollPatternSeed();
                if (event.detail > 0) {
                  onReturnFocus();
                }
              }}
              className="flex h-7 w-7 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            >
              <Dices size={16} strokeWidth={1.5} aria-hidden />
            </button>
          </Tooltip>
        </div>
      )}
      <div className="flex h-7 items-center gap-2">
        <span className="w-[60px] shrink-0 text-xs text-[var(--toolbar-icon)]">Draw as</span>
        <ToggleGroup
          label="Draw as"
          options={MODE_OPTIONS}
          value={pattern.mode === "lines" || pattern.mode === "shapes" ? pattern.mode : null}
          onChange={panel.setPatternMode}
          itemWidth={88}
          onReturnFocus={onReturnFocus}
        />
      </div>
      {pattern.wholeOffered && (
        <div className="flex h-7 items-center" data-pattern-whole>
          <PanelSwitch
            label="Whole pieces only"
            checked={pattern.wholePieces === "on"}
            onCheckedChange={panel.setPatternWholePieces}
            onReturnFocus={onReturnFocus}
          />
        </div>
      )}
      {pattern.colourRows && colourRows}
      {pattern.convert && (
        <button
          type="button"
          data-pattern-convert
          onClick={() => {
            panel.convertPattern();
            onReturnFocus();
          }}
          className="h-7 w-[244px] rounded-[5px] border border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)] text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
        >
          Convert to paths
        </button>
      )}
    </div>
  );
}

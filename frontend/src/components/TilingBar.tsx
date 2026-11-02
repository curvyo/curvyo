import { CircleAlert, Dices, SlidersHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { Divider } from "@/components/SelectToolbar";
import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import type { TilingApi } from "@/hooks/useTiling";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import {
  ANNOUNCE_DELAY_MS,
  EDIT_TILING,
  ENTER_NUMBER,
  IMAGES_TEXT,
  KEEP_SOURCE,
  KEEP_SOURCE_RULE,
  NEW_SEED,
  REFUSED_TEXT,
  TILE,
  VARIANCE_ROWS,
  WARN_TEXT,
  type FieldRule,
  type TilingRecipe,
  type Triple,
  countText,
  readField,
  tilingAnnouncement,
  varianceSummary,
} from "@/lib/tilingText";

const BUTTON_CLASS =
  "h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";
const SELECTED_CLASS = "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]";
const GROUP_CLASS = "flex flex-wrap items-center gap-x-2 gap-y-1 py-1";
const LABEL_CLASS = "text-sm text-[var(--toolbar-icon)]";
const CHIP_CLASS = "absolute left-0 top-full z-30 mt-1 max-w-[320px] rounded-lg px-2 py-1 text-xs";
const CHIP_STYLE = { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)", color: "var(--toolbar-icon)" };

/** Up to three decimals, no trailing zeros, the real minus sign. */
function shown(value: number): string {
  const text = Number.isInteger(value) ? String(value) : String(Number(value.toFixed(3)));
  return text.replace("-", "−");
}

interface FieldProps {
  label: string;
  /** The accessible name with its unit ("X, millimetres"). */
  name: string;
  value: number;
  unit?: string;
  rule: FieldRule;
  width: number;
  /** Takes a valid value; `refused` marks the field when the session does not take it. */
  onValue: (value: number, refused: () => void) => void;
  inputRef?: React.RefObject<HTMLInputElement | null>;
  hideLabel?: boolean;
}

/** A bar number field (UX notes §2.1): every valid keystroke reaches the preview (debounced), an
 * invalid text, or one the session refuses, keeps the last valid preview and marks the field. */
function TilingField({ label, name, value, unit, rule, width, onValue, inputRef, hideLabel }: FieldProps) {
  const [text, setText] = useState(shown(value));
  const [error, setError] = useState<string | null>(null);
  const focused = useRef(false);
  useEffect(() => {
    if (!focused.current) {
      setText(shown(value));
      setError(null);
    }
  }, [value]);
  return (
    <label className="relative flex items-center gap-1">
      {hideLabel ? null : <span className={LABEL_CLASS}>{label}</span>}
      <span className="relative">
        <input
          ref={inputRef}
          aria-label={name}
          aria-invalid={error !== null}
          inputMode="decimal"
          className={`h-7 rounded-md bg-white text-right text-sm tabular-nums text-black outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
            error ? "ring-2 ring-[var(--field-invalid)]" : ""
          }`}
          style={{ width, paddingRight: unit ? 32 : 6, paddingLeft: 6 }}
          value={text}
          onFocus={() => {
            focused.current = true;
          }}
          onBlur={() => {
            focused.current = false;
            if (error === null) {
              setText(shown(value));
            }
          }}
          onChange={(event) => {
            setText(event.target.value);
            const result = readField(event.target.value, rule);
            if ("error" in result) {
              setError(result.error);
            } else {
              setError(null);
              onValue(result.value, () => setError(ENTER_NUMBER));
            }
          }}
        />
        {unit ? <span className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[var(--panel-muted-fg)]">{unit}</span> : null}
        {error ? (
          <span role="status" className={CHIP_CLASS} style={CHIP_STYLE}>
            {error}
          </span>
        ) : null}
      </span>
    </label>
  );
}

/** A radiogroup of text segments (UX notes §2.1: one Tab stop, arrows move). */
function Segments<T extends string>({ name, options, value, onChange }: { name: string; options: { value: T; label: string }[]; value: T; onChange: (value: T) => void }) {
  return (
    <div role="radiogroup" aria-label={name} className="flex items-center gap-1">
      {options.map((option, index) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          tabIndex={option.value === value ? 0 : -1}
          className={`${BUTTON_CLASS} ${option.value === value ? SELECTED_CLASS : ""}`}
          onClick={() => onChange(option.value)}
          onKeyDown={(event) => {
            if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
              event.preventDefault();
              const step = event.key === "ArrowRight" ? 1 : options.length - 1;
              onChange(options[(index + step) % options.length].value);
            }
          }}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** The glyphs of the mirror and offset toggles: two rows (or columns) of squares, the second
 * flipped or shifted half a step. */
function LineGlyph({ columns, offset }: { columns: boolean; offset: boolean }) {
  const cells = [0, 1, 2].flatMap((i) => [
    { x: 2 + i * 4.5, y: 3, solid: true },
    { x: 2 + i * 4.5 + (offset ? 2.25 : 0), y: 9.5, solid: offset },
  ]);
  return (
    <svg viewBox="0 0 16 16" width={16} height={16} aria-hidden="true" style={columns ? { transform: "rotate(90deg)" } : undefined}>
      {cells.map((cell, i) => (
        <rect key={i} x={cell.x} y={cell.y} width={3.5} height={3.5} fill={cell.solid ? "currentColor" : "none"} stroke="currentColor" strokeWidth={1} />
      ))}
    </svg>
  );
}

function Toggle({ name, pressed, onPress, children }: { name: string; pressed: boolean; onPress: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      aria-label={name}
      title={name}
      aria-pressed={pressed}
      className={`${BUTTON_CLASS} flex w-7 items-center justify-center px-0 ${pressed ? SELECTED_CLASS : ""}`}
      onClick={onPress}
    >
      {children}
    </button>
  );
}

function GridFields({ recipe, tiling }: { recipe: TilingRecipe; tiling: TilingApi }) {
  const set = tiling.update;
  const type = tiling.type;
  return (
    <>
      <div className={GROUP_CLASS}>
        <TilingField label="Columns" name="Columns" value={recipe.columns} rule="count" width={44} onValue={(v, refused) => type({ columns: v }, refused)} inputRef={tiling.firstFieldRef} />
        <TilingField label="Rows" name="Rows" value={recipe.rows} rule="count" width={44} onValue={(v, refused) => type({ rows: v }, refused)} />
      </div>
      <Divider />
      <div className={GROUP_CLASS}>
        <Segments name="Spacing" value={recipe.pitch ? "pitch" : "gap"} options={[{ value: "gap", label: "Gap" }, { value: "pitch", label: "Pitch" }]} onChange={(v) => tiling.setPitch(v === "pitch")} />
        <TilingField label="X" name="X, millimetres" value={recipe.spacing_x} unit="mm" rule={recipe.pitch ? "positive" : "length"} width={72} onValue={(v, refused) => type({ spacing_x: v }, refused)} />
        <TilingField label="Y" name="Y, millimetres" value={recipe.spacing_y} unit="mm" rule={recipe.pitch ? "positive" : "length"} width={72} onValue={(v, refused) => type({ spacing_y: v }, refused)} />
      </div>
      <Divider />
      <div className={GROUP_CLASS}>
        <Toggle name="Mirror every second row" pressed={recipe.mirror_rows} onPress={() => set({ mirror_rows: !recipe.mirror_rows })}>
          <LineGlyph columns={false} offset={false} />
        </Toggle>
        <Toggle name="Mirror every second column" pressed={recipe.mirror_columns} onPress={() => set({ mirror_columns: !recipe.mirror_columns })}>
          <LineGlyph columns offset={false} />
        </Toggle>
      </div>
      <Divider />
      <div className={GROUP_CLASS}>
        <Toggle name="Offset every second row" pressed={recipe.offset === "rows"} onPress={() => set({ offset: recipe.offset === "rows" ? "none" : "rows" })}>
          <LineGlyph columns={false} offset />
        </Toggle>
        <Toggle name="Offset every second column" pressed={recipe.offset === "columns"} onPress={() => set({ offset: recipe.offset === "columns" ? "none" : "columns" })}>
          <LineGlyph columns offset />
        </Toggle>
        <TilingField label="Shift" name="Shift, percent" value={recipe.shift} unit="%" rule="shift" width={72} onValue={(v, refused) => type({ shift: v }, refused)} />
      </div>
    </>
  );
}

function CircleFields({ recipe, tiling }: { recipe: TilingRecipe; tiling: TilingApi }) {
  const set = tiling.update;
  const type = tiling.type;
  return (
    <div className={GROUP_CLASS}>
      <TilingField label="Count" name="Count" value={recipe.count} rule="count" width={44} onValue={(v, refused) => type({ count: v }, refused)} inputRef={tiling.firstFieldRef} />
      <TilingField label="Radius" name="Radius, millimetres" value={recipe.radius} unit="mm" rule="positive" width={72} onValue={(v, refused) => type({ radius: v }, refused)} />
      <TilingField label="Start angle" name="Start angle, degrees" value={recipe.start} unit={"°"} rule="angle" width={72} onValue={(v, refused) => type({ start: v }, refused)} />
      <TilingField label="Sweep" name="Sweep, degrees" value={recipe.sweep} unit={"°"} rule="sweep" width={72} onValue={(v, refused) => type({ sweep: v }, refused)} />
      <TilingField label="Centre X" name="Centre X, millimetres" value={recipe.centre_x} unit="mm" rule="length" width={72} onValue={(v, refused) => type({ centre_x: v }, refused)} />
      <TilingField label="Centre Y" name="Centre Y, millimetres" value={recipe.centre_y} unit="mm" rule="length" width={72} onValue={(v, refused) => type({ centre_y: v }, refused)} />
      <ToolbarSwitch label="Face centre" tooltip="Each instance turns to face the centre." checked={recipe.face_centre} onCheckedChange={(on) => set({ face_centre: on })} />
    </div>
  );
}

/** The Variance block (UX notes §2.4): random ±, per column and per row (circle: per step). */
function VarianceBlock({ recipe, tiling }: { recipe: TilingRecipe; tiling: TilingApi }) {
  const circle = recipe.layout === "circle";
  const headers = circle ? ["Random ±", "Per step"] : ["Random ±", "Per column", "Per row"];
  // The stored triple is [random, per row, per column]; the table shows per column before per row.
  const slots = circle ? [0, 2] : [0, 2, 1];
  return (
    <div className="max-h-[min(60vh,360px)] overflow-y-auto border-t py-1" style={{ borderColor: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}>
      <div className="grid items-center gap-x-2" style={{ gridTemplateColumns: `96px repeat(${headers.length}, 64px)` }}>
        <span />
        {headers.map((header) => (
          <span key={header} className="text-xs text-[var(--panel-muted-fg)]">
            {header}
          </span>
        ))}
        {VARIANCE_ROWS.map((row) => (
          <div key={row.key} className="contents">
            <span className={`${LABEL_CLASS} h-9 leading-9`}>{row.label}</span>
            {slots.map((slot, column) => (
              <TilingField
                key={slot}
                hideLabel
                label=""
                name={`${row.name} ${headers[column]}, ${row.unit}`}
                value={recipe[row.key][slot]}
                rule={slot === 0 ? "range" : "step"}
                width={64}
                onValue={(v, refused) => {
                  const next = [...recipe[row.key]] as Triple;
                  next[slot] = v;
                  tiling.type({ [row.key]: next }, refused);
                }}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * The tiling bar (`specs/0069-tiling` UX notes §2): it replaces the contents of the Select bar
 * while Tile or Edit tiling is open. Title, Cancel and Apply first, then the layout and its
 * fields, Variance, Seed with the dice, Result, Keep source in place and the count status.
 */
export function TilingBar({ tiling }: { tiling: TilingApi }) {
  const [varianceOpen, setVarianceOpen] = useState(false);
  const [announced, setAnnounced] = useState("");
  const { recipe, status } = tiling;
  const count = status?.count;
  // The polite live region: on entering and after a count change, once it is quiet (UX notes §6).
  useEffect(() => {
    if (count === undefined) {
      return undefined;
    }
    const timer = window.setTimeout(() => setAnnounced(tilingAnnouncement(count)), ANNOUNCE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [count]);
  if (!recipe || !status) {
    return null;
  }
  const type = tiling.type;
  const set = tiling.update;
  const refused = status.level === "refused";
  const summary = varianceSummary(recipe);
  return (
    <div data-context-bar="" className="pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm" style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}>
      <div className={BAR_ROW_CLASS}>
        <div className={GROUP_CLASS}>
          <span className="text-sm font-semibold text-[var(--toolbar-icon)]">{status.reopened ? EDIT_TILING : TILE}</span>
          <button type="button" className={`${BUTTON_CLASS} w-16`} onClick={tiling.cancel}>
            Cancel
          </button>
          <span className="relative">
            <button type="button" aria-disabled={refused} className={`${BUTTON_CLASS} ${SELECTED_CLASS} w-16`} onClick={() => !refused && tiling.apply()}>
              Apply
            </button>
            {status.refusal ? (
              <span role="alert" className={`${CHIP_CLASS} ring-1 ring-[var(--field-invalid)] text-[var(--field-invalid)]`} style={{ ...CHIP_STYLE, color: "var(--field-invalid)" }}>
                {status.refusal}
              </span>
            ) : null}
          </span>
        </div>
        <Divider />
        <div className={GROUP_CLASS}>
          <Segments name="Layout" value={recipe.layout} options={[{ value: "grid", label: "Grid" }, { value: "circle", label: "Circle" }]} onChange={(layout) => set({ layout })} />
        </div>
        <Divider />
        {recipe.layout === "grid" ? <GridFields recipe={recipe} tiling={tiling} /> : <CircleFields recipe={recipe} tiling={tiling} />}
        <Divider />
        <div className={GROUP_CLASS}>
          <button type="button" aria-expanded={varianceOpen} title={summary} className={`${BUTTON_CLASS} relative flex items-center gap-1 ${varianceOpen ? "bg-[var(--editor-accent-hover)]" : ""}`} onClick={() => setVarianceOpen((o) => !o)}>
            <SlidersHorizontal size={16} aria-hidden="true" />
            Variance
            {summary !== "Variance: none" ? <span aria-hidden="true" className="absolute left-5 top-1 h-1.5 w-1.5 rounded-full bg-[var(--toolbar-icon)]" /> : null}
          </button>
        </div>
        <Divider />
        <div className={GROUP_CLASS}>
          <TilingField label="Seed" name="Seed" value={recipe.seed} rule="seed" width={112} onValue={(v, refused) => type({ seed: v }, refused)} />
          <button type="button" aria-label={NEW_SEED} title={NEW_SEED} className={`${BUTTON_CLASS} flex w-7 items-center justify-center px-0`} onClick={tiling.dice}>
            <Dices size={16} aria-hidden="true" />
          </button>
        </div>
        <Divider />
        <div className={GROUP_CLASS}>
          <span className={LABEL_CLASS}>Result</span>
          <Segments name="Result" value={recipe.copies ? "copies" : "clones"} options={[{ value: "clones", label: "Clones" }, { value: "copies", label: "Copies" }]} onChange={(v) => set({ copies: v === "copies" })} />
        </div>
        <Divider />
        <ToolbarSwitch label={KEEP_SOURCE} shortLabel="Keep source" tooltip={KEEP_SOURCE_RULE} checked={recipe.keep_source} onCheckedChange={(on) => set({ keep_source: on })} />
        <div className={`${GROUP_CLASS} relative`}>
          <span className="text-xs tabular-nums text-[var(--toolbar-icon)]">{countText(status.count)}</span>
          {status.level === "simplified" ? (
            <span role="status" className="flex items-center gap-1 text-xs text-[var(--toolbar-icon)]">
              <CircleAlert size={12} aria-hidden="true" />
              {WARN_TEXT}
            </span>
          ) : null}
          {status.images_copies ? (
            <span role="status" className="text-xs text-[var(--toolbar-icon)]">
              {IMAGES_TEXT}
            </span>
          ) : null}
          {refused ? (
            <span role="alert" className={`${CHIP_CLASS} ring-1 ring-[var(--field-invalid)]`} style={{ ...CHIP_STYLE, color: "var(--field-invalid)" }}>
              {REFUSED_TEXT}
            </span>
          ) : null}
        </div>
      </div>
      {varianceOpen ? <VarianceBlock recipe={recipe} tiling={tiling} /> : null}
      <span aria-live="polite" className="sr-only">
        {announced}
      </span>
    </div>
  );
}

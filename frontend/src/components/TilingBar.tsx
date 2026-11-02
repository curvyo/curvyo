import { CircleAlert, SlidersHorizontal } from "lucide-react";
import { Tooltip } from "radix-ui";
import { createContext, useContext, useEffect, useId, useRef, useState } from "react";

import { Divider, useRowStarts } from "@/components/SelectToolbar";
import { ToolbarSwitch } from "@/components/ToolbarSwitch";
import type { TilingApi } from "@/hooks/useTiling";
import { BAR_ROW_CLASS } from "@/lib/barLayout";
import {
  ANNOUNCE_DELAY_MS,
  CIRCLE,
  EDIT_TILING,
  ENTER_NUMBER,
  GRID,
  KEEP_SOURCE,
  KEEP_SOURCE_RULE,
  REFUSED_TEXT,
  TILE,
  VARIANCE_BLOCKS,
  VARIANCE_ROWS,
  type FieldRule,
  type TilingRecipe,
  type Triple,
  noticeLines,
  readField,
  tilingAnnouncement,
  varianceSummary,
} from "@/lib/tilingText";

const BUTTON_CLASS =
  "h-7 whitespace-nowrap rounded-md px-2 text-sm text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--toolbar-bg)]";
const SELECTED_CLASS = "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:bg-[var(--toolbar-icon-active-bg)]";
/** A group of the bar: its controls on one 28 px centre line, 4 px above and below (UX notes
 * §12.2), `relative` for the divider before it. */
const GROUP_CLASS = "group/row relative flex flex-wrap items-center gap-x-2 gap-y-1 py-1";
const TOOLTIP_STYLE = { background: "var(--toolbar-bg)", color: "var(--toolbar-icon)", boxShadow: "var(--panel-elevation-shadow)" };
const LABEL_CLASS = "text-sm text-[var(--toolbar-icon)]";
const CHIP_CLASS = "absolute left-0 top-full z-30 mt-1 w-max max-w-[320px] rounded-lg px-2 py-1 text-xs";
const CHIP_STYLE = { background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)", color: "var(--toolbar-icon)" };

/** Lets a field tell the bar that it holds an invalid text (Apply is then disabled). */
const InvalidFields = createContext<(id: string, invalid: boolean) => void>(() => undefined);

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
  /** The Variance block's 24 px field (36 px for a coarse pointer). */
  compact?: boolean;
}

/** A bar number field (UX notes §2.1): every valid keystroke reaches the preview (debounced), an
 * invalid text, or one the session refuses, keeps the last valid preview and marks the field. */
function TilingField({ label, name, value, unit, rule, width, onValue, inputRef, hideLabel, compact }: FieldProps) {
  const [text, setText] = useState(shown(value));
  const [error, setError] = useState<string | null>(null);
  const focused = useRef(false);
  const id = useId();
  const report = useContext(InvalidFields);
  useEffect(() => {
    report(id, error !== null);
    return () => report(id, false);
  }, [id, error, report]);
  useEffect(() => {
    if (!focused.current) {
      setText(shown(value));
      setError(null);
    }
  }, [value]);
  return (
    <label className="relative flex items-center gap-2">
      {hideLabel ? null : <span className={LABEL_CLASS}>{label}</span>}
      <span className="relative">
        <input
          ref={inputRef}
          aria-label={name}
          aria-invalid={error !== null}
          inputMode="decimal"
          className={`${compact ? "h-6 pointer-coarse:h-9" : "h-7"} rounded-md bg-white text-right text-sm tabular-nums text-black outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
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
          onKeyDown={(event) => arrowStep(event, options, index, onChange)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

/** Left and right arrows choose the previous or next option, wrapping. */
function arrowStep<T>(event: React.KeyboardEvent, options: { value: T }[], index: number, onChange: (value: T) => void) {
  if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
    event.preventDefault();
    const step = event.key === "ArrowRight" ? 1 : options.length - 1;
    onChange(options[(index + step) % options.length].value);
  }
}

/** The Grid glyph: 3 x 3 squares of 3 px (UX notes §12.2). */
function GridGlyph() {
  return (
    <svg viewBox="0 0 16 16" width={16} height={16} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth={1.5}>
      {[1.5, 6.5, 11.5].flatMap((x) => [1.5, 6.5, 11.5].map((y) => <rect key={`${x}-${y}`} x={x} y={y} width={3} height={3} />))}
    </svg>
  );
}

/** The Circle glyph: six squares of 3 px on a ring (UX notes §12.2). */
function CircleGlyph() {
  return (
    <svg viewBox="0 0 16 16" width={16} height={16} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth={1.5}>
      {[0, 1, 2, 3, 4, 5].map((i) => {
        const angle = (i * Math.PI) / 3;
        return <rect key={i} x={8 + 5.5 * Math.cos(angle) - 1.5} y={8 - 5.5 * Math.sin(angle) - 1.5} width={3} height={3} />;
      })}
    </svg>
  );
}

const LAYOUTS = [
  { value: "grid" as const, label: GRID, glyph: <GridGlyph /> },
  { value: "circle" as const, label: CIRCLE, glyph: <CircleGlyph /> },
];

/** The Layout radiogroup (UX notes §12.2): two 28 px icon segments in the Offset chip's pill
 * look, the name in the tooltip and the accessible name; one Tab stop, arrows choose. */
function LayoutSegments({ value, onChange }: { value: "grid" | "circle"; onChange: (value: "grid" | "circle") => void }) {
  return (
    <div
      role="radiogroup"
      aria-label="Layout"
      className="flex h-7 rounded-[7px] p-[1.5px]"
      // A 1.5 px ring as a shadow: a CSS border of 1.5 px computes to 1 px at a device pixel ratio of 1.
      style={{ boxShadow: "inset 0 0 0 1.5px var(--toolbar-icon)" }}
    >
      {LAYOUTS.map((option, index) => {
        const selected = option.value === value;
        return (
          <Tooltip.Root key={option.value}>
            <Tooltip.Trigger asChild>
              <button
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={option.label}
                tabIndex={selected ? 0 : -1}
                className={`flex h-[25px] w-7 items-center justify-center rounded-[5px] text-[var(--toolbar-icon)] outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${
                  selected ? SELECTED_CLASS : "hover:bg-[var(--editor-accent-hover)]"
                }`}
                onClick={() => onChange(option.value)}
                onKeyDown={(event) => arrowStep(event, LAYOUTS, index, onChange)}
              >
                {option.glyph}
              </button>
            </Tooltip.Trigger>
            <Tooltip.Portal>
              <Tooltip.Content side="bottom" sideOffset={4} className="z-50 rounded-md px-2 py-1 text-xs" style={TOOLTIP_STYLE}>
                {option.label}
              </Tooltip.Content>
            </Tooltip.Portal>
          </Tooltip.Root>
        );
      })}
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
        <Divider />
        <TilingField label="Columns" name="Columns" value={recipe.columns} rule="count" width={44} onValue={(v, refused) => type({ columns: v }, refused)} inputRef={tiling.firstFieldRef} />
        <TilingField label="Rows" name="Rows" value={recipe.rows} rule="count" width={44} onValue={(v, refused) => type({ rows: v }, refused)} />
      </div>
      <div className={GROUP_CLASS}>
        <Divider />
        <Segments name="Spacing" value={recipe.pitch ? "pitch" : "gap"} options={[{ value: "gap", label: "Gap" }, { value: "pitch", label: "Pitch" }]} onChange={(v) => tiling.setPitch(v === "pitch")} />
        <TilingField label="X" name="X, millimetres" value={recipe.spacing_x} unit="mm" rule={recipe.pitch ? "positive" : "length"} width={72} onValue={(v, refused) => type({ spacing_x: v }, refused)} />
        <TilingField label="Y" name="Y, millimetres" value={recipe.spacing_y} unit="mm" rule={recipe.pitch ? "positive" : "length"} width={72} onValue={(v, refused) => type({ spacing_y: v }, refused)} />
      </div>
      <div className={GROUP_CLASS}>
        <Divider />
        <Toggle name="Mirror every second row" pressed={recipe.mirror_rows} onPress={() => set({ mirror_rows: !recipe.mirror_rows })}>
          <LineGlyph columns={false} offset={false} />
        </Toggle>
        <Toggle name="Mirror every second column" pressed={recipe.mirror_columns} onPress={() => set({ mirror_columns: !recipe.mirror_columns })}>
          <LineGlyph columns offset={false} />
        </Toggle>
      </div>
      <div className={GROUP_CLASS}>
        <Divider />
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

/** The circle's bar fields (UX notes §12.2): Count and Face centre. Centre, radius, start and
 * sweep are the canvas handles. */
function CircleFields({ recipe, tiling }: { recipe: TilingRecipe; tiling: TilingApi }) {
  return (
    <div className={GROUP_CLASS}>
      <Divider />
      <TilingField label="Count" name="Count" value={recipe.count} rule="count" width={44} onValue={(v, refused) => tiling.type({ count: v }, refused)} inputRef={tiling.firstFieldRef} />
      <ToolbarSwitch label="Face centre" tooltip="Each instance turns to face the centre." checked={recipe.face_centre} onCheckedChange={(on) => tiling.update({ face_centre: on })} />
    </div>
  );
}

/** The compact Variance block (UX notes §12.5): three blocks of two channels side by side, each
 * with its own header row, wrapping to two per line on a narrower bar; under 500 px of bar (a
 * container query) or for a coarse pointer one column of six rows under one header. Circle:
 * Random ± and Per step. */
function VarianceBlock({ recipe, tiling }: { recipe: TilingRecipe; tiling: TilingApi }) {
  const circle = recipe.layout === "circle";
  const headers = circle ? ["Random ±", "Per step"] : ["Random ±", "Per column", "Per row"];
  // The stored triple is [random, per row, per column]; the table shows per column before per row.
  const slots = circle ? [0, 2] : [0, 2, 1];
  // "Per column" needs 64 px at 12 px (UX notes §12.5).
  const columns = circle ? "56px 56px" : "56px 64px 56px";
  return (
    <div className="@container max-h-[min(60vh,360px)] overflow-y-auto border-t" style={{ borderColor: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}>
      <div className="flex flex-col gap-1 px-2 py-2 pointer-coarse:flex-col @min-[500px]:flex-row @min-[500px]:flex-wrap @min-[500px]:gap-x-3 @min-[500px]:gap-y-2">
        {VARIANCE_BLOCKS.map((block, blockIndex) => (
          <div
            key={blockIndex}
            className="grid items-center gap-x-1 gap-y-1 [grid-template-columns:72px_var(--columns)] @min-[500px]:[grid-template-columns:var(--label)_var(--columns)] pointer-coarse:[grid-template-columns:72px_var(--columns)]"
            style={{ "--columns": columns, "--label": `${block.labelWidth}px` } as React.CSSProperties}
          >
            <span className={`-mb-1 h-5 ${blockIndex > 0 ? "hidden @min-[500px]:block pointer-coarse:hidden" : ""}`} />
            {headers.map((header) => (
              <span key={header} className={`-mb-1 h-5 whitespace-nowrap text-right text-xs leading-5 text-[var(--panel-muted-fg)] ${blockIndex > 0 ? "hidden @min-[500px]:block pointer-coarse:hidden" : ""}`}>
                {header}
              </span>
            ))}
            {block.keys.map((key) => {
              const row = VARIANCE_ROWS.find((r) => r.key === key);
              if (!row) {
                return null;
              }
              return (
                <div key={row.key} className="contents">
                  <span className="h-6 whitespace-nowrap text-xs leading-6 text-[var(--toolbar-icon)] pointer-coarse:h-9 pointer-coarse:leading-9">{row.label}</span>
                  {slots.map((slot, column) => (
                    <TilingField
                      key={slot}
                      hideLabel
                      compact
                      label=""
                      name={`${row.name} ${headers[column]}, ${row.unit}`}
                      value={recipe[row.key][slot]}
                      rule={slot === 0 ? "range" : "step"}
                      width={column === 1 && !circle ? 64 : 56}
                      onValue={(v, refused) => {
                        const next = [...recipe[row.key]] as Triple;
                        next[slot] = v;
                        tiling.type({ [row.key]: next }, refused);
                      }}
                    />
                  ))}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}

/** The notice slot (UX notes §12.3): a quiet icon while a limit or the image rule is active, its
 * lines in the tooltip and the accessible name; the refusal also stays readable under it. */
function NoticeSlot({ lines, refused }: { lines: string[]; refused: boolean }) {
  if (lines.length === 0) {
    return null;
  }
  const text = lines.join("\n");
  return (
    <div className={GROUP_CLASS}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>
          <span
            role="status"
            aria-label={lines.join(" ")}
            tabIndex={0}
            className="relative flex size-7 items-center justify-center rounded-md outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
            style={{ color: refused ? "var(--field-invalid)" : "var(--toolbar-icon)" }}
          >
            <CircleAlert size={16} aria-hidden="true" />
            {refused ? (
              <span role="alert" className={`${CHIP_CLASS} ring-1 ring-[var(--field-invalid)]`} style={{ ...CHIP_STYLE, color: "var(--field-invalid)" }}>
                {REFUSED_TEXT}
              </span>
            ) : null}
          </span>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content side="bottom" sideOffset={4} className="z-50 whitespace-pre-line rounded-md px-2 py-1 text-xs" style={TOOLTIP_STYLE}>
            {text}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </div>
  );
}

/**
 * The tiling bar (`specs/0069-tiling` UX notes §2 and §12.2): it replaces the contents of the
 * Select bar while Tile or Edit tiling is open. A left cluster that wraps by whole group (Title,
 * Layout, the layout's fields, Variance, Result, Keep source, the notice slot) and Apply pinned at
 * the right end of line 1. Escape cancels; there is no Cancel button and no seed control.
 */
export function TilingBar({ tiling }: { tiling: TilingApi }) {
  const [varianceOpen, setVarianceOpen] = useState(false);
  const [announced, setAnnounced] = useState("");
  const [invalidIds, setInvalidIds] = useState<ReadonlySet<string>>(new Set());
  const clusterRef = useRef<HTMLDivElement>(null);
  useRowStarts(clusterRef);
  const report = useRef((id: string, invalid: boolean) => {
    setInvalidIds((current) => {
      if (current.has(id) === invalid) {
        return current;
      }
      const next = new Set(current);
      if (invalid) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  }).current;
  const { recipe, status } = tiling;
  const level = status?.level;
  // The polite live region: on entering and when a limit state changes, once it is quiet; it no
  // longer speaks every count change (UX notes §12.3).
  const countRef = useRef(0);
  const count = status?.count ?? 0;
  useEffect(() => {
    countRef.current = count;
  }, [count]);
  useEffect(() => {
    if (level === undefined) {
      return undefined;
    }
    const timer = window.setTimeout(() => setAnnounced(tilingAnnouncement(countRef.current)), ANNOUNCE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [level]);
  if (!recipe || !status) {
    return null;
  }
  const set = tiling.update;
  const refused = status.level === "refused";
  const applyBlocked = refused || invalidIds.size > 0;
  const summary = varianceSummary(recipe);
  return (
    <InvalidFields.Provider value={report}>
    <Tooltip.Provider delayDuration={400}>
    <div data-context-bar="" className="pointer-events-auto flex min-w-0 flex-col rounded-lg px-2 text-sm" style={{ background: "var(--toolbar-bg)", boxShadow: "var(--panel-elevation-shadow)" }}>
      <div className="flex min-w-0 items-start gap-x-[25px]">
        <div ref={clusterRef} className={`${BAR_ROW_CLASS} flex-1`}>
          <div className={GROUP_CLASS}>
            <span className="h-7 text-sm font-semibold leading-7 text-[var(--toolbar-icon)]">{status.reopened ? EDIT_TILING : TILE}</span>
          </div>
          <div className={GROUP_CLASS}>
            <Divider />
            <LayoutSegments value={recipe.layout} onChange={(layout) => set({ layout })} />
          </div>
          {recipe.layout === "grid" ? <GridFields recipe={recipe} tiling={tiling} /> : <CircleFields recipe={recipe} tiling={tiling} />}
          <div className={GROUP_CLASS}>
            <Divider />
            <button type="button" aria-expanded={varianceOpen} title={summary} className={`${BUTTON_CLASS} relative flex items-center gap-1 ${varianceOpen ? "bg-[var(--editor-accent-hover)]" : ""}`} onClick={() => setVarianceOpen((o) => !o)}>
              <SlidersHorizontal size={16} aria-hidden="true" />
              Variance
              {summary !== "Variance: none" ? <span aria-hidden="true" className="absolute left-5 top-1 h-1.5 w-1.5 rounded-full bg-[var(--toolbar-icon)]" /> : null}
            </button>
          </div>
          <div className={GROUP_CLASS}>
            <Divider />
            <span className={LABEL_CLASS}>Result</span>
            <Segments name="Result" value={recipe.copies ? "copies" : "clones"} options={[{ value: "clones", label: "Clones" }, { value: "copies", label: "Copies" }]} onChange={(v) => set({ copies: v === "copies" })} />
          </div>
          <div className={GROUP_CLASS}>
            <Divider />
            <div className="inline-flex h-7 items-center gap-2">
              <ToolbarSwitch label={KEEP_SOURCE} shortLabel="Keep source" tooltip={KEEP_SOURCE_RULE} checked={recipe.keep_source} onCheckedChange={(on) => set({ keep_source: on })} />
            </div>
          </div>
          <NoticeSlot lines={noticeLines(status)} refused={refused} />
        </div>
        <div className="relative shrink-0 self-start py-1">
          <button type="button" aria-disabled={applyBlocked} className={`${BUTTON_CLASS} ${SELECTED_CLASS} w-16 ${applyBlocked ? "opacity-40" : ""}`} onClick={() => !applyBlocked && tiling.apply()}>
            Apply
          </button>
          {status.refusal ? (
            <span role="alert" className={`${CHIP_CLASS} left-auto right-0 ring-1 ring-[var(--field-invalid)]`} style={{ ...CHIP_STYLE, color: "var(--field-invalid)" }}>
              {status.refusal}
            </span>
          ) : null}
        </div>
      </div>
      {varianceOpen ? <VarianceBlock recipe={recipe} tiling={tiling} /> : null}
      <span aria-live="polite" className="sr-only">
        {announced}
      </span>
    </div>
    </Tooltip.Provider>
    </InvalidFields.Provider>
  );
}

import { CircleAlert } from "lucide-react";
import { useEffect, useRef, useState, type KeyboardEvent } from "react";

import { SegmentGroup } from "@/components/panel/SegmentGroup";
import type { MaterialsApi } from "@/hooks/useMaterials";
import { SET_WORDS } from "@/lib/operationSettingsRows";
import { dropSaveDraft, keepDraft, keptDraft } from "@/lib/saveDraft";
import { passesOf, replacesSentence, savedStatus, thicknessOf, validateSave, type SaveDraft, type SaveRefusal } from "@/lib/testGridResult";
import type { TestGridView } from "@/lib/testGridView";

/** The guard of the Replace button against a double press (`docs/design-system.md`, "Inline confirm"). */
const GUARD_MS = 400;

const INPUT =
  "h-7 w-full rounded-[5px] border bg-white px-1.5 text-sm tabular-nums text-[var(--toolbar-icon)] outline-none focus:border-[var(--editor-accent)] focus:shadow-[inset_0_0_0_1px_var(--editor-accent)]";
const BUTTON =
  "h-7 w-[112px] rounded border text-sm focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

function Chip({ id, text }: { id: string; text: string }) {
  return (
    <div id={id} role="alert" className="mt-1 flex items-start gap-1 text-xs text-[var(--field-invalid)]">
      <CircleAlert size={12} aria-hidden className="mt-0.5 shrink-0" />
      {text}
    </div>
  );
}

interface SaveAsMaterialFormProps {
  view: TestGridView;
  materials: MaterialsApi;
  /** The form closes: `saved` is the polite status after a save, `null` after Cancel. */
  onClose: (saved: string | null) => void;
}

/**
 * The Save as material form (`specs/0028-material-test-library` `ux-notes.md` 9, criteria 15, 16):
 * Add to (only with a material), Name, Thickness and Passes for a new material, or the inline list
 * of this machine's materials and Passes, with the Replace sentence and its 400 ms guard.
 */
export function SaveAsMaterialForm({ view, materials, onClose }: SaveAsMaterialFormProps) {
  const key = `${view.group.peer}:${view.group.counter}`;
  const loaded = (materials.view?.rows ?? []).flatMap((row) => (row.state === "loaded" ? [row] : []));
  const [draft, setDraft] = useState<SaveDraft>(
    keptDraft(key) ?? { existing: "", name: materials.view?.nextName ?? "", thickness: "", passes: "1" },
  );
  const [refusal, setRefusal] = useState<SaveRefusal | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [guarded, setGuarded] = useState(false);
  const nameRef = useRef<HTMLInputElement>(null);
  const thicknessRef = useRef<HTMLInputElement>(null);
  const passesRef = useRef<HTMLInputElement>(null);
  const existing = draft.existing === "" ? null : (loaded.find((row) => row.id === draft.existing) ?? null);
  const selected = materials.view?.selected;
  const old = existing && selected?.id === existing.id ? selected.sets.find((set) => set.kind === view.kind) : undefined;
  const sentence = old ? replacesSentence(SET_WORDS[view.kind], old.numbers.powerPct, old.numbers.speedMmMin, old.source) : null;

  useEffect(() => {
    keepDraft(key, draft);
  }, [key, draft]);
  useEffect(() => {
    nameRef.current?.focus();
    nameRef.current?.select();
  }, []);
  // A material in the way of a save is only a guarded Replace after the sentence has been read.
  useEffect(() => {
    if (sentence === null) return undefined;
    setGuarded(true);
    const timer = window.setTimeout(() => setGuarded(false), GUARD_MS);
    return () => window.clearTimeout(timer);
  }, [sentence]);

  const change = (patch: Partial<SaveDraft>) => {
    setDraft((previous) => ({ ...previous, ...patch }));
    setRefusal(null);
    setFailure(null);
  };
  const close = (saved: string | null) => {
    dropSaveDraft();
    onClose(saved);
  };
  const save = () => {
    const wrong = validateSave(draft);
    if (wrong) {
      setRefusal(wrong);
      ({ name: nameRef, thickness: thicknessRef, passes: passesRef })[wrong.field].current?.focus();
      return;
    }
    const chip = materials.save(draft.existing, draft.name.trim(), thicknessOf(draft), passesOf(draft));
    if (chip !== "") {
      setFailure(chip);
      return;
    }
    close(savedStatus(existing?.name ?? draft.name.trim()));
  };
  const onKey = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close(null);
    } else if (event.key === "Enter" && (event.target as HTMLElement).tagName === "INPUT" && sentence === null) {
      event.preventDefault();
      save();
    }
  };
  const chipFor = (field: SaveRefusal["field"]) => (refusal?.field === field ? <Chip id={`save-${field}`} text={refusal.text} /> : null);
  const invalid = (field: SaveRefusal["field"]) => refusal?.field === field;
  const border = (field: SaveRefusal["field"]) =>
    invalid(field) ? "border-[var(--field-invalid)] shadow-[inset_0_0_0_2px_var(--field-invalid)]" : "border-[color-mix(in_srgb,var(--toolbar-icon)_60%,transparent)]";
  const passes = (
    <div className="relative w-[112px]">
      <input
        ref={passesRef}
        aria-label="Passes"
        aria-invalid={invalid("passes") || undefined}
        aria-describedby={invalid("passes") ? "save-passes" : undefined}
        inputMode="decimal"
        autoComplete="off"
        value={draft.passes}
        onChange={(event) => change({ passes: event.target.value })}
        className={`${INPUT} pr-6 text-right ${border("passes")}`}
      />
      <span aria-hidden className="pointer-events-none absolute top-3.5 right-1.5 -translate-y-1/2 text-xs text-[var(--panel-muted-fg)]">×</span>
      {chipFor("passes")}
    </div>
  );

  return (
    <div role="group" aria-label="Save as material" onKeyDown={onKey} className="flex flex-col gap-1 rounded border border-[var(--lane-line)] p-2">
      {loaded.length > 0 ? (
        <SegmentGroup
          name="Add to"
          options={[["new", "New material"], ["existing", "Existing"]] as const}
          value={draft.existing === "" ? "new" : "existing"}
          width={112}
          onChange={(value) => change({ existing: value === "new" ? "" : (loaded[0]?.id ?? "") })}
          onReturnFocus={() => {}}
        />
      ) : null}
      {draft.existing === "" ? (
        <>
          <div>
            <input
              ref={nameRef}
              aria-label="Name"
              aria-invalid={invalid("name") || undefined}
              aria-describedby={invalid("name") ? "save-name" : undefined}
              maxLength={60}
              autoComplete="off"
              value={draft.name}
              onChange={(event) => change({ name: event.target.value })}
              className={`${INPUT} ${border("name")}`}
            />
            {chipFor("name")}
          </div>
          <div className="flex gap-1">
            <div className="w-[112px]">
              <div className="relative">
                <input
                  ref={thicknessRef}
                  aria-label="Thickness, millimetres"
                  aria-invalid={invalid("thickness") || undefined}
                  aria-describedby={invalid("thickness") ? "save-thickness" : undefined}
                  inputMode="decimal"
                  autoComplete="off"
                  value={draft.thickness}
                  onChange={(event) => change({ thickness: event.target.value })}
                  className={`${INPUT} pr-8 text-right ${border("thickness")}`}
                />
                <span aria-hidden className="pointer-events-none absolute top-1/2 right-1.5 -translate-y-1/2 text-sm text-[var(--panel-muted-fg)]">mm</span>
              </div>
              {chipFor("thickness")}
            </div>
            {passes}
          </div>
        </>
      ) : (
        <>
          <div role="radiogroup" aria-label="Material" className="flex max-h-[140px] flex-col overflow-y-auto">
            {loaded.map((row) => (
              <button
                key={row.id}
                type="button"
                role="radio"
                aria-checked={row.id === draft.existing}
                onClick={() => {
                  materials.select(row.id);
                  change({ existing: row.id });
                }}
                className="flex h-7 items-center rounded px-1 text-left text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] aria-checked:bg-[var(--value-fill)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                <span className="min-w-0 flex-1 truncate">{row.name}</span>
              </button>
            ))}
          </div>
          {passes}
          {sentence ? <p className="text-xs text-[var(--toolbar-icon)]">{sentence}</p> : null}
        </>
      )}
      <div className="flex gap-1">
        <button type="button" onClick={() => close(null)} className={`${BUTTON} border-[var(--lane-line)] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)]`}>
          Cancel
        </button>
        <button
          type="button"
          aria-disabled={sentence !== null && guarded ? true : undefined}
          onClick={() => {
            if (!(sentence !== null && guarded)) save();
          }}
          className={
            sentence
              ? `${BUTTON} ml-auto border-[var(--destructive)] bg-[var(--destructive)] text-white aria-disabled:opacity-60`
              : `${BUTTON} ml-auto border-[var(--toolbar-icon-active-bg)] bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)]`
          }
        >
          {sentence ? "Replace" : "Save"}
        </button>
      </div>
      {failure ? <Chip id="save-failure" text={failure} /> : null}
      {materials.notice?.alert ? <Chip id="save-notice" text={materials.notice.text} /> : null}
    </div>
  );
}

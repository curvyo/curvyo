import { ChevronDown, ChevronRight, Download, Sparkles, Trash2, TriangleAlert, X } from "lucide-react";
import { GLYPH, SMALL_GLYPH } from "@/components/ui/looks";
import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { FocusEvent } from "react";

import type { AiSteps } from "@/lib/aiSteps";
import { ARM_GUARD_MS } from "@/lib/deleteArm";
import type { AiBody } from "@/lib/adjustmentsView";
import { Button } from "@/components/ui/button";
import { AlertLine } from "@/components/ui/notice";

const MUTED = "w-full text-xs text-[var(--panel-muted-fg)]";
const ALERT = "w-full text-xs text-[var(--field-invalid)]";

type Phase = "consent" | "progress" | "idle";

/** The body of an AI step row (`ux-notes.md` §8; criteria 47, 48, 63 to 66): Run, the consent
 * box before the first download, the progress bar with Cancel, the model's state, the stored
 * result with Remove stored result, Out of date, Upscale's size line and the model's licence.
 * Every word but the licence lines is the session's. */
export function AiStepBody({ index, body, steps }: { index: number; body: AiBody; steps: AiSteps | null }) {
  const phase: Phase = body.consent !== null ? "consent" : body.progress !== null ? "progress" : "idle";
  const { boxRef, primaryRef, onFocus, onBlur, downloadReady } = useBodyFocus(phase, body, steps);
  return (
    <div ref={boxRef} data-ai-body={body.model} className="flex w-full flex-col gap-1" onFocus={onFocus} onBlur={onBlur}>
      {body.sizeLine !== null ? (
        <p role={body.sizeRefused ? "alert" : undefined} className={body.sizeRefused ? ALERT : MUTED}>
          {body.sizeLine}
        </p>
      ) : null}
      {body.consent !== null ? (
        <div
          className="flex w-full flex-col gap-2 rounded-[5px] border border-[var(--field-border)] p-2"
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            // The consent's Escape only cancels the consent; the canvas's Escape (focus to the
            // canvas, clear the selection) does not also run.
            event.stopPropagation();
            event.preventDefault();
            steps?.dismiss(body.model);
          }}
        >
          <p className="text-xs text-[var(--toolbar-icon)]">{body.consent}</p>
          <div className="flex gap-2">
            <Button icon={X} ref={primaryRef} className="flex-1" onClick={() => steps?.dismiss(body.model)}>
              Cancel
            </Button>
            <Button variant="primary" icon={Download} className="flex-1"
              onClick={() => {
                if (downloadReady()) void steps?.consent(index, body.model);
              }}
            >
              Download
            </Button>
          </div>
        </div>
      ) : body.progress !== null ? (
        <div className="flex w-full flex-col gap-1">
          <div className="h-1 w-full rounded-full bg-[color-mix(in_srgb,var(--toolbar-icon)_20%,transparent)]">
            <div className="h-1 rounded-full bg-[var(--editor-accent)]" style={{ width: `${Math.round(body.progress.fraction * 100)}%` }} />
          </div>
          <div className="flex items-center justify-between gap-2">
            <p className="text-xs tabular-nums text-[var(--panel-muted-fg)]">{body.progress.line}</p>
            <Button icon={X} ref={primaryRef} onClick={() => steps?.cancel(body.model)}>
              Cancel
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex flex-wrap gap-1">
          <Button variant="primary" icon={Sparkles}
            ref={primaryRef}
            aria-disabled={body.sizeRefused}
            onClick={() => {
              if (!body.sizeRefused) void steps?.run(index, body.model);
            }}
          >
            {body.run}
          </Button>
          {body.stored !== null ? (
            <Button icon={Trash2} onClick={() => steps?.removeResult(index)}>
              Remove stored result
            </Button>
          ) : null}
        </div>
      )}
      <AiStateLines body={body} steps={steps} idle={phase === "idle"} />
      <ModelLicence steps={steps} kind={body.model} />
    </div>
  );
}

/** The lines under the Run row: a refusal, the stored result, Out of date, the model's line. */
function AiStateLines({ body, steps, idle }: { body: AiBody; steps: AiSteps | null; idle: boolean }) {
  return (
    <>
      {body.refusal !== null ? (
        <AlertLine>
          {body.refusal}
        </AlertLine>
      ) : null}
      {body.stored !== null ? <p className={MUTED}>{body.stored}</p> : null}
      {body.outOfDate !== null ? (
        <p className="flex items-center gap-1 text-xs text-[var(--toolbar-icon)]">
          <TriangleAlert {...GLYPH} aria-hidden className="shrink-0" />
          {body.outOfDate}
        </p>
      ) : null}
      {/* The consent sentence names the size; the "not downloaded" line would repeat it (draft 04 B). */}
      {body.state !== null && idle ? (
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
          <p role={body.alert ? "alert" : undefined} className={body.alert ? ALERT : "text-xs text-[var(--panel-muted-fg)]"}>
            {body.state}
          </p>
          {body.modelReady ? (
            <Button icon={Trash2} onClick={() => void steps?.removeModel(body.model)}>
              Remove model
            </Button>
          ) : null}
        </div>
      ) : null}
    </>
  );
}

/**
 * The body's focus and consent lifecycle (`ux-notes.md` §8, UI review 2026-11-23). The consent
 * box takes the focus on Cancel; any other phase change that removes the focused button
 * (Escape, Cancel, Download, the end of a Run, Remove model, Remove stored result) puts the focus
 * on the button in its place, never on the page. Leaving the consent (another selection, another
 * tab, folding the row) cancels it as Escape does. Download ignores a press within 400 ms of the
 * consent appearing (the Inline confirm's guard), so a double press on Run never downloads.
 */
function useBodyFocus(phase: Phase, body: AiBody, steps: AiSteps | null) {
  const boxRef = useRef<HTMLDivElement>(null);
  const primaryRef = useRef<HTMLButtonElement>(null);
  const lastFocused = useRef<HTMLElement | null>(null);
  const shownAt = useRef(0);
  const asking = useRef<{ steps: AiSteps | null; model: string } | null>(null);
  useEffect(() => {
    if (phase === "consent") shownAt.current = performance.now();
    asking.current = phase === "consent" ? { steps, model: body.model } : null;
  }, [phase, steps, body.model]);
  useLayoutEffect(() => {
    const gone = lastFocused.current !== null && !lastFocused.current.isConnected;
    const lost = document.activeElement === null || document.activeElement === document.body;
    if (phase === "consent" || (gone && lost)) {
      primaryRef.current?.focus();
      lastFocused.current = primaryRef.current;
    }
  }, [phase, body.stored, body.modelReady]);
  useEffect(
    () => () => {
      const left = asking.current;
      if (left) left.steps?.dismiss(left.model);
    },
    [],
  );
  return {
    boxRef,
    primaryRef,
    onFocus: (event: FocusEvent<HTMLDivElement>) => {
      lastFocused.current = event.target;
    },
    onBlur: (event: FocusEvent<HTMLDivElement>) => {
      const next = event.relatedTarget;
      if (next !== null && !boxRef.current?.contains(next)) lastFocused.current = null;
    },
    downloadReady: () => performance.now() - shownAt.current >= ARM_GUARD_MS,
  };
}

/** The model's licence and attribution (ADR 0076 §9), folded under the step's last line: there
 * is no About dialog, so the step that runs the model names it, downloaded or not (Proposal). */
function ModelLicence({ steps, kind }: { steps: AiSteps | null; kind: string }) {
  const [open, setOpen] = useState(false);
  const id = useId();
  const model = steps?.pinnedModel(kind);
  if (!model) return null;
  return (
    <div data-model-licence={kind} className="w-full">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={open ? id : undefined}
        onClick={() => setOpen(!open)}
        className="flex h-5 items-center gap-1 rounded-[5px] text-xs text-[var(--panel-muted-fg)] outline-none hover:text-[var(--toolbar-icon)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
      >
        {open ? <ChevronDown {...SMALL_GLYPH} aria-hidden /> : <ChevronRight {...SMALL_GLYPH} aria-hidden />}
        Model licence
      </button>
      {open ? (
        <div id={id} className="flex flex-col gap-0.5 pl-4 text-[11px] break-words text-[var(--panel-muted-fg)]">
          <p>
            {model.name}, version {model.version} · {model.licence}
          </p>
          <p>{model.copyright}</p>
          <p>{model.upstream.replace(/^https:\/\//, "")}</p>
          <p>Converted to ONNX by Curvyo, weights unchanged.</p>
          <p>Trained on {model.training_data}</p>
        </div>
      ) : null}
    </div>
  );
}

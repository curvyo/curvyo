import { CopyPlus } from "lucide-react";
import { useRef, useState } from "react";

import { TEXT_BUTTON } from "@/components/workbench/FixtureSetsSection";
import type { FixturesApi } from "@/hooks/useFixtures";
import { applyKeyTarget, applyLabel, applyReplacesText, candidateCountText } from "@/lib/fixtureSets";
import type { FixturesView } from "@/lib/fixturesView";

/**
 * "Apply fixtures from page…" (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 5.3): an inline
 * radio list of the other laser pages with job fixtures, the picked page's fixtures previewed on
 * this page, then Apply (which replaces this page's job fixtures) or Cancel. Nothing opens a popup; Escape, Cancel and a second press fold it
 * and drop the preview. Hidden while no other laser page has a job fixture.
 */
export function ApplyFixturesList({ view, api }: { view: FixturesView; api: FixturesApi }) {
  const [open, setOpen] = useState(false);
  const button = useRef<HTMLButtonElement>(null);
  if (view.applyCandidates.length === 0) return null;
  const ids = view.applyCandidates.map((candidate) => candidate.id);
  const picked = view.applyCandidates.find((candidate) => candidate.id === view.applyPicked) ?? null;
  const fold = () => {
    setOpen(false);
    api.setApplyPreview(null);
    requestAnimationFrame(() => button.current?.focus());
  };
  const apply = () => {
    if (picked === null) return;
    api.applyFromPage(picked.id);
    setOpen(false);
    api.setApplyPreview(null);
    requestAnimationFrame(() => button.current?.focus());
  };
  const stop = picked?.id ?? ids[0];
  return (
    // The list and its buttons keep the tab's 244 px column (UX notes 4).
    <div
      className="flex w-[244px] max-w-full flex-col gap-1"
      onKeyDown={(event) => {
        if (open && event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          fold();
        }
      }}
    >
      <button
        ref={button}
        type="button"
        aria-expanded={open}
        className={`${TEXT_BUTTON} ${open ? "!bg-[var(--toolbar-icon-active-bg)] !text-[var(--toolbar-icon-active-fg)]" : ""}`}
        onClick={() => (open ? fold() : setOpen(true))}
      >
        <CopyPlus size={16} strokeWidth={1.5} absoluteStrokeWidth aria-hidden />
        Apply fixtures from page…
      </button>
      {open ? (
        <>
          <div
            role="radiogroup"
            aria-label="From page"
            className="flex flex-col"
            onKeyDown={(event) => {
              const target = applyKeyTarget(event.key, ids, view.applyPicked);
              if (target === null) return;
              event.preventDefault();
              api.setApplyPreview(target);
              requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-apply-page="${target}"]`)?.focus());
            }}
          >
            {view.applyCandidates.map((candidate) => (
              <button
                key={candidate.id}
                type="button"
                role="radio"
                data-apply-page={candidate.id}
                aria-label={`${candidate.name}, ${candidateCountText(candidate.count)}`}
                aria-checked={candidate.id === picked?.id}
                tabIndex={candidate.id === stop ? 0 : -1}
                className={`flex h-7 items-center justify-between rounded-md px-1 text-left outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] ${candidate.id === picked?.id ? "bg-[var(--value-fill)] shadow-[inset_3px_0_0_var(--editor-accent)]" : "hover:bg-[var(--editor-accent-hover)]"}`}
                onClick={() => api.setApplyPreview(candidate.id)}
              >
                <span className="truncate text-[13px] text-[var(--toolbar-icon)]">{candidate.name}</span>
                <span className="shrink-0 pl-2 text-xs tabular-nums text-[var(--panel-muted-fg)]">{candidateCountText(candidate.count)}</span>
              </button>
            ))}
          </div>
          {picked && applyReplacesText(view.jobFixtures.length) ? (
            <p data-apply-replaces="" className="text-xs text-[var(--panel-muted-fg)]">
              {applyReplacesText(view.jobFixtures.length)}
            </p>
          ) : null}
          <div className="flex gap-2">
            {picked ? (
              <button
                type="button"
                className="h-7 w-[140px] rounded-md bg-[var(--editor-accent)] text-[13px] text-white outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 pointer-coarse:h-9"
                onClick={apply}
              >
                {applyLabel(picked.count)}
              </button>
            ) : null}
            <button
              type="button"
              className="h-7 w-24 rounded-md border border-[color-mix(in_srgb,var(--toolbar-icon)_40%,transparent)] text-[13px] text-[var(--toolbar-icon)] outline-none hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] pointer-coarse:h-9"
              onClick={fold}
            >
              Cancel
            </button>
          </div>
        </>
      ) : null}
    </div>
  );
}

import { Plus } from "lucide-react";
import { useLayoutEffect, useRef } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { LaserSets } from "@/components/panel/LaserSets";
import { MATERIAL_ROW } from "@/components/panel/MaterialRow";
import { TestDetail } from "@/components/panel/TestDetail";
import { TestRowView } from "@/components/panel/TestRow";
import type { DeleteArm } from "@/hooks/useDeleteArm";
import type { MaterialsApi } from "@/hooks/useMaterials";
import type { MaterialBody as Body } from "@/lib/materialsView";
import { CHIP_IS_MESSAGE, outcomeOf, stepText, typedNumber } from "@/lib/settingsFieldText";

const OUTLINE_BUTTON =
  "flex h-7 w-[228px] items-center gap-1 rounded border border-[var(--lane-line)] px-2 text-xs text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]";

interface MaterialBodyProps {
  body: Body;
  api: MaterialsApi;
  arm: DeleteArm;
  onReturnFocus: () => void;
  /** The body's height once drawn, for the window's one tall item. */
  onHeight: (height: number) => void;
}

/**
 * The unfolded material (`specs/0112` `ux-notes.md` 7.4): Thickness, one block per laser, "+ Sets
 * for another laser" (the library's machine profiles in place), and the tests; or the open test.
 */
export function MaterialBody({ body, api, arm, onReturnFocus, onHeight }: MaterialBodyProps) {
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const element = box.current;
    if (!element) return undefined;
    const report = () => onHeight(Math.ceil(element.getBoundingClientRect().height));
    report();
    const observer = new ResizeObserver(report);
    observer.observe(element);
    return () => observer.disconnect();
  }, [onHeight]);

  const back = () => {
    const id = body.test?.id;
    api.openTest("");
    window.requestAnimationFrame(() => {
      box.current?.querySelector<HTMLElement>(`[data-test-id="${id}"]`)?.focus();
    });
  };

  // A deleted test leaves with its detail: the focus goes to the material's row.
  const deleted = () => {
    window.requestAnimationFrame(() => {
      box.current?.closest(`[${MATERIAL_ROW}]`)?.querySelector<HTMLElement>("[data-row-name]")?.focus();
    });
  };

  // The pressed row leaves with the list: the focus goes to the detail's Back, never to `body`.
  const openTest = (id: string) => {
    api.openTest(id);
    window.requestAnimationFrame(() => {
      box.current?.querySelector<HTMLElement>("[data-test-back]")?.focus();
    });
  };

  return (
    <div ref={box} data-material-body="" className="ml-2 flex flex-col gap-2 pb-2 pt-1">
      {body.test ? (
        <TestDetail material={body} test={body.test} api={api} arm={arm} onBack={back} onDeleted={deleted} />
      ) : (
        <>
          <EntryField
            label="Thickness, millimetres"
            prefix="Thickness"
            shown={String(body.thicknessMm)}
            mixed={false}
            suffix="mm"
            width={112}
            commitOnBlur
            inputMode="decimal"
            stepText={stepText}
            onSubmit={(text) => outcomeOf(api.setThickness(body.id, typedNumber(text)))}
            messages={CHIP_IS_MESSAGE}
            onReturnFocus={onReturnFocus}
          />
          {body.lasers.map((block) => (
            <LaserSets key={block.laser.id} materialId={body.id} block={block} api={api} arm={arm} onReturnFocus={onReturnFocus} />
          ))}
          {body.laserPickOpen ? (
            <div role="radiogroup" aria-label="Sets for another laser" className="flex flex-col gap-1">
              {body.otherLasers.map((laser) => (
                <button
                  key={laser.id}
                  type="button"
                  role="radio"
                  aria-checked={false}
                  onClick={() => api.pickLaser(body.id, laser.id)}
                  className="flex h-7 w-[228px] items-center rounded px-2 text-left text-[13px] text-[var(--toolbar-icon)] hover:bg-[var(--editor-accent-hover)] focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
                >
                  <span className="truncate">{laser.name}</span>
                </button>
              ))}
              <button type="button" onClick={() => api.openLaserPick(false)} className={OUTLINE_BUTTON}>
                Cancel
              </button>
            </div>
          ) : body.otherLasers.length > 0 ? (
            <button type="button" onClick={() => api.openLaserPick(true)} className={OUTLINE_BUTTON}>
              <Plus size={16} aria-hidden />
              Sets for another laser
            </button>
          ) : null}
          <h3 className="flex h-6 items-center text-xs font-semibold text-[var(--toolbar-icon)]">Tests ({body.tests.length})</h3>
          {body.tests.length > 0 ? (
            <ul role="list" aria-label="Tests" className="flex flex-col">
              {body.tests.map((test) => (
                <TestRowView key={test.id} row={test} onOpen={() => openTest(test.id)} />
              ))}
            </ul>
          ) : null}
        </>
      )}
    </div>
  );
}

import { Plus } from "lucide-react";
import { useRef, useState } from "react";

import { FixtureSetRow } from "@/components/workbench/FixtureSetRow";
import { useSetDrag } from "@/hooks/useSetDrag";
import type { DeleteArm } from "@/hooks/useDeleteArm";
import type { FixturesApi } from "@/hooks/useFixtures";
import { tabStopRow } from "@/lib/fixtureRows";
import type { FixtureRow, FixturesView } from "@/lib/fixturesView";
import { buttonClass, GLYPH } from "@/components/ui/looks";

/** The outline text button of the tab (`+ Set`, Apply fixtures from page): 244 x 28px. */
export const TEXT_BUTTON = buttonClass("secondary", "panel", "w-[244px]");

/**
 * The Machine sets section (`specs/0113-fixture-sets-and-pins` `ux-notes.md` 4): the set rows with
 * their drag, and "+ Set" under the last one (hidden at 16 sets, criterion 17).
 */
export function FixtureSetsSection({
  view,
  api,
  arm,
  openId,
  onOpen,
}: {
  view: FixturesView;
  api: FixturesApi;
  arm: DeleteArm;
  openId: string | null;
  onOpen: (row: FixtureRow) => void;
}) {
  const list = useRef<HTMLUListElement>(null);
  const [focusedId, setFocusedId] = useState<string | null>(null);
  const { drag, press } = useSetDrag(list, (from, to) => {
    const id = view.sets[from]?.id;
    if (id !== undefined) api.moveSet(id, to);
  });
  const stop = tabStopRow(
    view.sets.map((set) => set.id),
    null,
    focusedId,
  );
  const addSet = () => {
    const id = api.addSet();
    // The new row takes the focus (UX notes 4.2).
    if (id !== null) requestAnimationFrame(() => document.querySelector<HTMLElement>(`[data-set-row="${id}"]`)?.focus());
  };
  return (
    <>
      {view.sets.length > 0 ? (
        <ul ref={list} aria-label="Machine sets" className="flex flex-col">
          {view.sets.map((set, index) => (
            <FixtureSetRow
              key={set.id}
              set={set}
              index={index}
              count={view.sets.length}
              view={view}
              api={api}
              arm={arm}
              openId={openId}
              onOpen={onOpen}
              tabStop={set.id === stop}
              onFocusRow={() => setFocusedId(set.id)}
              insertion={drag?.gap === index ? "top" : drag?.gap === view.sets.length && index === view.sets.length - 1 ? "bottom" : null}
              dragSource={drag?.source === index}
              onDragPress={press}
            />
          ))}
        </ul>
      ) : null}
      {view.canAddSet ? (
        <button type="button" className={TEXT_BUTTON} onClick={addSet}>
          <Plus {...GLYPH} aria-hidden />
          Set
        </button>
      ) : null}
      {drag ? (
        <div
          aria-hidden
          className="pointer-events-none fixed z-50 w-[244px] rounded-[8px] bg-[var(--toolbar-bg)] px-2 text-sm leading-6 text-[var(--toolbar-icon)] opacity-60"
          style={{ top: drag.y - 12, left: drag.left }}
        >
          {view.sets[drag.source]?.name}
        </div>
      ) : null}
    </>
  );
}

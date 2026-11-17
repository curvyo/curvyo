import { SwatchBook } from "lucide-react";
import type { RefObject } from "react";

import { NumbersText } from "@/components/panel/NumbersText";
import { OutputPick } from "@/components/panel/OutputPick";
import { FixtureGlyph } from "@/components/workbench/FixtureGlyphs";
import type { FixtureOutput } from "@/lib/fixtureJobsView";
import { KIND_WORDS } from "@/lib/operationRows";

/** The ring of a fixture run: the row ring of the Operations list with the fixture glyph in it. */
export function FixtureRing({ size }: { size: number }) {
  return (
    <span
      aria-hidden
      data-fixture-ring
      style={{ width: size, height: size }}
      className="flex shrink-0 items-center justify-center rounded-full border-[1.25px] border-[var(--toolbar-icon)] bg-white text-[var(--toolbar-icon)] [&>svg]:size-[10px]"
    >
      <FixtureGlyph kind="holder" />
    </span>
  );
}

/**
 * The output part at the top of the Fixture jobs section's body (`specs/0029-laser-job-preview-and-
 * output` UX 6.2, P32): the pick "Fixture jobs run with" and, under it, one run line per kind that
 * runs: line a the fixture ring, the kind word and the time at the right in ink, line b the numbers of
 * the picked material's set or "No settings". Static, no Tab stop but the pick.
 */
export function FixtureOutputPart({
  output,
  buttonRef,
  onPick,
  onReturnFocus,
}: {
  output: FixtureOutput;
  buttonRef: RefObject<HTMLButtonElement | null>;
  onPick: (id: string) => void;
  onReturnFocus: () => void;
}) {
  const { pick } = output;
  const options = [{ id: "", name: "None", pickable: true, note: null }, ...pick.options];
  return (
    <div className="flex flex-col gap-1" data-fixture-output>
      <OutputPick
        label="Fixture jobs run with"
        Icon={SwatchBook}
        value={pick.material ?? ""}
        valueText={pick.name ?? "None"}
        options={options}
        buttonRef={buttonRef}
        openRequest={0}
        onPick={onPick}
        onReturnFocus={onReturnFocus}
        dataName="data-fixture-pick"
      />
      {output.runs.length > 0 ? (
        <ul role="list" aria-label="Fixture runs" className="flex flex-col gap-0.5">
          {output.runs.map((run) => (
            <li key={run.kind} data-fixture-run={run.kind} className="flex flex-col text-[11px] leading-[14px] text-[var(--panel-muted-fg)]">
              <div className="flex h-4 items-center gap-1 leading-4">
                <FixtureRing size={14} />
                <span className="min-w-0 flex-1 truncate">{KIND_WORDS[run.kind]}</span>
                <span className="shrink-0 tabular-nums text-[var(--toolbar-icon)]">{run.time}</span>
              </div>
              {run.numbers.length === 0 ? (
                <span className="block h-[14px] truncate">No settings</span>
              ) : (
                run.numbers.map((line, at) => <NumbersText key={at} line={line} />)
              )}
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

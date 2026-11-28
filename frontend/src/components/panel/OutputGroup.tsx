import { Crosshair, Download } from "lucide-react";
import { GLYPH, SECTION_HEADING } from "@/components/ui/looks";
import { useEffect, useRef, useState } from "react";

import { EntryField } from "@/components/panel/EntryField";
import { OutputChecks } from "@/components/panel/OutputChecks";
import { OutputPick } from "@/components/panel/OutputPick";
import { SendToMachine } from "@/components/panel/SendToMachine";
import { Tooltip } from "@/components/ui/tooltip";
import { useLinkEntry } from "@/hooks/useLinkEntry";
import type { OutputApi } from "@/hooks/useOutput";
import { sendLinkOf } from "@/lib/connectorLink";
import type { OperationId } from "@/lib/operationsView";
import { EXPORT_NAME, START_DAMAGED, machineLineParts } from "@/lib/outputText";
import type { Check, StartValue } from "@/lib/outputView";
import { parseMeasure } from "@/lib/printAndCutView";
import { sendCauseText } from "@/lib/sendText";
import { isTauri } from "@/platform/host";

/**
 * The output group at the end of the Operations tab of a laser page
 * (`specs/0029-laser-job-preview-and-output` `ux-notes.md` 6.3 to 6.6), after the Fixture jobs
 * section: the machine line, the Start pick, the checks, the total and Export job file. Nothing in
 * it is a number to type; a wrong number is fixed where it lives and every name in a check is a
 * way there. Export is the one recorded exception to "Hidden, not disabled" (M5): `aria-disabled`,
 * still a Tab stop, the reason as tooltip and description.
 */
export function OutputGroup({
  api,
  onReturnFocus,
  onOpenOperation,
  onUnfoldOperation,
  onFixturePick,
}: {
  api: OutputApi;
  onReturnFocus: () => void;
  /** A check's operation: its row selected, unfolded and scrolled into view in the list above. */
  /** `focus` names the row's field the check points at (`dither`, `lowest_power`), else `null`. */
  onOpenOperation: (id: OperationId, focus: "dither" | "lowest_power" | null) => void;
  /** Unfolds an operation's row in the list above, the focus kept (a line that selects items). */
  onUnfoldOperation: (id: OperationId) => void;
  /** The "Fixture jobs" check: expand the section and focus its pick. */
  onFixturePick: () => void;
}) {
  const view = api.view;
  const startButton = useRef<HTMLButtonElement>(null);
  const [startRequest, setStartRequest] = useState(0);
  const send = api.view?.send ?? null;
  const entry = useLinkEntry(send?.profileId ?? null, send?.address ?? "", isTauri && send !== null, true);
  const spelling = isTauri ? sendLinkOf(entry) : "browser";
  const sendProfile = send?.profileId ?? null;
  const { applyLink, applyMachine, willStart } = api.send;
  useEffect(() => {
    if (sendProfile !== null) applyLink(sendProfile, spelling);
  }, [sendProfile, spelling, applyLink]);
  useEffect(() => {
    if (sendProfile !== null) applyMachine(entry);
  }, [sendProfile, entry, applyMachine]);
  if (view === null) return null;
  const machine = machineLineParts(view.machine);
  const press = (check: Check) => {
    const target = check.target;
    if (target.kind === "items") {
      api.selectItems(target.ids);
      if (target.unfold !== null) onUnfoldOperation(target.unfold);
    }
    else if (target.kind === "operations" && target.ids.length > 0) onOpenOperation(target.ids[0], target.focus);
    else if (target.kind === "fixture_pick") onFixturePick();
    else if (target.kind === "start_pick") setStartRequest((count) => count + 1);
  };
  const exportOn = view.export.enabled;
  const describedBy = "output-export-reason";
  const options = view.start.options.map((option) => ({
    id: option.value,
    name: option.label,
    pickable: true,
    note: null,
  }));
  const current = view.start.options.find((option) => option.value === view.start.value);
  const fixtureLine = view.total?.fixtures ?? null;
  return (
    <section aria-label="Output" data-output-group className="flex flex-col gap-2">
      <Divider />
      <h3 className={SECTION_HEADING}>Output</h3>
      <p className="text-xs text-[var(--toolbar-icon)]" data-output-machine>
        <span className="font-semibold">{machine.name}</span>
        <span className="text-[var(--panel-muted-fg)]">{machine.rest}</span>
      </p>
      <OutputPick
        label="Start"
        Icon={Crosshair}
        value={view.start.value ?? ""}
        valueText={view.start.damaged ? START_DAMAGED : (current?.label ?? "Machine origin")}
        options={options}
        buttonRef={startButton}
        openRequest={startRequest}
        onPick={(id) => api.setStart(id as StartValue)}
        onReturnFocus={onReturnFocus}
        dataName="data-start-pick"
      />
      {view.start.userOrigin === null ? null : (
        <UserOriginRow point={view.start.userOrigin} api={api} onReturnFocus={onReturnFocus} />
      )}
      {view.checks.length > 0 ? (
        <>
          <Divider />
          <OutputChecks checks={view.checks} onPress={press} />
        </>
      ) : null}
      {view.total ? <Divider /> : null}
      {view.total ? (
        <section aria-label="Total" role="status" aria-atomic="true" className="flex flex-col gap-0.5">
          <p className="text-sm leading-4 font-semibold text-[var(--toolbar-icon)]">{view.total.time}</p>
          {view.total.runs === null ? null : (
            <p className="text-xs tabular-nums text-[var(--panel-muted-fg)]" data-total-runs>
              {view.total.runs.before}
              <button
                type="button"
                onClick={() => api.selectRunning()}
                className="rounded-[5px] underline outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {view.total.runs.button}
              </button>
              {view.total.runs.after}
            </p>
          )}
          {fixtureLine === null ? null : (
            <p className="text-xs tabular-nums text-[var(--panel-muted-fg)]" data-total-fixtures>
              <button
                type="button"
                onClick={() => api.selectItems(fixtureLine.ids)}
                className="rounded-[5px] underline outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)]"
              >
                {fixtureLine.button}
              </button>
              {fixtureLine.after}
            </p>
          )}
          <p className="text-xs tabular-nums text-[var(--panel-muted-fg)]">{view.total.lengths}</p>
        </section>
      ) : null}
      <div className="flex gap-1">
        <Tooltip side="top" content={exportOn ? null : view.export.tooltip}>
          <button
            type="button"
            data-export-job
            aria-label={EXPORT_NAME}
            aria-disabled={exportOn ? undefined : true}
            aria-describedby={exportOn ? undefined : describedBy}
            onClick={(event) => {
              if (!exportOn) return;
              void api.exportJob();
              if (event.detail > 0) onReturnFocus();
            }}
            className={`flex h-8 min-w-0 flex-1 items-center justify-center gap-2 rounded-[5px] text-sm outline-none focus-visible:ring-2 focus-visible:ring-[var(--editor-accent)] focus-visible:ring-offset-1 ${
              exportOn
                ? "bg-[var(--toolbar-icon-active-bg)] text-[var(--toolbar-icon-active-fg)] hover:brightness-110"
                : "cursor-default bg-[var(--field-disabled-bg)] text-[var(--field-disabled-fg)]"
            }`}
          >
            <Download {...GLYPH} aria-hidden />
            <span aria-hidden>{EXPORT_NAME}</span>
          </button>
        </Tooltip>
        {send === null ? null : (
          <SendToMachine
            enabled={send.enabled}
            cause={sendCauseText(send)}
            sending={send.cause === "sending"}
            starting={entry.startStep}
            starts={send.starts}
            job={send.job}
            onSend={(start) => api.send.press(start)}
            willStart={() => willStart(send.profileId)}
            onReturnFocus={onReturnFocus}
          />
        )}
      </div>
      {exportOn ? null : (
        <span id={describedBy} className="sr-only">
          {view.export.tooltip}
        </span>
      )}
    </section>
  );
}

const ORIGIN_MESSAGES = { range: "Enter a number from -10000 to 10000" } as const;

/** Up and Down step 0.01 mm, Shift 0.1 mm (`specs/0118` W43, the `0121` Mark row rules). */
function stepOrigin(text: string, key: string, shift: boolean): string | null {
  const value = parseMeasure(text);
  if (typeof value !== "number" || (key !== "ArrowUp" && key !== "ArrowDown")) return null;
  const step = (shift ? 0.1 : 0.01) * (key === "ArrowUp" ? 1 : -1);
  return (Math.round((value + step) * 100) / 100).toFixed(2);
}

/** The User origin row under the Start pick while User origin is picked (`specs/0118` W43): the
 * stored point in bed mm, each commit one step. */
function UserOriginRow({
  point,
  api,
  onReturnFocus,
}: {
  point: { x: number; y: number };
  api: OutputApi;
  onReturnFocus: () => void;
}) {
  const submit = (axis: "x" | "y", text: string): string => {
    const value = parseMeasure(text);
    if (value === undefined || value === null) return "invalid:range";
    const next = axis === "x" ? { ...point, x: value } : { ...point, y: value };
    return api.typeUserOrigin(next.x, next.y) ? "committed" : "unchanged";
  };
  return (
    <div role="group" aria-label="User origin" data-user-origin className="-mt-1 flex gap-2">
      {(["x", "y"] as const).map((axis) => (
        <div key={axis} className="relative">
          <span
            aria-hidden
            className="pointer-events-none absolute top-1/2 left-2 z-10 -translate-y-1/2 text-xs text-[var(--panel-muted-fg)]"
          >
            {axis.toUpperCase()}
          </span>
          <EntryField
            label={`User origin ${axis.toUpperCase()}`}
            shown={point[axis].toFixed(2)}
            mixed={false}
            suffix="mm"
            width={118}
            inputMode="decimal"
            align="right"
            onSubmit={(text) => submit(axis, text)}
            stepText={(text, key, shift) => stepOrigin(text, key, shift)}
            messages={ORIGIN_MESSAGES}
            onReturnFocus={onReturnFocus}
          />
        </div>
      ))}
    </div>
  );
}

/** The 1 px rule between the group's parts (draft 02 `.hr`): before the group, the checks and the
 * total. */
function Divider() {
  return (
    <div
      aria-hidden
      data-output-divider
      className="my-0.5 h-px w-full shrink-0"
      style={{ background: "color-mix(in srgb, var(--toolbar-icon) 25%, transparent)" }}
    />
  );
}

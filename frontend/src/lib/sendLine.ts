// The state line and bar under the buttons of the Output group (`specs/0125-send-to-machine` S13,
// S15; UX notes §4, §5): the words of the connector's job for one profile, derived from the shared
// link entry. Pure, so the tests load it without a bundler.
import { formatCount, formatSize, hasProgress, isLossText, type StatusMessage } from "./connectorText.ts";
import { type LinkEntry } from "./connectorLink.ts";

/** The longest line (UX notes §4, U5). */
export const LINE_CHARS = 70;

/** The thin bar under the line. */
export interface Bar {
  line: number;
  lines: number;
  failure: boolean;
}

/** One state line with its bar. */
export interface StateLine {
  text: string;
  /** The failure look (`CircleAlert` or `TriangleAlert`, `--destructive`). */
  failure: boolean;
  /** `alarm` shows `TriangleAlert`, any other failure `CircleAlert`. */
  alarm: boolean;
  bar: Bar | null;
  /** The state words only, for the screen reader: no counts (U9). */
  spoken: string;
}

/** `name` cut at its end with an ellipsis so that `before + name + after` stays within
 * [`LINE_CHARS`]. */
export function withName(before: string, name: string, after = ""): string {
  const room = LINE_CHARS - before.length - after.length;
  const shown = name.length <= room ? name : `${name.slice(0, Math.max(room - 1, 0))}…`;
  return `${before}${shown}${after}`;
}

const plain = (text: string, bar: Bar | null = null, spoken = text): StateLine => ({
  text,
  failure: false,
  alarm: false,
  bar,
  spoken,
});

/** The words for the job in `status`; `null` when the line is absent. A job sent from this app
 * but not from this page adds " · <name>" in the active and done states (S15, U6). */
function jobLine(entry: LinkEntry, status: StatusMessage): StateLine | null {
  const { held, pageJobId } = entry;
  const id = status.job?.job_id ?? null;
  const mine = held !== null && id !== null && held.jobId === id ? held : null;
  const other = mine !== null && mine.jobId !== pageJobId ? mine.name : null;
  const lines = status.job?.lines ?? 0;
  const line = status.job?.line ?? 0;
  const bar = (failure = false): Bar => ({ line, lines, failure });
  const words = (text: string, b: Bar | null, spoken = text): StateLine =>
    plain(other === null ? text : withName(`${text} · `, other), b, spoken);
  switch (status.state.name) {
    case "Receiving":
      return plain(held ? withName("Sending ", held.name) : "Receiving a job");
    case "Ready":
      return plain(
        mine
          ? withName("Ready: ", mine.name, `, ${formatCount(mine.lines)} lines, ${formatSize(mine.bytes)}`)
          : `Ready, ${formatCount(lines)} lines`,
      );
    case "AwaitingLocalStart":
      return words("Waiting for the start at the machine", bar());
    case "Running":
      return words(`Running ${formatCount(line)} / ${formatCount(lines)}`, bar(), "Running");
    case "Paused":
      return words(`Paused at line ${formatCount(line)}`, bar());
    case "Done":
      if (status.state.result === "Completed") {
        return words(`Completed, ${formatCount(lines)} lines`, { line: lines, lines, failure: false });
      }
      if (status.state.result === "Stopped") return words(`Stopped at line ${formatCount(line)}`, bar());
      return { ...words(status.detail ?? `Failed at line ${formatCount(line)}`, bar(true)), failure: true };
    case "Alarm":
      return {
        text: `Alarm: ${status.alarm ?? ""}`,
        failure: true,
        alarm: true,
        bar: hasProgress(status) ? bar(true) : null,
        spoken: `Alarm: ${status.alarm ?? ""}`,
      };
    case "NoController":
    case "Idle":
      // A lost job or an unconfirmed stop, also after the reconnect (`0127` UX notes section 8);
      // an open failure without a job is the Connector row's line only.
      return isLossText(status.detail)
        ? { text: status.detail as string, failure: true, alarm: false, bar: null, spoken: status.detail as string }
        : null;
    default:
      return null;
  }
}

/** The hidden alert's text once the entry moved from `before` to `now` while the group is shown
 * (UX notes §4): a failure that answers a press made in this group, or an alarm that arrives
 * after a status was seen here; `""` once neither is left; `null` keeps the text. A failure or an
 * alarm the group finds when it is shown, or that the first status read brings, is never an
 * alert. */
export function alertAfter(before: LinkEntry, now: LinkEntry, pressedHere: boolean): string | null {
  const failure = now.sendFailure;
  if (failure !== null && failure !== before.sendFailure) return pressedHere ? failure.text : null;
  const alarm = now.status?.state.name === "Alarm";
  if (alarm && before.status !== null && before.status.state.name !== "Alarm") {
    return `Alarm: ${now.status?.alarm ?? ""}`;
  }
  const loss = isLossText(now.status?.detail) ? (now.status?.detail as string) : null;
  if (loss !== null && before.status !== null && before.status.detail !== loss) return loss;
  return failure === null && !alarm && loss === null ? "" : null;
}

/** The line and bar of the Output group for `entry`, `null` when there is nothing to say. */
export function stateLine(entry: LinkEntry): StateLine | null {
  if (entry.sendFailure) {
    return { text: entry.sendFailure.text, failure: true, alarm: false, bar: null, spoken: entry.sendFailure.text };
  }
  if (entry.pageSend) {
    return plain(withName("Sending ", entry.pageSend.name));
  }
  if (entry.status === null) return null;
  const line = jobLine(entry, entry.status);
  if (line === null) return null;
  if (entry.lost) {
    return { ...line, text: "Connection lost", failure: true, alarm: false, spoken: "Connection lost" };
  }
  return line;
}

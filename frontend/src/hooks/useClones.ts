import { useCallback, useEffect, useRef, useState } from "react";

import { afterFrames, useBusyWindow } from "@/hooks/useRailCommands";
import type { WasmSession } from "@/lib/editorSession";
import {
  CLONE_REFUSAL_MS,
  CLONE_STATUS_MS,
  type ClonePanelState,
  type CloneTag,
  NO_CLONE_PANEL,
  ORPHAN_NOTICE_MS,
} from "@/lib/cloneText";

/** The height of the strip at the top of the canvas that the Select bar takes: its 12 px top
 * edge, its 36 px and a 12 px gap. */
const BAR_STRIP_PX = 60;

/** Shows a notice in the canvas notice slot or the panel. */
type ShowNotice = (
  text: string,
  ms?: number,
  source?: "canvas" | "panel",
  kind?: "status" | "alert",
) => void;

export type CloneCommand = "linked-clone" | "unlink" | "select-original" | "select-root" | "select-clones";

/** The notice of Linked clone, shown 4 px under its button (UX notes "Focus"). */
export interface CloneBarNotice {
  id: number;
  text: string;
  alert: boolean;
}

/** Where a clone command was pressed: the Select bar or the canvas, the Style tab's block or
 * origin line, or the Layers toolbar. */
export type CloneSource = "canvas" | "panel" | "layers";

/** Where the keyboard focus goes after a key press in the Style tab (UX notes "Focus"): Stroke
 * Paint after Unlink, the first control of the new body after Select original or root. */
export interface CloneFocusRequest {
  id: number;
  target: "stroke" | "first";
}

export interface ClonesApi {
  /** The Linked clone button is in the Select bar. */
  shown: boolean;
  /** What the Style tab shows about linked clones. */
  panel: ClonePanelState;
  /** The tags and edge chips of the canvas marks. */
  tags: CloneTag[];
  /** Runs `command`; `source` says where its notice shows; `keyboard` moves the focus after it. */
  apply: (command: CloneCommand, source?: CloneSource, keyboard?: boolean) => void;
  /** Re-reads the session. Call after any change of it. */
  refresh: () => void;
  /** The notice of the last Linked clone, under the button; `null` once it ended. */
  barNotice: CloneBarNotice | null;
  /** The notice of the last command of the Style tab, where its button was; `null` once ended. */
  panelNotice: CloneBarNotice | null;
  /** The focus move asked by the last key press in the Style tab. */
  focusRequest: CloneFocusRequest | null;
  /** The pointer or focus is on Select original or Select root original, or left it. */
  hover: (kind: "original" | "root" | "") => void;
  /** A Delete key says how the clones that followed what it removed were repaired. */
  handleKeyOutcome: (outcome: string) => void;
}

function samePanel(a: ClonePanelState, b: ClonePanelState): boolean {
  return (Object.keys(a) as (keyof ClonePanelState)[]).every((key) => a[key] === b[key]);
}

function sameTags(a: CloneTag[], b: CloneTag[]): boolean {
  return a.length === b.length && a.every((tag, i) => JSON.stringify(tag) === JSON.stringify(b[i]));
}

/**
 * Linked clones in the frontend (`specs/0049-linked-clone`): the Select-bar button, the clone
 * block and origin line of the Style tab, the canvas tags, and the notices of the commands, of
 * the clone repair after a removal and of the orphan repair after an Open. Every decision and
 * every sentence is the session's; this hook forwards presses and shows what comes back.
 */
export function useClones(
  getSession: () => WasmSession | null,
  onChange: () => void,
  showNotice: ShowNotice,
  containerRef: React.RefObject<HTMLDivElement | null>,
): ClonesApi {
  const [shown, setShown] = useState(false);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  useBusyWindow(busy);
  const [barNotice, setBarNotice] = useState<CloneBarNotice | null>(null);
  const noticeCounter = useRef(0);
  useEffect(() => {
    if (!barNotice) {
      return undefined;
    }
    const timer = window.setTimeout(
      () => setBarNotice(null),
      barNotice.alert ? CLONE_REFUSAL_MS : CLONE_STATUS_MS,
    );
    return () => window.clearTimeout(timer);
  }, [barNotice]);
  const [panelNotice, setPanelNotice] = useState<CloneBarNotice | null>(null);
  useEffect(() => {
    if (!panelNotice) {
      return undefined;
    }
    const timer = window.setTimeout(
      () => setPanelNotice(null),
      panelNotice.alert ? CLONE_REFUSAL_MS : CLONE_STATUS_MS,
    );
    return () => window.clearTimeout(timer);
  }, [panelNotice]);
  const [focusRequest, setFocusRequest] = useState<CloneFocusRequest | null>(null);
  const [panel, setPanel] = useState<ClonePanelState>(NO_CLONE_PANEL);
  const [tags, setTags] = useState<CloneTag[]>([]);
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  const refresh = useCallback(() => {
    const session = getSession();
    if (!session) {
      return;
    }
    setShown(session.linked_clone_shown());
    const raw = session.clone_panel();
    const next: ClonePanelState = {
      clonesOnly: raw.clones_only,
      count: raw.count,
      original: raw.original,
      deepest: raw.deepest,
      root: raw.root,
      followers: raw.followers,
      followed: raw.followed,
    };
    raw.free();
    setPanel((previous) => (samePanel(previous, next) ? previous : next));
    // The DOM chips over the canvas, which no tag may meet (criterion 60).
    const host = containerRef.current;
    const canvas = host?.querySelector("canvas");
    if (host && canvas) {
      const origin = canvas.getBoundingClientRect();
      const flat: number[] = [];
      for (const element of Array.from(host.children)) {
        if (element === canvas.parentElement || element.contains(canvas) || element.hasAttribute("data-clone-tags")) {
          continue;
        }
        const rect = element.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          flat.push(rect.left - origin.left, rect.top - origin.top, rect.width, rect.height);
        }
      }
      // The tool rail and the Select bar are the edge of the free canvas: no tag goes under
      // them, after a pan either (`0049` UX re-check). The rail spans the height up to
      // `--rail-right`; the bar strip is the top 60 px right of it.
      const rail = parseFloat(getComputedStyle(host).getPropertyValue("--rail-right")) || 0;
      flat.push(0, 0, rail + 12, origin.height);
      flat.push(rail, 0, Math.max(0, origin.width - rail), BAR_STRIP_PX);
      session.set_clone_tag_obstacles(new Float64Array(flat));
    }
    const nextTags: CloneTag[] = session.clone_tags().map((view) => {
      const tag: CloneTag = {
        kind: view.kind === "chip" ? "chip" : "tag",
        text: view.text,
        x: view.x,
        y: view.y,
        w: view.w,
        toward: view.toward,
      };
      view.free();
      return tag;
    });
    setTags((previous) => (sameTags(previous, nextTags) ? previous : nextTags));
    // A command refused because a clone would pass the limit says so (criterion 17).
    const limit = session.take_clone_limit_notice();
    if (limit !== "") {
      showNotice(limit, CLONE_REFUSAL_MS, "canvas", "alert");
    }
    const orphans = session.take_orphan_notice();
    if (orphans !== "") {
      showNotice(orphans, ORPHAN_NOTICE_MS);
    }
  }, [getSession, showNotice, containerRef]);

  const run = useCallback(
    (command: CloneCommand, source: CloneSource, keyboard: boolean) => {
      const session = getSession();
      if (!session) {
        return;
      }
      const raw =
        command === "linked-clone"
          ? session.linked_clone()
          : command === "unlink"
            ? session.unlink_clones()
            : command === "select-original"
              ? session.select_original()
              : command === "select-root"
                ? session.select_root_original()
                : session.select_clones();
      const result = { kind: raw.kind, notice: raw.notice };
      raw.free();
      if (result.kind !== "ignored" && result.notice !== "") {
        const refused = result.kind === "refused";
        noticeCounter.current += 1;
        const notice = { id: noticeCounter.current, text: result.notice, alert: refused };
        if (source === "panel") {
          // The Style tab's commands: where the pressed button was (UX notes "Focus").
          setPanelNotice(notice);
        } else if (command === "linked-clone" && source === "canvas") {
          // The Select bar's Linked clone: under its button.
          setBarNotice(notice);
        } else {
          // The Layers toolbar's: the canvas notice slot, as its Duplicate.
          showNotice(result.notice, refused ? CLONE_REFUSAL_MS : undefined, "canvas", refused ? "alert" : "status");
        }
      }
      if (keyboard && source === "panel" && result.kind === "done") {
        noticeCounter.current += 1;
        setFocusRequest({ id: noticeCounter.current, target: command === "unlink" ? "stroke" : "first" });
      }
      onChangeRef.current();
    },
    [getSession, showNotice],
  );

  const apply = useCallback(
    (command: CloneCommand, source: CloneSource = "canvas", keyboard = false) => {
      if (busyRef.current) {
        return;
      }
      if (command !== "linked-clone" && command !== "unlink") {
        run(command, source, keyboard);
        return;
      }
      // Linked clone and Unlink of thousands of objects take a moment: the wait cursor and the
      // busy state of the rail commands (criterion 45).
      busyRef.current = true;
      setBusy(true);
      void (async () => {
        try {
          await afterFrames(2);
          run(command, source, keyboard);
        } finally {
          busyRef.current = false;
          setBusy(false);
        }
      })();
    },
    [run],
  );

  const hover = useCallback(
    (kind: "original" | "root" | "") => {
      getSession()?.set_clone_hover(kind);
    },
    [getSession],
  );

  const handleKeyOutcome = useCallback(
    (outcome: string) => {
      if (outcome !== "deleted") {
        return;
      }
      // "Deleted 1 object. 1 clone became a path." (criterion 31): both sentences, one notice.
      const notice = getSession()?.take_delete_notice() ?? "";
      if (notice !== "") {
        showNotice(notice);
      }
    },
    [getSession, showNotice],
  );

  return {
    shown,
    panel,
    tags,
    apply,
    refresh,
    barNotice,
    panelNotice,
    focusRequest,
    hover,
    handleKeyOutcome,
  };
}

import { useCallback, useEffect, useRef, useState } from "react";

import type { DocumentPanelApi } from "@/hooks/useDocumentPanel";
import type { EditorSession } from "@/hooks/useEditorSession";
import type { PanelOpenApi } from "@/hooks/usePanelOpen";
import type { ProjectApi } from "@/hooks/useProject";
import { mintPeerId, wasmReady } from "@/lib/editorSession";
import {
  type RecoveryRow,
  type RecoveryStore,
  type RecoveryWriteError,
  browserRecoveryStore,
  desktopRecoveryStore,
  indexedDbBackend,
  mintRecoveryKey,
} from "@/lib/recoveryStore";
import { type RowView, localParts, parseRowView } from "@/lib/recoveryRows";
import { RecoveryWorkerLink, type RecoveryWorker } from "@/lib/recoveryWriterJobs";
import { isTauri } from "@/platform/host";
import type { WasmSession } from "@/wasm-bindings/curvyo_editor_wasm.js";
import {
  WasmAssembler,
  WasmRecoverySchedule,
  recovery_check_entry,
  recovery_failure_sentence,
  recovery_heading,
  recovery_help_line,
  recovery_prune_plan,
  recovery_record_text,
  recovery_row_view,
} from "@/wasm-bindings/curvyo_editor_wasm.js";

/** How often the page looks for a new step and a due write (ms): "within 1 s" (criterion 1). */
const TICK_MS = 250;
/** How long the one canvas notice of a failed write stays (ms). */
const NOTICE_MS = 8000;

/** One row with its words. */
export interface RecoveryRowItem {
  row: RecoveryRow;
  view: RowView;
}

/** What the File tab shows of the recovery copies (`specs/0136-autosave-and-restore`). */
export interface RecoveryApi {
  /** The copies waiting at start, newest first; empty once the last one went. */
  rows: RecoveryRowItem[];
  heading: string;
  /** "Kept in <path>" or "Kept in this browser". */
  help: string;
  /** The full path for the tooltip, desktop only. */
  where: string | null;
  /** The failed-write line of the project block, `null` while writes work. */
  failure: string | null;
  restore: (item: RecoveryRowItem, refocus?: () => void) => void;
  discard: (item: RecoveryRowItem) => Promise<void>;
  /** The project's copy goes before the app quits. */
  end: () => Promise<void>;
}

interface RecoveryDeps {
  editor: EditorSession;
  project: ProjectApi;
  panelOpen: PanelOpenApi;
  panel: DocumentPanelApi;
}

/** The recovery rules the browser's listing asks of the wasm module. */
const browserRules = {
  check: (record: string, copy: Uint8Array) =>
    JSON.parse(recovery_check_entry(record, copy)) as ReturnType<
      NonNullable<Parameters<typeof browserRecoveryStore>[1]["check"]>
    >,
  prune: (candidates: Parameters<Parameters<typeof browserRecoveryStore>[1]["prune"]>[0]) =>
    JSON.parse(recovery_prune_plan(JSON.stringify(candidates))) as string[],
};

/** The words a write failure gets: a host error is its code or its own reason. */
function failureSentence(error: unknown): string {
  const code = (error as RecoveryWriteError | undefined)?.code ?? String((error as Error)?.message ?? error);
  return recovery_failure_sentence(code);
}

/**
 * The recovery copy of the open project (`specs/0136-autosave-and-restore`, ADR 0079): a step
 * schedule that is Rust's, a worker that holds a replica and packs, a store that keeps the bytes
 * (the host's app data folder or the browser's IndexedDB), the offer of the copies a crash left,
 * Restore and Discard. It writes nothing the maker sees: only a failed write shows a line and one
 * notice (criterion 13).
 */
export function useRecovery({ editor, project, panelOpen, panel }: RecoveryDeps): RecoveryApi {
  const depsRef = useRef({ editor, project, panelOpen, panel });
  useEffect(() => {
    depsRef.current = { editor, project, panelOpen, panel };
  });
  const storeRef = useRef<RecoveryStore | null>(null);
  const workerRef = useRef<RecoveryWorker | null>(null);
  const [items, setItems] = useState<RecoveryRowItem[]>([]);
  const [where, setWhere] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  /** The session being watched, its key and schedule. */
  const live = useRef<{
    session: WasmSession | null;
    key: string;
    /** Made in `swap`, once a session exists, so never before the wasm module is up. */
    schedule: WasmRecoverySchedule | null;
    stepAt: number;
    wasDirty: boolean;
    base: string;
  }>({ session: null, key: "", schedule: null, stepAt: 0, wasDirty: false, base: "" });
  /** The key a Restore adopts for the session it is about to open. */
  const adopt = useRef<string | null>(null);
  /** The one queue per key: writes and deletes of a key run in order, so a trailing write that
   * lands after Save cannot bring the copy back (ADR 0079 §6). */
  const chains = useRef(new Map<string, Promise<unknown>>());
  const pointerDown = useRef(false);

  const chain = useCallback(<T,>(key: string, work: () => Promise<T>): Promise<T> => {
    const previous = chains.current.get(key) ?? Promise.resolve();
    const next = previous.then(work, work);
    chains.current.set(
      key,
      next.catch(() => undefined),
    );
    return next;
  }, []);

  const store = useCallback((): RecoveryStore => {
    storeRef.current ??= isTauri ? desktopRecoveryStore : browserRecoveryStore(indexedDbBackend, browserRules);
    return storeRef.current;
  }, []);

  const worker = useCallback((): RecoveryWorker => {
    workerRef.current ??= new RecoveryWorkerLink(mintPeerId());
    return workerRef.current;
  }, []);

  /** Records the saved file as it is now: the base of criterion 6 (desktop only). */
  const captureBase = useCallback(async () => {
    const path = depsRef.current.project.info.path;
    const text = isTauri && path ? await store().fileBase(path).catch(() => "") : "";
    live.current.base = text;
  }, [store]);

  // ---- the offer at start --------------------------------------------------------------------

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        await wasmReady();
        const listing = await store().list();
        if (cancelled || listing.rows.length === 0) {
          return;
        }
        const now = localParts(Date.now());
        setWhere(listing.path);
        setItems(
          listing.rows.map((row) => ({
            row,
            view: parseRowView(recovery_row_view(JSON.stringify(row), JSON.stringify(localParts(row.last_step_ms)), JSON.stringify(now))),
          })),
        );
        // Frage 317 A: the panel body is shown and the File tab is active, once; focus stays on
        // the canvas.
        const { panelOpen: open, panel: view } = depsRef.current;
        open.setOpen(true, false);
        view.pressTab("file");
      } catch {
        // A store that cannot be read offers nothing; the project still works.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [store]);

  // ---- writing --------------------------------------------------------------------------------

  const fail = useCallback((error: unknown) => {
    const sentence = failureSentence(error);
    setFailure(sentence);
    if (live.current.schedule?.failed(performance.now())) {
      depsRef.current.editor.showHistoryNotice(sentence, NOTICE_MS, "canvas", "alert");
    }
  }, []);

  /** One write: the main-thread part (the delta and the assets the host lacks), then the worker's
   * pack, then the host's files. A step that lands meanwhile stays for the next write. */
  const write = useCallback(
    (current: typeof live.current, session: WasmSession) => {
      const { key, schedule } = current;
      if (!schedule) {
        return;
      }
      const sessionNow = () => depsRef.current.editor.getSession();
      schedule.begin();
      void chain(key, async () => {
        if (sessionNow() !== session) {
          schedule.written(performance.now());
          return;
        }
        try {
          await store().begin(key);
          const hand = async (): Promise<string> => {
            const delta = session.recovery_export();
            const { full, bytes } = delta;
            delta.free();
            return worker().apply(full, bytes);
          };
          let answer = await hand();
          if (answer === "reseed") {
            // The replica went its own way: a snapshot follows, here at the idle moment.
            session.recovery_reseed();
            answer = await hand();
          }
          if (answer !== "") {
            throw new Error("the recovery copy could not be prepared");
          }
          const names = JSON.parse(session.recovery_asset_names()) as string[];
          const assets = names.map((name) => ({ name, bytes: session.recovery_asset(name) }));
          const packed = await worker().pack();
          if (!packed) {
            throw new Error("the recovery copy could not be packed");
          }
          for (const asset of assets) {
            await store().writeAsset(key, asset.name, asset.bytes);
          }
          const info = depsRef.current.project.info;
          const record = recovery_record_text(key, info.fileName ?? undefined, current.base, current.stepAt, packed.pages);
          await store().writeCopy(key, record, packed.bytes);
          if (sessionNow() === session) {
            session.recovery_assets_done(JSON.stringify(names));
          }
          schedule.written(performance.now());
          setFailure(null);
        } catch (error) {
          fail(error);
        }
      });
    },
    [chain, fail, store, worker],
  );

  /** The project's copy goes and the session starts over under the same key (Save, a clean
   * project). */
  const dropCopy = useCallback(
    (current: typeof live.current, session: WasmSession) => {
      const { key, schedule } = current;
      if (!schedule) {
        return;
      }
      void chain(key, () => store().deleteCopy(key).catch(() => undefined));
      session.recovery_start_over();
      schedule.ended();
      setFailure(null);
    },
    [chain, store],
  );

  /** A session started or ended: the old one's key goes, the new one gets its own (or the
   * restored copy's). */
  const swap = useCallback(
    (session: WasmSession | null) => {
      const current = live.current;
      if (current.session !== null) {
        const old = current.key;
        void chain(old, () => store().end(old).catch(() => undefined));
      }
      current.schedule?.free();
      current.schedule = session ? new WasmRecoverySchedule() : null;
      current.session = session;
      current.key = adopt.current ?? mintRecoveryKey();
      current.stepAt = Date.now();
      current.wasDirty = false;
      workerRef.current?.reset();
      setFailure(null);
      if (!session) {
        return;
      }
      session.recovery_start_over();
      session.recovery_changed();
      if (adopt.current === null && session.is_dirty()) {
        // A project that starts unsaved (an SVG) has steps no copy holds.
        current.schedule?.step();
      }
      if (adopt.current !== null) {
        const key = current.key;
        void store().begin(key).catch(() => undefined);
        adopt.current = null;
      }
      void captureBase();
    },
    [captureBase, chain, store],
  );

  useEffect(() => {
    const down = () => {
      pointerDown.current = true;
    };
    const up = () => {
      pointerDown.current = false;
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("pointercancel", up, true);
    const timer = window.setInterval(() => {
      const session = depsRef.current.editor.getSession();
      const current = live.current;
      if (session !== current.session) {
        swap(session);
        return;
      }
      const schedule = current.schedule;
      if (!session || !schedule) {
        return;
      }
      if (session.recovery_changed()) {
        schedule.step();
        current.stepAt = Date.now();
      }
      const dirty = session.is_dirty();
      if (current.wasDirty && !dirty) {
        // Save wrote the file: the copy goes, the next step is written at once (criterion 3).
        dropCopy(current, session);
        void captureBase();
      }
      current.wasDirty = dirty;
      // Never for a drag in flight.
      if (pointerDown.current) {
        return;
      }
      const due = schedule.due(performance.now(), dirty);
      if (due === "write") {
        write(current, session);
      } else if (due === "delete") {
        dropCopy(current, session);
      }
    }, TICK_MS);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("pointercancel", up, true);
    };
  }, [captureBase, dropCopy, swap, write]);

  // The saved file the copy is based on is captured when the project's path changes (open, Save as).
  const path = project.info.path;
  useEffect(() => {
    void captureBase();
  }, [path, captureBase]);

  // ---- Restore and Discard --------------------------------------------------------------------

  const restore = useCallback(
    (item: RecoveryRowItem, refocus?: () => void) => {
      const { project: api } = depsRef.current;
      const open = async () => {
        const read = await store().read(item.row.key);
        if (!read) {
          setItems((all) => all.map((one) => (one === item ? { ...one, row: { ...one.row, problem: "damaged" }, view: { ...one.view, line2: "Cannot be restored: damaged." } } : one)));
          return;
        }
        const assembler = new WasmAssembler(read.copy);
        for (const asset of read.assets) {
          assembler.add(asset.name, asset.bytes);
        }
        const bytes = assembler.finish();
        assembler.free();
        if (!bytes) {
          setItems((all) => all.map((one) => (one === item ? { ...one, row: { ...one.row, problem: "damaged" } } : one)));
          return;
        }
        // The restored session takes the copy's key and lock, so the row goes and a second crash
        // before Save still recovers it (ADR 0079 §6).
        adopt.current = item.row.key;
        const named = item.row.name ? `${item.view.recovered_name}.curvyo` : null;
        const opened = await api.openRecovered(bytes, named);
        if (opened) {
          setItems((all) => all.filter((one) => one.row.key !== item.row.key));
          depsRef.current.editor.containerRef.current?.focus();
        } else {
          adopt.current = null;
        }
      };
      api.replaceThen("restore", open, refocus, item.view.subject);
    },
    [store],
  );

  const discard = useCallback(
    async (item: RecoveryRowItem) => {
      await chain(item.row.key, () => store().end(item.row.key).catch(() => undefined));
      setItems((all) => all.filter((one) => one.row.key !== item.row.key));
    },
    [chain, store],
  );

  const end = useCallback(async () => {
    const { key, session } = live.current;
    if (session) {
      await chain(key, () => store().end(key).catch(() => undefined));
    }
  }, [chain, store]);

  return {
    rows: items,
    // Rust's words, asked only once a row exists (so only after the wasm module is up).
    heading: items.length > 0 ? recovery_heading() : "",
    help: items.length > 0 ? recovery_help_line(where ?? undefined) : "",
    where,
    failure,
    restore,
    discard,
    end,
  };
}

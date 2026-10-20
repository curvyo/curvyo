// The cuttable check in the page (`specs/0075-font-library` criterion 18, `adrs.md` decision 5):
// the Curvyo fonts when listed, the system fonts once Cuttable was chosen on this computer, one
// face per idle slice, results found by stamp in the cache first, and the system run's progress.
import { useCallback, useEffect, useRef, useState } from "react";

import { listedFileBytes } from "@/hooks/useText";
import { page, representative, resultKey, stampOf } from "@/hooks/fontLibraryPage";
import type { FontFace as ListedFace, FontFamily } from "@/lib/textText";
import { invoke } from "@/platform/host";

const idle: (run: () => void) => void =
  typeof window !== "undefined" && window.requestIdleCallback
    ? (run) => window.requestIdleCallback(run)
    : (run) => window.setTimeout(run, 50);

/** How many results are kept before the cache is written (`adrs.md` decision 5). */
const WRITE_EVERY = 50;

export interface CuttableChecks {
  /** The system fonts' check: done of total, while it runs. */
  progress: { done: number; total: number } | null;
  /** Cuttable was chosen: the system fonts are checked from now on, on this computer. */
  requestSystem: () => void;
}

export function useCuttableChecks(
  ready: boolean,
  families: FontFamily[],
  systemRead: boolean,
  bump: () => void,
): CuttableChecks {
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const queue = useRef<{ face: ListedFace; family: string; system: boolean }[]>([]);
  const running = useRef(false);
  const sinceWrite = useRef(0);
  const progressRef = useRef<{ done: number; total: number } | null>(null);
  /** Cuttable was chosen once on this computer: the system fonts are checked. */
  const [systemAsked, setSystemAsked] = useState(page.checks?.system_requested() ?? false);

  const writeChecks = useCallback(() => {
    sinceWrite.current = 0;
    const checks = page.checks;
    if (checks) {
      void invoke("write_font_checks", { text: checks.text() }).catch(() => {});
    }
  }, []);

  const runQueue = useCallback(() => {
    if (running.current) {
      return;
    }
    const next = queue.current.shift();
    const checks = page.checks;
    if (!next || !checks) {
      if (sinceWrite.current > 0) {
        writeChecks();
      }
      if (progressRef.current) {
        progressRef.current = null;
        setProgress(null);
      }
      return;
    }
    running.current = true;
    const finish = (pieces: number) => {
      page.results.set(resultKey(next.face), pieces);
      page.library?.set_check(next.family, pieces);
      sinceWrite.current += 1;
      if (sinceWrite.current >= WRITE_EVERY) {
        writeChecks();
      }
      if (next.system && progressRef.current) {
        progressRef.current = { ...progressRef.current, done: progressRef.current.done + 1 };
        setProgress(progressRef.current);
      }
      bump();
      running.current = false;
      runQueueRef.current();
    };
    void listedFileBytes(next.face.id)
      .then((bytes) =>
        idle(() => finish(checks.check(next.face.id, next.face.faceIndex, stampOf(next.face), bytes))),
      )
      .catch(() => idle(() => finish(-1)));
  }, [bump, writeChecks]);
  const runQueueRef = useRef(runQueue);
  useEffect(() => {
    runQueueRef.current = runQueue;
  }, [runQueue]);

  /** Queues the families of `block` whose result is not known; a stamp that matches the cache
   * needs no bytes. */
  const enqueue = useCallback(
    (block: "curvyo" | "system") => {
      const checks = page.checks;
      if (!checks) {
        return 0;
      }
      let queued = 0;
      for (const family of families.filter((f) => f.block === block)) {
        const face = representative(family);
        const key = resultKey(face);
        if (page.results.has(key) || queue.current.some((q) => resultKey(q.face) === key)) {
          continue;
        }
        const cached = checks.by_stamp(face.id, face.faceIndex, stampOf(face));
        if (cached !== -2) {
          page.results.set(key, cached);
          page.library?.set_check(family.family, cached);
          continue;
        }
        const item = { face, family: family.family, system: block === "system" };
        // A Curvyo font runs next (criterion 18: within 1 s of its row appearing).
        if (block === "curvyo") {
          queue.current.unshift(item);
        } else {
          queue.current.push(item);
        }
        queued += 1;
      }
      return queued;
    },
    [families],
  );

  // The Curvyo fonts are checked when listed; the system fonts once Cuttable was chosen on this
  // computer.
  useEffect(() => {
    if (!ready) {
      return;
    }
    enqueue("curvyo");
    if (systemAsked && systemRead) {
      const queued = enqueue("system");
      if (queued > 0 && !progressRef.current) {
        progressRef.current = { done: 0, total: queued };
        setProgress(progressRef.current);
      }
    }
    bump();
    runQueue();
  }, [ready, enqueue, runQueue, bump, systemRead, systemAsked]);

  const requestSystem = useCallback(() => {
    if (page.checks?.request_system()) {
      writeChecks();
      setSystemAsked(true);
    }
  }, [writeChecks]);
  // The cache read at load says whether Cuttable was chosen before on this computer.
  useEffect(() => {
    if (ready) {
      setSystemAsked(page.checks?.system_requested() ?? false);
    }
  }, [ready]);

  return { progress, requestSystem };
}

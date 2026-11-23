// The recovery copies' types and the browser's store over an abstract backend
// (`specs/0136-autosave-and-restore` criteria 6, 7, 11, 12, ADR 0079 decision 8). Free of the page's
// platform imports so `tests/recoveryStore.test.ts` covers it with a double backend; the real
// IndexedDB and Web Locks backend is in `recoveryStore.ts`.

/** One copy waiting at start, the shape both stores answer (`curvyo-app`'s `Row`). */
export interface RecoveryRow {
  key: string;
  /** The saved file's name, `null` for a never-saved project. */
  name: string | null;
  pages: number;
  /** The time of the last step, ms since the epoch. */
  last_step_ms: number;
  /** The saved file the copy was based on has changed or is gone (desktop only). */
  changed: boolean;
  /** Criterion 7. */
  problem: "damaged" | "newer" | null;
}

/** The listing at start. `path` is where the desktop keeps the copies, `null` in the browser. */
export interface RecoveryListing {
  path: string | null;
  rows: RecoveryRow[];
}

/** A copy read back for Restore: its container and the assets that are there. */
export interface RecoveryRead {
  copy: Uint8Array;
  assets: { name: string; bytes: Uint8Array }[];
}

/** Why a write failed: a code the Rust words turn into a sentence (`recovery_failure_sentence`). */
export class RecoveryWriteError extends Error {
  readonly code: string;

  constructor(code: string) {
    super(code);
    this.code = code;
  }
}

export interface RecoveryStore {
  list(): Promise<RecoveryListing>;
  /** Holds the key for the life of this session (Restore adopts a crashed one); rejects with
   * `held` while a running session holds it. */
  begin(key: string): Promise<void>;
  writeAsset(key: string, name: string, bytes: Uint8Array): Promise<void>;
  /** The copy, then its record (`record` is the text of `recovery.json`). */
  writeCopy(key: string, record: string, copy: Uint8Array): Promise<void>;
  /** Save, or the project is clean again: the copy goes, the key stays. */
  deleteCopy(key: string): Promise<void>;
  /** Discard, a clean Close or a session that ended: the lock and everything go. */
  end(key: string): Promise<void>;
  read(key: string): Promise<RecoveryRead | null>;
  /** What the saved file at `path` looks like now (`{path, modified_ms, length}` JSON), or empty
   * when there is none; the browser has no files. */
  fileBase(path: string): Promise<string>;
}

/** A random 16-byte session key as 32 hex digits. */
export function mintRecoveryKey(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}


/** What the browser store needs of IndexedDB and the Web Locks; the page's own in the build, a
 * double in the tests. */
export interface BrowserBackend {
  /** The record and the copy in one transaction. */
  putCopy(key: string, record: string, copy: Uint8Array): Promise<void>;
  /** An asset, once: one that is there is left alone. */
  putAsset(key: string, name: string, bytes: Uint8Array): Promise<void>;
  /** The record, the copy and the assets of the key. */
  deleteCopy(key: string): Promise<void>;
  /** Every stored copy: key, record text, container and the bytes of its assets. */
  entries(): Promise<{ key: string; record: string; copy: Uint8Array; assetBytes: number }[]>;
  assets(key: string): Promise<{ name: string; bytes: Uint8Array }[]>;
  /** Takes the key's lock for the life of the tab; `false` while another tab holds it. */
  hold(key: string): Promise<boolean>;
  release(key: string): void;
  /** The keys whose lock a tab holds now. */
  held(): Promise<Set<string>>;
}

/** The Rust checks the listing needs (`recovery_check_entry`, `recovery_prune_plan`). */
export interface RecoveryRules {
  check(record: string, copy: Uint8Array): { name: string | null; pages: number; last_step_ms: number | null; problem: "damaged" | "newer" | null };
  prune(candidates: { key: string; bytes: number; last_step_ms: number; locked: boolean }[]): string[];
}

/** The browser store over `backend`; the browser has no file path, so a row never says "File
 * changed since" (criterion 6 has no counterpart here). */
export function browserRecoveryStore(backend: BrowserBackend, rules: RecoveryRules): RecoveryStore {
  const quota = (error: unknown) =>
    new RecoveryWriteError(
      (error as { name?: string } | null)?.name === "QuotaExceededError" ? "browser-full" : String((error as Error)?.message ?? error),
    );
  return {
    list: async () => {
      const held = await backend.held();
      const entries = await backend.entries();
      const checked = entries.map((entry) => {
        const seen = rules.check(entry.record, entry.copy);
        return { entry, seen, locked: held.has(entry.key) };
      });
      const doomed = new Set(
        rules.prune(
          checked.map(({ entry, seen, locked }) => ({
            key: entry.key,
            bytes: entry.copy.byteLength + entry.assetBytes,
            last_step_ms: seen.last_step_ms ?? 0,
            locked,
          })),
        ),
      );
      for (const key of doomed) {
        await backend.deleteCopy(key);
      }
      const rows = checked
        .filter(({ entry, locked }) => !locked && !doomed.has(entry.key))
        .map(({ entry, seen }): RecoveryRow => ({
          key: entry.key,
          name: seen.name,
          pages: seen.pages,
          last_step_ms: seen.last_step_ms ?? 0,
          changed: false,
          problem: seen.problem,
        }))
        .sort((a, b) => b.last_step_ms - a.last_step_ms || a.key.localeCompare(b.key));
      return { path: null, rows };
    },
    begin: async (key) => {
      if (!(await backend.hold(key))) {
        throw new RecoveryWriteError("held");
      }
    },
    writeAsset: (key, name, bytes) => backend.putAsset(key, name, bytes).catch((error: unknown) => {
      throw quota(error);
    }),
    writeCopy: async (key, record, copy) => {
      await backend.hold(key);
      await backend.putCopy(key, record, copy).catch((error: unknown) => {
        throw quota(error);
      });
    },
    deleteCopy: (key) => backend.deleteCopy(key),
    end: async (key) => {
      await backend.deleteCopy(key);
      backend.release(key);
    },
    read: async (key) => {
      const entry = (await backend.entries()).find((candidate) => candidate.key === key);
      return entry ? { copy: entry.copy, assets: await backend.assets(key) } : null;
    },
    fileBase: () => Promise.resolve(""),
  };
}


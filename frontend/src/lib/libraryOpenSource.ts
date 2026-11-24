// Open source (`specs/0137-document-library` criterion 7, UX notes section 8.3) in both builds:
// the source document is found again by the record's page id, opened the way its build opens it
// (a document of the open workspace through its tree; on the desktop another file through the
// host's Open; in the browser a document another stored workspace holds, through that workspace),
// and the page is activated by id; a page gone from the source gives the notice, a document gone
// the alert. The host calls are passed in, so this runs in a test without a host.
import type { CameFromRecord, SourcePlace } from "@/lib/libraryApi";

/** What Open source needs from its build. */
export interface OpenSourceDeps {
  /** Reads the sources again. */
  scan: () => Promise<unknown>;
  /** The document holding the record's page id, or null. */
  find: (record: CameFromRecord) => Promise<SourcePlace | null>;
  /** The desktop build (else the browser). */
  desktop: boolean;
  /** Whether `place` is the document open in this window. */
  isOpen: (place: SourcePlace) => boolean;
  /** Activates page `id` of the open document; false when it has no such page. */
  activateNow: (id: string) => boolean;
  /** Waits for the next opened document to activate the record's page (or say it is gone). */
  activateWhenOpen: (record: CameFromRecord) => void;
  /** Opens a document of the open workspace behind the unsaved-changes check. */
  openInWorkspace: (path: string[]) => void;
  /** Browser: opens the document `path` of the stored workspace `source`; false when it is gone. */
  openStored: (source: string, path: string[]) => Promise<boolean>;
  /** Desktop: opens a file outside the open workspace. */
  openPath: (file: string) => Promise<void>;
  /** The page-gone notice (status) and the document-gone alert. */
  pageGone: (record: CameFromRecord) => void;
  sourceGone: (record: CameFromRecord) => void;
}

/** Opens the source of `record`; `known` is where it was found when the button was shown, used
 * when the source changed since and no longer has the page (it opens on its first page). */
export async function openSource(
  record: CameFromRecord,
  known: SourcePlace | null,
  deps: OpenSourceDeps,
): Promise<void> {
  await deps.scan().catch(() => undefined);
  const found = await deps.find(record).catch(() => null);
  const place = found ?? known;
  if (!place) {
    deps.sourceGone(record);
    return;
  }
  if (deps.isOpen(place)) {
    if (!deps.activateNow(record.page_id)) {
      deps.pageGone(record);
    }
    return;
  }
  deps.activateWhenOpen(record);
  if (place.in_workspace) {
    deps.openInWorkspace(place.path);
  } else if (!deps.desktop) {
    if (!(await deps.openStored(place.source, place.path).catch(() => false))) {
      deps.sourceGone(record);
    }
  } else {
    await deps.openPath(place.file).catch(() => deps.sourceGone(record));
  }
}

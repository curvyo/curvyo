// The font sync of the page (`specs/0116-library-sync` criteria 16 to 19): the host side of the
// font entries of the device group. The group (wasm) decides what is switched on, what the
// budgets allow and what arrived; this module reads and writes the font files through the host,
// hashes them (the same SHA-256 as the entry key), carries a family's tags, star and Cuttable
// between the font library file and the group, and refreshes the listing after a write.
import {
  FONT_LIBRARY_CHANGED,
  type FontFileInfo,
  HashSetStore,
  familySyncAny,
  familySyncOn,
  filesOfFamily,
  groupFiles,
  newIdsOfHash,
  startsOnItsOwn,
  syncedIdsToTrash,
  travellingDocuments,
} from "@/lib/fontSyncModel";
import { loadPage, page } from "@/hooks/fontLibraryPage";
import { hostFolders, hostListing, listedFileBytes, forgetFileBytes } from "@/hooks/useText";
import { sha256 } from "@/lib/browserFonts";
import { invoke, isTauri } from "@/platform/host";
import { library_word, type WasmDeviceGroup } from "@/wasm-bindings/curvyo_editor_wasm.js";
import type { FontFamily } from "@/lib/textText";
import type { LibraryToggleRow } from "@/lib/librarySyncView";

/** What the font sync needs of its page. */
export interface FontSyncHost {
  /** Lists the Curvyo fonts again after a write. */
  refreshFonts: () => Promise<void>;
  /** A canvas notice. */
  notice: (text: string) => void;
  /** The hashes or toggles changed: the rows read again. */
  changed: () => void;
}

interface Arrival {
  hash: string;
  kind: "add" | "meta" | "removed";
  file_name: string | null;
  folder: string | null;
  tags: string[];
  favourite: boolean;
  cuttable: string;
  documents: { hash: string; name: string }[];
}

/** An array the host takes as bytes: a number array over Tauri's JSON, the array itself in the
 * browser. */
function hostBytes(bytes: Uint8Array): number[] | Uint8Array {
  return isTauri ? Array.from(bytes) : bytes;
}

function exact(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

/** Writes the font library file after a synced change (never over a newer one). */
async function saveLibrary(): Promise<void> {
  const library = page.library;
  if (!library || page.status === "newer") {
    return;
  }
  if (page.status === "broken") {
    await invoke("set_font_library_aside");
    page.status = "ok";
  }
  await invoke("write_font_library", { text: library.text() });
  window.dispatchEvent(new Event(FONT_LIBRARY_CHANGED));
}

/** The browser's storage, or none where it is blocked. */
function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

export class FontSync {
  /** The SHA-256 of each listed file, by host id. */
  readonly hashes = new Map<string, string>();
  private readonly stamps = new Map<string, string>();
  private busy = false;
  /** Hashes whose switch-on was refused: not tried again until the file changes. */
  private readonly refused = new Set<string>();
  /** The metadata last known equal on both sides, by hash. */
  private readonly meta = new Map<string, string>();
  private readonly removedHashes: string[] = [];
  /** The host ids of the files written here from another device: only these go to the trash when
   * the maker deletes the font elsewhere (ADR 0062 decision 4); an id is dropped when its file
   * goes. */
  private readonly viaSync = new HashSetStore("curvyo.fontsViaSyncIds", browserStorage());
  /** The hashes deleted on another device whose copy here stayed (the maker's own install): they
   * are not switched on again, which would publish the font to the group once more. */
  private readonly deletedElsewhere = new HashSetStore("curvyo.fontsDeletedElsewhere", browserStorage());
  /** The refusals already told after an automatic switch-on, so no start of the app tells them
   * again. */
  private readonly told = new HashSetStore("curvyo.fontsRefusalTold", browserStorage());

  private readonly host: FontSyncHost;

  constructor(host: FontSyncHost) {
    this.host = host;
  }

  /** The listed files of the Curvyo folder. */
  files(): FontFileInfo[] {
    return groupFiles(hostListing("curvyo"));
  }

  /** Whether the family's toggle reads on in `rows`. */
  isOn(family: FontFamily, rows: readonly LibraryToggleRow[]): boolean {
    return familySyncOn(filesOfFamily(this.files(), family.family), this.hashes, rows);
  }

  /** The maker deleted these faces' files: their entries go at the next pump. */
  forgetFiles(ids: string[]): void {
    for (const id of ids) {
      const hash = this.hashes.get(id);
      if (hash !== undefined) {
        this.removedHashes.push(hash);
      }
    }
  }

  /** The toggle of a family (criteria 3, 16, 19): a press switches every file of it off when any
   * is on (also a family part of whose files was refused), else every file on with its bytes. */
  async toggle(group: WasmDeviceGroup, family: FontFamily, rows: readonly LibraryToggleRow[]): Promise<void> {
    const files = filesOfFamily(this.files(), family.family);
    const off = familySyncAny(files, this.hashes, rows);
    for (const file of files) {
      const hash = this.hashes.get(file.id);
      if (off && hash !== undefined) {
        group.library_font_switch_off(hash);
      } else if (!off) {
        this.refused.delete(hash ?? "");
        this.deletedElsewhere.delete(hash ?? "");
        await this.switchOn(group, file, true);
      }
    }
    this.host.changed();
  }

  /** Switches one file on with its bytes and documents; a refusal is a notice (after an automatic
   * switch-on only the first time for that file). */
  private async switchOn(group: WasmDeviceGroup, file: FontFileInfo, pressed: boolean): Promise<void> {
    await loadPage();
    const bytes = await listedFileBytes(file.id).catch(() => null);
    if (bytes === null || page.library === null) {
      return;
    }
    for (const doc of await this.documents(file)) {
      group.library_font_stage_document(doc.name, doc.bytes);
    }
    const meta = JSON.parse(page.library.sync_meta(file.families[0])) as { tags: string[]; favourite: boolean; cuttable: string };
    const input = { file_name: file.file, folder: file.folder, ...meta };
    const out = JSON.parse(group.library_font_set(JSON.stringify(input), bytes)) as { hash?: string; notice?: string };
    if (out.hash !== undefined) {
      this.meta.set(out.hash, JSON.stringify(meta));
    } else if (out.notice) {
      const hash = this.hashes.get(file.id) ?? file.id;
      if (pressed || !this.told.has(hash)) {
        this.host.notice(out.notice);
      }
      this.told.add(hash);
    }
  }

  /** The text documents of the file's font folder, as bytes. */
  private async documents(file: FontFileInfo): Promise<{ name: string; bytes: Uint8Array }[]> {
    const out: { name: string; bytes: Uint8Array }[] = [];
    for (const name of travellingDocuments(file.folder, hostFolders())) {
      const read = await invoke<{ text: string; cut: boolean }>("read_font_document", { folder: file.folder, name }).catch(() => null);
      if (read !== null && !read.cut) {
        out.push({ name, bytes: new TextEncoder().encode(read.text) });
      }
    }
    return out;
  }

  /** The slow poll's work (every second): hashes, what the group asks for, what arrived, what
   * was deleted, the metadata changed here, and fonts installed while paired. */
  async pump(group: WasmDeviceGroup, newItems: boolean, rows: readonly LibraryToggleRow[]): Promise<void> {
    if (this.busy) {
      return;
    }
    this.busy = true;
    try {
      await loadPage();
      await this.hashListed();
      group.library_font_host_hashes(JSON.stringify([...new Set(this.hashes.values())]));
      await this.supply(group);
      await this.receive(group);
      for (const hash of this.removedHashes.splice(0)) {
        group.library_font_remove(hash);
      }
      this.publishMeta(group, rows);
      if (newItems) {
        await this.newFonts(group, rows);
      }
    } finally {
      this.busy = false;
    }
  }

  /** Hashes every listed file whose stamp changed; forgets the files that went. */
  private async hashListed(): Promise<void> {
    const files = this.files();
    let changed = false;
    for (const known of [...this.hashes.keys()]) {
      if (!files.some((file) => file.id === known)) {
        this.hashes.delete(known);
        this.stamps.delete(known);
        this.viaSync.delete(known);
        changed = true;
      }
    }
    if (files.length > 0) {
      this.viaSync.retain(new Set(files.map((file) => file.id)));
    }
    for (const file of files) {
      if (this.stamps.get(file.id) === file.stamp && this.hashes.has(file.id)) {
        continue;
      }
      const bytes = await listedFileBytes(file.id).catch(() => null);
      if (bytes !== null) {
        this.hashes.set(file.id, await sha256(exact(bytes)));
        this.stamps.set(file.id, file.stamp);
        changed = true;
      }
    }
    if (files.length > 0) {
      this.deletedElsewhere.retain(new Set(this.hashes.values()));
    }
    if (changed) {
      this.host.changed();
    }
  }

  /** Reads the files the group lacks the bytes of (its snapshot needs them, plan P24). */
  private async supply(group: WasmDeviceGroup): Promise<void> {
    const wants = JSON.parse(group.library_font_wants()) as string[];
    if (wants.length === 0) {
      return;
    }
    const files = this.files();
    for (const hash of wants) {
      const file = files.find((f) => this.hashes.get(f.id) === hash);
      if (file) {
        const bytes = await listedFileBytes(file.id).catch(() => null);
        if (bytes !== null) {
          group.library_font_supply(hash, bytes);
        }
        continue;
      }
      for (const holder of files) {
        for (const doc of await this.documents(holder)) {
          if ((await sha256(exact(doc.bytes))) === hash) {
            group.library_font_supply(hash, doc.bytes);
          }
        }
      }
    }
  }

  /** Writes the fonts that arrived, applies their metadata and deletes what was deleted. */
  private async receive(group: WasmDeviceGroup): Promise<void> {
    const arrivals = JSON.parse(group.library_font_arrivals()) as Arrival[];
    if (arrivals.length === 0) {
      return;
    }
    const adds = arrivals.filter((a) => a.kind === "add" && a.file_name !== null);
    const before = new Set<string>();
    let written: string[] = [];
    for (const arrival of arrivals) {
      if (arrival.kind !== "removed") {
        this.deletedElsewhere.delete(arrival.hash);
      }
    }
    if (adds.length > 0) {
      const fonts = adds.map((a) => ({
        file_name: a.file_name,
        folder: a.folder,
        bytes: hostBytes(group.library_font_blob(a.hash)),
        documents: a.documents.map((d) => ({ name: d.name, bytes: hostBytes(group.library_font_blob(d.hash)) })),
      }));
      const listedBefore = new Set(this.files().map((file) => file.id));
      const outcome = await invoke<string[]>("add_synced_fonts", { fonts }).catch(() => []);
      written = adds.filter((_, at) => outcome[at] === "written").map((add) => add.hash);
      listedBefore.forEach((id) => before.add(id));
      if (outcome.some((word) => word === "refused")) {
        this.host.notice(library_word("font-skipped"));
      }
    }
    await this.host.refreshFonts();
    this.stamps.clear();
    await this.hashListed();
    const listed = this.files().map((file) => file.id);
    for (const hash of written) {
      for (const id of newIdsOfHash(listed, before, this.hashes, hash)) {
        this.viaSync.add(id);
      }
    }
    for (const arrival of arrivals.filter((a) => a.kind === "removed")) {
      this.deletedElsewhere.add(arrival.hash);
      await this.trash(arrival.hash);
    }
    if (arrivals.some((a) => a.kind === "removed")) {
      await this.host.refreshFonts();
      this.stamps.clear();
      await this.hashListed();
    }
    let saved = false;
    for (const arrival of arrivals.filter((a) => a.kind !== "removed")) {
      saved = this.applyMeta(arrival) || saved;
    }
    if (saved) {
      await saveLibrary().catch(() => undefined);
    }
  }

  /** The font library carries what arrived for every family of the file; whether it changed. */
  private applyMeta(arrival: Arrival): boolean {
    const file = this.files().find((f) => this.hashes.get(f.id) === arrival.hash);
    if (!file || page.library === null) {
      return false;
    }
    const meta = JSON.stringify({ tags: arrival.tags, favourite: arrival.favourite, cuttable: arrival.cuttable });
    let changed = false;
    for (const family of file.families) {
      changed = page.library.apply_sync_meta(family, meta) || changed;
    }
    this.meta.set(arrival.hash, JSON.stringify(JSON.parse(page.library.sync_meta(file.families[0]))));
    return changed;
  }

  /** A font deleted on another device goes to the trash here (criterion 11): the files written
   * here through sync, by id. A copy the maker installed, here or in another folder, stays. */
  private async trash(hash: string): Promise<void> {
    const ids = syncedIdsToTrash(
      this.files().map((f) => f.id),
      this.hashes,
      hash,
      this.viaSync,
    );
    for (const id of ids) {
      this.viaSync.delete(id);
    }
    const faces = hostListing("curvyo")
      .filter((face) => ids.includes(face.id))
      .map((face) => ({ id: face.id, index: face.index }));
    if (faces.length > 0) {
      await invoke("trash_font_files", { faces }).catch(() => undefined);
      forgetFileBytes(ids);
    }
  }

  /** The metadata changed here goes to the group, once per change (criterion 17). */
  private publishMeta(group: WasmDeviceGroup, rows: readonly LibraryToggleRow[]): void {
    if (page.library === null) {
      return;
    }
    for (const file of this.files()) {
      const hash = this.hashes.get(file.id);
      if (hash === undefined || !rows.some((row) => row.id === hash && row.sync)) {
        continue;
      }
      const current = page.library.sync_meta(file.families[0]);
      if (this.meta.get(hash) !== current) {
        this.meta.set(hash, current);
        group.library_font_meta(hash, current);
      }
    }
  }

  /** A font installed while paired starts on (criterion 4, once the first sync was confirmed);
   * one the maker switched off has its marker and is left alone. */
  private async newFonts(group: WasmDeviceGroup, rows: readonly LibraryToggleRow[]): Promise<void> {
    for (const file of this.files()) {
      const hash = this.hashes.get(file.id);
      if (hash === undefined || !startsOnItsOwn(hash, rows, this.refused, this.deletedElsewhere)) {
        continue;
      }
      this.refused.add(hash);
      await this.switchOn(group, file, false);
    }
  }
}

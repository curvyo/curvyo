// The online part of the Library tab (`specs/0138-template-library`, ADR 0085): the desktop host's
// commands, which are the only door to the network. A command takes a source key and a name,
// never an address; nothing here is called until the maker acts (add, expand, Refresh, Load,
// Enter in the search). The browser build has none of it.
import { createIconBatcher } from "@/lib/iconBatch";
import { invoke } from "@/platform/host";

/** The sentence under the Address field, or the key of the added source. */
export type AddSourceAnswer =
  | { kind: "added"; key: string }
  | { kind: "refused"; text: string }
  | { kind: "cancelled" };

/** The Came from keys of an online item, as `curvyo_document_core::OnlineOrigin`. */
export interface OnlineOrigin {
  address: string;
  spdx: string;
  licence: string;
  licence_url: string;
  author: string;
  credit: string | null;
}

/** A template's Came from data. */
export interface TemplateOrigin {
  name: string;
  without_fonts: boolean;
  origin: OnlineOrigin;
}

/** An icon ready to import. */
export interface IconImport {
  svg: string;
  name: string;
  set: string;
  origin: OnlineOrigin;
}

/** What Load answered. */
export type LoadAnswer = ({ kind: "ready" } & TemplateOrigin) | { kind: "refused"; text: string };

/** Add source: the host checks the address with one request. */
export function addOnlineSource(kind: "git" | "iconify", address: string): Promise<AddSourceAnswer> {
  return invoke<AddSourceAnswer>("online_add", { kind, address });
}

/** Add on the ready Curvyo catalogue's entry: it joins the list and sends nothing. */
export function addReadySource(): Promise<AddSourceAnswer> {
  return invoke<AddSourceAnswer>("online_add_ready");
}

/** Remove a source from the list; its fetched data leaves the session. The sentence when the list
 * was made by a newer Curvyo and is left alone, else null. */
export function removeOnlineSource(key: string): Promise<string | null> {
  return invoke<string | null>("online_remove", { key });
}

/** Expanding a source, Refresh. */
export function openOnlineList(key: string, force: boolean): Promise<void> {
  return invoke<void>("online_open", { key, open: { what: "list", force } });
}

/** Expanding an icon set. */
export function openOnlineSet(key: string, prefix: string): Promise<void> {
  return invoke<void>("online_open", { key, open: { what: "set", prefix } });
}

/** Expanding a template: it is downloaded and checked. */
export function openOnlineTemplate(key: string, path: string): Promise<void> {
  return invoke<void>("online_open", { key, open: { what: "template", path } });
}

/** What Enter started: the words as they go out and the Iconify sources asked (marked searching). */
export interface SearchBegun {
  words: string;
  keys: string[];
}

/** Enter in the search, first step: marks every added Iconify source as searching; null for an empty text. */
export function beginOnlineSearch(text: string): Promise<SearchBegun | null> {
  return invoke<SearchBegun | null>("online_search_begin", { text });
}

/** Second step, one source: answered when that source is done; its announcement. */
export function searchOnlineSource(key: string, words: string): Promise<string> {
  return invoke<string>("online_search_source", { key, words });
}

/** Cancel of the Add source form: the host discards the check that runs. */
export function cancelOnlineAdd(): Promise<void> {
  return invoke<void>("online_add_cancel");
}

/** Load: the template is downloaded and checked, with its Came from data. */
export function loadOnlineTemplate(key: string, path: string): Promise<LoadAnswer> {
  return invoke<LoadAnswer>("online_load", { key, path });
}

/** The Came from data of a template the host holds. */
export function onlineTemplateOrigin(key: string, path: string): Promise<TemplateOrigin | null> {
  return invoke<TemplateOrigin | null>("online_template_origin", { key, path });
}

/** The icon of a cell, ready to import. */
export function onlineIconImport(key: string, prefix: string, name: string): Promise<IconImport | null> {
  return invoke<IconImport | null>("online_icon_import", { key, prefix, name });
}

/** Opens a licence address in the system browser. */
export function openLicence(url: string): Promise<void> {
  return invoke<void>("online_open_licence", { url });
}

/** The thumbnail PNG of a template row; empty when it has none. */
export async function onlineThumbnail(key: string, path: string): Promise<Uint8Array> {
  const raw = await invoke<ArrayBuffer>("online_thumbnail", { key, path });
  return new Uint8Array(raw);
}

const batcher = createIconBatcher(
  (key, prefix, names) => invoke<[string, string][]>("online_icons", { key, prefix, names }),
  (run) => void window.setTimeout(run, 50),
);

/** The drawn SVG of an icon: the cells of one screen are collected for 50 ms and asked in one request
 * per set, at most 100 names each (ADR 0085 decision 5); an icon fetched once is kept for the session. */
export function onlineIconSvg(key: string, prefix: string, name: string): Promise<string> {
  return batcher.request(key, prefix, name);
}

/** Forgets the icons held for a source (Remove, Refresh). */
export function forgetOnlineIcons(key: string): void {
  batcher.forget(key);
}
